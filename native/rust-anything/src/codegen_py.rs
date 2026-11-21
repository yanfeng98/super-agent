//! PyO3 bindings and the Python wrapper package for a bridge crate.
//!
//! Like the JS side, the binding layer follows `port.rs`: strings, ints, floats, bools and
//! lists or options of those cross natively; anything structured crosses as JSON text, which
//! the wrapper package (`python/<lib>/__init__.py`) encodes and decodes with Python markers.

use std::fmt::Write as _;

use crate::bridge::Manifest;
use crate::codegen::{Modules, PortApi, PortFn, Wire, wire};
use crate::ts::exports::Param;

/// A Python signature for `#[pyo3(signature = ...)]`: optional parameters default to `None`,
/// keyword-only ones follow `*`, the rest parameter is `*name`.
fn signature(f: &PortFn, source: &[Param]) -> String {
    let mut parts = Vec::new();
    let mut keyword_only = false;
    for (name, _) in &f.params {
        let param = source
            .iter()
            .find(|p| crate::bridge::snake_case(&p.name) == *name);
        if param.is_some_and(|p| p.keyword_only) && !keyword_only {
            keyword_only = true;
            parts.push("*".to_string());
        }
        let py_name = name.trim_start_matches("r#");
        match param {
            Some(p) if p.optional => match p.default.as_deref().and_then(rust_literal) {
                Some(literal) => parts.push(format!("{py_name}={literal}")),
                None => parts.push(format!("{py_name}=None")),
            },
            _ => parts.push(py_name.to_string()),
        }
    }
    parts.join(", ")
}

/// A Python literal default as a Rust expression, for `#[pyo3(signature = ...)]`: strings,
/// numbers and booleans. `None` and other expressions return `None` (the parameter is an
/// `Option`, and `None` stands for the default).
pub fn rust_literal(default: &str) -> Option<String> {
    let d = default.trim();
    match d {
        // Python's `True`; Ruby's and Java's `true`.
        "True" | "true" => return Some("true".into()),
        "False" | "false" => return Some("false".into()),
        "None" => return None,
        _ => {}
    }
    let digits = d.replace('_', "");
    if digits.parse::<i64>().is_ok() {
        return Some(digits);
    }
    if (digits.contains('.') || digits.contains('e')) && digits.parse::<f64>().is_ok() {
        return Some(if digits.contains('.') {
            digits
        } else {
            format!("{digits}_f64")
        });
    }
    let quote = d.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let inner = d.strip_prefix(quote)?.strip_suffix(quote)?;
    // Plain literals only: escapes and quotes inside would need Python's own parser.
    if inner.contains('\\') || inner.contains('"') || inner.contains('\'') {
        return None;
    }
    Some(format!("String::from(\"{inner}\")"))
}

/// Whether a parameter keeps Python's literal default (and is a plain, non-`Option` type).
pub fn has_literal_default(p: &Param) -> bool {
    p.optional && p.default.as_deref().and_then(rust_literal).is_some()
}

fn params_of<'m>(m: &'m Manifest, f: &PortFn, class: Option<&str>) -> &'m [Param] {
    match class {
        Some(c) => m
            .classes
            .iter()
            .find(|k| k.name == c)
            .map(|k| {
                if f.rust_name == "new" {
                    k.constructor.as_slice()
                } else {
                    k.methods
                        .iter()
                        .find(|x| x.name == f.js_name)
                        .map_or(&[][..], |x| x.params.as_slice())
                }
            })
            .unwrap_or(&[]),
        None => m
            .exports
            .iter()
            .find(|e| e.name == f.js_name)
            .map_or(&[][..], |e| e.params.as_slice()),
    }
}

/// The inner type of `Option<...>`, if that is what the port declares.
fn option_inner(ty: &str) -> Option<&str> {
    ty.strip_prefix("Option<").and_then(|t| t.strip_suffix('>'))
}

/// Whether the binding for a port parameter is `Option<Bound>`: an optional parameter whose
/// default is `None` and whose value crosses as a Python object. PyO3 needs `Option<...>` to
/// accept `None`; a plain `Bound` rejects it and the generated crate does not compile
/// (`E0308: expected Bound<'_, PyAny>, found Option<_>`).
fn is_option_bound(ty: &str) -> bool {
    option_inner(ty).is_some_and(|inner| wire(inner) == Wire::Json)
}

fn binding_type(ty: &str) -> String {
    if is_option_bound(ty) {
        return "Option<Bound<'_, PyAny>>".to_string();
    }
    match wire(ty) {
        // Structured values arrive as Python objects and are converted in Rust: no JSON text.
        Wire::Json => "Bound<'_, PyAny>".to_string(),
        _ => ty.to_string(),
    }
}

/// Whether `FromPy`/`ToPy` in `pyconv.rs` cover a type: then it converts directly, without
/// building a `serde_json::Value` first.
fn direct(ty: &str) -> bool {
    const KNOWN: &[&str] = &[
        "Vec",
        "Option",
        "HashMap",
        "std",
        "collections",
        "String",
        "f64",
        "i64",
        "bool",
        "serde_json",
        "Value",
        "Num",
        "crate",
        "pynum",
    ];
    ty.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .all(|w| KNOWN.contains(&w))
}

