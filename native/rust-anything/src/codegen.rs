//! Code generation for a bridge crate: port stubs from TS signatures, and the binding layer
//! (`lib.rs`, `index.cjs`) from the port's Rust signatures.
//!
//! The binding layer follows `port.rs`, not the TS: change a port's types, run
//! `rust-anything refresh`, and the bindings and loader follow. Values cross three ways:
//!
//! - native: strings, numbers, booleans and lists or options of those, with no extra cost;
//! - JSON text: anything structured, as one string each way (`JSON.parse` is fast in V8), with
//!   markers for what JSON lacks (`undefined`, NaN, lone surrogates; see `js.rs`);
//! - UTF-16 units: a `Vec<u16>` return, for strings that may hold lone surrogates.

use crate::MIN_RUST;
use std::fmt::Write as _;

use crate::bridge::{Kind, Manifest, RType, param_type, snake_case};
use crate::ts::exports::ExportedFn;

/// A port function, as declared in `port.rs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortFn {
    pub js_name: String,
    pub rust_name: String,
    pub params: Vec<(String, String)>,
    /// The success type, `()` for none.
    pub ret: String,
    /// Returns `Result<T, String>`; an `Err` becomes a thrown JS error.
    pub fallible: bool,
    pub is_async: bool,
}

/// A port class: a struct with `new` and methods taking `&self` or `&mut self`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortClass {
    pub js_name: String,
    pub rust_name: String,
    pub constructor: PortFn,
    pub methods: Vec<(PortFn, bool)>,
}

/// What `port.rs` offers to JS. `methods` pairs each method with whether it takes `&mut self`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PortApi {
    pub functions: Vec<PortFn>,
    pub classes: Vec<PortClass>,
    /// Every `pub struct` of `port.rs` (a batch uses `<Name>Index` when the port defines one).
    pub structs: Vec<String>,
    /// The structs whose `impl` has a `pub fn rank` (an index that ranks by itself).
    pub ranked: Vec<String>,
}

impl PortApi {
    /// Every function and method, for dependency and wire checks.
    pub fn all(&self) -> impl Iterator<Item = &PortFn> {
        self.functions.iter().chain(
            self.classes.iter().flat_map(|c| {
                std::iter::once(&c.constructor).chain(c.methods.iter().map(|(m, _)| m))
            }),
        )
    }

    pub fn needs_json(&self) -> bool {
        self.all().any(|f| {
            wire(&f.ret) == Wire::Json || f.params.iter().any(|(_, t)| wire(t) == Wire::Json)
        })
    }
}

/// How a Rust type crosses the bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wire {
    Native,
    Json,
    Units,
}

pub fn wire(ty: &str) -> Wire {
    let ty: String = ty.chars().filter(|c| !c.is_whitespace()).collect();
    match ty.as_str() {
        "String" | "f64" | "i64" | "bool" | "()" | "Vec<String>" | "Vec<f64>" | "Vec<i64>"
        | "Option<String>" | "Option<f64>" | "Option<i64>" | "Option<bool>" => Wire::Native,
        "Vec<u16>" => Wire::Units,
        _ => Wire::Json,
    }
}

// ---- port.rs stubs ---------------------------------------------------------------------------

/// `port.rs` with one stub per export, typed from the TS signature.
pub fn port_rs(m: &Manifest) -> String {
    let mut out = format!(
        "//! The Rust port of `{}`.\n//!\n\
         //! Replace each `unimplemented!` with the port of the source function. Return `Err` where\n\
         //! the source throws or raises: the caller gets an error there too. After changing a\n\
         //! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks\n\
         //! behavior against what the source recorded.\n\n\
         #![forbid(unsafe_code)]\n",
        m.unit
    );
    for f in &m.exports {
        let params = f
            .params
            .iter()
            .map(|p| format!("{}: {}", snake_case(&p.name), port_param_type(m, p).rust()))
            .collect::<Vec<_>>()
            .join(", ");
        let ret = match RType::from_ts(&f.returns) {
            RType::Unit => "()".to_string(),
            t => t.rust(),
        };
        let names: Vec<String> = f.params.iter().map(|p| snake_case(&p.name)).collect();
        let asyncness = if is_async(f) { "async " } else { "" };
        out.push('\n');
        let _ = writeln!(out, "/// Port of `{}` (`{}:{}`).", f.name, f.file, f.line);
        for p in f.params.iter().filter(|p| p.default.is_some()) {
            let _ = writeln!(
                out,
                "///\n/// `{}` defaults to `{}` in the source.",
                snake_case(&p.name),
                p.default.as_deref().unwrap_or_default()
            );
        }
        let _ = writeln!(
            out,
            "pub {asyncness}fn {}({params}) -> Result<{ret}, String> {{",
            snake_case(&f.name)
        );
        match names.len() {
            0 => {}
            1 => {
                let _ = writeln!(out, "    let _ = &{};", names[0]);
            }
            _ => {
                let refs: Vec<String> = names.iter().map(|n| format!("&{n}")).collect();
                let _ = writeln!(out, "    let _ = ({});", refs.join(", "));
            }
        }
        let _ = writeln!(
            out,
            "    unimplemented!(\"port `{}` from {}\")",
            f.name, f.file
        );
        out.push_str("}\n");
    }
    for c in &m.classes {
        let _ = writeln!(
            out,
            "\n/// Port of class `{}` (`{}:{}`): holds the instance state.",
            c.name, c.file, c.line
        );
        let _ = writeln!(out, "pub struct {} {{}}\n", c.name);
        let _ = writeln!(out, "impl {} {{", c.name);
        let ctor = stub_params(m, &c.constructor);
        let _ = writeln!(
            out,
            "    /// Port of `{}` constructor (`{}:{}`).",
            c.name, c.file, c.line
        );
        let _ = writeln!(out, "    pub fn new({}) -> Result<Self, String> {{", ctor.0);
        let _ = write!(out, "{}", ctor.1);
        let _ = writeln!(
            out,
            "        unimplemented!(\"port the `{}` constructor\")\n    }}",
            c.name
        );
        for f in &c.methods {
            let (params, unused) = stub_params(m, &f.params);
            let ret = match RType::from_ts(&f.returns) {
                RType::Unit => "()".to_string(),
                t => t.rust(),
            };
            let receiver = if params.is_empty() {
                "&mut self".to_string()
            } else {
                format!("&mut self, {params}")
            };
            let _ = writeln!(
                out,
                "\n    /// Port of `{}.{}` (`{}:{}`).",
                c.name, f.name, f.file, f.line
            );
            let _ = writeln!(
                out,
                "    pub fn {}({receiver}) -> Result<{ret}, String> {{",
                snake_case(&f.name)
            );
            let _ = write!(out, "{unused}");
            let _ = writeln!(
                out,
                "        unimplemented!(\"port `{}.{}`\")\n    }}",
                c.name, f.name
            );
        }
        out.push_str("}\n");
    }
    out
}

