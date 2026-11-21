//! Go: modules, imports, risks, the module-to-crate table, and exported functions with Go types
//! mapped into the TS-like type dialect. A unit is a package (a folder).

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tree_sitter::Node;

use super::syntax::{self, named_children, text};
use crate::risks::Risk;
use crate::ts::deps::CrateMatch;
use crate::ts::discover::{FileKind, Package, SourceFile, Workspace};
use crate::ts::exports::{ExportedFn, Param};
use crate::ts::scan::{FileFacts, Import};

fn language() -> tree_sitter::Language {
    tree_sitter_go::LANGUAGE.into()
}

const SKIP_DIRS: &[&str] = &["vendor", "testdata", "node_modules", "target", ".git"];

// ---- discovery -------------------------------------------------------------------------------

/// The first value on a single-line go.mod directive.
pub(crate) fn mod_directive<'a>(go_mod: &'a str, directive: &str) -> Option<&'a str> {
    go_mod.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        if words.next()? != directive {
            return None;
        }
        words.next().filter(|word| !word.starts_with("//"))
    })
}

/// The module path declared in a go.mod.
pub fn module_path(go_mod: &str) -> Option<String> {
    mod_directive(go_mod, "module").map(|path| path.trim_matches('"').to_string())
}

fn requirements(go_mod: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false;
    for line in go_mod.lines() {
        let line = line.trim();
        if line.starts_with("require (") {
            in_block = true;
            continue;
        }
        if in_block && line == ")" {
            in_block = false;
            continue;
        }
        let spec = if in_block {
            Some(line)
        } else {
            line.strip_prefix("require ")
        };
        if let Some(spec) = spec
            && !line.contains("// indirect")
            && let Some(name) = spec.split_whitespace().next()
            && !name.is_empty()
            && !name.starts_with("//")
        {
            out.push(name.to_string());
        }
    }
    out
}

pub fn discover(root: &Path) -> Result<Workspace> {
    let mut modules = Vec::new();
    let mut files = Vec::new();
    let walker = crate::walk::filtered(root, |e| {
        let name = e.file_name().to_string_lossy();
        !(e.file_type().is_some_and(|t| t.is_dir())
            && (SKIP_DIRS.contains(&name.as_ref()) || name.starts_with('_')))
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
        if name == "go.mod" {
            let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
            modules.push(Package {
                name: module_path(&text).unwrap_or_else(|| dir.clone()),
                src_root: dir.clone(),
                dir,
                deps: requirements(&text),
                dev_deps: Vec::new(),
            });
        } else if name.ends_with(".go") {
            let kind = if name.ends_with("_test.go") {
                FileKind::Test
            } else {
                FileKind::Source
            };
            files.push((rel, kind));
        }
    }
    if modules.is_empty() {
        modules.push(Package {
            name: root
                .file_name()
                .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned()),
            dir: String::new(),
            src_root: String::new(),
            deps: Vec::new(),
            dev_deps: Vec::new(),
        });
    }
    modules.sort_by(|a, b| a.dir.cmp(&b.dir));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let files = files
        .into_iter()
        .map(|(path, kind)| {
            let package = modules
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
    Ok(Workspace {
        packages: modules,
        files,
    })
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
        "import_spec" => {
            if let Some(path) = node.child_by_field_name("path") {
                let spec = text(path, source).trim_matches(|c| c == '"' || c == '`');
                match spec {
                    "unsafe" => add(&mut facts, Risk::Pointers, node),
                    "reflect" => add(&mut facts, Risk::Reflection, node),
                    "sync" | "sync/atomic" => add(&mut facts, Risk::Concurrency, node),
                    "regexp" => add(&mut facts, Risk::RegexSyntax, node),
                    "encoding/json" => add(&mut facts, Risk::Json, node),
                    _ => {}
                }
                facts.imports.push(Import {
                    specifier: spec.to_string(),
                    type_only: false,
                    dynamic: false,
                });
            }
        }
        "go_statement" | "channel_type" | "send_statement" | "select_statement" => {
            add(&mut facts, Risk::Concurrency, node)
        }
        "pointer_type" | "unary_expression" if text(node, source).starts_with('&') => {
            add(&mut facts, Risk::Pointers, node)
        }
        "pointer_type" => add(&mut facts, Risk::Pointers, node),
        "slice_expression" => add(&mut facts, Risk::StringOffsets, node),
        "binary_expression" => {
            let op = node
                .child_by_field_name("operator")
                .map_or("", |o| text(o, source));
            match op {
                "/" | "%" => add(&mut facts, Risk::DivisionSemantics, node),
                "<<" | ">>" => add(&mut facts, Risk::IntegerSemantics, node),
                _ => {}
            }
        }
        "type_conversion_expression" => add(&mut facts, Risk::IntegerSemantics, node),
        "call_expression" => {
            let callee = node
                .child_by_field_name("function")
                .map_or("", |f| text(f, source));
            match callee {
                "panic" | "recover" => add(&mut facts, Risk::Exceptions, node),
                "int" | "int32" | "int64" | "uint" | "uint32" | "uint64" | "uint8" | "byte"
                | "float64" => add(&mut facts, Risk::IntegerSemantics, node),
                _ => {}
            }
        }
        "nil" => add(&mut facts, Risk::NullReferences, node),
        "interface_type" | "type_switch_statement" | "type_assertion_expression" => {
            add(&mut facts, Risk::DynamicTyping, node)
        }
        "func_literal" | "function_type" => add(&mut facts, Risk::Callbacks, node),
        "defer_statement" => add(&mut facts, Risk::Exceptions, node),
        "range_clause" => {
            facts.loops += 1;
            if node
                .child_by_field_name("right")
                .is_some_and(|r| text(r, source).contains("map"))
            {
                add(&mut facts, Risk::UnorderedIteration, node);
            }
        }
        "for_statement" => facts.loops += 1,
        _ => {}
    });
    facts
}

