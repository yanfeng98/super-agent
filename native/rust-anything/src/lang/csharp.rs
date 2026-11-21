//! C#: .csproj projects and their NuGet packages, `using` directives, risks, the
//! package-to-crate table, and `public static` methods of public classes with C# types mapped
//! into the TS-like type dialect. A unit is a folder.

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
    tree_sitter_c_sharp::LANGUAGE.into()
}

const SKIP_DIRS: &[&str] = &[
    "bin",
    "obj",
    ".vs",
    "packages",
    "TestResults",
    "node_modules",
    "artifacts",
];

/// Packages a test project references.
const TEST_PACKAGES: &[&str] = &[
    "xunit",
    "NUnit",
    "MSTest.TestFramework",
    "Microsoft.NET.Test.Sdk",
];

// ---- discovery -------------------------------------------------------------------------------

fn attribute(tag: &str, name: &str) -> Option<String> {
    let start = tag.find(&format!("{name}=\""))? + name.len() + 2;
    let end = tag[start..].find('"')?;
    Some(tag[start..start + end].to_string())
}

/// `Include` of each `<PackageReference>` in a project file.
pub fn package_references(text: &str) -> Vec<String> {
    text.split("<PackageReference")
        .skip(1)
        .filter_map(|tag| attribute(tag.split('>').next().unwrap_or(""), "Include"))
        .collect()
}

fn element(text: &str, tag: &str) -> Option<String> {
    let value = text
        .split(&format!("<{tag}>"))
        .nth(1)?
        .split(&format!("</{tag}>"))
        .next()?
        .trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// A project of tests: it references a test framework or says `IsTestProject`.
pub fn is_test_project(text: &str) -> bool {
    package_references(text)
        .iter()
        .any(|d| TEST_PACKAGES.iter().any(|t| d.starts_with(t)))
        || element(text, "IsTestProject").as_deref() == Some("true")
}

pub fn discover(root: &Path) -> Result<Workspace> {
    let mut packages: Vec<Package> = Vec::new();
    let mut tests: Vec<String> = Vec::new();
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
        let name = rel.rsplit('/').next().unwrap_or(&rel).to_string();
        let dir = rel
            .rsplit_once('/')
            .map_or(String::new(), |(d, _)| d.to_string());
        if let Some(stem) = name.strip_suffix(".csproj") {
            let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
            let deps = package_references(&text);
            let is_test = is_test_project(&text);
            if is_test {
                tests.push(dir.clone());
            }
            let (deps, dev_deps) = if is_test {
                (Vec::new(), deps)
            } else {
                (deps, Vec::new())
            };
            packages.push(Package {
                name: element(&text, "AssemblyName").unwrap_or_else(|| stem.to_string()),
                src_root: dir.clone(),
                dir,
                deps,
                dev_deps,
            });
        } else if name.ends_with(".cs") {
            files.push(rel);
        }
    }
    if packages.is_empty() {
        packages.push(Package {
            name: root
                .file_name()
                .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned()),
            dir: String::new(),
            src_root: String::new(),
            deps: Vec::new(),
            dev_deps: Vec::new(),
        });
    }
    packages.sort_by(|a, b| a.dir.cmp(&b.dir));
    packages.dedup_by(|a, b| a.dir == b.dir);
    files.sort();
    let under = |path: &str, dir: &str| dir.is_empty() || path.starts_with(&format!("{dir}/"));
    let files = files
        .into_iter()
        .map(|path| {
            let package = packages
                .iter()
                .enumerate()
                .filter(|(_, p)| under(&path, &p.dir))
                .max_by_key(|(_, p)| p.dir.len())
                .map_or(0, |(i, _)| i);
            let file_name = path.rsplit('/').next().unwrap_or(&path);
            let is_test = tests.contains(&packages[package].dir)
                || file_name.ends_with("Tests.cs")
                || file_name.ends_with("Test.cs");
            SourceFile {
                path,
                package,
                kind: if is_test {
                    FileKind::Test
                } else {
                    FileKind::Source
                },
            }
        })
        .collect();
    Ok(Workspace { packages, files })
}

// ---- risks and imports -----------------------------------------------------------------------

fn regex_call(node: Node, source: &str) -> bool {
    let callee = node
        .child_by_field_name("function")
        .map_or("", |f| text(f, source));
    callee.starts_with("Regex.")
        && ["IsMatch", "Match", "Matches", "Replace", "Split"]
            .iter()
            .any(|m| callee.ends_with(&format!(".{m}")))
}

