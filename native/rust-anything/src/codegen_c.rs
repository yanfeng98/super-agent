//! C ABI bridge: drop-in `extern "C"` replacements with the original names and signatures.
//!
//! The port is safe Rust over C's ABI types (`c_int`, `usize`, `f64`, ...), with each
//! `const char *` as `Option<&[u8]>` (`None` for `NULL`). `lib.rs` is the only place with
//! `unsafe`: reading C strings, and exporting unmangled symbols.

use crate::MIN_RUST;
use std::fmt::Write as _;

use crate::bridge::Manifest;
use crate::lang::c::scalar;
use crate::ts::exports::{ExportedFn, Param};

/// How a C parameter crosses the bridge.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CKind {
    Scalar,
    /// `const char *`, NUL-terminated.
    Str,
    /// A `const` byte pointer read for the length in another parameter.
    Bytes,
    /// A byte pointer the function writes, sized by another parameter.
    Buffer,
    /// The length of the buffer parameter at this index.
    Len(usize),
    /// A pointer to one scalar the function may read and write.
    Out,
    /// A struct of scalars, passed by value (`struct:<C name>:<fields>`).
    Struct,
    /// A `char **` whose pointer points into the string or buffer parameter at this index.
    OutPtr(usize),
}

/// A struct of scalar fields that crosses the bridge by value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CStruct {
    /// The type as C code names it (`AVRational`, `struct point`).
    pub c_name: String,
    /// `(C scalar type, field name)` in declaration order.
    pub fields: Vec<(String, String)>,
}

impl CStruct {
    /// The Rust type at the crate root (`AVRational`, `point`).
    pub fn rust_name(&self) -> &str {
        self.c_name
            .strip_prefix("struct ")
            .unwrap_or(&self.c_name)
            .trim()
    }
}

