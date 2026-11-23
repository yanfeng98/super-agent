//! Ruby: gems, requires, risks, the gem-to-crate table, and module functions (`def self.x`,
//! `module_function`) with YARD types mapped into the TS-like type dialect.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tree_sitter::Node;

use super::syntax::{self, named_children, text};
use crate::risks::{Risk, regex_needs_fancy};
use crate::ts::deps::CrateMatch;
use crate::ts::discover::{FileKind, Package, SourceFile, Workspace};
use crate::ts::exports::{ExportedFn, Param};
use crate::ts::scan::{FileFacts, Import};

fn language() -> tree_sitter::Language {
    tree_sitter_ruby::LANGUAGE.into()
}

const SKIP_DIRS: &[&str] = &[
    "vendor",
    "node_modules",
    "tmp",
    "log",
    "coverage",
    "pkg",
    "target",
    ".bundle",
];

// ---- discovery -------------------------------------------------------------------------------

/// Whether a Ruby line opens a block that its own `end` closes.
fn opens_a_block(line: &str) -> bool {
    line.ends_with(" do")
        || line.contains(" do |")
        || ["if ", "unless ", "while ", "until ", "case "]
            .iter()
            .any(|kw| line.starts_with(kw))
}

/// `spec.add_dependency "name"` and `gem "name"` lines. A gem inside a `group` block is a
/// development one. Blocks nest (`platform :mri do ... end`), so the group ends at its own `end`,
/// not at the first one: otherwise every gem after a nested block is read as a runtime dependency.
fn gem_names(text: &str) -> (Vec<String>, Vec<String>) {
    static PATTERNS: std::sync::OnceLock<(regex::Regex, regex::Regex)> = std::sync::OnceLock::new();
    let (gem, spec) = PATTERNS.get_or_init(|| {
        (
            regex::Regex::new(r#"^gem\s*\(?\s*['"]([^'"]+)['"]"#).expect("valid regex"),
            regex::Regex::new(
                r#"\badd_(runtime_|development_)?dependency\s*\(?\s*['"]([^'"]+)['"]"#,
            )
            .expect("valid regex"),
        )
    });
    let (mut deps, mut dev) = (Vec::new(), Vec::new());
    let mut depth = 0usize;
    let mut group_at: Option<usize> = None;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('#') {
            continue;
        }
        if t.starts_with("group ") {
            group_at = Some(depth);
        }
        if opens_a_block(t) {
            depth += 1;
        }
        if t == "end" || t.starts_with("end ") {
            depth = depth.saturating_sub(1);
            if group_at.is_some_and(|at| depth <= at) {
                group_at = None;
            }
        }
        if let Some(c) = gem.captures(t) {
            if group_at.is_some() {
                dev.push(c[1].to_string());
            } else {
                deps.push(c[1].to_string());
            }
        }
        if let Some(c) = spec.captures(t) {
            if c.get(1).is_some_and(|m| m.as_str() == "development_") {
                dev.push(c[2].to_string());
            } else {
                deps.push(c[2].to_string());
            }
        }
    }
    (deps, dev)
}