fn body(f: &PortFn, target: &str, constructor: bool) -> String {
    let mut body = String::new();
    for (n, t) in &f.params {
        if wire(t) == Wire::Json {
            if is_option_bound(t) {
                // The binding is `Option<Bound>`: `None` stays `None`, a value converts.
                if direct(t) {
                    let _ = writeln!(
                        body,
                        "    let {n}: {t} = match &{n} {{\n        None => None,\n        Some(v) => Some(crate::pyconv::FromPy::from_py(v)?),\n    }};"
                    );
                } else {
                    let _ = writeln!(
                        body,
                        "    let {n} = match &{n} {{\n        None => None,\n        Some(v) => Some(crate::pyconv::arg(v)?),\n    }};"
                    );
                }
            } else if direct(t) {
                let _ = writeln!(
                    body,
                    "    let {n}: {t} = crate::pyconv::FromPy::from_py(&{n})?;"
                );
            } else {
                let _ = writeln!(body, "    let {n} = crate::pyconv::arg(&{n})?;");
            }
        }
    }
    let args: Vec<&str> = f.params.iter().map(|(n, _)| n.as_str()).collect();
    let call = format!(
        "{target}({}){}",
        args.join(", "),
        if f.fallible { "?" } else { "" }
    );
    if !constructor && f.ret.trim() == "()" {
        let _ = writeln!(body, "    {call};\n    Ok(())");
        return body;
    }
    let _ = writeln!(body, "    let out = {call};");
    let _ = writeln!(
        body,
        "    {}",
        if !constructor && wire(&f.ret) == Wire::Json && !direct(&f.ret) {
            "serde_json::to_value(&out).map_err(|e| e.to_string())"
        } else {
            "Ok(out)"
        }
    );
    body
}

/// The `_inner` function's success type: structured results as a `serde_json::Value`.
fn ret(f: &PortFn) -> String {
    match wire(&f.ret) {
        Wire::Json if direct(&f.ret) => f.ret.clone(),
        Wire::Json => "serde_json::Value".into(),
        _ => f.ret.clone(),
    }
}

/// The `#[pyfunction]`'s success type: structured results as a Python object.
fn py_ret(f: &PortFn) -> String {
    match wire(&f.ret) {
        Wire::Json => "Py<PyAny>".into(),
        _ => f.ret.clone(),
    }
}

/// How the `#[pyfunction]` hands back the `_inner` result.
fn py_out(f: &PortFn) -> &'static str {
    match wire(&f.ret) {
        Wire::Json => ".map_err(error).and_then(|v| crate::pyconv::ToPy::to_py(&v, py))",
        _ => ".map_err(error)",
    }
}

/// A port's `Err("KeyError: missing")` raises that built-in exception type with the message;
/// without a known type prefix, `ValueError`. Panics become `PanicException`.
const PY_ERROR: &str = r#"
fn error(message: String) -> PyErr {
    use pyo3::exceptions as e;
    let (kind, text) = message.split_once(": ").unwrap_or(("", message.as_str()));
    let text = text.to_string();
    match kind {
        "TypeError" => e::PyTypeError::new_err(text),
        "KeyError" => e::PyKeyError::new_err(text),
        "IndexError" => e::PyIndexError::new_err(text),
        "OverflowError" => e::PyOverflowError::new_err(text),
        "ZeroDivisionError" => e::PyZeroDivisionError::new_err(text),
        "AttributeError" => e::PyAttributeError::new_err(text),
        "RuntimeError" => e::PyRuntimeError::new_err(text),
        "NotImplementedError" => e::PyNotImplementedError::new_err(text),
        "LookupError" => e::PyLookupError::new_err(text),
        "ArithmeticError" => e::PyArithmeticError::new_err(text),
        "UnicodeError" => e::PyUnicodeError::new_err(text),
        "ValueError" => e::PyValueError::new_err(text),
        _ => e::PyValueError::new_err(message),
    }
}
"#;

