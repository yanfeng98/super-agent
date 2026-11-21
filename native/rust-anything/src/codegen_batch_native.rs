//! Batch classes for Ruby (magnus), Go (cgo) and Java (JNI): the same `run` and `rank` as the
//! TS and Python batches in `codegen_batch`, over items kept in Rust and run on all cores.

use std::fmt::Write as _;

use crate::bridge::Manifest;
use crate::codegen::PortApi;
use crate::codegen_batch::{
    Batch, default_index, fixed, rank_body, rank_for, spec_with, use_custom_rank,
};
use crate::ts::exports::ExportedFn;

/// The export a batch belongs to.
fn export<'m>(m: &'m Manifest, b: &Batch) -> Result<&'m ExportedFn, String> {
    m.exports
        .iter()
        .find(|e| e.name == b.function)
        .ok_or_else(|| format!("`{}` is not an export of the unit", b.function))
}

fn is_value(t: &str) -> bool {
    matches!(t.trim(), "Value" | "serde_json::Value")
}

// ---- Ruby ------------------------------------------------------------------------------------

/// Splits a Ruby parameter list at its top-level commas.
fn split_params(sig: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    for c in sig.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(' | '[' | '{') => depth += 1,
            (None, ')' | ']' | '}') => depth -= 1,
            (None, ',') if depth == 0 => {
                out.push(String::new());
                continue;
            }
            _ => {}
        }
        if let Some(last) = out.last_mut() {
            last.push(c);
        }
    }
    out.into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// The name a Ruby parameter declares: `sep = "-"` is `sep`, `max: nil` is `max`.
fn param_name(p: &str) -> &str {
    let p = p.trim_start_matches(['*', '&']);
    let end = p
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(p.len());
    &p[..end]
}

/// Batch classes for the magnus extension: `(Rust code, init lines)`.
pub fn ruby_rs(m: &Manifest, api: &PortApi) -> Result<(String, String), String> {
    let mut code = String::new();
    let mut init = String::new();
    for b in &m.batches {
        let s = spec_with(b, api, true)?;
        if !s.custom {
            code.push_str(&default_index(&s, true));
        }
        let fixed = fixed(&s);
        let item_value = is_value(&s.item_ty);
        let ret = s.f.ret.trim();
        let ret_value = is_value(ret);
        let params: Vec<String> = fixed
            .iter()
            .map(|(n, t)| {
                if is_value(t) {
                    format!("{n}: magnus::Value")
                } else {
                    format!("{n}: {t}")
                }
            })
            .collect();
        let mut convert = String::new();
        for (n, t) in fixed.iter().filter(|(_, t)| is_value(t)) {
            let _ = writeln!(
                convert,
                "        let {n}: {t} = rbconv::arg({n}).map_err(|m| error(ruby, m))?;"
            );
        }
        let names: Vec<&str> = fixed.iter().map(|(n, _)| n.as_str()).collect();
        let sep = if params.is_empty() { "" } else { ", " };
        let (items_param, items_convert) = if item_value {
            (
                "items: magnus::RArray".to_string(),
                "        let items = (0..items.len())\n            \
                 .map(|i| rbconv::from_rb(items.entry(isize::try_from(i).unwrap_or(isize::MAX)).map_err(|e| e.to_string())?))\n            \
                 .collect::<Result<Vec<_>, _>>()\n            .map_err(|m| error(ruby, m))?;\n",
            )
        } else {
            (format!("items: Vec<{}>", s.item_ty), "")
        };
        let (run_ret, run_tail) = if ret_value {
            (
                "magnus::RArray".to_string(),
                "\n            .and_then(|out| {\n                let array = ruby.ary_new_capa(out.len());\n                \
                 for v in &out {\n                    array.push(rbconv::to_rb(ruby, v)?)?;\n                }\n                Ok(array)\n            })",
            )
        } else {
            (format!("Vec<{ret}>"), "")
        };
        let _ = write!(
            code,
            "\n/// `{js}` over many `{param}` values kept in Rust: `run` maps them all in one call, on\n\
             /// all cores; `rank` returns the indexes of results >= `min`, best first.\n\
             #[magnus::wrap(class = \"RustAnythingNative{camel}::{class}\", free_immediately, size)]\n\
             pub struct {class} {{\n    inner: {index},\n}}\n\n\
             impl {class} {{\n    \
             fn new(ruby: &Ruby, {items_param}) -> Result<Self, Error> {{\n{items_convert}        \
             guard(ruby, || Ok(Self {{ inner: {index}::new(items) }}))\n    }}\n\n    \
             fn run(ruby: &Ruby, rb_self: &Self{sep}{params}) -> Result<{run_ret}, Error> {{\n{convert}        \
             guard(ruby, || rb_self.inner.run({names})){run_tail}\n    }}\n",
            js = s.f.js_name,
            param = b.param,
            camel = crate::codegen_ruby::camel(&m.lib_name()),
            class = s.class,
            index = s.index,
            params = params.join(", "),
            names = names.join(", "),
        );
        let n = fixed.len();
        let _ = write!(
            init,
            "    let class = native.define_class(\"{class}\", ruby.class_object())?;\n    \
             class.define_singleton_method(\"new\", function!({class}::new, 1))?;\n    \
             class.define_method(\"run\", method!({class}::run, {n}))?;\n",
            class = s.class,
        );
        if let Some(rank) = rank_for(&s) {
            let from = code.len();
            let _ = write!(
                code,
                "\n    fn rank(ruby: &Ruby, rb_self: &Self{sep}{params}, min: f64) -> Result<Vec<u32>, Error> {{\n{convert}        \
                 let out = guard(ruby, || rb_self.inner.run({names}))?;\n        {rank}\n        Ok(order)\n    }}\n",
                params = params.join(", "),
                names = names.join(", "),
            );
            use_custom_rank(&mut code, from, &s, &names);
            let _ = writeln!(
                init,
                "    class.define_method(\"rank\", method!({}::rank, {}))?;",
                s.class,
                n + 1
            );
        }
        code.push_str("}\n");
    }
    Ok((code, init))
}

/// Ruby classes in the original modules: `Textkit::SlugifyBatch.new(texts)`, then `run` and
/// `rank` with the original's other parameters, defaults and keywords included.
pub fn ruby_wrapper(m: &Manifest, api: &PortApi) -> Result<String, String> {
    let lib = m.lib_name();
    let mut out = String::new();
    for b in &m.batches {
        let s = spec_with(b, api, true)?;
        let e = export(m, b)?;
        let (recv, _) = crate::lang::ruby::receiver(e);
        let item = e
            .params
            .get(s.item)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| b.param.clone());
        let rest: Vec<String> = split_params(&crate::lang::ruby::signature(e))
            .into_iter()
            .filter(|p| param_name(p) != item)
            .collect();
        let args: Vec<&str> = e
            .params
            .iter()
            .filter(|p| p.name != item)
            .map(|p| p.name.as_str())
            .collect();
        let native = format!(
            "RustAnythingNative{}::{}",
            crate::codegen_ruby::camel(&lib),
            s.class
        );
        let mut rank_sig = rest.clone();
        rank_sig.push("min: 0".into());
        let mut rank_args: Vec<&str> = args.clone();
        rank_args.push("min");
        let rank = if rank_body(s.f.ret.trim()).is_some() {
            format!(
                "\n\n  # Indexes of the items whose result is at least `min`, highest result first.\n  \
                 def rank({})\n    @native.rank({})\n  end",
                rank_sig.join(", "),
                rank_args.join(", ")
            )
        } else {
            String::new()
        };
        let class = format!(
            "# `{name}` over many `{item}` values kept in Rust: built once, then each `run` maps them all\n\
             # in one call, on all cores.\n\
             class {class}\n  def initialize({item}s)\n    @native = {native}.new({item}s.to_a)\n  end\n\n  \
             # The result for every item, in order.\n  def run({sig})\n    @native.run({args})\n  end{rank}\nend",
            name = e.name,
            class = s.class,
            sig = rest.join(", "),
            args = args.join(", "),
        );
        let parts: Vec<&str> = recv.split("::").filter(|p| *p != "Object").collect();
        let mut code = String::new();
        for (depth, part) in parts.iter().enumerate() {
            let _ = writeln!(code, "{}module {part}", "  ".repeat(depth));
        }
        let indent = "  ".repeat(parts.len());
        for line in class.lines() {
            if line.is_empty() {
                code.push('\n');
            } else {
                let _ = writeln!(code, "{indent}{line}");
            }
        }
        for depth in (0..parts.len()).rev() {
            let _ = writeln!(code, "{}end", "  ".repeat(depth));
        }
        let _ = write!(out, "\n{code}");
    }
    Ok(out)
}

// ---- Go --------------------------------------------------------------------------------------

/// A Go batch: the export, its spec, and the Go types of the item, the other parameters and the
/// result.
struct GoBatch<'a> {
    e: &'a ExportedFn,
    s: crate::codegen_batch::Spec<'a>,
    item: crate::codegen_go::Typed,
    /// `(name, Go type, typed)` of the other parameters, in order.
    fixed: Vec<(String, String, crate::codegen_go::Typed)>,
    result: String,
    result_c: &'static str,
    fallible: bool,
}