/// The struct a dialect type stands for (`struct:AVRational:int num,int den`).
pub fn c_struct(ts_type: &str) -> Option<CStruct> {
    let (c_name, fields) = ts_type.strip_prefix("struct:")?.split_once(':')?;
    let fields = fields
        .split(',')
        .map(|f| {
            let (ty, name) = f.trim().rsplit_once(' ')?;
            Some((ty.to_string(), name.to_string()))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(CStruct {
        c_name: c_name.to_string(),
        fields,
    })
}

/// The structs `m`'s functions take or return by value, each once.
pub fn structs(m: &Manifest) -> Vec<CStruct> {
    let mut out: Vec<CStruct> = Vec::new();
    for f in &m.exports {
        for ts in f
            .params
            .iter()
            .map(|p| p.ts_type.as_str())
            .chain([f.returns.as_str()])
        {
            if let Some(s) = c_struct(ts)
                && !out.iter().any(|o| o.c_name == s.c_name)
            {
                out.push(s);
            }
        }
    }
    out
}

/// C definitions of `m`'s structs, for files that do not include the project's headers.
pub fn c_struct_defs(m: &Manifest) -> String {
    let mut out = String::new();
    for s in structs(m) {
        let fields: String = s
            .fields
            .iter()
            .map(|(ty, name)| format!(" {ty} {name};"))
            .collect();
        if s.c_name.starts_with("struct ") {
            let _ = writeln!(out, "{} {{{fields} }};", s.c_name);
        } else {
            let _ = writeln!(out, "typedef struct {{{fields} }} {};", s.c_name);
        }
    }
    out
}

/// The Rust definition of a struct, at the crate root.
fn rust_struct(s: &CStruct) -> String {
    let fields: String = s
        .fields
        .iter()
        .map(|(ty, name)| format!("    pub {}: {},\n", ident(name), abi(ty)))
        .collect();
    let name = s.rust_name();
    let mut allow = Vec::new();
    if !name.starts_with(|c: char| c.is_ascii_uppercase()) || name.contains('_') {
        allow.push("non_camel_case_types");
    }
    if s.fields
        .iter()
        .any(|(_, n)| n.chars().any(|c| c.is_ascii_uppercase()))
    {
        allow.push("non_snake_case");
    }
    let allow = if allow.is_empty() {
        String::new()
    } else {
        format!("#[allow({})]\n", allow.join(", "))
    };
    format!(
        "\n/// `{}` as C lays it out: passed and returned by value.\n#[repr(C)]\n\
         #[derive(Clone, Copy, Debug, Default, PartialEq)]\n{allow}pub struct {name} {{\n{fields}}}\n",
        s.c_name,
    )
}

pub fn kind(p: &Param) -> CKind {
    match p.ts_type.as_str() {
        "string" => CKind::Str,
        "bytes" => CKind::Bytes,
        "buffer" => CKind::Buffer,
        "out" => CKind::Out,
        t if t.starts_with("struct:") => CKind::Struct,
        t => {
            if let Some(i) = t.strip_prefix("outptr:").and_then(|i| i.parse().ok()) {
                return CKind::OutPtr(i);
            }
            t.strip_prefix("len:")
                .and_then(|i| i.parse().ok())
                .map_or(CKind::Scalar, CKind::Len)
        }
    }
}

/// The parameter a returned pointer points into (`returns` is `ptr:<index>`).
pub fn returns_into(f: &ExportedFn) -> Option<usize> {
    f.returns.strip_prefix("ptr:").and_then(|i| i.parse().ok())
}

/// Whether recorded results carry what the function wrote through its pointers.
pub fn writes(f: &ExportedFn) -> bool {
    f.params
        .iter()
        .any(|p| matches!(kind(p), CKind::Buffer | CKind::Out | CKind::OutPtr(_)))
}

/// The ABI type of a C scalar, as Rust spells it.
fn abi(c_type: &str) -> String {
    scalar(c_type).map_or_else(|| "()".to_string(), |(_, _, abi, _)| abi.to_string())
}

/// What a pointer type points to, without `const`.
pub fn pointee(c_type: &str) -> String {
    c_type
        .trim_end_matches(['*', ' '])
        .split_whitespace()
        .filter(|w| *w != "const")
        .collect::<Vec<_>>()
        .join(" ")
}

/// The Rust ABI type a C pointer points to (`u8`, `std::ffi::c_char`, `std::ffi::c_void`).
fn pointee_abi(c_type: &str) -> String {
    match pointee(c_type).as_str() {
        "void" => "std::ffi::c_void".into(),
        "char" => "std::ffi::c_char".into(),
        other => abi(other),
    }
}

fn native(p: &Param) -> &str {
    p.native_type.as_deref().unwrap_or("int")
}

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "static", "struct", "trait", "true", "type", "unsafe", "use", "where", "while", "async",
    "await", "dyn", "abstract", "become", "box", "do", "final", "macro", "override", "priv",
    "typeof", "unsized", "virtual", "yield", "try", "gen",
];

/// A parameter name as a Rust identifier (`in` becomes `r#in`).
fn ident(name: &str) -> String {
    if KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_string()
    }
}

fn ret_type(f: &ExportedFn) -> Option<String> {
    if let Some(s) = c_struct(&f.returns) {
        return Some(format!("crate::{}", s.rust_name()));
    }
    let r = f.native_return.as_deref().unwrap_or("void");
    if r.contains('*') {
        return Some(format!("*mut {}", pointee_abi(r)));
    }
    (r != "void").then(|| abi(r))
}

/// What a `char **` points to (`const char *`).
fn inner(c_type: &str) -> String {
    c_type
        .trim_end()
        .strip_suffix('*')
        .unwrap_or(c_type)
        .trim_end()
        .to_string()
}

/// A C declaration of `name` with type `c_type` (`const uint8_t *in`).
pub fn c_decl(c_type: &str, name: &str) -> String {
    let stars = c_type.matches('*').count();
    let base = c_type.trim_end_matches(['*', ' ']);
    if stars == 0 {
        format!("{base} {name}")
    } else {
        format!("{base} {}{name}", "*".repeat(stars))
    }
}

/// The function's prototype with typedefs resolved to the scalars they stand for, so it
/// compiles without the project's headers.
pub fn c_prototype(f: &ExportedFn, name: &str) -> String {
    let params: Vec<String> = f
        .params
        .iter()
        .map(|p| c_decl(native(p), &p.name))
        .collect();
    let params = if params.is_empty() {
        "void".to_string()
    } else {
        params.join(", ")
    };
    c_decl(
        f.native_return.as_deref().unwrap_or("void"),
        &format!("{name}({params})"),
    )
}