pub fn lib_rs(m: &Manifest, api: &PortApi, modules: &Modules) -> String {
    let mut out = format!(
        "//! Python bindings for `{}`: generated from the signatures in `port.rs` by rust-anything.\n\
         //! Regenerate with `rust-anything refresh` after changing them.\n\n\
         use pyo3::prelude::*;\n\n",
        m.unit
    );
    if modules.js {
        out.push_str("pub mod js;\n");
    }
    out.push_str("pub mod port;\n");
    if api.needs_json() {
        out.push_str(
            "pub mod pyconv;\npub mod pynum;\n\n#[allow(unused_imports)]\nuse pynum::Num;\n",
        );
    }
    if modules.utf16 {
        out.push_str("pub mod utf16;\n");
    }
    out.push_str(PY_ERROR);
    let mut registrations = Vec::new();
    for f in &api.functions {
        let params: Vec<String> = f
            .params
            .iter()
            .map(|(n, t)| format!("{n}: {}", binding_type(t)))
            .collect();
        let inner = format!("{}_inner", f.rust_name.trim_start_matches("r#"));
        let args: Vec<&str> = f.params.iter().map(|(n, _)| n.as_str()).collect();
        let _ = writeln!(
            out,
            "\nfn {inner}({}) -> Result<{}, String> {{\n{}}}\n",
            params.join(", "),
            ret(f),
            body(f, &format!("port::{}", f.rust_name), false)
        );
        let _ = writeln!(out, "/// `{}`.", f.js_name);
        let _ = writeln!(out, "#[pyfunction]");
        let _ = writeln!(
            out,
            "#[pyo3(name = \"{}\", signature = ({}))]",
            f.js_name,
            signature(f, params_of(m, f, None))
        );
        let py_param = if wire(&f.ret) == Wire::Json {
            "py: Python<'_>, "
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "fn {}({py_param}{}) -> PyResult<{}> {{",
            f.rust_name,
            params.join(", "),
            py_ret(f)
        );
        let _ = writeln!(out, "    {inner}({}){}\n}}", args.join(", "), py_out(f));
        registrations.push(format!(
            "    m.add_function(wrap_pyfunction!({}, m)?)?;",
            f.rust_name
        ));
    }
    for c in &api.classes {
        let (name, js) = (&c.rust_name, &c.js_name);
        let snake = crate::bridge::snake_case(name);
        let ctor_params: Vec<String> = c
            .constructor
            .params
            .iter()
            .map(|(n, t)| format!("{n}: {}", binding_type(t)))
            .collect();
        let ctor_args: Vec<&str> = c
            .constructor
            .params
            .iter()
            .map(|(n, _)| n.as_str())
            .collect();
        let _ = writeln!(
            out,
            "\nfn {snake}_new_inner({}) -> Result<port::{name}, String> {{\n{}}}",
            ctor_params.join(", "),
            body(&c.constructor, &format!("port::{name}::new"), true)
        );
        for (method, mutable) in &c.methods {
            let receiver = if *mutable {
                format!("inner: &mut port::{name}")
            } else {
                format!("inner: &port::{name}")
            };
            let params: Vec<String> = std::iter::once(receiver)
                .chain(
                    method
                        .params
                        .iter()
                        .map(|(n, t)| format!("{n}: {}", binding_type(t))),
                )
                .collect();
            let _ = writeln!(
                out,
                "\nfn {snake}_{}_inner({}) -> Result<{}, String> {{\n{}}}",
                method.rust_name.trim_start_matches("r#"),
                params.join(", "),
                ret(method),
                body(method, &format!("inner.{}", method.rust_name), false)
            );
        }
        let _ = writeln!(
            out,
            "\n/// Class `{js}`.\n#[pyclass(name = \"{js}\")]\npub struct {name} {{\n    inner: port::{name},\n}}\n"
        );
        let _ = writeln!(out, "#[pymethods]\nimpl {name} {{");
        let _ = writeln!(
            out,
            "    #[new]\n    #[pyo3(signature = ({}))]\n    fn new({}) -> PyResult<Self> {{\n        \
             {snake}_new_inner({}).map(|inner| Self {{ inner }}).map_err(error)\n    }}",
            signature(&c.constructor, params_of(m, &c.constructor, Some(js))),
            ctor_params.join(", "),
            ctor_args.join(", ")
        );
        for (method, mutable) in &c.methods {
            let receiver = if *mutable { "&mut self" } else { "&self" };
            let params: Vec<String> = method
                .params
                .iter()
                .map(|(n, t)| format!("{n}: {}", binding_type(t)))
                .collect();
            let sig = std::iter::once(receiver.to_string())
                .chain(params)
                .collect::<Vec<_>>()
                .join(", ");
            let borrow = if *mutable {
                "&mut self.inner"
            } else {
                "&self.inner"
            };
            let args = std::iter::once(borrow)
                .chain(method.params.iter().map(|(n, _)| n.as_str()))
                .collect::<Vec<_>>();
            let sig = if wire(&method.ret) == Wire::Json {
                sig.replacen("self", "self, py: Python<'_>", 1)
            } else {
                sig
            };
            let _ = writeln!(
                out,
                "\n    #[pyo3(name = \"{}\", signature = ({}))]\n    fn {}({sig}) -> PyResult<{}> {{\n        \
                 {snake}_{}_inner({}){}\n    }}",
                method.js_name,
                signature(method, params_of(m, method, Some(js))),
                method.rust_name,
                py_ret(method),
                method.rust_name.trim_start_matches("r#"),
                args.join(", "),
                py_out(method)
            );
        }
        out.push_str("}\n");
        registrations.push(format!("    m.add_class::<{name}>()?;"));
    }
    if !m.batches.is_empty() {
        match crate::codegen_batch::py_rs(m, api) {
            Ok((code, classes)) => {
                out.push_str(&code);
                for c in classes {
                    registrations.push(format!("    m.add_class::<{c}>()?;"));
                }
            }
            Err(e) => {
                let _ = writeln!(out, "\ncompile_error!({e:?});");
            }
        }
    }
    let _ = writeln!(
        out,
        "\n#[pymodule]\n#[pyo3(name = \"_native\")]\nfn native_module(m: &Bound<'_, PyModule>) -> PyResult<()> {{\n{}\n    Ok(())\n}}",
        registrations.join("\n")
    );
    out
}