fn go_batches<'a>(m: &'a Manifest, api: &'a PortApi) -> Result<Vec<GoBatch<'a>>, String> {
    use crate::codegen_go::typed_type;
    let mut out = Vec::new();
    for b in &m.batches {
        let s = spec_with(b, api, false)?;
        let e = export(m, b)?;
        let typed = |p: &crate::ts::exports::Param| {
            let go = p.native_type.clone().unwrap_or_default();
            match typed_type(&go) {
                Some(t) => Ok((go, t)),
                None => Err(format!(
                    "Go batches take numbers, booleans and strings; `{}` is `{go}`",
                    p.name
                )),
            }
        };
        let (_, item) = typed(
            e.params
                .get(s.item)
                .ok_or("parameter count differs from port.rs")?,
        )?;
        let mut fixed = Vec::new();
        for (i, p) in e.params.iter().enumerate().filter(|(i, _)| *i != s.item) {
            let (go, t) = typed(p)?;
            let rust = &s.f.params[i].0;
            fixed.push((rust.clone(), go, t));
        }
        let (results, fallible) = crate::codegen_go::results(e);
        let [result] = results.as_slice() else {
            return Err(format!("`{}` must return one value to batch", e.name));
        };
        let t = typed_type(result).ok_or_else(|| {
            format!(
                "Go batches return numbers, booleans or strings; `{}` returns `{result}`",
                e.name
            )
        })?;
        out.push(GoBatch {
            e,
            s,
            item,
            fixed,
            result: result.clone(),
            result_c: t.c,
            fallible,
        });
    }
    Ok(out)
}

/// The C parameters of the items, as declared and as Rust takes them.
fn go_item_params(item_c: &str) -> (&'static str, &'static str) {
    match item_c {
        "string" => (
            "const uint8_t *bytes, size_t bytes_len, const size_t *lens, size_t count",
            "bytes: *const u8, bytes_len: usize, lens: *const usize, count: usize",
        ),
        "int64_t" => (
            "const int64_t *items, size_t count",
            "items: *const i64, count: usize",
        ),
        "double" => (
            "const double *items, size_t count",
            "items: *const f64, count: usize",
        ),
        _ => (
            "const bool *items, size_t count",
            "items: *const bool, count: usize",
        ),
    }
}

/// The other parameters in C and Rust; strings as bytes and length.
fn go_fixed_params(
    fixed: &[(String, String, crate::codegen_go::Typed)],
) -> (Vec<String>, Vec<String>, String) {
    let (mut c, mut rust, mut convert) = (Vec::new(), Vec::new(), String::new());
    for (n, _, t) in fixed {
        if t.c == "string" {
            c.push(format!("const uint8_t *{n}_ptr, size_t {n}_len"));
            rust.push(format!("{n}_ptr: *const u8, {n}_len: usize"));
            let _ = writeln!(
                convert,
                "    // SAFETY: Go passes the string's bytes and length.\n    let {n} = unsafe {{ go_str({n}_ptr, {n}_len) }};"
            );
        } else {
            c.push(format!("{} {n}", t.c));
            rust.push(format!("{n}: {}", t.rust));
        }
    }
    (c, rust, convert)
}

/// The results' C out-parameters: Go-owned for numbers and booleans; for strings, the results
/// stay in Rust until Go has a buffer of their total size (`rust_anything_b_take_strings`).
fn go_out_params(result_c: &str) -> (&'static str, String) {
    match result_c {
        "string" => (
            "void **res, size_t *total",
            "res: *mut *mut Vec<String>, total: *mut usize".into(),
        ),
        "int64_t" => (
            "int64_t *out, size_t cap",
            "out: *mut i64, cap: usize".into(),
        ),
        "double" => (
            "double *out, size_t cap",
            "out: *mut f64, cap: usize".into(),
        ),
        _ => ("bool *out, size_t cap", "out: *mut bool, cap: usize".into()),
    }
}