pub fn discover(root: &Path) -> Result<Workspace> {
    let mut packages: Vec<Package> = Vec::new();
    let mut files = Vec::new();
    let walker = crate::walk::filtered(root, |e| {
        !(e.file_type().is_some_and(|t| t.is_dir())
            && SKIP_DIRS.contains(&e.file_name().to_string_lossy().as_ref()))
    })
    .hidden(true)
    .require_git(false)
    .build();
    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        let name = rel.rsplit('/').next().unwrap_or(&rel);
        let dir = rel
            .rsplit_once('/')
            .map_or(String::new(), |(d, _)| d.to_string());
        if name.ends_with(".gemspec") || name == "Gemfile" {
            let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
            let (deps, dev) = gem_names(&text);
            if let Some(p) = packages.iter_mut().find(|p| p.dir == dir) {
                p.deps.extend(deps);
                p.dev_deps.extend(dev);
                if name.ends_with(".gemspec") {
                    p.name = name.trim_end_matches(".gemspec").to_string();
                }
            } else {
                packages.push(Package {
                    name: if name.ends_with(".gemspec") {
                        name.trim_end_matches(".gemspec").to_string()
                    } else if dir.is_empty() {
                        root.file_name()
                            .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned())
                    } else {
                        dir.clone()
                    },
                    src_root: if root.join(&dir).join("lib").is_dir() {
                        join(&dir, "lib")
                    } else {
                        dir.clone()
                    },
                    dir,
                    deps,
                    dev_deps: dev,
                    namespaces: Vec::new(),
                });
            }
        } else if name.ends_with(".rb") {
            let is_test = name.ends_with("_test.rb")
                || name.ends_with("_spec.rb")
                || name.starts_with("test_")
                || rel
                    .split('/')
                    .any(|p| matches!(p, "test" | "spec" | "features"));
            files.push((
                rel,
                if is_test {
                    FileKind::Test
                } else {
                    FileKind::Source
                },
            ));
        }
    }
    if !packages.iter().any(|p| p.dir.is_empty()) {
        packages.push(Package {
            name: root
                .file_name()
                .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned()),
            dir: String::new(),
            src_root: if root.join("lib").is_dir() {
                "lib".into()
            } else {
                String::new()
            },
            deps: Vec::new(),
            dev_deps: Vec::new(),
            namespaces: Vec::new(),
        });
    }
    packages.sort_by(|a, b| a.dir.cmp(&b.dir));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let files = files
        .into_iter()
        .map(|(path, kind)| {
            let package = packages
                .iter()
                .enumerate()
                .filter(|(_, p)| p.dir.is_empty() || path.starts_with(&format!("{}/", p.dir)))
                .max_by_key(|(_, p)| p.dir.len())
                .map_or(0, |(i, _)| i);
            SourceFile {
                path,
                package,
                kind,
            }
        })
        .collect();
    Ok(Workspace { packages, files })
}

fn join(dir: &str, rest: &str) -> String {
    if dir.is_empty() {
        rest.to_string()
    } else {
        format!("{dir}/{rest}")
    }
}

// ---- risks and imports -----------------------------------------------------------------------

const CORE_CLASSES: &[&str] = &[
    "String",
    "Array",
    "Hash",
    "Integer",
    "Float",
    "Numeric",
    "Object",
    "Kernel",
    "Symbol",
    "NilClass",
    "Comparable",
    "Enumerable",
    "Module",
    "Class",
    "Time",
    "Range",
    "Proc",
];

const METAPROGRAMMING: &[&str] = &[
    "define_method",
    "method_missing",
    "respond_to_missing?",
    "instance_variable_set",
    "instance_variable_get",
    "class_eval",
    "module_eval",
    "instance_eval",
    "const_set",
    "remove_method",
    "alias_method",
    "prepend",
    "refine",
];