/// The wrapper package: imports `_native`, converting structured values.
pub fn package_init(m: &Manifest, api: &PortApi) -> String {
    // Structured values are converted in Rust (`pyconv.rs`): every export is the native one.
    let structured = |_: &PortFn| -> (Vec<String>, bool) { (Vec::new(), false) };
    let mut out = format!(
        "\"\"\"Rust port of `{}`, generated by rust-anything. Import this package where the code\n\
         imported the original module.\"\"\"\n\nfrom . import _native\n{CODEC_PY}\n",
        m.unit
    );
    let mut names = Vec::new();
    for f in &api.functions {
        let (json_params, json_ret) = structured(f);
        let positions: Vec<String> = f
            .params
            .iter()
            .enumerate()
            .filter(|(_, (_, t))| wire(t) == Wire::Json)
            .map(|(i, _)| i.to_string())
            .collect();
        if json_params.is_empty() && !json_ret {
            let _ = writeln!(out, "{} = _native.{}", f.js_name, f.js_name);
        } else {
            let _ = writeln!(
                out,
                "{name} = _wrap(_native.{name}, ({positions}), ({names_list}), {json_ret})",
                name = f.js_name,
                positions = positions
                    .iter()
                    .map(|p| format!("{p},"))
                    .collect::<String>(),
                names_list = json_params
                    .iter()
                    .map(|n| format!("'{n}',"))
                    .collect::<String>(),
                json_ret = if json_ret { "True" } else { "False" }
            );
        }
        names.push(f.js_name.clone());
    }
    for c in &api.classes {
        let needs = std::iter::once(&c.constructor)
            .chain(c.methods.iter().map(|(x, _)| x))
            .any(|f| {
                let (p, r) = structured(f);
                !p.is_empty() || r
            });
        if !needs {
            let _ = writeln!(out, "{} = _native.{}", c.js_name, c.js_name);
        } else {
            let methods: String = c
                .methods
                .iter()
                .map(|(method, _)| {
                    let (params, json_ret) = structured(method);
                    let positions: String = method
                        .params
                        .iter()
                        .enumerate()
                        .filter(|(_, (_, t))| wire(t) == Wire::Json)
                        .map(|(i, _)| format!("{i},"))
                        .collect();
                    format!(
                        "    {name} = _method('{name}', ({positions}), ({params}), {ret})\n",
                        name = method.js_name,
                        params = params.iter().map(|n| format!("'{n}',")).collect::<String>(),
                        ret = if json_ret { "True" } else { "False" }
                    )
                })
                .collect();
            let (ctor_params, _) = structured(&c.constructor);
            let ctor_positions: String = c
                .constructor
                .params
                .iter()
                .enumerate()
                .filter(|(_, (_, t))| wire(t) == Wire::Json)
                .map(|(i, _)| format!("{i},"))
                .collect();
            let _ = writeln!(
                out,
                "\nclass {js}:\n    def __init__(self, *args, **kwargs):\n        \
                 args, kwargs = _encode_args(args, kwargs, ({ctor_positions}), ({}))\n        \
                 self._inner = _native.{js}(*args, **kwargs)\n{methods}",
                ctor_params
                    .iter()
                    .map(|n| format!("'{n}',"))
                    .collect::<String>(),
                js = c.js_name
            );
        }
        names.push(c.js_name.clone());
    }
    for b in &m.batches {
        let class = format!("{}Batch", crate::codegen_batch::pascal(&b.function));
        let _ = writeln!(out, "{class} = _native.{class}");
        names.push(class);
    }
    let _ = writeln!(
        out,
        "\n__all__ = [{}]",
        names
            .iter()
            .map(|n| format!("'{n}'"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    out
}

const CODEC_PY: &str = r#"
import json as _json
import math as _math

# Structured values cross as JSON text. JSON has no NaN, tuples, sets, bytes, big ints or
# non-string keys: those travel as markers (see `js.rs`) and come back as Python values.
_BIG = 2 ** 53


def _enc(v):
    if v is None or isinstance(v, (bool, str)):
        return v
    if isinstance(v, int):
        return v if -_BIG <= v <= _BIG else {"$int": str(v)}
    if isinstance(v, float):
        if _math.isnan(v):
            return {"$num": "NaN"}
        if _math.isinf(v):
            return {"$num": "Infinity" if v > 0 else "-Infinity"}
        if v == 0 and _math.copysign(1, v) < 0:
            return {"$num": "-0"}
        return v
    if isinstance(v, list):
        return [_enc(x) for x in v]
    if isinstance(v, tuple):
        return {"$tuple": [_enc(x) for x in v]}
    if isinstance(v, (set, frozenset)):
        return {"$set": sorted((_enc(x) for x in v), key=_json.dumps)}
    if isinstance(v, (bytes, bytearray)):
        return {"$bytes": list(v)}
    if isinstance(v, dict):
        if all(isinstance(k, str) for k in v):
            return {k: _enc(x) for k, x in v.items()}
        return {"$dict": [[_enc(k), _enc(x)] for k, x in v.items()]}
    return {"$repr": repr(v)}


def _dec(v):
    if isinstance(v, list):
        return [_dec(x) for x in v]
    if not isinstance(v, dict):
        return v
    if len(v) == 1:
        (key, inner), = v.items()
        if key == "$undefined":
            return None
        if key == "$num":
            return {"NaN": _math.nan, "Infinity": _math.inf, "-Infinity": -_math.inf, "-0": -0.0}[inner]
        if key == "$int":
            return int(inner)
        if key == "$tuple":
            return tuple(_dec(x) for x in inner)
        if key == "$set":
            return {_dec(x) for x in inner}
        if key == "$bytes":
            return bytes(inner)
        if key == "$dict":
            return {_dec(k): _dec(x) for k, x in inner}
    return {k: _dec(x) for k, x in v.items()}


def _encode_args(args, kwargs, positions, names):
    args = tuple(_json.dumps(_enc(a)) if i in positions else a for i, a in enumerate(args))
    kwargs = {k: (_json.dumps(_enc(a)) if k in names else a) for k, a in kwargs.items()}
    return args, kwargs


def _wrap(fn, positions, names, json_result):
    def call(*args, **kwargs):
        args, kwargs = _encode_args(args, kwargs, positions, names)
        out = fn(*args, **kwargs)
        return _dec(_json.loads(out)) if json_result else out
    call.__name__ = fn.__name__
    call.__doc__ = fn.__doc__
    return call


def _method(name, positions, names, json_result):
    def call(self, *args, **kwargs):
        args, kwargs = _encode_args(args, kwargs, positions, names)
        out = getattr(self._inner, name)(*args, **kwargs)
        return _dec(_json.loads(out)) if json_result else out
    call.__name__ = name
    return call
"#;

/// `src/pyconv.rs`: Python objects to `serde_json::Value` and back, in Rust, without JSON text.
/// Values the JSON model lacks use the same markers as recorded cases: `$tuple`, `$set`,
/// `$bytes`, `$int` (beyond i64), `$dict` (non-string keys), `$num` (NaN, infinities, -0).
pub const PYCONV_RS: &str = r#"//! Python objects to `serde_json::Value` and back, in Rust. Generated by rust-anything.

#![forbid(unsafe_code)]

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyFloat, PyFrozenSet, PyInt, PyList, PySet, PyString, PyTuple};
use serde_json::{Map, Value, json};

fn type_error(obj: &Bound<'_, PyAny>) -> String {
    let name = obj.get_type().name().map(|n| n.to_string()).unwrap_or_default();
    format!("TypeError: unsupported argument type {name}")
}

fn float(x: f64) -> Value {
    if x.is_nan() {
        json!({"$num": "NaN"})
    } else if x.is_infinite() {
        json!({"$num": if x > 0.0 { "Infinity" } else { "-Infinity" }})
    } else if x == 0.0 && x.is_sign_negative() {
        json!({"$num": "-0"})
    } else {
        json!(x)
    }
}

fn items(obj: &Bound<'_, PyAny>) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for item in obj.try_iter().map_err(|e| e.to_string())? {
        out.push(from_py(&item.map_err(|e| e.to_string())?)?);
    }
    Ok(out)
}

/// A Python object as a JSON value.
pub fn from_py(obj: &Bound<'_, PyAny>) -> Result<Value, String> {
    if obj.is_none() {
        return Ok(Value::Null);
    }
    if obj.is_instance_of::<PyBool>() {
        return Ok(Value::Bool(obj.extract::<bool>().map_err(|e| e.to_string())?));
    }
    if obj.is_instance_of::<PyInt>() {
        return Ok(match obj.extract::<i64>() {
            Ok(i) => json!(i),
            Err(_) => json!({"$int": obj.str().map_err(|e| e.to_string())?.to_string()}),
        });
    }
    if obj.is_instance_of::<PyFloat>() {
        return Ok(float(obj.extract::<f64>().map_err(|e| e.to_string())?));
    }
    if obj.is_instance_of::<PyString>() {
        return Ok(Value::String(obj.extract::<String>().map_err(|e| e.to_string())?));
    }
    if obj.is_instance_of::<PyList>() {
        return Ok(Value::Array(items(obj)?));
    }
    if obj.is_instance_of::<PyTuple>() {
        return Ok(json!({"$tuple": items(obj)?}));
    }
    if obj.is_instance_of::<PyDict>() {
        let dict = obj.cast::<PyDict>().map_err(|e| e.to_string())?;
        let mut map = Map::new();
        let mut pairs = Vec::new();
        let mut string_keys = true;
        for (k, v) in dict.iter() {
            let value = from_py(&v)?;
            if let Ok(key) = k.extract::<String>() {
                map.insert(key.clone(), value.clone());
                pairs.push(json!([key, value]));
            } else {
                string_keys = false;
                pairs.push(json!([from_py(&k)?, value]));
            }
        }
        return Ok(if string_keys { Value::Object(map) } else { json!({"$dict": pairs}) });
    }
    if obj.is_instance_of::<PySet>() || obj.is_instance_of::<PyFrozenSet>() {
        return Ok(json!({"$set": items(obj)?}));
    }
    if obj.is_instance_of::<PyBytes>() {
        return Ok(json!({"$bytes": obj.extract::<Vec<u8>>().map_err(|e| e.to_string())?}));
    }
    Err(type_error(obj))
}

/// A structured argument, converted to the port's parameter type.
pub fn arg<T: serde::de::DeserializeOwned>(obj: &Bound<'_, PyAny>) -> Result<T, String> {
    serde_json::from_value(from_py(obj)?).map_err(|e| format!("TypeError: {e}"))
}

/// A Python argument as a Rust value, without going through JSON.
pub trait FromPy: Sized {
    fn from_py(obj: &Bound<'_, PyAny>) -> Result<Self, String>;
}

/// A Rust result as a Python object, without going through JSON.
pub trait ToPy {
    fn to_py(&self, py: Python<'_>) -> PyResult<Py<PyAny>>;
}

macro_rules! extract {
    ($($t:ty),*) => {$(
        impl FromPy for $t {
            fn from_py(obj: &Bound<'_, PyAny>) -> Result<Self, String> {
                obj.extract::<$t>().map_err(|_| type_error(obj))
            }
        }
        impl ToPy for $t {
            fn to_py(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
                Ok(self.clone().into_pyobject(py)?.to_owned().into_any().unbind())
            }
        }
    )*};
}
extract!(i64, bool, String);

impl FromPy for f64 {
    fn from_py(obj: &Bound<'_, PyAny>) -> Result<Self, String> {
        if obj.is_instance_of::<PyFloat>() || obj.is_instance_of::<PyInt>() {
            obj.extract::<f64>().map_err(|_| type_error(obj))
        } else {
            Err(type_error(obj))
        }
    }
}

impl ToPy for f64 {
    fn to_py(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(PyFloat::new(py, *self).into_any().unbind())
    }
}

impl FromPy for crate::pynum::Num {
    fn from_py(obj: &Bound<'_, PyAny>) -> Result<Self, String> {
        use crate::pynum::Num;
        if obj.is_instance_of::<PyFloat>() {
            return obj.extract::<f64>().map(Num::Float).map_err(|_| type_error(obj));
        }
        if obj.is_instance_of::<PyInt>() {
            return obj
                .extract::<i64>()
                .map(Num::Int)
                .map_err(|_| "OverflowError: int too large for the port".to_string());
        }
        Err(type_error(obj))
    }
}

impl ToPy for crate::pynum::Num {
    fn to_py(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match *self {
            crate::pynum::Num::Int(i) => i.to_py(py),
            crate::pynum::Num::Float(x) => x.to_py(py),
        }
    }
}

impl FromPy for Value {
    fn from_py(obj: &Bound<'_, PyAny>) -> Result<Self, String> {
        from_py(obj)
    }
}

impl ToPy for Value {
    fn to_py(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_py(py, self)
    }
}

impl<T: FromPy> FromPy for Option<T> {
    fn from_py(obj: &Bound<'_, PyAny>) -> Result<Self, String> {
        if obj.is_none() { Ok(None) } else { T::from_py(obj).map(Some) }
    }
}

impl<T: ToPy> ToPy for Option<T> {
    fn to_py(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self {
            Some(v) => v.to_py(py),
            None => Ok(py.None()),
        }
    }
}

impl<T: FromPy> FromPy for Vec<T> {
    fn from_py(obj: &Bound<'_, PyAny>) -> Result<Self, String> {
        if !obj.is_instance_of::<PyList>() && !obj.is_instance_of::<PyTuple>() {
            return Err(type_error(obj));
        }
        let mut out = Vec::new();
        for item in obj.try_iter().map_err(|e| e.to_string())? {
            out.push(T::from_py(&item.map_err(|e| e.to_string())?)?);
        }
        Ok(out)
    }
}

impl<T: ToPy> ToPy for Vec<T> {
    fn to_py(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let items: PyResult<Vec<Py<PyAny>>> = self.iter().map(|x| x.to_py(py)).collect();
        Ok(PyList::new(py, items?)?.into_any().unbind())
    }
}

impl<T: FromPy> FromPy for std::collections::HashMap<String, T> {
    fn from_py(obj: &Bound<'_, PyAny>) -> Result<Self, String> {
        let dict = obj.cast::<PyDict>().map_err(|_| type_error(obj))?;
        let mut out = Self::new();
        for (k, v) in dict.iter() {
            out.insert(k.extract::<String>().map_err(|_| type_error(&k))?, T::from_py(&v)?);
        }
        Ok(out)
    }
}

impl<T: ToPy> ToPy for std::collections::HashMap<String, T> {
    fn to_py(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (k, v) in self {
            dict.set_item(k, v.to_py(py)?)?;
        }
        Ok(dict.into_any().unbind())
    }
}

macro_rules! tuples {
    ($(($($t:ident $i:tt),+)),*) => {$(
        impl<$($t: FromPy),+> FromPy for ($($t,)+) {
            fn from_py(obj: &Bound<'_, PyAny>) -> Result<Self, String> {
                let t = obj.cast::<PyTuple>().map_err(|_| type_error(obj))?;
                Ok(($($t::from_py(&t.get_item($i).map_err(|e| e.to_string())?)?,)+))
            }
        }
        impl<$($t: ToPy),+> ToPy for ($($t,)+) {
            fn to_py(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
                Ok(PyTuple::new(py, [$(self.$i.to_py(py)?),+])?.into_any().unbind())
            }
        }
    )*};
}
tuples!((A 0, B 1), (A 0, B 1, C 2), (A 0, B 1, C 2, D 3));

/// A JSON value as a Python object; markers become the Python types they stand for.
pub fn to_py(py: Python<'_>, v: &Value) -> PyResult<Py<PyAny>> {
    let list = |items: &[Value]| -> PyResult<Vec<Py<PyAny>>> { items.iter().map(|x| to_py(py, x)).collect() };
    Ok(match v {
        Value::Null => py.None(),
        Value::Bool(b) => PyBool::new(py, *b).to_owned().into_any().unbind(),
        Value::Number(n) => match n.as_i64() {
            Some(i) if !n.is_f64() => i.into_pyobject(py)?.into_any().unbind(),
            _ => match n.as_u64() {
                Some(u) if !n.is_f64() => u.into_pyobject(py)?.into_any().unbind(),
                _ => PyFloat::new(py, n.as_f64().unwrap_or(f64::NAN)).into_any().unbind(),
            },
        },
        Value::String(s) => PyString::new(py, s).into_any().unbind(),
        Value::Array(items) => PyList::new(py, list(items)?)?.into_any().unbind(),
        Value::Object(o) if o.len() == 1 => {
            let (key, inner) = o.iter().next().expect("one entry");
            let items = || inner.as_array().map(Vec::as_slice).unwrap_or_default();
            match key.as_str() {
                "$tuple" => PyTuple::new(py, list(items())?)?.into_any().unbind(),
                "$set" => PySet::new(py, list(items())?)?.into_any().unbind(),
                "$bytes" => {
                    let bytes: Vec<u8> = items().iter().filter_map(|b| b.as_u64()).map(|b| b as u8).collect();
                    PyBytes::new(py, &bytes).into_any().unbind()
                }
                "$int" => py
                    .import("builtins")?
                    .getattr("int")?
                    .call1((inner.as_str().unwrap_or("0"),))?
                    .unbind(),
                "$float" => PyFloat::new(py, inner.as_str().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN)).into_any().unbind(),
                "$num" => PyFloat::new(
                    py,
                    match inner.as_str() {
                        Some("Infinity") => f64::INFINITY,
                        Some("-Infinity") => f64::NEG_INFINITY,
                        Some("-0") => -0.0,
                        _ => f64::NAN,
                    },
                )
                .into_any()
                .unbind(),
                "$dict" => {
                    let dict = PyDict::new(py);
                    for pair in items() {
                        if let Some([k, v]) = pair.as_array().map(Vec::as_slice) {
                            dict.set_item(to_py(py, k)?, to_py(py, v)?)?;
                        }
                    }
                    dict.into_any().unbind()
                }
                _ => {
                    let dict = PyDict::new(py);
                    dict.set_item(key, to_py(py, inner)?)?;
                    dict.into_any().unbind()
                }
            }
        }
        Value::Object(o) => {
            let dict = PyDict::new(py);
            for (k, x) in o {
                dict.set_item(k, to_py(py, x)?)?;
            }
            dict.into_any().unbind()
        }
    })
}
"#;

/// `src/pynum.rs`: Python's int-or-float, for ports of code that takes either.
pub const PYNUM_RS: &str = r#"//! Python numbers: an int stays an int through `+`, `-`, `*`, `//` and `%`; `/` gives a float.
//! Generated by rust-anything. Ints beyond i64 become floats (Python would keep a big int).

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug)]
pub enum Num {
    Int(i64),
    Float(f64),
}