/// C prototypes of the batch functions, for the cgo preamble.
pub fn go_prototypes(m: &Manifest, api: &PortApi) -> String {
    let Ok(batches) = go_batches(m, api) else {
        return String::new();
    };
    let mut out = String::new();
    if !batches.is_empty() {
        out.push_str(
            "void rust_anything_b_take_strings(void *res, uint8_t *buf, size_t len, size_t *lens, size_t cap);\n",
        );
    }
    for g in &batches {
        let name = &g.e.name;
        let (items, _) = go_item_params(g.item.c);
        let (fixed, _, _) = go_fixed_params(&g.fixed);
        let pre: String = fixed.iter().map(|p| format!(", {p}")).collect();
        let (outs, _) = go_out_params(g.result_c);
        let _ = writeln!(out, "void *rust_anything_b_{name}_new({items});");
        let _ = writeln!(out, "void rust_anything_b_{name}_free(void *h);");
        let _ = writeln!(
            out,
            "int rust_anything_b_{name}_run(const void *h{pre}, {outs}, char **err);"
        );
        if rank_body(g.s.f.ret.trim()).is_some() {
            let _ = writeln!(
                out,
                "int rust_anything_b_{name}_rank(const void *h{pre}, double min, uint32_t *order, size_t cap, size_t *n, char **err);"
            );
        }
    }
    out
}

const GO_BATCH_RS: &str = r#"
/// A slice from a pointer and length that may be null and 0.
///
/// # Safety
///
/// `ptr` points at `len` readable values, or `len` is 0.
// SAFETY: unsafe to call; callers uphold the contract above.
unsafe fn batch_in<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
    if len == 0 || ptr.is_null() {
        return &[];
    }
    // SAFETY: the caller passes a valid pointer and length (see above).
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

/// Strings packed by Go: their bytes, one after another, and each one's length.
///
/// # Safety
///
/// `bytes` holds `bytes_len` bytes and `lens` holds `count` lengths that sum to at most it.
// SAFETY: unsafe to call; callers uphold the contract above.
unsafe fn batch_strings(bytes: *const u8, bytes_len: usize, lens: *const usize, count: usize) -> Vec<String> {
    // SAFETY: as documented above.
    let (bytes, lens) = unsafe { (batch_in(bytes, bytes_len), batch_in(lens, count)) };
    let mut at = 0;
    lens.iter()
        .map(|&n| {
            let end = (at + n).min(bytes.len());
            let s = String::from_utf8_lossy(&bytes[at.min(end)..end]).into_owned();
            at = end;
            s
        })
        .collect()
}

/// Writes up to `cap` values to a Go slice.
///
/// # Safety
///
/// `out` points at `cap` writable values, or `cap` is 0.
// SAFETY: unsafe to call; callers uphold the contract above.
unsafe fn batch_out<T: Copy>(out: *mut T, cap: usize, values: &[T]) {
    if cap == 0 || out.is_null() {
        return;
    }
    // SAFETY: Go passes a slice of `cap` values (see above).
    let out = unsafe { std::slice::from_raw_parts_mut(out, cap) };
    for (o, v) in out.iter_mut().zip(values) {
        *o = *v;
    }
}

/// Moves string results into Go's buffer: their bytes one after another, and each one's length.
/// The copies and the frees run on all cores.
///
/// # Safety
///
/// `res` comes from a batch's `run` and is taken once; `buf` holds `len` bytes, the results'
/// total from `run`, and `lens` holds `cap` values.
// SAFETY: exported unmangled under a name only rust-anything defines.
#[unsafe(no_mangle)]
// SAFETY: callers uphold the contract under `# Safety`.
pub unsafe extern "C" fn rust_anything_b_take_strings(res: *mut Vec<String>, buf: *mut u8, len: usize, lens: *mut usize, cap: usize) {
    use rayon::prelude::*;
    if res.is_null() {
        return;
    }
    // SAFETY: `res` was boxed by `run` and is taken once (see above).
    let values: Vec<String> = *unsafe { Box::from_raw(res) };
    let sizes: Vec<usize> = values.iter().map(String::len).collect();
    // SAFETY: Go passes `lens` of `cap` values (see above).
    unsafe { batch_out(lens, cap, &sizes) };
    if len > 0 && !buf.is_null() {
        // SAFETY: Go passes `len` writable bytes, for this call only (see above).
        let mut rest = unsafe { std::slice::from_raw_parts_mut(buf, len) };
        let mut parts = Vec::with_capacity(values.len());
        for n in &sizes {
            let all = std::mem::take(&mut rest);
            let (head, tail) = all.split_at_mut((*n).min(all.len()));
            parts.push(head);
            rest = tail;
        }
        parts
            .into_par_iter()
            .zip(values.par_iter())
            .with_min_len(256)
            .for_each(|(dst, v)| dst.copy_from_slice(&v.as_bytes()[..dst.len()]));
    }
    values.into_par_iter().with_min_len(256).for_each(drop);
}
"#;

