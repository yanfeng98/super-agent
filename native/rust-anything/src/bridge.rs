//! Step 2: scaffold a Rust crate for one unit, with JS bindings and a manifest the later
//! steps read.
//!
//! The crate splits bindings from logic: `src/lib.rs` holds thin `#[napi]` or
//! `#[wasm_bindgen]` wrappers, `src/port.rs` holds the port in plain Rust with
//! `#![forbid(unsafe_code)]`, so it can be tested without a JS runtime.

use crate::walk::Canonical;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::analyze::{self, Bridge, Status};
use crate::codegen;
use crate::risks::Risk;
use crate::ts::exports::{
    ExportedClass, ExportedFn, LocalType, exported_classes, exported_functions, local_types,
};

pub const MANIFEST: &str = "rust-anything.json";
pub const SCHEMA: &str = "rust-anything/bridge@1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Napi,
    Wasm,
    Pyo3,
    /// `extern "C"` functions with the original names: a drop-in replacement for C code.
    #[serde(rename = "c-abi")]
    CAbi,
    /// A JSON dispatcher over the port, called from a generated Go, Ruby or Java wrapper.
    Ffi,
}

/// The global allocator of a crate with batches (`bridge --allocator`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum Allocator {
    /// mimalloc, built from C with `cc`. Wasm crates keep Rust's default allocator.
    #[default]
    #[serde(rename = "mimalloc")]
    #[value(name = "mimalloc")]
    Mimalloc,
    /// rusty_alloc, a pure-Rust remake of mimalloc: no C compiler, and wasm crates get it too.
    #[serde(rename = "rusty_alloc")]
    #[value(name = "rusty_alloc", alias = "rusty-alloc")]
    RustyAlloc,
}

impl Allocator {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// What `bridge` generated; every later step starts from it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: String,
    /// The repository root: absolute, or relative to the crate folder (so a crate kept inside
    /// the repository it ports works in any clone). Resolved when the manifest is loaded.
    pub root: PathBuf,
    /// `root` as written in the file, when it was relative; written back instead of the resolved
    /// path.
    #[serde(skip)]
    pub root_written: Option<PathBuf>,
    pub unit: String,
    #[serde(default)]
    pub language: crate::lang::Lang,
    pub kind: Kind,
    pub crate_name: String,
    /// Exports ported to Rust.
    pub exports: Vec<ExportedFn>,
    /// Exported classes ported to Rust: a struct with `new` and methods.
    #[serde(default)]
    pub classes: Vec<ExportedClass>,
    /// Exports that cannot cross the bridge as they are; they stay in TS and the shadow module
    /// re-exports them from the original.
    #[serde(default)]
    pub js_only: Vec<JsOnly>,
    /// Functions run over many items kept in Rust (`--batch function:param`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub batches: Vec<crate::codegen_batch::Batch>,
    /// The global allocator when the crate has batches (`bridge --allocator`).
    #[serde(default, skip_serializing_if = "Allocator::is_default")]
    pub allocator: Allocator,
    /// Floats match within this relative tolerance (`record --float-tolerance`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub float_tolerance: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsOnly {
    pub name: String,
    pub file: String,
    pub line: u32,
    pub reason: String,
}

/// Whether values of the local type `name` can hold functions: the type has a function or
/// method member, or extends or mentions a type that does.
fn has_functions(name: &str, local_types: &[LocalType], seen: &mut Vec<String>) -> bool {
    if seen.iter().any(|s| s == name) {
        return false;
    }
    seen.push(name.to_string());
    let Some(t) = local_types.iter().find(|t| t.name == name) else {
        return false;
    };
    let text = &t.text;
    let method = text.lines().skip(1).any(|l| {
        let l = l.trim().trim_start_matches("readonly ");
        l.contains('(')
            && l.split('(').next().is_some_and(|head| {
                !head.is_empty()
                    && head
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '?')
            })
    });
    if text.contains("=>") || method {
        return true;
    }
    text.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .filter(|w| !w.is_empty() && *w != name)
        .any(|w| has_functions(w, local_types, seen))
}

/// The exported functions of a TS/JS `file` with the types `bridge` judges them by: a sibling
/// `.d.ts` applied, aliases resolved and generic constraints substituted. `analyze` counts
/// bridgeable exports from the same list.
pub fn ts_functions(root: &Path, file: &str, text: &str, types: &[LocalType]) -> Vec<ExportedFn> {
    let mut functions = exported_functions(file, text);
    crate::ts::jsdoc::apply_declarations(root, file, &mut functions);
    for f in &mut functions {
        for p in &mut f.params {
            p.ts_type = resolve_aliases(&p.ts_type, types);
        }
        f.returns = resolve_aliases(&f.returns, types);
        substitute_constraints(f, types);
    }
    functions
}

/// Why a class has to stay in the source language: its blocker, else the first method (and, for
/// TS/JS, the constructor) that cannot cross. `None` when it can be bridged.
pub fn class_reason(class: &ExportedClass, types: &[LocalType], ts: bool) -> Option<String> {
    let constructor = ts.then(|| ExportedFn {
        name: format!("{} constructor", class.name),
        file: class.file.clone(),
        line: class.line,
        params: class.constructor.clone(),
        returns: "void".into(),
        ..Default::default()
    });
    class.blocker.clone().or_else(|| {
        constructor
            .iter()
            .chain(&class.methods)
            .find_map(|m| js_only_reason(m, types).map(|r| format!("`{}`: {r}", m.name)))
    })
}

/// Why an export cannot cross a JS/Rust bridge unchanged, if it cannot.
///
/// Functions, iterators, RegExp objects and objects with function members are JS values a
/// Rust function can neither take nor return; porting them would change the public API.
pub fn js_only_reason(f: &ExportedFn, local_types: &[LocalType]) -> Option<String> {
    if let Some(blocker) = &f.blocker {
        return Some(blocker.clone());
    }
    if f.is_generator {
        return Some("generator: returns a lazy iterator".into());
    }
    if f.uses_instanceof {
        return Some("checks `instanceof`: class identity does not survive the bridge".into());
    }
    let types: Vec<&str> = f
        .params
        .iter()
        .map(|p| p.ts_type.as_str())
        .chain(std::iter::once(f.returns.as_str()))
        .collect();
    if types.iter().any(|t| t.contains("=>")) {
        return Some("takes or returns a function".into());
    }
    let words = |t: &str| -> Vec<String> {
        t.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
            .filter(|w| !w.is_empty())
            .map(str::to_string)
            .collect()
    };
    for t in &types {
        for word in words(t) {
            match word.as_str() {
                "RegExp" => return Some("takes or returns a RegExp".into()),
                "Generator" | "Iterator" | "IterableIterator" | "AsyncGenerator"
                | "AsyncIterable" => {
                    return Some(format!("uses `{word}`, a lazy iterator"));
                }
                "Function" => return Some("takes or returns a function".into()),
                _ => {}
            }
            if f.type_params.contains(&word) {
                return Some(format!(
                    "generic over `{word}`: values of any shape, functions included"
                ));
            }
            if has_functions(&word, local_types, &mut Vec::new()) {
                return Some(format!("uses `{word}`, which has function members"));
            }
        }
    }
    None
}

impl Manifest {
    pub fn load(crate_dir: &Path) -> Result<Self> {
        let path = crate_dir.join(MANIFEST);
        let text = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "reading {} (run `rust-anything bridge` first)",
                path.display()
            )
        })?;
        let mut manifest: Self =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        if manifest.root.is_relative() {
            let resolved = crate_dir.join(&manifest.root);
            let resolved = resolved.canonical().unwrap_or(resolved);
            manifest.root_written = Some(std::mem::replace(&mut manifest.root, resolved));
        }
        Ok(manifest)
    }

    /// Writes the manifest back, keeping a relative root relative.
    pub fn save(&self, crate_dir: &Path) -> Result<()> {
        let mut m = self.clone();
        if let Some(written) = m.root_written.take() {
            m.root = written;
        }
        let path = crate_dir.join(MANIFEST);
        std::fs::write(&path, serde_json::to_string_pretty(&m)? + "\n")
            .with_context(|| format!("writing {}", path.display()))
    }

    /// Rust library name: the crate name with `-` replaced.
    pub fn lib_name(&self) -> String {
        self.crate_name.replace('-', "_")
    }
}

