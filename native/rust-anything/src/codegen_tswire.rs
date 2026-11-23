//! The binary wire for TypeScript/JavaScript: generated JS encoders write arguments into a
//! buffer registered once with the Rust module, the Rust side reads them in place, and the
//! result comes back the same way. No JSON text, no per-property napi calls.
//!
//! When an argument does not have the declared shape at run time, that call falls back to the
//! JSON bridge, so behavior on unexpected inputs stays exactly as before.

use std::fmt::Write as _;

use crate::bridge::Manifest;
use crate::codegen::{PortApi, PortFn, Wire, wire};
use crate::ts::exports::{ExportedFn, LocalType};
use crate::ts::shape::{self, Shape};
use crate::wire::{Ty, rust_read_args, rust_write, schema_expr};

/// The wire type of a TS shape. Shapes the wire does not model travel as tagged values.
fn shape_ty(s: &Shape, types: &[LocalType], depth: usize) -> Ty {
    if depth > 6 {
        return Ty::Dyn;
    }
    let str_like = |s: &Shape| matches!(s, Shape::Str | Shape::LitStr(_));
    let num_like = |s: &Shape| matches!(s, Shape::Num | Shape::Int | Shape::LitNum(_));
    let bool_like = |s: &Shape| matches!(s, Shape::Bool | Shape::LitBool(_));
    match s {
        Shape::Str | Shape::LitStr(_) => Ty::Str16,
        Shape::Num | Shape::Int | Shape::LitNum(_) => Ty::Num,
        Shape::Bool | Shape::LitBool(_) => Ty::Bool,
        Shape::Null | Shape::Undefined | Shape::Any | Shape::Tuple(_) => Ty::Dyn,
        Shape::Array(item) => Ty::List {
            item: Box::new(shape_ty(item, types, depth + 1)),
            native: "ts".into(),
            array: false,
        },
        Shape::Object(props, None) if props.len() <= usize::from(u8::MAX) => Ty::Obj(
            props
                .iter()
                .map(|(name, optional, s)| {
                    let t = shape_ty(s, types, depth + 1);
                    let t = match t {
                        Ty::OptJs {
                            inner, nullable, ..
                        } if *optional => Ty::OptJs {
                            inner,
                            nullable,
                            undefinable: true,
                        },
                        t if *optional && t != Ty::Dyn => Ty::OptJs {
                            inner: Box::new(t),
                            nullable: false,
                            undefinable: true,
                        },
                        t => t,
                    };
                    (name.clone(), t)
                })
                .collect(),
        ),
        Shape::Object(props, Some(value)) if props.is_empty() => {
            Ty::Map16(Box::new(shape_ty(value, types, depth + 1)))
        }
        Shape::Object(..) => Ty::Dyn,
        Shape::Union(members) => {
            let nullable = members.iter().any(|m| matches!(m, Shape::Null));
            let undefinable = members.iter().any(|m| matches!(m, Shape::Undefined));
            let rest: Vec<&Shape> = members
                .iter()
                .filter(|m| !matches!(m, Shape::Null | Shape::Undefined))
                .collect();
            let base = if rest.is_empty() {
                Ty::Dyn
            } else if rest.iter().all(|m| str_like(m)) {
                Ty::Str16
            } else if rest.iter().all(|m| num_like(m)) {
                Ty::Num
            } else if rest.iter().all(|m| bool_like(m)) {
                Ty::Bool
            } else if rest.len() == 1 {
                shape_ty(rest[0], types, depth + 1)
            } else {
                Ty::Dyn
            };
            if (nullable || undefinable) && base != Ty::Dyn {
                Ty::OptJs {
                    inner: Box::new(base),
                    nullable,
                    undefinable,
                }
            } else {
                base
            }
        }
        Shape::Ref(name, args) => {
            let resolved = shape::resolve(name, args, types);
            if matches!(resolved, Shape::Ref(..)) {
                Ty::Dyn
            } else {
                shape_ty(&resolved, types, depth + 1)
            }
        }
    }
}