/// The batch functions for `lib.rs`.
pub fn go_rs(m: &Manifest, api: &PortApi) -> Result<String, String> {
    let batches = go_batches(m, api)?;
    if batches.is_empty() {
        return Ok(String::new());
    }
    let mut out = GO_BATCH_RS.to_string();
    for g in &batches {
        let s = &g.s;
        if !s.custom {
            out.push_str(&default_index(s, true));
        }
        let name = &g.e.name;
        let index = &s.index;
        let (_, items) = go_item_params(g.item.c);
        let read_items = if g.item.c == "string" {
            "batch_strings(bytes, bytes_len, lens, count)".to_string()
        } else {
            "batch_in(items, count).to_vec()".to_string()
        };
        let (_, fixed, convert) = go_fixed_params(&g.fixed);
        let pre: String = fixed.iter().map(|p| format!(", {p}")).collect();
        let names: Vec<&str> = g.fixed.iter().map(|(n, ..)| n.as_str()).collect();
        let (_, outs) = go_out_params(g.result_c);
        let write = if g.result_c == "string" {
            "            // SAFETY: Go passes valid `res` and `total`, and takes the results with\n            \
             // `rust_anything_b_take_strings`.\n            \
             unsafe {\n                *total = values.iter().map(String::len).sum();\n                \
             *res = Box::into_raw(Box::new(values));\n            }"
                .to_string()
        } else {
            "            // SAFETY: Go passes a slice of `cap` values.\n            unsafe { batch_out(out, cap, &values) };".to_string()
        };
        let _ = write!(
            out,
            "\n/// Builds a `{class}`: the items cross once.\n///\n/// # Safety\n///\n/// Called by the generated Go wrapper with valid pointers.\n\
             // SAFETY: exported unmangled under a name only rust-anything defines.\n#[unsafe(no_mangle)]\n\
             // SAFETY: callers uphold the contract under `# Safety`.\n\
             pub unsafe extern \"C\" fn rust_anything_b_{name}_new({items}) -> *mut {index} {{\n    \
             // SAFETY: Go passes its items' memory (see above).\n    let items = unsafe {{ {read_items} }};\n    \
             std::panic::catch_unwind(|| {index}::new(items))\n        .map_or(std::ptr::null_mut(), |index| Box::into_raw(Box::new(index)))\n}}\n\n\
             /// Frees a `{class}`.\n///\n/// # Safety\n///\n/// `h` comes from `rust_anything_b_{name}_new`, once.\n\
             // SAFETY: exported unmangled under a name only rust-anything defines.\n#[unsafe(no_mangle)]\n\
             // SAFETY: callers uphold the contract under `# Safety`.\n\
             pub unsafe extern \"C\" fn rust_anything_b_{name}_free(h: *mut {index}) {{\n    if !h.is_null() {{\n        \
             // SAFETY: `h` was boxed by `_new` and is freed once.\n        drop(unsafe {{ Box::from_raw(h) }});\n    }}\n}}\n\n\
             /// `{name}` over every item: 0, or 1 with the error in `err`.\n///\n/// # Safety\n///\n/// Called by the generated Go wrapper with a live handle and valid pointers.\n\
             // SAFETY: exported unmangled under a name only rust-anything defines.\n#[unsafe(no_mangle)]\n\
             // SAFETY: callers uphold the contract under `# Safety`.\n\
             pub unsafe extern \"C\" fn rust_anything_b_{name}_run(h: *const {index}{pre}, {outs}, err: *mut *mut c_char) -> i32 {{\n{convert}    \
             // SAFETY: Go keeps the batch alive during the call.\n    let index = unsafe {{ &*h }};\n    \
             match run(|| index.run({args})) {{\n        Ok(values) => {{\n{write}\n            0\n        }}\n        \
             // SAFETY: Go passes a valid `err`.\n        Err(message) => unsafe {{ fail(err, message) }},\n    }}\n}}\n",
            class = s.class,
            args = names.join(", "),
        );
        if let Some(rank) = rank_for(s) {
            let from = out.len();
            let _ = write!(
                out,
                "\n/// Indexes of the items whose result is at least `min`, best first: 0, or 1 with the error.\n///\n/// # Safety\n///\n\
                 /// Called by the generated Go wrapper with a live handle and valid pointers.\n\
                 // SAFETY: exported unmangled under a name only rust-anything defines.\n#[unsafe(no_mangle)]\n\
                 // SAFETY: callers uphold the contract under `# Safety`.\n\
                 pub unsafe extern \"C\" fn rust_anything_b_{name}_rank(h: *const {index}{pre}, min: f64, order_out: *mut u32, cap: usize, n: *mut usize, err: *mut *mut c_char) -> i32 {{\n{convert}    \
                 // SAFETY: Go keeps the batch alive during the call.\n    let index = unsafe {{ &*h }};\n    \
                 match run(|| index.run({args})) {{\n        Ok(out) => {{\n            {rank}\n            \
                 // SAFETY: Go passes `order_out` of `cap` values and a valid `n`.\n            unsafe {{\n                \
                 batch_out(order_out, cap, &order);\n                *n = order.len().min(cap);\n            }}\n            0\n        }}\n        \
                 // SAFETY: Go passes a valid `err`.\n        Err(message) => unsafe {{ fail(err, message) }},\n    }}\n}}\n",
                args = names.join(", "),
            );
            use_custom_rank(&mut out, from, s, &names);
        }
    }
    Ok(out)
}

