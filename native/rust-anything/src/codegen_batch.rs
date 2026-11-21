//! Batch classes: a function run over many items that stay on the Rust side.
//!
//! `bridge --batch fuzzyScore:text` gives the crate a `FuzzyScoreBatch` class: the constructor
//! takes the items (the `text` values) once; `run(query)` applies the port to every item on all
//! cores and returns the results as one typed array; `rank(query, min)` returns the indexes of
//! the items whose result is at least `min`, best first. One call does what a loop of per-item
//! calls did, so the crossing is paid once instead of per item.
//!
//! The default index calls the port per item. A port can do better by defining
//! `pub struct FuzzyScoreIndex` in `port.rs` with `pub fn new(items: Vec<T>) -> Self` and
//! `pub fn run(&self, <other params>) -> Result<Vec<R>, String>`: it can prepare the items once
//! (lowercase them, build masks) and the class uses it instead.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::bridge::{Kind, Manifest, snake_case};
use crate::codegen::{PortApi, PortFn};

/// `--batch function:param`: run `function` over many values of `param`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Batch {
    pub function: String,
    pub param: String,
}

impl Batch {
    pub fn parse(spec: &str) -> Option<Self> {
        let (function, param) = spec.split_once(':')?;
        Some(Self {
            function: function.trim().into(),
            param: param.trim().into(),
        })
    }
}

pub fn pascal(name: &str) -> String {
    let mut out = String::new();
    let mut up = true;
    for c in name.chars() {
        if c == '_' || c == '-' {
            up = true;
        } else if up {
            out.extend(c.to_uppercase());
            up = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// A batch the port's signature allows.
pub(crate) struct Spec<'a> {
    pub f: &'a PortFn,
    pub item: usize,
    /// The stored item type (`&str` items are stored as `String`).
    pub item_ty: String,
    pub class: String,
    pub index: String,
    pub custom: bool,
    /// The custom index has its own `rank(&self, <fixed>, min: f64) -> Result<Vec<u32>, String>`.
    pub custom_rank: bool,
}

const ITEMS: &[&str] = &["String", "&str", "f64", "i64", "bool"];
/// Whether a parameter type is `Copy`, so each item's call takes it as is; the rest are cloned.
pub(crate) fn is_copy(ty: &str) -> bool {
    let ty = ty.trim();
    if let Some(inner) = ty.strip_prefix("Option<").and_then(|t| t.strip_suffix('>')) {
        return is_copy(inner);
    }
    if let Some(items) = ty.strip_prefix('(').and_then(|t| t.strip_suffix(')')) {
        return items
            .split(',')
            .filter(|t| !t.trim().is_empty())
            .all(is_copy);
    }
    matches!(
        ty,
        "f64"
            | "f32"
            | "i64"
            | "i32"
            | "i16"
            | "i8"
            | "u64"
            | "u32"
            | "u16"
            | "u8"
            | "usize"
            | "isize"
            | "bool"
            | "char"
    )
}

const RESULTS: &[&str] = &["f64", "i64", "bool", "String"];

/// Why a batch cannot be generated, or its spec.
fn spec<'a>(b: &Batch, api: &'a PortApi) -> Result<Spec<'a>, String> {
    spec_with(b, api, false)
}