fn js_key(k: &str) -> String {
    serde_json::to_string(k).unwrap_or_else(|_| "\"\"".into())
}

/// JS statements writing `x` as `t`; `return false` when `x` does not have the shape.
fn js_enc(x: &str, t: &Ty, d: usize) -> String {
    match t {
        Ty::Num => format!("if (typeof {x} !== 'number') return false\nwF64({x})\n"),
        Ty::Bool => format!("if (typeof {x} !== 'boolean') return false\nwU8({x} ? 1 : 0)\n"),
        Ty::Str16 => format!("if (typeof {x} !== 'string') return false\nwStr({x})\n"),
        Ty::Dyn => format!("if (!wDyn({x})) return false\n"),
        Ty::OptJs {
            inner,
            nullable,
            undefinable,
        } => format!(
            "if ({x} === undefined) {{\n{}wU8(0)\n}} else if ({x} === null) {{\n{}wU8(2)\n}} else {{\nwU8(1)\n{}}}\n",
            if *undefinable { "" } else { "return false\n" },
            if *nullable { "" } else { "return false\n" },
            js_enc(x, inner, d)
        ),
        Ty::List { item, .. } => format!(
            "if (!Array.isArray({x})) return false\nwU8(1)\nwU32({x}.length)\nfor (let i{d} = 0; i{d} < {x}.length; i{d}++) {{\nconst v{d} = {x}[i{d}]\n{}}}\n",
            js_enc(&format!("v{d}"), item, d + 1)
        ),
        Ty::Map16(value) => format!(
            "if ({x} === null || typeof {x} !== 'object' || Array.isArray({x})) return false\n{{\nconst k{d} = Object.keys({x})\nwU8(1)\nwU32(k{d}.length)\nfor (const key{d} of k{d}) {{\nwStr(key{d})\nconst v{d} = {x}[key{d}]\n{}}}\n}}\n",
            js_enc(&format!("v{d}"), value, d + 1)
        ),
        Ty::Obj(fields) => {
            // Keys in the object's own order (a port may pass the object through); a key the
            // type does not list sends the call through JSON.
            let required = fields
                .iter()
                .filter(|(_, ft)| {
                    !(matches!(
                        ft,
                        Ty::OptJs {
                            undefinable: true,
                            ..
                        }
                    ) || *ft == Ty::Dyn)
                })
                .count();
            let mut cases = String::new();
            for (i, (key, ft)) in fields.iter().enumerate() {
                let optional = matches!(
                    ft,
                    Ty::OptJs {
                        undefinable: true,
                        ..
                    }
                ) || *ft == Ty::Dyn;
                let _ = write!(
                    cases,
                    "case {}: {{\nwU8({i})\n{}const f{d} = {x}[k{d}]\n{}break\n}}\n",
                    js_key(key),
                    if optional {
                        String::new()
                    } else {
                        format!("r{d}++\n")
                    },
                    js_enc(&format!("f{d}"), ft, d + 1)
                );
            }
            format!(
                "if ({x} === null || typeof {x} !== 'object' || Array.isArray({x})) return false\n{{\n\
                 const at{d} = at\nwU8(0)\nlet n{d} = 0, r{d} = 0\nfor (const k{d} in {x}) {{\nswitch (k{d}) {{\n{cases}default: return false\n}}\n\
                 if (++n{d} > 255) return false\n}}\nif (r{d} !== {required}) return false\niv.setUint8(at{d}, n{d})\n}}\n"
            )
        }
        _ => "return false\n".into(),
    }
}