pub fn scan(source: &str) -> FileFacts {
    let mut facts = FileFacts {
        loc: syntax::loc(source),
        ..FileFacts::default()
    };
    let Some(tree) = syntax::parse(&language(), source) else {
        return facts;
    };
    let root = tree.root_node();
    facts.parse_errors = usize::from(root.has_error());
    let add = |facts: &mut FileFacts, risk: Risk, node: Node| {
        facts
            .risks
            .entry(risk)
            .or_default()
            .push(syntax::line(node))
    };
    syntax::walk(root, |node| match node.kind() {
        "call" => {
            let method = node
                .child_by_field_name("method")
                .map_or("", |m| text(m, source));
            let has_receiver = node.child_by_field_name("receiver").is_some();
            match method {
                "require" | "require_relative" | "load" if !has_receiver => {
                    let arg = node
                        .child_by_field_name("arguments")
                        .and_then(|a| named_children(a).into_iter().next());
                    match arg {
                        Some(a) if a.kind() == "string" && !text(a, source).contains("#{") => {
                            let spec = text(a, source).trim_matches(|c| c == '"' || c == '\'');
                            facts.imports.push(Import {
                                specifier: if method == "require_relative" {
                                    format!("./{spec}")
                                } else {
                                    spec.to_string()
                                },
                                type_only: false,
                                dynamic: false,
                            });
                        }
                        _ => add(&mut facts, Risk::DynamicCode, node),
                    }
                }
                "eval" | "instance_exec" | "class_exec" => add(&mut facts, Risk::DynamicCode, node),
                "send" | "public_send" | "__send__" | "method" | "const_get" | "respond_to?"
                | "is_a?" | "kind_of?" | "instance_of?" => add(&mut facts, Risk::Reflection, node),
                m if METAPROGRAMMING.contains(&m) => add(&mut facts, Risk::Monkeypatching, node),
                "each" | "map" | "each_with_index" | "times" | "each_char" | "select"
                | "reduce" | "inject" | "each_with_object" | "flat_map" => facts.loops += 1,
                "sort_by" | "sort" | "uniq" | "group_by" | "tally" => {
                    add(&mut facts, Risk::OrderedCollections, node)
                }
                "new"
                    if node.child_by_field_name("receiver").is_some_and(|r| {
                        matches!(text(r, source), "Thread" | "Mutex" | "Queue" | "Ractor")
                    }) =>
                {
                    add(&mut facts, Risk::Concurrency, node)
                }
                "new"
                    if node
                        .child_by_field_name("receiver")
                        .is_some_and(|r| text(r, source) == "Regexp") =>
                {
                    add(&mut facts, Risk::DynamicRegex, node)
                }
                "to_json" | "parse" | "generate" if text(node, source).contains("JSON") => {
                    add(&mut facts, Risk::Json, node)
                }
                "raise" | "fail" => add(&mut facts, Risk::Exceptions, node),
                _ => {}
            }
        }
        "class" | "module" => {
            if node.child_by_field_name("superclass").is_some() {
                add(&mut facts, Risk::ClassInheritance, node);
            }
            // Reopening a core class changes it for every caller.
            let name = node
                .child_by_field_name("name")
                .map_or("", |n| text(n, source));
            if CORE_CLASSES.contains(&name) && node.parent().is_some_and(|p| p.kind() == "program")
            {
                add(&mut facts, Risk::Monkeypatching, node);
            }
        }
        "singleton_class" => add(&mut facts, Risk::Monkeypatching, node),
        "regex" => {
            let pattern = text(node, source);
            let body = pattern
                .strip_prefix('/')
                .and_then(|p| p.rsplit_once('/'))
                .map_or(pattern, |(b, _)| b);
            // Onigmo's \h is hex, not horizontal space; other features need fancy-regex.
            if regex_needs_fancy(body, "") || body.contains("\\h") {
                add(&mut facts, Risk::RegexSyntax, node);
            }
        }
        "element_reference" => add(&mut facts, Risk::StringOffsets, node),
        "binary" => {
            let op = node
                .child_by_field_name("operator")
                .map_or("", |o| text(o, source));
            match op {
                "/" | "%" => add(&mut facts, Risk::DivisionSemantics, node),
                "**" | "<<" => add(&mut facts, Risk::IntegerSemantics, node),
                _ => {}
            }
        }
        "rescue" | "ensure" => add(&mut facts, Risk::Exceptions, node),
        "block" | "do_block" | "lambda" | "block_parameter" => {
            add(&mut facts, Risk::Callbacks, node)
        }
        "yield" => add(&mut facts, Risk::Generators, node),
        "nil" => add(&mut facts, Risk::NullReferences, node),
        "hash" => add(&mut facts, Risk::OrderedCollections, node),
        "while" | "until" | "for" => facts.loops += 1,
        "method" | "singleton_method" => add(&mut facts, Risk::DynamicTyping, node),
        _ => {}
    });
    facts
}

