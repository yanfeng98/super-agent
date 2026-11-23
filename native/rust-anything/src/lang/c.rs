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
            namespaces: Vec::new(),
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
            namespaces: Vec::new(),
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
            _ if STANDARD_HEADERS.contains(&spec) => super::Resolution::Stdlib,
            _ => super::Resolution::External(spec.to_string()),
        };
    }
    let dir = from.rsplit_once('/').map_or("", |(d, _)| d);
    let mut candidates = vec![
        join(dir, spec),
        spec.to_string(),
        join("include", spec),
        join("src", spec),
    ];
    if let Some((first, rest)) = spec.split_once('/') {
        let prefix = format!("{first}/");
        if !index.keys().any(|k| k.starts_with(&prefix)) {
            candidates.push(rest.to_string());
        }
    }
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

/// CPU-architecture folders: code there is written for one target (assembly, intrinsics,
/// feature detection), so a recording on this machine cannot vouch for it.
const ARCH_DIRS: &[&str] = &[
    "x86",
    "x86_64",
    "i386",
    "amd64",
    "arm",
    "arm64",
    "aarch64",
    "ppc",
    "powerpc",
    "mips",
    "loongarch",
    "riscv",
    "sparc",
    "s390",
    "alpha",
    "sh4",
    "avr32",
    "bfin",
];

