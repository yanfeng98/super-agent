//! R backend: apps run as  Rscript app.R. Base R only, no R headers or libR to link: the port is
//! the same C entry point + JSON crate as Ruby's, with two extra functions R's .C() can call, and
//! its reply comes back as R source that R's own parser reads. Profiled with Rprof, recorded and
//! verified by R scripts that R loads through R_PROFILE_USER, so the app keeps its command line.

use crate::backend::{self, Profile, Recording};
use crate::hotspot::Hotspot;
use crate::native::{cargo_toml, snake};
use crate::python::DISABLE_ENV;
use crate::state::{Ctx, Port, Verify};
use crate::util::{combined, forward_slashes, output, relative, slug, tail};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const RUNNER: &str = include_str!("r/runner.R");
const STARTUP: &str = include_str!("r/startup.R");
const BRIDGE: &str = include_str!("r/bridge.R");
/// R's "run as a script" check, like Python's if __name__ == "__main__".
const GUARDS: &[&str] = &["if(sys.nframe()==0)", "if(sys.nframe()==0L)", "if(0==sys.nframe())", "if(0L==sys.nframe())"];

pub fn is_r(program: &str) -> bool {
    crate::util::program_stem(program) == "rscript"
}

pub fn resolve(program: &str, root: &Path) -> Result<String> {
    let prog = if program.contains(['/', '\\']) { root.join(program) } else { PathBuf::from(program) };
    let script = "cat(file.path(R.home('bin'), if (.Platform$OS.type == 'windows') 'Rscript.exe' else 'Rscript'), \
                  paste(R.version$major, R.version$minor, sep = '.'), l10n_info()[['UTF-8']], sep = '\\n')";
    let out = output(Command::new(&prog).args(["-e", script]).current_dir(root), "Rscript")?;
    ensure!(out.status.success(), "{program} failed:\n{}", tail(&combined(&out), 5));
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut lines = text.lines();
    let exe = lines.next().unwrap_or_default().trim().to_string();
    let version: Vec<u32> = lines.next().unwrap_or_default().split('.').filter_map(|s| s.parse().ok()).collect();
    ensure!(version.as_slice() >= &[4, 0][..], "R 4.0 or newer is needed (found {version:?})");
    if lines.next().map(str::trim) == Some("FALSE") {
        crate::ui::warn(
            "R runs in a locale that isn't UTF-8 (see LANG), so it treats non-ASCII text byte by byte while a Rust port sees characters. Use a UTF-8 locale, e.g. LANG=C.UTF-8",
        );
    }
    Ok(exe)
}

/// The script in "Rscript [options] app.R args".
fn entry(args: &[String]) -> Result<&String> {
    for a in args {
        ensure!(
            a != "--vanilla" && a != "--no-init-file",
            "{a} stops R from loading fast-anything's startup file: leave it out while fast-anything profiles or records"
        );
        ensure!(
            a != "-e",
            "Rscript -e isn't supported; point it at a script, e.g.  fast-anything scan -- Rscript app.R"
        );
        if !a.starts_with('-') {
            return Ok(a);
        }
    }
    bail!("name the script to run, e.g.  fast-anything scan -- Rscript app.R")
}

fn main_guard(text: &str) -> Option<usize> {
    backend::line_starting(text, GUARDS)
}

/// A JSON value as an R literal (for the job files, so the R side needs no JSON parser).
fn r_literal(v: &Value) -> String {
    match v {
        Value::Null => "NULL".into(),
        Value::Bool(b) => (if *b { "TRUE" } else { "FALSE" }).into(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => r_string(s),
        Value::Array(items) => format!("list({})", items.iter().map(r_literal).collect::<Vec<_>>().join(", ")),
        Value::Object(map) => format!(
            "list({})",
            map.iter().map(|(k, x)| format!("{} = {}", r_string(k), r_literal(x))).collect::<Vec<_>>().join(", ")
        ),
    }
}

/// An R string literal, ASCII only.
fn r_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            c => out.push_str(&format!("\\U{{{:x}}}", c as u32)),
        }
    }
    out.push('"');
    out
}

fn write_scripts(ctx: &Ctx) -> Result<()> {
    fs::write(ctx.fast.join("runner.R"), RUNNER)?;
    fs::write(ctx.fast.join("startup.R"), STARTUP)?;
    fs::write(ctx.fast.join("bridge.R"), BRIDGE)?;
    Ok(())
}