/// The Go side: a `<Fn>Batch` type per batch in the wrapper package.
pub fn go_wrapper(m: &Manifest, api: &PortApi) -> Result<String, String> {
    let mut out = String::new();
    for g in go_batches(m, api)? {
        let name = &g.e.name;
        let class = &g.s.class;
        let item_go = g.e.params[g.s.item].native_type.clone().unwrap_or_default();
        let items = match g.item.c {
            "string" => format!(
                "\ttotal := 0\n\tfor _, s := range items {{\n\t\ttotal += len(s)\n\t}}\n\
                 \tbytes := make([]byte, 0, total)\n\tlens := make([]C.size_t, len(items))\n\
                 \tfor i, s := range items {{\n\t\tbytes = append(bytes, s...)\n\t\tlens[i] = C.size_t(len(s))\n\t}}\n\
                 \th := C.rust_anything_b_{name}_new(rustAnythingBytes(bytes), C.size_t(len(bytes)), rustAnythingPtr(lens), C.size_t(len(items)))\n"
            ),
            c => {
                let cty = match c {
                    "int64_t" => "C.int64_t",
                    "double" => "C.double",
                    _ => "C.bool",
                };
                format!(
                    "\tvalues := make([]{cty}, len(items))\n\tfor i, v := range items {{\n\t\tvalues[i] = {cty}(v)\n\t}}\n\
                     \th := C.rust_anything_b_{name}_new(rustAnythingPtr(values), C.size_t(len(items)))\n"
                )
            }
        };
        let params: Vec<String> = g
            .fixed
            .iter()
            .map(|(n, go, _)| format!("{n} {go}"))
            .collect();
        let mut args = String::new();
        for (n, _, t) in &g.fixed {
            let _ = write!(
                args,
                ", {}",
                match t.c {
                    "string" => format!("rustAnythingStr({n}), C.size_t(len({n}))"),
                    "int64_t" => format!("C.int64_t({n})"),
                    "double" => format!("C.double({n})"),
                    _ => format!("C.bool({n})"),
                }
            );
        }
        let fail = |zero: &str| {
            if g.fallible {
                format!(
                    "\tif status == 1 {{\n\t\treturn {zero}, errors.New(rustAnythingErr(errp))\n\t}}\n"
                )
            } else {
                "\tif status == 1 {\n\t\tpanic(rustAnythingErr(errp))\n\t}\n".to_string()
            }
        };
        let ok = if g.fallible { ", nil" } else { "" };
        let wrap = |t: &str| {
            if g.fallible {
                format!("([]{t}, error)")
            } else {
                format!("[]{t}")
            }
        };
        let result = &g.result;
        let run_body = match g.result_c {
            "string" => format!(
                "\tvar errp *C.char\n\tvar res unsafe.Pointer\n\tvar total C.size_t\n\
                 \tstatus := C.rust_anything_b_{name}_run(b.h{args}, &res, &total, &errp)\n\
                 \truntime.KeepAlive(b)\n{fail}\
                 \t// Rust writes the results straight into Go memory; each result is a slice of it.\n\
                 \tbuf := make([]byte, int(total))\n\tlens := make([]C.size_t, b.n)\n\
                 \tC.rust_anything_b_take_strings(res, rustAnythingBytes(buf), total, rustAnythingPtr(lens), C.size_t(b.n))\n\
                 \tall := unsafe.String(unsafe.SliceData(buf), len(buf))\n\
                 \tout := make([]{result}, b.n)\n\tat := 0\n\tfor i, n := range lens {{\n\t\tout[i] = {result}(all[at : at+int(n)])\n\t\tat += int(n)\n\t}}\n\
                 \treturn out{ok}\n",
                fail = fail("nil"),
            ),
            c => {
                let cty = match c {
                    "int64_t" => "C.int64_t",
                    "double" => "C.double",
                    _ => "C.bool",
                };
                format!(
                    "\tvar errp *C.char\n\tvalues := make([]{cty}, b.n)\n\
                     \tstatus := C.rust_anything_b_{name}_run(b.h{args}, rustAnythingPtr(values), C.size_t(b.n), &errp)\n\
                     \truntime.KeepAlive(b)\n{fail}\
                     \tout := make([]{result}, b.n)\n\tfor i, v := range values {{\n\t\tout[i] = {result}(v)\n\t}}\n\
                     \treturn out{ok}\n",
                    fail = fail("nil"),
                )
            }
        };
        let rank = if rank_body(g.s.f.ret.trim()).is_some() {
            let mut rank_params = params.clone();
            rank_params.push("min float64".into());
            format!(
                "\n// Rank returns the indexes of the items whose result is at least min, highest first.\n\
                 func (b *{class}) Rank({}) {} {{\n\
                 \tvar errp *C.char\n\torder := make([]C.uint32_t, b.n)\n\tvar n C.size_t\n\
                 \tstatus := C.rust_anything_b_{name}_rank(b.h{args}, C.double(min), rustAnythingPtr(order), C.size_t(b.n), &n, &errp)\n\
                 \truntime.KeepAlive(b)\n{}\
                 \tout := make([]int, int(n))\n\tfor i := range out {{\n\t\tout[i] = int(order[i])\n\t}}\n\treturn out{ok}\n}}\n",
                rank_params.join(", "),
                wrap("int"),
                fail("nil"),
            )
        } else {
            String::new()
        };
        let _ = write!(
            out,
            "\n// {class} runs {name} over many {item} values kept in Rust: built once, then each\n\
             // Run maps them all in one call, on all cores. Safe for concurrent Run and Rank; call\n\
             // Close when done (a finalizer frees it otherwise).\n\
             type {class} struct {{\n\th unsafe.Pointer\n\tn int\n}}\n\n\
             // New{class} copies the items into Rust.\nfunc New{class}(items []{item_go}) *{class} {{\n{items}\
             \tif h == nil {{\n\t\tpanic(\"rust-anything: building {class} failed\")\n\t}}\n\
             \tb := &{class}{{h: h, n: len(items)}}\n\truntime.SetFinalizer(b, (*{class}).Close)\n\treturn b\n}}\n\n\
             // Close frees the items on the Rust side.\nfunc (b *{class}) Close() {{\n\tif b.h != nil {{\n\t\tC.rust_anything_b_{name}_free(b.h)\n\t\tb.h = nil\n\t}}\n}}\n\n\
             // Run returns {name}'s result for every item, in order.\nfunc (b *{class}) Run({params}) {ret} {{\n{run_body}}}\n{rank}",
            item = g.e.params[g.s.item].name,
            params = params.join(", "),
            ret = wrap(result),
        );
    }
    if !out.is_empty() {
        out.insert_str(
            0,
            "\n// rustAnythingPtr is a slice's first element for C; nil when empty.\n\
             func rustAnythingPtr[T any](s []T) *T {\n\tif len(s) == 0 {\n\t\treturn nil\n\t}\n\treturn &s[0]\n}\n\n\
             // rustAnythingBytes is a byte slice for C; nil when empty.\n\
             func rustAnythingBytes(b []byte) *C.uint8_t {\n\tif len(b) == 0 {\n\t\treturn nil\n\t}\n\treturn (*C.uint8_t)(unsafe.Pointer(&b[0]))\n}\n",
        );
    }
    Ok(out)
}

// ---- Java ------------------------------------------------------------------------------------

/// A Java type a batch carries: Java name, JNI scalar, JNI array type, port type, and how a
/// port value becomes the JNI scalar.
struct JavaTy {
    java: &'static str,
    scalar: &'static str,
    array: &'static str,
    port: &'static str,
    /// `new_<x>_array` / `<x>_array_region` in the jni crate.
    region: &'static str,
    /// The conversion of a port value to the JNI scalar, as a function path; `None` when it is the
    /// same type.
    to_jni: Option<&'static str>,
    /// The conversion of a JNI scalar to the port value.
    from_jni: Option<&'static str>,
}

/// `expr` converted by `f`.
fn apply(f: Option<&str>, expr: &str) -> String {
    f.map_or_else(|| expr.to_string(), |f| format!("{f}({expr})"))
}

/// `.map(f)` over an iterator, or nothing.
fn map(f: Option<&str>) -> String {
    f.map(|f| format!(".map({f})")).unwrap_or_default()
}

fn java_ty(java: &str) -> Option<JavaTy> {
    Some(match java.trim() {
        "int" => JavaTy {
            java: "int",
            scalar: "jni::sys::jint",
            array: "jni::objects::JIntArray<'local>",
            port: "i64",
            region: "int",
            to_jni: Some("batch_jint"),
            from_jni: Some("i64::from"),
        },
        "long" => JavaTy {
            java: "long",
            scalar: "jni::sys::jlong",
            array: "jni::objects::JLongArray<'local>",
            port: "i64",
            region: "long",
            to_jni: None,
            from_jni: None,
        },
        "double" => JavaTy {
            java: "double",
            scalar: "jni::sys::jdouble",
            array: "jni::objects::JDoubleArray<'local>",
            port: "f64",
            region: "double",
            to_jni: None,
            from_jni: None,
        },
        "boolean" => JavaTy {
            java: "boolean",
            scalar: "jni::sys::jboolean",
            array: "jni::objects::JBooleanArray<'local>",
            port: "bool",
            region: "boolean",
            to_jni: Some("jni::sys::jboolean::from"),
            from_jni: Some("batch_bool"),
        },
        "String" => JavaTy {
            java: "String",
            scalar: "jni::objects::JString<'local>",
            array: "jni::objects::JObjectArray<'local>",
            port: "String",
            region: "",
            to_jni: None,
            from_jni: None,
        },
        _ => return None,
    })
}

