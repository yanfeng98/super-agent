//! Python backend: profile with cProfile, record real calls, scaffold a PyO3 crate, build it,
//! replay the recordings against it, and swap it in with a small reversible shim.

use crate::backend::{self, Profile, Recording};
use crate::hotspot::Hotspot;
use crate::state::{Ctx, Port, Verify};
use crate::util::{combined, forward_slashes, output, relative, tail};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const RUNNER: &str = include_str!("python/runner.py");
const HOOK: &str = include_str!("python/sitecustomize.py");
const PYO3: &str = "0.30";
/// rust-numpy, for NDArray parameters and results; its versions follow PyO3's.
const NUMPY: &str = "0.30";
pub const DISABLE_ENV: &str = "FAST_ANYTHING_DISABLE";

pub fn is_python(program: &str) -> bool {
    let stem = crate::util::program_stem(program);
    stem == "py" || stem.starts_with("python") || stem.starts_with("pypy")
}

/// Tools that run a command inside a project's virtualenv: `uv run app.py`, `poetry run python app.py`.
const ENV_RUNNERS: &[&str] = &["uv", "poetry", "pipenv", "pdm", "hatch", "rye"];

/// `uv run [python] app.py ...` (and the other ENV_RUNNERS) as `<the env's python> app.py ...`, so
/// every later run uses the same interpreter and packages. `query` runs the runner's own python
/// and returns what it prints. None when the command doesn't start with a runner.
pub fn unwrap_env_runner(command: &[String], query: impl Fn(&[&str]) -> Result<String>) -> Result<Option<Vec<String>>> {
    let runner = crate::util::program_stem(command.first().map(String::as_str).unwrap_or_default());
    if !ENV_RUNNERS.contains(&runner.as_str()) || command.get(1).map(String::as_str) != Some("run") {
        return Ok(None);
    }
    let mut rest = &command[2..];
    if rest.first().is_some_and(|a| is_python(a)) {
        rest = &rest[1..];
    }
    let shown = command[..2].join(" ");
    match rest.first().map(String::as_str) {
        None => bail!("name the script after {shown}, e.g.  fast-anything scan -- {shown} app.py"),
        Some(a) if a.starts_with('-') && a != "-m" => bail!(
            "fast-anything can't pass options to {shown}. Get its interpreter with  {shown} python -c \"import sys; print(sys.executable)\"  and scan with that: fast-anything scan -- <that python> app.py"
        ),
        Some(a) if a != "-m" && !a.ends_with(".py") => bail!(
            "{a} isn't a .py script. Run the code it starts as  {shown} python -m <module>  or  {shown} python <script>.py"
        ),
        _ => {}
    }
    let mut ask = command[..2].iter().map(String::as_str).collect::<Vec<_>>();
    ask.extend(["python", "-c", "import sys; print(sys.executable)"]);
    let exe = query(&ask)?.trim().to_string();
    ensure!(!exe.is_empty(), "{shown} didn't say which python it uses");
    Ok(Some(std::iter::once(exe).chain(rest.iter().cloned()).collect()))
}

/// The absolute interpreter path behind python, python3, py, .venv/bin/python, ...
pub fn resolve(program: &str, root: &Path) -> Result<String> {
    if !is_python(program) {
        bail!(
            "fast-anything v0.1 speaks Python. Start the command with python, e.g.\n    fast-anything scan -- python app.py\n  (Node/TypeScript is next on the roadmap)"
        );
    }
    let prog = if program.contains(['/', '\\']) { root.join(program) } else { PathBuf::from(program) };
    let out = output(
        Command::new(&prog)
            .args(["-c", "import sys; print(sys.executable); print(*sys.version_info[:2])"])
            .current_dir(root),
        "python",
    )?;
    ensure!(out.status.success(), "{program} failed:\n{}", tail(&combined(&out), 5));
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut lines = text.lines();
    let exe = lines.next().unwrap_or_default().trim().to_string();
    let version: Vec<u32> =
        lines.next().unwrap_or_default().split_whitespace().filter_map(|s| s.parse().ok()).collect();
    ensure!(!exe.is_empty(), "couldn't find the interpreter behind {program}");
    ensure!(version.as_slice() >= &[3, 10][..], "Python 3.10 or newer is needed (found {version:?})");
    Ok(exe)
}

fn helper(ctx: &Ctx, python: &str, mode: &str, job: Value, app_args: &[String]) -> Result<Output> {
    let runner = ctx.fast.join("runner.py");
    fs::write(&runner, RUNNER)?;
    let job_path = ctx.fast.join(format!("job-{mode}.json"));
    fs::write(&job_path, serde_json::to_vec_pretty(&job)?)?;
    let mut cmd = Command::new(python);
    cmd.arg(&runner)
        .arg(mode)
        .arg(&job_path)
        .arg("--")
        .args(app_args)
        .current_dir(&ctx.root)
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .stdin(Stdio::null());
    if mode != "verify" {
        watch_workers(ctx, &mut cmd, &runner, &job_path, mode)?;
    }
    let out = output(&mut cmd, "python")?;
    if !out.status.success() {
        bail!("the python {mode} run failed ({}):\n{}", out.status, tail(&combined(&out), 30));
    }
    Ok(out)
}

/// Let the app's multiprocessing workers profile or record too: a sitecustomize first on PYTHONPATH
/// wraps each worker (see python/sitecustomize.py), which leaves its results in .fast/children.
fn watch_workers(ctx: &Ctx, cmd: &mut Command, runner: &Path, job: &Path, mode: &str) -> Result<()> {
    let hook = ctx.fast.join("hook");
    fs::create_dir_all(&hook)?;
    fs::write(hook.join("sitecustomize.py"), HOOK)?;
    let children = ctx.fast.join("children");
    let _ = fs::remove_dir_all(&children);
    fs::create_dir_all(&children)?;
    let mut paths = vec![hook];
    if let Some(existing) = std::env::var_os("PYTHONPATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    cmd.env("PYTHONPATH", std::env::join_paths(paths)?)
        .env("FAST_ANYTHING_CHILDREN", &children)
        .env("FAST_ANYTHING_RUNNER", runner)
        .env("FAST_ANYTHING_JOB", job)
        .env("FAST_ANYTHING_MODE", mode);
    Ok(())
}

pub fn profile(ctx: &Ctx, python: &str, app_args: &[String]) -> Result<Profile> {
    let out = ctx.fast.join("profile.json");
    helper(ctx, python, "profile", json!({ "root": ctx.root, "out": out, "max": 40 }), app_args)?;
    serde_json::from_str(&fs::read_to_string(&out)?).context("reading the profile")
}

const MAIN_GUARD: &[&str] =
    &["if__name__==\"__main__\":", "if__name__=='__main__':", "if\"__main__\"==__name__:", "if'__main__'==__name__:"];

fn main_guard(text: &str) -> Option<usize> {
    backend::line_starting(text, MAIN_GUARD)
}

pub fn record(
    ctx: &Ctx,
    python: &str,
    app_args: &[String],
    hs: &Hotspot,
    out: &Path,
    limit: usize,
) -> Result<Recording> {
    let module = hs.module.as_deref().context("this function can't be imported")?;
    let meta = out.with_extension("meta.json");
    let job = json!({ "module": module, "func": hs.name, "owner": hs.owner, "out": out, "meta": meta, "limit": limit });
    if module != "__main__" {
        helper(ctx, python, "record", job, app_args)?;
    } else {
        // The entry script can't be patched from outside: insert a wrapping line before its
        // if __name__ == "__main__" block for this run, then put the file back byte for byte.
        let file = ctx.root.join(&hs.file);
        let original = fs::read_to_string(&file).with_context(|| format!("reading {}", hs.file))?;
        let backup_dir = ctx.fast.join("backup");
        fs::create_dir_all(&backup_dir)?;
        let backup = backup_dir.join(crate::util::slug(&hs.file));
        fs::write(&backup, &original)?;
        let wrap = match hs.owner.as_deref() {
            Some(cls) => format!(
                "try:\n    {cls}.{f} = staticmethod(__fast_anything_wrap__({cls}.{f}))\nexcept NameError:\n    pass",
                f = hs.name
            ),
            None => format!("try:\n    {f} = __fast_anything_wrap__({f})\nexcept NameError:\n    pass", f = hs.name),
        };
        fs::write(&file, backend::insert_marked(&original, "#", &format!("recording {}", hs.id), &wrap, main_guard))?;
        let run = helper(ctx, python, "record", job, app_args);
        fs::write(&file, &original)
            .with_context(|| format!("restoring {} (a copy is in {})", hs.file, backup.display()))?;
        let _ = fs::remove_file(&backup);
        run?;
    }
    serde_json::from_str(&fs::read_to_string(&meta)?).context("reading the recording summary")
}

// ------------------------------------------------------------------ the Rust crate

/// Rust type for a Python annotation, if there's an obvious one.
pub fn rust_type(annotation: &str, owned: bool) -> Option<String> {
    rust_type_in(annotation, owned, &|_| None)
}

/// rust_type, with `known` naming the Rust type for other names (structs generated for dataclasses).
fn rust_type_in(annotation: &str, owned: bool, known: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    rust_type_at(annotation, owned, false, known)
}

/// `returned`: a tuple[T, ...] in the result has to stay a tuple, and a Vec would come back as a list.
fn rust_type_at(
    annotation: &str,
    owned: bool,
    returned: bool,
    known: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    let a: String = annotation.chars().filter(|c| !c.is_whitespace()).collect();
    let a = a.trim_start_matches("typing.");
    let inner = |prefix: &[&str]| {
        prefix
            .iter()
            .find_map(|p| a.strip_prefix(p).and_then(|r| r.strip_prefix('[')).and_then(|r| r.strip_suffix(']')))
    };
    if let Some(x) = a.strip_suffix("|None").or_else(|| a.strip_prefix("None|")).or_else(|| inner(&["Optional"])) {
        return rust_type_at(x, owned, returned, known).map(|t| format!("Option<{t}>"));
    }
    Some(match a {
        "int" => "i64".into(),
        "float" => "f64".into(),
        "bool" => "bool".into(),
        "str" if owned => "String".into(),
        "str" => "&str".into(),
        "bytes" if owned => "Vec<u8>".into(),
        "bytes" => "&[u8]".into(),
        "bytearray" if owned => "Vec<u8>".into(),
        "bytearray" => "&[u8]".into(),
        "None" if owned => "()".into(),
        _ => {
            if let Some(t) = known(a) {
                t
            } else if let Some(x) = inner(&["NDArray", "npt.NDArray", "np.typing.NDArray", "numpy.typing.NDArray"]) {
                let elem = numpy_elem(x)?;
                if owned {
                    format!("Py<numpy::PyArrayDyn<{elem}>>")
                } else {
                    format!("numpy::PyReadonlyArrayDyn<'_, {elem}>")
                }
            } else if let Some(x) = inner(&["list", "List", "Sequence"]) {
                format!("Vec<{}>", rust_type_at(x, true, returned, known)?)
            } else if let Some(x) = inner(&["tuple", "Tuple"]) {
                let raw_parts = split_top(x);
                let any_length = raw_parts.len() == 2 && raw_parts[1].trim() == "...";
                if any_length && returned {
                    "Py<pyo3::types::PyTuple>".into()
                } else if any_length {
                    format!("Vec<{}>", rust_type_in(&raw_parts[0], true, known)?)
                } else {
                    let parts: Option<Vec<String>> =
                        raw_parts.iter().map(|p| rust_type_at(p, true, returned, known)).collect();
                    let parts = parts?;
                    if parts.iter().any(|p| p == "...") {
                        return None;
                    }
                    if parts.len() == 1 { format!("({},)", parts[0]) } else { format!("({})", parts.join(", ")) }
                }
            } else if let Some(x) = inner(&["dict", "Dict", "Mapping"]) {
                let kv = split_top(x);
                if kv.len() != 2 {
                    return None;
                }
                format!(
                    "std::collections::HashMap<{}, {}>",
                    rust_type_in(&kv[0], true, known)?,
                    rust_type_at(&kv[1], true, returned, known)?
                )
            } else {
                let x = inner(&["set", "Set", "frozenset"])?;
                format!("std::collections::HashSet<{}>", rust_type_in(x, true, known)?)
            }
        }
    })
}

/// Rust element type for a NumPy dtype in NDArray[...]: np.float64 -> f64.
fn numpy_elem(dtype: &str) -> Option<&'static str> {
    let d = dtype.trim_start_matches("numpy.").trim_start_matches("np.");
    Some(match d {
        "float64" | "double" => "f64",
        "float32" | "single" => "f32",
        "int64" => "i64",
        "int32" => "i32",
        "int16" => "i16",
        "int8" => "i8",
        "uint64" => "u64",
        "uint32" => "u32",
        "uint16" => "u16",
        "uint8" => "u8",
        "bool" | "bool_" => "bool",
        _ => return None,
    })
}

