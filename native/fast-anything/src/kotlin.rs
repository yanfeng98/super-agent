//! Kotlin backend (Kotlin/JVM): Gradle projects with sources in src/main/kotlin, run as
//! gradle run. The JVM side is the Java backend's: Flight Recorder profiles the
//! run and the same generated Java classes record calls and call the port through the FFM API;
//! they go in src/main/java, where the build compiles them. Kotlin's own part is reading the
//! sources (top-level functions, object and companion members, extension functions) and the
//! Kotlin prologues that hand a call to those classes.

use crate::backend::{Lang, Profile, Recording};
use crate::hotspot::{Hotspot, Param};
use crate::java::{self, Method, Runner};
use crate::native::{self, Flavor, Patches};
use crate::python::DISABLE_ENV;
use crate::state::{Ctx, Port, Verify};
use crate::util::{self, combined, output, tail};
use anyhow::{Context, Result, bail, ensure};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// A Gradle build with Kotlin sources is a Kotlin app.
pub fn refine(lang: Lang, program: &str, root: &Path) -> Lang {
    let build = java::runner(program) == Some(Runner::Gradle);
    if lang == Lang::Java && build && !kotlin_files(root).is_empty() { Lang::Kotlin } else { lang }
}

fn kotlin_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                if !name.starts_with('.') && !["build", "target", "out", "node_modules"].contains(&name.as_str()) {
                    stack.push(p);
                }
            } else if name.ends_with(".kt") && in_kotlin_sources(&p) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// .../src/main/kotlin/...
fn in_kotlin_sources(p: &Path) -> bool {
    let parts: Vec<String> = p.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    parts.windows(3).any(|w| w == ["src", "main", "kotlin"])
}

// ------------------------------------------------------------------ reading Kotlin

/// The source with comments, strings and char literals blanked out (same length, newlines kept),
/// so braces, parentheses and keywords can be found by scanning.
fn code_only(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    while i < b.len() {
        let end = if b[i..].starts_with(b"//") {
            b[i..].iter().position(|&c| c == b'\n').map_or(b.len(), |p| i + p)
        } else if b[i..].starts_with(b"/*") {
            let (mut depth, mut j) = (0, i);
            while j < b.len() {
                if b[j..].starts_with(b"/*") {
                    depth += 1;
                    j += 2;
                } else if b[j..].starts_with(b"*/") {
                    depth -= 1;
                    j += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    j += 1;
                }
            }
            j
        } else if b[i] == b'"' || b[i] == b'\'' {
            string_end(b, i)
        } else {
            i += 1;
            continue;
        };
        for c in &mut out[i..end] {
            if *c != b'\n' {
                *c = b' ';
            }
        }
        i = end;
    }
    String::from_utf8(out).unwrap_or_default()
}

/// Where the string (raw """ too) or char literal starting at i ends, ${...} templates included.
fn string_end(b: &[u8], i: usize) -> usize {
    if b[i] == b'\'' {
        let mut j = i + 1;
        while j < b.len() && b[j] != b'\'' {
            j += if b[j] == b'\\' { 2 } else { 1 };
        }
        return (j + 1).min(b.len());
    }
    let raw = b[i..].starts_with(b"\"\"\"");
    let mut j = i + if raw { 3 } else { 1 };
    while j < b.len() {
        if raw && b[j..].starts_with(b"\"\"\"") {
            // a raw string can end with extra quotes: """a"""" is the string a"
            let mut k = j + 3;
            while k < b.len() && b[k] == b'"' {
                k += 1;
            }
            return k;
        }
        if !raw && b[j] == b'"' {
            return j + 1;
        }
        if !raw && b[j] == b'\\' {
            j += 2;
            continue;
        }
        if b[j..].starts_with(b"${") {
            let mut depth = 0;
            while j < b.len() {
                match b[j] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    b'"' | b'\'' => {
                        j = string_end(b, j);
                        continue;
                    }
                    _ => {}
                }
                j += 1;
            }
        }
        j += 1;
    }
    b.len()
}

/// The index of the bracket matching the one at `open` (in code_only text).
fn matching(code: &[u8], open: usize) -> Option<usize> {
    let (o, c) = (
        code[open],
        match code[open] {
            b'(' => b')',
            b'{' => b'}',
            b'<' => b'>',
            _ => b']',
        },
    );
    let mut depth = 0;
    for (k, &ch) in code.iter().enumerate().skip(open) {
        if ch == o {
            depth += 1;
        } else if ch == c {
            depth -= 1;
            if depth == 0 {
                return Some(k);
            }
        }
    }
    None
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// The whole word at i (no identifier characters right before or after).
fn word_at(code: &[u8], i: usize, word: &str) -> bool {
    code[i..].starts_with(word.as_bytes())
        && (i == 0 || !is_ident(code[i - 1]))
        && code.get(i + word.len()).is_none_or(|&c| !is_ident(c))
}

fn words(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '@')).filter(|w| !w.is_empty()).collect()
}