/// JS statements decoding a value of `t` into the variable `target`.
fn js_dec(target: &str, t: &Ty, d: usize) -> String {
    match t {
        Ty::Num => format!("{target} = rF64()\n"),
        Ty::Bool => format!("{target} = rU8() === 1\n"),
        Ty::Str16 => format!("{target} = rStr()\n"),
        Ty::Dyn => format!("{target} = rDyn()\n"),
        Ty::OptJs { inner, .. } => format!(
            "{{\nconst g{d} = rU8()\nif (g{d} === 0) {target} = undefined\nelse if (g{d} === 2) {target} = null\nelse {{\n{}}}\n}}\n",
            js_dec(target, inner, d + 1)
        ),
        Ty::List { item, .. } => format!(
            "if (rU8() === 0) {target} = null\nelse {{\nconst n{d} = rU32()\nconst a{d} = new Array(n{d})\nfor (let i{d} = 0; i{d} < n{d}; i{d}++) {{\nlet v{d}\n{}a{d}[i{d}] = v{d}\n}}\n{target} = a{d}\n}}\n",
            js_dec(&format!("v{d}"), item, d + 1)
        ),
        Ty::Map16(value) => format!(
            "if (rU8() === 0) {target} = null\nelse {{\nconst n{d} = rU32()\nconst o{d} = {{}}\nfor (let i{d} = 0; i{d} < n{d}; i{d}++) {{\nconst k{d} = rStr()\nlet v{d}\n{}o{d}[k{d}] = v{d}\n}}\n{target} = o{d}\n}}\n",
            js_dec(&format!("v{d}"), value, d + 1)
        ),
        Ty::Obj(fields) => {
            let mut cases = String::new();
            for (i, (key, ft)) in fields.iter().enumerate() {
                let _ = write!(
                    cases,
                    "case {i}: {{\nlet v{d}\n{}o{d}[{}] = v{d}\nbreak\n}}\n",
                    js_dec(&format!("v{d}"), ft, d + 1),
                    js_key(key)
                );
            }
            format!(
                "{{\nconst o{d} = {{}}\nconst n{d} = rU8()\nfor (let j{d} = 0; j{d} < n{d}; j{d}++) {{\nswitch (rU8()) {{\n{cases}default: {{\nconst k{d} = rStr()\no{d}[k{d}] = rDyn()\n}}\n}}\n}}\n{target} = o{d}\n}}\n"
            )
        }
        _ => format!("{target} = undefined\n"),
    }
}

/// The JS runtime: registered buffers, writers, readers, tagged values.
const WIRE_NAPI_CJS: &str = r#"
// The binary wire (see rust-anything's `wire.rs`): arguments are written into `wIn`, results read
// from `wOut`; both are registered with the native module once.
let wIn = new Uint8Array(1 << 16), wOut = new Uint8Array(1 << 16)
// Its own ArrayBuffer: Bun (JavaScriptCore) may relocate a small typed array whose `.buffer`
// nobody read, and Rust then reads a length of 0 and reports `wire data ended early`.
const wLen = new Uint32Array(new ArrayBuffer(4))
let iv = new DataView(wIn.buffer), ov = new DataView(wOut.buffer), at = 0, p = 0
native.rustAnythingWireInit(wIn, wOut, wLen)
const regrow = () => native.rustAnythingWireInit(wIn, wOut, wLen)
// Runs a wire function; a result larger than `wOut` is fetched once `wOut` has grown.
const wireCall = fn => {
  wLen[0] = at
  const n = fn()
  if (Number.isNaN(n)) return false
  if (n === Infinity) {
    p = 0
    throw new Error(rStr())
  }
  if (n < 0) {
    wOut = new Uint8Array(Math.max(wOut.length * 2, -n))
    ov = new DataView(wOut.buffer)
    native.rustAnythingWireInit(wIn, wOut, wLen)
    native.rustAnythingWireFlush()
  }
  p = 0
  return true
}
"#;