/// NumPy's name for a dtype string as recorded ("<f8" -> float64).
fn dtype_name(dtype: &str) -> &str {
    match dtype.trim_start_matches(['<', '>', '|', '=']) {
        "f8" => "float64",
        "f4" => "float32",
        "i8" => "int64",
        "i4" => "int32",
        "i2" => "int16",
        "i1" => "int8",
        "u8" => "uint64",
        "u4" => "uint32",
        "u2" => "uint16",
        "u1" => "uint8",
        "b1" => "bool",
        other => other,
    }
}

/// A recorded array as Python would show it: array([1.0, 2.5], dtype=float64), or just its shape.
fn array_value(dtype: &str, shape: &[Value], data: &str) -> String {
    let kind = dtype.trim_start_matches(['<', '>', '|', '=']);
    let name = dtype_name(dtype);
    let dims: Vec<String> = shape.iter().map(|d| d.to_string()).collect();
    let size: usize = kind.get(1..).and_then(|n| n.parse().ok()).unwrap_or(0);
    let bytes = base64_decode(data).filter(|_| dims.len() == 1 && size > 0 && size <= 8 && !dtype.starts_with('>'));
    let Some(bytes) = bytes else {
        let trailing = if dims.len() == 1 { "," } else { "" };
        return format!("array(shape=({}{trailing}), dtype={name})", dims.join(", "));
    };
    let mut values: Vec<String> = bytes
        .chunks_exact(size)
        .take(8)
        .map(|c| {
            let mut b = [0u8; 8];
            b[..size].copy_from_slice(c);
            match kind.as_bytes()[0] {
                b'f' if size == 8 => format!("{:?}", f64::from_le_bytes(b)),
                b'f' => format!("{:?}", f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
                b'b' => (if b[0] != 0 { "True" } else { "False" }).to_string(),
                b'i' => {
                    let shift = 64 - 8 * size as u32;
                    ((i64::from_le_bytes(b) << shift) >> shift).to_string()
                }
                _ => u64::from_le_bytes(b).to_string(),
            }
        })
        .collect();
    if bytes.len() / size > 8 {
        values.push("…".into());
    }
    format!("array([{}], dtype={name})", values.join(", "))
}

fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let digit = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let (mut out, mut acc, mut bits) = (Vec::new(), 0u32, 0);
    for c in text.bytes().filter(|&c| c != b'=') {
        acc = (acc << 6) | digit(c)? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// Split "a,b[c,d],e" on top-level commas.
fn split_top(s: &str) -> Vec<String> {
    let (mut depth, mut cur, mut out) = (0, String::new(), vec![]);
    for c in s.chars() {
        match c {
            '[' => depth += 1,
            ']' => depth -= 1,
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    out.push(cur);
    out
}

const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "box", "break", "const", "continue", "crate", "do", "dyn", "else", "enum", "extern",
    "false", "final", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut",
    "override", "priv", "pub", "ref", "return", "static", "struct", "trait", "true", "try", "type", "typeof", "unsafe",
    "unsized", "use", "virtual", "where", "while", "yield",
];

fn ident(name: &str) -> String {
    if RUST_KEYWORDS.contains(&name) { format!("r#{name}") } else { name.to_string() }
}

/// A Python default value as a PyO3 signature default, if it's a simple literal.
fn rust_default(value: &str) -> Option<String> {
    let v = value.trim();
    Some(match v {
        "None" => "None".into(),
        "True" => "true".into(),
        "False" => "false".into(),
        _ if v.parse::<i64>().is_ok() || v.parse::<f64>().is_ok() => v.into(),
        _ if (v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')) => {
            format!("{:?}", &v[1..v.len() - 1])
        }
        _ => return None,
    })
}

/// The PyO3 signature default for a parameter: its literal, or None when the Python default
/// isn't a simple literal (the port then gets None and applies the default itself).
fn signature_default(p: &crate::hotspot::Param) -> Option<String> {
    p.default.as_deref().map(|d| rust_default(d).unwrap_or_else(|| "None".into()))
}

/// The annotation names this class (as a whole word: Point in list[Point], not in PointCloud).
fn mentions(annotation: Option<&str>, class: &str) -> bool {
    annotation.is_some_and(|a| a.split(|c: char| !c.is_alphanumeric() && c != '_').any(|w| w == class))
}

/// Rust name for the struct of a dataclass or NamedTuple: its own, unless that would hide a Rust
/// prelude type (a NamedTuple called Box).
fn struct_name(class: &str) -> String {
    const PRELUDE: &[&str] = &[
        "Box",
        "Vec",
        "String",
        "Option",
        "Some",
        "None",
        "Result",
        "Ok",
        "Err",
        "Self",
        "Copy",
        "Clone",
        "Send",
        "Sync",
        "Sized",
        "Drop",
        "Fn",
        "FnMut",
        "FnOnce",
        "Default",
        "Iterator",
        "ToString",
        "ToOwned",
        "Into",
        "From",
        "AsRef",
        "AsMut",
        "Eq",
        "PartialEq",
        "Ord",
        "PartialOrd",
        "Extend",
        "IntoIterator",
        "Unpin",
    ];
    if PRELUDE.contains(&class) { format!("Py{class}") } else { class.to_string() }
}

/// `#[derive(FromPyObject)]` structs for the dataclasses and NamedTuples the parameters use, when
/// every field has a Rust type; PyO3 reads them field by field, by attribute.
/// Returns (Python class name, Rust struct definition).
fn record_structs(hs: &Hotspot) -> Vec<(String, String)> {
    let mut out = vec![];
    let in_params = |name: &str| hs.params.iter().any(|p| mentions(p.annotation.as_deref(), name));
    for (name, fields) in hs.records.iter().filter(|(name, _)| in_params(name)) {
        let Some(types) =
            fields.iter().map(|f| f.annotation.as_deref().and_then(|a| rust_type(a, true))).collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let mut def = format!("#[derive(FromPyObject)]\n#[allow(dead_code)]\nstruct {} {{\n", struct_name(name));
        for (f, ty) in fields.iter().zip(types) {
            if ident(&f.name) != f.name {
                def.push_str(&format!("    #[pyo3(attribute(\"{}\"))]\n", f.name));
            }
            def.push_str(&format!("    {}: {ty},\n", ident(&f.name)));
        }
        def.push_str("}\n\n");
        out.push((name.clone(), def));
    }
    out
}

pub fn lib_rs(hs: &Hotspot, port: &Port) -> String {
    let structs = record_structs(hs);
    let known = |a: &str| structs.iter().any(|(name, _)| name == a).then(|| struct_name(a));
    let mut params: Vec<String> = hs
        .params
        .iter()
        .map(|p| {
            let ty = p
                .annotation
                .as_deref()
                .and_then(|a| rust_type_in(a, false, &known))
                .unwrap_or_else(|| "&Bound<'_, PyAny>".into());
            let ty = if signature_default(p).as_deref() == Some("None") && !ty.starts_with("Option<") {
                format!("Option<{ty}>")
            } else {
                ty
            };
            format!("{}: {ty}", ident(&p.name))
        })
        .collect();
    let signature = if hs.params.iter().any(|p| p.default.is_some() || p.kwonly) {
        let mut parts = vec![];
        for (i, p) in hs.params.iter().enumerate() {
            if p.kwonly && (i == 0 || !hs.params[i - 1].kwonly) {
                parts.push("*".to_string());
            }
            parts.push(match signature_default(p) {
                Some(d) => format!("{} = {d}", ident(&p.name)),
                None => ident(&p.name),
            });
        }
        format!("#[pyo3(signature = ({}))]\n", parts.join(", "))
    } else {
        String::new()
    };
    let ret =
        hs.returns.as_deref().and_then(|a| rust_type_at(a, true, true, &known)).unwrap_or_else(|| "Py<PyAny>".into());
    if ret.contains("PyTuple") {
        params.insert(0, "py: Python<'_>".into());
    }
    format!(
        "//! Rust port of {id} ({file}:{line}), generated by fast-anything.\n\
         //! Replace the todo!() with a faithful, fast port. TASK.md has the original and real calls.\n\
         //! Check it with:  fast-anything verify {id}\n\n\
         use pyo3::prelude::*;\n\n\
         {structs}\
         #[pyfunction]\n\
         {signature}\
         #[allow(unused_variables)]\n\
         fn {func}({params}) -> PyResult<{ret}> {{\n    todo!(\"port {id}\")\n}}\n\n\
         #[pymodule]\n\
         fn {ext}(m: &Bound<'_, PyModule>) -> PyResult<()> {{\n    m.add_function(wrap_pyfunction!({func}, m)?)?;\n    Ok(())\n}}\n",
        id = port.id,
        file = hs.file,
        line = hs.line,
        func = ident(&hs.name),
        params = params.join(", "),
        ext = port.ext,
        structs = structs.iter().map(|(_, def)| def.as_str()).collect::<String>(),
    )
}

fn cargo_toml(port: &Port, numpy: bool) -> String {
    format!(
        "# Rust port of {id}, generated by fast-anything.\n\
         [package]\nname = \"{ext}\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n\
         [lib]\nname = \"{ext}\"\ncrate-type = [\"cdylib\"]\n\n\
         [dependencies]\npyo3 = {{ version = \"{PYO3}\", features = [\"extension-module\", \"abi3-py310\"] }}\n{numpy}\n\
         [profile.release]\nopt-level = 3\nlto = \"fat\"\ncodegen-units = 1\n\n\
         # A crate of its own, even inside another Cargo workspace.\n[workspace]\n",
        id = port.id,
        ext = port.ext,
        numpy = if numpy { format!("numpy = \"{NUMPY}\"\n") } else { String::new() },
    )
}

/// Render a recorded JSON value the way Python would print it.
fn py_value(v: &Value) -> String {
    match v {
        Value::Null => "None".into(),
        Value::Bool(b) => {
            if *b {
                "True".into()
            } else {
                "False".into()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => serde_json::to_string(s).unwrap_or_default(),
        Value::Array(items) => format!("[{}]", items.iter().map(py_value).collect::<Vec<_>>().join(", ")),
        Value::Object(map) => match map.iter().next() {
            Some((tag, Value::Array(items))) if tag == "$tuple" => {
                let inner: Vec<String> = items.iter().map(py_value).collect();
                if inner.len() == 1 { format!("({},)", inner[0]) } else { format!("({})", inner.join(", ")) }
            }
            Some((tag, Value::Array(items))) if tag == "$set" && items.is_empty() => "set()".into(),
            Some((tag, Value::Array(items))) if tag == "$set" => {
                format!("{{{}}}", items.iter().map(py_value).collect::<Vec<_>>().join(", "))
            }
            Some((tag, Value::Array(pairs))) if tag == "$dict" => {
                let inner: Vec<String> = pairs
                    .iter()
                    .filter_map(|p| p.as_array())
                    .filter(|p| p.len() == 2)
                    .map(|p| format!("{}: {}", py_value(&p[0]), py_value(&p[1])))
                    .collect();
                format!("{{{}}}", inner.join(", "))
            }
            Some((tag, Value::String(s))) if tag == "$bytes" => {
                format!("__import__('base64').b64decode({s:?})")
            }
            Some((tag, Value::String(s))) if tag == "$float" => format!("float({s:?})"),
            Some((tag, Value::Array(p))) if tag == "$ndarray" && p.len() == 3 => array_value(
                p[0].as_str().unwrap_or_default(),
                p[1].as_array().map(Vec::as_slice).unwrap_or_default(),
                p[2].as_str().unwrap_or_default(),
            ),
            Some((tag, Value::Array(p))) if tag == "$npscalar" && p.len() == 2 => {
                format!("np.{}({})", dtype_name(p[0].as_str().unwrap_or_default()), py_value(&p[1]))
            }
            Some((tag, Value::Array(parts))) if tag == "$obj" && parts.len() == 3 => {
                let class = parts[1].as_str().unwrap_or_default();
                let fields: Vec<String> = parts[2]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|f| f.as_array().filter(|f| f.len() == 2))
                    .map(|f| format!("{}={}", f[0].as_str().unwrap_or_default(), py_value(&f[1])))
                    .collect();
                format!("{}({})", class.rsplit('.').next().unwrap_or(class), fields.join(", "))
            }
            _ => v.to_string(),
        },
    }
}

fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n - 1).collect::<String>()) }
}