/// Split on commas outside brackets.
fn split_top(s: &str) -> Vec<String> {
    let (mut depth, mut cur, mut out) = (0i32, String::new(), vec![]);
    for c in s.chars() {
        match c {
            '<' | '(' | '[' | '{' => depth += 1,
            '>' | ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

#[derive(Clone)]
enum Scope {
    Top,
    /// a class, object or companion: its JVM class and how Kotlin code names it
    Type {
        jvm: String,
        name: String,
        object: bool,
    },
    Other,
}

const IO: &[&str] = &[
    "println(",
    "print(",
    "readLine(",
    "readln(",
    "File(",
    "java.io.",
    "kotlin.io.",
    "Files.",
    "System.out",
    "System.err",
    "Paths.",
    "URL(",
    "Socket(",
    "ProcessBuilder(",
    "Runtime.getRuntime",
];
const NONDET: &[&str] = &[
    "Random",
    "random(",
    "shuffled(",
    "shuffle(",
    "currentTimeMillis",
    "nanoTime",
    "UUID",
    "Clock.",
    "Instant.now",
    "LocalDate.now",
    "LocalDateTime.now",
];
const COROUTINES: &[&str] = &["launch", "async", "runBlocking", "withContext", "delay("];

/// Every function fast-anything can name in one Kotlin file.
fn describe_file(path: &Path) -> Vec<Method> {
    let Ok(src) = fs::read_to_string(path) else { return vec![] };
    let code = code_only(&src);
    let c = code.as_bytes();
    let line_of = |pos: usize| src[..pos].matches('\n').count() as u32 + 1;

    let package = code
        .lines()
        .find_map(|l| l.trim().strip_prefix("package ").map(|p| p.trim().trim_end_matches(';').to_string()))
        .unwrap_or_default();
    let qualify = |name: &str| if package.is_empty() { name.to_string() } else { format!("{package}.{name}") };
    // top-level functions live in <File>Kt, or the name @file:JvmName gives
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let file_class = src
        .find("@file:JvmName(\"")
        .and_then(|i| src[i + 15..].split('"').next().map(String::from))
        .unwrap_or_else(|| {
            let mut chars = stem.chars();
            chars.next().map(|f| f.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default() + "Kt"
        });

    struct Found {
        jvm: String,
        module: String,
        owner: String,
        start: usize,
        decl: usize,
        name: String,
        receiver: Option<String>,
        params: String,
        returns: Option<String>,
        body: Option<(usize, usize)>,
        expression: bool,
        prefix: String,
        generic: bool,
    }
    let mut found: Vec<Found> = Vec::new();
    let mut stack = vec![Scope::Top];
    let mut boundary = 0; // where the current statement's header starts
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            b'{' => {
                let header = code[boundary..i].trim();
                let w = words(header);
                let parent = stack.last().cloned().unwrap_or(Scope::Other);
                let scope = if matches!(parent, Scope::Other) || w.contains(&"fun") {
                    Scope::Other
                } else if let Some(k) = w.iter().position(|x| *x == "object") {
                    let anonymous = header.contains('=')
                        || header[header.find("object").unwrap_or(0) + 6..].trim_start().starts_with(':');
                    let companion = k > 0 && w[k - 1] == "companion";
                    let named = w
                        .get(k + 1)
                        .filter(|n| n.chars().next().is_some_and(char::is_alphabetic))
                        .map(|n| n.to_string());
                    match (&parent, anonymous, companion) {
                        (_, true, _) => Scope::Other,
                        (Scope::Type { jvm, name, .. }, _, true) => {
                            let own = named.unwrap_or_else(|| "Companion".into());
                            Scope::Type { jvm: format!("{jvm}${own}"), name: name.clone(), object: true }
                        }
                        (Scope::Top, _, false) => {
                            let own = named.unwrap_or_default();
                            Scope::Type { jvm: qualify(&own), name: own, object: true }
                        }
                        (Scope::Type { jvm, name, .. }, _, false) => {
                            let own = named.unwrap_or_default();
                            Scope::Type { jvm: format!("{jvm}${own}"), name: format!("{name}.{own}"), object: true }
                        }
                        _ => Scope::Other,
                    }
                } else if let Some(k) = w.iter().position(|x| *x == "class" || *x == "interface") {
                    let own = w.get(k + 1).map(|s| s.to_string()).unwrap_or_default();
                    match &parent {
                        Scope::Top => Scope::Type { jvm: qualify(&own), name: own, object: false },
                        Scope::Type { jvm, name, .. } => {
                            Scope::Type { jvm: format!("{jvm}${own}"), name: format!("{name}.{own}"), object: false }
                        }
                        Scope::Other => Scope::Other,
                    }
                } else {
                    Scope::Other
                };
                stack.push(scope);
                boundary = i + 1;
            }
            b'}' => {
                stack.pop();
                boundary = i + 1;
            }
            b';' => boundary = i + 1,
            b'f' if word_at(c, i, "fun") => {
                let collect = match stack.last() {
                    Some(Scope::Top) => Some((qualify(&file_class), package.clone(), file_class.clone())),
                    Some(Scope::Type { jvm, name, object: true }) => {
                        Some((jvm.clone(), qualify(name), name.rsplit('.').next().unwrap_or(name).to_string()))
                    }
                    _ => None,
                };
                if let Some((jvm, module, owner)) = collect {
                    // the declaration: its line (modifiers) and the annotation lines right above it
                    let line_start = |pos: usize| code[..pos].rfind('\n').map_or(0, |p| p + 1);
                    let mut prefix_start = line_start(i).max(boundary);
                    while prefix_start > boundary {
                        let above = line_start(prefix_start - 1).max(boundary);
                        if !code[above..prefix_start].trim().starts_with('@') {
                            break;
                        }
                        prefix_start = above;
                    }
                    while prefix_start < i && c[prefix_start].is_ascii_whitespace() {
                        prefix_start += 1;
                    }
                    let mut j = i + 3;
                    while j < c.len() && c[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    let generic = c.get(j) == Some(&b'<');
                    if generic {
                        j = matching(c, j).map_or(c.len(), |k| k + 1);
                    }
                    let Some(open) = c[j..].iter().position(|&ch| ch == b'(').map(|p| j + p) else {
                        i += 3;
                        continue;
                    };
                    let target = code[j..open].trim();
                    let (receiver, name) = match target.rfind('.') {
                        Some(dot)
                            if !target[..dot].contains('<')
                                || target[..dot].matches('<').count() == target[..dot].matches('>').count() =>
                        {
                            (Some(target[..dot].to_string()), target[dot + 1..].to_string())
                        }
                        _ => (None, target.to_string()),
                    };
                    let Some(close) = matching(c, open) else { break };
                    let mut k = close + 1;
                    while k < c.len() && c[k].is_ascii_whitespace() && c[k] != b'\n' {
                        k += 1;
                    }
                    let mut returns = None;
                    if c.get(k) == Some(&b':') {
                        let mut depth = 0;
                        let start = k + 1;
                        while k < c.len() {
                            match c[k] {
                                b'<' | b'(' => depth += 1,
                                b'>' | b')' => depth -= 1,
                                b'{' | b'=' | b'\n' if depth == 0 => break,
                                _ => {}
                            }
                            k += 1;
                        }
                        returns = Some(src[start..k].trim().to_string());
                    }
                    while k < c.len() && c[k].is_ascii_whitespace() {
                        k += 1;
                    }
                    let (body, expression) = match c.get(k) {
                        Some(b'{') => (matching(c, k).map(|e| (k, e)), false),
                        Some(b'=') => (None, true),
                        _ => (None, false),
                    };
                    found.push(Found {
                        jvm,
                        module,
                        owner,
                        start: prefix_start,
                        decl: i,
                        name: name.trim().to_string(),
                        receiver,
                        params: src[open + 1..close].to_string(),
                        returns,
                        body,
                        expression,
                        prefix: code[prefix_start..i].to_string(),
                        generic,
                    });
                    // skip the signature; the body's braces are scanned as usual
                    i = close + 1;
                    continue;
                }
                i += 3;
                continue;
            }
            _ => {}
        }
        i += 1;
    }

    let names: Vec<String> = found.iter().map(|f| f.name.clone()).collect();
    found
        .into_iter()
        .map(|f| {
            let mut blockers = Vec::new();
            let mut warnings = Vec::new();
            let modifiers = words(&f.prefix);
            if modifiers.contains(&"suspend") {
                blockers.push("suspend function".to_string());
            }
            if modifiers.contains(&"inline") {
                blockers.push(
                    "inline function (the recording and the switch to Rust call it from inside itself)".to_string(),
                );
            }
            if f.generic {
                blockers.push("generic".to_string());
            }
            if f.expression {
                blockers.push("expression body: write it as { return ... } to port it".to_string());
            } else if f.body.is_none() {
                blockers.push("no body".to_string());
            }
            match f.returns.as_deref() {
                None | Some("Unit") => blockers.push("returns nothing (Unit)".to_string()),
                _ => {}
            }
            let mut params = Vec::new();
            if let Some(r) = &f.receiver {
                params.push(Param {
                    name: "receiver".into(),
                    annotation: Some(r.clone()),
                    default: None,
                    kwonly: false,
                });
            }
            for p in split_top(&f.params) {
                let p = p.trim();
                if words(p).contains(&"vararg") {
                    blockers.push("takes vararg".to_string());
                }
                let p = p.trim_start_matches(|ch: char| ch != ':' && !ch.is_alphanumeric() && ch != '_');
                let (decl, default) = match p.split_once('=') {
                    Some((d, v)) => (d.trim(), Some(v.trim().to_string())),
                    None => (p, None),
                };
                let (name, ty) = decl.split_once(':').unwrap_or((decl, ""));
                let name = words(name).last().copied().unwrap_or_default().to_string();
                params.push(Param { name, annotation: Some(ty.trim().to_string()), default, kwonly: false });
            }
            let body = f.body.map(|(b, e)| &code[b..=e]).unwrap_or_default();
            let io: Vec<&str> = IO.iter().filter(|w| body.contains(*w)).map(|w| w.trim_end_matches('(')).collect();
            if !io.is_empty() {
                blockers.push(format!("does I/O: {}", io[..io.len().min(3)].join(", ")));
            }
            let nondet: Vec<&str> =
                NONDET.iter().filter(|w| body.contains(*w)).map(|w| w.trim_end_matches(['(', '.'])).collect();
            if !nondet.is_empty() {
                blockers.push(format!("not deterministic: {}", nondet[..nondet.len().min(3)].join(", ")));
            }
            let b = body.as_bytes();
            if COROUTINES.iter().any(|w| (0..b.len()).any(|k| word_at(b, k, w.trim_end_matches('(')))) {
                blockers.push("uses coroutines".to_string());
            }
            let calls: Vec<String> = names
                .iter()
                .filter(|n| {
                    **n != f.name
                        && (0..b.len())
                            .any(|k| word_at(b, k, n) && b[k + n.len()..].trim_ascii_start().starts_with(b"("))
                })
                .cloned()
                .collect();
            if !calls.is_empty() {
                warnings.push(format!(
                    "calls {}, they get ported together with it",
                    calls[..calls.len().min(3)].join(", ")
                ));
            }
            let end = f.body.map_or_else(|| src[f.decl..].find('\n').map_or(src.len(), |p| f.decl + p), |(_, e)| e + 1);
            Method {
                class: f.jvm,
                owner: f.owner,
                name: f.name,
                path: path.to_path_buf(),
                line: line_of(f.decl),
                end_line: line_of(end.saturating_sub(1)),
                params,
                returns: f.returns.unwrap_or_else(|| "Unit".into()),
                blockers,
                warnings,
                calls,
                source: src[f.start..end].to_string(),
                module: Some(f.module),
            }
        })
        .filter(|m| !m.name.is_empty())
        .collect()
}

fn describe(ctx: &Ctx) -> Result<Vec<Method>> {
    let files = kotlin_files(&ctx.root);
    ensure!(!files.is_empty(), "no Kotlin sources (src/main/kotlin) in this project");
    Ok(files.iter().flat_map(|f| describe_file(f)).collect())
}

pub fn profile(ctx: &Ctx, program: &str, args: &[String]) -> Result<Profile> {
    java::profile_with(ctx, program, args, |_| describe(ctx))
}

// ------------------------------------------------------------------ editing the source

/// The line where the function's body opens.
fn find_body(name: &str, hint: u32) -> impl Fn(&str) -> Option<usize> {
    let (spaced, dotted, gap) = (format!(" {name}("), format!(".{name}("), format!(" {name} ("));
    move |text: &str| {
        native::body_line(text, hint, |l| {
            let code = l.split("//").next().unwrap_or_default();
            code.contains("fun ") && (code.contains(&spaced) || code.contains(&dotted) || code.contains(&gap))
        })
    }
}

fn package_of(text: &str) -> String {
    text.lines()
        .find_map(|l| l.trim().strip_prefix("package ").map(|p| p.trim().trim_end_matches(';').to_string()))
        .unwrap_or_default()
}

/// src/main/java/<package> of the module the Kotlin file is in: where the build compiles the Java
/// classes fast-anything adds (Java files among the Kotlin sources would be seen by kotlinc but not
/// compiled).
fn java_dir(file: &Path, package: &str) -> Result<PathBuf> {
    let mut module = file.parent();
    while let Some(dir) = module {
        if dir.ends_with("src/main/kotlin") {
            let mut out = dir.parent().context("no src/main")?.join("java");
            for part in package.split('.').filter(|p| !p.is_empty()) {
                out.push(part);
            }
            return Ok(out);
        }
        module = dir.parent();
    }
    bail!("{} isn't under src/main/kotlin", file.display())
}

/// Create a folder; returns the ones that didn't exist, outermost first.
fn create_dirs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut missing = Vec::new();
    let mut d = Some(dir);
    while let Some(p) = d.filter(|p| !p.exists()) {
        missing.push(p.to_path_buf());
        d = p.parent();
    }
    fs::create_dir_all(dir)?;
    missing.reverse();
    Ok(missing)
}

/// Remove empty folders from `dir` up to src/main/java itself.
fn prune(dir: &Path) {
    let mut d = Some(dir);
    while let Some(p) = d {
        if fs::remove_dir(p).is_err() || p.ends_with("src/main/java") {
            break;
        }
        d = p.parent();
    }
}

fn args_array(hs: &Hotspot) -> String {
    let names: Vec<&str> =
        hs.params.iter().map(|p| if p.name == "receiver" { "this" } else { p.name.as_str() }).collect();
    format!("arrayOf<Any?>({})", names.join(", "))
}

/// The function called again with the arguments from an Object[]: name(a[0] as T, ...), or for an
/// extension (a[0] as R).name(...).
fn call_with(hs: &Hotspot) -> String {
    let cast = |i: usize, p: &Param| format!("__faA[{i}] as {}", p.annotation.as_deref().unwrap_or("Any?"));
    match hs.params.first().filter(|p| p.name == "receiver") {
        Some(r) => {
            let rest: Vec<String> = hs.params.iter().enumerate().skip(1).map(|(i, p)| cast(i, p)).collect();
            format!("({}).{}({})", cast(0, r), hs.name, rest.join(", "))
        }
        None => {
            let all: Vec<String> = hs.params.iter().enumerate().map(|(i, p)| cast(i, p)).collect();
            format!("{}({})", hs.name, all.join(", "))
        }
    }
}

fn ret(hs: &Hotspot) -> String {
    hs.returns.clone().unwrap_or_else(|| "Any?".into())
}

fn record_prologue(hs: &Hotspot) -> String {
    format!(
        "if (!FastAnythingRec.inside) {{\n    @Suppress(\"UNCHECKED_CAST\")\n    val __faR = FastAnythingRec.record({args}) {{ __faA -> {call} }} as {ret}\n    return __faR\n}}",
        args = args_array(hs),
        call = call_with(hs),
        ret = ret(hs),
    )
}

pub fn record(ctx: &Ctx, program: &str, args: &[String], hs: &Hotspot, out: &Path, limit: usize) -> Result<Recording> {
    let file = ctx.root.join(&hs.file);
    let text = fs::read_to_string(&file).with_context(|| format!("reading {}", hs.file))?;
    let package = package_of(&text);
    let dir = java_dir(&file, &package)?;
    let package_line = if package.is_empty() { String::new() } else { format!("package {package};") };
    let rec = java::REC.replace("/*PACKAGE*/", &package_line).replace("/*ID*/", &hs.id).replace("/*JSON*/", java::JSON);
    let patched = native::insert_in_body(
        &text,
        "//",
        &format!("recording {}", hs.id),
        &record_prologue(hs),
        find_body(&hs.name, hs.line),
    )?;
    let meta = out.with_extension("meta.json");
    let created = create_dirs(&dir)?;
    let mut patches = Patches::new(ctx)?;
    patches.write(&dir.join("FastAnythingRec.java"), &rec)?;
    patches.write(&file, &patched)?;
    let run = output(
        java::app_command(program, args)
            .current_dir(&ctx.root)
            .env("FAST_ANYTHING_RECORD_OUT", out)
            .env("FAST_ANYTHING_RECORD_META", &meta)
            .env("FAST_ANYTHING_RECORD_LIMIT", limit.to_string())
            .env(DISABLE_ENV, "1")
            .stdin(Stdio::null()),
        program,
    );
    patches.restore()?;
    for d in created.iter().rev() {
        let _ = fs::remove_dir(d);
    }
    let run = run?;
    ensure!(run.status.success(), "the app failed while recording ({}):\n{}", run.status, tail(&combined(&run), 30));
    serde_json::from_str(&fs::read_to_string(&meta).context("the recorder wrote nothing")?)
        .context("reading the recording summary")
}

// ------------------------------------------------------------------ the crate

/// Rust type for a Kotlin type.
pub fn rust_type(t: &str) -> Option<String> {
    let t: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    let t = t.trim_start_matches("kotlin.collections.").trim_start_matches("kotlin.");
    if let Some(inner) = t.strip_suffix('?') {
        return rust_type(inner).map(|r| format!("Option<{r}>"));
    }
    Some(match t {
        "Int" => "i32".into(),
        "Long" => "i64".into(),
        "Short" => "i16".into(),
        "Byte" => "i8".into(),
        "Double" => "f64".into(),
        "Float" => "f32".into(),
        "Boolean" => "bool".into(),
        "String" => "String".into(),
        "IntArray" => "Vec<i32>".into(),
        "LongArray" => "Vec<i64>".into(),
        "ShortArray" => "Vec<i16>".into(),
        "ByteArray" => "Vec<i8>".into(),
        "DoubleArray" => "Vec<f64>".into(),
        "FloatArray" => "Vec<f32>".into(),
        "BooleanArray" => "Vec<bool>".into(),
        _ => {
            let (raw, inner) = t.strip_suffix('>')?.split_once('<')?;
            match raw {
                "List" | "MutableList" | "ArrayList" | "Collection" | "Iterable" | "Array" => {
                    format!("Vec<{}>", rust_type(inner)?)
                }
                "Map" | "MutableMap" | "HashMap" | "LinkedHashMap" => {
                    format!("std::collections::HashMap<String, {}>", rust_type(inner.strip_prefix("String,")?)?)
                }
                _ => return None,
            }
        }
    })
}

/// A Kotlin type in Java syntax, for the bridge to convert the port's result back to.
fn java_type(t: &str, boxed: bool) -> String {
    let t: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    let t = t.trim_start_matches("kotlin.collections.").trim_start_matches("kotlin.");
    if let Some(inner) = t.strip_suffix('?') {
        return java_type(inner, true);
    }
    let primitive = |p: &str, b: &str| if boxed { b.to_string() } else { p.to_string() };
    match t {
        "Int" => primitive("int", "Integer"),
        "Long" => primitive("long", "Long"),
        "Short" => primitive("short", "Short"),
        "Byte" => primitive("byte", "Byte"),
        "Double" => primitive("double", "Double"),
        "Float" => primitive("float", "Float"),
        "Boolean" => primitive("boolean", "Boolean"),
        "String" => "String".into(),
        "IntArray" => "int[]".into(),
        "LongArray" => "long[]".into(),
        "ShortArray" => "short[]".into(),
        "ByteArray" => "byte[]".into(),
        "DoubleArray" => "double[]".into(),
        "FloatArray" => "float[]".into(),
        "BooleanArray" => "boolean[]".into(),
        _ => match t.strip_suffix('>').and_then(|x| x.split_once('<')) {
            Some(("List" | "MutableList" | "ArrayList" | "Collection" | "Iterable", inner)) => {
                format!("java.util.List<{}>", java_type(inner, true))
            }
            Some(("Array", inner)) => format!("{}[]", java_type(inner, true)),
            Some(("Map" | "MutableMap" | "HashMap" | "LinkedHashMap", inner)) => match split_top(inner).as_slice() {
                [k, v] => format!("java.util.Map<{}, {}>", java_type(k, true), java_type(v, true)),
                _ => "Object".into(),
            },
            _ => "Object".into(),
        },
    }
}

pub const FLAVOR: Flavor = Flavor {
    lang: Lang::Kotlin,
    fence: "kotlin",
    rust_type,
    default_json: |_| None,
    rules: &[
        "Kotlin `Int` is `i32` and wraps on overflow (as on the JVM): use `wrapping_*` where the original can overflow. `Long` is `i64`.",
        "Kotlin Strings are UTF-16: `length`, indexes and `Char` count UTF-16 units. If the code indexes strings, work on `encode_utf16()` (or take a fast path when `is_ascii()`).",
        "`lowercase()` / `uppercase()` don't depend on the locale; integer `/` and `%` truncate toward zero, as in Rust.",
        "Lists and arrays arrive as JSON arrays, Maps as objects with String keys, null as null (`Option<T>` for `T?`). For an extension function, the receiver is the first argument.",
    ],
};

pub fn scaffold(ctx: &Ctx, hs: &Hotspot, helpers: &[Hotspot], port: &Port) -> Result<String> {
    native::scaffold(ctx, hs, helpers, port, &FLAVOR)
}

pub fn verify(ctx: &Ctx, port: &Port) -> Result<Verify> {
    native::verify(ctx, port, false)
}

// ------------------------------------------------------------------ the bridge

fn bridge_class(port: &Port) -> String {
    format!("FastAnything_{}", util::slug(&port.id))
}

fn hotspot_of<'a>(ctx: &'a Ctx, port: &Port) -> Result<&'a Hotspot> {
    ctx.state
        .hotspots
        .iter()
        .find(|h| h.id == port.id)
        .with_context(|| format!("{} isn't in the last scan: scan again", port.id))
}