const WIRE_WASM_CJS: &str = r#"
// The binary wire (see rust-anything's `wire.rs`): arguments are written into `wIn` and copied into
// wasm memory in one step; results are read where the Rust side left them.
let wIn = new Uint8Array(1 << 16), iv = new DataView(wIn.buffer), at = 0, p = 0
const mem = native.rustAnythingWireMemory()
let wOut = new Uint8Array(mem.buffer), ov = new DataView(mem.buffer)
const regrow = () => {}
// Growing wasm memory replaces its buffer: views are made again when that happened.
const views = () => {
  if (wOut.buffer !== mem.buffer) {
    wOut = new Uint8Array(mem.buffer)
    ov = new DataView(mem.buffer)
  }
}
const wireCall = fn => {
  const ptr = native.rustAnythingWireIn(at)
  views()
  wOut.set(wIn.subarray(0, at), ptr)
  if (fn(at) === 0xFFFFFFFF) return false
  views()
  p = native.rustAnythingWireOut()
  return true
}
"#;

const WIRE_CJS: &str = r#"
const need = n => {
  if (at + n <= wIn.length) return
  const grown = new Uint8Array(Math.max(wIn.length * 2, at + n))
  grown.set(wIn)
  wIn = grown
  iv = new DataView(wIn.buffer)
  regrow()
}
const wU8 = v => { need(1); iv.setUint8(at++, v) }
const wU32 = v => { need(4); iv.setUint32(at, v, true); at += 4 }
const wF64 = v => { need(8); iv.setFloat64(at, v, true); at += 8 }
const wUnits = s => {
  need(4 + 2 * s.length)
  iv.setUint32(at, s.length, true)
  at += 4
  for (let i = 0; i < s.length; i++) { iv.setUint16(at, s.charCodeAt(i), true); at += 2 }
}
const wStr = s => { wU8(4); wUnits(s) }
const wDyn = v => {
  switch (typeof v) {
    case 'undefined': wU8(0); return true
    case 'boolean': wU8(v ? 3 : 2); return true
    case 'number': wU8(4); wF64(v); return true
    case 'string': wU8(5); wUnits(v); return true
    case 'object':
      if (v === null) { wU8(1); return true }
      if (Array.isArray(v)) {
        wU8(6); wU32(v.length)
        for (let i = 0; i < v.length; i++) if (!wDyn(v[i])) return false
        return true
      }
      if (Object.getPrototypeOf(v) !== Object.prototype && Object.getPrototypeOf(v) !== null) return false
      {
        const keys = Object.keys(v)
        wU8(7); wU32(keys.length)
        for (const k of keys) { wUnits(k); if (!wDyn(v[k])) return false }
        return true
      }
  }
  return false
}
const rU8 = () => ov.getUint8(p++)
const rU32 = () => { const v = ov.getUint32(p, true); p += 4; return v }
const rF64 = () => { const v = ov.getFloat64(p, true); p += 8; return v }
// ignoreBOM: a leading U+FEFF is part of the string, not a byte-order mark.
const utf16 = new TextDecoder('utf-16le', { ignoreBOM: true })
const rUnits = (wellFormed) => {
  const n = rU32()
  let s
  if (n > 24 && wellFormed) s = utf16.decode(wOut.subarray(p, p + 2 * n))
  else {
    s = ''
    for (let i = 0; i < n; i++) s += String.fromCharCode(ov.getUint16(p + 2 * i, true))
  }
  p += 2 * n
  return s
}
// Tag 1: a string from Rust (well-formed); 4: raw code units, lone surrogates included.
const rStr = () => rUnits(rU8() === 1)
const rDyn = () => {
  switch (rU8()) {
    case 0: return undefined
    case 1: return null
    case 2: return false
    case 3: return true
    case 4: return rF64()
    case 5: return rUnits(false)
    case 6: { const n = rU32(); const a = new Array(n); for (let i = 0; i < n; i++) a[i] = rDyn(); return a }
    case 7: { const n = rU32(); const o = {}; for (let i = 0; i < n; i++) { const k = rUnits(false); o[k] = rDyn() } return o }
  }
}
"#;