pub fn port_rs(m: &Manifest) -> String {
    let mut out = format!(
        "//! The Rust port of `{}`: safe Rust; `lib.rs` exports each function under its C name.\n//!\n\
         //! Replace each `unimplemented!` with the port of the C function. C has no error channel:\n\
         //! return what the C code returns, and never panic (a panic aborts at the C boundary).\n\n\
         #![forbid(unsafe_code)]\n",
        m.unit
    );
    for f in &m.exports {
        let shown: Vec<&Param> = f
            .params
            .iter()
            .filter(|p| !matches!(kind(p), CKind::Len(_)))
            .collect();
        let params: Vec<String> = shown
            .iter()
            .map(|p| {
                let ty = match kind(p) {
                    CKind::Str => "Option<&[u8]>".to_string(),
                    CKind::Bytes => "&[u8]".to_string(),
                    CKind::Buffer => "&mut [u8]".to_string(),
                    CKind::Out => format!("Option<&mut {}>", abi(&pointee(native(p)))),
                    CKind::Struct => c_struct(&p.ts_type)
                        .map_or_else(|| "()".into(), |s| format!("crate::{}", s.rust_name())),
                    CKind::OutPtr(_) => "Option<&mut Option<usize>>".to_string(),
                    CKind::Scalar | CKind::Len(_) => abi(native(p)),
                };
                format!("{}: {ty}", ident(&p.name))
            })
            .collect();
        let ret = if returns_into(f).is_some() {
            " -> Option<usize>".to_string()
        } else {
            ret_type(f).map(|r| format!(" -> {r}")).unwrap_or_default()
        };
        out.push('\n');
        let _ = writeln!(
            out,
            "/// Port of `{}` (`{}:{}`).",
            f.prototype.as_deref().unwrap_or(&f.name),
            f.file,
            f.line
        );
        for p in &shown {
            let doc = match kind(p) {
                CKind::Str => "the bytes of the C string, without its NUL; `None` for `NULL`.",
                CKind::Bytes => "the bytes the C code reads, as long as its length parameter says.",
                CKind::Buffer => {
                    "the buffer the C code writes, as long as its size parameter says (a negative \
                     size is empty)."
                }
                CKind::Out => "the value behind the pointer; `None` for `NULL`.",
                CKind::OutPtr(i) => {
                    let _ = writeln!(
                        out,
                        "///\n/// `{}`: the pointer behind it, as the offset into `{}` it points to \
                         (`None` for `NULL`); the outer `None` is a `NULL` `{}`.",
                        p.name,
                        f.params[i].name,
                        native(p)
                    );
                    continue;
                }
                _ => continue,
            };
            let _ = writeln!(out, "///\n/// `{}`: {doc}", p.name);
        }
        if let Some(i) = returns_into(f) {
            let _ = writeln!(
                out,
                "///\n/// Returns the offset into `{}` that the C function's pointer points to; `None` for `NULL`.",
                f.params[i].name
            );
        }
        let _ = writeln!(out, "pub fn {}({}){ret} {{", f.name, params.join(", "));
        let names: Vec<String> = shown
            .iter()
            .map(|p| format!("&{}", ident(&p.name)))
            .collect();
        match names.len() {
            0 => {}
            1 => {
                let _ = writeln!(out, "    let _ = {};", names[0]);
            }
            _ => {
                let _ = writeln!(out, "    let _ = ({});", names.join(", "));
            }
        }
        let _ = writeln!(
            out,
            "    unimplemented!(\"port `{}` from {}\")\n}}",
            f.name, f.file
        );
    }
    out
}

/// Whether `port.rs` takes this `const char *` parameter of `f` as `CChars` (read up to its NUL
/// as the port goes) instead of a measured slice.
fn lazy(port: &crate::codegen::PortApi, f: &ExportedFn, p: &Param) -> bool {
    kind(p) == CKind::Str
        && port
            .functions
            .iter()
            .find(|g| g.rust_name.trim_start_matches("r#") == f.name)
            .and_then(|g| g.params.iter().find(|(name, _)| *name == ident(&p.name)))
            .is_some_and(|(_, ty)| ty.contains("CChars"))
}