/// Ruby's standard library and default gems: no crate needed to replace a `require` of them.
const STDLIB: &[&str] = &[
    "json",
    "set",
    "time",
    "date",
    "securerandom",
    "digest",
    "base64",
    "erb",
    "fileutils",
    "pathname",
    "tempfile",
    "stringio",
    "strscan",
    "uri",
    "net/http",
    "open3",
    "optparse",
    "logger",
    "yaml",
    "psych",
    "csv",
    "bigdecimal",
    "forwardable",
    "singleton",
    "observer",
    "English",
    "shellwords",
    "zlib",
    "socket",
    "timeout",
    "benchmark",
    "pp",
    "prettyprint",
    "ostruct",
    "open-uri",
    "tmpdir",
    "etc",
    "io/console",
    "monitor",
    "weakref",
    "objspace",
    "ripper",
    "coverage",
    "digest/sha2",
    "digest/md5",
    "openssl",
    "cgi",
    "rbconfig",
];

pub fn resolve(
    from: &str,
    spec: &str,
    index: &HashMap<&str, usize>,
    ws: &Workspace,
) -> super::Resolution {
    let with_ext = |p: &str| {
        if p.ends_with(".rb") {
            p.to_string()
        } else {
            format!("{p}.rb")
        }
    };
    if let Some(rel) = spec.strip_prefix("./") {
        let dir = from.rsplit_once('/').map_or("", |(d, _)| d);
        if let Some(path) = normalize(&join(dir, &with_ext(rel)))
            && let Some(i) = index.get(path.as_str())
        {
            return super::Resolution::File(*i);
        }
        return super::Resolution::Unresolved;
    }
    for p in &ws.packages {
        let candidate = join(&p.src_root, &with_ext(spec));
        if let Some(i) = index.get(candidate.as_str()) {
            return super::Resolution::File(*i);
        }
    }
    if STDLIB.contains(&spec) || STDLIB.contains(&spec.split('/').next().unwrap_or(spec)) {
        return super::Resolution::Stdlib;
    }
    super::Resolution::External(spec.split('/').next().unwrap_or(spec).to_string())
}

