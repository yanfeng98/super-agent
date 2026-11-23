//! The binary wire for Go and Java: values cross as compact bytes whose encoders and decoders
//! are generated per type on both sides, without JSON text or reflection.
//!
//! Layout, little endian: `I64` and `F64` as 8 bytes, `Bool` as 1 byte. `Str`, `List`, `Map`
//! and `Struct` start with a presence byte (0 for null/nil, 1 otherwise); strings then carry a
//! u32 length and UTF-8 bytes, lists and maps a u32 count and their items (maps as string key,
//! value). `Opt` is a presence byte and the value. `Tuple` (several results) is its items in order.

use std::fmt::Write as _;

/// A source-language type the wire carries. `native` is the type as the source spells it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ty {
    I64(String),
    F64(String),
    Bool,
    Str,
    /// `array` marks a Java array (`int[]`) rather than a list.
    List {
        item: Box<Ty>,
        native: String,
        array: bool,
    },
    Map {
        value: Box<Ty>,
        native: String,
    },
    /// Fields: (source field name, JSON key, type).
    Struct {
        name: String,
        fields: Vec<(String, String, Ty)>,
    },
    Opt {
        inner: Box<Ty>,
        native: String,
    },
    Tuple(Vec<Ty>),
    // JS (TS) values: a JS number; a string as UTF-16 code units (lone surrogates survive);
    // `undefined`/`null`/value; an object whose fields travel by index in the writer's key
    // order (unknown keys by name); a string-keyed record; and any value, tagged by kind.
    Num,
    Str16,
    OptJs {
        inner: Box<Ty>,
        nullable: bool,
        undefinable: bool,
    },
    Obj(Vec<(String, Ty)>),
    Map16(Box<Ty>),
    Dyn,
}