/// The napi side: buffer registration and the call helper.
const WIRE_NAPI_RS: &str = r#"
// ---- the binary wire ------------------------------------------------------------------------

type WireBuffers = (
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint32Array,
);

thread_local! {
    static WIRE: std::cell::RefCell<Option<WireBuffers>> = const { std::cell::RefCell::new(None) };
    static PENDING: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
    static SCRATCH: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Registers the argument, result and argument-length buffers `index.cjs` uses.
#[napi(js_name = "rustAnythingWireInit")]
pub fn rust_anything_wire_init(
    input: napi::bindgen_prelude::Uint8Array,
    output: napi::bindgen_prelude::Uint8Array,
    len: napi::bindgen_prelude::Uint32Array,
) {
    WIRE.with(|w| *w.borrow_mut() = Some((input, output, len)));
}

/// Copies a result that did not fit into the (now larger) result buffer.
#[napi(js_name = "rustAnythingWireFlush")]
pub fn rust_anything_wire_flush() -> f64 {
    let bytes = PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()));
    WIRE.with(|w| {
        let mut w = w.borrow_mut();
        let Some((_, output, _)) = w.as_mut() else {
            return -1.0;
        };
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = bytes.len().min(out.len());
        out[..n].copy_from_slice(&bytes[..n]);
        n as f64
    })
}

/// Reads the argument bytes (`wLen[0]` of them), runs `f`, and writes its result; a negative
/// return is the size of a result that did not fit, kept for `rustAnythingWireFlush`.
fn wire_io(f: impl FnOnce(&mut wire::Reader, &mut wire::Writer) -> Result<(), String>) -> napi::Result<f64> {
    WIRE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let Some((input, output, len)) = guard.as_mut() else {
            return Err(napi::Error::from_reason("rust-anything: the wire buffers are not registered"));
        };
        let len = (len.first().copied().unwrap_or(0) as usize).min(input.len());
        let mut w = wire::Writer(SCRATCH.with(|s| std::mem::take(&mut *s.borrow_mut())));
        w.0.clear();
        let result = run(|| f(&mut wire::Reader::new(&input[..len]), &mut w));
        let keep = |w: wire::Writer| SCRATCH.with(|s| *s.borrow_mut() = w.0);
        match result {
            // A result of another shape than the TS type: JS runs the call through JSON.
            Err(e) if e.reason.starts_with(wire::MISMATCH) => {
                keep(w);
                return Ok(f64::NAN);
            }
            Err(e) => {
                keep(w);
                let mut m = wire::Writer::default();
                m.string16(&e.reason);
                // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
                let out = unsafe { output.as_mut() };
                if m.0.len() > out.len() {
                    return Err(e);
                }
                out[..m.0.len()].copy_from_slice(&m.0);
                return Ok(f64::INFINITY);
            }
            Ok(()) => {}
        }
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = w.0.len();
        if n > out.len() {
            PENDING.with(|p| *p.borrow_mut() = w.0);
            return Ok(-(n as f64));
        }
        out[..n].copy_from_slice(&w.0);
        keep(w);
        Ok(n as f64)
    })
}
"#;

/// The wasm side: buffers in wasm memory, and the call helper.
const WIRE_WASM_RS: &str = r#"
// ---- the binary wire ------------------------------------------------------------------------