pub fn enable(ctx: &Ctx, port: &Port) -> Result<()> {
    if let Some(release) = java::declared_release(&ctx.root).filter(|r| *r < 22) {
        bail!(
            "the build compiles for Java {release}; the Rust bridge uses the FFM API from Java 22. Raise the JVM toolchain to 22+"
        );
    }
    let hs = hotspot_of(ctx, port)?;
    let file = ctx.root.join(&port.file);
    let text = fs::read_to_string(&file).with_context(|| format!("reading {}", port.file))?;
    let package = package_of(&text);
    let dir = java_dir(&file, &package)?;
    let class = bridge_class(port);
    let (_, lib) = port.lang.artifact(&port.ext);
    let lib = util::forward_slashes(&ctx.lib_dir().join(lib));
    let package_line = if package.is_empty() { String::new() } else { format!("package {package};") };
    let bridge = java::BRIDGE
        .replace("/*PACKAGE*/", &package_line)
        .replace("/*IMPORTS*/", "")
        .replace("/*CLASS*/", &class)
        .replace("/*RET*/", &java_type(&ret(hs), false))
        .replace("/*LIB*/", &serde_json::to_string(&lib)?)
        .replace("/*ID*/", &port.id)
        .replace("/*JSON*/", java::JSON);
    let prologue = format!(
        "if ({class}.ON) {{\n    @Suppress(\"UNCHECKED_CAST\")\n    val __faR = {class}.call({args}) as {ret}\n    return __faR\n}}",
        args = args_array(hs),
        ret = ret(hs),
    );
    let patched = native::insert_in_body(&text, "//", &port.id, &prologue, find_body(&port.func, hs.line))?;
    create_dirs(&dir)?;
    fs::write(dir.join(format!("{class}.java")), bridge)?;
    fs::write(&file, patched).with_context(|| format!("writing {}", port.file))
}