pub fn sample_calls(path: &Path, func: &str, n: usize) -> Vec<String> {
    let Ok(text) = fs::read_to_string(path) else { return vec![] };
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .take(n)
        .map(|case| {
            let mut args: Vec<String> =
                case["args"].as_array().map(|a| a.iter().map(py_value).collect()).unwrap_or_default();
            if let Some(kw) = case["kwargs"].as_object() {
                args.extend(kw.iter().map(|(k, v)| format!("{k}={}", py_value(v))));
            }
            let outcome = match case.get("raises") {
                Some(e) => format!("raises {}", e.as_str().unwrap_or("an exception")),
                None => py_value(&case["result"]),
            };
            format!("{}  ->  {}", clip(&format!("{func}({})", args.join(", ")), 110), clip(&outcome, 60))
        })
        .collect()
}

pub fn task_md(hs: &Hotspot, helpers: &[Hotspot], port: &Port, crate_dir: &Path) -> String {
    let fence = "```";
    let samples = sample_calls(&crate_dir.join("recordings.jsonl"), &hs.name, 8).join("\n");
    let mut notes = String::new();
    for w in &hs.warnings {
        notes.push_str(&format!("- Note: {w}.\n"));
    }
    let structs: Vec<String> = record_structs(hs).into_iter().map(|(name, _)| name).collect();
    for (name, fields) in &hs.records {
        let list: Vec<String> =
            fields.iter().map(|f| format!("{}: {}", f.name, f.annotation.as_deref().unwrap_or("?"))).collect();
        notes.push_str(&format!("- Note: `{name}` is a dataclass/NamedTuple in {} ({}).", hs.file, list.join(", ")));
        if structs.contains(name) {
            notes.push_str(&format!(
                " As a parameter it arrives as the Rust struct `{}` in src/lib.rs.",
                struct_name(name)
            ));
        }
        if mentions(hs.returns.as_deref(), name) {
            notes.push_str(&format!(
                " The result must be a `{name}` too: get the class with `py.import(\"{}\")?.getattr(\"{name}\")?` and call it; verify checks the class and every field.",
                hs.module.as_deref().unwrap_or_default()
            ));
        }
        notes.push('\n');
    }
    if lib_rs(hs, port).contains("numpy::") {
        notes.push_str("- Note: NDArray parameters arrive as `numpy::PyReadonlyArrayDyn` (`.as_array()` gives an ndarray view, `.as_slice()` a slice when contiguous); return arrays as `Py<numpy::PyArrayDyn<T>>`, e.g. `numpy::ndarray::Array::from(vec).into_dyn().into_pyarray(py).unbind()` with `use numpy::IntoPyArray`. verify compares shape, dtype and values.\n");
    }
    for p in &hs.params {
        if let Some(d) = p.default.as_deref().filter(|d| rust_default(d).is_none()) {
            notes.push_str(&format!(
                "- Note: `{}` defaults to `{d}` in Python; the Rust signature takes `None` for it, use that default then.\n",
                p.name
            ));
        }
    }
    format!(
        "# Port `{id}` to Rust\n\n\
You're in a Rust crate fast-anything generated for this: `{dir}`.\n\
Write `{func}` in `src/lib.rs` so it behaves **exactly** like the Python below, and runs fast.\n\
Only edit files in this folder. Don't touch the Python project.\n\n\
## The original ({file}:{line})\n\n{fence}python\n{source}\n{fence}\n\n\
{helpers}## Real calls\n\n\
fast-anything recorded {recorded} real calls (out of {seen} while the app ran). A few:\n\n{fence}\n{samples}\n{fence}\n\n\
## Rules\n\n\
- Same result for every input, edge cases included: empty input, zero, negatives, NaN and infinity, non-ASCII text, very long input. verify tests these too.\n\
- Python `str` indexes by Unicode code point: use `.chars()` (or bytes only after checking `is_ascii()`).\n\
- Python ints don't overflow. `i64` is fine unless the recorded values are bigger.\n\
- Float results are compared with a relative tolerance of 1e-9. Keep the same order of operations.\n\
- Where Python raises, raise too (`PyValueError::new_err(...)`, `PyKeyError`, ...).\n\
- Make it fast: no needless allocation or cloning, borrow inputs, `with_capacity`, fast paths for common cases.\n\
- std only, unless a crate clearly pays off. Keep the function and module names as they are.\n\
{notes}\n\
## Check\n\n\
Run `cargo check` here. Then fast-anything builds the crate in release mode and replays all {recorded} recorded calls against it\n\
(`fast-anything verify {id}`).\n",
        id = port.id,
        dir = crate_dir.display(),
        func = hs.name,
        file = hs.file,
        line = hs.line,
        source = hs.source,
        recorded = port.recorded,
        seen = crate::util::thousands(port.calls_seen),
        helpers = helpers_md(hs, helpers),
    )
}