fn runner(ctx: &Ctx, rscript: &str, mode: &str, job: Value) -> Result<Output> {
    write_scripts(ctx)?;
    let job_path = ctx.fast.join(format!("job-{mode}.R"));
    fs::write(&job_path, format!("job <- {}\n", r_literal(&job)))?;
    let out = output(
        Command::new(rscript)
            .arg(ctx.fast.join("runner.R"))
            .arg(mode)
            .arg(&job_path)
            .current_dir(&ctx.root)
            .stdin(Stdio::null()),
        "Rscript",
    )?;
    if !out.status.success() {
        bail!("the R {mode} step failed ({}):\n{}", out.status, tail(&combined(&out), 30));
    }
    Ok(out)
}

/// Run the app with fast-anything's startup file loaded first (the app's own still runs after it).
fn run_app(ctx: &Ctx, rscript: &str, args: &[String], mode: &str, env: &[(&str, &Path)]) -> Result<Output> {
    write_scripts(ctx)?;
    let mut cmd = Command::new(rscript);
    cmd.args(args)
        .current_dir(&ctx.root)
        .env("R_PROFILE_USER", ctx.fast.join("startup.R"))
        .env("FAST_ANYTHING_MODE", mode)
        .env("FAST_ANYTHING_BRIDGE", ctx.fast.join("bridge.R"))
        .stdin(Stdio::null());
    if let Some(user) = std::env::var_os("R_PROFILE_USER") {
        cmd.env("FAST_ANYTHING_USER_RPROFILE", user);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    output(&mut cmd, "Rscript")
}

pub fn profile(ctx: &Ctx, rscript: &str, args: &[String]) -> Result<Profile> {
    let script = entry(args)?.clone();
    let rprof = ctx.fast.join("Rprof.out");
    let run = run_app(ctx, rscript, args, "profile", &[("FAST_ANYTHING_OUT", &rprof)])?;
    ensure!(run.status.success(), "the app failed ({}):\n{}", run.status, tail(&combined(&run), 30));
    let result = ctx.fast.join("profile.json");
    let job = json!({ "root": ctx.root, "rprof": rprof, "entry": ctx.root.join(script), "out": result });
    runner(ctx, rscript, "report", job)?;
    serde_json::from_str(&fs::read_to_string(&result)?).context("reading the profile")
}

fn wrap_line(name: &str) -> String {
    format!("if (exists(\".fast_anything_wrap\")) {name} <- .fast_anything_wrap({name})")
}

/// Record calls for one run: a line right after the definitions (appended to the file, or put
/// before the entry script's main block) hands the function to the recorder. The file is
/// restored afterwards.
pub fn record(ctx: &Ctx, rscript: &str, args: &[String], hs: &Hotspot, out: &Path, limit: usize) -> Result<Recording> {
    entry(args)?;
    let file = ctx.root.join(&hs.file);
    let original = fs::read_to_string(&file).with_context(|| format!("reading {}", hs.file))?;
    let backup_dir = ctx.fast.join("backup");
    fs::create_dir_all(&backup_dir)?;
    let backup = backup_dir.join(slug(&hs.file));
    fs::write(&backup, &original)?;
    let id = format!("recording {}", hs.id);
    let line = wrap_line(&hs.name);
    let patched = if hs.module.as_deref() == Some("__main__") {
        backend::insert_marked(&original, "#", &id, &line, main_guard)
    } else {
        backend::append_marked(&original, "#", &id, &line)
    };
    fs::write(&file, patched)?;
    let meta = out.with_extension("meta.json");
    let limit_text = PathBuf::from(limit.to_string());
    let run = run_app(
        ctx,
        rscript,
        args,
        "record",
        &[
            ("FAST_ANYTHING_RECORD_OUT", out),
            ("FAST_ANYTHING_RECORD_META", &meta),
            ("FAST_ANYTHING_RECORD_LIMIT", &limit_text),
            (DISABLE_ENV, Path::new("1")),
        ],
    );
    fs::write(&file, &original)
        .with_context(|| format!("restoring {} (a copy is in {})", hs.file, backup.display()))?;
    let _ = fs::remove_file(&backup);
    let run = run?;
    ensure!(run.status.success(), "the app failed while recording ({}):\n{}", run.status, tail(&combined(&run), 30));
    serde_json::from_str(&fs::read_to_string(&meta)?).context("reading the recording summary")
}

// ------------------------------------------------------------------ the crate

/// An R default value as JSON, if it's a simple literal (the R side passes every argument, so
/// this only matters for calls made from Rust).
pub fn default_json(value: &str) -> Option<String> {
    let v = value.trim();
    Some(match v {
        "NULL" | "NA" => "null".into(),
        "TRUE" | "T" => "true".into(),
        "FALSE" | "F" => "false".into(),
        _ if v.strip_suffix('L').unwrap_or(v).parse::<i64>().is_ok() => v.trim_end_matches('L').into(),
        _ if v.parse::<f64>().is_ok() => v.into(),
        _ if v.len() >= 2
            && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\''))) =>
        {
            serde_json::to_string(&v[1..v.len() - 1]).ok()?
        }
        _ => return None,
    })
}