impl Num {
    pub fn f(self) -> f64 {
        match self {
            Self::Int(i) => i as f64,
            Self::Float(x) => x,
        }
    }

    fn op(self, o: Self, int: fn(i64, i64) -> Option<i64>, float: fn(f64, f64) -> f64) -> Self {
        match (self, o) {
            (Self::Int(a), Self::Int(b)) => int(a, b).map_or_else(|| Self::Float(float(a as f64, b as f64)), Self::Int),
            (a, b) => Self::Float(float(a.f(), b.f())),
        }
    }

    /// `/`: always a float; `ZeroDivisionError` for zero.
    pub fn true_div(self, o: Self) -> Result<Self, String> {
        match (self, o) {
            (_, o) if o.f() == 0.0 => Err(match (self, o) {
                (Self::Int(_), Self::Int(_)) => "ZeroDivisionError: division by zero".into(),
                _ => "ZeroDivisionError: float division by zero".into(),
            }),
            (a, b) => Ok(Self::Float(a.f() / b.f())),
        }
    }

    /// `%`: the result takes the divisor's sign.
    pub fn modulo(self, o: Self) -> Result<Self, String> {
        match (self, o) {
            (Self::Int(_), Self::Int(0)) => Err("ZeroDivisionError: integer modulo by zero".into()),
            (Self::Int(a), Self::Int(b)) => {
                let r = a.wrapping_rem(b);
                Ok(Self::Int(if r != 0 && (r < 0) != (b < 0) { r + b } else { r }))
            }
            (a, b) => {
                let (x, y) = (a.f(), b.f());
                if y == 0.0 {
                    return Err("ZeroDivisionError: float modulo".into());
                }
                let m = x % y;
                Ok(Self::Float(if m != 0.0 {
                    if (y < 0.0) != (m < 0.0) { m + y } else { m }
                } else {
                    0.0_f64.copysign(y)
                }))
            }
        }
    }