/// A parameter's type in the port. Python parameters with a literal default (`sep="-"`) are
/// plain values: PyO3 fills in the default, and an explicit `None` stays an error, as in Python.
fn port_param_type(m: &Manifest, p: &crate::ts::exports::Param) -> RType {
    if matches!(
        m.language,
        crate::lang::Lang::Python | crate::lang::Lang::Ruby
    ) && crate::codegen_py::has_literal_default(p)
    {
        RType::from_ts(&p.ts_type)
    } else {
        param_type(p)
    }
}

/// Parameters of a method stub, and a line marking them used.
fn stub_params(m: &Manifest, params: &[crate::ts::exports::Param]) -> (String, String) {
    let list = params
        .iter()
        .map(|p| format!("{}: {}", snake_case(&p.name), port_param_type(m, p).rust()))
        .collect::<Vec<_>>()
        .join(", ");
    let names: Vec<String> = params
        .iter()
        .map(|p| format!("&{}", snake_case(&p.name)))
        .collect();
    let unused = match names.len() {
        0 => String::new(),
        1 => format!("        let _ = {};\n", names[0]),
        _ => format!("        let _ = ({});\n", names.join(", ")),
    };
    (list, unused)
}

pub fn is_async(f: &ExportedFn) -> bool {
    f.is_async || f.returns.trim().starts_with("Promise<")
}

// ---- reading port.rs -------------------------------------------------------------------------

/// The functions and classes `port.rs` offers. A function's JS name comes from its
/// `/// Port of `name`` doc line (else its Rust name in camelCase); a class is a `pub struct`
/// documented `/// Port of class `Name``, with an `impl` holding `new` and its methods.
pub fn parse_port(source: &str) -> PortApi {
    let lines: Vec<&str> = source.lines().collect();
    let mut api = PortApi {
        functions: functions_in(&lines, ""),
        classes: Vec::new(),
        structs: lines
            .iter()
            .filter_map(|l| l.trim_start().strip_prefix("pub struct "))
            .map(|r| {
                r.chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect()
            })
            .collect(),
        ranked: Vec::new(),
    };
    // `impl Name {` ... `}` at column 0: does it define `pub fn rank`?
    for (i, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("impl ") else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let body = lines[i + 1..].iter().take_while(|l| !l.starts_with('}'));
        if body
            .into_iter()
            .any(|l| l.trim_start().starts_with("pub fn rank("))
        {
            api.ranked.push(name);
        }
    }
    for (i, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("pub struct ") else {
            continue;
        };
        let rust_name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let Some(js_name) = doc_name(&lines[..i], "Port of class `") else {
            continue;
        };
        // The `impl Name {` block, up to its closing `}` at column 0.
        let Some(start) = lines
            .iter()
            .position(|l| l.trim_end() == format!("impl {rust_name} {{"))
        else {
            continue;
        };
        let end = lines[start..]
            .iter()
            .position(|l| *l == "}")
            .map_or(lines.len(), |e| start + e);
        let block = &lines[start + 1..end];
        let mut constructor = None;
        let mut methods = Vec::new();
        for f in functions_in(block, "    ") {
            if f.rust_name == "new" {
                constructor = Some(f);
            } else {
                let signature_line = block
                    .iter()
                    .find(|l| l.contains(&format!("fn {}(", f.rust_name)))
                    .copied()
                    .unwrap_or("");
                let block_text = block.join("\n");
                let mutable = signature_line.contains("&mut self") || {
                    let at = block_text
                        .find(&format!("fn {}(", f.rust_name))
                        .unwrap_or(0);
                    block_text[at..]
                        .split('{')
                        .next()
                        .unwrap_or("")
                        .contains("&mut self")
                };
                methods.push((f, mutable));
            }
        }
        if let Some(constructor) = constructor {
            api.classes.push(PortClass {
                js_name,
                rust_name,
                constructor,
                methods,
            });
        }
    }
    api
}

fn doc_name(before: &[&str], prefix: &str) -> Option<String> {
    before
        .iter()
        .rev()
        .take_while(|l| l.trim_start().starts_with("///") || l.trim_start().starts_with("#["))
        .find_map(|l| {
            let doc = l.trim_start().strip_prefix("///")?.trim();
            let rest = doc.strip_prefix(prefix)?;
            Some(rest.split('`').next()?.to_string())
        })
}