/// Go standard-library import paths have no dot in their first element.
fn is_stdlib(spec: &str) -> bool {
    !spec.split('/').next().unwrap_or("").contains('.')
}

pub fn resolve(spec: &str, index: &HashMap<&str, usize>, ws: &Workspace) -> super::Resolution {
    // The longest module path that prefixes the import is the module it belongs to.
    if let Some(module) = ws
        .packages
        .iter()
        .filter(|p| spec == p.name || spec.starts_with(&format!("{}/", p.name)))
        .max_by_key(|p| p.name.len())
    {
        let sub = spec[module.name.len()..].trim_start_matches('/');
        let dir = match (module.dir.as_str(), sub) {
            ("", s) => s.to_string(),
            (d, "") => d.to_string(),
            (d, s) => format!("{d}/{s}"),
        };
        let mut best: Option<(&str, usize)> = None;
        for (path, i) in index {
            let parent = path.rsplit_once('/').map_or("", |(d, _)| d);
            if parent == dir
                && path.ends_with(".go")
                && !path.ends_with("_test.go")
                && best.is_none_or(|(p, _)| *path < p)
            {
                best = Some((path, *i));
            }
        }
        return best.map_or(super::Resolution::Unresolved, |(_, i)| {
            super::Resolution::File(i)
        });
    }
    if is_stdlib(spec) {
        return super::Resolution::Stdlib;
    }
    super::Resolution::External(spec.to_string())
}

/// Rust counterparts of common Go modules.
pub fn lookup(module: &str) -> Option<CrateMatch> {
    crate::crates::find("go", module)
}

// ---- exports ---------------------------------------------------------------------------------

/// Go builtin types the bridge carries, as the TS-like dialect.
fn builtin(t: &str) -> Option<&'static str> {
    Some(match t {
        "string" => "string",
        "bool" => "boolean",
        "int" | "int8" | "int16" | "int32" | "int64" | "uint" | "uint8" | "uint16" | "uint32"
        | "uint64" | "uintptr" | "byte" | "rune" => "int",
        "float32" | "float64" => "number",
        _ => return None,
    })
}