    /// Python's `max(a, b, ...)`: the first value no later value is greater than.
    pub fn max(values: &[Self]) -> Self {
        values[1..].iter().fold(values[0], |m, &x| if x > m { x } else { m })
    }

    /// Python's `min(a, b, ...)`.
    pub fn min(values: &[Self]) -> Self {
        values[1..].iter().fold(values[0], |m, &x| if x < m { x } else { m })
    }
}

impl From<f64> for Num {
    fn from(x: f64) -> Self {
        Self::Float(x)
    }
}

impl From<i64> for Num {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl Add for Num {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        self.op(o, i64::checked_add, |a, b| a + b)
    }
}

impl Sub for Num {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        self.op(o, i64::checked_sub, |a, b| a - b)
    }
}

impl Mul for Num {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        self.op(o, i64::checked_mul, |a, b| a * b)
    }
}

impl Neg for Num {
    type Output = Self;
    fn neg(self) -> Self {
        match self {
            Self::Int(i) => i.checked_neg().map_or(Self::Float(-(i as f64)), Self::Int),
            Self::Float(x) => Self::Float(-x),
        }
    }
}

impl PartialEq for Num {
    fn eq(&self, o: &Self) -> bool {
        match (*self, *o) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (a, b) => a.f() == b.f(),
        }
    }
}