fn pattern_risk(pattern: Option<Node>, source: &str) -> Option<Risk> {
    let literal = match pattern.map(|p| (p.kind(), text(p, source))) {
        Some(("string_literal", p)) => p.trim_matches('"').replace("\\\\", "\\"),
        Some(("verbatim_string_literal", p)) => p.trim_start_matches('@').trim_matches('"').into(),
        _ => return Some(Risk::DynamicRegex),
    };
    regex_needs_fancy(&literal, "").then_some(Risk::RegexSyntax)
}

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
        "using_directive" => {
            let Some(name) = named_children(node)
                .into_iter()
                .rfind(|c| matches!(c.kind(), "qualified_name" | "identifier"))
            else {
                return;
            };
            let spec = text(name, source).to_string();
            let is_static = text(node, source).contains("using static");
            match spec.as_str() {
                s if s.starts_with("System.Reflection") => add(&mut facts, Risk::Reflection, node),
                s if s.starts_with("System.Threading") => add(&mut facts, Risk::Concurrency, node),
                s if s.starts_with("System.Text.RegularExpressions") => {
                    add(&mut facts, Risk::RegexSyntax, node)
                }
                _ => {}
            }
            if !is_static || spec.contains('.') {
                facts.imports.push(Import {
                    specifier: spec,
                    type_only: false,
                    dynamic: false,
                });
            }
        }
        "lock_statement" => add(&mut facts, Risk::Concurrency, node),
        "modifier" if text(node, source) == "volatile" => add(&mut facts, Risk::Concurrency, node),
        "modifier" if text(node, source) == "async" => add(&mut facts, Risk::Async, node),
        "modifier" if text(node, source) == "unsafe" => add(&mut facts, Risk::Pointers, node),
        "unsafe_statement" | "pointer_type" | "fixed_statement" => {
            add(&mut facts, Risk::Pointers, node)
        }
        "await_expression" => add(&mut facts, Risk::Async, node),
        "yield_statement" => add(&mut facts, Risk::Generators, node),
        "query_expression" => add(&mut facts, Risk::Generators, node),
        "null_literal" => add(&mut facts, Risk::NullReferences, node),
        "is_pattern_expression" | "is_expression" | "as_expression" | "cast_expression" => {
            add(&mut facts, Risk::DynamicTyping, node)
        }
        "identifier" if text(node, source) == "dynamic" => {
            add(&mut facts, Risk::DynamicTyping, node)
        }
        "predefined_type" if text(node, source) == "decimal" => {
            add(&mut facts, Risk::NumberSemantics, node)
        }
        "lambda_expression" | "anonymous_method_expression" => {
            add(&mut facts, Risk::Callbacks, node)
        }
        "throw_statement" | "throw_expression" | "try_statement" | "catch_clause" => {
            add(&mut facts, Risk::Exceptions, node)
        }
        "base_list" => add(&mut facts, Risk::ClassInheritance, node),
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
        "object_creation_expression" => {
            let ty = node
                .child_by_field_name("type")
                .map_or("", |t| text(t, source));
            match ty {
                "Regex" => {
                    let pattern = node
                        .child_by_field_name("arguments")
                        .and_then(|a| named_children(a).into_iter().next())
                        .and_then(|a| named_children(a).into_iter().next());
                    if let Some(risk) = pattern_risk(pattern, source) {
                        add(&mut facts, risk, node);
                    }
                }
                "HashSet" | "Dictionary" => add(&mut facts, Risk::UnorderedIteration, node),
                t if t.starts_with("HashSet<") || t.starts_with("Dictionary<") => {
                    add(&mut facts, Risk::UnorderedIteration, node)
                }
                _ => {}
            }
        }
        "invocation_expression" => {
            if regex_call(node, source) {
                let pattern = node
                    .child_by_field_name("arguments")
                    .and_then(|a| named_children(a).into_iter().nth(1))
                    .and_then(|a| named_children(a).into_iter().next());
                if let Some(risk) = pattern_risk(pattern, source) {
                    add(&mut facts, risk, node);
                }
                return;
            }
            let callee = node
                .child_by_field_name("function")
                .map_or("", |f| text(f, source));
            let method = callee.rsplit('.').next().unwrap_or(callee);
            match method {
                "Substring" | "IndexOf" | "LastIndexOf" | "Remove" | "Insert" | "PadLeft"
                | "PadRight" => add(&mut facts, Risk::StringOffsets, node),
                "GetType" | "GetMethod" | "GetProperty" | "Invoke" | "CreateInstance" => {
                    add(&mut facts, Risk::Reflection, node)
                }
                "GetHashCode" => add(&mut facts, Risk::IntegerSemantics, node),
                _ => {}
            }
        }
        "for_statement" | "foreach_statement" | "while_statement" | "do_statement" => {
            facts.loops += 1
        }
        _ => {}
    });
    facts
}