/// Create the crate (never overwriting a src/lib.rs that's already there) and its TASK.md.
pub fn scaffold(ctx: &Ctx, hs: &Hotspot, helpers: &[Hotspot], port: &Port) -> Result<String> {
    let dir = ctx.crate_dir(port);
    fs::create_dir_all(dir.join("src"))?;
    fs::create_dir_all(dir.join(".cargo"))?;
    fs::write(dir.join("Cargo.toml"), cargo_toml(port, lib_rs(hs, port).contains("numpy::")))?;
    fs::write(dir.join(".cargo/config.toml"), backend::CARGO_CONFIG)?;
    let lib = dir.join("src/lib.rs");
    if !lib.exists() {
        fs::write(&lib, lib_rs(hs, port))?;
    }
    let task = task_md(hs, helpers, port, &dir);
    fs::write(dir.join("TASK.md"), &task)?;
    Ok(task)
}

/// TASK.md section listing the same-file functions the target calls, to port alongside it.
pub fn helpers_md(hs: &Hotspot, helpers: &[Hotspot]) -> String {
    if hs.calls.is_empty() {
        return String::new();
    }
    let mut out = "## Helpers it calls\n\nPort these as private Rust functions in the same crate (they stay in Python for everyone else).\n\n".to_string();
    for name in &hs.calls {
        match helpers.iter().find(|h| &h.name == name) {
            Some(h) => out.push_str(&format!("```python\n{}\n```\n\n", h.source)),
            None => out.push_str(&format!("- `{name}`: read it in {}\n", hs.file)),
        }
    }
    out
}

// ------------------------------------------------------------------ verify

pub fn verify(ctx: &Ctx, port: &Port) -> Result<Verify> {
    let python = ctx.runtime()?;
    let dir = ctx.crate_dir(port);
    let out = dir.join("verify.json");
    let job = json!({
        "base": ctx.base_dir(), "libdir": ctx.lib_dir(), "ext": port.ext, "module": port.module,
        "module_file": ctx.root.join(&port.file),
        "func": port.func, "owner": port.owner, "cases": dir.join("recordings.jsonl"), "out": out, "budget": 0.5,
    });
    helper(ctx, python, "verify", job, &[])?;
    let mut v: Verify = serde_json::from_str(&fs::read_to_string(&out)?).context("reading verify results")?;
    v.built = true;
    Ok(v)
}

// ------------------------------------------------------------------ the shim

/// Lines appended to the module (between markers) that swap in the Rust function.
pub fn shim(port: &Port, lib_rel: &str) -> String {
    let f = &port.func;
    [
        "# Uses the verified Rust port. FAST_ANYTHING_DISABLE=1 runs the Python above instead.",
        &format!("# Undo with:  fast-anything disable {}", port.id),
        "def _fast_anything_load():",
        "    import os, sys",
        "    if os.environ.get(\"FAST_ANYTHING_DISABLE\"):",
        "        return None",
        &format!("    lib = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), \"{lib_rel}\"))"),
        "    if lib not in sys.path:",
        "        sys.path.append(lib)",
        "    try:",
        &format!("        from {} import {f}", port.ext),
        "    except ImportError:",
        "        return None",
        &format!("    return {f}"),
        &match port.owner.as_deref() {
            Some(cls) => format!("{cls}.{f} = staticmethod(_fast_anything_load() or {cls}.{f})"),
            None => format!("{f} = _fast_anything_load() or {f}"),
        },
        "del _fast_anything_load",
    ]
    .join("\n")
}

pub fn strip_shim(text: &str, id: &str) -> String {
    backend::strip_marked(text, "#", id)
}

pub fn add_shim(text: &str, port: &Port, lib_rel: &str) -> String {
    if port.module == "__main__" {
        // before the if __name__ == "__main__" block, so the app's main code sees the Rust version
        backend::insert_marked(text, "#", &port.id, &shim(port, lib_rel), main_guard)
    } else {
        backend::append_marked(text, "#", &port.id, &shim(port, lib_rel))
    }
}

pub fn enable(ctx: &Ctx, port: &Port) -> Result<()> {
    let file = ctx.root.join(&port.file);
    let text = fs::read_to_string(&file).with_context(|| format!("reading {}", port.file))?;
    let from = file.parent().context("module has no folder")?;
    let lib_rel = forward_slashes(&relative(from, &ctx.lib_dir()));
    fs::write(&file, add_shim(&text, port, &lib_rel)).with_context(|| format!("writing {}", port.file))
}