pub struct Options<'a> {
    pub root: &'a Path,
    pub unit: &'a str,
    pub kind: Option<Kind>,
    pub out: Option<PathBuf>,
    pub name: Option<String>,
    /// Bridge a unit the plan marks blocked or keep.
    pub force: bool,
    /// Exports to keep in TS, with an optional reason: `name` or `name: reason`.
    pub keep: Vec<String>,
    /// Source language; detected from the repository when `None`.
    pub lang: Option<crate::lang::Lang>,
    /// `function:param` pairs: batch classes running `function` over many `param` values.
    pub batch: Vec<String>,
    /// Type parameters the app only fills with data: bridged as JSON values instead of keeping
    /// the export in TS.
    pub as_json: Vec<String>,
    /// The global allocator for crates with batches.
    pub allocator: Allocator,
}

#[derive(Debug, Serialize)]
pub struct Outcome {
    pub crate_dir: PathBuf,
    pub kind: Kind,
    pub files: Vec<String>,
    pub exports: Vec<String>,
    pub notes: Vec<String>,
}

/// A Rust type as seen by the port, from a TS type annotation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RType {
    Str,
    F64,
    /// A whole number (`int` in the type dialect: Python, Go, Java and C have one; JS does not).
    Int,
    Bool,
    Unit,
    Vec(Box<RType>),
    Opt(Box<RType>),
    /// Anything structured: crosses the bridge as JSON.
    Json,
}

impl RType {
    pub fn from_ts(ts: &str) -> Self {
        let ts = ts.trim().trim_start_matches("readonly ").trim();
        // `(string | null)`: one parenthesized group around the whole type.
        if let Some(inner) = ts.strip_prefix('(').and_then(|t| t.strip_suffix(')'))
            && balanced(inner)
        {
            return Self::from_ts(inner);
        }
        if let Some(inner) = ts
            .strip_prefix("Promise<")
            .and_then(|t| t.strip_suffix('>'))
        {
            return Self::from_ts(inner);
        }
        let parts = split_top(ts, '|');
        if parts.len() > 1 {
            let (nullish, rest): (Vec<&str>, Vec<&str>) = parts
                .iter()
                .partition(|p| matches!(p.trim(), "undefined" | "null" | "void"));
            if rest.iter().all(|p| is_string_literal(p)) {
                return Self::wrap_opt(Self::Str, !nullish.is_empty());
            }
            if rest.len() == 1 {
                return Self::wrap_opt(Self::from_ts(rest[0]), !nullish.is_empty());
            }
            return Self::Json;
        }
        if let Some(inner) = ts.strip_suffix("[]") {
            return Self::Vec(Box::new(Self::from_ts(
                inner.trim_start_matches('(').trim_end_matches(')'),
            )));
        }
        for wrapper in ["Array<", "ReadonlyArray<"] {
            if let Some(inner) = ts.strip_prefix(wrapper).and_then(|t| t.strip_suffix('>')) {
                return Self::Vec(Box::new(Self::from_ts(inner)));
            }
        }
        // `value is MessageLeaf`: a type predicate returns a boolean; `asserts x` returns nothing.
        if ts.starts_with("asserts ") {
            return Self::Unit;
        }
        if let Some((subject, _)) = ts.split_once(" is ")
            && subject
                .trim()
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
        {
            return Self::Bool;
        }
        match ts {
            "string" => Self::Str,
            "number" => Self::F64,
            "int" => Self::Int,
            "boolean" => Self::Bool,
            "void" | "undefined" => Self::Unit,
            _ if is_string_literal(ts) => Self::Str,
            _ => Self::Json,
        }
    }

    fn wrap_opt(t: Self, optional: bool) -> Self {
        if optional { Self::Opt(Box::new(t)) } else { t }
    }

    pub fn rust(&self) -> String {
        match self {
            Self::Str => "String".into(),
            Self::F64 => "f64".into(),
            Self::Int => "i64".into(),
            Self::Bool => "bool".into(),
            Self::Unit => "()".into(),
            Self::Vec(t) => format!("Vec<{}>", t.rust()),
            Self::Opt(t) => format!("Option<{}>", t.rust()),
            Self::Json => "serde_json::Value".into(),
        }
    }

    pub fn contains_json(&self) -> bool {
        match self {
            Self::Json => true,
            Self::Vec(t) | Self::Opt(t) => t.contains_json(),
            _ => false,
        }
    }
}

fn is_string_literal(ts: &str) -> bool {
    let ts = ts.trim();
    ts.len() >= 2
        && ((ts.starts_with('\'') && ts.ends_with('\''))
            || (ts.starts_with('"') && ts.ends_with('"')))
}