pub fn in_arch_dir(path: &str) -> bool {
    path.rsplit_once('/')
        .is_some_and(|(dir, _)| dir.split('/').any(|seg| ARCH_DIRS.contains(&seg)))
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

pub const STANDARD_HEADERS: &[&str] = &[
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
        "char" => ("int", "i8", "std::ffi::c_char", "c_byte"),
        "signed char" => ("int", "i8", "std::ffi::c_schar", "c_byte"),
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

thread_local! {
    static TYPEDEFS: std::cell::RefCell<HashMap<String, String>> =
        std::cell::RefCell::new(HashMap::new());
    static STRUCTS: std::cell::RefCell<HashMap<String, Fields>> =
        std::cell::RefCell::new(HashMap::new());
}

/// A struct's fields in order, as `(C scalar type, name)`.
type Fields = Vec<(String, String)>;

/// The project's `typedef`s of scalars and enums (`typedef uint32_t AVAdler;`), by name, as the
/// C scalar each one stands for.
pub fn typedefs(root: &Path) -> HashMap<String, String> {
    project_types(root).0
}

/// The project's scalar typedefs (see `typedefs`), and its structs whose fields are all scalars
/// (`typedef struct AVRational { int num, den; } AVRational;`), by the name a declaration uses
/// (`AVRational`, `struct AVRational`).
fn project_types(root: &Path) -> (HashMap<String, String>, HashMap<String, Fields>) {
    let mut raw = HashMap::new();
    let mut bodies = HashMap::new();
    let mut aliases = HashMap::new();
    for entry in crate::walk::builder(root)
        .require_git(false)
        .build()
        .flatten()
    {
        if !entry
            .path()
            .extension()
            .is_some_and(|e| e == "h" || e == "c")
        {
            continue;
        }
        if let Ok(source) = std::fs::read_to_string(entry.path()) {
            let Some(tree) = syntax::parse(&language(), &source) else {
                continue;
            };
            collect_typedefs_in(tree.root_node(), &source, &mut raw);
            collect_structs(tree.root_node(), &source, &mut bodies, &mut aliases);
        }
    }
    let mut out = HashMap::new();
    for name in raw.keys() {
        let mut ty = raw[name].clone();
        for _ in 0..8 {
            match raw.get(&ty) {
                Some(next) => ty = next.clone(),
                None => break,
            }
        }
        if scalar(&ty).is_some() {
            out.insert(name.clone(), ty);
        }
    }
    let field_type = |ty: &str| -> Option<String> {
        let bare = ty
            .split_whitespace()
            .filter(|w| *w != "const")
            .collect::<Vec<_>>()
            .join(" ");
        let resolved = out.get(&bare).cloned().unwrap_or(bare);
        scalar(&resolved)
            .is_some_and(|(d, ..)| d != "void")
            .then_some(resolved)
    };
    let mut structs: HashMap<String, Fields> = bodies
        .into_iter()
        .filter_map(|(name, fields): (String, Fields)| {
            let fields = fields
                .into_iter()
                .map(|(ty, field)| Some((field_type(&ty)?, field)))
                .collect::<Option<Fields>>()?;
            Some((name, fields))
        })
        .collect();
    for (name, target) in aliases {
        if let Some(fields) = structs.get(&target).cloned() {
            structs.entry(name).or_insert(fields);
        }
    }
    (out, structs)
}

#[cfg(test)]
fn collect_typedefs(source: &str, out: &mut HashMap<String, String>) {
    if let Some(tree) = syntax::parse(&language(), source) {
        collect_typedefs_in(tree.root_node(), source, out);
    }
}

fn collect_typedefs_in(root: Node, source: &str, out: &mut HashMap<String, String>) {
    syntax::walk(root, |node| {
        if node.kind() != "type_definition" {
            return;
        }
        let Some(declarator) = node.child_by_field_name("declarator") else {
            return;
        };
        if declarator.kind() != "type_identifier" {
            return;
        }
        let Some(ty) = node.child_by_field_name("type") else {
            return;
        };
        let underlying = match ty.kind() {
            "enum_specifier" => "int".to_string(),
            "primitive_type" | "sized_type_specifier" | "type_identifier" => text(ty, source)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
            _ => return,
        };
        out.insert(text(declarator, source).to_string(), underlying);
    });
}

/// The fields of a struct definition as written, when each is a plain named field (no
/// pointer, array, bit-field or nested definition).
fn struct_fields(spec: Node, source: &str) -> Option<Fields> {
    let body = spec.child_by_field_name("body")?;
    let mut fields = Vec::new();
    for field in named_children(body) {
        match field.kind() {
            "comment" => continue,
            "field_declaration" => {}
            _ => return None,
        }
        if named_children(field)
            .iter()
            .any(|c| c.kind() == "bitfield_clause")
        {
            return None;
        }
        let ty = field.child_by_field_name("type")?;
        if !matches!(
            ty.kind(),
            "primitive_type" | "sized_type_specifier" | "type_identifier"
        ) {
            return None;
        }
        let ty = base_type(field, source);
        let mut cursor = field.walk();
        let names: Vec<Node> = field
            .children_by_field_name("declarator", &mut cursor)
            .collect();
        if names.is_empty() {
            return None;
        }
        for name in names {
            if name.kind() != "field_identifier" {
                return None;
            }
            fields.push((ty.clone(), text(name, source).to_string()));
        }
    }
    (!fields.is_empty()).then_some(fields)
}

/// Struct definitions (`struct Tag { ... }`, `typedef struct { ... } Name;`) and typedefs of
/// structs defined elsewhere (`typedef struct Tag Name;`, kept in `aliases`).
fn collect_structs(
    root: Node,
    source: &str,
    bodies: &mut HashMap<String, Fields>,
    aliases: &mut HashMap<String, String>,
) {
    let mut opaque = Vec::new();
    syntax::walk(root, |node| match node.kind() {
        "struct_specifier" if node.child_by_field_name("body").is_some() => {
            if let Some(tag) = node.child_by_field_name("name") {
                let name = format!("struct {}", text(tag, source));
                match struct_fields(node, source) {
                    Some(fields) => {
                        bodies.insert(name, fields);
                    }
                    None => opaque.push(name),
                }
            }
        }
        "type_definition" => {
            let (Some(declarator), Some(ty)) = (
                node.child_by_field_name("declarator"),
                node.child_by_field_name("type"),
            ) else {
                return;
            };
            if declarator.kind() != "type_identifier" || ty.kind() != "struct_specifier" {
                return;
            }
            let name = text(declarator, source).to_string();
            if ty.child_by_field_name("body").is_some() {
                match struct_fields(ty, source) {
                    Some(fields) => {
                        bodies.insert(name, fields);
                    }
                    None => opaque.push(name),
                }
            } else if let Some(tag) = ty.child_by_field_name("name") {
                aliases.insert(name, format!("struct {}", text(tag, source)));
            }
        }
        _ => {}
    });
    for name in opaque {
        bodies.remove(&name);
    }
}

/// Makes `exports` read these typedefs and structs on this thread, for the project `root` it is
/// looking at.
pub fn use_typedefs(root: &Path) {
    let (scalars, structs) = project_types(root);
    TYPEDEFS.with(|t| *t.borrow_mut() = scalars);
    STRUCTS.with(|t| *t.borrow_mut() = structs);
}

/// The dialect type of a struct passed by value whose fields are all scalars:
/// `struct:<C name>:<C type> <field>,...` (`struct:AVRational:int num,int den`).
fn struct_type(c_type: &str) -> Option<String> {
    if c_type.contains('*') || c_type.contains("[]") || c_type.contains("(fn)") {
        return None;
    }
    let bare = c_type
        .split_whitespace()
        .filter(|w| *w != "const")
        .collect::<Vec<_>>()
        .join(" ");
    let fields = STRUCTS.with(|s| s.borrow().get(&bare).cloned())?;
    let fields: Vec<String> = fields.iter().map(|(t, n)| format!("{t} {n}")).collect();
    Some(format!("struct:{bare}:{}", fields.join(",")))
}

/// A declared type with typedefs and enums replaced by their scalar, `const` kept.
fn resolve_type(c_type: &str) -> String {
    let stars = c_type.matches('*').count();
    let base = c_type.trim_end_matches(['*', ' ']).trim();
    let is_const = base.split_whitespace().any(|w| w == "const");
    let bare: Vec<&str> = base.split_whitespace().filter(|w| *w != "const").collect();
    let bare = bare.join(" ");
    let resolved = if bare.starts_with("enum ") {
        "int".to_string()
    } else {
        TYPEDEFS
            .with(|t| t.borrow().get(&bare).cloned())
            .unwrap_or(bare)
    };
    format!(
        "{}{resolved}{}",
        if is_const { "const " } else { "" },
        "*".repeat(stars)
    )
}

const BYTE_TYPES: &[&str] = &[
    "uint8_t",
    "unsigned char",
    "char",
    "signed char",
    "int8_t",
    "void",
];

/// The pointee of a one-star pointer type, without `const`, and whether it is `const`.
fn pointee(c_type: &str) -> Option<(String, bool)> {
    let normalized = c_type.replace("char const", "const char");
    if normalized.matches('*').count() != 1 || normalized.contains("(fn)") {
        return None;
    }
    let base = normalized.trim_end_matches(['*', ' ']).trim();
    let is_const = base.split_whitespace().any(|w| w == "const");
    let bare = base
        .split_whitespace()
        .filter(|w| *w != "const")
        .collect::<Vec<_>>()
        .join(" ");
    Some((bare, is_const))
}

fn is_integer(c_type: &str) -> bool {
    !c_type.contains('*') && scalar(c_type).is_some_and(|(d, ..)| d == "int")
}

/// Each parameter's dialect type, or why it cannot cross a drop-in bridge. Beyond scalars and
/// `const char *` strings: a byte pointer followed by an integer is a buffer of that length
/// (`bytes` when `const`, `buffer` when the function writes it; the length is `len:<index>`),
/// a mutable pointer to a scalar is `out` (read and written by the function), a `char **` is
/// `outptr:<index>` (the pointer it holds points into the first `char` string or buffer
/// parameter), and a struct of scalars is passed by value (see `struct_type`).
fn classify_params(types: &[(String, Option<String>)]) -> Vec<Result<String, String>> {
    let mut out: Vec<Option<Result<String, String>>> = vec![None; types.len()];
    let lens = |out: &[Option<Result<String, String>>], i: usize| -> bool {
        out[i].is_none() && is_integer(&types[i].0)
    };
    for (i, (ty, _)) in types.iter().enumerate() {
        if let Some((base, true)) = pointee(ty)
            && base == "char"
        {
            out[i] = Some(Ok("string".into()));
        }
    }
    for (i, (ty, _)) in types.iter().enumerate() {
        if out[i].is_some() {
            continue;
        }
        let Some((base, true)) = pointee(ty) else {
            continue;
        };
        if BYTE_TYPES.contains(&base.as_str()) && i + 1 < types.len() && lens(&out, i + 1) {
            out[i] = Some(Ok("bytes".into()));
            out[i + 1] = Some(Ok(format!("len:{i}")));
        }
    }
    for (i, (ty, _)) in types.iter().enumerate() {
        if out[i].is_some() {
            continue;
        }
        let Some((base, false)) = pointee(ty) else {
            continue;
        };
        if !BYTE_TYPES.contains(&base.as_str()) {
            continue;
        }
        let sized = |j: usize| {
            types[j].1.as_deref().is_some_and(|n| {
                let n = n.to_ascii_lowercase();
                n.contains("size") || n.contains("len") || n == "n" || n == "cap"
            })
        };
        let partner = if i + 1 < types.len() && lens(&out, i + 1) {
            Some(i + 1)
        } else {
            (0..types.len()).find(|&j| j != i && lens(&out, j) && sized(j))
        };
        if let Some(j) = partner {
            out[i] = Some(Ok("buffer".into()));
            out[j] = Some(Ok(format!("len:{i}")));
        }
    }
    for (i, (ty, _)) in types.iter().enumerate() {
        if out[i].is_some() || !is_char_pointer_pointer(ty) {
            continue;
        }
        let target = (0..types.len()).find(|&j| {
            matches!(
                out[j].as_ref().and_then(|k| k.as_deref().ok()),
                Some("string" | "bytes" | "buffer")
            ) && pointee(&types[j].0).is_some_and(|(b, _)| b == "char")
        });
        out[i] = Some(match target {
            Some(j) => Ok(format!("outptr:{j}")),
            None => Err(format!(
                "passes a pointer to a pointer (`{ty}`) with no `char *` parameter it could point into"
            )),
        });
    }
    types
        .iter()
        .zip(out)
        .map(|((ty, _), done)| {
            done.unwrap_or_else(|| match pointee(ty) {
                Some((base, true)) if BYTE_TYPES.contains(&base.as_str()) => Err(format!(
                    "passes a pointer (`{ty}`) with no length parameter after it"
                )),
                Some((base, false)) if BYTE_TYPES.contains(&base.as_str()) => Err(format!(
                    "writes to a buffer (`{ty}`) whose size is not a parameter"
                )),
                Some((base, false)) if scalar(&base).is_some_and(|(d, ..)| d != "void") => {
                    Ok("out".into())
                }
                _ => classify(ty),
            })
        })
        .collect()
}

/// Whether `c_type` is `char **` or `const char **`.
fn is_char_pointer_pointer(c_type: &str) -> bool {
    let normalized = c_type.replace("char const", "const char");
    let Some(base) = normalized.trim_end().strip_suffix('*') else {
        return false;
    };
    pointee(base).is_some_and(|(b, _)| b == "char")
}

/// A C parameter or return type: its dialect type, or why it cannot cross a drop-in bridge.
fn classify(c_type: &str) -> Result<String, String> {
    if c_type.contains("(fn)") {
        return Err("takes a function pointer".into());
    }
    if c_type.contains('*') || c_type.contains("[]") {
        return Err(format!(
            "passes a pointer (`{c_type}`): ownership must be designed"
        ));
    }
    if let Some(ts) = struct_type(c_type) {
        return Ok(ts);
    }
    match scalar(c_type) {
        Some((dialect, ..)) => Ok(dialect.to_string()),
        None if c_type.starts_with("struct") || c_type.starts_with("union") => Err(format!(
            "passes `{c_type}` by value, and not all of its fields are scalars"
        )),
        None => Err(format!(
            "uses `{c_type}`, which is not a C scalar the bridge knows"
        )),
    }
}

/// Whether a function body allocates what it may return (`malloc`, `strdup`, `av_asprintf`).
fn allocates(body: Node, source: &str) -> bool {
    let mut found = false;
    syntax::walk(body, |n| {
        if n.kind() == "call_expression" {
            let callee = n
                .child_by_field_name("function")
                .map_or("", |f| text(f, source));
            if callee.contains("alloc") || callee.contains("dup") || callee.contains("printf") {
                found = true;
            }
        }
    });
    found
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
        let mut declared_params = Vec::new();
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
                declared_params.push((
                    resolve_type(&c_type),
                    param_name.unwrap_or_else(|| format!("arg{i}")),
                ));
            }
        }
        let typed: Vec<(String, Option<String>)> = declared_params
            .iter()
            .map(|(t, n)| (t.clone(), Some(n.clone())))
            .collect();
        let kinds = classify_params(&typed);
        let mut params = Vec::new();
        for ((c_type, param_name), kind) in declared_params.into_iter().zip(kinds) {
            let ts_type = kind.unwrap_or_else(|reason| {
                blocker.get_or_insert(reason);
                "unknown".into()
            });
            params.push(Param {
                name: param_name,
                ts_type,
                native_type: Some(c_type),
                ..Default::default()
            });
        }
        let ret = resolve_type(&ret);
        let returns = match ret.as_str() {
            "void" => "void".to_string(),
            r if r.contains('*') => {
                let into = pointee(r).and_then(|(target, _)| {
                    params.iter().position(|p| {
                        matches!(p.ts_type.as_str(), "string" | "bytes" | "buffer")
                            && p.native_type
                                .as_deref()
                                .and_then(pointee)
                                .is_some_and(|(b, _)| b == target)
                    })
                });
                let body_allocates = node
                    .child_by_field_name("body")
                    .is_some_and(|b| allocates(b, source));
                match into {
                    Some(i) if !body_allocates => format!("ptr:{i}"),
                    _ => {
                        blocker.get_or_insert(format!(
                            "returns a pointer (`{r}`): who frees it must be designed"
                        ));
                        "unknown".into()
                    }
                }
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

/// The project's `.c` files (beside the headers `unit` includes with quotes) that define a
/// function one of `ported` calls: a port replaces those too, since Rust cannot call back into
/// them. Calls from functions kept in C do not count.
pub fn called_implementations(root: &Path, unit: &str, ported: &[&str]) -> Vec<std::path::PathBuf> {
    let Ok(source) = std::fs::read_to_string(root.join(unit)) else {
        return Vec::new();
    };
    let Some(tree) = syntax::parse(&language(), &source) else {
        return Vec::new();
    };
    let mut callees = std::collections::HashSet::new();
    let mut headers = Vec::new();
    for function in named_children(tree.root_node()) {
        let name = function
            .child_by_field_name("declarator")
            .map(|mut d| {
                while d.kind() != "function_declarator" {
                    match d.child_by_field_name("declarator") {
                        Some(inner) => d = inner,
                        None => break,
                    }
                }
                d.child_by_field_name("declarator")
                    .map_or("", |n| text(n, &source))
            })
            .unwrap_or("");
        if function.kind() != "function_definition" || !ported.contains(&name) {
            continue;
        }
        syntax::walk(function, |node| {
            if node.kind() == "call_expression"
                && let Some(f) = node.child_by_field_name("function")
            {
                callees.insert(text(f, &source).to_string());
            }
        });
    }
    syntax::walk(tree.root_node(), |node| {
        if node.kind() == "preproc_include"
            && let Some(path) = node.child_by_field_name("path")
            && path.kind() == "string_literal"
        {
            headers.push(text(path, &source).trim_matches('"').to_string());
        }
    });
    let dir = Path::new(unit).parent().unwrap_or(Path::new(""));
    let mut out = Vec::new();
    for header in headers {
        let Some(stem) = header.strip_suffix(".h") else {
            continue;
        };
        let mut candidates = vec![
            root.join(dir).join(format!("{stem}.c")),
            root.join(format!("{stem}.c")),
        ];
        if let Some((_, rest)) = stem.split_once('/') {
            candidates.push(root.join(format!("{rest}.c")));
        }
        let Some(file) = candidates.into_iter().find(|c| c.is_file()) else {
            continue;
        };
        if file == root.join(unit) || out.contains(&file) {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let name = file.to_string_lossy().into_owned();
        if exports(&name, &text)
            .iter()
            .any(|f| callees.contains(&f.name))
        {
            out.push(file);
        }
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
            char *dup(const char *s) { return strdup(s); }\n\
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
        assert!(found[3].blocker.is_none());
        assert_eq!(found[3].params[0].ts_type, "buffer");
        assert_eq!(found[3].params[1].ts_type, "len:0");
        assert!(found[4].blocker.as_deref().unwrap().contains("pointer"));
        assert!(found[5].blocker.as_deref().unwrap().contains("variadic"));
        assert!(defines_main(source));
    }

    #[test]
    fn pairs_pointers_with_lengths_and_resolves_typedefs() {
        let header = "typedef uint32_t Checksum;\ntypedef Checksum Alias;\n\
            typedef enum { ROUND_DOWN, ROUND_UP } Rounding;\ntypedef struct Ctx Ctx;\n";
        let mut defs = HashMap::new();
        collect_typedefs(header, &mut defs);
        let dir = std::env::temp_dir().join(format!("ra-typedefs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("t.h"), header).unwrap();
        use_typedefs(&dir);
        std::fs::remove_dir_all(&dir).unwrap();
        let source = "Alias update(Alias sum, const uint8_t *buf, size_t len) { return sum; }\n\
            int decode(uint8_t *out, const char *in, int out_size) { return 0; }\n\
            char *encode(char *out, int out_size, const uint8_t *in, int in_size) { return out; }\n\
            int reduce(int *num, int *den, int64_t n, int64_t d, enum Mode m, Rounding r) { return 0; }\n\
            char *find(const char *hay, const char *needle) { return (char *)hay; }\n\
            char *copy(const char *s) { return strdup(s); }\n\
            void sum16(uint8_t *dst, const uint8_t *src, size_t len) {}\n\
            int init(Ctx *ctx) { return 0; }\n";
        let found = exports("u.c", source);
        let kinds = |i: usize| -> Vec<&str> {
            found[i].params.iter().map(|p| p.ts_type.as_str()).collect()
        };
        assert_eq!(found[0].native_return.as_deref(), Some("uint32_t"));
        assert_eq!(kinds(0), ["int", "bytes", "len:1"]);
        assert_eq!(kinds(1), ["buffer", "string", "len:0"]);
        assert_eq!(kinds(2), ["buffer", "len:0", "bytes", "len:2"]);
        assert_eq!(found[2].returns, "ptr:0");
        assert_eq!(kinds(3), ["out", "out", "int", "int", "int", "int"]);
        assert!(found[3].blocker.is_none());
        assert_eq!(found[4].returns, "ptr:0");
        assert!(
            found[5]
                .blocker
                .as_deref()
                .unwrap()
                .contains("who frees it")
        );
        assert!(
            found[6]
                .blocker
                .as_deref()
                .unwrap()
                .contains("size is not a parameter")
        );
        assert!(found[7].blocker.is_some());
        assert!(defs.contains_key("Rounding"));
        assert!(in_arch_dir("libavutil/x86/cpu.c") && !in_arch_dir("libavutil/crc.c"));
    }

    #[test]
    fn passes_structs_of_scalars_by_value_and_char_pointer_pointers() {
        let header = "typedef int64_t Big;\n\
            typedef struct AVRational { int num; int den; } AVRational;\n\
            struct pair { Big a, b; };\ntypedef struct pair Pair;\n\
            typedef struct { int *p; } Holder;\nstruct bits { int a : 3; };\n";
        let dir = std::env::temp_dir().join(format!("ra-structs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("t.h"), header).unwrap();
        use_typedefs(&dir);
        std::fs::remove_dir_all(&dir).unwrap();
        let source = "AVRational mul(AVRational b, const AVRational c) { return b; }\n\
            Pair swap(struct pair p) { return p; }\n\
            int start(const char *str, const char *pfx, const char **ptr) { return 0; }\n\
            long parse(const char *s, char **end) { return 0; }\n\
            int held(Holder h) { return 0; }\n\
            int bits(struct bits b) { return 0; }\n\
            int token(const char **buf) { return 0; }\n";
        let found = exports("s.c", source);
        let rational = "struct:AVRational:int num,int den";
        assert!(found[0].blocker.is_none(), "{:?}", found[0].blocker);
        assert_eq!(found[0].params[0].ts_type, rational);
        assert_eq!(found[0].params[1].ts_type, rational);
        assert_eq!(found[0].returns, rational);
        assert_eq!(
            found[1].params[0].ts_type,
            "struct:struct pair:int64_t a,int64_t b"
        );
        assert_eq!(found[1].returns, "struct:Pair:int64_t a,int64_t b");
        assert_eq!(found[2].params[2].ts_type, "outptr:0");
        assert_eq!(found[3].params[1].ts_type, "outptr:0");
        assert!(found[4].blocker.is_some());
        assert!(
            found[5]
                .blocker
                .as_deref()
                .unwrap()
                .contains("not all of its fields")
        );
        assert!(
            found[6]
                .blocker
                .as_deref()
                .unwrap()
                .contains("pointer to a pointer")
        );
    }

    #[test]
    fn counts_the_implementations_ported_functions_call() {
        let root = std::env::temp_dir().join(format!("ra-called-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("lib")).unwrap();
        let write = |p: &str, t: &str| std::fs::write(root.join(p), t).unwrap();
        write(
            "lib/crc.h",
            "unsigned crc(const unsigned char *b, unsigned long n);\n",
        );
        write(
            "lib/crc.c",
            "unsigned crc(const unsigned char *b, unsigned long n) { return n; }\n",
        );
        write("lib/print.h", "void print(const char *s);\n");
        write("lib/print.c", "void print(const char *s) {}\n");
        write(
            "lib/wrap.c",
            "#include \"lib/crc.h\"\n#include \"print.h\"\n\
             unsigned one(const unsigned char *b, unsigned long n) { return crc(b, n); }\n\
             void kept(const char *s) { print(s); }\n",
        );
        let found = called_implementations(&root, "lib/wrap.c", &["one"]);
        assert_eq!(found, [root.join("lib/crc.c")]);
        let both = called_implementations(&root, "lib/wrap.c", &["one", "kept"]);
        assert_eq!(both.len(), 2);
        std::fs::remove_dir_all(&root).unwrap();
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