/// `CChars`, emitted at the crate root when a port takes one.
const C_CHARS: &str = r#"
/// A `const char *` read one byte at a time up to its NUL, without measuring it first: for ports
/// where `strlen` would cost more than the work (`av_strcasecmp` on short words). A port opts in
/// by typing the parameter `Option<crate::CChars<'_>>`; `rust-anything refresh` then passes it.
#[derive(Clone, Debug)]
pub struct CChars<'a> {
    ptr: std::ptr::NonNull<std::ffi::c_char>,
    life: std::marker::PhantomData<&'a std::ffi::CStr>,
}

impl<'a> CChars<'a> {
    /// The bytes of `s`, for calling a port from Rust.
    pub fn new(s: &'a std::ffi::CStr) -> Self {
        Self {
            ptr: std::ptr::NonNull::from(s).cast(),
            life: std::marker::PhantomData,
        }
    }

    /// The next byte without moving past it; `0` at the end.
    pub fn peek(&self) -> u8 {
        // SAFETY: `ptr` points into a NUL-terminated string valid for `'a`, at or before its NUL.
        unsafe { *self.ptr.as_ptr() as u8 }
    }

    /// The rest of the string as a slice, without its NUL (this measures it).
    pub fn to_bytes(&self) -> &'a [u8] {
        // SAFETY: as in `peek`; the rest of the string is itself NUL-terminated.
        unsafe { std::ffi::CStr::from_ptr(self.ptr.as_ptr()) }.to_bytes()
    }
}

impl Iterator for CChars<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        let b = self.peek();
        if b == 0 {
            return None;
        }
        // SAFETY: `b` is not the NUL, so the next byte is still inside the string.
        self.ptr = unsafe { self.ptr.add(1) };
        Some(b)
    }
}

impl std::iter::FusedIterator for CChars<'_> {}
"#;