/// The spec; `dynamic` also accepts `serde_json::Value` items and results (Ruby converts them
/// per item in Rust).
pub(crate) fn spec_with<'a>(
    b: &Batch,
    api: &'a PortApi,
    dynamic: bool,
) -> Result<Spec<'a>, String> {
    let f = api
        .functions
        .iter()
        .find(|f| f.js_name == b.function)
        .ok_or_else(|| format!("`{}` is not a function of src/port.rs", b.function))?;
    let wanted = snake_case(&b.param);
    let item = f
        .params
        .iter()
        .position(|(n, _)| *n == wanted || *n == b.param)
        .ok_or_else(|| format!("`{}` has no parameter `{}`", b.function, b.param))?;
    let ty = f.params[item].1.trim();
    let value = |t: &str| dynamic && matches!(t, "Value" | "serde_json::Value");
    if !ITEMS.contains(&ty) && !value(ty) {
        return Err(format!(
            "batch items must be strings, numbers or booleans; `{}` is `{ty}`",
            b.param
        ));
    }
    if f.is_async {
        return Err(format!("`{}` is async", b.function));
    }
    if !RESULTS.contains(&f.ret.trim()) && !value(f.ret.trim()) {
        return Err(format!(
            "batch results must be numbers, booleans or strings; `{}` returns `{}`",
            b.function, f.ret
        ));
    }
    let base = pascal(&f.js_name);
    let custom = api.structs.iter().any(|s| *s == format!("{base}Index"));
    let custom_rank = custom && api.ranked.iter().any(|s| *s == format!("{base}Index"));
    Ok(Spec {
        f,
        item,
        item_ty: if ty == "&str" {
            "String".into()
        } else {
            ty.into()
        },
        class: format!("{base}Batch"),
        index: if custom {
            format!("port::{base}Index")
        } else {
            format!("{base}DefaultIndex")
        },
        custom,
        custom_rank,
    })
}

/// The non-item parameters, as `name: Type` (by value: `&str` becomes `String`).
pub(crate) fn fixed(s: &Spec) -> Vec<(String, String)> {
    s.f.params
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != s.item)
        .map(|(_, (n, t))| {
            (
                n.clone(),
                if t.trim() == "&str" {
                    "String".to_string()
                } else {
                    t.clone()
                },
            )
        })
        .collect()
}

/// The default index: the items as the port takes them, the port called per item.
pub(crate) fn default_index(s: &Spec, parallel: bool) -> String {
    let fixed = fixed(s);
    let params: Vec<String> = fixed.iter().map(|(n, t)| format!("{n}: {t}")).collect();
    let args: Vec<String> =
        s.f.params
            .iter()
            .enumerate()
            .map(|(i, (n, t))| {
                let by_ref = t.trim() == "&str";
                match (i == s.item, by_ref) {
                    (true, true) => "item.as_str()".to_string(),
                    (true, false) if is_copy(&s.item_ty) => "*item".into(),
                    (true, false) => "item.clone()".into(),
                    (false, true) => format!("{n}.as_str()"),
                    (false, false) if is_copy(t) => n.clone(),
                    (false, false) => format!("{n}.clone()"),
                }
            })
            .collect();
    let call = format!("port::{}({})", s.f.rust_name, args.join(", "));
    let call = if s.f.fallible {
        call
    } else {
        format!("Ok::<_, String>({call})")
    };
    let iter = if parallel { "par_iter" } else { "iter" };
    format!(
        "\n/// The items of `{class}` as the port takes them; the port runs per item. Define\n\
         /// `pub struct {base}Index` in `port.rs` (with `new` and `run`) to prepare items once.\n\
         pub struct {index} {{\n    items: Vec<{item}>,\n}}\n\n\
         impl {index} {{\n    pub fn new(items: Vec<{item}>) -> Self {{\n        Self {{ items }}\n    }}\n\n    \
         pub fn run(&self, {params}) -> Result<Vec<{ret}>, String> {{\n        \
         {use_rayon}self.items.{iter}().map(|item| {call}).collect::<Vec<_>>().into_iter().collect()\n    }}\n}}\n",
        class = s.class,
        base = s.class.trim_end_matches("Batch"),
        index = s.index,
        item = s.item_ty,
        params = params.join(", "),
        ret = s.f.ret,
        use_rayon = if parallel {
            "use rayon::prelude::*;\n        "
        } else {
            ""
        },
    )
}

/// How each language hands back a `Vec<R>`.
fn napi_out(ret: &str) -> (&'static str, &'static str) {
    match ret {
        "f64" => (
            "napi::bindgen_prelude::Float64Array",
            "napi::bindgen_prelude::Float64Array::new(out)",
        ),
        "i64" => (
            "napi::bindgen_prelude::Float64Array",
            "napi::bindgen_prelude::Float64Array::new(out.into_iter().map(|x| x as f64).collect())",
        ),
        "bool" => (
            "napi::bindgen_prelude::Uint8Array",
            "napi::bindgen_prelude::Uint8Array::new(out.into_iter().map(u8::from).collect())",
        ),
        _ => ("Vec<String>", "out"),
    }
}