/// The folder a namespace names, relative to a project: `Acme.Textkit.Text` in the project
/// `Acme.Textkit` is its `Text/` folder.
fn namespace_dir(namespace: &str, project: &str) -> String {
    let rest = namespace
        .strip_prefix(project)
        .map_or(namespace, |r| r.trim_start_matches('.'));
    rest.replace('.', "/")
}

pub fn resolve(spec: &str, index: &HashMap<&str, usize>, ws: &Workspace) -> super::Resolution {
    let first = spec.split('.').next().unwrap_or("");
    if first == "System"
        || (first == "Microsoft"
            && !spec.starts_with("Microsoft.Extensions")
            && !spec.starts_with("Microsoft.AspNetCore")
            && !spec.starts_with("Microsoft.EntityFrameworkCore"))
    {
        return super::Resolution::Stdlib;
    }
    for p in &ws.packages {
        for dir in [namespace_dir(spec, &p.name), spec.replace('.', "/")] {
            if dir.is_empty() {
                continue;
            }
            let found = index
                .iter()
                .filter(|(f, _)| {
                    let folder = f.rsplit_once('/').map_or("", |(d, _)| d);
                    (p.dir.is_empty() || folder.starts_with(&format!("{}/", p.dir)))
                        && (folder == dir || folder.ends_with(&format!("/{dir}")))
                })
                .min_by_key(|(f, _)| **f);
            if let Some((_, i)) = found {
                return super::Resolution::File(*i);
            }
        }
    }
    if ws.packages.iter().any(|p| spec == p.name) {
        return super::Resolution::Stdlib;
    }
    super::Resolution::External(spec.split('.').take(2).collect::<Vec<_>>().join("."))
}

/// Rust counterparts of common NuGet packages, by package id or namespace.
pub fn lookup(name: &str) -> Option<CrateMatch> {
    crate::crates::find("csharp", name)
}

// ---- exports ---------------------------------------------------------------------------------