pub fn disable(ctx: &Ctx, port: &Port) -> Result<()> {
    let file = ctx.root.join(&port.file);
    let text = fs::read_to_string(&file).with_context(|| format!("reading {}", port.file))?;
    fs::write(&file, strip_shim(&text, &port.id)).with_context(|| format!("writing {}", port.file))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn port() -> Port {
        Port {
            id: "fuzzy.levenshtein".into(),
            file: "fuzzy.py".into(),
            module: "fuzzy".into(),
            func: "levenshtein".into(),
            ext: "fa_fuzzy_levenshtein".into(),
            crate_dir: ".fast/ports/fuzzy.levenshtein".into(),
            recorded: 3,
            calls_seen: 10,
            verify: None,
            enabled: false,
            lang: crate::backend::Lang::Python,
            owner: None,
            line: 1,
            sources: vec![],
        }
    }

    #[test]
    fn shim_round_trips() {
        for original in ["def f():\n    return 1\n", "def f():\r\n    return 1\r\n", "x = 1"] {
            let on = add_shim(original, &port(), ".fast/lib");
            assert!(on.contains("from fa_fuzzy_levenshtein import levenshtein"));
            assert_eq!(add_shim(&on, &port(), ".fast/lib"), on, "enabling twice changes nothing");
            let off = strip_shim(&on, "fuzzy.levenshtein");
            assert_eq!(off.trim_end(), original.trim_end());
            if original.ends_with('\n') {
                assert_eq!(off, original);
            }
        }
    }

    #[test]
    fn shim_for_a_static_method() {
        let port = Port { owner: Some("Text".into()), ..port() };
        let on = add_shim("class Text:\n    pass\n", &port, ".fast/lib");
        assert!(on.contains("Text.levenshtein = staticmethod(_fast_anything_load() or Text.levenshtein)"), "{on}");
        assert_eq!(strip_shim(&on, "fuzzy.levenshtein"), "class Text:\n    pass\n");
    }

    #[test]
    fn reading_the_environment_or_argv_blocks() {
        let Some(python) = ["python3", "python"]
            .into_iter()
            .find(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()))
        else {
            eprintln!("no python on PATH, skipping");
            return;
        };
        let dir = std::env::temp_dir().join(format!("fa-environ-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("runner.py"), RUNNER).unwrap();
        fs::write(
            dir.join("work.py"),
            "import json\nimport os\nimport sys\n\n\
             def subscript(x):\n    return x * int(os.environ['SCALE'])\n\n\
             def chained(x):\n    return os.environ['PATHS'].split(':') + [x]\n\n\
             def member(x):\n    return x if 'CI' in os.environ else -x\n\n\
             def argv(x):\n    return len(sys.argv[1]) + x\n\n\
             def getenv(x):\n    return x * int(os.environ.get('SCALE', 1))\n\n\
             def constants(x):\n    return x + len(os.sep) + sys.maxsize\n\n\
             def keys(x):\n    return [k for k in os.environb] + [x]\n\n\
             def orig(x):\n    return sys.orig_argv[0] + x\n\n\
             def load():\n    return json.load(sys.stdin)\n\n\
             def write(x):\n    sys.stdout.write(x)\n",
        )
        .unwrap();
        let check = "import runpy, sys, os\n\
                     d = sys.argv[1]\n\
                     describe = runpy.run_path(os.path.join(d, 'runner.py'))['describe']\n\
                     f = os.path.join(d, 'work.py')\n\
                     want = {5: ('subscript', ['does I/O: os.environ']), 8: ('chained', ['does I/O: os.environ']),\n\
                             11: ('member', ['does I/O: os.environ']), 14: ('argv', ['does I/O: sys.argv']),\n\
                             17: ('getenv', ['does I/O: os.environ.get']), 20: ('constants', []),\n\
                             23: ('keys', ['does I/O: os.environb']), 26: ('orig', ['does I/O: sys.orig_argv']),\n\
                             29: ('load', ['does I/O: sys.stdin']), 32: ('write', ['does I/O: sys.stdout.write'])}\n\
                     for line, (name, blockers) in want.items():\n\
                     \x20   got = describe(f, line, name, None, d)['blockers']\n\
                     \x20   assert got == blockers, (name, got)\n";
        let out = Command::new(python).args(["-I", "-c", check]).arg(&dir).output().unwrap();
        let _ = fs::remove_dir_all(&dir);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }

    #[test]
    fn static_methods_are_portable() {
        let Some(python) = ["python3", "python"]
            .into_iter()
            .find(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()))
        else {
            eprintln!("no python on PATH, skipping");
            return;
        };
        let dir = std::env::temp_dir().join(format!("fa-static-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("runner.py"), RUNNER).unwrap();
        fs::write(
            dir.join("text.py"),
            "class Text:\n    @staticmethod\n    def lev(a: str, b: str) -> int:\n        return 0\n\n    def size(self) -> int:\n        return 0\n",
        )
        .unwrap();
        let check = "import runpy, sys, os\n\
                     d = sys.argv[1]\n\
                     describe = runpy.run_path(os.path.join(d, 'runner.py'))['describe']\n\
                     f = os.path.join(d, 'text.py')\n\
                     info = describe(f, 3, 'lev', None, d)\n\
                     assert info['owner'] == 'Text' and info['blockers'] == [], info\n\
                     info = describe(f, 6, 'size', None, d)\n\
                     assert info['owner'] is None and info['blockers'], info\n";
        let out = Command::new(python).args(["-I", "-c", check]).arg(&dir).output().unwrap();
        let _ = fs::remove_dir_all(&dir);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }

    /// Calls reached through an import: `sp.run` after `import subprocess as sp`, `system` after
    /// `from os import system`. Each case says what `describe` must block, and what it must leave alone
    /// (shadowed names, imports in another function, pure calls, the checks that worked without imports).
    const IMPORT_CASES: &str = r#"import ast, os, runpy, sys
runner, d = sys.argv[1], sys.argv[2]
describe = runpy.run_path(runner)["describe"]
CASES = [
    # (name, source, function, expected blockers)
    ("alias", "import subprocess as sp\ndef f(x):\n    sp.run(['true'])\n    return x\n", "f", ["does I/O: subprocess.run"]),
    ("from", "from os import system\ndef f(x):\n    system('true')\n    return x\n", "f", ["does I/O: os.system"]),
    ("path", "from pathlib import Path\ndef f(x, p):\n    Path(p).write_text(str(x))\n    return x\n", "f", ["does I/O: pathlib.Path"]),
    ("random", "from random import random\ndef f(x):\n    return x + random()\n", "f", ["not deterministic: random.random"]),
    ("local import", "def f(x):\n    from time import sleep\n    sleep(0)\n    return x\n", "f", ["not deterministic: time.sleep"]),
    ("gzip open (regression)", "from gzip import open\ndef f(p):\n    return open(p).read()\n", "f", ["does I/O: open"]),
    ("rich print (regression)", "from rich import print\ndef f(x):\n    print(x)\n    return x\n", "f", ["does I/O: print"]),
    ("builtins alias", "import builtins as b\ndef f(p):\n    return b.open(p)\n", "f", ["does I/O: open"]),
    ("numpy random (regression)", "from numpy import random\ndef f():\n    return random.random()\n", "f", ["not deterministic: random.random"]),
    ("local leak FP", "def other():\n    import random as r\n    return r.random()\n\ndef pure(r):\n    return r.get('k')\n", "pure", []),
    ("local overwrite FN", "import random as r\ndef impure():\n    return r.random()\ndef other():\n    import math as r\n    return r.sqrt(4)\n", "impure", ["not deterministic: random.random"]),
    ("local overwrite FP", "import math as m\ndef pure(x):\n    return m.sqrt(x)\ndef other():\n    import random as m\n    return m.random()\n", "pure", []),
    ("main leak FP", "def parse(p):\n    return p.split('/')\n\nif __name__ == '__main__':\n    import pathlib as p\n    p.Path('x').write_text('hi')\n", "parse", []),
    ("param shadow", "from os import path\ndef f(path):\n    return path.strip()\n", "f", []),
    ("local assign shadow", "from os import system\ndef f(x):\n    system = lambda v: v + 1\n    return system(x)\n", "f", []),
    ("nested def shadow", "from os import system\ndef f(x):\n    def system(y):\n        return y + 1\n    return system(x)\n", "f", []),
    ("class shadow", "from pathlib import Path\nclass Path:\n    pass\ndef f(x):\n    return Path(x)\n", "f", []),
    ("local import beats top", "def run(x):\n    return x + 1\ndef danger(cmd):\n    from subprocess import run\n    run(cmd)\n    return 0\n", "danger", ["does I/O: subprocess.run"]),
    ("top beats module import", "from subprocess import run\ndef run(x):\n    return x + 1\ndef f(x):\n    return run(x)\n", "f", []),
    ("try/except both", "try:\n    import subprocess as sp\nexcept ImportError:\n    import my_stub as sp\ndef f(x):\n    sp.run(['true'])\n    return x\n", "f", ["does I/O: subprocess.run"]),
    ("relative shadows", "import random as helper\ndef f():\n    from . import helper\n    return helper.compute()\n", "f", []),
    ("urlparse pure", "from urllib.parse import urlparse\ndef f(u):\n    return urlparse(u).netloc\n", "f", []),
    ("timedelta pure", "from datetime import date, timedelta\ndef f(y):\n    return date(y, 1, 1) + timedelta(days=1)\n", "f", []),
    ("datetime.now nondet", "from datetime import datetime\ndef f():\n    return datetime.now()\n", "f", ["not deterministic: datetime.now"]),
    ("os.path.join pure", "from os.path import join\ndef f(a, b):\n    return join(a, b)\n", "f", []),
    ("os.path.exists io", "from os.path import exists\ndef f(p):\n    return exists(p)\n", "f", ["does I/O: os.path.exists"]),
    ("StringIO pure", "from io import StringIO\ndef f(s):\n    return StringIO(s).read()\n", "f", []),
    ("math pure", "from math import sqrt\ndef f(x):\n    return sqrt(x)\n", "f", []),
    ("future", "from __future__ import annotations\ndef f(x: int) -> int:\n    return x\n", "f", []),
    ("dotted alias", "import os.path as osp\ndef f(p):\n    return osp.getsize(p)\n", "f", ["does I/O: os.path.getsize"]),
    ("plain dotted import", "import os.path\ndef f(p):\n    return os.path.getsize(p)\n", "f", ["does I/O: os.path.getsize"]),
    ("pl alias is pathlib", "import pathlib as pl\ndef f(p, x):\n    pl.Path(p).write_text(str(x))\n    return x\n", "f", ["does I/O: pathlib.Path"]),
    ("numpy random alias", "from numpy import random as rng\ndef f():\n    return rng.random()\n", "f", ["not deterministic: numpy.random.random"]),
    ("comprehension target", "import pathlib as p\ndef f(items):\n    xs = [p.strip() for p in items]\n    p.Path('o').write_text('x')\n    return xs\n", "f", ["does I/O: pathlib.Path"]),
    ("nested def binding", "from os import system\ndef f(x):\n    def inner():\n        system = 1\n    return system(x)\n", "f", ["does I/O: os.system"]),
    ("nested def import", "import random as r\ndef f():\n    def inner():\n        import math as r\n    return r.random()\n", "f", ["not deterministic: random.random"]),
    ("nested import stays inside", "import math as m\ndef f(x):\n    def inner():\n        import subprocess as m\n    return m.sqrt(x)\n", "f", []),
    ("lambda param", "from os import path\ndef f(items):\n    return sorted(items, key=lambda path: path.lower())\n", "f", []),
    ("nested def param", "from os import path\ndef f(items):\n    def norm(path):\n        return path.strip()\n    return [norm(i) for i in items]\n", "f", []),
    ("main guard import counts", "def f(x):\n    return p.Path(x).read_text()\nif __name__ == '__main__':\n    import pathlib as p\n    f('x')\n", "f", ["does I/O: pathlib.Path"]),
    ("polyfill def", "try:\n    from subprocess import run\nexcept ImportError:\n    def run(cmd):\n        return None\ndef f(cmd):\n    run(cmd)\n    return 0\n", "f", ["does I/O: subprocess.run"]),
    ("import after def", "def run(x):\n    return x + 1\nfrom subprocess import run\ndef f(x):\n    return run(x)\n", "f", ["does I/O: subprocess.run"]),
    ("module reassign", "from os import path\npath = {'a': 1}\ndef f(k):\n    return path.get(k)\n", "f", []),
    ("module relative after", "import random as helper\nfrom . import helper\ndef f():\n    return helper.compute()\n", "f", []),
    ("match capture", "from os import path\ndef f(x):\n    match x:\n        case str() as path:\n            return path.strip()\n    return x\n", "f", []),
    ("builtins param", "def f(builtins, x):\n    return builtins.open(x)\n", "f", []),
    ("plain builtins import", "import builtins\ndef f(x):\n    return builtins.open(x)\n", "f", ["does I/O: open"]),
    ("fromisoformat pure", "from datetime import date\ndef f(s):\n    return date.fromisoformat(s)\n", "f", []),
    ("datetime alias now", "from datetime import datetime as dt\ndef f():\n    return dt.now()\n", "f", ["not deterministic: datetime.datetime.now"]),
    ("uuid5 pure", "from uuid import uuid5, NAMESPACE_DNS\ndef f(s):\n    return uuid5(NAMESPACE_DNS, s)\n", "f", []),
    ("class method scope", "import subprocess as sp\nclass C:\n    sp = 1\n    @staticmethod\n    def f(x):\n        sp.run(['true'])\n        return x\n", "f", ["does I/O: subprocess.run"]),
    ("np.random", "import numpy as np\ndef f(n):\n    return np.random.rand(n)\n", "f", ["not deterministic: numpy.random.rand"]),
    ("local import then reassign", "def f(k):\n    from os import path\n    path = {'a': 1}\n    return path.get(k)\n", "f", []),
    ("two local imports", "def f(x):\n    import subprocess as m\n    import math as m\n    return m.sqrt(x)\n", "f", []),
    ("local import then relative", "def f():\n    import random as helper\n    from . import helper\n    return helper.compute()\n", "f", []),
    ("param then later import", "def f(p):\n    s = p.strip()\n    import pathlib as p\n    return p.PurePath(s)\n", "f", []),
    ("comprehension first iterable", "from pathlib import Path\ndef f():\n    return [Path.name for Path in Path('.').iterdir()]\n", "f", ["does I/O: pathlib.Path"]),
    ("nested default", "import random as r\ndef f():\n    def inner(r=r.random()):\n        return r\n    return inner()\n", "f", ["not deterministic: random.random"]),
    ("nested decorator", "import random as r\nfrom functools import lru_cache\ndef f():\n    @lru_cache(maxsize=r.randint(1, 9))\n    def g(r):\n        return r\n    return g(1)\n", "f", ["not deterministic: random.randint"]),
    ("own default", "from os import getcwd\ndef f(getcwd=getcwd()):\n    return getcwd\n", "f", ["does I/O: os.getcwd"]),
    ("class body is no scope for methods", "import subprocess as sp\ndef f(x):\n    class C:\n        sp = 1\n        def m(self):\n            sp.run(['true'])\n    C().m()\n    return x\n", "f", ["does I/O: subprocess.run"]),
    ("class import stays in class", "import math as m\ndef f(x):\n    class C:\n        import subprocess as m\n        def g(self):\n            return m.sqrt(x)\n    return C().g()\n", "f", []),
    ("bare annotation", "from pathlib import Path\nPath: object\ndef f(p):\n    Path(p).write_text('hi')\n", "f", ["does I/O: pathlib.Path"]),
    ("module for target", "import subprocess as sp\nfor sp in []:\n    pass\ndef f():\n    sp.run(['true'])\n", "f", ["does I/O: subprocess.run"]),
    ("rebound module name", "import time\nFLAG = True\nif FLAG:\n    time = lambda x: x + 1\ndef f(x):\n    return time(x)\n", "f", []),
    ("if-branch local binding", "from os import path\ndef f(x):\n    if x:\n        path = x\n    return path.strip()\n", "f", []),
    ("rebind from own value", "def f():\n    import subprocess as sp\n    sp = sp.run(['true'])\n    return sp\n", "f", ["does I/O: subprocess.run"]),
    ("with target from own call", "def f():\n    from pathlib import Path\n    with Path('x').open() as Path:\n        pass\n", "f", ["does I/O: pathlib.Path"]),
    ("class in class", "import subprocess as sp\ndef f():\n    class C:\n        sp = 1\n        class D:\n            y = sp.run(['true'])\n    return C\n", "f", ["does I/O: subprocess.run"]),
]
fails = 0
for i, (label, src, func, want) in enumerate(CASES):
    f = os.path.join(d, f"case{i}.py")
    open(f, "w").write(src)
    line = next(n.lineno for n in ast.walk(ast.parse(src)) if isinstance(n, ast.FunctionDef) and n.name == func)
    info = describe(f, line, func, None, d)
    got = info["blockers"] if info else None
    ok = got == want and info["module"] == f"case{i}"
    fails += not ok
    if not ok:
        print("FAIL", label, f"got={got} want={want}", file=sys.stderr)
# array-library warning kept, named once
src = "import numpy as np\ndef f(x):\n    return np.sum(x)\n"
f = os.path.join(d, "arr.py"); open(f, "w").write(src)
w = describe(f, 2, "f", None, d)["warnings"]
ok = any(x.startswith("already uses np,") for x in w); fails += not ok
if not ok:
    print("FAIL numpy warning", w, file=sys.stderr)
sys.exit(1 if fails else 0)
"#;

    #[test]
    fn io_through_an_import_is_seen() {
        let Some(python) = ["python3", "python"]
            .into_iter()
            .find(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()))
        else {
            eprintln!("no python on PATH, skipping");
            return;
        };
        let dir = std::env::temp_dir().join(format!("fa-import-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("runner.py"), RUNNER).unwrap();
        let out = Command::new(python)
            .args(["-I", "-c", IMPORT_CASES])
            .arg(dir.join("runner.py"))
            .arg(&dir)
            .output()
            .unwrap();
        let _ = fs::remove_dir_all(&dir);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }

    #[test]
    fn maps_annotations() {
        assert_eq!(rust_type("str", false).as_deref(), Some("&str"));
        assert_eq!(rust_type("str", true).as_deref(), Some("String"));
        assert_eq!(rust_type("bytes", false).as_deref(), Some("&[u8]"));
        assert_eq!(rust_type("bytes", true).as_deref(), Some("Vec<u8>"));
        assert_eq!(rust_type("bytearray", false).as_deref(), Some("&[u8]"));
        assert_eq!(rust_type("bytearray", true).as_deref(), Some("Vec<u8>"));
        assert_eq!(rust_type("list[int]", false).as_deref(), Some("Vec<i64>"));
        assert_eq!(rust_type("List[str]", false).as_deref(), Some("Vec<String>"));
        assert_eq!(rust_type("list[bytes]", false).as_deref(), Some("Vec<Vec<u8>>"));
        assert_eq!(rust_type("tuple[int, float]", true).as_deref(), Some("(i64, f64)"));
        assert_eq!(rust_type("tuple[int]", true).as_deref(), Some("(i64,)"));
        assert_eq!(
            rust_type("dict[str, list[float]]", true).as_deref(),
            Some("std::collections::HashMap<String, Vec<f64>>")
        );
        assert_eq!(rust_type("float | None", false).as_deref(), Some("Option<f64>"));
        assert_eq!(rust_type("Optional[int]", false).as_deref(), Some("Option<i64>"));
        assert_eq!(rust_type("MyClass", false), None);
        assert_eq!(rust_type("tuple[int, ...]", true).as_deref(), Some("Vec<i64>"));
        assert_eq!(rust_type("tuple[str, ...]", true).as_deref(), Some("Vec<String>"));
    }

    #[test]
    fn renders_python_values() {
        let v: Value = serde_json::from_str(
            r#"[null, true, "a", {"$bytes": "aGVsbG8="}, {"$tuple": [1, 2]}, {"$dict": [["k", 1.5]]}]"#,
        )
        .unwrap();
        assert_eq!(
            py_value(&v),
            r#"[None, True, "a", __import__('base64').b64decode("aGVsbG8="), (1, 2), {"k": 1.5}]"#
        );
        let sets: Value = serde_json::from_str(r#"[{"$set": []}, {"$set": [1]}, {"$dict": []}]"#).unwrap();
        assert_eq!(py_value(&sets), "[set(), {1}, {}]");
    }

    #[test]
    fn signature_keeps_required_params_before_optional() {
        use crate::hotspot::Param;
        let param = |name: &str, annotation: &str, default: Option<&str>, kwonly: bool| Param {
            name: name.into(),
            annotation: Some(annotation.into()),
            default: default.map(Into::into),
            kwonly,
        };
        let mut hs = Hotspot {
            name: "join_words".into(),
            params: vec![
                param("words", "list[str]", None, false),
                param("n", "int", Some("3"), false),
                param("sep", "str", Some("SEP"), false),
            ],
            ..Hotspot::default()
        };
        let lib = lib_rs(&hs, &port());
        assert!(lib.contains("#[pyo3(signature = (words, n = 3, sep = None))]"), "{lib}");
        assert!(lib.contains("fn join_words(words: Vec<String>, n: i64, sep: Option<&str>)"), "{lib}");

        hs.params = vec![param("a", "int", Some("1"), false), param("b", "int", None, true)];
        let lib = lib_rs(&hs, &port());
        assert!(lib.contains("#[pyo3(signature = (a = 1, *, b))]"), "{lib}");
        assert!(lib.contains("fn join_words(a: i64, b: i64)"), "{lib}");
    }

    #[test]
    fn tuple_returns_stay_tuples() {
        for (returns, want) in [
            ("tuple[int, ...]", "(py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>>"),
            ("typing.Tuple[str, ...]", "-> PyResult<Py<pyo3::types::PyTuple>>"),
            ("tuple[int, ...] | None", "-> PyResult<Option<Py<pyo3::types::PyTuple>>>"),
            ("list[tuple[int, ...]]", "-> PyResult<Vec<Py<pyo3::types::PyTuple>>>"),
            ("tuple[int, float]", "-> PyResult<(i64, f64)>"),
            ("list[tuple[int, int]]", "-> PyResult<Vec<(i64, i64)>>"),
        ] {
            let lib = lib_rs(&Hotspot { returns: Some(returns.into()), ..Hotspot::default() }, &port());
            assert!(lib.contains(want), "{lib}");
        }
    }

    #[test]
    fn verify_tells_ints_from_floats() {
        let Some(python) = ["python3", "python"]
            .into_iter()
            .find(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()))
        else {
            eprintln!("no python on PATH, skipping");
            return;
        };
        let runner = std::env::temp_dir().join(format!("fa-runner-test-{}.py", std::process::id()));
        fs::write(&runner, RUNNER).unwrap();
        let check = "import runpy, sys\n\
                     same = runpy.run_path(sys.argv[1])['same']\n\
                     assert not same(3.0, 3) and not same(3, 3.0)\n\
                     assert not same([1.5, 2.0], [1.5, 2])\n\
                     assert not same({'k': 2.0}, {'k': 2})\n\
                     assert same(3.0, 3.0 + 1e-12) and same(3, 3) and same((1.0, 2), (1.0, 2))\n\
                     assert not same((1, 2), [1, 2]) and not same([(1, 2)], [[1, 2]])\n\
                     assert same(float('nan'), float('nan')) and not same(True, 1)\n";
        let out = Command::new(python).args(["-I", "-c", check]).arg(&runner).output().unwrap();
        let _ = fs::remove_file(&runner);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }

    #[test]
    fn verify_wants_the_same_exception_not_a_panic() {
        let Some(python) = ["python3", "python"]
            .into_iter()
            .find(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()))
        else {
            eprintln!("no python on PATH, skipping");
            return;
        };
        let runner = std::env::temp_dir().join(format!("fa-runner-raises-{}.py", std::process::id()));
        fs::write(&runner, RUNNER).unwrap();
        let check = "import runpy, sys\n\
                     ns = runpy.run_path(sys.argv[1])\n\
                     raises_like, best_pass = ns['raises_like'], ns['best_pass']\n\
                     class PanicException(BaseException): pass\n\
                     assert raises_like(ValueError('x'), 'ValueError')\n\
                     assert not raises_like(KeyError('x'), 'ValueError')\n\
                     assert not raises_like(PanicException('index out of bounds'), 'ValueError')\n\
                     assert not raises_like(None, 'ValueError')\n\
                     def panics(*a):\n    raise PanicException('boom')\n\
                     assert best_pass(panics, [((), {})], 0.0) >= 0\n";
        let out = Command::new(python).args(["-I", "-c", check]).arg(&runner).output().unwrap();
        let _ = fs::remove_file(&runner);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }

    #[test]
    fn env_runners_resolve_to_their_python() {
        let cmd = |s: &str| s.split_whitespace().map(String::from).collect::<Vec<_>>();
        let query = |argv: &[&str]| -> Result<String> {
            assert_eq!(&argv[1..], ["run", "python", "-c", "import sys; print(sys.executable)"]);
            Ok(format!("/proj/.venv-{}/bin/python\n", argv[0]))
        };
        let got = |s: &str| unwrap_env_runner(&cmd(s), query);
        assert_eq!(got("uv run app.py data.txt").unwrap().unwrap(), cmd("/proj/.venv-uv/bin/python app.py data.txt"));
        assert_eq!(got("poetry run python app.py").unwrap().unwrap(), cmd("/proj/.venv-poetry/bin/python app.py"));
        assert_eq!(got("pdm run python3 -m pkg").unwrap().unwrap(), cmd("/proj/.venv-pdm/bin/python -m pkg"));
        assert!(got("python app.py").unwrap().is_none());
        assert!(got("uv pip list").unwrap().is_none());
        assert!(got("uv run --with numpy app.py").unwrap_err().to_string().contains("can't pass options"));
        assert!(got("uv run pytest").unwrap_err().to_string().contains("isn't a .py script"));
        assert!(got("uv run").is_err());
    }

    #[test]
    fn dataclass_parameters_get_structs() {
        use crate::hotspot::Param;
        let field = |name: &str, annotation: &str| Param {
            name: name.into(),
            annotation: Some(annotation.into()),
            ..Param::default()
        };
        let mut hs = Hotspot {
            name: "closest".into(),
            params: vec![
                Param { name: "points".into(), annotation: Some("list[Point]".into()), ..Param::default() },
                Param { name: "box".into(), annotation: Some("Box".into()), ..Param::default() },
            ],
            returns: Some("Stats".into()),
            ..Hotspot::default()
        };
        hs.records.insert("Point".into(), vec![field("x", "float"), field("type", "str")]);
        hs.records.insert("Box".into(), vec![field("lo", "float"), field("hi", "float")]);
        hs.records.insert("Stats".into(), vec![field("n", "int")]);
        hs.records.insert("Shape".into(), vec![field("p", "Polygon")]);
        let lib = lib_rs(&hs, &port());
        assert!(lib.contains("fn closest(points: Vec<Point>, r#box: PyBox) -> PyResult<Py<PyAny>>"), "{lib}");
        assert!(lib.contains("struct PyBox {\n    lo: f64,\n    hi: f64,\n}"), "{lib}");
        assert!(lib.contains("#[pyo3(attribute(\"type\"))]\n    r#type: String,"), "{lib}");
        assert!(!lib.contains("struct Stats"), "only parameter types get a struct: {lib}");

        let v: Value = serde_json::from_str(r#"{"$obj": ["geo", "Point", [["x", 1.5], ["type", "a"]]]}"#).unwrap();
        assert_eq!(py_value(&v), r#"Point(x=1.5, type="a")"#);
    }

    #[test]
    fn records_round_trip_in_the_runner() {
        let Some(python) = ["python3", "python"]
            .into_iter()
            .find(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()))
        else {
            eprintln!("no python on PATH, skipping");
            return;
        };
        let dir = std::env::temp_dir().join(format!("fa-records-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("runner.py"), RUNNER).unwrap();
        fs::write(
            dir.join("geo.py"),
            "from dataclasses import dataclass, field\nfrom typing import NamedTuple\n\n\
             @dataclass(frozen=True)\nclass P:\n    x: float\n    y: float\n\n\
             class B(NamedTuple):\n    lo: float\n    hi: float\n\n\
             @dataclass\nclass C:\n    n: int\n    seen: int = field(init=False, default=0)\n\n\
             def f(ps: list[P], b: B) -> C:\n    return C(len(ps))\n",
        )
        .unwrap();
        let check = "import runpy, sys, os, json\n\
                     d = sys.argv[1]; sys.path.insert(0, d)\n\
                     ns = runpy.run_path(os.path.join(d, 'runner.py'))\n\
                     enc, dec, same, edges, small, variants, describe = (ns[k] for k in ('enc', 'dec', 'same', 'edges', 'small', 'variants', 'describe'))\n\
                     import geo, random\n\
                     c = geo.C(3); object.__setattr__(c, 'seen', 7)\n\
                     for v in (geo.P(1.0, 2.0), geo.B(0.0, 5.0), c, [geo.P(1.0, 2.0)]):\n\
                     \x20   back = dec(json.loads(json.dumps(enc(v))))\n\
                     \x20   assert same(v, back) and type(back) is type(v), (v, back)\n\
                     assert enc(geo.B(0.0, 5.0))['$obj'][:2] == ['geo', 'B']\n\
                     assert not same(geo.B(0.0, 5.0), (0.0, 5.0)) and not same(geo.P(1.0, 2.0), geo.P(1.0, 3.0))\n\
                     b = geo.B(0.0, 5.0)\n\
                     assert edges(b) == [] and small(b) is b and variants(b, random.Random(0)) == []\n\
                     info = describe(os.path.join(d, 'geo.py'), 18, 'f', None, d)\n\
                     assert set(info['records']) == {'P', 'B', 'C'}, info['records']\n\
                     assert info['records']['C'] == [{'name': 'n', 'annotation': 'int'}, {'name': 'seen', 'annotation': 'int'}]\n";
        let out = Command::new(python).args(["-I", "-c", check]).arg(&dir).output().unwrap();
        let _ = fs::remove_dir_all(&dir);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }

    #[test]
    fn numpy_arrays_map_and_render() {
        assert_eq!(rust_type("npt.NDArray[np.float64]", false).as_deref(), Some("numpy::PyReadonlyArrayDyn<'_, f64>"));
        assert_eq!(rust_type("NDArray[numpy.int32]", true).as_deref(), Some("Py<numpy::PyArrayDyn<i32>>"));
        assert_eq!(rust_type("list[npt.NDArray[np.uint8]]", false).as_deref(), Some("Vec<Py<numpy::PyArrayDyn<u8>>>"));
        assert_eq!(rust_type("npt.NDArray[np.complex128]", false), None);
        assert_eq!(rust_type("np.ndarray", false), None, "no dtype, no typed array");
        // [1.5, -2.0] as little-endian float64, [-1, 300] as int16
        let v: Value = serde_json::from_str(
            r#"[{"$ndarray": ["<f8", [2], "AAAAAAAA+D8AAAAAAAAAwA=="]}, {"$ndarray": ["<i2", [2], "//8sAQ=="]},
                {"$ndarray": ["<f8", [2, 3], ""]}, {"$npscalar": ["<f8", 0.5]}]"#,
        )
        .unwrap();
        assert_eq!(
            py_value(&v),
            "[array([1.5, -2.0], dtype=float64), array([-1, 300], dtype=int16), array(shape=(2, 3), dtype=float64), np.float64(0.5)]"
        );
        let hs = Hotspot {
            name: "f".into(),
            params: vec![crate::hotspot::Param {
                name: "xs".into(),
                annotation: Some("npt.NDArray[np.float64]".into()),
                ..Default::default()
            }],
            ..Hotspot::default()
        };
        assert!(cargo_toml(&port(), lib_rs(&hs, &port()).contains("numpy::")).contains("numpy = \"0.30\""));
        assert!(!cargo_toml(&port(), false).contains("numpy"));
    }

    #[test]
    fn numpy_values_in_the_runner() {
        let Some(python) = ["python3", "python"]
            .into_iter()
            .find(|p| Command::new(p).args(["-c", "import numpy"]).output().is_ok_and(|o| o.status.success()))
        else {
            eprintln!("no python with numpy on PATH, skipping");
            return;
        };
        let runner = std::env::temp_dir().join(format!("fa-runner-numpy-{}.py", std::process::id()));
        fs::write(&runner, RUNNER).unwrap();
        let check = "import runpy, sys, json, math\n\
                     import numpy as np\n\
                     ns = runpy.run_path(sys.argv[1])\n\
                     enc, dec, same, edges, bigger, variants = (ns[k] for k in ('enc', 'dec', 'same', 'edges', 'bigger', 'variants'))\n\
                     Unsupported = ns['Unsupported']\n\
                     a = np.array([[1.5, -2.0, 3.0], [0.0, 4.0, 5.0]])\n\
                     for v in (a, np.arange(5, dtype=np.int16), np.array([True, False]), np.float64(2.5), np.int32(7), [a[0], a[1]]):\n\
                     \x20   back = dec(json.loads(json.dumps(enc(v))))\n\
                     \x20   assert same(v, back), (v, back)\n\
                     assert type(dec(json.loads(json.dumps(enc(np.int32(7)))))) is np.int32\n\
                     assert not same(a, a.astype(np.float32)) and not same(a, a.tolist()) and not same(a, a[0])\n\
                     assert same(np.array([math.nan, 1.0]), np.array([math.nan, 1.0 + 1e-12]))\n\
                     assert same(np.float64(0.5), 0.5) and not same(np.int64(3), 3.0)\n\
                     x = np.array([1.0, 2.0, 3.0])\n\
                     e = edges(x); assert len(e[0]) == 0 and math.isnan(e[1][0]) and x[0] == 1.0\n\
                     assert edges(np.array([1, 2], dtype=np.uint8))[1][0] == 0\n\
                     assert list(bigger(x, 2)) == [1.0, 2.0, 3.0, 1.0, 2.0, 3.0]\n\
                     import random; assert all(isinstance(v, np.ndarray) for v in variants(x, random.Random(0)))\n\
                     try:\n    enc(np.zeros(1 << 18)); raise SystemExit('big array recorded')\nexcept Unsupported: pass\n\
                     try:\n    enc(np.array(['a'])); raise SystemExit('str array recorded')\nexcept Unsupported: pass\n";
        let out = Command::new(python).args(["-I", "-c", check]).arg(&runner).output().unwrap();
        let _ = fs::remove_file(&runner);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }

    #[test]
    fn spots_python() {
        assert!(is_python("python"));
        assert!(is_python("python3.12"));
        assert!(is_python(".venv/bin/python"));
        assert!(is_python("C:\\Python313\\python.exe"));
        assert!(is_python("py"));
        assert!(!is_python("node"));
    }

    struct WorkerEnv {
        python: &'static str,
        dir: PathBuf,
    }

    impl WorkerEnv {
        fn new(tag: &str, files: &[(&str, &str)]) -> Option<Self> {
            let python = ["python3", "python"]
                .into_iter()
                .find(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()));
            let Some(python) = python else {
                eprintln!("no python on PATH, skipping");
                return None;
            };
            // macOS's temp folder is a symlink and Python reports the files' real paths; on Windows,
            // canonicalize() would give a \\?\ path Python doesn't use
            let tmp = std::env::temp_dir();
            let tmp = if cfg!(windows) { tmp } else { tmp.canonicalize().unwrap() };
            let dir = tmp.join(format!("fa-{tag}-{}", std::process::id()));
            fs::create_dir_all(dir.join("hook")).unwrap();
            fs::create_dir_all(dir.join("children")).unwrap();
            fs::write(dir.join("hook/sitecustomize.py"), HOOK).unwrap();
            fs::write(dir.join("runner.py"), RUNNER).unwrap();
            for (name, body) in files {
                fs::write(dir.join(name), body).unwrap();
            }
            Some(Self { python, dir })
        }

        fn run(&self, mode: &str, job: Value, extra: &[&str]) {
            let job_path = self.dir.join(format!("job-{mode}.json"));
            let children = self.dir.join("children");
            fs::write(&job_path, job.to_string()).unwrap();
            let _ = fs::remove_dir_all(&children);
            fs::create_dir_all(&children).unwrap();
            let mut cmd = Command::new(self.python);
            cmd.args([
                self.dir.join("runner.py").as_os_str(),
                mode.as_ref(),
                job_path.as_os_str(),
                "--".as_ref(),
                "app.py".as_ref(),
            ])
            .args(extra)
            .current_dir(&self.dir)
            .env("PYTHONPATH", self.dir.join("hook"))
            .env("FAST_ANYTHING_CHILDREN", &children)
            .env("FAST_ANYTHING_RUNNER", self.dir.join("runner.py"))
            .env("FAST_ANYTHING_JOB", &job_path)
            .env("FAST_ANYTHING_MODE", mode);
            let out = cmd.output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        }
    }

    impl Drop for WorkerEnv {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn assert_workers_profiled_and_recorded(tag: &str, app: &str) {
        let Some(env) =
            WorkerEnv::new(tag, &[("work.py", "def sq(x: int) -> int:\n    return x * x\n"), ("app.py", app)])
        else {
            return;
        };
        env.run("profile", json!({ "root": env.dir, "out": env.dir.join("profile.json"), "max": 40 }), &[]);
        let profile: Value = serde_json::from_str(&fs::read_to_string(env.dir.join("profile.json")).unwrap()).unwrap();
        let sq = profile["functions"].as_array().unwrap().iter().find(|f| f["name"] == "sq").cloned();
        env.run(
            "record",
            json!({ "module": "work", "func": "sq", "owner": null, "limit": 500,
                    "out": env.dir.join("rec.jsonl"), "meta": env.dir.join("rec.meta.json") }),
            &[],
        );
        let meta: Value = serde_json::from_str(&fs::read_to_string(env.dir.join("rec.meta.json")).unwrap()).unwrap();
        let recorded = fs::read_to_string(env.dir.join("rec.jsonl")).unwrap().lines().count();
        assert_eq!(sq.map(|f| f["ncalls"].clone()), Some(json!(20)), "sq ran 20 times in the workers");
        assert_eq!((meta["calls"].clone(), recorded), (json!(20), 20));
    }

    #[test]
    fn workers_are_profiled_and_recorded() {
        assert_workers_profiled_and_recorded(
            "workers-test",
            "from concurrent.futures import ProcessPoolExecutor\nfrom multiprocessing import get_context\n\
             from work import sq\n\nif __name__ == \"__main__\":\n\
             \x20   with ProcessPoolExecutor(2, mp_context=get_context(\"spawn\")) as pool:\n\
             \x20       print(sum(pool.map(sq, range(20))))\n",
        );
    }

    #[test]
    fn pool_context_manager_workers_are_profiled_and_recorded() {
        assert_workers_profiled_and_recorded(
            "pool-with-test",
            "from multiprocessing import Pool\nfrom work import sq\n\nif __name__ == \"__main__\":\n\
             \x20   with Pool(2) as pool:\n\
             \x20       print(sum(pool.map(sq, range(20))))\n",
        );
    }

    #[test]
    fn pool_terminate_does_not_run_queued_tasks_or_hang() {
        let tasks = "import pathlib, time\n\n\
             def sq(x: int) -> int:\n    return x * x\n\n\
             def hold(started_path: str) -> None:\n\
             \x20   pathlib.Path(started_path).write_text('1', encoding='utf-8')\n\
             \x20   time.sleep(1.0)\n\n\
             def side_effect(marker_path: str) -> None:\n\
             \x20   pathlib.Path(marker_path).write_text('ran', encoding='utf-8')\n";
        let app = "import pathlib, sys, time\n\
             from multiprocessing import Pool\n\
             from multiprocessing.pool import Pool as PoolClass, ThreadPool\n\
             from tasks import hold, side_effect, sq\n\n\
             class TrackedPool(PoolClass):\n\
             \x20   close_calls = 0\n\
             \x20   def close(self):\n\
             \x20       TrackedPool.close_calls += 1\n\
             \x20       super().close()\n\n\
             if __name__ == \"__main__\":\n\
             \x20   started, marker = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])\n\
             \x20   with Pool(1) as pool:\n\
             \x20       pool.apply_async(hold, (str(started),))\n\
             \x20       pool.apply_async(side_effect, (str(marker),))\n\
             \x20       deadline = time.monotonic() + 10\n\
             \x20       while not started.exists() and time.monotonic() < deadline:\n\
             \x20           time.sleep(0.02)\n\
             \x20       assert started.exists(), 'worker did not start blocking task'\n\
             \x20   assert not marker.exists(), 'queued task ran after terminate'\n\
             \x20   with TrackedPool(2) as pool:\n\
             \x20       assert pool.map(sq, range(4)) == [0, 1, 4, 9]\n\
             \x20   assert TrackedPool.close_calls == 0, 'terminate must not call subclass close()'\n\
             \x20   with ThreadPool(2) as pool:\n\
             \x20       assert pool.map(sq, range(4)) == [0, 1, 4, 9]\n\
             \x20   with Pool(2) as pool:\n\
             \x20       res = pool.apply_async(sq, (5,))\n\
             \x20       pool.close()\n\
             \x20       assert res.get() == 25\n\
             \x20   t1 = time.monotonic()\n\
             \x20   with Pool(1, initializer=time.sleep, initargs=(30,)) as pool:\n\
             \x20       pass\n\
             \x20   assert time.monotonic() - t1 < 10, 'terminate hung on stuck initializer'\n";
        let Some(env) = WorkerEnv::new("pool-term-test", &[("tasks.py", tasks), ("app.py", app)]) else {
            return;
        };
        let started = env.dir.join("started.txt");
        let marker = env.dir.join("marker.txt");
        env.run(
            "profile",
            json!({ "root": env.dir, "out": env.dir.join("profile.json"), "max": 40 }),
            &[started.to_str().unwrap(), marker.to_str().unwrap()],
        );
        assert!(!marker.exists(), "queued task must not execute after terminate");
    }
}