pub fn lib_rs(m: &Manifest, port: &str) -> String {
    let port = crate::codegen::parse_port(port);
    let all = || m.exports.iter().flat_map(|f| f.params.iter().map(kind));
    let any_lazy = m
        .exports
        .iter()
        .any(|f| f.params.iter().any(|p| lazy(&port, f, p)));
    let any_measured = m.exports.iter().any(|f| {
        f.params
            .iter()
            .any(|p| kind(p) == CKind::Str && !lazy(&port, f, p))
    });
    let any_string = all().any(|k| k == CKind::Str);
    let any_bytes = all().any(|k| k == CKind::Bytes);
    let any_buffer = all().any(|k| k == CKind::Buffer);
    let any_out = all().any(|k| k == CKind::Out);
    let any_outptr = all().any(|k| matches!(k, CKind::OutPtr(_)));
    let any_len = all().any(|k| matches!(k, CKind::Len(_)));
    let mut out = format!(
        "//! C ABI for `{}`: drop-in replacements with the original names and signatures, generated\n\
         //! by rust-anything from `rust-anything.json`. All of the crate's `unsafe` code lives in this file.\n\n",
        m.unit
    );
    out.push_str("pub mod port;\n");
    for s in structs(m) {
        out.push_str(&rust_struct(&s));
    }
    if any_string {
        out.push_str("\nuse std::ffi::c_char;\n");
    }
    if any_lazy {
        out.push_str(C_CHARS);
    }
    if any_string || any_bytes || any_buffer || any_out || any_outptr || any_len {
        out.push_str("\nmod __rust_anything_c_abi {\n");
        if any_lazy {
            out.push_str(
                r#"    /// A `const char *` argument read as the port goes; `None` for `NULL`.
    pub(super) fn c_chars<'a>(ptr: *const std::ffi::c_char) -> Option<super::CChars<'a>> {
        std::ptr::NonNull::new(ptr.cast_mut()).map(|ptr| super::CChars {
            ptr,
            life: std::marker::PhantomData,
        })
    }
"#,
            );
        }
        if any_measured {
            out.push_str(
                r#"    use std::ffi::{CStr, c_char};

    /// A `const char *` argument as bytes; `None` for `NULL`.
    pub(super) fn c_str<'a>(ptr: *const c_char) -> Option<&'a [u8]> {
        if ptr.is_null() {
            return None;
        }

        // SAFETY: the C API takes NUL-terminated strings that stay valid for the call;
        // NULL is handled above.
        Some(unsafe { CStr::from_ptr(ptr) }.to_bytes())
    }
"#,
            );
        }
        if any_bytes {
            out.push_str(
                r#"
    /// A `const` byte pointer and its length as a slice; empty for `NULL` or a zero length.
    pub(super) fn bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
        if ptr.is_null() || len == 0 {
            return &[];
        }

        // SAFETY: the C API passes `len` readable bytes at `ptr` that stay valid for the call;
        // NULL is handled above.
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
"#,
            );
        }
        if any_buffer {
            out.push_str(
                r#"
    /// A writable byte pointer and its size as a slice; empty for `NULL` or a zero size.
    pub(super) fn bytes_mut<'a>(ptr: *mut u8, len: usize) -> &'a mut [u8] {
        if ptr.is_null() || len == 0 {
            return &mut [];
        }

        // SAFETY: the C API passes `len` writable bytes at `ptr`, used by nothing else during the
        // call; NULL is handled above.
        unsafe { std::slice::from_raw_parts_mut(ptr, len) }
    }
"#,
            );
        }
        if any_out {
            out.push_str(
                r#"
    /// A pointer to one value as a reference; `None` for `NULL`.
    pub(super) fn out<'a, T>(ptr: *mut T) -> Option<&'a mut T> {
        // SAFETY: the C API passes NULL or a valid, aligned pointer used by nothing else
        // during the call.
        unsafe { ptr.as_mut() }
    }
"#,
            );
        }
        if any_outptr {
            out.push_str(
                r#"
    /// Where the pointer behind `slot` points, as an offset from `base`: `None` for a `NULL`
    /// `slot`, `Some(None)` when it holds `NULL`.
    pub(super) fn offset_in<T>(slot: *mut *const T, base: *const u8) -> Option<Option<usize>> {
        // SAFETY: the C API passes NULL or a valid, aligned pointer to a pointer, used by nothing
        // else during the call; NULL is handled by `as_ref`.
        let target = unsafe { slot.as_ref() }?;
        Some((!target.is_null()).then(|| target.addr().wrapping_sub(base.addr())))
    }

    /// Points the pointer behind `slot` at `offset` bytes into `base` (`NULL` for `None`); a
    /// `NULL` `slot` is left alone.
    pub(super) fn set_offset<T>(slot: *mut *const T, base: *const u8, offset: Option<Option<usize>>) {
        // SAFETY: as in `offset_in`.
        if let (Some(target), Some(offset)) = (unsafe { slot.as_mut() }, offset) {
            *target = offset.map_or(std::ptr::null(), |o| base.wrapping_add(o).cast());
        }
    }
"#,
            );
        }
        if any_len {
            out.push_str(
                r#"
    /// A C length as `usize`; a negative length is zero.
    pub(super) fn len<T: TryInto<usize>>(n: T) -> usize {
        n.try_into().unwrap_or(0)
    }
"#,
            );
        }
        out.push_str("}\n");
    }
    for f in &m.exports {
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| {
                let ty = match kind(p) {
                    CKind::Str => "*const c_char".to_string(),
                    CKind::Bytes => format!("*const {}", pointee_abi(native(p))),
                    CKind::Buffer => format!("*mut {}", pointee_abi(native(p))),
                    CKind::Out => format!("*mut {}", abi(&pointee(native(p)))),
                    CKind::Struct => c_struct(&p.ts_type)
                        .map_or_else(|| "()".into(), |s| s.rust_name().to_string()),
                    CKind::OutPtr(_) => format!("*mut *const {}", pointee_abi(&inner(native(p)))),
                    CKind::Scalar | CKind::Len(_) => abi(native(p)),
                };
                format!("{}: {ty}", ident(&p.name))
            })
            .collect();
        let len_of = |i: usize| -> String {
            f.params
                .iter()
                .find(|p| kind(p) == CKind::Len(i))
                .map_or_else(
                    || "0".to_string(),
                    |p| format!("crate::__rust_anything_c_abi::len({})", ident(&p.name)),
                )
        };
        let args: Vec<String> = f
            .params
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                let name = ident(&p.name);
                Some(match kind(p) {
                    CKind::Str if lazy(&port, f, p) => {
                        format!("crate::__rust_anything_c_abi::c_chars({name})")
                    }
                    CKind::Str => format!("crate::__rust_anything_c_abi::c_str({name})"),
                    CKind::Bytes => format!(
                        "crate::__rust_anything_c_abi::bytes({name}.cast::<u8>(), {})",
                        len_of(i)
                    ),
                    CKind::Buffer => format!(
                        "crate::__rust_anything_c_abi::bytes_mut({name}.cast::<u8>(), {})",
                        len_of(i)
                    ),
                    CKind::Out => format!("crate::__rust_anything_c_abi::out({name})"),
                    CKind::OutPtr(_) => format!("{}_offset.as_mut()", p.name),
                    CKind::Len(_) => return None,
                    CKind::Struct | CKind::Scalar => name,
                })
            })
            .collect();
        let ret = ret_type(f).map(|r| format!(" -> {r}")).unwrap_or_default();
        let call = format!("port::{}({})", f.name, args.join(", "));
        let body = match returns_into(f) {
            Some(i) => {
                let target = ident(&f.params[i].name);
                let cast = if kind(&f.params[i]) == CKind::Buffer {
                    target
                } else {
                    format!("{target}.cast_mut()")
                };
                let ret_ptr = ret_type(f).unwrap_or_default();
                format!(
                    "match {call} {{\n        Some(offset) => {cast}.cast::<u8>().wrapping_add(offset).cast(),\n        \
                     None => std::ptr::null_mut::<{}>(),\n    }}",
                    ret_ptr.trim_start_matches("*mut ")
                )
            }
            None => call,
        };
        let outptrs: Vec<(String, String)> = f
            .params
            .iter()
            .filter_map(|p| match kind(p) {
                CKind::OutPtr(i) => {
                    let base = ident(&f.params[i].name);
                    let base = if kind(&f.params[i]) == CKind::Buffer {
                        format!("{base}.cast::<u8>().cast_const()")
                    } else {
                        format!("{base}.cast::<u8>()")
                    };
                    Some((ident(&p.name), base))
                }
                _ => None,
            })
            .collect();
        let body = if outptrs.is_empty() {
            body
        } else {
            let before: String = outptrs
                .iter()
                .map(|(name, base)| {
                    format!(
                        "let mut {}_offset = crate::__rust_anything_c_abi::offset_in({name}, {base});\n    ",
                        name.trim_start_matches("r#")
                    )
                })
                .collect();
            let after: String = outptrs
                .iter()
                .map(|(name, base)| {
                    format!(
                        "\n    crate::__rust_anything_c_abi::set_offset({name}, {base}, {}_offset);",
                        name.trim_start_matches("r#")
                    )
                })
                .collect();
            if ret_type(f).is_some() {
                format!("{before}let result = {body};{after}\n    result")
            } else {
                format!("{before}{body};{after}")
            }
        };
        let prototype = f.prototype.as_deref().unwrap_or(&f.name);
        let _ = writeln!(
            out,
            "\n/// `{prototype}` (`{}:{}`).\n\
             #[cfg(not(feature = \"shadow\"))]\n\
             // SAFETY: exported under the C name it replaces: link it instead of the original object.\n\
             #[unsafe(no_mangle)]\npub extern \"C\" fn {name}({params}){ret} {{\n    {body}\n}}\n\n\
             /// `{name}` as `rust_anything_{name}`, for shadow builds beside the original.\n\
             // SAFETY: a name no C code defines.\n\
             #[unsafe(no_mangle)]\npub extern \"C\" fn rust_anything_{name}({params}){ret} {{\n    {body}\n}}",
            f.file,
            f.line,
            name = f.name,
            params = params.join(", "),
        );
    }
    out
}