/// What R's .C() calls, and the r module ports use: generated next to the shared glue.
const R_GLUE: &str = r#"
// ------------------------------------------------------------------ R

/// Types for R values. R has no scalars (a length-1 vector arrives as a plain value), NA arrives
/// as null, and NaN / Inf / -Inf, which JSON has no numbers for, as {"$num": "NaN" | "Inf" | "-Inf"}.
#[allow(dead_code)]
pub mod r {
    use serde::de::{self, Deserialize, Deserializer};
    use serde::ser::{Serialize, SerializeMap, Serializer};

    /// A number that may be NaN or +-Inf (NA is null: use Option<Num>).
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
    pub struct Num(pub f64);

    impl<'de> Deserialize<'de> for Num {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            #[derive(serde::Deserialize)]
            #[serde(untagged)]
            enum Repr {
                Plain(f64),
                Tagged {
                    #[serde(rename = "$num")]
                    num: String,
                },
            }
            match Repr::deserialize(d)? {
                Repr::Plain(x) => Ok(Num(x)),
                Repr::Tagged { num } => match num.as_str() {
                    "NaN" => Ok(Num(f64::NAN)),
                    "Inf" => Ok(Num(f64::INFINITY)),
                    "-Inf" => Ok(Num(f64::NEG_INFINITY)),
                    other => Err(de::Error::custom(format!("not a number: {other}"))),
                },
            }
        }
    }

    impl Serialize for Num {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            if self.0.is_finite() {
                return s.serialize_f64(self.0);
            }
            let mut map = s.serialize_map(Some(1))?;
            map.serialize_entry("$num", if self.0.is_nan() { "NaN" } else if self.0 > 0.0 { "Inf" } else { "-Inf" })?;
            map.end()
        }
    }

    /// A vector argument: an array, or the plain value a length-1 vector arrives as.
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct Vector<T>(pub Vec<T>);

    impl<'de, T: Deserialize<'de>> Deserialize<'de> for Vector<T> {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            #[derive(serde::Deserialize)]
            #[serde(untagged)]
            enum Repr<T> {
                Many(Vec<T>),
                One(T),
            }
            Ok(match Repr::deserialize(d)? {
                Repr::Many(v) => Vector(v),
                Repr::One(x) => Vector(vec![x]),
            })
        }
    }

    impl<T: Serialize> Serialize for Vector<T> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.collect_seq(&self.0)
        }
    }

    impl<T> std::ops::Deref for Vector<T> {
        type Target = Vec<T>;
        fn deref(&self) -> &Vec<T> {
            &self.0
        }
    }

    /// R's NULL as a result (None, like null anywhere, is NA).
    pub struct Null;

    impl Serialize for Null {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut map = s.serialize_map(Some(1))?;
            map.serialize_entry("$null", &true)?;
            map.end()
        }
    }

    /// A result that must be an R list. Without it, an array of plain values becomes a vector
    /// (c(...)), and an object whose values are all plain becomes a named vector.
    pub struct List<T>(pub T);

    impl<T: Serialize> Serialize for List<T> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut map = s.serialize_map(Some(1))?;
            map.serialize_entry("$list", &self.0)?;
            map.end()
        }
    }

    pub(crate) fn string(s: &str) -> String {
        let mut out = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
                c => out.push_str(&format!("\\U{{{:x}}}", c as u32)),
            }
        }
        out.push('"');
        out
    }

    fn special(v: &serde_json::Value) -> Option<&str> {
        v.as_object().filter(|m| m.len() == 1)?.get("$num")?.as_str()
    }

    fn plain(v: &serde_json::Value) -> bool {
        !v.is_array() && !v.is_object() || special(v).is_some()
    }

    fn named(map: &serde_json::Map<String, serde_json::Value>, item: fn(&serde_json::Value) -> String) -> String {
        map.iter().map(|(k, x)| format!("{} = {}", string(k), item(x))).collect::<Vec<_>>().join(", ")
    }

    fn element(v: &serde_json::Value) -> String {
        if v.is_null() { "NULL".into() } else { source(v) }
    }

    fn list(v: &serde_json::Value) -> String {
        match v {
            serde_json::Value::Array(items) => {
                format!("list({})", items.iter().map(element).collect::<Vec<_>>().join(", "))
            }
            serde_json::Value::Object(map) => format!("list({})", named(map, element)),
            other => format!("list({})", element(other)),
        }
    }

    /// A value as R source.
    pub(crate) fn source(v: &serde_json::Value) -> String {
        use serde_json::Value;
        match v {
            Value::Null => "NA".into(),
            Value::Bool(b) => (if *b { "TRUE" } else { "FALSE" }).into(),
            Value::Number(n) => match n.as_i64() {
                Some(i) => i.to_string(),
                None => format!("{:?}", n.as_f64().unwrap_or(f64::NAN)),
            },
            Value::String(s) => string(s),
            Value::Array(items) if items.is_empty() => "logical(0)".into(),
            Value::Array(items) if items.iter().all(plain) => {
                format!("c({})", items.iter().map(source).collect::<Vec<_>>().join(", "))
            }
            Value::Array(_) => list(v),
            Value::Object(map) if map.len() == 1 && map.contains_key("$null") => "NULL".into(),
            Value::Object(map) => match (special(v), map.get("$list")) {
                (Some("NaN"), _) => "NaN".into(),
                (Some("Inf"), _) => "Inf".into(),
                (Some("-Inf"), _) => "-Inf".into(),
                (_, Some(inner)) if map.len() == 1 => list(inner),
                _ if map.is_empty() => "list()".into(),
                _ if map.values().all(plain) => format!("c({})", named(map, source)),
                _ => list(v),
            },
        }
    }
}

