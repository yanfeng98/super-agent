//! C: build folders, `#include`s, risks, and exported functions whose signatures a drop-in
//! `extern "C"` replacement can take over: scalars and `const char *` in, scalars out.

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
    tree_sitter_c::LANGUAGE.into()
}

const SKIP_DIRS: &[&str] = &[
    "build",
    "dist",
    "target",
    "vendor",
    "third_party",
    "external",
    "fixtures",
    "examples",
    "docs",
];

pub fn discover(root: &Path) -> Result<Workspace> {
    let mut manifests = Vec::new();
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
        if matches!(name, "Makefile" | "CMakeLists.txt" | "meson.build") {
            let dir = rel
                .rsplit_once('/')
                .map_or(String::new(), |(d, _)| d.to_string());
            if !manifests.contains(&dir) {
                manifests.push(dir);
            }
        } else if name.ends_with(".c") || name.ends_with(".h") {
            let is_test = name.starts_with("test_")
                || name.ends_with("_test.c")
                || rel.split('/').any(|p| matches!(p, "tests" | "test"));
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
    let mut packages: Vec<Package> = manifests
        .iter()
        .map(|dir| Package {
            name: if dir.is_empty() {
                root.file_name()
                    .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned())
            } else {
                dir.clone()
            },
            src_root: if root.join(dir).join("src").is_dir() {
                join(dir, "src")
            } else {
                dir.clone()
            },
            dir: dir.clone(),
            deps: Vec::new(),
            dev_deps: Vec::new(),
        })
        .collect();
    if !packages.iter().any(|p| p.dir.is_empty()) {
        packages.push(Package {
            name: root
                .file_name()
                .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned()),
            dir: String::new(),
            src_root: if root.join("src").is_dir() {
                "src".into()
            } else {
                String::new()
            },
            deps: Vec::new(),
            dev_deps: Vec::new(),
        });
    }
    packages.sort_by(|a, b| a.dir.cmp(&b.dir));
    // A header with an implementation file is that file's interface: one unit, not two.
    let stems: std::collections::HashSet<String> = files
        .iter()
        .filter_map(|(f, _)| f.strip_suffix(".c"))
        .map(|f| f.rsplit('/').next().unwrap_or(f).to_string())
        .collect();
    files.retain(|(f, _)| {
        f.strip_suffix(".h")
            .is_none_or(|h| !stems.contains(h.rsplit('/').next().unwrap_or(h)))
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
        "preproc_include" => {
            if let Some(path) = node.child_by_field_name("path") {
                let raw = text(path, source);
                // `"local.h"` is part of the project; `<stdio.h>` is the system's.
                let spec = if raw.starts_with('"') {
                    raw.trim_matches('"').to_string()
                } else {
                    format!("<{}>", raw.trim_matches(|c| c == '<' || c == '>'))
                };
                facts.imports.push(Import {
                    specifier: spec,
                    type_only: false,
                    dynamic: false,
                });
            }
        }
        "pointer_expression" | "pointer_declarator" => add(&mut facts, Risk::Pointers, node),
        "call_expression" => {
            let callee = node
                .child_by_field_name("function")
                .map_or("", |f| text(f, source));
            match callee {
                "malloc" | "calloc" | "realloc" | "free" | "memcpy" | "memmove" => {
                    add(&mut facts, Risk::Pointers, node)
                }
                "strlen" | "strcpy" | "strncpy" | "strcat" | "strcmp" | "strchr" | "strstr"
                | "sprintf" | "snprintf" => {
                    add(&mut facts, Risk::StringOffsets, node);
                }
                "pthread_create" | "pthread_mutex_lock" | "atomic_fetch_add" => {
                    add(&mut facts, Risk::Concurrency, node)
                }
                "setjmp" | "longjmp" | "signal" | "exit" | "abort" => {
                    add(&mut facts, Risk::Exceptions, node)
                }
                _ => {}
            }
        }
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
        "cast_expression" => {
            add(&mut facts, Risk::IntegerSemantics, node);
            let value = node.child_by_field_name("value");
            let ty = node
                .child_by_field_name("type")
                .map_or("", |t| text(t, source));
            if ty.contains('*') && value.is_some_and(|v| v.kind() == "number_literal") {
                add(&mut facts, Risk::PlatformSpecific, node);
            }
        }
        "gnu_asm_expression" | "bitfield_clause" | "ms_declspec_modifier" => {
            add(&mut facts, Risk::PlatformSpecific, node)
        }
        "type_qualifier" if text(node, source) == "volatile" => {
            add(&mut facts, Risk::PlatformSpecific, node)
        }
        "attribute_specifier" | "attribute_declaration"
            if ["packed", "aligned", "section", "interrupt", "naked"]
                .iter()
                .any(|a| text(node, source).contains(a)) =>
        {
            add(&mut facts, Risk::PlatformSpecific, node)
        }
        "preproc_call"
            if text(node, source).trim_start().starts_with("#pragma")
                && text(node, source).contains("pack") =>
        {
            add(&mut facts, Risk::PlatformSpecific, node)
        }
        "null" => add(&mut facts, Risk::NullReferences, node),
        "goto_statement" => add(&mut facts, Risk::DynamicCode, node),
        "preproc_function_def" => add(&mut facts, Risk::Reflection, node),
        "for_statement" | "while_statement" | "do_statement" => facts.loops += 1,
        _ => {}
    });
    facts
}

