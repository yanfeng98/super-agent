//! Kotlin: Gradle Kotlin DSL modules, imports, risks, the artifact-to-crate table, and the
//! JVM-visible functions a Kotlin file exposes.
//!
//! Kotlin has no `static` keyword, but it has two ways to produce a JVM static method:
//! a top-level function (compiled into `<File>Kt`) and `@JvmStatic` in an `object` or a
//! `companion object`. Both are exported here with a Java-shaped `prototype`, so the
//! existing Java JNI codegen, batch codegen, reflection driver and shadow generation can
//! be reused without changes. A Java wrapper can call a Kotlin static method directly.

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
    tree_sitter_kotlin_ng::LANGUAGE.into()
}

const SKIP_DIRS: &[&str] = &[
    "build",
    ".gradle",
    ".idea",
    "out",
    "target",
    "node_modules",
    "bin",
    ".kotlin",
];

// ---- discovery -------------------------------------------------------------------------------

pub fn discover(root: &Path) -> Result<Workspace> {
    let mut packages: Vec<Package> = Vec::new();
    let mut files = Vec::new();
    let walker = crate::walk::filtered(root, |e| {
        if !e.file_type().is_some_and(|t| t.is_dir())
            || !SKIP_DIRS.contains(&e.file_name().to_string_lossy().as_ref())
        {
            return true;
        }
        // `src/main/kotlin/build/` can be a real package; other `build/` folders are output.
        !e.path()
            .components()
            .any(|c| c.as_os_str() == std::ffi::OsStr::new("src"))
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
        if matches!(name, "pom.xml" | "build.gradle" | "build.gradle.kts") {
            let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
            let (deps, dev) = super::java::dependencies(&text);
            packages.push(Package {
                name: if dir.is_empty() {
                    root.file_name()
                        .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned())
                } else {
                    dir.clone()
                },
                src_root: if root.join(&dir).join("src/main/kotlin").is_dir() {
                    join(&dir, "src/main/kotlin")
                } else if root.join(&dir).join("src/main/java").is_dir() {
                    join(&dir, "src/main/java")
                } else {
                    dir.clone()
                },
                dir,
                deps,
                dev_deps: dev,
                namespaces: Vec::new(),
            });
        } else if name.ends_with(".kt") {
            let is_test = rel.contains("src/test/")
                || rel.contains("src/androidTest/")
                || name.ends_with("Test.kt");
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
    if packages.is_empty() {
        packages.push(Package {
            name: root
                .file_name()
                .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned()),
            dir: String::new(),
            src_root: if root.join("src/main/kotlin").is_dir() {
                "src/main/kotlin".into()
            } else if root.join("src").is_dir() {
                "src".into()
            } else {
                String::new()
            },
            deps: Vec::new(),
            dev_deps: Vec::new(),
            namespaces: Vec::new(),
        });
    }
    packages.sort_by(|a, b| a.dir.cmp(&b.dir));
    packages.dedup_by(|a, b| {
        let same = a.dir == b.dir;
        if same {
            b.deps.append(&mut a.deps);
            b.dev_deps.append(&mut a.dev_deps);
        }
        same
    });
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
        "import" if node.is_named() => {
            let spec = text(node, source)
                .trim_start_matches("import")
                .trim()
                .to_string();
            match spec.as_str() {
                s if s.starts_with("kotlinx.coroutines") => {
                    add(&mut facts, Risk::Concurrency, node)
                }
                s if s.starts_with("kotlin.reflect") => add(&mut facts, Risk::Reflection, node),
                s if s.starts_with("java.util.regex") || s.starts_with("kotlin.text.Regex") => {
                    add(&mut facts, Risk::RegexSyntax, node)
                }
                s if s.contains("HashMap") || s.contains("HashSet") => {
                    add(&mut facts, Risk::UnorderedIteration, node)
                }
                _ => {}
            }
            facts.imports.push(Import {
                specifier: spec,
                type_only: false,
                dynamic: false,
            });
        }
        "nullable_type" => add(&mut facts, Risk::NullReferences, node),
        "throw_expression" | "try_expression" => add(&mut facts, Risk::Exceptions, node),
        "is_expression" | "type_test" | "as_expression" => {
            add(&mut facts, Risk::DynamicTyping, node)
        }
        "lambda_parameters" => add(&mut facts, Risk::Callbacks, node),
        "delegation_specifiers" => add(&mut facts, Risk::ClassInheritance, node),
        "annotation" => {
            let a = text(node, source);
            if a.contains("Synchronized") || a.contains("Volatile") {
                add(&mut facts, Risk::Concurrency, node);
            }
        }
        "binary_expression" => {
            let op = node
                .child_by_field_name("operator")
                .map_or("", |o| text(o, source));
            match op {
                "/" | "%" => add(&mut facts, Risk::DivisionSemantics, node),
                "<<" | ">>" | ">>>" | "*" => add(&mut facts, Risk::IntegerSemantics, node),
                _ => {}
            }
        }
        "for_statement" | "while_statement" | "do_while_statement" => facts.loops += 1,
        "call_expression" => {
            let callee = text(node, source);
            for (needle, risk) in [
                ("substring", Risk::StringOffsets),
                ("charAt", Risk::StringOffsets),
                ("indexOf", Risk::StringOffsets),
                ("toInt(", Risk::IntegerSemantics),
                ("toLong(", Risk::IntegerSemantics),
                ("getClass", Risk::Reflection),
                ("javaClass", Risk::Reflection),
                ("forName", Risk::Reflection),
                ("getDeclaredMethod", Risk::Reflection),
                ("newInstance", Risk::Reflection),
                ("::class", Risk::Reflection),
                ("synchronized(", Risk::Concurrency),
            ] {
                if callee.contains(needle) {
                    add(&mut facts, risk, node);
                }
            }
            if callee.contains("Regex(") {
                let pattern = callee.split("Regex(").nth(1).unwrap_or("");
                if regex_needs_fancy(pattern, "") {
                    add(&mut facts, Risk::RegexSyntax, node);
                }
            }
        }
        _ => {}
    });
    facts
}

