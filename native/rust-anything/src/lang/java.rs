//! Java: Maven and Gradle modules, imports, risks, the artifact-to-crate table, and `public
//! static` methods of public classes with Java types mapped into the TS-like type dialect.
//! A unit is a package (a folder).

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
    tree_sitter_java::LANGUAGE.into()
}

const SKIP_DIRS: &[&str] = &[
    "target",
    "build",
    "out",
    ".gradle",
    ".idea",
    "node_modules",
    "bin",
];

// ---- discovery -------------------------------------------------------------------------------

/// `groupId:artifactId` of each `<dependency>` in a pom, or `group:name` of Gradle coordinates.
/// Comments do not count: a commented-out `<dependency>` or `implementation("a:b:1")` is not one
/// the project declares.
pub(crate) fn dependencies(text: &str) -> (Vec<String>, Vec<String>) {
    let (mut deps, mut dev) = (Vec::new(), Vec::new());
    let pom = syntax::without_xml_comments(text);
    for block in pom.split("<dependency>").skip(1) {
        let block = block.split("</dependency>").next().unwrap_or("");
        let tag = |t: &str| {
            block
                .split(&format!("<{t}>"))
                .nth(1)
                .and_then(|r| r.split(&format!("</{t}>")).next())
                .map(str::trim)
                .unwrap_or("")
                .to_string()
        };
        let (group, artifact) = (tag("groupId"), tag("artifactId"));
        if group.is_empty() || artifact.is_empty() {
            continue;
        }
        let name = format!("{group}:{artifact}");
        if tag("scope") == "test" {
            dev.push(name);
        } else {
            deps.push(name);
        }
    }
    static GRADLE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let gradle = GRADLE.get_or_init(|| {
        regex::Regex::new(
            r#"\b(implementation|api|compile|compileOnly|runtimeOnly|testImplementation|testCompile|testRuntimeOnly|androidTestImplementation)\s*\(?\s*['"]([^:'"\s]+:[^:'"\s]+)"#,
        )
        .expect("valid regex")
    });
    for c in gradle.captures_iter(&syntax::without_slash_comments(text)) {
        let name = c[2].to_string();
        if c[1].starts_with("test") || c[1].starts_with("androidTest") {
            dev.push(name);
        } else {
            deps.push(name);
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
        if matches!(name, "pom.xml" | "build.gradle" | "build.gradle.kts") {
            let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
            let (deps, dev) = dependencies(&text);
            let artifact = text
                .split("<artifactId>")
                .nth(1)
                .and_then(|r| r.split("</artifactId>").next())
                .map(str::to_string);
            packages.push(Package {
                name: artifact.unwrap_or_else(|| {
                    if dir.is_empty() {
                        root.file_name()
                            .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned())
                    } else {
                        dir.clone()
                    }
                }),
                src_root: if root.join(&dir).join("src/main/java").is_dir() {
                    join(&dir, "src/main/java")
                } else {
                    dir.clone()
                },
                dir,
                deps,
                dev_deps: dev,
                namespaces: Vec::new(),
            });
        } else if name.ends_with(".java") {
            let is_test = rel.contains("src/test/")
                || name.ends_with("Test.java")
                || name.ends_with("Tests.java")
                || name.ends_with("IT.java");
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
            src_root: if root.join("src/main/java").is_dir() {
                "src/main/java".into()
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
        "import_declaration" => {
            let spec = text(node, source)
                .trim_start_matches("import")
                .trim()
                .trim_start_matches("static ")
                .trim_end_matches(';')
                .trim()
                .to_string();
            match spec.as_str() {
                s if s.starts_with("java.lang.reflect") => add(&mut facts, Risk::Reflection, node),
                s if s.starts_with("java.util.concurrent") => {
                    add(&mut facts, Risk::Concurrency, node)
                }
                s if s.starts_with("java.util.regex") => add(&mut facts, Risk::RegexSyntax, node),
                "java.util.HashMap" | "java.util.HashSet" => {
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
        "synchronized_statement" => add(&mut facts, Risk::Concurrency, node),
        "modifiers"
            if text(node, source).contains("synchronized")
                || text(node, source).contains("volatile") =>
        {
            add(&mut facts, Risk::Concurrency, node)
        }
        "null_literal" => add(&mut facts, Risk::NullReferences, node),
        "instanceof_expression" | "cast_expression" => add(&mut facts, Risk::DynamicTyping, node),
        "lambda_expression" | "method_reference" => add(&mut facts, Risk::Callbacks, node),
        "throw_statement" | "try_statement" | "catch_clause" => {
            add(&mut facts, Risk::Exceptions, node)
        }
        "superclass" | "super_interfaces" => add(&mut facts, Risk::ClassInheritance, node),
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
        "method_invocation" => {
            let name = node
                .child_by_field_name("name")
                .map_or("", |n| text(n, source));
            match name {
                "substring" | "charAt" | "indexOf" | "lastIndexOf" | "codePointAt" | "length" => {
                    add(&mut facts, Risk::StringOffsets, node)
                }
                "getClass" | "forName" | "getDeclaredMethod" | "invoke" | "newInstance" => {
                    add(&mut facts, Risk::Reflection, node)
                }
                "matches" | "replaceAll" | "replaceFirst" | "split" | "compile" => {
                    // Java regex is close to the `regex` crate; lookaround and backreferences are not.
                    let pattern = node
                        .child_by_field_name("arguments")
                        .and_then(|a| named_children(a).into_iter().next())
                        .map(|a| text(a, source).to_string());
                    match pattern {
                        Some(p) if p.starts_with('"') => {
                            let unescaped = p.trim_matches('"').replace("\\\\", "\\");
                            if regex_needs_fancy(&unescaped, "") {
                                add(&mut facts, Risk::RegexSyntax, node);
                            }
                        }
                        _ => add(&mut facts, Risk::DynamicRegex, node),
                    }
                }
                "hashCode" => add(&mut facts, Risk::IntegerSemantics, node),
                _ => {}
            }
        }
        "for_statement" | "enhanced_for_statement" | "while_statement" | "do_statement" => {
            facts.loops += 1
        }
        _ => {}
    });
    facts
}

pub fn resolve(spec: &str, index: &HashMap<&str, usize>, ws: &Workspace) -> super::Resolution {
    let first = spec.split('.').next().unwrap_or("");
    if matches!(first, "java" | "javax" | "jdk" | "sun") || spec.starts_with("com.sun.") {
        return super::Resolution::Stdlib;
    }
    let path = spec.trim_end_matches(".*").replace('.', "/");
    for p in &ws.packages {
        // `import a.b.C` names a file; `import a.b.*` a folder.
        let file = join(&p.src_root, &format!("{path}.java"));
        if let Some(i) = index.get(file.as_str()) {
            return super::Resolution::File(*i);
        }
        let dir = join(&p.src_root, &path);
        if let Some((_, i)) = index
            .iter()
            .filter(|(f, _)| f.rsplit_once('/').is_some_and(|(d, _)| d == dir))
            .min_by_key(|(f, _)| **f)
        {
            return super::Resolution::File(*i);
        }
    }
    // The table's Java rows are keyed by import prefix (`com.google.common` for Guava), so the
    // longest configured prefix is the library; two bare segments (`com.google`) name nothing.
    // `lang/kotlin.rs` already resolves this way.
    let external = crate::crates::longest_prefix("java", spec)
        .map(str::to_string)
        .unwrap_or_else(|| spec.split('.').take(2).collect::<Vec<_>>().join("."));
    super::Resolution::External(external)
}

/// Rust counterparts of common Java libraries, by import prefix or Maven coordinates.
pub fn lookup(name: &str) -> Option<CrateMatch> {
    let key = name.split(':').next().unwrap_or(name);
    crate::crates::find("java", key)
}

// ---- exports ---------------------------------------------------------------------------------

/// A Java type in the TS-like dialect, or why the bridge cannot carry it.
pub fn java_to_ts(t: &str) -> Result<String, String> {
    let t = t.trim();
    if let Some(inner) = t.strip_suffix("[]") {
        return java_to_ts(inner).map(|i| format!("{i}[]"));
    }
    let (head, args) = match t.find('<') {
        Some(i) if t.ends_with('>') => (&t[..i], split_top(&t[i + 1..t.len() - 1])),
        _ => (t, Vec::new()),
    };
    let head = head.trim().rsplit('.').next().unwrap_or(head);
    let arg = |i: usize| -> Result<String, String> {
        args.get(i)
            .map_or_else(|| Err(format!("raw `{head}`")), |a| java_to_ts(a))
    };
    Ok(match head {
        "String" | "CharSequence" => "string".into(),
        "int" | "long" | "short" | "byte" | "Integer" | "Long" | "Short" | "Byte" => "int".into(),
        "double" | "float" | "Double" | "Float" => "number".into(),
        "boolean" | "Boolean" => "boolean".into(),
        "void" | "Void" => "void".into(),
        "List" | "ArrayList" | "Collection" | "Iterable" | "LinkedList" => {
            let inner = arg(0)?;
            if inner.contains(' ') {
                format!("({inner})[]")
            } else {
                format!("{inner}[]")
            }
        }
        "Map" | "LinkedHashMap" | "TreeMap" | "HashMap" => {
            if arg(0)? != "string" {
                return Err(format!("`{t}`: JSON object keys are strings"));
            }
            format!("Record<string, {}>", arg(1)?)
        }
        "Optional" => format!("{} | undefined", arg(0)?),
        "OptionalInt" | "OptionalLong" => "int | undefined".into(),
        "OptionalDouble" => "number | undefined".into(),
        "char" | "Character" => return Err("uses `char`, a UTF-16 code unit".into()),
        "Object" => return Err("takes or returns `Object`: values of any type".into()),
        "Function" | "BiFunction" | "Supplier" | "Consumer" | "Predicate" | "Runnable"
        | "Callable" | "Comparator" => {
            return Err("takes or returns a function".into());
        }
        "Stream" | "Iterator" | "IntStream" => {
            return Err(format!("uses `{head}`, a lazy sequence"));
        }
        other => return Err(format!("uses the class `{other}`")),
    })
}

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

/// The `package` of a Java file.
pub fn package_name(source: &str) -> Option<String> {
    let tree = syntax::parse(&language(), source)?;
    package_name_in(tree.root_node(), source)
}

fn package_name_in(root: Node<'_>, source: &str) -> Option<String> {
    let declaration = named_children(root)
        .into_iter()
        .find(|node| node.kind() == "package_declaration")?;
    let name = named_children(declaration)
        .into_iter()
        .find(|node| matches!(node.kind(), "identifier" | "scoped_identifier"))?;
    let mut components = Vec::new();
    syntax::walk(name, |node| {
        if node.kind() == "identifier" {
            components.push(text(node, source));
        }
    });
    Some(components.join("."))
}

/// `public static` methods of the file's public top-level classes. `prototype` holds the
/// declaring class's fully qualified name and the signature: `com.x.Strings.slugify(String text)`.
pub fn exports(path: &str, source: &str) -> Vec<ExportedFn> {
    if !path.ends_with(".java") {
        return Vec::new();
    }
    let Some(tree) = syntax::parse(&language(), source) else {
        return Vec::new();
    };
    let package = package_name_in(tree.root_node(), source);
    let mut out = Vec::new();
    for class in named_children(tree.root_node()) {
        if class.kind() != "class_declaration" {
            continue;
        }
        let modifiers = |n: Node| {
            named_children(n)
                .into_iter()
                .find(|c| c.kind() == "modifiers")
                .map_or(String::new(), |m| text(m, source).to_string())
        };
        if !modifiers(class).split_whitespace().any(|w| w == "public") {
            continue;
        }
        let Some(class_name) = class
            .child_by_field_name("name")
            .map(|n| text(n, source).to_string())
        else {
            continue;
        };
        let qualified = match &package {
            Some(p) => format!("{p}.{class_name}"),
            None => class_name.clone(),
        };
        let Some(body) = class.child_by_field_name("body") else {
            continue;
        };
        for method in named_children(body) {
            if method.kind() != "method_declaration" {
                continue;
            }
            let mods = modifiers(method);
            let words: Vec<&str> = mods.split_whitespace().collect();
            if !(words.contains(&"public") && words.contains(&"static")) {
                continue;
            }
            let Some(name) = method
                .child_by_field_name("name")
                .map(|n| text(n, source).to_string())
            else {
                continue;
            };
            let mut blocker = None;
            if method.child_by_field_name("type_parameters").is_some() {
                blocker = Some("generic: erased types cannot be recovered".to_string());
            }
            let mut params = Vec::new();
            let mut signature = Vec::new();
            if let Some(list) = method.child_by_field_name("parameters") {
                for p in named_children(list) {
                    let (ty, pname) = match p.kind() {
                        "formal_parameter" => (
                            p.child_by_field_name("type")
                                .map_or("", |t| text(t, source))
                                .to_string(),
                            p.child_by_field_name("name")
                                .map_or("", |n| text(n, source))
                                .to_string(),
                        ),
                        "spread_parameter" => {
                            blocker.get_or_insert_with(|| "is variadic".into());
                            continue;
                        }
                        _ => continue,
                    };
                    let ts_type = java_to_ts(&ty).unwrap_or_else(|reason| {
                        blocker.get_or_insert(reason);
                        "unknown".into()
                    });
                    signature.push(format!("{ty} {pname}"));
                    params.push(Param {
                        name: pname,
                        ts_type,
                        native_type: Some(ty),
                        ..Default::default()
                    });
                }
            }
            let ret = method
                .child_by_field_name("type")
                .map_or("void", |t| text(t, source))
                .to_string();
            let returns = java_to_ts(&ret).unwrap_or_else(|reason| {
                blocker.get_or_insert(reason);
                "unknown".into()
            });
            if out.iter().any(|f: &ExportedFn| f.name == name) {
                blocker = Some("overloaded: one name, several signatures".into());
            }
            out.push(ExportedFn {
                name,
                file: path.to_string(),
                line: syntax::line(method),
                params,
                returns,
                native_return: Some(ret),
                prototype: Some(format!(
                    "{qualified}.{}({})",
                    text(method.child_by_field_name("name").unwrap_or(method), source),
                    signature.join(", ")
                )),
                blocker,
                ..Default::default()
            });
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
    out
}

/// The declaring class and the method name of an export: `("com.x.Strings", "slugify")`.
pub fn receiver(f: &ExportedFn) -> (String, String) {
    let proto = f.prototype.as_deref().unwrap_or(&f.name);
    let head = proto.split('(').next().unwrap_or(proto);
    match head.rsplit_once('.') {
        Some((class, name)) => (class.to_string(), name.to_string()),
        None => (String::new(), f.name.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_types() {
        assert_eq!(java_to_ts("List<String>").unwrap(), "string[]");
        assert_eq!(
            java_to_ts("Map<String, List<Integer>>").unwrap(),
            "Record<string, int[]>"
        );
        assert_eq!(java_to_ts("double[]").unwrap(), "number[]");
        assert!(java_to_ts("Point").is_err());
    }

    #[test]
    fn finds_public_static_methods() {
        let src = "package com.example.text;\nimport java.util.List;\npublic final class Strings {\n\
                   private Strings() {}\n\
                   public static String slugify(String text, int max) { return text; }\n\
                   public static List<String> words(final String text) { return null; }\n\
                   public static <T> T id(T x) { return x; }\n\
                   static int hidden() { return 1; }\n\
                   public int instance() { return 1; }\n}\n";
        let fns = exports("src/main/java/com/example/text/Strings.java", src);
        let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["slugify", "words", "id"]);
        assert_eq!(fns[0].params[1].ts_type, "int");
        assert_eq!(
            receiver(&fns[0]),
            ("com.example.text.Strings".into(), "slugify".into())
        );
        assert_eq!(fns[1].returns, "string[]");
        assert!(fns[2].blocker.is_some());
    }

    #[test]
    fn every_overload_of_a_name_is_blocked() {
        let src = "package com.example;\npublic final class Nums {\n\
                   public static int add(int a, int b) { return a + b; }\n\
                   public static double add(double a, double b) { return a + b; }\n\
                   public static int one() { return 1; }\n}\n";
        let fns = exports("src/main/java/com/example/Nums.java", src);
        let blocked: Vec<(&str, bool)> = fns
            .iter()
            .map(|f| (f.name.as_str(), f.blocker.is_some()))
            .collect();
        assert_eq!(blocked, [("add", true), ("add", true), ("one", false)]);
    }
}