thread_local! {
    static WIRE_IN: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
    static WIRE_OUT: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// This module's memory, where `index.cjs` reads and writes the wire buffers.
#[wasm_bindgen(js_name = "rustAnythingWireMemory")]
pub fn rust_anything_wire_memory() -> JsValue {
    wasm_bindgen::memory()
}

/// The address of an argument buffer of `len` bytes.
#[wasm_bindgen(js_name = "rustAnythingWireIn")]
pub fn rust_anything_wire_in(len: u32) -> u32 {
    WIRE_IN.with(|b| {
        let mut b = b.borrow_mut();
        b.resize(len as usize, 0);
        b.as_mut_ptr() as usize as u32
    })
}

/// The address of the last result.
#[wasm_bindgen(js_name = "rustAnythingWireOut")]
pub fn rust_anything_wire_out() -> u32 {
    WIRE_OUT.with(|b| b.borrow().as_ptr() as usize as u32)
}

/// Reads `len` argument bytes, runs `f`, and keeps its result for `rustAnythingWireOut`.
fn wire_io(len: u32, f: impl FnOnce(&mut wire::Reader, &mut wire::Writer) -> Result<(), String>) -> Result<u32, JsError> {
    let input = WIRE_IN.with(|b| std::mem::take(&mut *b.borrow_mut()));
    let mut w = wire::Writer::default();
    let result = f(&mut wire::Reader::new(&input[..(len as usize).min(input.len())]), &mut w);
    WIRE_IN.with(|b| *b.borrow_mut() = input);
    match result {
        // A result of another shape than the TS type: JS runs the call through JSON.
        Err(e) if e.starts_with(wire::MISMATCH) => return Ok(u32::MAX),
        other => other.map_err(|e| JsError::new(&e))?,
    }
    let n = w.0.len();
    WIRE_OUT.with(|b| *b.borrow_mut() = w.0);
    Ok(u32::try_from(n).unwrap_or(u32::MAX))
}
"#;

/// One wire function: its export, parameter and result types, and the generated code.
struct WireFn<'m> {
    export: &'m ExportedFn,
    port: &'m PortFn,
    params: Vec<Ty>,
    result: Option<Ty>,
    rust: String,
}

fn types_for(m: &Manifest, file: &str) -> Vec<LocalType> {
    let text = std::fs::read_to_string(m.root.join(file)).unwrap_or_default();
    crate::bridge::types_in_scope(&m.root, file, &text)
}