pub fn resolve(spec: &str, index: &HashMap<&str, usize>, ws: &Workspace) -> super::Resolution {
    // `import a.b.C as Alias` keeps the dotted path, not the local alias.
    let spec = spec.split(" as ").next().unwrap_or(spec).trim();
    let wildcard = spec.ends_with(".*");
    let first = spec.split('.').next().unwrap_or("");
    if matches!(first, "kotlin" | "java" | "javax" | "jdk" | "sun") || spec.starts_with("com.sun.")
    {
        return super::Resolution::Stdlib;
    }
    let path = spec.trim_end_matches(".*").replace('.', "/");
    let find = |p: &str, packages: &[Package]| -> Option<usize> {
        for pkg in packages {
            let file = join(&pkg.src_root, &format!("{p}.kt"));
            if let Some(i) = index.get(file.as_str()) {
                return Some(*i);
            }
            // Kotlin `package a.b` maps to a folder; find any file in that folder.
            let dir = join(&pkg.src_root, p);
            if let Some((_, i)) = index
                .iter()
                .filter(|(f, _)| f.rsplit_once('/').is_some_and(|(d, _)| d == dir))
                .min_by_key(|(f, _)| **f)
            {
                return Some(*i);
            }
        }
        None
    };
    if let Some(i) = find(&path, &ws.packages) {
        return super::Resolution::File(i);
    }
    // A class import can also resolve to a file in its parent package.
    if !wildcard
        && let Some((parent, _)) = path.rsplit_once('/')
        && let Some(i) = find(parent, &ws.packages)
    {
        return super::Resolution::File(i);
    }
    let external = crate::crates::longest_prefix("kotlin", spec)
        .map(str::to_string)
        .unwrap_or_else(|| spec.split('.').take(2).collect::<Vec<_>>().join("."));
    super::Resolution::External(external)
}

pub fn lookup(name: &str) -> Option<CrateMatch> {
    let key = name.split(':').next().unwrap_or(name);
    crate::crates::find("kotlin", key)
}

// ---- types -----------------------------------------------------------------------------------