thread_local! {
    static FA_R_REPLY: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// R's .C("fa_r_call", json, n = integer(1)): runs the port and keeps its reply (R source for
/// list(ok = ...) or list(error = "...")) for fa_r_take; n is its length in bytes.
///
/// # Safety
/// input must point to one NUL-terminated string and len to an int, as .C() passes them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fa_r_call(input: *const *const std::ffi::c_char, len: *mut i32) {
    let bytes = unsafe { std::ffi::CStr::from_ptr(*input) }.to_bytes();
    let reply = match std::panic::catch_unwind(|| call(bytes)) {
        Ok(Ok(value)) => format!("list(ok = {})", r::source(&value)),
        Ok(Err(message)) => format!("list(error = {})", r::string(&message)),
        Err(panic) => {
            let text = panic.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| panic.downcast_ref::<String>().cloned());
            format!("list(error = {})", r::string(&format!("panic: {}", text.unwrap_or_default())))
        }
    };
    unsafe { *len = reply.len() as i32 };
    FA_R_REPLY.with(|r| *r.borrow_mut() = reply.into_bytes());
}

/// R's .C("fa_r_take", strrep(" ", n)): copies the reply into that string.
///
/// # Safety
/// out must point to a writable string of at least the length fa_r_call reported.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fa_r_take(out: *mut *mut std::ffi::c_char) {
    FA_R_REPLY.with(|r| {
        let reply = r.borrow();
        unsafe { std::ptr::copy_nonoverlapping(reply.as_ptr(), *out as *mut u8, reply.len()) };
    });
}
"#;

pub fn port_rs(hs: &Hotspot, port: &Port) -> String {
    let params: Vec<String> = hs.params.iter().map(|p| format!("{}: serde_json::Value", snake(&p.name))).collect();
    format!(
        "//! Rust port of {id} ({file}:{line}), generated by fast-anything.\n\
         //! Replace the todo!() with a faithful, fast port, and the serde_json::Value types with real ones\n\
         //! (TASK.md says how R values arrive). Return Err(message) where R would stop().\n\
         //! Check it with:  fast-anything verify {id}\n\n\
         #[allow(unused_imports)]\n\
         use crate::r::{{List, Null, Num, Vector}};\n\n\
         #[allow(unused_variables)]\n\
         pub fn {func}({params}) -> Result<serde_json::Value, String> {{\n    todo!(\"port {id}\")\n}}\n",
        id = port.id,
        file = hs.file,
        line = hs.line,
        func = snake(&hs.name),
        params = params.join(", "),
    )
}