/// `#include "x.h"`: the header relative to the including file or the project's include folders,
/// then the `.c` file beside it, so the unit graph follows implementations.
pub fn resolve(from: &str, spec: &str, index: &HashMap<&str, usize>) -> super::Resolution {
    if spec.starts_with('<') {
        // The C standard library is part of every platform; other system headers are libraries.
        return match lookup(spec) {
            Some(m) if m.crates == "std" => super::Resolution::Stdlib,
            _ if STANDARD.contains(&spec) => super::Resolution::Stdlib,
            _ => super::Resolution::External(spec.to_string()),
        };
    }
    let dir = from.rsplit_once('/').map_or("", |(d, _)| d);
    let candidates = [
        join(dir, spec),
        spec.to_string(),
        join("include", spec),
        join("src", spec),
    ];
    for header in candidates.iter().filter_map(|c| normalize(c)) {
        let implementation = header.strip_suffix(".h").map(|stem| format!("{stem}.c"));
        if let Some(i) = implementation
            .as_deref()
            .and_then(|c| index.get(c))
            .or_else(|| index.get(header.as_str()))
        {
            return super::Resolution::File(*i);
        }
    }
    super::Resolution::Unresolved
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

const STANDARD: &[&str] = &[
    "<stdio.h>",
    "<stdlib.h>",
    "<string.h>",
    "<math.h>",
    "<ctype.h>",
    "<stdint.h>",
    "<stdbool.h>",
    "<stddef.h>",
    "<limits.h>",
    "<errno.h>",
    "<assert.h>",
    "<float.h>",
    "<stdarg.h>",
    "<time.h>",
    "<inttypes.h>",
    "<wchar.h>",
    "<locale.h>",
    "<signal.h>",
    "<setjmp.h>",
    "<iso646.h>",
    "<uchar.h>",
    "<wctype.h>",
];

/// Rust counterparts of common C libraries, by header.
pub fn lookup(header: &str) -> Option<CrateMatch> {
    crate::crates::find("c", header)
}

// ---- exports ---------------------------------------------------------------------------------

/// A C scalar type: (dialect type, Rust type in the port, Rust type at the C ABI, ctypes name).
pub fn scalar(c: &str) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    let c = c
        .split_whitespace()
        .filter(|w| *w != "const")
        .collect::<Vec<_>>()
        .join(" ");
    Some(match c.as_str() {
        "int" | "signed int" | "signed" => ("int", "i32", "std::ffi::c_int", "c_int"),
        "unsigned" | "unsigned int" => ("int", "u32", "std::ffi::c_uint", "c_uint"),
        "short" | "short int" | "signed short" => ("int", "i16", "std::ffi::c_short", "c_short"),
        "unsigned short" | "unsigned short int" => ("int", "u16", "std::ffi::c_ushort", "c_ushort"),
        "long" | "long int" | "signed long" => ("int", "i64", "std::ffi::c_long", "c_long"),
        "unsigned long" | "unsigned long int" => ("int", "u64", "std::ffi::c_ulong", "c_ulong"),
        "long long" | "long long int" | "signed long long" => {
            ("int", "i64", "std::ffi::c_longlong", "c_longlong")
        }
        "unsigned long long" | "unsigned long long int" => {
            ("int", "u64", "std::ffi::c_ulonglong", "c_ulonglong")
        }
        "char" | "signed char" => ("int", "i8", "std::ffi::c_schar", "c_byte"),
        "unsigned char" => ("int", "u8", "std::ffi::c_uchar", "c_ubyte"),
        "int8_t" => ("int", "i8", "i8", "c_int8"),
        "int16_t" => ("int", "i16", "i16", "c_int16"),
        "int32_t" => ("int", "i32", "i32", "c_int32"),
        "int64_t" => ("int", "i64", "i64", "c_int64"),
        "uint8_t" => ("int", "u8", "u8", "c_uint8"),
        "uint16_t" => ("int", "u16", "u16", "c_uint16"),
        "uint32_t" => ("int", "u32", "u32", "c_uint32"),
        "uint64_t" => ("int", "u64", "u64", "c_uint64"),
        "size_t" => ("int", "usize", "usize", "c_size_t"),
        "ssize_t" | "ptrdiff_t" | "intptr_t" => ("int", "isize", "isize", "c_ssize_t"),
        "float" => ("number", "f32", "f32", "c_float"),
        "double" => ("number", "f64", "f64", "c_double"),
        "bool" | "_Bool" => ("boolean", "bool", "bool", "c_bool"),
        "void" => ("void", "()", "()", "None"),
        _ => return None,
    })
}