/// The ranking code for a spec: the index's own `rank` when it has one, else the generic body
/// over `run`'s results (the generated call is switched by [`use_custom_rank`]).
pub(crate) fn rank_for(s: &Spec) -> Option<String> {
    let body = rank_body(s.f.ret.trim())?;
    Some(if s.custom_rank {
        "let order: Vec<u32> = out;".to_string()
    } else {
        body
    })
}

/// In code appended to `out` since `from`: calls the index's `rank(<fixed>, min)` instead of
/// `run(<fixed>)`, when it has one.
pub(crate) fn use_custom_rank(out: &mut String, from: usize, s: &Spec, names: &[&str]) {
    if !s.custom_rank {
        return;
    }
    let args = names.join(", ");
    let with_min = if args.is_empty() {
        "min".to_string()
    } else {
        format!("{args}, min")
    };
    let tail = out.split_off(from);
    out.push_str(&tail.replacen(&format!(".run({args})"), &format!(".rank({with_min})"), 1));
}

pub(crate) fn rank_body(ret: &str) -> Option<String> {
    let value = match ret {
        "f64" => "r",
        "i64" => "r as f64",
        _ => return None,
    };
    Some(format!(
        "let mut hits: Vec<(f64, u32)> = out\n            .into_iter()\n            .enumerate()\n            \
         .map(|(i, r)| ({value}, u32::try_from(i).unwrap_or(u32::MAX)))\n            .filter(|(r, _)| *r >= min)\n            .collect();\n        \
         // Stable: equal results keep the items' order, as JS `sort` and Python `sorted` do.\n        \
         hits.sort_by(|a, b| b.0.total_cmp(&a.0));\n        let order: Vec<u32> = hits.into_iter().map(|(_, i)| i).collect();"
    ))
}