fn sample_calls(path: &Path, func: &str, n: usize) -> Vec<String> {
    let Ok(text) = fs::read_to_string(path) else { return vec![] };
    let clip = |s: String, n: usize| {
        if s.chars().count() <= n { s } else { format!("{}…", s.chars().take(n - 1).collect::<String>()) }
    };
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .take(n)
        .map(|case| {
            let args: Vec<String> =
                case["args"].as_array().map(|a| a.iter().map(|v| v.to_string()).collect()).unwrap_or_default();
            let outcome = match case.get("raises") {
                Some(e) => format!("stops ({})", e.as_str().unwrap_or("error")),
                None => case["result"].to_string(),
            };
            format!("{}  ->  {}", clip(format!("{func}({})", args.join(", ")), 110), clip(outcome, 60))
        })
        .collect()
}

pub fn task_md(hs: &Hotspot, helpers: &[Hotspot], port: &Port, crate_dir: &Path) -> String {
    let fence = "```";
    let samples = sample_calls(&crate_dir.join("recordings.jsonl"), &hs.name, 8).join("\n");
    let mut helper_md = String::new();
    if !hs.calls.is_empty() {
        helper_md.push_str("## Helpers it calls\n\nPort these as private Rust functions in src/port.rs (they stay in R for everyone else).\n\n");
        for name in &hs.calls {
            match helpers.iter().find(|h| &h.name == name) {
                Some(h) => helper_md.push_str(&format!("{fence}r\n{}\n{fence}\n\n", h.source)),
                None => helper_md.push_str(&format!("- `{name}`: read it in {}\n", hs.file)),
            }
        }
    }
    format!(
        "# Port `{id}` to Rust\n\n\
You're in a Rust crate fast-anything generated for this: `{dir}`.\n\
Write `{func}` in `src/port.rs` so it behaves **exactly** like the R below, and runs fast.\n\
`src/lib.rs` is generated glue (JSON in, R values out, through a C entry point): don't edit it.\n\
Only edit files in this folder. Don't touch the R project.\n\n\
## The original ({file}:{line})\n\n{fence}r\n{source}\n{fence}\n\n\
{helper_md}\
## Real calls\n\n\
fast-anything recorded {recorded} real calls (out of {seen} while the app ran). Arguments arrive as JSON, in parameter order. A few:\n\n{fence}\n{samples}\n{fence}\n\n\
## How R values arrive and go back\n\n\
- R has no scalars: a length-1 vector arrives as a plain value, a longer one as an array. Take vectors as `Vector<T>` (`crate::r::Vector`, accepts both; derefs to `Vec<T>`).\n\
- Integer and double vectors both arrive as numbers; take `f64` (or `i64` when the R code only uses whole numbers). NA arrives as null: `Option<T>`.\n\
- NaN, Inf and -Inf arrive as `{{\"$num\": \"NaN\"}}` etc.: take `Num` (`crate::r::Num`, a wrapped f64) where they can occur, and return `Num` to give them back.\n\
- Named vectors and named lists arrive as objects (`HashMap<String, _>` or a struct); unnamed lists as arrays.\n\
- The result goes back as R values: a number or string becomes a length-1 vector, an array of plain values a vector (`c(...)`), an object of plain values a named vector, anything nested a list. Wrap it in `List(...)` (`crate::r::List`) when R returns a list; `None` is NA, and `Null` is R's NULL.\n\
- Integer vs double doesn't matter to verify (R prints them alike); names, NA and list structure do. Where R stops with an error, return `Err(message)`.\n\n\
## Rules\n\n\
- Same result for every input, edge cases included: empty vectors, zero, negatives, NA, non-ASCII text, very long input. verify tests these too.\n\
- R strings index by character: use `.chars()` (or bytes only after checking `is_ascii()`). R indexes from 1.\n\
- Make it fast: no needless allocation, borrow, `with_capacity`, fast paths for common cases.\n\
- Keep the function name.\n\n\
## Check\n\n\
Run `cargo check` here. Then fast-anything builds the crate in release mode and replays all recorded calls against it\n\
(`fast-anything verify {id}`).\n",
        id = port.id,
        dir = crate_dir.display(),
        func = hs.name,
        file = hs.file,
        line = hs.line,
        source = hs.source,
        recorded = port.recorded,
        seen = crate::util::thousands(port.calls_seen),
    )
}