struct JavaBatch<'a> {
    e: &'a ExportedFn,
    s: crate::codegen_batch::Spec<'a>,
    package: String,
    item: JavaTy,
    /// `(Rust name, Java name, type)` of the other parameters.
    fixed: Vec<(String, String, JavaTy)>,
    result: JavaTy,
}

fn java_batches<'a>(m: &'a Manifest, api: &'a PortApi) -> Result<Vec<JavaBatch<'a>>, String> {
    let mut out = Vec::new();
    for b in &m.batches {
        let s = spec_with(b, api, false)?;
        let e = export(m, b)?;
        let ty = |java: &str, what: &str| {
            java_ty(java).ok_or_else(|| {
                format!("Java batches take and return int, long, double, boolean or String; {what} is `{java}`")
            })
        };
        let p = e
            .params
            .get(s.item)
            .ok_or("parameter count differs from port.rs")?;
        let item = ty(
            p.native_type.as_deref().unwrap_or_default(),
            &format!("`{}`", p.name),
        )?;
        let mut fixed = Vec::new();
        for (i, p) in e.params.iter().enumerate().filter(|(i, _)| *i != s.item) {
            let t = ty(
                p.native_type.as_deref().unwrap_or_default(),
                &format!("`{}`", p.name),
            )?;
            fixed.push((s.f.params[i].0.clone(), p.name.clone(), t));
        }
        let result = ty(
            e.native_return.as_deref().unwrap_or("void"),
            &format!("`{}`'s result", e.name),
        )?;
        let (class, _) = crate::lang::java::receiver(e);
        let package = class
            .rsplit_once('.')
            .map(|(p, _)| p.to_string())
            .unwrap_or_default();
        out.push(JavaBatch {
            e,
            s,
            package,
            item,
            fixed,
            result,
        });
    }
    Ok(out)
}

/// The JNI symbol of `method` in `package.class`.
fn jni_symbol(package: &str, class: &str, method: &str) -> String {
    let mangle = crate::codegen_java::mangle_jni_component;
    let mut parts: Vec<String> = package
        .split('.')
        .filter(|p| !p.is_empty())
        .map(mangle)
        .collect();
    parts.push(mangle(class));
    parts.push(mangle(method));
    format!("Java_{}", parts.join("_"))
}

const JAVA_BATCH_RS: &str = r#"
/// A batch's handle as Java holds it.
///
/// # Safety
///
/// `handle` comes from a batch's `nNew` and is not freed yet; Java checks it is not 0.
// SAFETY: unsafe to call; callers uphold the contract above.
unsafe fn batch_ref<'a, T>(handle: jni::sys::jlong) -> &'a T {
    // SAFETY: as documented above.
    unsafe { &*(handle as *const T) }
}

/// A Java `String[]` as strings; `None` with a NullPointerException pending for a null element.
fn batch_strings(env: &mut jni::JNIEnv, items: &jni::objects::JObjectArray) -> Option<Vec<String>> {
    let n = env.get_array_length(items).ok()?;
    let mut out = Vec::with_capacity(usize::try_from(n).unwrap_or(0));
    for i in 0..n {
        let item = jni::objects::JString::from(env.get_object_array_element(items, i).ok()?);
        let value = string_arg(env, &item);
        let _ = env.delete_local_ref(item);
        out.push(value?);
    }
    Some(out)
}