impl PartialOrd for Num {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        match (*self, *o) {
            (Self::Int(a), Self::Int(b)) => a.partial_cmp(&b),
            (a, b) => a.f().partial_cmp(&b.f()),
        }
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::{Kind, Manifest};
    use crate::codegen::{Modules, PortApi, PortFn};
    use crate::lang::Lang;
    use crate::ts::exports::{ExportedFn, Param};

    #[test]
    fn converts_python_literals() {
        assert_eq!(rust_literal("'-'").as_deref(), Some("String::from(\"-\")"));
        assert_eq!(rust_literal("8").as_deref(), Some("8"));
        assert_eq!(rust_literal("1_000").as_deref(), Some("1000"));
        assert_eq!(rust_literal("0.5").as_deref(), Some("0.5"));
        assert_eq!(rust_literal("True").as_deref(), Some("true"));
        assert_eq!(rust_literal("None"), None);
        assert_eq!(rust_literal("[]"), None);
        assert_eq!(rust_literal("'a\\nb'"), None);
    }

    #[test]
    fn optional_object_params_bind_as_option_bound() {
        // `thresholds=None` must bind as `Option<Bound>` so PyO3 accepts `None`;
        // required structured parameters stay a plain `Bound`, native options are untouched.
        assert_eq!(
            binding_type("Option<serde_json::Value>").as_str(),
            "Option<Bound<'_, PyAny>>"
        );
        assert_eq!(
            binding_type("serde_json::Value").as_str(),
            "Bound<'_, PyAny>"
        );
        assert_eq!(binding_type("Option<i64>").as_str(), "Option<i64>");
        assert_eq!(binding_type("String").as_str(), "String");
    }