/// The C type of a declarator: base type plus pointers (`const char *`), and the name.
fn declared(node: Node, base: &str, source: &str) -> (String, Option<String>) {
    let mut ty = base.to_string();
    let mut current = Some(node);
    let mut name = None;
    while let Some(n) = current {
        match n.kind() {
            "pointer_declarator" | "abstract_pointer_declarator" => {
                ty.push('*');
                current = n.child_by_field_name("declarator");
            }
            "identifier" => {
                name = Some(text(n, source).to_string());
                current = None;
            }
            "array_declarator" => {
                ty.push_str("[]");
                current = n.child_by_field_name("declarator");
            }
            "function_declarator" => {
                ty.push_str("(fn)");
                current = n.child_by_field_name("declarator");
            }
            _ => current = n.child_by_field_name("declarator"),
        }
    }
    (ty, name)
}

/// The text of a declaration's type specifiers and qualifiers (`const unsigned long`).
fn base_type(node: Node, source: &str) -> String {
    let mut parts = Vec::new();
    for child in named_children(node) {
        match child.kind() {
            "type_qualifier"
            | "primitive_type"
            | "sized_type_specifier"
            | "type_identifier"
            | "struct_specifier"
            | "enum_specifier"
            | "union_specifier" => parts.push(text(child, source).to_string()),
            _ => {}
        }
    }
    parts.join(" ")
}

/// A C parameter or return type: its dialect type, or why it cannot cross a drop-in bridge.
fn classify(c_type: &str) -> Result<String, String> {
    let normalized = c_type.replace("char const", "const char");
    if normalized.trim_end_matches('*').trim() == "const char"
        && normalized.matches('*').count() == 1
    {
        return Ok("string".into());
    }
    if c_type.contains("(fn)") {
        return Err("takes a function pointer".into());
    }
    if c_type.contains('*') || c_type.contains("[]") {
        return Err(format!(
            "passes a pointer (`{c_type}`): ownership must be designed"
        ));
    }
    match scalar(c_type) {
        Some((dialect, ..)) => Ok(dialect.to_string()),
        None if c_type.starts_with("struct") || c_type.starts_with("union") => {
            Err(format!("passes `{c_type}` by value"))
        }
        None => Err(format!(
            "uses `{c_type}`, which is not a C scalar the bridge knows"
        )),
    }
}