fn direct_struct_fields(struct_type: Node<'_>) -> Vec<Node<'_>> {
    named_children(struct_type)
        .into_iter()
        .find(|node| node.kind() == "field_declaration_list")
        .map(|list| {
            named_children(list)
                .into_iter()
                .filter(|field| field.kind() == "field_declaration")
                .collect()
        })
        .unwrap_or_default()
}

/// Exported struct types of a file: name to dialect object type, from their JSON field names.
fn structs(root: Node, source: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for decl in named_children(root)
        .into_iter()
        .filter(|n| n.kind() == "type_declaration")
    {
        for spec in named_children(decl)
            .into_iter()
            .filter(|n| n.kind() == "type_spec")
        {
            let (Some(name), Some(ty)) = (
                spec.child_by_field_name("name"),
                spec.child_by_field_name("type"),
            ) else {
                continue;
            };
            if ty.kind() != "struct_type" {
                continue;
            }
            let mut fields = Vec::new();
            let mut ok = true;
            for n in direct_struct_fields(ty) {
                let field_type = n
                    .child_by_field_name("type")
                    .map_or("", |t| text(t, source));
                let tag = n.child_by_field_name("tag").map_or("", |t| text(t, source));
                let json_name = tag
                    .split("json:\"")
                    .nth(1)
                    .and_then(|r| r.split('"').next())
                    .map(|r| r.split(',').next().unwrap_or("").to_string());
                let mut cursor = n.walk();
                let names: Vec<String> = n
                    .children_by_field_name("name", &mut cursor)
                    .map(|f| text(f, source).to_string())
                    .collect();
                if names.is_empty() {
                    ok = false; // embedded field
                }
                for field in names {
                    if !field.starts_with(char::is_uppercase) || json_name.as_deref() == Some("-") {
                        continue;
                    }
                    match go_to_ts(field_type, &HashMap::new()) {
                        Ok(t) => {
                            let key = json_name.clone().filter(|j| !j.is_empty()).unwrap_or(field);
                            fields.push(format!("{key}: {t}"));
                        }
                        Err(_) => ok = false,
                    }
                }
            }
            if ok {
                out.insert(
                    text(name, source).to_string(),
                    format!("{{ {} }}", fields.join("; ")),
                );
            }
        }
    }
    out
}

/// A Go type in the TS-like dialect, or why the bridge cannot carry it.
pub fn go_to_ts(t: &str, structs: &HashMap<String, String>) -> Result<String, String> {
    let t = t.trim();
    if let Some(b) = builtin(t) {
        return Ok(b.to_string());
    }
    if let Some(inner) = t.strip_prefix("[]") {
        return go_to_ts(inner, structs).map(|i| {
            if i.contains(' ') || i.contains('|') {
                format!("({i})[]")
            } else {
                format!("{i}[]")
            }
        });
    }
    if let Some(rest) = t.strip_prefix("map[") {
        let mut depth = 1;
        let close = rest.char_indices().find_map(|(i, c)| {
            match c {
                '[' => depth += 1,
                ']' => depth -= 1,
                _ => {}
            }
            (depth == 0).then_some(i)
        });
        if let Some(close) = close {
            let (key, value) = (&rest[..close], &rest[close + 1..]);
            if key.trim() != "string" {
                return Err(format!("`{t}`: JSON object keys are strings"));
            }
            return go_to_ts(value, structs).map(|v| format!("Record<string, {v}>"));
        }
    }
    if t.starts_with('*') {
        return Err(format!("takes or returns a pointer (`{t}`)"));
    }
    if t.starts_with("func") {
        return Err("takes or returns a function".into());
    }
    if t.starts_with("chan") || t.starts_with("<-chan") {
        return Err("takes or returns a channel".into());
    }
    if t == "any" || t.starts_with("interface") {
        return Err(format!("`{t}`: values of any type"));
    }
    if t.starts_with('[') {
        return Err(format!("`{t}`: fixed-size array"));
    }
    if t == "error" {
        return Err("takes an `error` value".into());
    }
    if t.contains('.') {
        return Err(format!("uses `{t}` from another package"));
    }
    if !t.starts_with(char::is_uppercase) {
        return Err(format!("uses the unexported type `{t}`"));
    }
    Ok(structs.get(t).cloned().unwrap_or_else(|| "unknown".into()))
}

/// Exported top-level functions. Methods stay with their types: Go has no classes to bridge.
pub fn exports(path: &str, source: &str) -> Vec<ExportedFn> {
    if !path.ends_with(".go") || path.ends_with("_test.go") {
        return Vec::new();
    }
    let Some(tree) = syntax::parse(&language(), source) else {
        return Vec::new();
    };
    let root = tree.root_node();
    let structs = structs(root, source);
    let mut out = Vec::new();
    for node in named_children(root) {
        if node.kind() != "function_declaration" {
            continue;
        }
        let name = node
            .child_by_field_name("name")
            .map(|n| text(n, source).to_string())
            .unwrap_or_default();
        if !name.starts_with(char::is_uppercase) {
            continue;
        }
        let mut blocker = None;
        if node.child_by_field_name("type_parameters").is_some() {
            blocker = Some("generic: Go instantiates it per type".to_string());
        }
        let mut params = Vec::new();
        if let Some(list) = node.child_by_field_name("parameters") {
            for (i, p) in named_children(list).into_iter().enumerate() {
                if p.kind() == "variadic_parameter_declaration" {
                    blocker.get_or_insert_with(|| "is variadic".into());
                }
                let go_type = p
                    .child_by_field_name("type")
                    .map_or("", |t| text(t, source))
                    .to_string();
                let ts_type = go_to_ts(&go_type, &structs).unwrap_or_else(|reason| {
                    blocker.get_or_insert(reason);
                    "unknown".into()
                });
                let mut cursor = p.walk();
                let mut names: Vec<String> = p
                    .children_by_field_name("name", &mut cursor)
                    .map(|n| text(n, source).to_string())
                    .collect();
                if names.is_empty() {
                    names.push(format!("arg{i}"));
                }
                for n in names {
                    let n = if n == "_" {
                        format!("arg{}", params.len())
                    } else {
                        n
                    };
                    params.push(Param {
                        name: n,
                        ts_type: ts_type.clone(),
                        native_type: Some(go_type.clone()),
                        ..Default::default()
                    });
                }
            }
        }
        // Results: none, one type, or a list; a trailing `error` is the error channel.
        let mut results: Vec<String> = Vec::new();
        if let Some(r) = node.child_by_field_name("result") {
            if r.kind() == "parameter_list" {
                for p in named_children(r) {
                    let t = p
                        .child_by_field_name("type")
                        .map_or("", |t| text(t, source))
                        .to_string();
                    let mut cursor = p.walk();
                    let count = p.children_by_field_name("name", &mut cursor).count().max(1);
                    results.extend(std::iter::repeat_n(t, count));
                }
            } else {
                results.push(text(r, source).to_string());
            }
        }
        let fallible = results.last().is_some_and(|r| r == "error");
        if fallible {
            results.pop();
        }
        let mut dialect = Vec::new();
        for r in &results {
            match go_to_ts(r, &structs) {
                Ok(t) => dialect.push(t),
                Err(reason) => {
                    blocker.get_or_insert(reason);
                    dialect.push("unknown".into());
                }
            }
        }
        let returns = match dialect.len() {
            0 => "void".to_string(),
            1 => dialect.remove(0),
            _ => format!("[{}]", dialect.join(", ")),
        };
        let signature = node.child_by_field_name("body").map_or_else(
            || text(node, source),
            |b| source[node.start_byte()..b.start_byte()].trim(),
        );
        out.push(ExportedFn {
            name,
            file: path.to_string(),
            line: syntax::line(node),
            params,
            returns,
            native_return: Some(format!(
                "({}{})",
                results.join(", "),
                if fallible {
                    if results.is_empty() {
                        "error"
                    } else {
                        ", error"
                    }
                } else {
                    ""
                }
            )),
            prototype: Some(signature.split_whitespace().collect::<Vec<_>>().join(" ")),
            blocker,
            ..Default::default()
        });
    }
    out
}

/// The package clause of a Go file.
pub fn package_name(source: &str) -> Option<String> {
    source.lines().find_map(|l| {
        l.trim()
            .strip_prefix("package ")
            .map(|p| p.split_whitespace().next().unwrap_or("").to_string())
    })
}

/// The `type` declarations named `names`, as written, from the given sources.
pub fn type_decls(sources: &[String], names: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for source in sources {
        let Some(tree) = syntax::parse(&language(), source) else {
            continue;
        };
        for decl in named_children(tree.root_node())
            .into_iter()
            .filter(|n| n.kind() == "type_declaration")
        {
            for spec in named_children(decl)
                .into_iter()
                .filter(|n| n.kind() == "type_spec")
            {
                if spec
                    .child_by_field_name("name")
                    .is_some_and(|n| names.iter().any(|w| w == text(n, source)))
                {
                    out.push(format!("type {}", text(spec, source)));
                }
            }
        }
    }
    out
}

/// The exported fields of struct type `name`: (field, Go type, JSON key). `None` when the type
/// is not a plain struct (embedded fields, or not found).
pub fn struct_fields(sources: &[String], name: &str) -> Option<Vec<(String, String, String)>> {
    for source in sources {
        let Some(tree) = syntax::parse(&language(), source) else {
            continue;
        };
        for decl in named_children(tree.root_node())
            .into_iter()
            .filter(|n| n.kind() == "type_declaration")
        {
            for spec in named_children(decl)
                .into_iter()
                .filter(|n| n.kind() == "type_spec")
            {
                if spec.child_by_field_name("name").map(|n| text(n, source)) != Some(name) {
                    continue;
                }
                let ty = spec.child_by_field_name("type")?;
                if ty.kind() != "struct_type" {
                    return None;
                }
                let mut out = Vec::new();
                let mut ok = true;
                for n in direct_struct_fields(ty) {
                    let field_type = n
                        .child_by_field_name("type")
                        .map_or("", |t| text(t, source))
                        .to_string();
                    let tag = n.child_by_field_name("tag").map_or("", |t| text(t, source));
                    let json = tag
                        .split("json:\"")
                        .nth(1)
                        .and_then(|r| r.split('"').next())
                        .map(|r| r.split(',').next().unwrap_or("").to_string());
                    let mut cursor = n.walk();
                    let names: Vec<String> = n
                        .children_by_field_name("name", &mut cursor)
                        .map(|f| text(f, source).to_string())
                        .collect();
                    if names.is_empty() {
                        ok = false;
                    }
                    for field in names {
                        if !field.starts_with(char::is_uppercase) || json.as_deref() == Some("-") {
                            continue;
                        }
                        let key = json
                            .clone()
                            .filter(|j| !j.is_empty())
                            .unwrap_or_else(|| field.clone());
                        out.push((field, field_type.clone(), key));
                    }
                }
                return ok.then_some(out);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_types() {
        let none = HashMap::new();
        assert_eq!(go_to_ts("[]string", &none).unwrap(), "string[]");
        assert_eq!(
            go_to_ts("map[string][]int", &none).unwrap(),
            "Record<string, int[]>"
        );
        assert!(go_to_ts("*Point", &none).is_err());
        assert!(go_to_ts("map[int]string", &none).is_err());
    }

    #[test]
    fn hidden_nested_struct_does_not_add_phantom_fields() {
        let source = String::from(
            r#"package x
type Point struct {
    hidden struct { Extra float64 }
    X int `json:"x"`
}
func Echo(p Point) Point { return p }"#,
        );
        let exports = exports("x.go", &source);
        assert_eq!(exports[0].params[0].ts_type, "{ x: int }");
        assert_eq!(exports[0].returns, "{ x: int }");
        assert_eq!(
            struct_fields(&[source], "Point").unwrap(),
            [("X".into(), "int".into(), "x".into())]
        );
    }

    #[test]
    fn finds_exports() {
        let src = "package x\n\ntype Point struct {\n\tX int `json:\"x\"`\n\tY float64\n\tz int\n}\n\n\
                   func Add(a, b int) int { return a + b }\n\
                   func Div(a float64, b float64) (float64, error) { return a / b, nil }\n\
                   func Many(xs ...string) []string { return xs }\n\
                   func (p Point) Norm() float64 { return 0 }\n\
                   func Gen[T any](x T) T { return x }\n\
                   func Move(p Point) Point { return p }\n\
                   func hidden() {}\n";
        let fns = exports("x/x.go", src);
        let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["Add", "Div", "Many", "Gen", "Move"]);
        assert_eq!(fns[0].params.len(), 2);
        assert_eq!(fns[0].params[1].ts_type, "int");
        assert_eq!(fns[1].returns, "number");
        assert_eq!(fns[1].native_return.as_deref(), Some("(float64, error)"));
        assert!(fns[2].blocker.is_some() && fns[3].blocker.is_some());
        assert_eq!(fns[4].params[0].ts_type, "{ x: int; Y: number }");
        assert_eq!(
            fns[4].prototype.as_deref(),
            Some("func Move(p Point) Point")
        );
    }
}