    fn verdict_port() -> PortFn {
        PortFn {
            js_name: "verdict".into(),
            rust_name: "verdict".into(),
            params: vec![
                ("readings".into(), "serde_json::Value".into()),
                ("thresholds".into(), "Option<serde_json::Value>".into()),
            ],
            ret: "serde_json::Value".into(),
            fallible: true,
            is_async: false,
        }
    }

    fn verdict_manifest() -> Manifest {
        let param = |name: &str, ts_type: &str, optional: bool, default: Option<&str>| Param {
            name: name.into(),
            ts_type: ts_type.into(),
            optional,
            default: default.map(str::to_string),
            ..Default::default()
        };
        Manifest {
            schema: "1".into(),
            root: std::path::PathBuf::from("/tmp"),
            root_written: None,
            unit: "hpdet/".into(),
            language: Lang::Python,
            kind: Kind::Pyo3,
            crate_name: "hpdet-rs".into(),
            exports: vec![ExportedFn {
                name: "verdict".into(),
                file: "hpdet/verdict.py".into(),
                line: 1,
                params: vec![
                    param("readings", "Record<string, number>", false, None),
                    param("thresholds", "Thresholds | undefined", true, Some("None")),
                ],
                returns: "Record<string, number>".into(),
                ..Default::default()
            }],
            classes: Vec::new(),
            js_only: Vec::new(),
            batches: Vec::new(),
            allocator: Default::default(),
            float_tolerance: None,
        }
    }

    #[test]
    fn optional_object_wrapper_accepts_none() {
        let api = PortApi {
            functions: vec![verdict_port()],
            ..Default::default()
        };
        let modules = Modules {
            utf16: false,
            js: false,
        };
        let lib = lib_rs(&verdict_manifest(), &api, &modules);
        // The signature still defaults to `None`, and the Rust parameter takes `Option`.
        assert!(
            lib.contains("#[pyo3(name = \"verdict\", signature = (readings, thresholds=None))]"),
            "{lib}"
        );
        assert!(
            lib.contains(
                "fn verdict_inner(readings: Bound<'_, PyAny>, \
                 thresholds: Option<Bound<'_, PyAny>>)"
            ),
            "{lib}"
        );
        // `None` converts to `None`; a value converts through the port's type.
        assert!(lib.contains("match &thresholds"), "{lib}");
        assert!(lib.contains("None => None,"), "{lib}");
    }
}