/// String results as one `char[]` of UTF-16, each result's length written to `lens`: Java
/// slices it into strings, instead of one JNI call per string. Encoding and freeing run on all
/// cores.
fn batch_utf16(env: &mut jni::JNIEnv, values: Vec<String>, lens: &jni::objects::JIntArray) -> jni::sys::jcharArray {
    use rayon::prelude::*;
    let sizes: Vec<usize> = values.par_iter().with_min_len(256).map(|v| v.encode_utf16().count()).collect();
    let mut chars = vec![0u16; sizes.iter().sum()];
    let mut rest = chars.as_mut_slice();
    let mut parts = Vec::with_capacity(values.len());
    for n in &sizes {
        let (head, tail) = std::mem::take(&mut rest).split_at_mut(*n);
        parts.push(head);
        rest = tail;
    }
    parts.into_par_iter().zip(values.par_iter()).with_min_len(256).for_each(|(dst, v)| {
        for (d, u) in dst.iter_mut().zip(v.encode_utf16()) {
            *d = u;
        }
    });
    values.into_par_iter().with_min_len(256).for_each(drop);
    let counts: Vec<jni::sys::jint> = sizes.iter().map(|&n| jni::sys::jint::try_from(n).unwrap_or(jni::sys::jint::MAX)).collect();
    if env.set_int_array_region(lens, 0, &counts).is_err() {
        return std::ptr::null_mut();
    }
    let Ok(n) = jni::sys::jsize::try_from(chars.len()) else {
        let _ = env.throw_new("java/lang/OutOfMemoryError", "batch results over 2^31 characters");
        return std::ptr::null_mut();
    };
    match env.new_char_array(n) {
        Ok(array) => {
            let _ = env.set_char_array_region(&array, 0, &chars);
            array.into_raw()
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// A port's integer as a Java `int`, wrapping like a Java cast.
#[allow(dead_code)]
fn batch_jint(v: i64) -> jni::sys::jint {
    v as jni::sys::jint
}

/// A Java `boolean`.
#[allow(dead_code)]
fn batch_bool(v: jni::sys::jboolean) -> bool {
    v != 0
}

/// Indexes as a Java `int[]`.
fn batch_int_array(env: &mut jni::JNIEnv, order: &[u32]) -> jni::sys::jintArray {
    let values: Vec<jni::sys::jint> = order.iter().map(|&i| jni::sys::jint::try_from(i).unwrap_or(jni::sys::jint::MAX)).collect();
    let n = jni::sys::jsize::try_from(values.len()).unwrap_or(jni::sys::jsize::MAX);
    match env.new_int_array(n) {
        Ok(array) => {
            let _ = env.set_int_array_region(&array, 0, &values);
            array.into_raw()
        }
        Err(_) => std::ptr::null_mut(),
    }
}
"#;

/// The batch functions for `lib.rs`.
pub fn java_rs(m: &Manifest, api: &PortApi) -> Result<String, String> {
    let batches = java_batches(m, api)?;
    if batches.is_empty() {
        return Ok(String::new());
    }
    let mut out = JAVA_BATCH_RS.to_string();
    for g in &batches {
        let s = &g.s;
        if !s.custom {
            out.push_str(&default_index(s, true));
        }
        let index = &s.index;
        let sym = |method: &str| jni_symbol(&g.package, &s.class, method);
        let read_items = if g.item.port == "String" {
            "    let Some(items) = batch_strings(&mut env, &items) else {\n        return 0;\n    };\n".to_string()
        } else {
            format!(
                "    let n = env.get_array_length(&items).unwrap_or(0);\n    \
                 let mut raw: Vec<{scalar}> = vec![Default::default(); usize::try_from(n).unwrap_or(0)];\n    \
                 if env.get_{region}_array_region(&items, 0, &mut raw).is_err() {{\n        return 0;\n    }}\n    \
                 let items: Vec<{port}> = raw.into_iter(){from}.collect();\n",
                scalar = g.item.scalar,
                region = g.item.region,
                port = g.item.port,
                from = map(g.item.from_jni),
            )
        };
        let mut params = String::new();
        let mut convert = String::new();
        let mut args = Vec::new();
        for (n, _, t) in &g.fixed {
            let _ = write!(params, ", {n}: {}", t.scalar);
            if t.port == "String" {
                let _ = writeln!(
                    convert,
                    "    let Some({n}) = string_arg(&mut env, &{n}) else {{\n        return std::ptr::null_mut();\n    }};"
                );
            } else {
                if t.from_jni.is_some() {
                    let _ = writeln!(convert, "    let {n} = {};", apply(t.from_jni, n));
                }
            }
            args.push(n.as_str());
        }
        let (array_ty, write) = if g.result.port == "String" {
            (
                "jni::sys::jcharArray".to_string(),
                "batch_utf16(&mut env, values, &lens)".to_string(),
            )
        } else {
            (
                format!("jni::sys::j{}Array", g.result.region),
                format!(
                    "{{\n            let raw: Vec<{scalar}> = values.into_iter(){to}.collect();\n            \
                     let n = jni::sys::jsize::try_from(raw.len()).unwrap_or(jni::sys::jsize::MAX);\n            \
                     match env.new_{region}_array(n) {{\n                Ok(array) => {{\n                    \
                     let _ = env.set_{region}_array_region(&array, 0, &raw);\n                    array.into_raw()\n                }}\n                \
                     Err(_) => std::ptr::null_mut(),\n            }}\n        }}",
                    scalar = g.result.scalar,
                    to = map(g.result.to_jni),
                    region = g.result.region,
                ),
            )
        };
        let _ = write!(
            out,
            "\n/// Builds a `{class}`: the items cross once; Java keeps the handle.\n\
             // SAFETY: the JNI symbol of `{package}.{class}.nNew`; nothing else defines it.\n#[unsafe(no_mangle)]\n\
             pub extern \"system\" fn {new}<'local>(mut env: jni::JNIEnv<'local>, _class: jni::objects::JClass<'local>, items: {item_array}) -> jni::sys::jlong {{\n\
             {read_items}    match std::panic::catch_unwind(|| {index}::new(items)) {{\n        \
             Ok(index) => Box::into_raw(Box::new(index)) as jni::sys::jlong,\n        \
             Err(_) => {{\n            let _ = env.throw_new(\"java/lang/RuntimeException\", \"Rust panic building {class}\");\n            0\n        }}\n    }}\n}}\n\n\
             /// Frees a `{class}`.\n// SAFETY: the JNI symbol of `{package}.{class}.nFree`; nothing else defines it.\n#[unsafe(no_mangle)]\n\
             pub extern \"system\" fn {free}<'local>(_env: jni::JNIEnv<'local>, _class: jni::objects::JClass<'local>, handle: jni::sys::jlong) {{\n    \
             if handle != 0 {{\n        // SAFETY: Java frees each handle from `nNew` once (its Cleaner runs once).\n        \
             drop(unsafe {{ Box::from_raw(handle as *mut {index}) }});\n    }}\n}}\n\n\
             /// `{name}` over every item; errors go to `slot`.\n// SAFETY: the JNI symbol of `{package}.{class}.nRun`; nothing else defines it.\n#[unsafe(no_mangle)]\n\
             pub extern \"system\" fn {run}<'local>(mut env: jni::JNIEnv<'local>, _class: jni::objects::JClass<'local>, handle: jni::sys::jlong{params}{lens_param}, slot: jni::objects::JObjectArray<'local>) -> {array_ty} {{\n\
             {convert}    // SAFETY: Java passes a live handle (it checks for 0 and keeps the batch reachable).\n    \
             let index: &{index} = unsafe {{ batch_ref(handle) }};\n    \
             match run(|| index.run({args})) {{\n        Ok(values) => {write},\n        \
             Err(message) => {{\n            report(&mut env, &slot, &message);\n            std::ptr::null_mut()\n        }}\n    }}\n}}\n",
            class = s.class,
            package = g.package,
            name = g.e.name,
            new = sym("nNew"),
            free = sym("nFree"),
            run = sym("nRun"),
            item_array = g.item.array,
            lens_param = if g.result.port == "String" {
                ", lens: jni::objects::JIntArray<'local>"
            } else {
                ""
            },
            args = args.join(", "),
        );
        if let Some(rank) = rank_for(s) {
            let from = out.len();
            let _ = write!(
                out,
                "\n/// Indexes of the items whose result is at least `min`, best first.\n\
                 // SAFETY: the JNI symbol of `{package}.{class}.nRank`; nothing else defines it.\n#[unsafe(no_mangle)]\n\
                 pub extern \"system\" fn {rank_sym}<'local>(mut env: jni::JNIEnv<'local>, _class: jni::objects::JClass<'local>, handle: jni::sys::jlong{params}, min: jni::sys::jdouble, slot: jni::objects::JObjectArray<'local>) -> jni::sys::jintArray {{\n\
                 {convert}    // SAFETY: Java passes a live handle (it checks for 0 and keeps the batch reachable).\n    \
                 let index: &{index} = unsafe {{ batch_ref(handle) }};\n    \
                 match run(|| index.run({args})) {{\n        Ok(out) => {{\n            {rank}\n            batch_int_array(&mut env, &order)\n        }}\n        \
                 Err(message) => {{\n            report(&mut env, &slot, &message);\n            std::ptr::null_mut()\n        }}\n    }}\n}}\n",
                class = s.class,
                package = g.package,
                rank_sym = sym("nRank"),
                args = args.join(", "),
            );
            use_custom_rank(&mut out, from, s, &args);
        }
    }
    Ok(out)
}

/// The Java classes: `(path, source)`, one `<Fn>Batch` per batch, beside the original class.
pub fn java_files(m: &Manifest, api: &PortApi) -> Result<Vec<(String, String)>, String> {
    let native = crate::codegen_ffi::java_native_class(m);
    let mut files = Vec::new();
    for g in java_batches(m, api)? {
        let class = &g.s.class;
        let item = &g.e.params[g.s.item].name;
        let item_java = g.item.java;
        let params: Vec<String> = g
            .fixed
            .iter()
            .map(|(_, n, t)| format!("{} {n}", t.java))
            .collect();
        let native_params: String = g
            .fixed
            .iter()
            .map(|(_, n, t)| format!(", {} {n}", t.java))
            .collect();
        let args: String = g.fixed.iter().map(|(_, n, _)| format!(", {n}")).collect();
        let list = if item_java == "String" {
            format!(
                "\n    /** Copies the items into Rust. */\n    public {class}(java.util.List<String> {item}s) {{\n        this({item}s.toArray(new String[0]));\n    }}\n"
            )
        } else {
            String::new()
        };
        let rank = if rank_body(g.s.f.ret.trim()).is_some() {
            let mut rank_params = params.clone();
            rank_params.push("double min".into());
            format!(
                "\n    /** Indexes of the items whose result is at least {{@code min}}, highest result first. */\n    \
                 public int[] rank({}) {{\n        String[] slot = RustAnythingError.slot();\n        \
                 int[] out = nRank(handle(){args}, min, slot);\n        RustAnythingError.check(slot);\n        \
                 java.lang.ref.Reference.reachabilityFence(this);\n        return out;\n    }}\n\n    \
                 private static native int[] nRank(long handle{native_params}, double min, String[] slot);\n",
                rank_params.join(", ")
            )
        } else {
            String::new()
        };
        let strings = g.result.java == "String";
        let run_body = if strings {
            format!(
                "        String[] slot = RustAnythingError.slot();\n        int[] lens = new int[count];\n        \
                 char[] chars = nRun(handle(){args}, lens, slot);\n        RustAnythingError.check(slot);\n        \
                 java.lang.ref.Reference.reachabilityFence(this);\n        \
                 // One array from Rust; each result is a slice of it.\n        \
                 String[] out = new String[count];\n        int at = 0;\n        for (int i = 0; i < count; i++) {{\n            \
                 out[i] = new String(chars, at, lens[i]);\n            at += lens[i];\n        }}\n        return out;\n"
            )
        } else {
            format!(
                "        String[] slot = RustAnythingError.slot();\n        {ret}[] out = nRun(handle(){args}, slot);\n        \
                 RustAnythingError.check(slot);\n        java.lang.ref.Reference.reachabilityFence(this);\n        return out;\n",
                ret = g.result.java
            )
        };
        let mut src = String::new();
        if !g.package.is_empty() {
            let _ = writeln!(src, "package {};\n", g.package);
        }
        let _ = write!(
            src,
            "import rustanything.RustAnythingError;\n\n\
             /**\n * {{@code {name}}} over many {{@code {item}}} values kept in Rust, generated by rust-anything: built once,\n \
             * then each {{@link #run}} maps them all in one call, on all cores. Safe for concurrent use;\n \
             * close it when done (a Cleaner frees it otherwise).\n */\n\
             public final class {class} implements AutoCloseable {{\n    \
             static {{\n        try {{\n            Class.forName(\"rustanything.{native}\");\n        }} catch (ClassNotFoundException e) {{\n            \
             throw new ExceptionInInitializerError(e);\n        }}\n    }}\n\n    \
             private static final java.lang.ref.Cleaner CLEANER = java.lang.ref.Cleaner.create();\n\n    \
             /** The Rust handle; freed once, by {{@link #close}} or the Cleaner. */\n    \
             private static final class State implements Runnable {{\n        private long handle;\n\n        \
             State(long handle) {{\n            this.handle = handle;\n        }}\n\n        \
             @Override\n        public synchronized void run() {{\n            if (handle != 0) {{\n                nFree(handle);\n                handle = 0;\n            }}\n        }}\n\n        \
             synchronized long get() {{\n            if (handle == 0) {{\n                throw new IllegalStateException(\"{class} is closed\");\n            }}\n            return handle;\n        }}\n    }}\n\n    \
             private final State state;\n    private final java.lang.ref.Cleaner.Cleanable cleanable;\n    private final int count;\n\n    \
             /** Copies the items into Rust. */\n    public {class}({item_java}[] {item}s) {{\n        \
             state = new State(nNew({item}s));\n        count = {item}s.length;\n        cleanable = CLEANER.register(this, state);\n    }}\n{list}\n    \
             private long handle() {{\n        return state.get();\n    }}\n\n    \
             /** {{@code {name}}}'s result for every item, in order. */\n    public {ret}[] run({params}) {{\n{run_body}    }}\n{rank}\n    \
             /** Frees the items on the Rust side. */\n    @Override\n    public void close() {{\n        cleanable.clean();\n    }}\n\n    \
             private static native long nNew({item_java}[] items);\n\n    \
             private static native void nFree(long handle);\n\n    \
             private static native {native_ret}[] nRun(long handle{native_params}{lens}, String[] slot);\n}}\n",
            name = g.e.name,
            ret = g.result.java,
            params = params.join(", "),
            native_ret = if strings { "char" } else { g.result.java },
            lens = if strings { ", int[] lens" } else { "" },
        );
        let dir = if g.package.is_empty() {
            "java".to_string()
        } else {
            format!("java/{}", g.package.replace('.', "/"))
        };
        files.push((format!("{dir}/{class}.java"), src));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruby_params() {
        assert_eq!(
            split_params("text, sep = \"-,\", max: nil, opts = {a: 1, b: 2}"),
            ["text", "sep = \"-,\"", "max: nil", "opts = {a: 1, b: 2}"]
        );
        assert_eq!(param_name("max: nil"), "max");
        assert_eq!(param_name("*rest"), "rest");
        assert_eq!(param_name("sep = \"-\""), "sep");
    }
}