/// `include/<lib>.h`: the prototypes, and the `rust_anything_` aliases.
pub fn header(m: &Manifest) -> String {
    let guard = format!("{}_H", m.lib_name().to_uppercase());
    let mut out = format!(
        "/* C API of the Rust port of `{}`, generated by rust-anything. */\n#ifndef {guard}\n#define {guard}\n\n\
         #include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n#include <sys/types.h>\n\n",
        m.unit
    );
    let defs = c_struct_defs(m);
    if !defs.is_empty() {
        let _ = writeln!(out, "{defs}");
    }
    for f in &m.exports {
        let _ = writeln!(out, "{};", c_prototype(f, &f.name));
    }
    out.push_str("\n/* The same functions under `rust_anything_` names, for shadow runs. */\n");
    for f in &m.exports {
        let _ = writeln!(
            out,
            "{};",
            c_prototype(f, &format!("rust_anything_{}", f.name))
        );
    }
    let _ = writeln!(out, "\n#endif /* {guard} */");
    out
}

/// A C expression for the byte length of buffer parameter `i` of `f`.
fn c_len_of(f: &ExportedFn, i: usize) -> String {
    f.params
        .iter()
        .find(|p| kind(p) == CKind::Len(i))
        .map_or_else(
            || "0".to_string(),
            |p| format!("({0} > 0 ? (size_t){0} : 0)", p.name),
        )
}