/// The functions that cross the wire: synchronous, with a structured parameter or result, and
/// port types that fit the wire types.
fn wire_fns<'m>(m: &'m Manifest, api: &'m PortApi) -> Vec<WireFn<'m>> {
    let mut out = Vec::new();
    let mut scopes: std::collections::HashMap<&str, Vec<LocalType>> =
        std::collections::HashMap::new();
    for f in &api.functions {
        if f.is_async {
            continue;
        }
        let structured =
            f.params.iter().any(|(_, t)| wire(t) == Wire::Json) || wire(&f.ret) == Wire::Json;
        let napi = m.kind == crate::bridge::Kind::Napi;
        if !structured && !napi {
            continue;
        }
        let Some(e) = m.exports.iter().find(|e| e.name == f.js_name) else {
            continue;
        };
        if e.params.len() != f.params.len() || e.params.iter().any(|p| p.rest) {
            continue;
        }
        let types = scopes
            .entry(e.file.as_str())
            .or_insert_with(|| types_for(m, &e.file));
        let params: Vec<Ty> = e
            .params
            .iter()
            .map(|p| {
                let t = shape_ty(&shape::parse(&p.ts_type), types, 0);
                if p.optional || p.default.is_some() {
                    match t {
                        Ty::OptJs {
                            inner, nullable, ..
                        } => Ty::OptJs {
                            inner,
                            nullable,
                            undefinable: true,
                        },
                        Ty::Dyn => Ty::Dyn,
                        t => Ty::OptJs {
                            inner: Box::new(t),
                            nullable: false,
                            undefinable: true,
                        },
                    }
                } else {
                    t
                }
            })
            .collect();
        let params: Vec<Ty> = params
            .into_iter()
            .zip(&f.params)
            .map(|(t, (_, rust))| {
                let rust: String = rust.chars().filter(|c| !c.is_whitespace()).collect();
                let value = rust
                    .strip_prefix("Option<")
                    .and_then(|r| r.strip_suffix('>'))
                    .unwrap_or(&rust);
                if matches!(value, "Value" | "serde_json::Value") {
                    Ty::Dyn
                } else {
                    t
                }
            })
            .collect();
        let result = (f.ret != "()").then(|| shape_ty(&shape::parse(&e.returns), types, 0));
        let named: Vec<(String, Ty)> = f
            .params
            .iter()
            .map(|(n, _)| n.clone())
            .zip(params.iter().cloned())
            .collect();
        let mut statics = String::new();
        let prefix = format!("S_{}", crate::bridge::snake_case(&f.js_name).to_uppercase());
        let Some(reads) = rust_read_args(&named, &f.params, &prefix, &mut statics) else {
            continue;
        };
        let args: Vec<&str> = f.params.iter().map(|(n, _)| n.as_str()).collect();
        let call = format!("port::{}({})", f.rust_name, args.join(", "));
        let call = if f.fallible { format!("{call}?") } else { call };
        let write = match &result {
            None => format!("        let () = {call};\n"),
            Some(t) => {
                let schema = format!("{prefix}_OUT");
                let Some(w) = rust_write(t, &f.ret, "out", &schema, 0) else {
                    continue;
                };
                if w.contains(&schema) {
                    let _ = writeln!(
                        statics,
                        "static {schema}: wire::Schema = {};",
                        schema_expr(t)
                    );
                }
                format!("        let out = &{call};\n        {w}\n")
            }
        };
        let (attr, ret, len, len_arg) = if napi {
            ("napi", "napi::Result<f64>", "", "")
        } else {
            ("wasm_bindgen", "Result<u32, JsError>", "len: u32", "len, ")
        };
        let r = if reads.contains("r.") || reads.contains("(r") {
            "r"
        } else {
            "_r"
        };
        let w = if write.contains("w.") || write.contains("(w") {
            "w"
        } else {
            "_w"
        };
        let rust = format!(
            "\n{statics}\n/// `{js}` over the binary wire.\n#[{attr}(js_name = \"{js}Wire\")]\npub fn {name}_wire({len}) -> {ret} {{\n    \
             wire_io({len_arg}|{r}, {w}| {{\n{reads}{write}        Ok(())\n    }})\n}}\n",
            js = f.js_name,
            name = f.rust_name.trim_start_matches("r#"),
        );
        out.push(WireFn {
            export: e,
            port: f,
            params,
            result,
            rust,
        });
    }
    out
}

/// Rust code for `lib.rs` (napi): the wire runtime and one function per wire export; empty
/// when no function uses the wire.
pub fn napi_rs(m: &Manifest, api: &PortApi) -> String {
    let fns = wire_fns(m, api);
    if fns.is_empty() {
        return String::new();
    }
    let mut out = String::from(if m.kind == crate::bridge::Kind::Wasm {
        WIRE_WASM_RS
    } else {
        WIRE_NAPI_RS
    });
    for f in fns {
        out.push_str(&f.rust);
    }
    out
}

/// The functions `index.cjs` sends over the wire: their wrapper also refuses lone surrogates and
/// keeps JS error classes, in place of the guard layer.
pub fn wired(m: &Manifest, api: &PortApi) -> Vec<String> {
    if !matches!(
        m.kind,
        crate::bridge::Kind::Napi | crate::bridge::Kind::Wasm
    ) {
        return Vec::new();
    }
    wire_fns(m, api)
        .iter()
        .map(|f| f.port.js_name.clone())
        .collect()
}

/// Whether the crate has wire functions (and needs `src/wire.rs`).
pub fn uses_wire(m: &Manifest, api: &PortApi) -> bool {
    matches!(
        m.kind,
        crate::bridge::Kind::Napi | crate::bridge::Kind::Wasm
    ) && !wire_fns(m, api).is_empty()
}