pub fn disable(ctx: &Ctx, port: &Port) -> Result<()> {
    let file = ctx.root.join(&port.file);
    let text = fs::read_to_string(&file).with_context(|| format!("reading {}", port.file))?;
    fs::write(&file, native::strip_in_body(&text, "//", &port.id)).with_context(|| format!("writing {}", port.file))?;
    let dir = java_dir(&file, &package_of(&text))?;
    let bridge = dir.join(format!("{}.java", bridge_class(port)));
    if bridge.exists() {
        fs::remove_file(bridge)?;
        prune(&dir);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn describe_text(name: &str, text: &str) -> Vec<Method> {
        let dir =
            std::env::temp_dir().join(format!("fa-kt-{}-{name}", std::process::id())).join("src/main/kotlin/demo");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join(name);
        fs::write(&file, text).unwrap();
        let out = describe_file(&file);
        let _ = fs::remove_dir_all(dir.ancestors().nth(4).unwrap());
        out
    }

    #[test]
    fn reads_kotlin_functions() {
        let ms = describe_text(
            "fuzzy.kt",
            "package demo\n\n// fun commented(x: Int): Int { return x }\nval s = \"fun inString(x: Int) { ${ \"}\" }\"\n\n\
             fun levenshtein(a: String, b: String): Int {\n    return a.length + helper(b)\n}\n\n\
             private fun helper(s: String) = s.length\n\n\
             fun String.shout(times: Int = 2): String {\n    return this.repeat(times)\n}\n\n\
             object Stats {\n    @JvmStatic\n    fun mean(xs: List<Double>): Double {\n        val f = fun(y: Int): Int { return y }\n        return xs.sum() / xs.size\n    }\n}\n\n\
             class Box(val v: Int) {\n    fun get(): Int { return v }\n    companion object {\n        fun make(v: Int): Box { return Box(v) }\n    }\n}\n\n\
             suspend fun later(x: Int): Int { return x }\nfun log(x: Int) { println(x) }\n",
        );
        let by = |n: &str| {
            ms.iter()
                .find(|m| m.name == n)
                .unwrap_or_else(|| panic!("no {n}: {:?}", ms.iter().map(|m| &m.name).collect::<Vec<_>>()))
        };
        let names: Vec<&str> = ms.iter().map(|m| m.name.as_str()).collect();
        assert!(!names.contains(&"commented") && !names.contains(&"inString") && !names.contains(&"get"), "{names:?}");
        let lev = by("levenshtein");
        assert_eq!((lev.class.as_str(), lev.module.as_deref(), lev.line), ("demo.FuzzyKt", Some("demo"), 6));
        assert_eq!(
            lev.params.iter().map(|p| (p.name.as_str(), p.annotation.as_deref().unwrap())).collect::<Vec<_>>(),
            [("a", "String"), ("b", "String")]
        );
        assert!(lev.blockers.is_empty() && lev.calls == ["helper"], "{:?} {:?}", lev.blockers, lev.calls);
        assert!(by("helper").blockers.iter().any(|b| b.starts_with("expression body")));
        let shout = by("shout");
        assert_eq!(shout.params[0].annotation.as_deref(), Some("String"));
        assert_eq!(shout.params[1].default.as_deref(), Some("2"));
        assert_eq!((by("mean").class.as_str(), by("mean").module.as_deref()), ("demo.Stats", Some("demo.Stats")));
        assert_eq!(by("make").class, "demo.Box$Companion");
        assert!(by("later").blockers.contains(&"suspend function".to_string()));
        assert!(
            by("log").blockers.iter().any(|b| b.contains("I/O"))
                && by("log").blockers.iter().any(|b| b.contains("Unit"))
        );
        let named = describe_text("tools.kt", "@file:JvmName(\"Tools\")\npackage demo\nfun one(): Int { return 1 }\n");
        assert_eq!(named[0].class, "demo.Tools");
    }

    #[test]
    fn kotlin_types() {
        assert_eq!(rust_type("List<String>").as_deref(), Some("Vec<String>"));
        assert_eq!(rust_type("Map<String, List<Int>>").as_deref(), Some("std::collections::HashMap<String, Vec<i32>>"));
        assert_eq!(rust_type("IntArray?").as_deref(), Some("Option<Vec<i32>>"));
        assert_eq!(rust_type("Point"), None);
        assert_eq!(java_type("List<List<Int>>", false), "java.util.List<java.util.List<Integer>>");
        assert_eq!(java_type("Int", false), "int");
        assert_eq!(java_type("Double?", false), "Double");
        assert_eq!(java_type("Map<String, Long>", false), "java.util.Map<String, Long>");
        assert_eq!(java_type("Array<String>", false), "String[]");
    }

    #[test]
    fn prologues_call_the_function_again() {
        let hs = |params: Vec<Param>, ret: &str| Hotspot {
            name: "f".into(),
            params,
            returns: Some(ret.into()),
            ..Hotspot::default()
        };
        let p = |n: &str, t: &str| Param { name: n.into(), annotation: Some(t.into()), ..Param::default() };
        let plain = hs(vec![p("a", "List<String>"), p("n", "Int")], "Int");
        assert_eq!(args_array(&plain), "arrayOf<Any?>(a, n)");
        assert_eq!(call_with(&plain), "f(__faA[0] as List<String>, __faA[1] as Int)");
        let ext = hs(vec![p("receiver", "String"), p("n", "Int")], "String");
        assert_eq!(args_array(&ext), "arrayOf<Any?>(this, n)");
        assert_eq!(call_with(&ext), "(__faA[0] as String).f(__faA[1] as Int)");
        assert!(record_prologue(&plain).contains("as Int\n    return __faR"));
        assert!(
            in_kotlin_sources(Path::new("app/src/main/kotlin/demo/A.kt"))
                && !in_kotlin_sources(Path::new("src/test/kotlin/A.kt"))
        );
        assert_eq!(
            java_dir(Path::new("/p/app/src/main/kotlin/demo/A.kt"), "demo.x").unwrap(),
            Path::new("/p/app/src/main/java/demo/x")
        );
    }
}