/// Split a generic argument list at top-level commas.
fn split_top(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for c in text.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
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

/// A Kotlin type in the TS-like dialect, or why the bridge cannot carry it.
pub fn kotlin_to_ts(t: &str) -> Result<String, String> {
    let t = t.trim();
    if let Some(inner) = t.strip_suffix('?') {
        return kotlin_to_ts(inner).map(|i| format!("{i} | undefined"));
    }
    if let Some(inner) = t.strip_suffix("[]") {
        return kotlin_to_ts(inner).map(|i| format!("{i}[]"));
    }
    let (head, args) = match t.find('<') {
        Some(i) if t.ends_with('>') => (&t[..i], split_top(&t[i + 1..t.len() - 1])),
        _ => (t, Vec::new()),
    };
    let head = head.trim().rsplit('.').next().unwrap_or(head);
    let arg = |i: usize| -> Result<String, String> {
        args.get(i)
            .map_or_else(|| Err(format!("raw `{head}`")), |a| kotlin_to_ts(a))
    };
    Ok(match head {
        "String" | "CharSequence" => "string".into(),
        "Int" | "Long" | "Short" | "Byte" => "int".into(),
        "Float" | "Double" => "number".into(),
        "Boolean" => "boolean".into(),
        "Unit" | "Nothing" => "void".into(),
        "List" | "MutableList" | "Collection" | "Iterable" | "Array" | "IntArray" | "LongArray"
        | "DoubleArray" | "BooleanArray" => {
            let inner = if head.ends_with("Array") && !matches!(head, "Array") {
                match head {
                    "IntArray" => "int".into(),
                    "LongArray" => "int".into(),
                    "DoubleArray" => "number".into(),
                    "BooleanArray" => "boolean".into(),
                    _ => arg(0)?,
                }
            } else {
                arg(0)?
            };
            if inner.contains(' ') {
                format!("({inner})[]")
            } else {
                format!("{inner}[]")
            }
        }
        "Map" | "MutableMap" | "LinkedHashMap" => {
            if arg(0)? != "string" {
                return Err(format!("`{t}`: JSON object keys are strings"));
            }
            format!("Record<string, {}>", arg(1)?)
        }
        "Char" => return Err("uses `Char`, a UTF-16 code unit".into()),
        "Any" | "Any?" => return Err("takes or returns `Any`: values of any type".into()),
        "Set" | "MutableSet" | "HashSet" => {
            return Err(format!("uses `{head}`, an unordered collection"));
        }
        "Sequence" | "Iterator" => return Err(format!("uses `{head}`, a lazy sequence")),
        "Pair" | "Triple" => return Err(format!("uses `{head}`, no tuple in the dialect")),
        "Function0" | "Function1" | "Function2" | " suspend " => {
            return Err("takes or returns a function".into());
        }
        other if other.contains(" -> ") => return Err("takes or returns a function".into()),
        other => return Err(format!("uses the class `{other}`")),
    })
}

// ---- exports ---------------------------------------------------------------------------------

/// A Kotlin type as a Java type the generated wrappers can declare and reflection can read.
/// `None` when the type has no Java counterpart in the bridged subset.
fn kotlin_to_java(t: &str) -> Option<String> {
    let t = t.trim();
    let boxed = |s: String| match s.as_str() {
        "int" => "Integer".into(),
        "long" => "Long".into(),
        "short" => "Short".into(),
        "byte" => "Byte".into(),
        "float" => "Float".into(),
        "double" => "Double".into(),
        "boolean" => "Boolean".into(),
        "char" => "Character".into(),
        _ => s,
    };
    if let Some(inner) = t.strip_suffix('?') {
        return kotlin_to_java(inner).map(boxed);
    }
    if let Some(inner) = t.strip_suffix("[]") {
        return kotlin_to_java(inner).map(|i| format!("{}[]", boxed(i)));
    }
    let (head, args) = match t.find('<') {
        Some(i) if t.ends_with('>') => (&t[..i], split_top(&t[i + 1..t.len() - 1])),
        _ => (t, Vec::new()),
    };
    let head = head.trim().rsplit('.').next().unwrap_or(head);
    let arg =
        |i: usize| -> Option<String> { args.get(i).and_then(|a| kotlin_to_java(a)).map(boxed) };
    Some(match head {
        "String" | "CharSequence" => "String".into(),
        "Int" => "int".into(),
        "Long" => "long".into(),
        "Short" => "short".into(),
        "Byte" => "byte".into(),
        "Float" => "float".into(),
        "Double" => "double".into(),
        "Boolean" => "boolean".into(),
        "Unit" | "Nothing" => "void".into(),
        "List" | "MutableList" | "Collection" | "Iterable" => format!("List<{}>", arg(0)?),
        "Map" | "MutableMap" | "LinkedHashMap" => {
            format!("Map<String, {}>", arg(1)?)
        }
        "Array" => format!("{}[]", arg(0)?),
        "IntArray" => "int[]".into(),
        "LongArray" => "long[]".into(),
        "ShortArray" => "short[]".into(),
        "ByteArray" => "byte[]".into(),
        "FloatArray" => "float[]".into(),
        "DoubleArray" => "double[]".into(),
        "BooleanArray" => "boolean[]".into(),
        _ => return None,
    })
}

fn package_name_in(root: Node<'_>, source: &str) -> Option<String> {
    let header = named_children(root)
        .into_iter()
        .find(|n| n.kind() == "package_header")?;
    named_children(header)
        .into_iter()
        .find(|n| n.kind() == "qualified_identifier")
        .map(|n| text(n, source).replace(' ', ""))
}

fn modifiers(node: Node<'_>, source: &str) -> String {
    const MODIFIER_KINDS: &[&str] = &[
        "modifiers",
        "annotation",
        "function_modifier",
        "member_modifier",
        "visibility_modifier",
        "class_modifier",
        "parameter_modifier",
    ];
    named_children(node)
        .into_iter()
        .filter(|c| MODIFIER_KINDS.contains(&c.kind()))
        .map(|m| text(m, source))
        .collect::<Vec<_>>()
        .join(" ")
}

fn modifier_words(mods: &str) -> impl Iterator<Item = &str> {
    let mut depth = 0usize;
    mods.split(move |c: char| {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        c.is_whitespace() || c == '(' || c == ')' || depth > 0
    })
    .filter(|w| !w.is_empty())
}

fn type_text(node: Node<'_>, source: &str) -> String {
    text(node, source).replace(' ', "")
}

/// Type nodes in the Kotlin grammar: a bare `type`, plus the named variants the grammar
/// wraps around it (`user_type`, `nullable_type`, ...). Parameters and return types carry no
/// `type` field, so they are found by kind.
fn is_type_node(kind: &str) -> bool {
    matches!(
        kind,
        "type"
            | "user_type"
            | "nullable_type"
            | "non_nullable_type"
            | "parenthesized_type"
            | "function_type"
    )
}

/// Collect the exports of one Kotlin declaration container (top level or an `object`).
fn collect(
    container: Node<'_>,
    source: &str,
    owner: &str,
    path: &str,
    default_static: bool,
    out: &mut Vec<ExportedFn>,
) {
    let children = named_children(container);
    // `class_member_declaration` is a hidden supertype in this grammar, so the concrete
    // declarations appear directly under `class_body`.
    let members: Vec<Node> = children
        .iter()
        .flat_map(|n| {
            if n.kind() == "class_body" {
                named_children(*n)
                    .into_iter()
                    .filter(|m| m.kind() == "function_declaration")
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        })
        .collect();
    // A top-level `function_declaration` is its own container; an `object` or `class_body`
    // holds its functions as members.
    let candidates: Vec<Node> = if container.kind() == "function_declaration" {
        vec![container]
    } else {
        members
            .into_iter()
            .filter(|n| n.kind() == "function_declaration")
            .collect()
    };
    for function in candidates {
        let mods = modifiers(function, source);
        let has = |word: &str| modifier_words(&mods).any(|w| w == word);
        if has("private") || has("protected") || has("internal") {
            continue;
        }
        if !default_static && !has("@JvmStatic") {
            continue;
        }
        let Some(name_node) = function.child_by_field_name("name") else {
            continue;
        };
        let name = text(name_node, source).to_string();
        let mut blocker = None;
        // `fun Foo.bar()` needs a dispatched receiver; the JVM owner would not be the file class.
        if source
            .get(function.start_byte()..name_node.start_byte())
            .unwrap_or_default()
            .trim_end()
            .ends_with('.')
        {
            blocker.get_or_insert_with(|| "extension functions need a receiver".into());
        }
        if mods.split_whitespace().any(|w| w == "suspend") {
            blocker = Some("suspend functions keep their continuation in Kotlin".into());
        }
        if mods.split_whitespace().any(|w| w == "inline") {
            blocker.get_or_insert_with(|| "inline functions are not callable over JNI".into());
        }
        if mods.contains("JvmName") {
            blocker.get_or_insert_with(|| "renames the JVM method with `@JvmName`".into());
        }
        if named_children(function)
            .iter()
            .any(|n| n.kind() == "type_parameters")
        {
            blocker.get_or_insert_with(|| "generic: erased types cannot be recovered".into());
        }
        let mut params = Vec::new();
        let mut signature = Vec::new();
        for child in named_children(function) {
            if child.kind() != "function_value_parameters" {
                continue;
            }
            for p in named_children(child) {
                if p.kind() != "parameter" {
                    continue;
                }
                let pchildren = named_children(p);
                let pname = pchildren
                    .iter()
                    .find(|n| n.kind() == "identifier")
                    .map_or(String::new(), |n| text(*n, source).to_string());
                let pmods = modifiers(p, source);
                if pmods.contains("vararg") || pmods.contains("noinline") {
                    blocker.get_or_insert_with(|| "is variadic or takes an inline function".into());
                }
                let ptype = pchildren
                    .iter()
                    .find(|n| is_type_node(n.kind()))
                    .map_or_else(|| "Any".to_string(), |n| type_text(*n, source));
                let ts_type = kotlin_to_ts(&ptype).unwrap_or_else(|reason| {
                    blocker.get_or_insert(reason);
                    "unknown".into()
                });
                let java_type = kotlin_to_java(&ptype).unwrap_or_else(|| {
                    blocker.get_or_insert_with(|| {
                        format!("`{ptype}` has no Java counterpart for the wrapper")
                    });
                    "Object".into()
                });
                signature.push(format!("{ptype} {pname}"));
                params.push(Param {
                    name: pname,
                    ts_type,
                    native_type: Some(java_type),
                    ..Default::default()
                });
            }
        }
        let ret = named_children(function)
            .iter()
            .find(|n| n.start_byte() > name_node.end_byte() && is_type_node(n.kind()))
            .map_or_else(
                || {
                    if source
                        .get(name_node.end_byte()..function.end_byte())
                        .unwrap_or_default()
                        .trim_start()
                        .starts_with('=')
                    {
                        blocker.get_or_insert_with(|| {
                            "expression body has no explicit return type".into()
                        });
                    }
                    "Unit".to_string()
                },
                |n| type_text(*n, source),
            );
        let returns = kotlin_to_ts(&ret).unwrap_or_else(|reason| {
            blocker.get_or_insert(reason);
            "unknown".into()
        });
        let native_return = kotlin_to_java(&ret).unwrap_or_else(|| {
            blocker
                .get_or_insert_with(|| format!("`{ret}` has no Java counterpart for the wrapper"));
            "Object".into()
        });
        if out.iter().any(|f: &ExportedFn| f.name == name) {
            blocker = Some("overloaded: one name, several signatures".into());
        }
        out.push(ExportedFn {
            name,
            file: path.to_string(),
            line: syntax::line(function),
            params,
            returns,
            native_return: Some(native_return),
            prototype: Some(format!(
                "{owner}.{}({})",
                text(name_node, source),
                signature.join(", ")
            )),
            blocker,
            ..Default::default()
        });
    }
}

/// `@file:JvmName("Utils")`, which renames the class a file's top-level functions live on.
fn file_jvm_name(root: Node<'_>, source: &str) -> Option<String> {
    named_children(root)
        .into_iter()
        .filter(|n| n.kind() == "file_annotation")
        .map(|n| text(n, source))
        .find_map(|t| {
            let rest = &t[t.find("JvmName")?..];
            let start = rest.find('"')? + 1;
            let end = start + rest[start..].find('"')?;
            Some(rest[start..end].to_string())
        })
}

/// Whether `name` can be a Java class name, so a wrapper can declare it.
fn is_java_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// JVM-visible functions of a Kotlin file, with Java-shaped prototypes so the Java JNI
/// codegen can be reused unchanged.
pub fn exports(path: &str, source: &str) -> Vec<ExportedFn> {
    if !path.ends_with(".kt") {
        return Vec::new();
    }
    let Some(tree) = syntax::parse(&language(), source) else {
        return Vec::new();
    };
    let root = tree.root_node();
    let package = package_name_in(root, source);
    let stem = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".kt");
    let top_class = file_jvm_name(root, source).unwrap_or_else(|| {
        // Kotlin capitalizes the file's first letter for the default JVM facade:
        // `app.kt` in package `p` becomes `p.AppKt`, not `p.appKt`.
        let mut chars = stem.chars();
        let head = chars
            .next()
            .map(|c| c.to_uppercase().collect::<String>())
            .unwrap_or_default();
        format!("{head}{}Kt", chars.as_str())
    });
    let owner = |class: &str| match &package {
        Some(p) => format!("{p}.{class}"),
        None => class.to_string(),
    };
    let mut out = Vec::new();
    for child in named_children(root) {
        match child.kind() {
            "function_declaration" => {
                // Top-level function: JVM owner is `<File>Kt`, or `@file:JvmName`.
                collect(child, source, &owner(&top_class), path, true, &mut out);
            }
            "object_declaration" => {
                let Some(name) = child.child_by_field_name("name") else {
                    continue;
                };
                collect(
                    child,
                    source,
                    &owner(text(name, source)),
                    path,
                    false,
                    &mut out,
                );
            }
            "class_declaration" => {
                let Some(name) = child.child_by_field_name("name") else {
                    continue;
                };
                let class_owner = owner(text(name, source));
                for member in named_children(child) {
                    if member.kind() == "class_body" {
                        for decl in named_children(member) {
                            if decl.kind() == "object_declaration" {
                                // A nested object has its own JVM class; only a companion
                                // object's statics live on the enclosing class.
                                if let Some(nested) = decl.child_by_field_name("name") {
                                    collect(
                                        decl,
                                        source,
                                        &owner(text(nested, source)),
                                        path,
                                        false,
                                        &mut out,
                                    );
                                }
                            } else if decl.kind() == "companion_object" {
                                collect(decl, source, &class_owner, path, false, &mut out);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    // The reflection driver keys methods by name alone: an overload would pick an arbitrary
    // signature. Block every export sharing a name, not just the later declaration.
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for f in &out {
        *counts.entry(f.name.clone()).or_default() += 1;
    }
    for f in &mut out {
        if counts.get(&f.name).copied().unwrap_or(0) > 1 {
            f.blocker = Some("overloaded: one name, several signatures".into());
        }
    }
    for f in &mut out {
        let (class, _) = receiver(f);
        let simple = class.rsplit('.').next().unwrap_or(&class);
        if !is_java_identifier(simple) {
            f.blocker.get_or_insert_with(|| {
                format!("JVM class `{class}` is not a valid Java name; add `@file:JvmName`")
            });
        }
    }
    out
}

/// The declaring JVM class and method name of an export, e.g. `("com.x.TextKt", "slugify")`.
pub fn receiver(f: &ExportedFn) -> (String, String) {
    crate::lang::java::receiver(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_risks_and_imports() {
        let src =
            "import kotlinx.coroutines.runBlocking\n\nfun f(x: String?): Int = x!!.toInt() / 2\n";
        let facts = scan(src);
        assert!(facts.risks.contains_key(&Risk::Concurrency));
        assert!(facts.risks.contains_key(&Risk::NullReferences));
        assert!(facts.risks.contains_key(&Risk::IntegerSemantics));
        assert!(facts.risks.contains_key(&Risk::DivisionSemantics));
        assert_eq!(facts.imports.len(), 1, "{:?}", facts.imports);
        assert_eq!(facts.parse_errors, 0);
    }

    #[test]
    fn maps_types() {
        assert_eq!(kotlin_to_ts("String?").unwrap(), "string | undefined");
        assert_eq!(kotlin_to_ts("List<String>").unwrap(), "string[]");
        assert_eq!(
            kotlin_to_ts("Map<String, List<Int>>").unwrap(),
            "Record<string, int[]>"
        );
        assert_eq!(kotlin_to_ts("IntArray").unwrap(), "int[]");
        assert_eq!(kotlin_to_ts("Unit").unwrap(), "void");
        assert!(kotlin_to_ts("Char").is_err());
        assert!(kotlin_to_ts("Any").is_err());
        assert!(kotlin_to_ts("Set<String>").is_err());
    }

    #[test]
    fn maps_java_types() {
        assert_eq!(kotlin_to_java("Int").unwrap(), "int");
        assert_eq!(kotlin_to_java("String?").unwrap(), "String");
        assert_eq!(kotlin_to_java("List<Int>").unwrap(), "List<Integer>");
        assert_eq!(
            kotlin_to_java("Map<String, Long>").unwrap(),
            "Map<String, Long>"
        );
        assert_eq!(kotlin_to_java("IntArray").unwrap(), "int[]");
        assert_eq!(kotlin_to_java("Unit").unwrap(), "void");
        assert!(kotlin_to_java("Char").is_none());
        assert!(kotlin_to_java("Foo").is_none());
    }

    #[test]
    fn finds_top_level_and_jvm_static_exports() {
        let src = "package com.example\n\nexternal fun add(a: Int, b: Int): Int\n\
                   private fun hidden() {}\n\
                   object Math {\n  @JvmStatic external fun mul(a: Int, b: Int): Int\n\
                   fun instanceOnly() {}\n}\n";
        let fns = exports("src/main/kotlin/com/example/NativeLib.kt", src);
        assert_eq!(fns.len(), 2, "{fns:?}");
        let by_name = |n: &str| fns.iter().find(|f| f.name == n).unwrap();
        assert_eq!(receiver(by_name("add")).0, "com.example.NativeLibKt");
        assert_eq!(receiver(by_name("mul")).0, "com.example.Math");
    }

    #[test]
    fn annotations_do_not_count_as_modifiers() {
        let src = "package com.example\n\n@Suppress(\"private\") fun shown(a: Int): Int = a\n\
                   @InternalApi fun alsoShown(a: Int): Int = a\n\
                   object Math {\n  @JvmStaticSomething fun notStatic(a: Int): Int = a\n}\n";
        let mut names: Vec<String> = exports("src/main/kotlin/com/example/Lib.kt", src)
            .into_iter()
            .map(|f| f.name)
            .collect();
        names.sort();
        assert_eq!(names, ["alsoShown", "shown"]);
    }

    #[test]
    fn a_companion_object_static_lives_on_the_class() {
        let src = "package com.example\n\nclass Math {\n  companion object {\n    @JvmStatic fun add(a: Int, b: Int): Int = a + b\n  }\n}\n";
        let fns = exports("src/main/kotlin/com/example/Math.kt", src);
        assert_eq!(fns.len(), 1, "{fns:?}");
        assert_eq!(receiver(&fns[0]).0, "com.example.Math");
    }

    #[test]
    fn file_jvm_name_overrides_the_class() {
        let src =
            "@file:JvmName(\"Utils\")\npackage com.example\n\nfun ping(): String = \"pong\"\n";
        let fns = exports("src/main/kotlin/com/example/NativeLib.kt", src);
        assert_eq!(receiver(&fns[0]).0, "com.example.Utils");
    }

    #[test]
    fn an_invalid_jvm_class_name_is_blocked_not_guessed() {
        let src = "package com.example\n\nfun ping(): String = \"pong\"\n";
        let fns = exports("src/main/kotlin/my-file.kt", src);
        assert!(fns[0].blocker.as_deref().unwrap_or("").contains("JvmName"));
    }

    #[test]
    fn blocks_suspend_and_generic() {
        let src = "package com.example\n\nsuspend fun slow(x: String): String = x\n\
                   fun <T> identity(x: T): T = x\n\
                   @JvmName(\"renamed\") fun named(): String = \"\"\n";
        let fns = exports("src/main/kotlin/com/example/X.kt", src);
        assert!(fns.iter().all(|f| f.blocker.is_some()), "{fns:?}");
        assert!(
            fns.iter()
                .any(|f| f.blocker.as_deref().unwrap_or("").contains("JvmName")),
            "{fns:?}"
        );
    }
}