fn balanced(ts: &str) -> bool {
    let mut depth = 0i32;
    for c in ts.chars() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

/// Splits on `sep` outside brackets.
fn split_top(ts: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in ts.char_indices() {
        match c {
            '<' | '(' | '[' | '{' => depth += 1,
            '>' | ')' | ']' | '}' => depth -= 1,
            c if c == sep && depth == 0 => {
                out.push(ts[start..i].trim());
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(ts[start..].trim());
    out.retain(|p| !p.is_empty());
    out
}

/// A parameter's type for the port, rest and optional parameters included.
pub fn param_type(p: &crate::ts::exports::Param) -> RType {
    let base = RType::from_ts(&p.ts_type);
    let base = if p.rest {
        match base {
            RType::Vec(_) => base,
            other => RType::Vec(Box::new(other)),
        }
    } else {
        base
    };
    if p.optional && !matches!(base, RType::Opt(_)) {
        RType::Opt(Box::new(base))
    } else {
        base
    }
}

pub fn snake_case(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev = i.checked_sub(1).map(|j| chars[j]);
            let next = chars.get(i + 1);
            let boundary = prev.is_some_and(|p| p.is_lowercase() || p.is_ascii_digit())
                || (prev.is_some_and(char::is_uppercase) && next.is_some_and(|n| n.is_lowercase()));
            if boundary {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else if c.is_alphanumeric() {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    const KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
        "where", "while", "async", "await", "dyn", "gen",
    ];
    if KEYWORDS.contains(&out.as_str()) {
        format!("r#{out}")
    } else {
        out
    }
}

/// Types declared in `file` and in the files it imports relatively (one level deep).
pub fn types_in_scope(root: &Path, file: &str, text: &str) -> Vec<LocalType> {
    let mut types = Vec::new();
    let mut seen = std::collections::HashSet::new();
    collect_types(root, file, text, 0, &mut seen, &mut types);
    types
}

/// Types of a file and of the files it imports or re-exports, a few levels deep
/// (`./types` -> `types/index.ts` -> `export * from './host'`).
fn collect_types(
    root: &Path,
    file: &str,
    text: &str,
    depth: usize,
    seen: &mut std::collections::HashSet<String>,
    types: &mut Vec<LocalType>,
) {
    if depth > 4 || !seen.insert(file.to_string()) {
        return;
    }
    types.extend(local_types(file, text));
    let dir = file.rsplit_once('/').map_or("", |(d, _)| d);
    for import in crate::ts::scan::scan(file, text).imports {
        let spec = import.specifier;
        // Workspace packages (`@x18n/core`) are followed to their source entry.
        let base = if spec.starts_with('.') {
            let joined = if dir.is_empty() {
                spec.clone()
            } else {
                format!("{dir}/{spec}")
            };
            match normalize_rel(&joined) {
                Some(base) => base,
                None => continue,
            }
        } else {
            let name = crate::ts::deps::package_name(&spec);
            let Some(pkg_dir) = workspace_packages(root).get(name).cloned() else {
                continue;
            };
            let sub = spec[name.len()..].trim_start_matches('/');
            let src = if root.join(&pkg_dir).join("src").is_dir() {
                format!("{pkg_dir}/src")
            } else {
                pkg_dir
            };
            let src = src.trim_start_matches('/').to_string();
            if sub.is_empty() {
                format!("{src}/index")
            } else {
                format!("{src}/{sub}")
            }
        };
        let stem = base.trim_end_matches(".js");
        let candidates = [
            format!("{stem}.ts"),
            format!("{stem}.mts"),
            format!("{stem}.d.ts"),
            format!("{stem}/index.ts"),
            base.clone(),
        ];
        if let Some(found) = candidates.iter().find(|c| root.join(c).is_file())
            && let Ok(imported) = std::fs::read_to_string(root.join(found))
        {
            collect_types(root, found, &imported, depth + 1, seen, types);
        }
    }
}

/// Workspace package names and their root-relative folders, read once per root.
fn workspace_packages(root: &Path) -> std::collections::HashMap<String, String> {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<
        Mutex<std::collections::HashMap<PathBuf, std::collections::HashMap<String, String>>>,
    > = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache
        .entry(root.to_path_buf())
        .or_insert_with(|| {
            crate::ts::discover::discover(root)
                .map(|ws| ws.packages.into_iter().map(|p| (p.name, p.dir)).collect())
                .unwrap_or_default()
        })
        .clone()
}

/// Collapses `.` and `..` in a root-relative path.
fn normalize_rel(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// `f<T extends string>(s: T)` takes a string: type parameters with a simple constraint are
/// replaced by it. The rest stay listed, and keep the function in TS.
pub fn substitute_constraints(f: &mut ExportedFn, types: &[LocalType]) {
    let mut remaining = Vec::new();
    for tp in std::mem::take(&mut f.type_params) {
        let Some((name, constraint)) = tp.split_once(" extends ") else {
            remaining.push(tp);
            continue;
        };
        let constraint = resolve_aliases(constraint, types);
        // Data constraints (`string`, `string | readonly string[]`) are substituted; anything that
        // could hold a function keeps the type parameter, and the function stays in TS.
        let open_ended = constraint.contains("=>")
            || constraint
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|w| {
                    matches!(w, "Function" | "object" | "unknown" | "any")
                        || has_functions(w, types, &mut Vec::new())
                });
        if open_ended {
            remaining.push(name.to_string());
            continue;
        }
        let replacement = format!("({constraint})");
        for p in &mut f.params {
            p.ts_type = replace_word(&p.ts_type, name, &replacement);
        }
        f.returns = replace_word(&f.returns, name, &replacement);
    }
    f.type_params = remaining;
}

/// Turns the type parameters named in `names` into `unknown` (a JSON value on the Rust side).
pub fn substitute_json(f: &mut ExportedFn, names: &[String]) {
    let mut remaining = Vec::new();
    for tp in std::mem::take(&mut f.type_params) {
        let name = tp
            .split_once(" extends ")
            .map_or(tp.as_str(), |(n, _)| n)
            .trim();
        if !names.iter().any(|n| n == name) {
            remaining.push(tp);
            continue;
        }
        for p in &mut f.params {
            p.ts_type = replace_word(&p.ts_type, name, "unknown");
        }
        f.returns = replace_word(&f.returns, name, "unknown");
    }
    f.type_params = remaining;
}

fn replace_word(text: &str, word: &str, with: &str) -> String {
    let mut out = String::new();
    let mut current = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_alphanumeric() || c == '_' || c == '$' {
            current.push(c);
        } else {
            out.push_str(if current == word { with } else { &current });
            current.clear();
            out.push(c);
        }
    }
    out.pop();
    out
}

/// Replaces simple type aliases (`type KeyString = string`) by what they stand for.
pub fn resolve_aliases(ts: &str, types: &[LocalType]) -> String {
    let mut out = ts.to_string();
    for _ in 0..4 {
        let mut changed = false;
        let mut next = String::with_capacity(out.len());
        let mut word = String::new();
        let flush = |word: &mut String, next: &mut String, changed: &mut bool| {
            if let Some(t) = types.iter().find(|t| t.alias && t.name == *word) {
                let text = t.text.trim();
                if text.contains('|') {
                    next.push('(');
                    next.push_str(text);
                    next.push(')');
                } else {
                    next.push_str(text);
                }
                *changed = true;
            } else {
                next.push_str(word);
            }
            word.clear();
        };
        for c in out.chars() {
            if c.is_alphanumeric() || c == '_' || c == '$' {
                word.push(c);
            } else {
                flush(&mut word, &mut next, &mut changed);
                next.push(c);
            }
        }
        flush(&mut word, &mut next, &mut changed);
        out = next;
        if !changed {
            break;
        }
    }
    out
}

fn needs_js(api: &codegen::PortApi) -> bool {
    api.needs_json()
}

/// The reason given with `--keep name` or `--keep "name: reason"`, if `name` is kept.
fn keep_reason(keep: &[String], name: &str) -> Option<String> {
    keep.iter().find_map(|k| {
        let (kept, reason) = k
            .split_once(':')
            .map_or((k.as_str(), ""), |(n, r)| (n.trim(), r.trim()));
        (kept == name).then(|| {
            if reason.is_empty() {
                "kept on request".to_string()
            } else {
                reason.to_string()
            }
        })
    })
}

/// Formats the generated Rust, best effort: the gate checks formatting.
fn format_rust(crate_dir: &Path) {
    let files: Vec<PathBuf> = std::fs::read_dir(crate_dir.join("src"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    let _ = std::process::Command::new("rustfmt")
        .args(["--edition", "2024"])
        .args(&files)
        .output();
}

/// Formats a generated Go wrapper with `gofmt`, best effort.
fn format_wrapper(crate_dir: &Path) {
    if crate_dir.join("go").is_dir() {
        let _ = std::process::Command::new("gofmt")
            .arg("-w")
            .arg(crate_dir.join("go"))
            .output();
    }
}

/// `lib.rs` of a JS crate: the bindings, and the binary wire when functions use it.
fn js_lib_rs(m: &Manifest, api: &codegen::PortApi, modules: &codegen::Modules) -> Result<String> {
    let mut lib = with_allocator(m, codegen::lib_rs(m, api, modules));
    if m.kind != Kind::Pyo3 && !m.batches.is_empty() {
        lib.push_str(&crate::codegen_batch::js_rs(m, api).map_err(anyhow::Error::msg)?);
    }
    if !crate::codegen_tswire::uses_wire(m, api) {
        return Ok(lib);
    }
    Ok(lib.replacen(
        "pub mod port;",
        "pub mod port;\npub mod wire;\n\n#[allow(unused_imports)]\nuse serde_json::Value;",
        1,
    ) + &crate::codegen_tswire::napi_rs(m, api))
}

fn parse_batches(specs: &[String]) -> Result<Vec<crate::codegen_batch::Batch>> {
    specs
        .iter()
        .map(|s| {
            crate::codegen_batch::Batch::parse(s)
                .with_context(|| format!("`--batch {s}`: write it as function:param"))
        })
        .collect()
}

/// Adds batch classes to a bridged crate, then regenerates its bindings.
pub fn add_batches(crate_dir: &Path, specs: &[String]) -> Result<Refreshed> {
    let path = crate_dir.join(MANIFEST);
    let before = std::fs::read_to_string(&path).with_context(|| format!("{}", path.display()))?;
    let mut manifest = Manifest::load(crate_dir)?;
    for b in parse_batches(specs)? {
        if !manifest.batches.contains(&b) {
            manifest.batches.push(b);
        }
    }
    if let Some(written) = manifest.root_written.clone() {
        manifest.root = written;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&manifest)? + "\n")?;
    // A batch the port cannot serve leaves the crate as it was.
    refresh(crate_dir).inspect_err(|_| {
        let _ = std::fs::write(&path, &before);
    })
}

/// Cargo.toml with the dependencies and library types today's bindings need, for crates
/// bridged by an older rust-anything; versions and features already set are kept.
fn with_template_deps(cargo: &str, template: &str) -> Result<String> {
    let mut doc: toml_edit::DocumentMut = cargo.parse().context("Cargo.toml")?;
    let template: toml_edit::DocumentMut = template.parse().context("Cargo.toml template")?;
    if let Some(want) = template.get("dependencies").and_then(|d| d.as_table()) {
        let deps = doc["dependencies"].or_insert(toml_edit::table());
        for (name, item) in want {
            if deps.get(name).is_none() {
                deps[name] = item.clone();
            }
        }
    }
    if let Some(want) = template.get("features").and_then(|d| d.as_table()) {
        let features = doc["features"].or_insert(toml_edit::table());
        for (name, item) in want {
            if features.get(name).is_none() {
                features[name] = item.clone();
            }
        }
    }
    if let Some(kinds) = template.get("lib").and_then(|l| l.get("crate-type")) {
        doc["lib"]["crate-type"] = kinds.clone();
    }
    Ok(doc.to_string())
}

/// The global allocator of a crate, if it sets one: crates with batches allocate from all cores
/// at once (the system allocator serializes them: 1.6x on 4 cores instead of 3.2x). mimalloc
/// leaves wasm alone, which runs on one thread; rusty_alloc also replaces Rust's wasm allocator.
/// Only Rust code uses this heap; every buffer that crosses the bridge is copied or freed by Rust.
fn global_allocator(m: &Manifest) -> Option<Allocator> {
    if m.batches.is_empty() {
        return None;
    }
    match m.allocator {
        Allocator::Mimalloc if m.kind == Kind::Wasm => None,
        a => Some(a),
    }
}

/// `lib.rs` with the [`global_allocator`].
fn with_allocator(m: &Manifest, lib: String) -> String {
    let line = match global_allocator(m) {
        None => return lib,
        Some(Allocator::Mimalloc) => {
            "/// Batches allocate on all cores at once; mimalloc keeps that from serializing.\n\
             #[global_allocator]\nstatic GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;\n"
        }
        Some(Allocator::RustyAlloc) => {
            "/// Batches allocate on all cores at once; rusty_alloc keeps that from serializing.\n\
             #[global_allocator]\n\
             static GLOBAL: rusty_alloc_api::RustyAlloc = rusty_alloc_api::RustyAlloc;\n"
        }
    };
    lib.replacen("pub mod port;", &format!("pub mod port;\n\n{line}"), 1)
}

/// Cargo.toml with rayon (and the global allocator) when batches run on all cores.
fn with_rayon(m: &Manifest, cargo: String) -> Result<String> {
    let mut doc: toml_edit::DocumentMut = cargo.parse().context("Cargo.toml")?;
    let deps = doc["dependencies"].or_insert(toml_edit::table());
    // Batch methods take their other parameters as napi values; structured ones need serde.
    if m.kind == Kind::Napi
        && !m.batches.is_empty()
        && let Some(napi) = deps.get_mut("napi")
    {
        let mut table = toml_edit::InlineTable::new();
        match napi {
            toml_edit::Item::Value(toml_edit::Value::String(v)) => {
                table.insert("version", v.value().clone().into());
            }
            toml_edit::Item::Value(toml_edit::Value::InlineTable(t)) => table = t.clone(),
            _ => {}
        }
        let mut features = table
            .get("features")
            .and_then(|f| f.as_array())
            .cloned()
            .unwrap_or_default();
        if !features.iter().any(|f| f.as_str() == Some("serde-json")) {
            features.push("serde-json");
        }
        table.insert("features", toml_edit::Value::Array(features));
        *napi = toml_edit::value(table);
    }
    if !crate::codegen_batch::needs_rayon(m) {
        return Ok(doc.to_string());
    }
    if deps.get("rayon").is_none() {
        deps["rayon"] = toml_edit::value("1");
    }
    let allocator = match global_allocator(m) {
        Some(Allocator::Mimalloc) => Some(("mimalloc", "0.1")),
        Some(Allocator::RustyAlloc) => Some(("rusty_alloc-api", "2.6")),
        None => None,
    };
    if let Some((name, version)) = allocator
        && deps.get(name).is_none()
    {
        deps[name] = toml_edit::value(version);
    }
    Ok(doc.to_string())
}

/// The language-side loader: `index.cjs` for JS, the wrapper package for Python.
fn loader_file(m: &Manifest, api: &codegen::PortApi) -> (String, String) {
    match m.kind {
        Kind::Pyo3 => (
            format!("python/{}/__init__.py", m.lib_name()),
            crate::codegen_py::package_init(m, api),
        ),
        _ => (
            "index.cjs".into(),
            codegen::index_cjs(m, api)
                + &crate::codegen_tswire::index_cjs(m, api)
                + &crate::codegen_batch::index_cjs(m, api)
                + &codegen::guard_cjs(api, &crate::codegen_tswire::wired(m, api)),
        ),
    }
}

/// A port function whose doc line does not name an export (`/// Port of \`Integer.toString\``)
/// takes the name of the export it was generated for, matched by its Rust name.
pub fn match_export_names(m: &Manifest, api: &mut codegen::PortApi) {
    for f in &mut api.functions {
        if m.exports.iter().any(|e| e.name == f.js_name) {
            continue;
        }
        let rust = f.rust_name.trim_start_matches("r#");
        if let Some(e) = m
            .exports
            .iter()
            .find(|e| snake_case(&e.name).trim_start_matches("r#") == rust)
        {
            f.js_name = e.name.clone();
        }
    }
}

/// What `refresh` rewrote.
#[derive(Debug, Serialize)]
pub struct Refreshed {
    pub functions: Vec<String>,
    pub files: Vec<String>,
}

/// Regenerates the binding layer (`lib.rs`, `index.cjs`, `js.rs`) from the signatures in
/// `port.rs`, and adds the dependencies they need to Cargo.toml. Never touches `port.rs`.
pub fn refresh(crate_dir: &Path) -> Result<Refreshed> {
    let manifest = Manifest::load(crate_dir)?;
    if manifest.kind == Kind::CAbi {
        let port = std::fs::read_to_string(crate_dir.join("src/port.rs"))
            .context("reading src/port.rs")?;
        let mut files = c_bindings(&manifest, &port);
        let cargo =
            std::fs::read_to_string(crate_dir.join("Cargo.toml")).context("reading Cargo.toml")?;
        files.push((
            "Cargo.toml".into(),
            with_template_deps(&cargo, &crate::codegen_c::cargo_toml(&manifest))?,
        ));
        write_files(crate_dir, &files)?;
        format_rust(crate_dir);
        return Ok(Refreshed {
            functions: manifest.exports.iter().map(|f| f.name.clone()).collect(),
            files: files.into_iter().map(|(p, _)| p).collect(),
        });
    }
    if manifest.kind == Kind::Ffi {
        let port = std::fs::read_to_string(crate_dir.join("src/port.rs"))
            .context("reading src/port.rs")?;
        let mut api = codegen::parse_port(&port);
        match_export_names(&manifest, &mut api);
        let mut files = ffi_bindings(&manifest, &api)?;
        let cargo =
            std::fs::read_to_string(crate_dir.join("Cargo.toml")).context("reading Cargo.toml")?;
        let template = if manifest.language == crate::lang::Lang::Ruby {
            crate::codegen_ruby::cargo_toml(&manifest)
        } else {
            crate::codegen_ffi::cargo_toml(&manifest)
        };
        let cargo = with_template_deps(&cargo, &template)?;
        files.push(("Cargo.toml".into(), with_rayon(&manifest, cargo)?));
        write_files(crate_dir, &files)?;
        format_rust(crate_dir);
        format_wrapper(crate_dir);
        return Ok(Refreshed {
            functions: api.functions.iter().map(|f| f.js_name.clone()).collect(),
            files: files.into_iter().map(|(p, _)| p).collect(),
        });
    }
    let port =
        std::fs::read_to_string(crate_dir.join("src/port.rs")).context("reading src/port.rs")?;
    let mut fns = codegen::parse_port(&port);
    match_export_names(&manifest, &mut fns);
    if fns.functions.is_empty() && fns.classes.is_empty() {
        bail!("src/port.rs has no `pub fn` or ported class: nothing to bind");
    }
    let modules = codegen::Modules {
        utf16: crate_dir.join("src/utf16.rs").exists(),
        js: needs_js(&fns) || crate_dir.join("src/js.rs").exists(),
    };
    let mut files: Vec<(String, String)> = vec![
        ("src/lib.rs".into(), js_lib_rs(&manifest, &fns, &modules)?),
        loader_file(&manifest, &fns),
    ];
    if modules.js {
        files.push(("src/js.rs".into(), codegen::JS_RS.to_string()));
    }
    if crate::codegen_tswire::uses_wire(&manifest, &fns) {
        files.push(("src/wire.rs".into(), crate::wire::WIRE_RS.into()));
    }
    let extension = manifest.kind == Kind::Pyo3
        || (manifest.kind == Kind::Ffi && manifest.language == crate::lang::Lang::Ruby);
    if extension && !crate_dir.join("build.rs").exists() {
        files.push(("build.rs".into(), DYNAMIC_LOOKUP_BUILD_RS.into()));
    }
    if manifest.kind == Kind::Pyo3 && fns.needs_json() {
        files.push(("src/pyconv.rs".into(), crate::codegen_py::PYCONV_RS.into()));
        files.push(("src/pynum.rs".into(), crate::codegen_py::PYNUM_RS.into()));
    }
    let cargo_path = crate_dir.join("Cargo.toml");
    let cargo = std::fs::read_to_string(&cargo_path).context("reading Cargo.toml")?;
    let cargo = codegen::ensure_deps(&cargo, manifest.kind, &fns).map_err(anyhow::Error::msg)?;
    let cargo = with_rayon(&manifest, cargo)?;
    files.push(("Cargo.toml".into(), cargo));
    for (path, text) in &files {
        let target = crate_dir.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, text).with_context(|| format!("writing {path}"))?;
    }
    format_rust(crate_dir);
    Ok(Refreshed {
        functions: fns
            .functions
            .iter()
            .map(|f| f.js_name.clone())
            .chain(fns.classes.iter().map(|c| format!("class {}", c.js_name)))
            .collect(),
        files: files.into_iter().map(|(p, _)| p).collect(),
    })
}

/// `build.rs` for Python and Ruby extensions: on macOS the library (and its test binary) links
/// against symbols the interpreter provides only at load time.
pub(crate) const DYNAMIC_LOOKUP_BUILD_RS: &str = r#"fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");
    }
}
"#;

pub fn bridge(opts: &Options) -> Result<Outcome> {
    let root = opts
        .root
        .canonical()
        .with_context(|| format!("resolving {}", opts.root.display()))?;
    let lang = opts
        .lang
        .unwrap_or_else(|| crate::lang::Lang::detect(&root));
    let report = analyze::analyze_lang(&root, lang)?;
    let wanted = opts.unit.trim_start_matches("./");
    let whole = report
        .units
        .iter()
        .find(|u| u.id == wanted || u.id.trim_end_matches('/') == wanted.trim_end_matches('/'));
    let single = || {
        report
            .units
            .iter()
            .find(|u| u.files.iter().any(|f| f == wanted))
            .map(|u| {
                let mut one = u.clone();
                one.id = wanted.to_string();
                one.files = vec![wanted.to_string()];
                one
            })
    };
    let unit = match whole.cloned().or_else(single) {
        Some(unit) => unit,
        None => bail!(
            "no unit or source file `{wanted}`; `rust-anything analyze` lists unit ids, and any \
             source file inside a unit can be bridged on its own"
        ),
    };
    let unit = &unit;
    if matches!(unit.status, Status::Keep | Status::Blocked) && !opts.force {
        bail!(
            "unit `{}` is {:?}: {}. Pass --force to bridge it anyway.",
            unit.id,
            unit.status,
            unit.notes.first().map_or("", String::as_str)
        );
    }
    if unit.bridge == Bridge::PhpFfi && !opts.batch.is_empty() {
        bail!("`--batch` is not available for PHP units yet");
    }
    if unit.bridge == Bridge::DotnetFfi && !opts.batch.is_empty() {
        bail!("`--batch` is not available for C# units yet");
    }
    let kind = opts.kind.unwrap_or(match unit.bridge {
        Bridge::WasmBindgen => Kind::Wasm,
        Bridge::Pyo3 => Kind::Pyo3,
        Bridge::CAbi => Kind::CAbi,
        Bridge::Cgo | Bridge::Fiddle | Bridge::Jni | Bridge::PhpFfi | Bridge::DotnetFfi => {
            Kind::Ffi
        }
        _ => Kind::Napi,
    });

    let mut exports = Vec::new();
    let mut classes = Vec::new();
    let mut js_only = Vec::new();
    if lang == crate::lang::Lang::C {
        crate::lang::c::use_typedefs(&root);
    }
    if lang == crate::lang::Lang::Csharp {
        crate::lang::csharp::use_shapes(&root);
    }
    for file in &unit.files {
        if file.ends_with(".vue") || file.ends_with(".svelte") || file.ends_with(".astro") {
            continue;
        }
        let text =
            std::fs::read_to_string(root.join(file)).with_context(|| format!("reading {file}"))?;
        if lang != crate::lang::Lang::Ts {
            let (functions, foreign_classes) = crate::lang::exports(lang, file, &text);
            for f in functions {
                match keep_reason(&opts.keep, &f.name).or_else(|| js_only_reason(&f, &[])) {
                    Some(reason) => js_only.push(JsOnly {
                        name: f.name,
                        file: f.file,
                        line: f.line,
                        reason,
                    }),
                    None => exports.push(f),
                }
            }
            for class in foreign_classes {
                match keep_reason(&opts.keep, &class.name)
                    .or_else(|| class_reason(&class, &[], false))
                {
                    Some(reason) => js_only.push(JsOnly {
                        name: class.name,
                        file: class.file,
                        line: class.line,
                        reason: format!("class: {reason}"),
                    }),
                    None => classes.push(class),
                }
            }
            continue;
        }
        let types = types_in_scope(&root, file, &text);
        for mut class in exported_classes(file, &text) {
            let kept = keep_reason(&opts.keep, &class.name);
            match kept.or_else(|| class_reason(&class, &types, true)) {
                Some(reason) => js_only.push(JsOnly {
                    name: class.name,
                    file: class.file,
                    line: class.line,
                    reason: format!("class: {reason}"),
                }),
                None => {
                    for p in &mut class.constructor {
                        p.ts_type = resolve_aliases(&p.ts_type, &types);
                    }
                    for m in &mut class.methods {
                        for p in &mut m.params {
                            p.ts_type = resolve_aliases(&p.ts_type, &types);
                        }
                        m.returns = resolve_aliases(&m.returns, &types);
                    }
                    classes.push(class);
                }
            }
        }
        for mut f in ts_functions(&root, file, &text, &types) {
            substitute_json(&mut f, &opts.as_json);
            let kept = keep_reason(&opts.keep, &f.name);
            match kept.or_else(|| js_only_reason(&f, &types)) {
                Some(reason) => js_only.push(JsOnly {
                    name: f.name,
                    file: f.file,
                    line: f.line,
                    reason,
                }),
                None => exports.push(f),
            }
        }
    }
    let mut defined: Vec<(&str, String)> = exports
        .iter()
        .map(|f| (f.name.as_str(), format!("{}:{}", f.file, f.line)))
        .chain(
            classes
                .iter()
                .map(|c| (c.name.as_str(), format!("{}:{}", c.file, c.line))),
        )
        .chain(
            js_only
                .iter()
                .map(|j| (j.name.as_str(), format!("{}:{}", j.file, j.line))),
        )
        .collect();
    defined.sort();
    if let Some(pair) = defined.windows(2).find(|w| w[0].0 == w[1].0) {
        bail!(
            "`{}` is defined twice in `{}` ({} and {}), and the bridge names every export by \
             itself, so one would silently replace the other. Bridge the module you want on its \
             own (`rust-anything bridge <root> <that file>`), or rename one of them",
            pair[0].0,
            unit.id,
            pair[0].1,
            pair[1].1
        );
    }
    if exports.is_empty() && classes.is_empty() && !js_only.is_empty() {
        bail!(
            "every export of `{}` must stay in {}: {}",
            unit.id,
            lang.name(),
            js_only
                .iter()
                .map(|j| format!("{} ({})", j.name, j.reason))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if exports.is_empty() && classes.is_empty() {
        bail!(
            "unit `{}` exports no functions; bridge a unit with an exported function API",
            unit.id
        );
    }

    let stem = unit
        .id
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("unit")
        .split('.')
        .next()
        .unwrap_or("unit");
    let crate_name = opts.name.clone().unwrap_or_else(|| {
        let clean: String = stem
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        format!("{}-rs", clean.trim_matches('-'))
    });
    let crate_dir = opts
        .out
        .clone()
        .unwrap_or_else(|| root.join("rust").join(&crate_name));
    if crate_dir.exists() && std::fs::read_dir(&crate_dir)?.next().is_some() {
        bail!(
            "{} already exists and is not empty; rust-anything never overwrites a port in progress",
            crate_dir.display()
        );
    }

    // UTF-16 helpers matter for JS strings only; Python indexes code points, like `chars()`.
    let utf16 =
        lang == crate::lang::Lang::Ts && unit.risks.iter().any(|r| r.risk == Risk::StringOffsets);
    let manifest = Manifest {
        schema: SCHEMA.into(),
        root: root.clone(),
        root_written: None,
        unit: unit.id.clone(),
        language: lang,
        kind,
        crate_name: crate_name.clone(),
        exports: exports.clone(),
        classes: classes.clone(),
        js_only: js_only.clone(),
        batches: parse_batches(&opts.batch)?,
        allocator: opts.allocator,
        float_tolerance: None,
    };
    if kind == Kind::CAbi {
        return bridge_c(&manifest, &crate_dir, &js_only);
    }
    if kind == Kind::Ffi {
        return bridge_ffi(&manifest, &crate_dir, &js_only);
    }
    let port = codegen::port_rs(&manifest);
    let fns = codegen::parse_port(&port);
    let modules = codegen::Modules {
        utf16,
        js: needs_js(&fns),
    };
    let mut files: Vec<(String, String)> = vec![
        (
            "Cargo.toml".into(),
            with_rayon(&manifest, codegen::cargo_toml(&manifest, &fns))?,
        ),
        ("src/lib.rs".into(), js_lib_rs(&manifest, &fns, &modules)?),
        ("src/port.rs".into(), port),
        loader_file(&manifest, &fns),
        (
            ".gitignore".into(),
            "target/\n*.node\npkg/\n.rust-anything/\n*.so\n*.pyd\n__pycache__/\n".into(),
        ),
        ("README.md".into(), readme(&manifest)),
        (
            MANIFEST.into(),
            serde_json::to_string_pretty(&manifest)? + "\n",
        ),
    ];
    match kind {
        Kind::Napi => files.push((
            "build.rs".into(),
            "fn main() {\n    napi_build::setup();\n}\n".into(),
        )),
        Kind::Pyo3 => files.push(("build.rs".into(), DYNAMIC_LOOKUP_BUILD_RS.into())),
        _ => {}
    }
    if utf16 {
        files.push(("src/utf16.rs".into(), UTF16_RS.into()));
    }
    if modules.js {
        files.push(("src/js.rs".into(), codegen::JS_RS.into()));
    }
    if crate::codegen_tswire::uses_wire(&manifest, &fns) {
        files.push(("src/wire.rs".into(), crate::wire::WIRE_RS.into()));
    }
    if manifest.kind == Kind::Pyo3 && fns.needs_json() {
        files.push(("src/pyconv.rs".into(), crate::codegen_py::PYCONV_RS.into()));
        files.push(("src/pynum.rs".into(), crate::codegen_py::PYNUM_RS.into()));
    }
    for (path, text) in &files {
        let target = crate_dir.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, text).with_context(|| format!("writing {}", target.display()))?;
    }

    format_rust(&crate_dir);
    let mut notes = vec![format!(
        "Port each function in src/port.rs, then: rust-anything record {0} && rust-anything build {0} && rust-anything diff {0}",
        crate_dir.display()
    )];
    if utf16 {
        notes.push("The unit works with string offsets: src/utf16.rs has UTF-16 helpers that match JS indexing.".into());
    }
    for j in &js_only {
        notes.push(format!(
            "`{}` stays in {}: {}. The shadow module re-exports the original.",
            j.name,
            lang.name(),
            j.reason
        ));
    }
    if exports.iter().any(codegen::is_async) {
        notes.push(
            "Async TS functions are `async fn` in the port; the bindings return promises.".into(),
        );
    }
    Ok(Outcome {
        crate_dir,
        kind,
        files: files.into_iter().map(|(p, _)| p).collect(),
        exports: exports
            .iter()
            .map(|f| f.name.clone())
            .chain(classes.iter().map(|c| format!("class {}", c.name)))
            .collect(),
        notes,
    })
}

fn write_files(crate_dir: &Path, files: &[(String, String)]) -> Result<()> {
    for (path, text) in files {
        let target = crate_dir.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, text).with_context(|| format!("writing {}", target.display()))?;
    }
    Ok(())
}

/// The generated C-side files: the `extern "C"` layer and its header. C signatures are fixed
/// by the original, so they come from the manifest, not from `port.rs`.
fn c_bindings(m: &Manifest, port: &str) -> Vec<(String, String)> {
    vec![
        ("src/lib.rs".into(), crate::codegen_c::lib_rs(m, port)),
        (
            format!("include/{}.h", m.lib_name()),
            crate::codegen_c::header(m),
        ),
    ]
}

/// The generated layers of a JSON-dispatcher crate: `lib.rs` and the language-side wrapper.
fn ffi_bindings(m: &Manifest, api: &codegen::PortApi) -> Result<Vec<(String, String)>> {
    let mut files = if m.language == crate::lang::Lang::Ruby {
        vec![
            (
                "src/lib.rs".to_string(),
                with_allocator(
                    m,
                    crate::codegen_ruby::lib_rs(m, api).map_err(anyhow::Error::msg)?,
                ),
            ),
            (
                "src/rbconv.rs".to_string(),
                crate::codegen_ruby::RBCONV_RS.to_string(),
            ),
            ("build.rs".to_string(), DYNAMIC_LOOKUP_BUILD_RS.to_string()),
        ]
    } else {
        let mut lib = crate::codegen_ffi::lib_rs(m, api);
        if matches!(
            m.language,
            crate::lang::Lang::Java | crate::lang::Lang::Kotlin
        ) {
            lib.push_str(&crate::codegen_java::typed_rs(m, api));
        }
        if m.language == crate::lang::Lang::Go {
            lib.push_str(&crate::codegen_go::typed_rs(m, api));
            lib.push_str(&crate::codegen_go::wire_rs(m, api));
            lib.push_str(&crate::codegen_batch_native::go_rs(m, api).map_err(anyhow::Error::msg)?);
        }
        if matches!(
            m.language,
            crate::lang::Lang::Java | crate::lang::Lang::Kotlin
        ) {
            lib.push_str(&crate::codegen_java::wire_rs(m, api));
            lib.push_str(
                &crate::codegen_batch_native::java_rs(m, api).map_err(anyhow::Error::msg)?,
            );
        }
        if matches!(
            m.language,
            crate::lang::Lang::Go | crate::lang::Lang::Java | crate::lang::Lang::Kotlin
        ) {
            lib = lib.replacen("pub mod port;", "pub mod port;\npub mod wire;", 1);
        }
        let mut files = vec![("src/lib.rs".to_string(), with_allocator(m, lib))];
        if matches!(
            m.language,
            crate::lang::Lang::Go | crate::lang::Lang::Java | crate::lang::Lang::Kotlin
        ) {
            files.push(("src/wire.rs".to_string(), crate::wire::WIRE_RS.to_string()));
        }
        files
    };
    match m.language {
        crate::lang::Lang::Go => {
            // The wrapper package takes its name and types from the original sources: without
            // them it would be written under a wrong name beside the old one.
            if crate::codegen_go::unit_sources(m).is_empty() {
                bail!(
                    "cannot read the Go sources of `{}` under {}: is `root` in {MANIFEST} right?",
                    m.unit,
                    m.root.display()
                );
            }
            files.push(("go/go.mod".into(), crate::harness_go::wrapper_go_mod(m)));
            files.push((
                format!("go/{}.go", crate::codegen_go::package_name(m)),
                crate::codegen_go::wrapper(m, api),
            ));
        }
        crate::lang::Lang::Ruby => {
            files.push((
                crate::harness_ruby::wrapper_path(m),
                crate::codegen_ruby::wrapper(m, api).map_err(anyhow::Error::msg)?,
            ));
        }
        crate::lang::Lang::Php => {
            files.push((
                crate::codegen_php::wrapper_path(m),
                crate::codegen_php::wrapper(m),
            ));
        }
        crate::lang::Lang::Csharp => {
            files.push((
                crate::codegen_csharp::wrapper_path(m),
                crate::codegen_csharp::wrapper(m),
            ));
        }
        crate::lang::Lang::Java | crate::lang::Lang::Kotlin => {
            files.extend(crate::codegen_java::wrappers(m, api));
            files.extend(
                crate::codegen_batch_native::java_files(m, api).map_err(anyhow::Error::msg)?,
            );
            files.push((
                format!(
                    "java/rustanything/{}.java",
                    crate::codegen_ffi::java_native_class(m)
                ),
                crate::codegen_java::native_class(m, api),
            ));
            files.push((
                "java/rustanything/RustAnythingJson.java".into(),
                crate::codegen_java::JSON_JAVA.into(),
            ));
            files.push((
                "java/rustanything/RustAnythingWire.java".into(),
                crate::codegen_java::WIRE_JAVA.into(),
            ));
            files.push((
                "java/rustanything/RustAnythingError.java".into(),
                crate::codegen_java::ERROR_JAVA.into(),
            ));
        }
        other => bail!("no wrapper generator for {}", other.name()),
    }
    Ok(files)
}

fn bridge_ffi(manifest: &Manifest, crate_dir: &Path, kept: &[JsOnly]) -> Result<Outcome> {
    let port = codegen::port_rs(manifest);
    let api = codegen::parse_port(&port);
    let mut files = vec![
        (
            "Cargo.toml".to_string(),
            with_rayon(
                manifest,
                if manifest.language == crate::lang::Lang::Ruby {
                    crate::codegen_ruby::cargo_toml(manifest)
                } else {
                    crate::codegen_ffi::cargo_toml(manifest)
                },
            )?,
        ),
        ("src/port.rs".into(), port),
        (
            ".gitignore".into(),
            "target/\n.rust-anything/\nruby/*.so\nruby/*.bundle\nphp/*.so\nphp/*.dylib\nphp/*.dll\ncs/*.so\ncs/*.dylib\ncs/*.dll\n".into(),
        ),
        ("README.md".into(), readme(manifest)),
        (
            MANIFEST.into(),
            serde_json::to_string_pretty(manifest)? + "\n",
        ),
    ];
    files.extend(ffi_bindings(manifest, &api)?);
    write_files(crate_dir, &files)?;
    format_rust(crate_dir);
    format_wrapper(crate_dir);
    let mut notes = vec![format!(
        "Port each function in src/port.rs, then: rust-anything record {0} && rust-anything build {0} && rust-anything diff {0}",
        crate_dir.display()
    )];
    for j in kept {
        notes.push(format!(
            "`{}` stays in {}: {}.",
            j.name,
            manifest.language.name(),
            j.reason
        ));
    }
    Ok(Outcome {
        crate_dir: crate_dir.to_path_buf(),
        kind: Kind::Ffi,
        files: files.into_iter().map(|(p, _)| p).collect(),
        exports: manifest.exports.iter().map(|f| f.name.clone()).collect(),
        notes,
    })
}

fn bridge_c(manifest: &Manifest, crate_dir: &Path, kept: &[JsOnly]) -> Result<Outcome> {
    let mut files = vec![
        (
            "Cargo.toml".to_string(),
            crate::codegen_c::cargo_toml(manifest),
        ),
        ("src/port.rs".into(), crate::codegen_c::port_rs(manifest)),
        (
            ".gitignore".into(),
            "target/\n.rust-anything/\nruby/*.so\nruby/*.bundle\nphp/*.so\nphp/*.dylib\nphp/*.dll\n".into(),
        ),
        ("README.md".into(), readme(manifest)),
        (
            MANIFEST.into(),
            serde_json::to_string_pretty(manifest)? + "\n",
        ),
    ];
    let port = crate::codegen_c::port_rs(manifest);
    files.extend(c_bindings(manifest, &port));
    write_files(crate_dir, &files)?;
    format_rust(crate_dir);
    let mut notes = vec![
        format!(
            "Port each function in src/port.rs, then: rust-anything record {0} && rust-anything build {0} && rust-anything diff {0}",
            crate_dir.display()
        ),
        format!(
            "Link target/release/lib{}.a (or the .so) instead of the C objects; include/{}.h has the prototypes.",
            manifest.lib_name(),
            manifest.lib_name()
        ),
    ];
    for j in kept {
        notes.push(format!("`{}` stays in C: {}.", j.name, j.reason));
    }
    Ok(Outcome {
        crate_dir: crate_dir.to_path_buf(),
        kind: Kind::CAbi,
        files: files.into_iter().map(|(p, _)| p).collect(),
        exports: manifest.exports.iter().map(|f| f.name.clone()).collect(),
        notes,
    })
}

fn readme(m: &Manifest) -> String {
    let functions: String = m
        .exports
        .iter()
        .map(|f| format!("- `{}` (`{}:{}`)\n", f.name, f.file, f.line))
        .collect();
    format!(
        "# {name}\n\nRust port of `{unit}`, bridged with {kind}. Generated by rust-anything.\n\n## Functions\n\n{functions}\n\
         ## Loop\n\n```sh\nrust-anything record .   # record what the TS code does\nrust-anything build .    # build the {kind} module\n\
         rust-anything diff .     # run the recorded cases against Rust\nrust-anything shadow .   # run both in the app, log mismatches\n\
         rust-anything gate .     # fmt, clippy, tests, unsafe, API, diff\n```\n",
        name = m.crate_name,
        unit = m.unit,
        kind = match m.kind {
            Kind::Napi => "napi-rs",
            Kind::Wasm => "wasm-bindgen",
            Kind::Pyo3 => "PyO3",
            Kind::CAbi => "the C ABI",
            Kind::Ffi => "a JSON dispatcher",
        },
    )
}

const UTF16_RS: &str = r#"//! JS string indexing in Rust.
//!
//! JS strings are indexed in UTF-16 code units; Rust `str` in UTF-8 bytes. A port that returns
//! or consumes offsets must use these helpers, or non-ASCII text gets wrong results silently.

#![forbid(unsafe_code)]

/// The UTF-16 code units of `s`, as JS sees the string.
pub fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// The string `units[start..end]` decodes to, like JS `slice` (lone surrogates become U+FFFD).
pub fn slice(units: &[u16], start: usize, end: usize) -> String {
    let end = end.min(units.len());
    let start = start.min(end);
    String::from_utf16_lossy(&units[start..end])
}

/// JS `haystack.indexOf(needle, from)`, in UTF-16 code units.
pub fn index_of(haystack: &[u16], needle: &[u16], from: usize) -> Option<usize> {
    let from = from.min(haystack.len());
    if needle.is_empty() {
        return Some(from);
    }
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|i| i + from)
}

/// JS `haystack.lastIndexOf(needle)`, in UTF-16 code units.
pub fn last_index_of(haystack: &[u16], needle: &[u16]) -> Option<usize> {
    if needle.is_empty() {
        return Some(haystack.len());
    }
    haystack.windows(needle.len()).rposition(|w| w == needle)
}

/// UTF-8 byte offset of a UTF-16 offset in `s`, for slicing a Rust `str`.
pub fn byte_offset(s: &str, utf16_offset: usize) -> usize {
    let mut seen = 0;
    for (byte, c) in s.char_indices() {
        if seen >= utf16_offset {
            return byte;
        }
        seen += c.len_utf16();
    }
    s.len()
}

/// UTF-16 offset of a UTF-8 byte offset in `s`, for returning ranges to JS.
pub fn utf16_offset(s: &str, byte_offset: usize) -> usize {
    s[..byte_offset.min(s.len())].encode_utf16().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_js_indexing() {
        let s = units("a👍b");
        assert_eq!(s.len(), 4);
        assert_eq!(index_of(&s, &units("b"), 0), Some(3));
        assert_eq!(last_index_of(&s, &units("a")), Some(0));
        assert_eq!(slice(&s, 3, 4), "b");
        assert_eq!(byte_offset("a👍b", 3), 5);
        assert_eq!(utf16_offset("a👍b", 5), 3);
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_relative_root_is_resolved_from_the_crate_and_kept_relative() {
        let base = std::env::temp_dir().join(format!("rust-anything-root-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let crate_dir = base.join("repo/rust/x-rs");
        std::fs::create_dir_all(&crate_dir).unwrap();
        std::fs::write(
            crate_dir.join(MANIFEST),
            r#"{"schema": "rust-anything/bridge@1", "root": "../..", "unit": "src/x.ts", "kind": "napi", "crate_name": "x-rs", "exports": []}"#,
        )
        .unwrap();
        let m = Manifest::load(&crate_dir).unwrap();
        assert_eq!(m.root, base.join("repo").canonical().unwrap());
        assert_eq!(m.root_written.as_deref(), Some(Path::new("../..")));
        // Written back, the file keeps the relative root.
        let mut back = m.clone();
        if let Some(w) = back.root_written.clone() {
            back.root = w;
        }
        assert!(
            serde_json::to_string(&back)
                .unwrap()
                .contains(r#""root":"../..""#)
        );
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn maps_ts_types() {
        assert_eq!(RType::from_ts("string").rust(), "String");
        assert_eq!(RType::from_ts("readonly string[]").rust(), "Vec<String>");
        assert_eq!(RType::from_ts("Array<number>").rust(), "Vec<f64>");
        assert_eq!(
            RType::from_ts("string | undefined").rust(),
            "Option<String>"
        );
        assert_eq!(RType::from_ts("'a' | 'b'").rust(), "String");
        assert_eq!(RType::from_ts("Promise<boolean>").rust(), "bool");
        assert_eq!(
            RType::from_ts("Map<string, number>").rust(),
            "serde_json::Value"
        );
        assert_eq!(
            RType::from_ts("string | number").rust(),
            "serde_json::Value"
        );
        assert_eq!(
            RType::from_ts("Record<string, string | number>").rust(),
            "serde_json::Value"
        );
        assert_eq!(
            RType::from_ts("(string | undefined)").rust(),
            "Option<String>"
        );
        assert_eq!(RType::from_ts("(a) | (b)").rust(), "serde_json::Value");
    }

    #[test]
    fn keeps_unbridgeable_exports_in_js() {
        let source = "export interface LineIndex {\n  readonly lineCount: number\n  position(offset: number): number\n}\n\
            export interface Range { start: number, end: number }\n\
            export function keyMatcher(globs: readonly string[]): (key: string) => boolean { return () => true }\n\
            export function globToRegExp(glob: string): RegExp { return /x/ }\n\
            export function createLineIndex(text: string): LineIndex { return null! }\n\
            export function* walk(tree: object) {}\n\
            export function sortTree(tree: object, compare: (a: string, b: string) => number) {}\n\
            export function contains(range: Range, offset: number): boolean { return true }\n";
        let types = local_types("a.ts", source);
        let reasons: Vec<(String, Option<String>)> = exported_functions("a.ts", source)
            .iter()
            .map(|f| (f.name.clone(), js_only_reason(f, &types)))
            .collect();
        let js_only: Vec<&str> = reasons
            .iter()
            .filter(|(_, r)| r.is_some())
            .map(|(n, _)| n.as_str())
            .collect();
        assert_eq!(
            js_only,
            [
                "keyMatcher",
                "globToRegExp",
                "createLineIndex",
                "walk",
                "sortTree"
            ]
        );
    }

    #[test]
    fn resolves_simple_aliases() {
        let types = local_types(
            "m.ts",
            "export type KeyString = string\nexport type Leaf = string | null\nexport type Map2<T> = T\n",
        );
        assert_eq!(
            resolve_aliases("readonly KeyString[]", &types),
            "readonly string[]"
        );
        assert_eq!(resolve_aliases("Leaf", &types), "(string | null)");
        assert_eq!(resolve_aliases("Map2<KeyString>", &types), "Map2<string>");
    }

    #[test]
    fn substitutes_simple_constraints() {
        let found = exported_functions(
            "a.ts",
            "export function up<T extends string>(s: T): Uppercase<T> { return s as any }\n\
             export function pick<T>(x: T): T { return x }\n\
             export function first<S extends readonly string[]>(xs: S, n?: number): string { return '' }\n",
        );
        let mut fns = found;
        for f in &mut fns {
            substitute_constraints(f, &[]);
        }
        assert_eq!(fns[0].params[0].ts_type, "(string)");
        assert!(fns[0].type_params.is_empty());
        assert_eq!(fns[1].type_params, ["T"]);
        assert_eq!(fns[2].params[0].ts_type, "(readonly string[])");
        assert!(js_only_reason(&fns[0], &[]).is_none() && js_only_reason(&fns[1], &[]).is_some());
    }

    #[test]
    fn as_json_bridges_a_data_generic() {
        let mut fns = exported_functions(
            "a.ts",
            "export function paginate<T>(items: T[], page: number): { data: T[] } { return { data: items } }\n",
        );
        assert!(js_only_reason(&fns[0], &[]).is_some());
        substitute_json(&mut fns[0], &["T".to_string()]);
        assert!(fns[0].type_params.is_empty());
        assert_eq!(fns[0].params[0].ts_type, "unknown[]");
        assert_eq!(fns[0].returns, "{ data: unknown[] }");
        assert!(js_only_reason(&fns[0], &[]).is_none());
    }

    #[test]
    fn makes_rust_names() {
        assert_eq!(snake_case("fuzzyScore"), "fuzzy_score");
        assert_eq!(snake_case("type"), "r#type");
        assert_eq!(snake_case("toURL"), "to_url");
        assert_eq!(snake_case("parseHTMLString"), "parse_html_string");
    }
}