/// `pub fn` items at one indentation level. In an `impl`, `Port of `Class.method`` names the
/// method `method`.
fn functions_in(lines: &[&str], indent: &str) -> Vec<PortFn> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].strip_prefix(indent).unwrap_or("");
        let (is_async, rest) = if let Some(rest) = line.strip_prefix("pub async fn ") {
            (true, rest)
        } else if let Some(rest) = line.strip_prefix("pub fn ") {
            (false, rest)
        } else {
            i += 1;
            continue;
        };
        // The signature runs to the `{` that opens the body, possibly over several lines.
        let mut signature = rest.to_string();
        let mut j = i;
        while !balanced_open_brace(&signature) && j + 1 < lines.len() {
            j += 1;
            signature.push(' ');
            signature.push_str(lines[j].trim());
        }
        let js_name = doc_name(&lines[..i], "Port of `")
            .map(|n| n.rsplit('.').next().unwrap_or(&n).to_string());
        if let Some(f) = parse_signature(&signature, is_async, js_name) {
            out.push(f);
        }
        i = j + 1;
    }
    out
}

/// Whether the text reaches the body's `{` outside parentheses and angle brackets.
fn balanced_open_brace(text: &str) -> bool {
    let mut depth = 0i32;
    let mut previous = ' ';
    for c in text.chars() {
        let arrow = c == '>' && previous == '-';
        previous = c;
        match c {
            '(' | '<' | '[' => depth += 1,
            // `->` is not a closing bracket.
            '>' if arrow => {}
            ')' | '>' | ']' => depth -= 1,
            '{' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

fn parse_signature(signature: &str, is_async: bool, js_name: Option<String>) -> Option<PortFn> {
    let open = signature.find('(')?;
    let rust_name = signature[..open].trim().to_string();
    let mut depth = 0i32;
    let mut close = None;
    for (i, c) in signature.char_indices().skip(open) {
        match c {
            '(' | '<' | '[' => depth += 1,
            ')' | '>' | ']' => {
                depth -= 1;
                if depth == 0 && c == ')' {
                    close = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    let params = split_top(&signature[open + 1..close])
        .into_iter()
        .filter(|p| !p.trim().is_empty())
        .filter_map(|p| {
            let (name, ty) = p.split_once(':')?;
            Some((
                name.trim().trim_start_matches("mut ").to_string(),
                ty.trim().to_string(),
            ))
        })
        .collect();
    let after = &signature[close + 1..];
    let body = after.find('{').unwrap_or(after.len());
    let ret_text = after[..body].trim();
    let ret_text = ret_text.strip_prefix("->").map_or("()", str::trim);
    let (ret, fallible) = match ret_text
        .strip_prefix("Result<")
        .and_then(|r| r.strip_suffix('>'))
    {
        Some(inner) => (
            split_top(inner)
                .first()
                .map_or("()", |t| t.trim())
                .to_string(),
            true,
        ),
        None => (ret_text.to_string(), false),
    };
    Some(PortFn {
        js_name: js_name.unwrap_or_else(|| camel_case(&rust_name)),
        rust_name,
        params,
        ret,
        fallible,
        is_async,
    })
}

fn split_top(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in text.char_indices() {
        match c {
            '(' | '<' | '[' => depth += 1,
            ')' | '>' | ']' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&text[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&text[start..]);
    out
}

fn camel_case(name: &str) -> String {
    let name = name.trim_start_matches("r#");
    let mut out = String::new();
    let mut upper = false;
    for c in name.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

// ---- lib.rs ----------------------------------------------------------------------------------

pub struct Modules {
    pub utf16: bool,
    pub js: bool,
}

pub fn lib_rs(m: &Manifest, api: &PortApi, modules: &Modules) -> String {
    if m.kind == Kind::Pyo3 {
        return crate::codegen_py::lib_rs(m, api, modules);
    }
    let mut out = format!(
        "//! JS bindings for `{}`: generated from the signatures in `port.rs` by rust-anything.\n\
         //! Regenerate with `rust-anything refresh` after changing them.\n\n",
        m.unit
    );
    out.push_str(match m.kind {
        Kind::Wasm => "use wasm_bindgen::prelude::*;\n\n",
        _ => "use napi_derive::napi;\n\n",
    });
    if modules.js {
        out.push_str("pub mod js;\n");
    }
    out.push_str("pub mod port;\n");
    if modules.utf16 {
        out.push_str("pub mod utf16;\n");
    }
    if m.kind == Kind::Napi {
        out.push_str(NAPI_RUN);
        if api.all().any(|f| f.is_async) {
            out.push_str(NAPI_RUN_ASYNC);
        }
    }
    for f in &api.functions {
        out.push('\n');
        let _ = writeln!(out, "/// `{}`.", f.js_name);
        bind(&mut out, m.kind, f);
    }
    for c in &api.classes {
        bind_class(&mut out, m.kind, c);
    }
    out
}

/// The body of a binding: decode structured arguments, call, encode the result.
fn body(f: &PortFn, target: &str, kind: Kind, constructor: bool) -> String {
    let mut body = String::new();
    for (n, t) in &f.params {
        // The port's own type names may only resolve inside `port.rs`: let inference pick the type.
        if wire(t) == Wire::Json {
            let _ = writeln!(body, "        let {n} = crate::js::from_json(&{n})?;");
        }
    }
    let args: Vec<&str> = f.params.iter().map(|(n, _)| n.as_str()).collect();
    let call = format!(
        "{target}({}){}{}",
        args.join(", "),
        if f.is_async { ".await" } else { "" },
        if f.fallible { "?" } else { "" }
    );
    // A unit result: call it, then return `()` (`let out = ()` is a clippy error).
    if !constructor && f.ret.trim() == "()" {
        let _ = writeln!(body, "        {call};\n        Ok(())");
        return body;
    }
    let _ = writeln!(body, "        let out = {call};");
    let result = match (constructor, wire(&f.ret), kind) {
        (true, _, Kind::Napi) => "Ok(Self { inner: out })",
        (true, _, Kind::Wasm) => "Ok(out)",
        (false, Wire::Json, _) => "crate::js::to_json(&out)",
        (false, Wire::Units, Kind::Napi) => "Ok(napi::bindgen_prelude::Uint16Array::new(out))",
        (false, Wire::Native, Kind::Napi) if option_inner(&f.ret).is_some() => {
            "Ok(out.map_or(napi::bindgen_prelude::Either::B(()), napi::bindgen_prelude::Either::A))"
        }
        _ => "Ok(out)",
    };
    let _ = writeln!(body, "        {result}");
    body
}

fn binding_params(f: &PortFn) -> Vec<String> {
    f.params
        .iter()
        .map(|(n, t)| match wire(t) {
            Wire::Json => format!("{n}: String"),
            _ => format!("{n}: {t}"),
        })
        .collect()
}

fn binding_ret(f: &PortFn, kind: Kind) -> String {
    match (wire(&f.ret), kind) {
        (Wire::Json, _) => "String".to_string(),
        (Wire::Units, Kind::Napi) => "napi::bindgen_prelude::Uint16Array".to_string(),
        // napi turns `None` into `null`; JS code returning nothing gives `undefined`.
        (Wire::Native, Kind::Napi) if option_inner(&f.ret).is_some() => {
            format!(
                "napi::bindgen_prelude::Either<{}, ()>",
                option_inner(&f.ret).unwrap_or_default()
            )
        }
        _ => f.ret.clone(),
    }
}

/// `T` of `Option<T>`.
fn option_inner(ty: &str) -> Option<String> {
    let ty: String = ty.chars().filter(|c| !c.is_whitespace()).collect();
    ty.strip_prefix("Option<")?
        .strip_suffix('>')
        .map(str::to_string)
}

fn bind_class(out: &mut String, kind: Kind, c: &PortClass) {
    let (name, js) = (&c.rust_name, &c.js_name);
    let snake = snake_case(name);
    let ctor_params = binding_params(&c.constructor).join(", ");
    let ctor_args: Vec<&str> = c
        .constructor
        .params
        .iter()
        .map(|(n, _)| n.as_str())
        .collect();
    match kind {
        Kind::Napi => {
            let _ = writeln!(
                out,
                "\n/// Class `{js}`.\n#[napi(js_name = \"{js}\")]\npub struct {name} {{\n    inner: port::{name},\n}}\n"
            );
            let _ = writeln!(out, "#[napi]\nimpl {name} {{");
            let _ = writeln!(
                out,
                "    #[napi(constructor)]\n    pub fn new({ctor_params}) -> napi::Result<Self> {{"
            );
            let _ = writeln!(
                out,
                "        run(move || {{\n{}        }})\n    }}",
                indent(&body(
                    &c.constructor,
                    &format!("port::{name}::new"),
                    kind,
                    true
                ))
            );
            for (m, mutable) in &c.methods {
                let receiver = if *mutable { "&mut self" } else { "&self" };
                let params = binding_params(m);
                let sig = if params.is_empty() {
                    receiver.to_string()
                } else {
                    format!("{receiver}, {}", params.join(", "))
                };
                let _ = writeln!(out, "\n    #[napi(js_name = \"{}\")]", m.js_name);
                let _ = writeln!(
                    out,
                    "    pub fn {}({sig}) -> napi::Result<{}> {{",
                    m.rust_name,
                    binding_ret(m, kind)
                );
                let borrow = if *mutable {
                    "&mut self.inner"
                } else {
                    "&self.inner"
                };
                let _ = writeln!(out, "        let inner = {borrow};");
                let _ = writeln!(
                    out,
                    "        run(move || {{\n{}        }})\n    }}",
                    indent(&body(m, &format!("inner.{}", m.rust_name), kind, false))
                );
            }
            out.push_str("}\n");
        }
        // PyO3 bindings come from `codegen_py`.
        Kind::Wasm | Kind::Pyo3 | Kind::CAbi | Kind::Ffi => {
            let _ = writeln!(
                out,
                "\nfn {snake}_new_inner({ctor_params}) -> Result<port::{name}, String> {{\n{}}}",
                dedent(&body(
                    &c.constructor,
                    &format!("port::{name}::new"),
                    kind,
                    true
                ))
            );
            for (m, mutable) in &c.methods {
                let params = binding_params(m);
                let receiver = if *mutable {
                    format!("inner: &mut port::{name}")
                } else {
                    format!("inner: &port::{name}")
                };
                let sig = std::iter::once(receiver)
                    .chain(params)
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = writeln!(
                    out,
                    "\nfn {snake}_{}_inner({sig}) -> Result<{}, String> {{\n{}}}",
                    m.rust_name.trim_start_matches("r#"),
                    binding_ret(m, kind),
                    dedent(&body(m, &format!("inner.{}", m.rust_name), kind, false))
                );
            }
            let _ = writeln!(
                out,
                "\n/// Class `{js}`.\n#[wasm_bindgen(js_name = \"{js}\")]\npub struct {name} {{\n    inner: port::{name},\n}}\n"
            );
            let _ = writeln!(out, "#[wasm_bindgen(js_class = \"{js}\")]\nimpl {name} {{");
            let _ = writeln!(
                out,
                "    #[wasm_bindgen(constructor)]\n    pub fn new({ctor_params}) -> Result<{name}, JsError> {{"
            );
            let _ = writeln!(
                out,
                "        let inner = {snake}_new_inner({}).map_err(|e| JsError::new(&e))?;\n        Ok(Self {{ inner }})\n    }}",
                ctor_args.join(", ")
            );
            for (m, mutable) in &c.methods {
                let receiver = if *mutable { "&mut self" } else { "&self" };
                let params = binding_params(m);
                let sig = if params.is_empty() {
                    receiver.to_string()
                } else {
                    format!("{receiver}, {}", params.join(", "))
                };
                let args: Vec<&str> = m.params.iter().map(|(n, _)| n.as_str()).collect();
                let borrow = if *mutable {
                    "&mut self.inner"
                } else {
                    "&self.inner"
                };
                let call_args = std::iter::once(borrow)
                    .chain(args)
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = writeln!(out, "\n    #[wasm_bindgen(js_name = \"{}\")]", m.js_name);
                let _ = writeln!(
                    out,
                    "    pub fn {}({sig}) -> Result<{}, JsError> {{",
                    m.rust_name,
                    binding_ret(m, kind)
                );
                let _ = writeln!(
                    out,
                    "        {snake}_{}_inner({call_args}).map_err(|e| JsError::new(&e))\n    }}",
                    m.rust_name.trim_start_matches("r#")
                );
            }
            out.push_str("}\n");
        }
    }
}

fn indent(text: &str) -> String {
    text.lines().map(|l| format!("    {l}\n")).collect()
}

fn dedent(text: &str) -> String {
    text.lines()
        .map(|l| format!("{}\n", l.strip_prefix("    ").unwrap_or(l)))
        .collect()
}

fn bind(out: &mut String, kind: Kind, f: &PortFn) {
    let params = binding_params(f).join(", ");
    let ret = binding_ret(f, kind);
    let body = body(f, &format!("port::{}", f.rust_name), kind, false);
    let args: Vec<&str> = f.params.iter().map(|(n, _)| n.as_str()).collect();
    let asyncness = if f.is_async { "async " } else { "" };
    match kind {
        Kind::Napi => {
            let _ = writeln!(out, "#[napi(js_name = \"{}\")]", f.js_name);
            let _ = writeln!(
                out,
                "pub {asyncness}fn {}({params}) -> napi::Result<{ret}> {{",
                f.rust_name
            );
            if f.is_async {
                let _ = writeln!(
                    out,
                    "    run_async(async move {{\n{body}    }})\n    .await"
                );
            } else {
                let _ = writeln!(out, "    run(move || {{\n{body}    }})");
            }
            out.push_str("}\n");
        }
        // PyO3 bindings come from `codegen_py`.
        Kind::Wasm | Kind::Pyo3 | Kind::CAbi | Kind::Ffi => {
            // `?` needs a function boundary: the body lives in a private helper.
            let inner = format!("{}_inner", f.rust_name.trim_start_matches("r#"));
            let _ = writeln!(
                out,
                "{asyncness}fn {inner}({params}) -> Result<{ret}, String> {{\n{}}}\n",
                dedent(&body)
            );
            let _ = writeln!(out, "#[wasm_bindgen(js_name = \"{}\")]", f.js_name);
            let _ = writeln!(
                out,
                "pub {asyncness}fn {}({params}) -> Result<{ret}, JsError> {{",
                f.rust_name
            );
            let _ = writeln!(
                out,
                "    {inner}({}){}.map_err(|e| JsError::new(&e))",
                args.join(", "),
                if f.is_async { ".await" } else { "" }
            );
            out.push_str("}\n");
        }
    }
}

/// napi: a panic must not unwind into Node (it aborts the process, and in shadow mode the
/// app), and `Err` becomes a JS error.
const NAPI_RUN: &str = r#"
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

#[allow(dead_code)]
fn run<T>(f: impl FnOnce() -> Result<T, String>) -> napi::Result<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!("Rust panic: {}", panic_message(&*payload)))),
    }
}

"#;

const NAPI_RUN_ASYNC: &str = r#"
#[allow(dead_code)]
async fn run_async<T>(f: impl std::future::Future<Output = Result<T, String>>) -> napi::Result<T> {
    use futures::FutureExt as _;
    match std::panic::AssertUnwindSafe(f).catch_unwind().await {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!("Rust panic: {}", panic_message(&*payload)))),
    }
}
"#;

// ---- index.cjs -------------------------------------------------------------------------------

fn spec(f: &PortFn) -> (String, &'static str) {
    let args: Vec<&str> = f
        .params
        .iter()
        .map(|(_, t)| if wire(t) == Wire::Json { "1" } else { "0" })
        .collect();
    let ret = match wire(&f.ret) {
        Wire::Json => "'json'",
        Wire::Units => "'units'",
        Wire::Native => "'raw'",
    };
    (format!("[{}]", args.join(", ")), ret)
}

fn needs_wrap(f: &PortFn) -> bool {
    wire(&f.ret) != Wire::Native || f.params.iter().any(|(_, t)| wire(t) == Wire::Json)
}

pub fn index_cjs(m: &Manifest, api: &PortApi) -> String {
    let load = match m.kind {
        Kind::Napi => format!(
            "// Loads the native module built by `rust-anything build`.\nconst native = require('./{}.node')\n",
            m.lib_name()
        ),
        Kind::Wasm | Kind::Pyo3 | Kind::CAbi | Kind::Ffi => format!(
            "// Loads the wasm-bindgen (nodejs target) output built by `rust-anything build`.\nconst native = require('./pkg/{}.js')\n",
            m.lib_name()
        ),
    };
    let functions: Vec<String> = api
        .functions
        .iter()
        .filter(|f| needs_wrap(f))
        .map(|f| {
            let (args, ret) = spec(f);
            format!("  {}: [{args}, {ret}, {}],", f.js_name, f.is_async)
        })
        .collect();
    let classes: Vec<String> = api
        .classes
        .iter()
        .filter(|c| needs_wrap(&c.constructor) || c.methods.iter().any(|(m, _)| needs_wrap(m)))
        .map(|c| {
            let methods: Vec<String> = c
                .methods
                .iter()
                .map(|(m, _)| {
                    let (args, ret) = spec(m);
                    format!("{}: [{args}, {ret}]", m.js_name)
                })
                .collect();
            format!(
                "  {}: [{}, {{ {} }}],",
                c.js_name,
                spec(&c.constructor).0,
                methods.join(", ")
            )
        })
        .collect();
    if functions.is_empty() && classes.is_empty() {
        return format!("{load}module.exports = native\n");
    }
    format!(
        "{load}{CODEC_CJS}\nmodule.exports = {{ ...native }}\nconst specs = {{\n{}\n}}\nconst classes = {{\n{}\n}}\n{WRAP_CJS}",
        functions.join("\n"),
        classes.join("\n")
    )
}

/// The last layer of `index.cjs`: errors keep their JS class (`TypeError: x` from Rust throws a
/// `TypeError`), panics carry `code: 'RUST_PANIC'`, async exports reject instead of throwing, and
/// a string a Rust `String` cannot hold (a lone surrogate) is refused instead of altered.
/// The parameters a Rust `String` holds, by position: a lone surrogate is refused there.
pub(crate) fn string_params(f: &PortFn) -> Vec<usize> {
    f.params
        .iter()
        .enumerate()
        .filter(|(_, (_, t))| {
            let t: String = t.chars().filter(|c| !c.is_whitespace()).collect();
            matches!(t.as_str(), "String" | "Option<String>" | "Vec<String>")
        })
        .map(|(i, _)| i)
        .collect()
}

/// `wired` functions already check their arguments and errors in their wire wrapper.
pub fn guard_cjs(api: &PortApi, wired: &[String]) -> String {
    if api.functions.is_empty() {
        return String::new();
    }
    let guards: Vec<String> = api
        .functions
        .iter()
        .filter(|f| !wired.contains(&f.js_name))
        .map(|f| {
            let strings: Vec<String> = string_params(f).iter().map(ToString::to_string).collect();
            format!(
                "  {}: [{}, [{}]],",
                f.js_name,
                f.is_async,
                strings.join(", ")
            )
        })
        .collect();
    format!("const guards = {{\n{}\n}}\n{GUARD_CJS}", guards.join("\n"))
}

const GUARD_CJS: &str = r#"const JS_ERRORS = /^(TypeError|RangeError|SyntaxError|ReferenceError|URIError|EvalError): ([\s\S]*)$/
const jsError = e => {
  const message = typeof e?.message === 'string' ? e.message : ''
  const typed = JS_ERRORS.exec(message)
  if (typed) return new globalThis[typed[1]](typed[2])
  if (/Expected$/.test(e?.code)) return Object.assign(new TypeError(message), { code: e.code })
  if (message.startsWith('Rust panic:')) e.code = 'RUST_PANIC'
  return e
}
const illFormed = v => typeof v === 'string' ? !v.isWellFormed() : Array.isArray(v) && v.some(illFormed)
for (const [name, [isAsync, strings]] of Object.entries(guards)) {
  const f = module.exports[name]
  if (typeof f !== 'function') continue
  const check = a => {
    for (const i of strings) {
      if (illFormed(a[i])) throw new TypeError(`argument ${i + 1} of ${name} has a lone surrogate, which a Rust String cannot hold`)
    }
  }
  module.exports[name] = isAsync
    ? { [name]: async function (...a) { check(a); try { return await f(...a) } catch (e) { throw jsError(e) } } }[name]
    : { [name]: function (...a) { check(a); try { return f(...a) } catch (e) { throw jsError(e) } } }[name]
}
"#;

const CODEC_CJS: &str = r#"
// Structured values cross as JSON text. JSON has no `undefined`, NaN, -0, bigint, Date, Map, Set
// or bytes, and Rust strings no lone surrogates: those travel as one-key marker objects
// (`{"$undefined": true}`, `{"$num": "NaN"}`, `{"$map": [[k, v]]}`, `{"$bytes": [...]}`, ...),
// the same markers recordings use.
const replacer = function (k, v) {
  const raw = this[k]
  if (raw instanceof Date) return { $date: Number.isNaN(raw.getTime()) ? 'Invalid' : raw.toISOString() }
  if (typeof Buffer !== 'undefined' && Buffer.isBuffer(raw)) return { $buffer: Array.from(raw) }
  if (raw instanceof Uint8Array) return { $bytes: Array.from(raw) }
  if (raw instanceof Map) return { $map: [...raw] }
  if (raw instanceof Set) return { $set: [...raw] }
  return v === undefined ? { $undefined: true }
    : typeof v === 'bigint' ? { $bigint: String(v) }
    : typeof v === 'number' ? (Number.isFinite(v) && !Object.is(v, -0) ? v : { $num: Object.is(v, -0) ? '-0' : String(v) })
    : typeof v === 'string' && !v.isWellFormed() ? { $utf16: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) }
    : v
}
const encode = v => JSON.stringify(v, replacer)
const single = (v, key) => Object.keys(v).length === 1 && key in v
const revive = v => Array.isArray(v) ? v.map(revive)
  : v !== null && typeof v === 'object'
    ? (single(v, '$undefined') ? undefined
      : single(v, '$utf16') ? fromUnits(v.$utf16)
      : single(v, '$num') ? Number(v.$num)
      : single(v, '$bigint') ? BigInt(v.$bigint)
      : single(v, '$date') ? new Date(v.$date)
      : single(v, '$bytes') ? Uint8Array.from(v.$bytes)
      : single(v, '$buffer') ? Buffer.from(v.$buffer)
      : single(v, '$map') ? new Map(v.$map.map(([k, x]) => [revive(k), revive(x)]))
      : single(v, '$set') ? new Set(v.$set.map(revive))
      : Object.fromEntries(Object.entries(v).map(([k, x]) => [k, revive(x)])))
    : v
// Most results carry no marker: skip the walk unless the text has one.
const decode = text => {
  const value = JSON.parse(text)
  return text.includes('"$') ? revive(value) : value
}
const fromUnits = (units) => {
  let out = ''
  for (let i = 0; i < units.length; i += 8192)
    out += String.fromCharCode.apply(null, Array.prototype.slice.call(units, i, i + 8192))
  return out
}
"#;

const WRAP_CJS: &str = r#"const convert = result => result === 'json' ? decode : result === 'units' ? fromUnits : x => x
// An omitted structured argument is `undefined`, not a missing value.
const prepare = (a, structured) => Array.from({ length: Math.max(a.length, structured.length) }, (_, i) => structured[i] ? encode(a[i]) : a[i])
// Functions: [structured arguments, result kind, async].
for (const [name, [structured, result, isAsync]] of Object.entries(specs)) {
  const call = native[name]
  const out = convert(result)
  module.exports[name] = isAsync
    ? async (...a) => out(await call(...prepare(a, structured)))
    : (...a) => out(call(...prepare(a, structured)))
}
// Classes: a JS class around the native one, converting arguments and results.
const INNER = Symbol('native')
for (const [name, [ctor, methods]] of Object.entries(classes)) {
  const Native = native[name]
  const Wrapped = { [name]: class { constructor(...a) { this[INNER] = new Native(...prepare(a, ctor)) } } }[name]
  for (const [method, [structured, result]] of Object.entries(methods)) {
    const out = convert(result)
    Wrapped.prototype[method] = function (...a) { return out(this[INNER][method](...prepare(a, structured))) }
  }
  module.exports[name] = Wrapped
}
"#;

// ---- js.rs -----------------------------------------------------------------------------------

pub const JS_RS: &str = concat!(
    r#"//! JS values that JSON cannot express, and the JSON transport of the bindings.

#![forbid(unsafe_code)]

/// Reads a structured argument sent by `index.cjs`.
pub fn from_json<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, String> {
    let mut de = serde_json::Deserializer::from_str(text);
    de.disable_recursion_limit();
    let value = <T as serde::Deserialize>::deserialize(&mut de).map_err(|e| format!("TypeError: {e}"))?;
    de.end().map_err(|e| format!("TypeError: {e}"))?;
    Ok(value)
}

/// Writes a structured result for `index.cjs`.
pub fn to_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

/// JS `undefined` inside a structured value: `{ a: undefined }` is not `{ a: null }`.
pub fn undefined() -> serde_json::Value {
    serde_json::json!({ "$undefined": true })
}

/// Whether a structured value received from JS was `undefined`.
pub fn is_undefined(value: &serde_json::Value) -> bool {
    value.as_object().is_some_and(|o| {
        o.len() == 1 && o.get("$undefined") == Some(&serde_json::Value::Bool(true))
    })
}

/// A JS string from UTF-16 code units: a plain string when well-formed, else a marker that
/// `index.cjs` turns into the exact JS string, lone surrogates included.
pub fn utf16_string(units: &[u16]) -> serde_json::Value {
    match String::from_utf16(units) {
        Ok(s) => serde_json::Value::String(s),
        Err(_) => serde_json::json!({ "$utf16": units }),
    }
}

/// The UTF-16 code units of a string received from JS, plain or marked.
pub fn string_units(value: &serde_json::Value) -> Option<Vec<u16>> {
    match value {
        serde_json::Value::String(s) => Some(s.encode_utf16().collect()),
        serde_json::Value::Object(o) if o.len() == 1 => o
            .get("$utf16")?
            .as_array()?
            .iter()
            .map(|u| u.as_u64().and_then(|u| u16::try_from(u).ok()))
            .collect(),
        _ => None,
    }
}

/// A JS number inside a structured value. JSON has no NaN, Infinity or -0: those cross as
/// `{"$num": "NaN"}` and friends, which `index.cjs` turns back into numbers.
pub fn number(n: f64) -> serde_json::Value {
    if n.is_nan() {
        serde_json::json!({ "$num": "NaN" })
    } else if n.is_infinite() {
        serde_json::json!({ "$num": if n > 0.0 { "Infinity" } else { "-Infinity" } })
    } else if n == 0.0 && n.is_sign_negative() {
        serde_json::json!({ "$num": "-0" })
    } else if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
        serde_json::Value::from(n as i64)
    } else {
        serde_json::Value::from(n)
    }
}

/// JS `Array.prototype.sort(compare)`, as V8 runs it.
///
/// With a consistent comparator any stable sort agrees with JS. With one that returns NaN or
/// contradicts itself, the order depends on the algorithm: under 64 elements this reproduces
/// V8's TimSort exactly (one run, then binary insertion); longer arrays use Rust's stable sort.
pub fn sort<T>(items: &mut [T], mut compare: impl FnMut(&T, &T) -> f64) {
    let n = items.len();
    if n < 2 {
        return;
    }
    if n >= 64 {
        items.sort_by(|a, b| compare(a, b).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
        return;
    }
    // CountAndMakeRun: the leading ascending run, or strictly descending run (reversed).
    let mut run = 2;
    let descending = compare(&items[1], &items[0]) < 0.0;
    while run < n {
        let order = compare(&items[run], &items[run - 1]);
        if (descending && order >= 0.0) || (!descending && order < 0.0) {
            break;
        }
        run += 1;
    }
    if descending {
        items[..run].reverse();
    }
    // BinaryInsertionSort of the rest.
    for start in run..n {
        let (mut left, mut right) = (0, start);
        while left < right {
            let mid = left + ((right - left) >> 1);
            if compare(&items[start], &items[mid]) < 0.0 {
                right = mid;
            } else {
                left = mid + 1;
            }
        }
        items[left..=start].rotate_right(1);
    }
}

/// A number received from JS inside a structured value, plain or marked.
pub fn as_number(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::Object(o) if o.len() == 1 => match o.get("$num")?.as_str()? {
            "NaN" => Some(f64::NAN),
            "Infinity" => Some(f64::INFINITY),
            "-Infinity" => Some(f64::NEG_INFINITY),
            "-0" => Some(-0.0),
            _ => None,
        },
        _ => None,
    }
}
"#,
    include_str!("js_number.rs")
);

// ---- Cargo.toml ------------------------------------------------------------------------------

/// Dependencies the bindings need, as `(name, TOML value)`.
pub fn required_deps(kind: Kind, api: &PortApi) -> Vec<(&'static str, String)> {
    let wire = kind == Kind::Napi && api.functions.iter().any(|f| !f.is_async);
    let json = api.needs_json() || wire;
    let any_async = api.all().any(|f| f.is_async);
    let mut deps = Vec::new();
    match kind {
        Kind::Napi => {
            deps.push((
                "napi",
                if any_async {
                    r#"{ version = "3", features = ["async"] }"#.into()
                } else {
                    r#""3""#.into()
                },
            ));
            deps.push(("napi-derive", r#""3""#.into()));
            if any_async {
                deps.push(("futures", r#""0.3""#.into()));
            }
        }
        Kind::Pyo3 => {
            // abi3: one build for every Python from 3.9 on.
            deps.push((
                "pyo3",
                r#"{ version = "0.30", features = ["extension-module", "abi3-py39"] }"#.into(),
            ));
        }
        // The C ABI needs no binding crate.
        Kind::CAbi | Kind::Ffi => {}
        Kind::Wasm => {
            deps.push(("wasm-bindgen", r#""0.2""#.into()));
            if any_async {
                deps.push(("wasm-bindgen-futures", r#""0.4""#.into()));
            }
        }
    }
    if json {
        deps.push(("serde", r#""1""#.into()));
        // JS objects keep insertion order; serde_json sorts keys unless told otherwise. Its
        // default float parser can be off by one ULP; `float_roundtrip` makes it exact.
        deps.push((
            "serde_json",
            r#"{ version = "1", features = ["preserve_order", "float_roundtrip", "unbounded_depth"] }"#.into(),
        ));
    }
    deps
}

pub fn cargo_toml(m: &Manifest, api: &PortApi) -> String {
    let deps: String = required_deps(m.kind, api)
        .iter()
        .map(|(name, value)| format!("{name} = {value}\n"))
        .collect();
    let build = if m.kind == Kind::Napi {
        "\n[build-dependencies]\nnapi-build = \"2\"\n"
    } else {
        ""
    };
    format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2024\"\nrust-version = \"{MIN_RUST}\"\npublish = false\ndescription = \"Rust port of `{}`, generated by rust-anything\"\n\n\
         [lib]\ncrate-type = [\"cdylib\", \"rlib\"]\n\n[dependencies]\n{deps}{build}\n\
         [profile.release]\nlto = true\n\n[lints.clippy]\nall = {{ level = \"warn\", priority = -1 }}\n\n[lints.rust]\nunused_lifetimes = \"warn\"\ntrivial_numeric_casts = \"warn\"\n",
        m.crate_name, m.unit
    )
}

/// Adds missing required dependencies to an existing Cargo.toml, keeping everything else.
pub fn ensure_deps(cargo: &str, kind: Kind, api: &PortApi) -> Result<String, String> {
    let mut doc: toml_edit::DocumentMut = cargo.parse().map_err(|e| format!("Cargo.toml: {e}"))?;
    let deps = doc
        .entry("dependencies")
        .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()))
        .as_table_mut()
        .ok_or("Cargo.toml: [dependencies] is not a table")?;
    for (name, value) in required_deps(kind, api) {
        let wanted: toml_edit::Item = format!("x = {value}")
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| e.to_string())?["x"]
            .clone();
        match deps.get(name) {
            None => {
                deps.insert(name, wanted);
            }
            // napi needs the `async` feature once an async function appears.
            Some(_) if name == "napi" && value.contains("async") => {
                deps.insert(name, wanted);
            }
            Some(_) => {}
        }
    }
    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_port_signatures() {
        let source = "//! doc\n\n#![forbid(unsafe_code)]\n\n/// Port of `fuzzyScore` (`a.ts:1`).\n\
            pub fn fuzzy_score(query: String, text: String) -> Result<f64, String> {\n    Ok(1.0)\n}\n\n\
            /// Port of `load` (`a.ts:9`).\npub async fn load(\n    path: Option<String>,\n    data: Vec<serde_json::Value>,\n) -> Result<serde_json::Value, String> {\n    todo!()\n}\n\n\
            fn helper(x: Vec<u8>) -> u8 { 0 }\n\npub fn plain_name(a: f64) -> Vec<u16> {\n    vec![]\n}\n";
        let fns = parse_port(source).functions;
        assert_eq!(fns.len(), 3);
        assert_eq!(fns[0].js_name, "fuzzyScore");
        assert_eq!(
            fns[0].params,
            [
                ("query".into(), "String".into()),
                ("text".into(), "String".into())
            ]
        );
        assert_eq!(
            (fns[0].ret.as_str(), fns[0].fallible, fns[0].is_async),
            ("f64", true, false)
        );
        assert_eq!(fns[1].params[1].1, "Vec<serde_json::Value>");
        assert_eq!(
            (fns[1].ret.as_str(), fns[1].is_async),
            ("serde_json::Value", true)
        );
        assert_eq!(
            (
                fns[2].js_name.as_str(),
                fns[2].ret.as_str(),
                fns[2].fallible
            ),
            ("plainName", "Vec<u16>", false)
        );
    }

    #[test]
    fn picks_wires() {
        assert_eq!(wire("Vec<String>"), Wire::Native);
        assert_eq!(wire("Option < f64 >"), Wire::Native);
        assert_eq!(wire("Vec<serde_json::Value>"), Wire::Json);
        assert_eq!(wire("Vec<bool>"), Wire::Json);
        assert_eq!(wire("Vec<u16>"), Wire::Units);
    }

    #[test]
    fn adds_missing_deps_only() {
        let fns = parse_port(
            "/// Port of `a`.\npub async fn a(x: serde_json::Value) -> Result<(), String> {\n}\n",
        );
        let cargo =
            "[package]\nname = \"x\"\n\n[dependencies]\nnapi = \"3\"\nindexmap = \"2\" # mine\n";
        let out = ensure_deps(cargo, Kind::Napi, &fns).unwrap();
        assert!(out.contains("indexmap = \"2\" # mine"));
        assert!(out.contains("features = [\"async\"]"));
        assert!(out.contains("futures") && out.contains("preserve_order"));
    }
}