/// JS for `index.cjs`, after the exports are set: the runtime, and each wire function in front
/// of its JSON version.
pub fn index_cjs(m: &Manifest, api: &PortApi) -> String {
    let fns = wire_fns(m, api);
    if fns.is_empty() {
        return String::new();
    }
    let mut out = String::from(if m.kind == crate::bridge::Kind::Wasm {
        WIRE_WASM_CJS
    } else {
        WIRE_NAPI_CJS
    });
    out.push_str(WIRE_CJS);
    for f in &fns {
        let n = f.params.len();
        let names: Vec<String> = (0..n).map(|i| format!("a{i}")).collect();
        let mut enc = String::new();
        for (i, t) in f.params.iter().enumerate() {
            enc.push_str(&js_enc(&format!("a{i}"), t, 0));
        }
        let dec = f
            .result
            .as_ref()
            .map_or_else(String::new, |t| js_dec("t", t, 0));
        let js = &f.port.js_name;
        let checks: String = crate::codegen::string_params(f.port)
            .iter()
            .map(|i| {
                format!(
                    "  if (illFormed(a{i})) throw new TypeError('argument {} of {js} has a lone surrogate, which a Rust String cannot hold')\n",
                    i + 1
                )
            })
            .collect();
        let _ = write!(
            out,
            "\n// `{js}`: binary wire; the JSON version handles inputs of other shapes.\n{{\n\
             const json = module.exports.{js}\nconst call = native.{js}Wire\n\
             const encode = ({args}) => {{\nat = 0\n{enc}return true\n}}\n\
             const decode = () => {{\nlet t\n{dec}return t\n}}\n\
             module.exports.{js} = function {js} ({args}) {{\n{checks}  try {{\n    \
             if (arguments.length <= {n} && encode({args}) && wireCall(call)) return decode()\n    \
             return json(...arguments)\n  }} catch (e) {{\n    throw jsError(e)\n  }}\n}}\n}}\n",
            args = names.join(", "),
        );
        let _ = f.export;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_argument_length_has_an_array_buffer_of_its_own() {
        // `wIn` and `wOut` are registered after their `.buffer` is read for the DataViews above;
        // `wLen` never was, and Bun (JavaScriptCore) may then move the small typed array, so Rust
        // read a length of 0 and every call failed with `wire data ended early`.
        assert!(
            WIRE_NAPI_CJS.contains("const wLen = new Uint32Array(new ArrayBuffer(4))"),
            "the loader must give `wLen` its own ArrayBuffer"
        );
        assert!(
            !WIRE_NAPI_CJS.contains("new Uint32Array(1)"),
            "a bare `new Uint32Array(1)` is the shape Bun relocates"
        );
    }

    #[test]
    fn wide_objects_use_dynamic_keys_instead_of_truncating_field_indexes() {
        let shape = |n| {
            Shape::Object(
                (0..n)
                    .map(|i| (format!("p{i}"), true, Shape::Num))
                    .collect(),
                None,
            )
        };
        assert!(matches!(shape_ty(&shape(255), &[], 0), Ty::Obj(fields) if fields.len() == 255));
        assert_eq!(shape_ty(&shape(256), &[], 0), Ty::Dyn);
        assert_eq!(shape_ty(&shape(257), &[], 0), Ty::Dyn);
    }

    #[test]
    fn maps_ts_shapes() {
        let none: Vec<LocalType> = Vec::new();
        let t = shape_ty(
            &shape::parse("{ start: number; end: number; label?: string | null }"),
            &none,
            0,
        );
        let Ty::Obj(fields) = &t else { panic!("{t:?}") };
        assert_eq!(fields[0], ("start".into(), Ty::Num));
        assert_eq!(
            fields[2].1,
            Ty::OptJs {
                inner: Box::new(Ty::Str16),
                nullable: true,
                undefinable: true
            }
        );
        assert_eq!(shape_ty(&shape::parse("'a' | 'b'"), &none, 0), Ty::Str16);
        assert_eq!(shape_ty(&shape::parse("unknown"), &none, 0), Ty::Dyn);
        assert!(matches!(
            shape_ty(&shape::parse("Record<string, number>"), &none, 0),
            Ty::Map16(_)
        ));
    }
}