/// A C shadow file: each function calls the original (compiled renamed) and the Rust port.
/// The port gets copies of what the function writes, so the caller sees the original's writes
/// once, and both are compared.
pub fn shadow_c(m: &Manifest) -> String {
    let mut out = String::from(
        "/* Generated by rust-anything: runs the C original and the Rust port side by side.\n\
         *\n * Build: compile the original sources with `-D<name>=<name>__original` for each function\n\
         * below (see `shadow/flags.txt`), then link this file, those objects and the Rust library.\n\
         * Build the Rust library with `cargo build --release --features shadow`, so it exports only the\n\
         * `rust_anything_` names. Callers get the original's results; differences are counted and reported\n\
         * on stderr.\n */\n\
         #include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include <sys/types.h>\n\n\
         long rust_anything_shadow_calls = 0;\nlong rust_anything_shadow_mismatches = 0;\n\n",
    );
    let defs = c_struct_defs(m);
    if !defs.is_empty() {
        let _ = writeln!(out, "{defs}");
    }
    for f in &m.exports {
        let prototype = c_prototype(f, &f.name);
        let _ = writeln!(
            out,
            "{};\n{};\n",
            c_prototype(f, &format!("{}__original", f.name)),
            c_prototype(f, &format!("rust_anything_{}", f.name))
        );
        let mut before = String::new();
        let mut after = String::new();
        let mut rust_args = Vec::new();
        for (i, p) in f.params.iter().enumerate() {
            match kind(p) {
                CKind::Buffer => {
                    let len = c_len_of(f, i);
                    let _ = writeln!(
                        before,
                        "    size_t len_{n} = {len};\n    unsigned char *copy_{n} = len_{n} ? malloc(len_{n}) : NULL;\n    \
                         if (copy_{n}) memcpy(copy_{n}, {n}, len_{n});",
                        n = p.name
                    );
                    let _ = writeln!(
                        after,
                        "    if (copy_{n} && memcmp(copy_{n}, {n}, len_{n}) != 0) differ = 1;\n    free(copy_{n});",
                        n = p.name
                    );
                    rust_args.push(format!("({})copy_{}", native(p), p.name));
                }
                CKind::Out => {
                    let ty = pointee(native(p));
                    let _ = writeln!(before, "    {ty} copy_{n} = {n} ? *{n} : 0;", n = p.name);
                    let _ = writeln!(
                        after,
                        "    if ({n} && memcmp(&copy_{n}, {n}, sizeof copy_{n}) != 0) differ = 1;",
                        n = p.name
                    );
                    rust_args.push(format!("{n} ? &copy_{n} : NULL", n = p.name));
                }
                CKind::OutPtr(t) => {
                    let target = &f.params[t];
                    let base = if kind(target) == CKind::Buffer {
                        format!("copy_{}", target.name)
                    } else {
                        target.name.clone()
                    };
                    let _ = writeln!(
                        before,
                        "    {} = {n} ? *{n} : NULL;",
                        c_decl(&inner(native(p)), &format!("copy_{}", p.name)),
                        n = p.name
                    );
                    let _ = writeln!(
                        after,
                        "    if ({n} && ((*{n} == NULL) != (copy_{n} == NULL) || (*{n} && (const char *)*{n} - (const char *){t} != (const char *)copy_{n} - (const char *){base}))) differ = 1;",
                        n = p.name,
                        t = target.name
                    );
                    rust_args.push(format!("{n} ? &copy_{n} : NULL", n = p.name));
                }
                _ => rust_args.push(p.name.clone()),
            }
        }
        let args = f
            .params
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let rust_args = rust_args.join(", ");
        let void = f.native_return.as_deref() == Some("void");
        let ret = f.native_return.as_deref().unwrap_or("int");
        let compare_ret = match returns_into(f) {
            _ if void => String::new(),
            Some(i) => {
                let target = &f.params[i];
                let base = if kind(target) == CKind::Buffer {
                    format!("(const char *)copy_{}", target.name)
                } else {
                    format!("(const char *){}", target.name)
                };
                format!(
                    "    if ((expected == NULL) != (actual == NULL) || (expected && (const char *)expected - (const char *){} != (const char *)actual - {base})) differ = 1;\n",
                    target.name
                )
            }
            None => match c_struct(&f.returns) {
                Some(s) => s
                    .fields
                    .iter()
                    .map(|(_, n)| {
                        format!(
                            "    if (memcmp(&expected.{n}, &actual.{n}, sizeof expected.{n}) != 0) differ = 1;\n"
                        )
                    })
                    .collect(),
                None => {
                    "    if (memcmp(&expected, &actual, sizeof expected) != 0) differ = 1;\n".into()
                }
            },
        };
        let (call_original, call_rust, give) = if void {
            (
                format!("    {}__original({args});\n", f.name),
                format!("    rust_anything_{}({rust_args});\n", f.name),
                String::new(),
            )
        } else {
            (
                format!(
                    "    {} = {}__original({args});\n",
                    c_decl(ret, "expected"),
                    f.name
                ),
                format!(
                    "    {} = rust_anything_{}({rust_args});\n",
                    c_decl(ret, "actual"),
                    f.name
                ),
                "    return expected;\n".to_string(),
            )
        };
        let _ = writeln!(
            out,
            "{prototype} {{\n    int differ = 0;\n{before}{call_original}{call_rust}{compare_ret}{after}\
             \x20   rust_anything_shadow_calls++;\n    if (differ) {{\n\
             \x20       rust_anything_shadow_mismatches++;\n        if (rust_anything_shadow_mismatches <= 10)\n\
             \x20           fprintf(stderr, \"[rust-anything] mismatch in {name}\\n\");\n    }}\n{give}}}\n",
            name = f.name
        );
    }
    out
}

/// `-D` flags that rename the originals for a shadow build.
pub fn shadow_flags(m: &Manifest) -> String {
    m.exports
        .iter()
        .map(|f| format!("-D{0}={0}__original", f.name))
        .collect::<Vec<_>>()
        .join(" ")
        + "\n"
}

pub fn cargo_toml(m: &Manifest) -> String {
    format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2024\"\nrust-version = \"{MIN_RUST}\"\npublish = false\n\
         description = \"Rust port of `{}`, generated by rust-anything\"\n\n\
         [lib]\ncrate-type = [\"cdylib\", \"staticlib\", \"rlib\"]\n\n\
         [features]\n# Export only the `rust_anything_` names, to link beside the original C code.\nshadow = []\n\n[dependencies]\n\n\
         [profile.release]\nlto = true\n\n[lints.clippy]\nall = {{ level = \"warn\", priority = -1 }}\n\n[lints.rust]\nunused_lifetimes = \"warn\"\ntrivial_numeric_casts = \"warn\"\n",
        m.crate_name, m.unit
    )
}