fn split_top(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for c in text.chars() {
        match c {
            '<' | '(' => depth += 1,
            '>' | ')' => depth -= 1,
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

/// A C# type in the TS-like dialect, or why the bridge cannot carry it.
pub fn csharp_to_ts(t: &str) -> Result<String, String> {
    let t = t.trim();
    if let Some(inner) = t.strip_suffix('?') {
        return csharp_to_ts(inner).map(|i| format!("{i} | undefined"));
    }
    if let Some(inner) = t.strip_suffix("[]") {
        return csharp_to_ts(inner).map(|i| {
            if i.contains(' ') {
                format!("({i})[]")
            } else {
                format!("{i}[]")
            }
        });
    }
    if t.starts_with('(') {
        return Err("uses a tuple".into());
    }
    let (head, args) = match t.find('<') {
        Some(i) if t.ends_with('>') => (&t[..i], split_top(&t[i + 1..t.len() - 1])),
        _ => (t, Vec::new()),
    };
    let head = head.trim().rsplit('.').next().unwrap_or(head);
    let arg = |i: usize| -> Result<String, String> {
        args.get(i)
            .map_or_else(|| Err(format!("raw `{head}`")), |a| csharp_to_ts(a))
    };
    Ok(match head {
        "string" | "String" => "string".into(),
        "int" | "long" | "short" | "byte" | "sbyte" | "uint" | "ushort" | "Int32" | "Int64"
        | "Int16" | "Byte" | "SByte" | "UInt32" | "UInt16" => "int".into(),
        "ulong" | "UInt64" | "nint" | "nuint" => {
            return Err(format!("uses `{head}`, wider than a JSON-safe integer"));
        }
        "double" | "float" | "Double" | "Single" => "number".into(),
        "decimal" | "Decimal" => {
            return Err("uses `decimal`, a 128-bit decimal no JSON number keeps exactly".into());
        }
        "bool" | "Boolean" => "boolean".into(),
        "void" => "void".into(),
        "List"
        | "IList"
        | "IReadOnlyList"
        | "ICollection"
        | "IReadOnlyCollection"
        | "IEnumerable"
        | "HashSet"
        | "ISet" => {
            let inner = arg(0)?;
            if inner.contains(' ') {
                format!("({inner})[]")
            } else {
                format!("{inner}[]")
            }
        }
        "Dictionary" | "IDictionary" | "IReadOnlyDictionary" | "SortedDictionary" => {
            if arg(0)? != "string" {
                return Err(format!("`{t}`: JSON object keys are strings"));
            }
            format!("Record<string, {}>", arg(1)?)
        }
        "Nullable" => format!("{} | undefined", arg(0)?),
        "char" | "Char" => return Err("uses `char`, a UTF-16 code unit".into()),
        "object" | "Object" | "dynamic" => {
            return Err("takes or returns `object`: values of any type".into());
        }
        "Func" | "Action" | "Predicate" | "Comparison" | "Delegate" => {
            return Err("takes or returns a delegate".into());
        }
        "Task" | "ValueTask" => return Err("is async (`Task`)".into()),
        "IAsyncEnumerable" | "IEnumerator" | "Span" | "ReadOnlySpan" | "Memory" => {
            return Err(format!("uses `{head}`"));
        }
        other => return Err(format!("uses the type `{other}`")),
    })
}

fn modifiers(node: Node, source: &str) -> Vec<String> {
    named_children(node)
        .into_iter()
        .filter(|c| c.kind() == "modifier")
        .map(|c| text(c, source).to_string())
        .collect()
}

fn method_export(
    method: Node,
    qualified: &str,
    path: &str,
    source: &str,
    out: &mut Vec<ExportedFn>,
) {
    let mods = modifiers(method, source);
    if !(mods.iter().any(|m| m == "public") && mods.iter().any(|m| m == "static")) {
        return;
    }
    let Some(name) = method
        .child_by_field_name("name")
        .map(|n| text(n, source).to_string())
    else {
        return;
    };
    let mut blocker = None;
    if method.child_by_field_name("type_parameters").is_some() {
        blocker = Some("generic".to_string());
    }
    let mut params = Vec::new();
    let mut signature = Vec::new();
    if let Some(list) = method.child_by_field_name("parameters") {
        for p in named_children(list) {
            if p.kind() != "parameter" {
                blocker.get_or_insert_with(|| "is variadic (`params`)".into());
                continue;
            }
            let all = text(p, source);
            let words: Vec<&str> = all.split_whitespace().collect();
            if words
                .first()
                .is_some_and(|w| matches!(*w, "ref" | "out" | "in"))
            {
                blocker.get_or_insert_with(|| "takes a parameter by reference".into());
            }
            if words.first() == Some(&"params") {
                blocker.get_or_insert_with(|| "is variadic (`params`)".into());
            }
            let ty = p
                .child_by_field_name("type")
                .map_or("", |t| text(t, source))
                .to_string();
            let pname = p
                .child_by_field_name("name")
                .map_or("", |n| text(n, source))
                .to_string();
            let ts_type = csharp_to_ts(&ty).unwrap_or_else(|reason| {
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
        .child_by_field_name("returns")
        .map_or("void", |t| text(t, source))
        .to_string();
    let returns = csharp_to_ts(&ret).unwrap_or_else(|reason| {
        blocker.get_or_insert(reason);
        "unknown".into()
    });
    if out.iter().any(|f: &ExportedFn| f.name == name) {
        blocker = Some("overloaded: one name, several signatures".into());
    }
    out.push(ExportedFn {
        prototype: Some(format!("{qualified}.{name}({})", signature.join(", "))),
        name,
        file: path.to_string(),
        line: syntax::line(method),
        params,
        returns,
        native_return: Some(ret),
        blocker,
        ..Default::default()
    });
}

fn collect(node: Node, namespace: &str, path: &str, source: &str, out: &mut Vec<ExportedFn>) {
    let mut namespace = namespace.to_string();
    for child in named_children(node) {
        match child.kind() {
            "file_scoped_namespace_declaration" => {
                namespace = child
                    .child_by_field_name("name")
                    .map_or(String::new(), |n| text(n, source).to_string());
            }
            "namespace_declaration" => {
                let name = child
                    .child_by_field_name("name")
                    .map_or("", |n| text(n, source));
                let inner = if namespace.is_empty() {
                    name.to_string()
                } else {
                    format!("{namespace}.{name}")
                };
                if let Some(body) = child.child_by_field_name("body") {
                    collect(body, &inner, path, source, out);
                }
            }
            "class_declaration" => {
                if !modifiers(child, source).iter().any(|m| m == "public") {
                    continue;
                }
                let Some(class) = child
                    .child_by_field_name("name")
                    .map(|n| text(n, source).to_string())
                else {
                    continue;
                };
                let qualified = if namespace.is_empty() {
                    class
                } else {
                    format!("{namespace}.{class}")
                };
                if let Some(body) = child.child_by_field_name("body") {
                    for member in named_children(body) {
                        if member.kind() == "method_declaration" {
                            method_export(member, &qualified, path, source, out);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// `public static` methods of the file's public classes. `prototype` holds the declaring class's
/// fully qualified name and the signature: `Acme.Text.Slug.Slugify(string text)`.
pub fn exports(path: &str, source: &str) -> Vec<ExportedFn> {
    if !path.ends_with(".cs") {
        return Vec::new();
    }
    let Some(tree) = syntax::parse(&language(), source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect(tree.root_node(), "", path, source, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_types() {
        assert_eq!(csharp_to_ts("List<string>").unwrap(), "string[]");
        assert_eq!(
            csharp_to_ts("Dictionary<string, List<int>>").unwrap(),
            "Record<string, int[]>"
        );
        assert_eq!(csharp_to_ts("double[]").unwrap(), "number[]");
        assert_eq!(csharp_to_ts("int?").unwrap(), "int | undefined");
        assert!(csharp_to_ts("decimal").is_err());
        assert!(csharp_to_ts("Task<int>").is_err());
        assert!(csharp_to_ts("Point").is_err());
    }

    #[test]
    fn finds_public_static_methods_in_namespaces() {
        let src = "using System.Collections.Generic;\nnamespace Acme.Text;\n\
                   public static class Slug {\n\
                   public static string Slugify(string text, int max) => text;\n\
                   public static List<string> Words(string text) { return new(); }\n\
                   public static T Id<T>(T x) => x;\n\
                   public static void Swap(ref int a, ref int b) {}\n\
                   static int Hidden() => 1;\n\
                   public int Instance() => 1;\n}\n\
                   internal static class Private { public static int No() => 1; }\n";
        let fns = exports("src/Text/Slug.cs", src);
        let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["Slugify", "Words", "Id", "Swap"]);
        assert_eq!(fns[0].params[1].ts_type, "int");
        assert_eq!(
            fns[0].prototype.as_deref(),
            Some("Acme.Text.Slug.Slugify(string text, int max)")
        );
        assert_eq!(fns[1].returns, "string[]");
        assert!(fns[2].blocker.is_some());
        assert!(
            fns[3]
                .blocker
                .as_deref()
                .is_some_and(|b| b.contains("reference"))
        );

        let block = "namespace Acme { namespace Text { public class A { public static bool B() => true; } } }";
        let fns = exports("A.cs", block);
        assert_eq!(fns[0].prototype.as_deref(), Some("Acme.Text.A.B()"));
    }

    #[test]
    fn flags_risks_and_usings() {
        let src = "using System.Text.RegularExpressions;\nusing Newtonsoft.Json;\n\
                   class C { async Task<int> F(string s) { lock (this) {} \
                   var r = Regex.IsMatch(s, @\"(?<=a)b\"); decimal d = 1m; \
                   return s.Substring(1).Length / 2; } }";
        let facts = scan(src);
        let specs: Vec<&str> = facts.imports.iter().map(|i| i.specifier.as_str()).collect();
        assert_eq!(specs, ["System.Text.RegularExpressions", "Newtonsoft.Json"]);
        for risk in [
            Risk::Async,
            Risk::Concurrency,
            Risk::RegexSyntax,
            Risk::NumberSemantics,
            Risk::StringOffsets,
            Risk::DivisionSemantics,
        ] {
            assert!(
                facts.risks.contains_key(&risk),
                "{risk:?}: {:?}",
                facts.risks
            );
        }
    }

    #[test]
    fn classifies_test_sources_in_a_root_project_without_test_suffixes() {
        let dir = std::env::temp_dir().join(format!(
            "rust-anything-csharp-test-root-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("Tests.csproj"),
            "<Project><IsTestProject>true</IsTestProject></Project>",
        )
        .unwrap();
        std::fs::write(dir.join("ValidatorCases.cs"), "class ValidatorCases {}").unwrap();
        std::fs::write(dir.join("src/Library.csproj"), "<Project />").unwrap();
        std::fs::write(dir.join("src/Library.cs"), "public class Library {}").unwrap();
        let ws = discover(&dir).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        assert_eq!(ws.files[0].kind, FileKind::Test);
        assert_eq!(ws.files[1].kind, FileKind::Source);
    }

    #[test]
    fn reads_package_references() {
        let proj = r#"<Project Sdk="Microsoft.NET.Sdk"><ItemGroup>
            <PackageReference Include="Newtonsoft.Json" Version="13.0.3" />
            <PackageReference Include="Serilog" Version="4.0.0"></PackageReference>
            </ItemGroup></Project>"#;
        assert_eq!(package_references(proj), ["Newtonsoft.Json", "Serilog"]);
    }
}