/// Non-`static` function definitions in `.c` files.
pub fn exports(path: &str, source: &str) -> Vec<ExportedFn> {
    if !path.ends_with(".c") {
        return Vec::new();
    }
    let Some(tree) = syntax::parse(&language(), source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for node in named_children(tree.root_node()) {
        if node.kind() != "function_definition" {
            continue;
        }
        if named_children(node)
            .iter()
            .any(|c| c.kind() == "storage_class_specifier" && text(*c, source) == "static")
        {
            continue;
        }
        let base = base_type(node, source);
        let Some(declarator) = node.child_by_field_name("declarator") else {
            continue;
        };
        // A function returning a pointer wraps its function declarator in a pointer declarator.
        let mut ret = base.clone();
        let mut function = declarator;
        while function.kind() == "pointer_declarator" {
            ret.push('*');
            match function.child_by_field_name("declarator") {
                Some(inner) => function = inner,
                None => break,
            }
        }
        if function.kind() != "function_declarator" {
            continue;
        }
        let name = function
            .child_by_field_name("declarator")
            .map(|n| text(n, source).to_string())
            .unwrap_or_default();
        if name == "main" {
            continue;
        }
        let mut blocker = None;
        let mut params = Vec::new();
        if let Some(list) = function.child_by_field_name("parameters") {
            for (i, p) in named_children(list).into_iter().enumerate() {
                if p.kind() == "variadic_parameter" {
                    blocker = Some("is variadic".to_string());
                    continue;
                }
                if p.kind() != "parameter_declaration" {
                    continue;
                }
                let base = base_type(p, source);
                let (c_type, param_name) = match p.child_by_field_name("declarator") {
                    Some(d) => declared(d, &base, source),
                    None => (base.clone(), None),
                };
                if c_type == "void" && param_name.is_none() {
                    continue;
                }
                let ts_type = classify(&c_type).unwrap_or_else(|reason| {
                    blocker.get_or_insert(reason);
                    "unknown".into()
                });
                params.push(Param {
                    name: param_name.unwrap_or_else(|| format!("arg{i}")),
                    ts_type,
                    native_type: Some(c_type),
                    ..Default::default()
                });
            }
        }
        let returns = match ret.as_str() {
            "void" => "void".to_string(),
            r if r.contains('*') => {
                blocker.get_or_insert(format!(
                    "returns a pointer (`{r}`): who frees it must be designed"
                ));
                "unknown".into()
            }
            r => classify(r).unwrap_or_else(|reason| {
                blocker.get_or_insert(reason);
                "unknown".into()
            }),
        };
        out.push(ExportedFn {
            name,
            file: path.to_string(),
            line: syntax::line(node),
            params,
            returns,
            native_return: Some(ret),
            prototype: Some(
                format!("{} {}", base, text(declarator, source))
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            blocker,
            ..Default::default()
        });
    }
    out
}

/// Whether a `.c` file defines `main`: it cannot go into the library `record` builds.
pub fn defines_main(source: &str) -> bool {
    let Some(tree) = syntax::parse(&language(), source) else {
        return false;
    };
    named_children(tree.root_node()).into_iter().any(|n| {
        n.kind() == "function_definition"
            && n.child_by_field_name("declarator")
                .and_then(|d| d.child_by_field_name("declarator"))
                .is_some_and(|i| text(i, source) == "main")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_drop_in_candidates() {
        let source = "#include \"text.h\"\n#include <string.h>\n\
            int word_count(const char *text) { return 0; }\n\
            static int helper(int x) { return x; }\n\
            unsigned long checksum(const char *data, size_t len) { return 0; }\n\
            char *dup(const char *s) { return 0; }\n\
            void fill(char *out, int n) {}\n\
            double mean(const double *xs, int n) { return 0; }\n\
            int sum(int count, ...) { return 0; }\n\
            int main(void) { return 0; }\n";
        let found = exports("text.c", source);
        let names: Vec<&str> = found.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            ["word_count", "checksum", "dup", "fill", "mean", "sum"]
        );
        assert!(found[0].blocker.is_none());
        assert_eq!(found[0].params[0].ts_type, "string");
        assert_eq!(
            found[0].prototype.as_deref(),
            Some("int word_count(const char *text)")
        );
        assert_eq!(found[1].params[1].native_type.as_deref(), Some("size_t"));
        assert!(
            found[2]
                .blocker
                .as_deref()
                .unwrap()
                .contains("returns a pointer")
        );
        assert!(found[3].blocker.as_deref().unwrap().contains("pointer"));
        assert!(found[5].blocker.as_deref().unwrap().contains("variadic"));
        assert!(defines_main(source));
    }

    #[test]
    fn scans_c_risks() {
        let facts = scan(
            "#include \"a.h\"\n#include <stdlib.h>\nint f(int *p) { int *q = malloc(4); for (;;) {} return *p / 2; }\n",
        );
        assert!(
            facts.risks.contains_key(&Risk::Pointers)
                && facts.risks.contains_key(&Risk::DivisionSemantics)
        );
        let specs: Vec<&str> = facts.imports.iter().map(|i| i.specifier.as_str()).collect();
        assert_eq!(specs, ["a.h", "<stdlib.h>"]);
    }
}