pub fn scaffold(ctx: &Ctx, hs: &Hotspot, helpers: &[Hotspot], port: &Port) -> Result<String> {
    let dir = ctx.crate_dir(port);
    fs::create_dir_all(dir.join("src"))?;
    fs::create_dir_all(dir.join(".cargo"))?;
    fs::write(dir.join("Cargo.toml"), cargo_toml(port))?;
    fs::write(dir.join(".cargo/config.toml"), backend::CARGO_CONFIG)?;
    fs::write(dir.join("src/lib.rs"), crate::native::glue_rs(hs, port, default_json) + R_GLUE)?;
    let body = dir.join("src/port.rs");
    if !body.exists() {
        fs::write(&body, port_rs(hs, port))?;
    }
    let task = task_md(hs, helpers, port, &dir);
    fs::write(dir.join("TASK.md"), &task)?;
    Ok(task)
}

pub fn verify(ctx: &Ctx, port: &Port) -> Result<Verify> {
    let rscript = ctx.runtime()?;
    let dir = ctx.crate_dir(port);
    let out = dir.join("verify.json");
    let (_, lib) = backend::Lang::R.artifact(&port.ext);
    let job = json!({
        "root": ctx.root, "lib": ctx.lib_dir().join(lib), "func": port.func,
        "module_file": ctx.root.join(&port.file), "cases": dir.join("recordings.rds"), "out": out, "budget": 0.5,
    });
    runner(ctx, rscript, "verify", job)?;
    let mut v: Verify = serde_json::from_str(&fs::read_to_string(&out)?).context("reading verify results")?;
    v.built = true;
    Ok(v)
}

// ------------------------------------------------------------------ the shim

/// R has no reliable "this file's folder": the shim looks for the port next to the file it was
/// sourced from (or the Rscript script), then where the project was when it was enabled.
pub fn shim(port: &Port, lib_rel: &str, lib_abs: &str) -> String {
    let f = &port.func;
    [
        "# Uses the verified Rust port. FAST_ANYTHING_DISABLE=1 runs the R above instead.".to_string(),
        format!("# Undo with:  fast-anything disable {}", port.id),
        format!("if (!nzchar(Sys.getenv(\"FAST_ANYTHING_DISABLE\"))) {f} <- local({{"),
        "  from <- c(tryCatch(dirname(normalizePath(sys.frame(1)$ofile)), error = function(e) NULL),".to_string(),
        "            tryCatch(dirname(normalizePath(sub(\"^--file=\", \"\", grep(\"^--file=\", commandArgs(FALSE), value = TRUE)[1]))), error = function(e) NULL))".to_string(),
        format!("  libs <- c(file.path(from, {}), {})", r_string(lib_rel), r_string(lib_abs)),
        "  lib <- libs[file.exists(libs)][1]".to_string(),
        format!("  if (is.na(lib)) {f} else tryCatch({{"),
        "    fa <- new.env()".to_string(),
        "    sys.source(file.path(dirname(lib), \"fast_anything.R\"), envir = fa)".to_string(),
        format!("    fa$.fa_port(lib, {f})"),
        format!("  }}, error = function(e) {f})"),
        "})".to_string(),
    ]
    .join("\n")
}

pub fn enable(ctx: &Ctx, port: &Port) -> Result<()> {
    let file = ctx.root.join(&port.file);
    let text = fs::read_to_string(&file).with_context(|| format!("reading {}", port.file))?;
    let from = file.parent().context("file has no folder")?;
    let (_, lib) = backend::Lang::R.artifact(&port.ext);
    let lib_path = ctx.lib_dir().join(lib);
    fs::write(ctx.lib_dir().join("fast_anything.R"), BRIDGE)?;
    let rel = forward_slashes(&relative(from, &lib_path));
    let abs = forward_slashes(&lib_path);
    let block = shim(port, &rel, &abs);
    let patched = if port.module == "__main__" {
        backend::insert_marked(&text, "#", &port.id, &block, main_guard)
    } else {
        backend::append_marked(&text, "#", &port.id, &block)
    };
    fs::write(&file, patched).with_context(|| format!("writing {}", port.file))
}