/// Batch classes for a napi-rs or wasm-bindgen `lib.rs`.
pub fn js_rs(m: &Manifest, api: &PortApi) -> Result<String, String> {
    let mut out = String::new();
    let parallel = m.kind == Kind::Napi;
    let fast = if parallel {
        fast_batches(m, api)
    } else {
        Vec::new()
    };
    let strings_out = fast.iter().any(|b| b.strings_out);
    if strings_out {
        out.push_str(JOINED_OUT_RS);
    }
    if fast.iter().any(|b| b.items == Items::Strings) {
        out.push_str(JOINED_IN_RS);
    }
    for b in &m.batches {
        let s = spec(b, api)?;
        if !s.custom {
            out.push_str(&default_index(&s, parallel));
        }
        let fixed = fixed(&s);
        let params: Vec<String> = fixed.iter().map(|(n, t)| format!("{n}: {t}")).collect();
        let names: Vec<&str> = fixed.iter().map(|(n, _)| n.as_str()).collect();
        let ret = s.f.ret.trim();
        let (out_ty, out_expr) = napi_out(ret);
        let wasm = m.kind == Kind::Wasm;
        let (attr, ctor_attr, err, res) = if wasm {
            (
                "wasm_bindgen",
                "#[wasm_bindgen(constructor)]",
                ".map_err(|e| JsError::new(&e))",
                "Result<{T}, JsError>",
            )
        } else {
            ("napi", "#[napi(constructor)]", "", "napi::Result<{T}>")
        };
        let call = |body: &str| -> String {
            if wasm {
                format!(
                    "let out = self.inner.run({}){err}?;\n        {body}",
                    names.join(", ")
                )
            } else {
                format!(
                    "let out = run(|| self.inner.run({}))?;\n        {body}",
                    names.join(", ")
                )
            }
        };
        let out_ty = if wasm {
            out_ty
                .replace("napi::bindgen_prelude::", "Vec<")
                .replace("Float64Array", "f64>")
                .replace("Uint8Array", "u8>")
        } else {
            out_ty.to_string()
        };
        let out_expr = if wasm {
            match ret {
                "i64" => "out.into_iter().map(|x| x as f64).collect()".to_string(),
                "bool" => "out.into_iter().map(u8::from).collect()".to_string(),
                _ => "out".to_string(),
            }
        } else {
            out_expr.to_string()
        };
        let _ = write!(
            out,
            "\n/// `{js}` over many `{param}` values kept in Rust: `run` maps them all in one call{cores};\n\
             /// `rank` returns the indexes of results >= `min`, best first.\n\
             #[{attr}(js_name = \"{class}\")]\npub struct {class} {{\n    inner: {index},\n}}\n\n\
             {impl_attr}\nimpl {class} {{\n    {ctor_attr}\n    pub fn new(items: Vec<{item}>) -> Self {{\n        Self {{ inner: {index}::new(items) }}\n    }}\n{from_joined}\n    \
             /// The result for every item, in order.\n    #[{attr}]\n    pub fn run(&self, {params}) -> {res_run} {{\n        {run_body}\n    }}\n{run_joined}",
            impl_attr = if wasm {
                format!("#[wasm_bindgen(js_class = \"{}\")]", s.class)
            } else {
                "#[napi]".to_string()
            },
            js = s.f.js_name,
            param = b.param,
            cores = if parallel { ", on all cores" } else { "" },
            class = s.class,
            index = s.index,
            item = s.item_ty,
            params = params.join(", "),
            res_run = res.replace("{T}", &out_ty),
            run_body = call(&format!("Ok({out_expr})")),
            from_joined = if parallel && s.item_ty == "String" {
                format!(
                    "\n    /// `new` for items sent as one joined string and their UTF-16 end offsets.\n    \
                     #[napi(factory, js_name = \"fromJoined\")]\n    \
                     pub fn from_joined(text: String, ends: napi::bindgen_prelude::Uint32Array) -> Self {{\n        \
                     Self {{ inner: {}::new(split_joined(&text, &ends)) }}\n    }}\n",
                    s.index
                )
            } else if parallel && s.item_ty == "f64" {
                format!(
                    "\n    /// `new` for items sent as one `Float64Array`.\n    \
                     #[napi(factory, js_name = \"fromFloat64\")]\n    \
                     pub fn from_float64(items: napi::bindgen_prelude::Float64Array) -> Self {{\n        \
                     Self {{ inner: {}::new(items.to_vec()) }}\n    }}\n",
                    s.index
                )
            } else {
                String::new()
            },
            run_joined = if parallel && ret == "String" {
                format!(
                    "\n    /// `run` with the results joined into one string and their UTF-16 end offsets.\n    \
                     #[napi(js_name = \"runJoined\")]\n    \
                     pub fn run_joined(&self, {}) -> napi::Result<JoinedStrings> {{\n        {}\n    }}\n",
                    params.join(", "),
                    call("Ok(joined(out))")
                )
            } else {
                String::new()
            },
        );
        if let Some(rank) = rank_for(&s) {
            let from = out.len();
            let order_ty = if wasm {
                "Vec<u32>"
            } else {
                "napi::bindgen_prelude::Uint32Array"
            };
            let order_expr = if wasm {
                "order"
            } else {
                "napi::bindgen_prelude::Uint32Array::new(order)"
            };
            let sep = if params.is_empty() { "" } else { ", " };
            let _ = write!(
                out,
                "\n    /// Indexes of the items whose result is at least `min`, highest result first.\n    #[{attr}]\n    \
                 pub fn rank(&self, {}{sep}min: f64) -> {} {{\n        {}\n    }}\n",
                params.join(", "),
                res.replace("{T}", order_ty),
                call(&format!("{rank}\n        Ok({order_expr})"))
            );
            use_custom_rank(&mut out, from, &s, &names);
        }
        out.push_str("}\n");
    }
    Ok(out)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Items {
    Strings,
    Numbers,
    Other,
}

/// A napi batch whose values cross in bulk instead of one JS value per item.
pub(crate) struct FastBatch {
    class: String,
    items: Items,
    strings_out: bool,
}

/// The napi batches whose items or results cross in bulk. One JS value per item costs more than
/// the work of a small function, so `index.cjs` joins string items into one string, packs number
/// items into a `Float64Array`, and splits joined string results with `slice`.
pub(crate) fn fast_batches(m: &Manifest, api: &PortApi) -> Vec<FastBatch> {
    if m.kind != Kind::Napi {
        return Vec::new();
    }
    m.batches
        .iter()
        .filter_map(|b| spec(b, api).ok())
        .map(|s| FastBatch {
            class: s.class.clone(),
            items: match s.item_ty.as_str() {
                "String" => Items::Strings,
                "f64" => Items::Numbers,
                _ => Items::Other,
            },
            strings_out: s.f.ret.trim() == "String",
        })
        .filter(|b| b.items != Items::Other || b.strings_out)
        .collect()
}

const JOINED_OUT_RS: &str = r#"
/// Strings joined into one, with the UTF-16 end offset of each (what JS `slice` takes).
#[napi(object)]
pub struct JoinedStrings {
    pub text: String,
    pub ends: napi::bindgen_prelude::Uint32Array,
}

fn joined(out: Vec<String>) -> JoinedStrings {
    let mut text = String::with_capacity(out.iter().map(String::len).sum());
    let mut ends = Vec::with_capacity(out.len());
    let mut at = 0u32;
    for s in &out {
        text.push_str(s);
        let units = if s.is_ascii() { s.len() } else { s.encode_utf16().count() };
        at = at.saturating_add(u32::try_from(units).unwrap_or(u32::MAX));
        ends.push(at);
    }
    JoinedStrings {
        text,
        ends: napi::bindgen_prelude::Uint32Array::new(ends),
    }
}

"#;

const JOINED_IN_RS: &str = r#"
/// Splits `text` at UTF-16 end offsets. A lone surrogate arrived as U+FFFD, one unit as before.
fn split_joined(text: &str, ends: &[u32]) -> Vec<String> {
    let ascii = text.is_ascii()
        && ends.last().is_none_or(|&e| e as usize == text.len())
        && ends.windows(2).all(|w| w[0] <= w[1]);
    let mut out = Vec::with_capacity(ends.len());
    if ascii {
        let mut from = 0;
        for &e in ends {
            out.push(text[from..e as usize].to_string());
            from = e as usize;
        }
        return out;
    }
    let mut chars = text.chars();
    let mut unit = 0usize;
    for &e in ends {
        let mut s = String::new();
        while unit < e as usize {
            let Some(c) = chars.next() else { break };
            unit += c.len_utf16();
            s.push(c);
        }
        out.push(s);
    }
    out
}
"#;

/// The `index.cjs` part that sends and receives the values of [`fast_batches`] in bulk.
pub fn index_cjs(m: &Manifest, api: &PortApi) -> String {
    let batches = fast_batches(m, api);
    if batches.is_empty() {
        return String::new();
    }
    let list = |pick: &dyn Fn(&FastBatch) -> bool| -> String {
        batches
            .iter()
            .filter(|b| pick(b))
            .map(|b| format!("'{}'", b.class))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{JOINED_CJS}for (const name of [{}]) joinItems(name)\n\
         for (const name of [{}]) packNumbers(name)\n\
         for (const name of [{}]) splitResults(name)\n",
        list(&|b| b.items == Items::Strings),
        list(&|b| b.items == Items::Numbers),
        list(&|b| b.strings_out)
    )
}

const JOINED_CJS: &str = r#"
// Batches: strings cross as one joined string and the UTF-16 end offset of each, numbers as one
// Float64Array, which costs far less than one JS value per item. `new Batch(items)` and
// `batch.run()` keep their shape.
const joinItems = name => {
  const Native = native[name]
  const Batch = { [name]: function (items) {
    if (!Array.isArray(items)) throw new TypeError(`${name}: items must be an array of strings`)
    const ends = new Uint32Array(items.length)
    let at = 0
    for (let i = 0; i < items.length; i++) {
      if (typeof items[i] !== 'string') throw new TypeError(`${name}: items must be an array of strings`)
      ends[i] = at += items[i].length
    }
    return Native.fromJoined(items.join(''), ends)
  } }[name]
  Batch.prototype = Native.prototype
  Batch.fromJoined = Native.fromJoined
  module.exports[name] = Batch
}
const packNumbers = name => {
  const Native = native[name]
  const Batch = { [name]: function (items) {
    if (!Array.isArray(items)) throw new TypeError(`${name}: items must be an array of numbers`)
    const packed = new Float64Array(items.length)
    for (let i = 0; i < items.length; i++) {
      if (typeof items[i] !== 'number') throw new TypeError(`${name}: items must be an array of numbers`)
      packed[i] = items[i]
    }
    return Native.fromFloat64(packed)
  } }[name]
  Batch.prototype = Native.prototype
  Batch.fromFloat64 = Native.fromFloat64
  module.exports[name] = Batch
}
const splitResults = name => {
  Object.defineProperty(native[name].prototype, 'run', {
    value: function (...a) {
      const { text, ends } = this.runJoined(...a)
      const out = new Array(ends.length)
      for (let i = 0, from = 0; i < ends.length; from = ends[i++]) out[i] = text.slice(from, ends[i])
      return out
    },
    writable: true,
    configurable: true,
  })
}
"#;

/// Batch classes for a PyO3 `lib.rs`, and the names to register.
pub fn py_rs(m: &Manifest, api: &PortApi) -> Result<(String, Vec<String>), String> {
    let mut out = String::new();
    let mut classes = Vec::new();
    for b in &m.batches {
        let s = spec(b, api)?;
        if !s.custom {
            out.push_str(&default_index(&s, true));
        }
        let fixed = fixed(&s);
        let params: Vec<String> = fixed.iter().map(|(n, t)| format!("{n}: {t}")).collect();
        let names: Vec<&str> = fixed.iter().map(|(n, _)| n.as_str()).collect();
        let ret = s.f.ret.trim();
        let sep = if params.is_empty() { "" } else { ", " };
        let _ = write!(
            out,
            "\n/// `{js}` over many `{param}` values kept in Rust: `run` maps them all in one call, on all\n\
             /// cores, without the GIL; `rank` returns the indexes of results >= `min`, best first.\n\
             #[pyclass(name = \"{class}\")]\npub struct {class} {{\n    inner: {index},\n}}\n\n#[pymethods]\nimpl {class} {{\n    \
             #[new]\n    fn new(items: Vec<{item}>) -> Self {{\n        Self {{ inner: {index}::new(items) }}\n    }}\n\n    \
             /// The result for every item, in order.\n    fn run(&self, py: Python<'_>{sep}{params}) -> PyResult<Vec<{ret}>> {{\n        \
             py.detach(|| self.inner.run({names})).map_err(error)\n    }}\n",
            js = s.f.js_name,
            param = b.param,
            class = s.class,
            index = s.index,
            item = s.item_ty,
            params = params.join(", "),
            names = names.join(", "),
        );
        if let Some(rank) = rank_for(&s) {
            let from = out.len();
            let _ = write!(
                out,
                "\n    /// Indexes of the items whose result is at least `min`, highest result first.\n    \
                 fn rank(&self, py: Python<'_>{sep}{params}, min: f64) -> PyResult<Vec<u32>> {{\n        \
                 let out = py.detach(|| self.inner.run({names})).map_err(error)?;\n        {rank}\n        Ok(order)\n    }}\n",
                params = params.join(", "),
                names = names.join(", "),
            );
            use_custom_rank(&mut out, from, &s, &names);
        }
        out.push_str("}\n");
        classes.push(s.class);
    }
    Ok((out, classes))
}

/// Whether the crate depends on rayon: every crate with batches. Wasm gets it too, so a
/// `port.rs` index using it builds for both kinds: without threads, rayon runs on the calling
/// thread.
pub fn needs_rayon(m: &Manifest) -> bool {
    !m.batches.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_only_copy_types() {
        for t in ["f64", "Option<i64>", "(f64, bool)", "Option<(u32, char)>"] {
            assert!(is_copy(t), "{t}");
        }
        for t in [
            "String",
            "Option<String>",
            "Vec<f64>",
            "(f64, String)",
            "serde_json::Value",
        ] {
            assert!(!is_copy(t), "{t}");
        }
    }

    #[test]
    fn parses_and_names() {
        let b = Batch::parse("fuzzyScore:text").unwrap();
        assert_eq!(
            b,
            Batch {
                function: "fuzzyScore".into(),
                param: "text".into()
            }
        );
        assert_eq!(pascal("fuzzyScore"), "FuzzyScore");
        assert_eq!(pascal("rgb_to_hsv"), "RgbToHsv");
    }
}