fn normalize(path: &str) -> Option<String> {
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

/// Rust counterparts of common gems.
pub fn lookup(gem: &str) -> Option<CrateMatch> {
    crate::crates::find("ruby", gem)
}

// ---- exports ---------------------------------------------------------------------------------

/// A YARD type list (`String, nil`, `Array<Integer>`, `Hash{String => Integer}`) in the dialect.
pub fn yard_to_ts(types: &str) -> String {
    let parts = split_top(types, ',');
    if parts.len() > 1 {
        let mapped: Vec<String> = parts.iter().map(|p| yard_to_ts(p)).collect();
        return mapped.join(" | ");
    }
    let t = types.trim();
    if let Some(inner) = t.strip_prefix("Array<").and_then(|r| r.strip_suffix('>')) {
        let inner = yard_to_ts(inner);
        return if inner.contains(' ') {
            format!("({inner})[]")
        } else {
            format!("{inner}[]")
        };
    }
    if let Some(inner) = t.strip_prefix("Hash{").and_then(|r| r.strip_suffix('}'))
        && let Some((k, v)) = inner.split_once("=>")
    {
        let key = yard_to_ts(k);
        return if key == "string" {
            format!("Record<string, {}>", yard_to_ts(v))
        } else {
            "unknown".into()
        };
    }
    match t {
        "String" => "string".into(),
        "Integer" | "Fixnum" | "Bignum" => "int".into(),
        "Float" | "Numeric" => "number".into(),
        "Boolean" | "TrueClass" | "FalseClass" | "true" | "false" | "Bool" => "boolean".into(),
        "nil" | "NilClass" | "void" => "undefined".into(),
        "Array" => "unknown[]".into(),
        "Symbol" => "string".into(),
        _ => "unknown".into(),
    }
}

fn split_top(text: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for c in text.chars() {
        match c {
            '<' | '{' | '(' | '[' => depth += 1,
            '>' | '}' | ')' | ']' => depth -= 1,
            _ if c == sep && depth == 0 => {
                out.push(current.trim().to_string());
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

/// The comment lines right above line `row` (0-based), top to bottom.
fn doc_above(lines: &[&str], row: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = row;
    while i > 0 {
        i -= 1;
        let l = lines[i].trim();
        if let Some(c) = l.strip_prefix('#') {
            out.push(c.trim().to_string());
        } else {
            break;
        }
    }
    out.reverse();
    out
}

/// The dialect type of a literal default (`"-"`, `0`, `1.5`, `true`, `[]`).
fn literal_type(value: &str) -> Option<&'static str> {
    let v = value.trim();
    if v.starts_with('"') || v.starts_with('\'') {
        Some("string")
    } else if v == "true" || v == "false" {
        Some("boolean")
    } else if v.parse::<i64>().is_ok() {
        Some("int")
    } else if v.parse::<f64>().is_ok() {
        Some("number")
    } else {
        None
    }
}

fn method_fn(
    path: &str,
    module: &str,
    node: Node,
    source: &str,
    lines: &[&str],
) -> Option<ExportedFn> {
    let name = text(node.child_by_field_name("name")?, source).to_string();
    let doc = doc_above(lines, node.start_position().row);
    let yard_param = |p: &str| {
        doc.iter().find_map(|l| {
            let rest = l.strip_prefix("@param ")?;
            let (pname, rest) = rest.split_once(' ')?;
            if pname.trim_start_matches(':').trim_end_matches(':') != p {
                return None;
            }
            let types = rest.trim().strip_prefix('[')?.split(']').next()?;
            Some(yard_to_ts(types))
        })
    };
    let returns = doc
        .iter()
        .find_map(|l| {
            let rest = l.strip_prefix("@return ")?.trim().strip_prefix('[')?;
            Some(yard_to_ts(rest.split(']').next()?))
        })
        .unwrap_or_else(|| "unknown".into());
    let mut blocker = None;
    if name.ends_with('=')
        || name.ends_with('!')
        || !name
            .chars()
            .next()
            .is_some_and(|c| c.is_lowercase() || c == '_')
    {
        blocker = Some(format!("`{name}` is not a plain method name"));
    }
    let mut params = Vec::new();
    let mut signature = String::new();
    if let Some(list) = node.child_by_field_name("parameters") {
        signature = text(list, source)
            .trim_matches(|c| c == '(' || c == ')')
            .to_string();
        for p in named_children(list) {
            let pname = p.child_by_field_name("name").map_or_else(
                || text(p, source).to_string(),
                |n| text(n, source).to_string(),
            );
            let value = p
                .child_by_field_name("value")
                .map(|v| text(v, source).to_string());
            let (keyword_only, optional) = match p.kind() {
                "identifier" => (false, false),
                "optional_parameter" => (false, true),
                "keyword_parameter" => (true, value.is_some()),
                "splat_parameter" => {
                    blocker.get_or_insert_with(|| "takes `*args`".into());
                    (false, true)
                }
                "hash_splat_parameter" => {
                    blocker.get_or_insert_with(|| "takes `**options`".into());
                    (false, true)
                }
                "block_parameter" => {
                    blocker.get_or_insert_with(|| "takes a block".into());
                    continue;
                }
                _ => {
                    blocker.get_or_insert_with(|| format!("parameter `{}`", text(p, source)));
                    continue;
                }
            };
            let mut ts_type = yard_param(&pname)
                .or_else(|| value.as_deref().and_then(literal_type).map(str::to_string))
                .unwrap_or_else(|| "unknown".into());
            if value.as_deref() == Some("nil") && !ts_type.contains("undefined") {
                ts_type = format!("{ts_type} | undefined");
            }
            params.push(Param {
                name: pname,
                ts_type,
                optional,
                default: value,
                keyword_only,
                ..Default::default()
            });
        }
    }
    infer_numbers(&mut params, node, source);
    // A method that yields takes a block, even without `&block`.
    if let Some(body) = node.child_by_field_name("body") {
        let mut yields = false;
        syntax::walk(body, |n| yields |= n.kind() == "yield");
        if yields {
            blocker.get_or_insert_with(|| "yields to a block".into());
        }
    }
    Some(ExportedFn {
        name,
        file: path.to_string(),
        line: syntax::line(node),
        params,
        returns,
        prototype: Some(format!(
            "{module}.{}({signature})",
            text(node.child_by_field_name("name")?, source)
        )),
        blocker,
        ..Default::default()
    })
}

/// Parameters without a YARD type that the body uses in arithmetic become `number`.
fn infer_numbers(params: &mut [Param], node: Node, source: &str) {
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };
    let mut numeric: Vec<String> = Vec::new();
    syntax::walk(body, |n| {
        if n.kind() != "binary" {
            return;
        }
        let op = n
            .child_by_field_name("operator")
            .map_or("", |o| text(o, source));
        let (Some(l), Some(r)) = (
            n.child_by_field_name("left"),
            n.child_by_field_name("right"),
        ) else {
            return;
        };
        let literal = |x: Node| matches!(x.kind(), "integer" | "float");
        if matches!(op, "-" | "*" | "/" | "**")
            || (matches!(op, "+" | "%" | "<" | ">" | "<=" | ">=") && (literal(l) || literal(r)))
        {
            for side in [l, r] {
                if side.kind() == "identifier" {
                    numeric.push(text(side, source).to_string());
                }
            }
        }
    });
    for p in params
        .iter_mut()
        .filter(|p| p.ts_type == "unknown" && numeric.contains(&p.name))
    {
        p.ts_type = "number".into();
    }
}

/// Module functions: `def self.name` in a module or class, and methods after `module_function`
/// or inside `class << self`. The receiver goes into `prototype` as `Module::Path.name(...)`.
pub fn exports(path: &str, source: &str) -> Vec<ExportedFn> {
    if !path.ends_with(".rb") {
        return Vec::new();
    }
    let Some(tree) = syntax::parse(&language(), source) else {
        return Vec::new();
    };
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    for child in named_children(tree.root_node()) {
        if matches!(child.kind(), "module" | "class") {
            namespace(path, child, source, &lines, &mut Vec::new(), &mut out);
        }
    }
    out.sort_by_key(|f| f.line);
    out
}

/// The module functions of one `module` or `class`, and of the namespaces nested in it.
fn namespace(
    path: &str,
    node: Node,
    source: &str,
    lines: &[&str],
    scope: &mut Vec<String>,
    out: &mut Vec<ExportedFn>,
) {
    let Some(name) = node.child_by_field_name("name") else {
        return;
    };
    scope.push(text(name, source).to_string());
    let receiver = scope.join("::");
    let is_module = node.kind() == "module";
    let mut module_function = false;
    let mut private = false;
    let mut hidden: Vec<String> = Vec::new();
    let mut found = Vec::new();
    // Instance methods so far, for `module_function :name` after them.
    let mut instance: Vec<Node> = Vec::new();
    for item in node
        .child_by_field_name("body")
        .map(named_children)
        .unwrap_or_default()
    {
        match item.kind() {
            "identifier" if text(item, source) == "module_function" && is_module => {
                module_function = true;
            }
            "identifier" if matches!(text(item, source), "private" | "protected") => {
                private = true;
            }
            "call" if text(item, source).starts_with("private_class_method") => {
                hidden.push(text(item, source).to_string());
            }
            "call" if is_module && text(item, source).starts_with("module_function") => {
                let call = text(item, source);
                for m in &instance {
                    let name = m
                        .child_by_field_name("name")
                        .map_or("", |n| text(n, source));
                    if call.contains(&format!(":{name}")) {
                        found.extend(method_fn(path, &receiver, *m, source, lines));
                    }
                }
            }
            "singleton_method"
                if item
                    .child_by_field_name("object")
                    .is_some_and(|o| text(o, source) == "self") =>
            {
                found.extend(method_fn(path, &receiver, item, source, lines));
            }
            "method" if module_function && !private => {
                found.extend(method_fn(path, &receiver, item, source, lines));
            }
            "method" if !private => instance.push(item),
            "singleton_class" => {
                let mut private_here = false;
                for m in item
                    .child_by_field_name("body")
                    .map(named_children)
                    .unwrap_or_default()
                {
                    if m.kind() == "identifier" && text(m, source) == "private" {
                        private_here = true;
                    }
                    if m.kind() == "method" && !private_here {
                        found.extend(method_fn(path, &receiver, m, source, lines));
                    }
                }
            }
            "module" | "class" => namespace(path, item, source, lines, scope, out),
            _ => {}
        }
    }
    found.retain(|f| !hidden.iter().any(|h| h.contains(&format!(":{}", f.name))));
    found.sort_by_key(|f| f.line);
    out.extend(found);
    scope.pop();
}

/// The receiver and method name of an export: `("Textkit::Words", "slugify")`.
pub fn receiver(f: &ExportedFn) -> (String, String) {
    let proto = f.prototype.as_deref().unwrap_or(&f.name);
    let head = proto.split('(').next().unwrap_or(proto);
    match head.rsplit_once('.') {
        Some((recv, name)) => (recv.to_string(), name.to_string()),
        None => ("Object".into(), f.name.clone()),
    }
}

/// The parameter list as written: `text, sep = "-", max: nil`.
pub fn signature(f: &ExportedFn) -> String {
    let proto = f.prototype.as_deref().unwrap_or("");
    proto
        .split_once('(')
        .map(|(_, rest)| rest.strip_suffix(')').unwrap_or(rest).to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_yard_types() {
        assert_eq!(yard_to_ts("String, nil"), "string | undefined");
        assert_eq!(yard_to_ts("Array<Integer>"), "int[]");
        assert_eq!(yard_to_ts("Hash{String => Integer}"), "Record<string, int>");
    }

    #[test]
    fn finds_module_functions() {
        let src = "module Textkit\n  # Slug.\n  # @param text [String]\n  # @return [String]\n  \
                   def self.slugify(text, sep = \"-\", max: nil)\n    text\n  end\n\n  \
                   def self.each_word(text)\n    yield text\n  end\n\n  \
                   module Nums\n    module_function\n\n    def clamp(v, lo = 0, hi = 10) = v\n\n    private\n\n    def helper = 1\n  end\n\n  \
                   class Stack\n    def push(x) = x\n  end\nend\n";
        let fns = exports("lib/textkit.rb", src);
        let names: Vec<(String, String)> = fns.iter().map(receiver).collect();
        assert_eq!(
            names,
            [
                ("Textkit".to_string(), "slugify".to_string()),
                ("Textkit".to_string(), "each_word".to_string()),
                ("Textkit::Nums".to_string(), "clamp".to_string()),
            ]
        );
        let slug = &fns[0];
        assert_eq!(slug.params[0].ts_type, "string");
        assert_eq!(slug.params[1].ts_type, "string");
        assert!(slug.params[2].keyword_only);
        assert_eq!(slug.params[2].ts_type, "unknown | undefined");
        assert_eq!(slug.returns, "string");
        assert_eq!(signature(slug), "text, sep = \"-\", max: nil");
        assert!(fns[1].blocker.is_some());
        assert_eq!(fns[2].params[1].ts_type, "int");
    }

    #[test]
    fn module_function_with_names_and_inferred_numbers() {
        let src = "module Shellwords\n  def shellsplit(line)\n    line\n  end\n  module_function :shellsplit\n\n  \
                   def helper(x) = x\n\n  def scale(x, factor)\n    x * factor\n  end\n  module_function :scale\nend\n";
        let fns = exports("lib/shellwords.rb", src);
        let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["shellsplit", "scale"]);
        assert_eq!(fns[0].params[0].ts_type, "unknown");
        assert_eq!(fns[1].params[0].ts_type, "number");
    }
}