pub fn disable(ctx: &Ctx, port: &Port) -> Result<()> {
    let file = ctx.root.join(&port.file);
    let text = fs::read_to_string(&file).with_context(|| format!("reading {}", port.file))?;
    fs::write(&file, backend::strip_marked(&text, "#", &port.id)).with_context(|| format!("writing {}", port.file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_r_and_its_script() {
        assert!(is_r("Rscript") && is_r("/usr/bin/Rscript") && is_r("C:\\R\\bin\\Rscript.exe") && !is_r("R"));
        let args = |s: &str| s.split_whitespace().map(String::from).collect::<Vec<_>>();
        assert_eq!(entry(&args("--no-save app.R data.csv")).unwrap(), "app.R");
        assert!(entry(&args("--vanilla app.R")).unwrap_err().to_string().contains("startup file"));
        assert!(entry(&args("-e 1")).is_err());
        assert_eq!(main_guard("f <- function(x) x\n\nif (sys.nframe() == 0) {\n  main()\n}\n"), Some(20));
    }

    #[test]
    fn bridge_encodes_and_compares_like_r() {
        if !Command::new("Rscript").arg("--version").output().is_ok_and(|o| o.status.success()) {
            eprintln!("no Rscript on PATH, skipping");
            return;
        }
        let bridge = std::env::temp_dir().join(format!("fa-bridge-test-{}.R", std::process::id()));
        fs::write(&bridge, BRIDGE).unwrap();
        let check = r#"
            fa <- new.env(); sys.source(commandArgs(TRUE)[1], envir = fa)
            stopifnot(
              fa$.fa_json(2) == "2", fa$.fa_json(c(1.5, 2)) == "[1.5,2]", fa$.fa_json(0.1) == "0.1",
              fa$.fa_json(5L) == "5", fa$.fa_json(c(TRUE, NA)) == "[true,null]", fa$.fa_json(NULL) == "null",
              fa$.fa_json(c(NaN, Inf, -Inf)) == '[{"$num":"NaN"},{"$num":"Inf"},{"$num":"-Inf"}]',
              fa$.fa_json(c(a = 1, b = 2)) == '{"a":1,"b":2}', fa$.fa_json(character(0)) == "[]",
              fa$.fa_json(list(1, "x\n\"")) == '[1,"x\\n\\""]', fa$.fa_json("\u00e9") == '"\u00e9"',
              fa$.fa_json(list(a = list(1, 2))) == '{"a":[1,2]}'
            )
            stopifnot(inherits(tryCatch(fa$.fa_json(factor("a")), error = function(e) e), "fa_unsupported"))
            stopifnot(
              fa$.fa_same(5L, 5), fa$.fa_same(c(a = 1), c(a = 1 + 1e-12)), !fa$.fa_same(c(a = 1), c(b = 1)),
              fa$.fa_same(NA_real_, NA), fa$.fa_same(NA_character_, NA), !fa$.fa_same(NA, 1),
              fa$.fa_same(Inf, Inf), !fa$.fa_same(Inf, -Inf), fa$.fa_same(character(0), NULL),
              !fa$.fa_same(list(1), 1), fa$.fa_same(list(c(1L, 2L)), list(c(1, 2))), !fa$.fa_same("a", "b")
            )
        "#;
        let script = bridge.with_extension("check.R");
        fs::write(&script, check).unwrap();
        let out = Command::new("Rscript").arg(&script).arg(&bridge).output().unwrap();
        let _ = fs::remove_file(&bridge);
        let _ = fs::remove_file(&script);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }

    #[test]
    fn r_literals() {
        assert_eq!(r_string("a\"b\\\n\u{e9}"), "\"a\\\"b\\\\\\n\\U{e9}\"");
        assert_eq!(
            r_literal(&json!({ "lib": "C:\\x", "n": 2, "on": true, "none": null, "xs": [1, "a"] })),
            "list(\"lib\" = \"C:\\\\x\", \"n\" = 2, \"none\" = NULL, \"on\" = TRUE, \"xs\" = list(1, \"a\"))"
        );
        assert_eq!(default_json("2L").as_deref(), Some("2"));
        assert_eq!(default_json("TRUE").as_deref(), Some("true"));
        assert_eq!(default_json("NA").as_deref(), Some("null"));
        assert_eq!(default_json("'a'").as_deref(), Some("\"a\""));
        assert_eq!(default_json("c(1, 2)"), None);
    }
}