/// The `wire::Schema` constant expression for a type, for `read_value`/`write_value`.
pub fn schema_expr(t: &Ty) -> String {
    match t {
        Ty::I64(_) => "wire::Schema::I64".into(),
        Ty::F64(_) => "wire::Schema::F64".into(),
        Ty::Bool => "wire::Schema::Bool".into(),
        Ty::Str => "wire::Schema::Str".into(),
        Ty::List { item, .. } => format!("wire::Schema::List(&{})", schema_expr(item)),
        Ty::Map { value, .. } => format!("wire::Schema::Map(&{})", schema_expr(value)),
        Ty::Opt { inner, .. } => format!("wire::Schema::Opt(&{})", schema_expr(inner)),
        Ty::Struct { fields, .. } => format!(
            "wire::Schema::Struct(&[{}])",
            fields
                .iter()
                .map(|(_, key, t)| format!("({key:?}, {})", schema_expr(t)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Ty::Tuple(items) => format!(
            "wire::Schema::Tuple(&[{}])",
            items.iter().map(schema_expr).collect::<Vec<_>>().join(", ")
        ),
        Ty::Num => "wire::Schema::Num".into(),
        Ty::Str16 => "wire::Schema::Str16".into(),
        Ty::OptJs { inner, .. } => format!("wire::Schema::OptJs(&{})", schema_expr(inner)),
        Ty::Obj(fields) => format!(
            "wire::Schema::Obj(&[{}])",
            fields
                .iter()
                .map(|(key, t)| format!("({key:?}, {})", schema_expr(t)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Ty::Map16(value) => format!("wire::Schema::Map16(&{})", schema_expr(value)),
        Ty::Dyn => "wire::Schema::Dyn".into(),
    }
}

fn generic_arg<'a>(rust: &'a str, head: &str) -> Option<&'a str> {
    let rust = rust.trim();
    let rest = rust
        .strip_prefix(head)
        .or_else(|| rust.strip_prefix(&format!("std::collections::{head}")))?;
    rest.strip_prefix('<')?.strip_suffix('>').map(str::trim)
}

fn map_value(rust: &str) -> Option<&str> {
    for head in ["HashMap", "BTreeMap", "IndexMap"] {
        if let Some(args) = generic_arg(rust, head) {
            let (key, value) = args.split_once(',')?;
            return (key.trim() == "String").then_some(value.trim());
        }
    }
    None
}

fn is_value(rust: &str) -> bool {
    matches!(rust.trim(), "serde_json::Value" | "Value")
}

/// A closure body: `r.f64()` stays as is, anything else is wrapped in `Ok(...)`.
fn body(read: &str) -> String {
    match read.strip_suffix('?') {
        Some(call) if !call.contains('?') => call.to_string(),
        _ => format!("Ok({read})"),
    }
}

/// A schema as an argument: `&S` for a static, `S.inner()` (already a reference) as is.
fn schema_ref(schema: &str) -> String {
    if schema.ends_with(')') {
        schema.to_string()
    } else {
        format!("&{schema}")
    }
}

/// Rust code reading a value of `t` into the port type `rust` from reader `r`; `None` when the
/// port type does not fit the wire type (the function then keeps the JSON bridge).
pub fn rust_read(t: &Ty, rust: &str, schema: &str) -> Option<String> {
    if is_value(rust) {
        return Some(format!("wire::read_value(r, {})?", schema_ref(schema)));
    }
    let rust = rust.trim();
    if *t == Ty::Dyn
        && matches!(
            rust.split_whitespace().collect::<String>().as_str(),
            "Option<Value>" | "Option<serde_json::Value>"
        )
    {
        return Some("wire::read_dyn_opt(r)?".into());
    }
    if *t == Ty::Dyn {
        // Any JS value into a typed port parameter: through a tagged value and serde.
        return Some(
            "serde_json::from_value(wire::read_dyn(r)?).map_err(|e| format!(\"TypeError: {e}\"))?"
                .into(),
        );
    }
    let structural = rust_read_structural(t, rust, schema);
    if structural.is_none() && rust.contains("Value") {
        return Some(format!(
            "serde_json::from_value(wire::read_value(r, {})?).map_err(|e| format!(\"TypeError: {{e}}\"))?",
            schema_ref(schema)
        ));
    }
    structural
}

fn rust_read_structural(t: &Ty, rust: &str, schema: &str) -> Option<String> {
    Some(match t {
        Ty::I64(_) if rust == "i64" => "r.i64()?".into(),
        Ty::F64(_) if rust == "f64" => "r.f64()?".into(),
        Ty::Bool if rust == "bool" => "r.bool()?".into(),
        Ty::Str if rust == "String" => "r.string()?".into(),
        Ty::Opt { inner, .. } => match generic_arg(rust, "Option") {
            Some(inner_rust) => format!(
                "r.opt(|r| {})?",
                body(&rust_read(inner, inner_rust, &format!("{schema}.inner()"))?)
            ),
            None => format!(
                "r.required(|r| {})?",
                body(&rust_read(inner, rust, &format!("{schema}.inner()"))?)
            ),
        },
        Ty::List { item, .. } => format!(
            "r.list(|r| {})?",
            body(&rust_read(
                item,
                generic_arg(rust, "Vec")?,
                &format!("{schema}.inner()")
            )?)
        ),
        Ty::Map { value, .. } => format!(
            "r.map(|r| {})?.into_iter().collect()",
            body(&rust_read(
                value,
                map_value(rust)?,
                &format!("{schema}.inner()")
            )?)
        ),
        Ty::Num if rust == "f64" => "r.f64()?".into(),
        Ty::Str16 if rust == "String" => "r.string16()?".into(),
        Ty::Str16 if rust == "Vec<u16>" => "r.units()?".into(),
        Ty::OptJs { inner, .. } => match generic_arg(rust, "Option") {
            Some(inner_rust) => format!(
                "r.opt_js(|r| {})?",
                body(&rust_read(inner, inner_rust, &format!("{schema}.inner()"))?)
            ),
            None => format!(
                "r.required_js(|r| {})?",
                body(&rust_read(inner, rust, &format!("{schema}.inner()"))?)
            ),
        },
        Ty::Map16(value) => format!(
            "r.map16(|r| {})?.into_iter().collect()",
            body(&rust_read(
                value,
                map_value(rust)?,
                &format!("{schema}.inner()")
            )?)
        ),
        _ => return None,
    })
}

/// Rust statements writing `expr` (a reference to the port type `rust`) as `t` into writer `w`.
pub fn rust_write(t: &Ty, rust: &str, expr: &str, schema: &str, depth: usize) -> Option<String> {
    if is_value(rust) {
        return Some(format!(
            "wire::write_value(w, {expr}, {})?;",
            schema_ref(schema)
        ));
    }
    if *t == Ty::Dyn {
        // `serde_json::to_value` turns NaN and the infinities into `Value::Null`, so a scalar
        // float whose declaration is only `unknown` goes over the wire as a number: JS then sees
        // NaN and ±Infinity, instead of `null` that `diff` blames on the port.
        let spelled = rust.trim();
        let scalar = spelled.rsplit("::").next().unwrap_or(spelled);
        if matches!(scalar, "f64" | "f32") {
            return Some(format!("w.tag(4); w.f64(f64::from(*{expr}));"));
        }
        return Some(format!(
            "wire::write_dyn(w, &serde_json::to_value({expr}).map_err(|e| e.to_string())?);"
        ));
    }
    if rust.contains("Value") {
        if let Some(code) = rust_write_structural(t, rust, expr, schema, depth) {
            return Some(code);
        }
        return Some(format!(
            "wire::write_value(w, &serde_json::to_value({expr}).map_err(|e| e.to_string())?, {})?;",
            schema_ref(schema)
        ));
    }
    rust_write_structural(t, rust, expr, schema, depth)
}

fn rust_write_structural(
    t: &Ty,
    rust: &str,
    expr: &str,
    schema: &str,
    depth: usize,
) -> Option<String> {
    let rust = rust.trim();
    let x = format!("x{depth}");
    // A method receiver: `out` as is, `&out` in parentheses.
    let r = if expr.starts_with('&') {
        format!("({expr})")
    } else {
        expr.to_string()
    };
    Some(match t {
        Ty::I64(_) if rust == "i64" => format!("w.i64(*{expr});"),
        Ty::F64(_) if rust == "f64" => format!("w.f64(*{expr});"),
        Ty::Bool if rust == "bool" => format!("w.bool(*{expr});"),
        Ty::Str if rust == "String" => format!("w.string({expr});"),
        Ty::Opt { inner, .. } => {
            let inner_schema = format!("{schema}.inner()");
            match generic_arg(rust, "Option") {
                Some(inner_rust) => format!(
                    "match {expr} {{ Some({x}) => {{ w.tag(1); {} }} None => w.tag(0) }}",
                    rust_write(inner, inner_rust, &x, &inner_schema, depth + 1)?
                ),
                None => format!(
                    "w.tag(1); {}",
                    rust_write(inner, rust, expr, &inner_schema, depth + 1)?
                ),
            }
        }
        Ty::List { item, .. } => format!(
            "w.tag(1); w.count({r}.len()); for {x} in {r}.iter() {{ {} }}",
            rust_write(
                item,
                generic_arg(rust, "Vec")?,
                &x,
                &format!("{schema}.inner()"),
                depth + 1
            )?
        ),
        Ty::Map { value, .. } => format!(
            "w.tag(1); w.count({r}.len()); for (k{depth}, {x}) in {r}.iter() {{ w.string(k{depth}); {} }}",
            rust_write(
                value,
                map_value(rust)?,
                &x,
                &format!("{schema}.inner()"),
                depth + 1
            )?
        ),
        Ty::Num if rust == "f64" => format!("w.f64(*{expr});"),
        Ty::Str16 if rust == "String" => format!("w.string16({expr});"),
        Ty::Str16 if rust == "Vec<u16>" => format!("w.units({expr});"),
        Ty::OptJs {
            inner, undefinable, ..
        } => {
            let inner_schema = format!("{schema}.inner()");
            // `None` is `undefined`, or `null` for a `T | null` that cannot be undefined.
            let none = if *undefinable { 0 } else { 2 };
            match generic_arg(rust, "Option") {
                Some(inner_rust) => format!(
                    "match {expr} {{ Some({x}) => {{ w.tag(1); {} }} None => w.tag({none}) }}",
                    rust_write(inner, inner_rust, &x, &inner_schema, depth + 1)?
                ),
                None => format!(
                    "w.tag(1); {}",
                    rust_write(inner, rust, expr, &inner_schema, depth + 1)?
                ),
            }
        }
        Ty::Map16(value) => format!(
            "w.tag(1); w.count({r}.len()); for (k{depth}, {x}) in {r}.iter() {{ w.string16(k{depth}); {} }}",
            rust_write(
                value,
                map_value(rust)?,
                &x,
                &format!("{schema}.inner()"),
                depth + 1
            )?
        ),
        _ => return None,
    })
}

/// `src/wire.rs` of a generated crate: the reader, the writer, and schema-driven conversion of
/// `serde_json::Value` (for ports that keep structured values as JSON values).
pub const WIRE_RS: &str = r#"//! The binary wire between the source language and the port. Generated by rust-anything.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use serde_json::{Map, Value, json};

/// A wire type, for converting `serde_json::Value`s.
pub enum Schema {
    I64,
    F64,
    Bool,
    Str,
    List(&'static Schema),
    Map(&'static Schema),
    Opt(&'static Schema),
    Struct(&'static [(&'static str, Schema)]),
    Tuple(&'static [Schema]),
    Num,
    Str16,
    OptJs(&'static Schema),
    Obj(&'static [(&'static str, Schema)]),
    Map16(&'static Schema),
    Dyn,
}

impl Schema {
    /// The item type of a list, map or option.
    pub const fn inner(&self) -> &Schema {
        match self {
            Self::List(s) | Self::Map(s) | Self::Opt(s) | Self::OptJs(s) | Self::Map16(s) => s,
            _ => self,
        }
    }
}

pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

const SHORT: &str = "TypeError: wire data ended early";

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.bytes.len()).ok_or(SHORT)?;
        let out = &self.bytes[self.at..end];
        self.at = end;
        Ok(out)
    }

    pub fn tag(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    pub fn count(&mut self) -> Result<usize, String> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    }

    pub fn i64(&mut self) -> Result<i64, String> {
        let b = self.take(8)?;
        Ok(i64::from_le_bytes(b.try_into().map_err(|_| SHORT)?))
    }

    pub fn f64(&mut self) -> Result<f64, String> {
        let b = self.take(8)?;
        Ok(f64::from_le_bytes(b.try_into().map_err(|_| SHORT)?))
    }

    pub fn bool(&mut self) -> Result<bool, String> {
        Ok(self.tag()? != 0)
    }

    /// A string; `None` for null.
    pub fn opt_string(&mut self) -> Result<Option<String>, String> {
        if self.tag()? == 0 {
            return Ok(None);
        }
        let n = self.count()?;
        Ok(Some(String::from_utf8_lossy(self.take(n)?).into_owned()))
    }

    pub fn string(&mut self) -> Result<String, String> {
        self.opt_string()?
            .ok_or_else(|| "NullPointerException: null string".to_string())
    }

    pub fn opt<T>(&mut self, item: impl FnOnce(&mut Self) -> Result<T, String>) -> Result<Option<T>, String> {
        if self.tag()? == 0 { Ok(None) } else { item(self).map(Some) }
    }

    pub fn required<T>(&mut self, item: impl FnOnce(&mut Self) -> Result<T, String>) -> Result<T, String> {
        self.opt(item)?
            .ok_or_else(|| "NullPointerException: null value".to_string())
    }

    /// A JS string as UTF-16 code units.
    pub fn units(&mut self) -> Result<Vec<u16>, String> {
        if self.tag()? == 0 {
            return Err("TypeError: expected a string".into());
        }
        let n = self.count()?;
        let bytes = self.take(n.checked_mul(2).ok_or(SHORT)?)?;
        Ok(bytes.as_chunks::<2>().0.iter().map(|b| u16::from_le_bytes(*b)).collect())
    }

    /// A JS string; lone surrogates become U+FFFD, as napi's own conversion does.
    pub fn string16(&mut self) -> Result<String, String> {
        Ok(String::from_utf16_lossy(&self.units()?))
    }

    /// A JS optional value: 0 `undefined`, 1 a value, 2 `null`.
    pub fn opt_js<T>(&mut self, item: impl FnOnce(&mut Self) -> Result<T, String>) -> Result<Option<T>, String> {
        if self.tag()? == 1 { item(self).map(Some) } else { Ok(None) }
    }

    pub fn required_js<T>(&mut self, item: impl FnOnce(&mut Self) -> Result<T, String>) -> Result<T, String> {
        self.opt_js(item)?
            .ok_or_else(|| "TypeError: expected a value, got undefined or null".to_string())
    }

    /// A string-keyed record (keys as JS strings).
    pub fn map16<T>(&mut self, mut item: impl FnMut(&mut Self) -> Result<T, String>) -> Result<Vec<(String, T)>, String> {
        if self.tag()? == 0 {
            return Ok(Vec::new());
        }
        let n = self.count()?;
        let mut out = Vec::with_capacity(n.min(1 << 16));
        for _ in 0..n {
            let key = self.string16()?;
            out.push((key, item(self)?));
        }
        Ok(out)
    }

    /// A list; nil reads as empty.
    pub fn list<T>(&mut self, mut item: impl FnMut(&mut Self) -> Result<T, String>) -> Result<Vec<T>, String> {
        if self.tag()? == 0 {
            return Ok(Vec::new());
        }
        let n = self.count()?;
        let mut out = Vec::with_capacity(n.min(1 << 16));
        for _ in 0..n {
            out.push(item(self)?);
        }
        Ok(out)
    }

    /// A map's entries; nil reads as empty.
    pub fn map<T>(&mut self, mut item: impl FnMut(&mut Self) -> Result<T, String>) -> Result<Vec<(String, T)>, String> {
        if self.tag()? == 0 {
            return Ok(Vec::new());
        }
        let n = self.count()?;
        let mut out = Vec::with_capacity(n.min(1 << 16));
        for _ in 0..n {
            let key = self.string()?;
            out.push((key, item(self)?));
        }
        Ok(out)
    }
}

#[derive(Default)]
pub struct Writer(pub Vec<u8>);

impl Writer {
    pub fn tag(&mut self, t: u8) {
        self.0.push(t);
    }

    pub fn count(&mut self, n: usize) {
        self.0.extend_from_slice(&u32::try_from(n).unwrap_or(u32::MAX).to_le_bytes());
    }

    pub fn i64(&mut self, v: i64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    pub fn f64(&mut self, v: f64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    pub fn bool(&mut self, v: bool) {
        self.0.push(u8::from(v));
    }

    pub fn string(&mut self, s: &str) {
        self.tag(1);
        self.count(s.len());
        self.0.extend_from_slice(s.as_bytes());
    }

    /// A JS string from a Rust one (well-formed: tag 1).
    pub fn string16(&mut self, s: &str) {
        self.tag(1);
        let at = self.0.len();
        self.count(0);
        let mut n = 0usize;
        for u in s.encode_utf16() {
            self.0.extend_from_slice(&u.to_le_bytes());
            n += 1;
        }
        self.0[at..at + 4].copy_from_slice(&u32::try_from(n).unwrap_or(u32::MAX).to_le_bytes());
    }

    /// A JS string from code units, lone surrogates included (tag 4).
    pub fn units(&mut self, units: &[u16]) {
        self.tag(4);
        self.count(units.len());
        for u in units {
            self.0.extend_from_slice(&u.to_le_bytes());
        }
    }
}

fn float_value(x: f64) -> Value {
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

fn value_float(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::Object(o) => match o.get("$num")?.as_str()? {
            "NaN" => Some(f64::NAN),
            "Infinity" => Some(f64::INFINITY),
            "-Infinity" => Some(f64::NEG_INFINITY),
            "-0" => Some(-0.0),
            _ => None,
        },
        _ => None,
    }
}

/// Reads a value of `schema` as JSON: null for nil, `{"$num": ...}` for NaN and infinities.
pub fn read_value(r: &mut Reader, schema: &Schema) -> Result<Value, String> {
    Ok(match schema {
        Schema::I64 => json!(r.i64()?),
        Schema::F64 => float_value(r.f64()?),
        Schema::Bool => Value::Bool(r.bool()?),
        Schema::Str => r.opt_string()?.map_or(Value::Null, Value::String),
        Schema::Opt(inner) => r.opt(|r| read_value(r, inner))?.unwrap_or(Value::Null),
        Schema::List(item) => {
            if r.tag()? == 0 {
                Value::Null
            } else {
                let n = r.count()?;
                let mut out = Vec::with_capacity(n.min(1 << 16));
                for _ in 0..n {
                    out.push(read_value(r, item)?);
                }
                Value::Array(out)
            }
        }
        Schema::Map(item) => {
            if r.tag()? == 0 {
                Value::Null
            } else {
                let n = r.count()?;
                let mut out = Map::new();
                for _ in 0..n {
                    let key = r.string()?;
                    out.insert(key, read_value(r, item)?);
                }
                Value::Object(out)
            }
        }
        Schema::Struct(fields) => {
            if r.tag()? == 0 {
                Value::Null
            } else {
                let mut out = Map::new();
                for (key, field) in *fields {
                    out.insert((*key).to_string(), read_value(r, field)?);
                }
                Value::Object(out)
            }
        }
        Schema::Tuple(items) => Value::Array(items.iter().map(|s| read_value(r, s)).collect::<Result<_, _>>()?),
        Schema::Num => js_number(r.f64()?),
        Schema::Str16 => js_string(r.units()?),
        Schema::OptJs(inner) => match r.tag()? {
            1 => read_value(r, inner)?,
            2 => Value::Null,
            _ => json!({"$undefined": true}),
        },
        Schema::Obj(fields) => {
            let n = r.tag()?;
            let mut out = Map::new();
            for _ in 0..n {
                let index = usize::from(r.tag()?);
                match fields.get(index) {
                    Some((key, field)) => {
                        out.insert((*key).to_string(), read_value(r, field)?);
                    }
                    None => {
                        let key = r.string16()?;
                        out.insert(key, read_dyn(r)?);
                    }
                }
            }
            Value::Object(out)
        }
        Schema::Map16(item) => {
            if r.tag()? == 0 {
                Value::Null
            } else {
                let n = r.count()?;
                let mut out = Map::new();
                for _ in 0..n {
                    let key = r.string16()?;
                    out.insert(key, read_value(r, item)?);
                }
                Value::Object(out)
            }
        }
        Schema::Dyn => read_dyn(r)?,
    })
}

/// A JS number as JSON, as the JSON bridge reads it: integers as integers, markers for the rest.
fn js_number(x: f64) -> Value {
    if x.is_finite() && !(x == 0.0 && x.is_sign_negative()) && x.fract() == 0.0 && x.abs() < 9_007_199_254_740_992.0 {
        json!(x as i64)
    } else {
        float_value(x)
    }
}

/// A JS string as JSON: lone surrogates as `{"$utf16": [...]}`.
fn js_string(units: Vec<u16>) -> Value {
    match String::from_utf16(&units) {
        Ok(s) => Value::String(s),
        Err(_) => json!({"$utf16": units}),
    }
}

/// Any JS value, tagged: 0 undefined, 1 null, 2 false, 3 true, 4 number, 5 string, 6 array,
/// 7 object.
/// An optional JS value: `undefined` and `null` are `None`, as JSON gives for an `Option`.
pub fn read_dyn_opt(r: &mut Reader) -> Result<Option<Value>, String> {
    if matches!(r.bytes.get(r.at), Some(0 | 1)) {
        r.at += 1;
        return Ok(None);
    }
    read_dyn(r).map(Some)
}

pub fn read_dyn(r: &mut Reader) -> Result<Value, String> {
    let units = |r: &mut Reader| -> Result<Vec<u16>, String> {
        let n = r.count()?;
        let bytes = r.take(n.checked_mul(2).ok_or(SHORT)?)?;
        Ok(bytes.as_chunks::<2>().0.iter().map(|b| u16::from_le_bytes(*b)).collect())
    };
    Ok(match r.tag()? {
        0 => json!({"$undefined": true}),
        1 => Value::Null,
        2 => Value::Bool(false),
        3 => Value::Bool(true),
        4 => js_number(r.f64()?),
        5 => js_string(units(r)?),
        6 => {
            let n = r.count()?;
            let mut out = Vec::with_capacity(n.min(1 << 16));
            for _ in 0..n {
                out.push(read_dyn(r)?);
            }
            Value::Array(out)
        }
        7 => {
            let n = r.count()?;
            let mut out = Map::new();
            for _ in 0..n {
                let key = String::from_utf16_lossy(&units(r)?);
                out.insert(key, read_dyn(r)?);
            }
            Value::Object(out)
        }
        t => return Err(format!("TypeError: unknown value tag {t}")),
    })
}

fn marker<'v>(v: &'v Value, key: &str) -> Option<&'v Value> {
    v.as_object().filter(|o| o.len() == 1).and_then(|o| o.get(key))
}

fn write_units16(w: &mut Writer, v: &Value) -> Option<()> {
    if let Value::String(s) = v {
        w.string16(s);
        return Some(());
    }
    let units: Vec<u16> = marker(v, "$utf16")?
        .as_array()?
        .iter()
        .map(|u| u.as_u64().and_then(|u| u16::try_from(u).ok()))
        .collect::<Option<_>>()?;
    w.units(&units);
    Some(())
}

/// Writes any JSON value as a tagged JS value; markers become what they stand for.
pub fn write_dyn(w: &mut Writer, v: &Value) {
    match v {
        Value::Null => w.tag(1),
        Value::Bool(b) => w.tag(if *b { 3 } else { 2 }),
        Value::Number(n) => {
            w.tag(4);
            w.f64(n.as_f64().unwrap_or(f64::NAN));
        }
        Value::String(s) => {
            w.tag(5);
            let units: Vec<u16> = s.encode_utf16().collect();
            w.count(units.len());
            for u in units {
                w.0.extend_from_slice(&u.to_le_bytes());
            }
        }
        Value::Array(items) => {
            w.tag(6);
            w.count(items.len());
            for x in items {
                write_dyn(w, x);
            }
        }
        Value::Object(o) => {
            if marker(v, "$undefined").is_some() {
                w.tag(0);
            } else if let Some(x) = value_float(v).filter(|_| marker(v, "$num").is_some()) {
                w.tag(4);
                w.f64(x);
            } else if let Some(units) = marker(v, "$utf16").and_then(Value::as_array) {
                w.tag(5);
                w.count(units.len());
                for u in units {
                    w.0.extend_from_slice(&u16::try_from(u.as_u64().unwrap_or(0xFFFD)).unwrap_or(0xFFFD).to_le_bytes());
                }
            } else {
                w.tag(7);
                w.count(o.len());
                for (k, x) in o {
                    let units: Vec<u16> = k.encode_utf16().collect();
                    w.count(units.len());
                    for u in units {
                        w.0.extend_from_slice(&u.to_le_bytes());
                    }
                    write_dyn(w, x);
                }
            }
        }
    }
}

/// The start of the error a result of the wrong shape raises; JS bridges answer it by running
/// the call through JSON instead.
pub const MISMATCH: &str = "TypeError: the port returned ";

fn mismatch(schema: &str, v: &Value) -> String {
    format!("{MISMATCH}{v} where the original returns {schema}")
}

/// Writes a JSON value as `schema`; the port's value must fit the original's type.
pub fn write_value(w: &mut Writer, v: &Value, schema: &Schema) -> Result<(), String> {
    match schema {
        Schema::I64 => w.i64(v.as_i64().or_else(|| v.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64)).ok_or_else(|| mismatch("an integer", v))?),
        Schema::F64 => w.f64(value_float(v).ok_or_else(|| mismatch("a float", v))?),
        Schema::Bool => w.bool(v.as_bool().ok_or_else(|| mismatch("a bool", v))?),
        Schema::Str => match v {
            Value::Null => w.tag(0),
            Value::String(s) => w.string(s),
            _ => return Err(mismatch("a string", v)),
        },
        Schema::Opt(inner) => {
            if v.is_null() {
                w.tag(0);
            } else {
                w.tag(1);
                write_value(w, v, inner)?;
            }
        }
        Schema::List(item) => match v {
            Value::Null => w.tag(0),
            Value::Array(items) => {
                w.tag(1);
                w.count(items.len());
                for x in items {
                    write_value(w, x, item)?;
                }
            }
            _ => return Err(mismatch("a list", v)),
        },
        Schema::Map(item) => match v {
            Value::Null => w.tag(0),
            Value::Object(o) => {
                w.tag(1);
                w.count(o.len());
                for (k, x) in o {
                    w.string(k);
                    write_value(w, x, item)?;
                }
            }
            _ => return Err(mismatch("a map", v)),
        },
        Schema::Struct(fields) => match v {
            Value::Null => w.tag(0),
            Value::Object(o) => {
                w.tag(1);
                for (key, field) in *fields {
                    write_value(w, o.get(*key).unwrap_or(&Value::Null), field)?;
                }
            }
            _ => return Err(mismatch("a struct", v)),
        },
        Schema::Tuple(items) => match v {
            Value::Array(values) if values.len() == items.len() => {
                for (x, s) in values.iter().zip(*items) {
                    write_value(w, x, s)?;
                }
            }
            _ => return Err(mismatch("several results", v)),
        },
        Schema::Num => w.f64(value_float(v).ok_or_else(|| mismatch("a number", v))?),
        Schema::Str16 => write_units16(w, v).ok_or_else(|| mismatch("a string", v))?,
        Schema::OptJs(inner) => {
            if marker(v, "$undefined").is_some() {
                w.tag(0);
            } else if v.is_null() {
                w.tag(2);
            } else {
                w.tag(1);
                write_value(w, v, inner)?;
            }
        }
        Schema::Obj(fields) => {
            let o = v.as_object().ok_or_else(|| mismatch("an object", v))?;
            w.tag(u8::try_from(o.len()).map_err(|_| mismatch("an object of at most 255 keys", v))?);
            for (k, x) in o {
                match fields.iter().position(|(key, _)| key == k) {
                    Some(i) if i < 255 => {
                        w.tag(i as u8);
                        write_value(w, x, &fields[i].1)?;
                    }
                    _ => {
                        w.tag(255);
                        w.string16(k);
                        write_dyn(w, x);
                    }
                }
            }
        }
        Schema::Map16(item) => match v {
            Value::Null => w.tag(0),
            Value::Object(o) => {
                w.tag(1);
                w.count(o.len());
                for (k, x) in o {
                    w.string16(k);
                    write_value(w, x, item)?;
                }
            }
            _ => return Err(mismatch("a record", v)),
        },
        Schema::Dyn => write_dyn(w, v),
    }
    Ok(())
}
"#;

/// Rust statements that read every argument of a port function from `r`; `None` when one does
/// not fit.
pub fn rust_read_args(
    params: &[(String, Ty)],
    port_params: &[(String, String)],
    schema_prefix: &str,
    statics: &mut String,
) -> Option<String> {
    let mut out = String::new();
    for (i, ((_, ty), (name, rust))) in params.iter().zip(port_params).enumerate() {
        let schema = format!("{schema_prefix}_{i}");
        let read = rust_read(ty, rust, &schema)?;
        if read.contains(&schema) {
            let _ = writeln!(
                statics,
                "static {schema}: wire::Schema = {};",
                schema_expr(ty)
            );
        }
        let _ = writeln!(out, "        let {name}: {rust} = {read};");
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(item: Ty) -> Ty {
        Ty::List {
            item: Box::new(item),
            native: String::new(),
            array: false,
        }
    }

    #[test]
    fn reads_fit_the_port_types() {
        let strings = list(Ty::Str);
        assert_eq!(
            rust_read(&strings, "Vec<String>", "S").unwrap(),
            "r.list(|r| r.string())?"
        );
        let boxed = Ty::Opt {
            inner: Box::new(Ty::I64("int".into())),
            native: "Integer".into(),
        };
        assert_eq!(
            rust_read(&boxed, "i64", "S").unwrap(),
            "r.required(|r| r.i64())?"
        );
        assert_eq!(
            rust_read(&boxed, "Option<i64>", "S").unwrap(),
            "r.opt(|r| r.i64())?"
        );
        assert_eq!(
            rust_read(&strings, "serde_json::Value", "S").unwrap(),
            "wire::read_value(r, &S)?"
        );
        // A port type that does not fit the wire type keeps the JSON bridge.
        assert!(rust_read(&strings, "Vec<i64>", "S").is_none());
    }

    #[test]
    fn a_scalar_float_under_an_untyped_declaration_keeps_nan_and_infinities() {
        // `serde_json::to_value(NaN)` is `Value::Null`, so a port typed `f64` whose declaration
        // says only `unknown` reached JS as `null`: `diff` then blamed the port for a binding bug.
        let code = rust_write(&Ty::Dyn, "f64", "out", "S", 0).unwrap();
        assert!(code.contains("w.f64("), "{code}");
        assert!(!code.contains("to_value"), "{code}");
        let single = rust_write(&Ty::Dyn, "f32", "out", "S", 0).unwrap();
        assert!(single.contains("w.f64("), "{single}");
        // A qualified spelling is the same type.
        for spelled in ["std::primitive::f64", "core::primitive::f32"] {
            let qualified = rust_write(&Ty::Dyn, spelled, "out", "S", 0).unwrap();
            assert!(qualified.contains("w.f64("), "{spelled}: {qualified}");
            assert!(!qualified.contains("to_value"), "{spelled}: {qualified}");
        }
    }

    #[test]
    fn writes_fit_the_port_types() {
        let floats = list(Ty::F64("double".into()));
        assert_eq!(
            rust_write(&floats, "Vec<f64>", "out", "S", 0).unwrap(),
            "w.tag(1); w.count(out.len()); for x0 in out.iter() { w.f64(*x0); }"
        );
        assert_eq!(
            schema_expr(&floats),
            "wire::Schema::List(&wire::Schema::F64)"
        );
    }
}
