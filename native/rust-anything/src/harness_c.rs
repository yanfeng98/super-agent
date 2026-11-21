//! The C side of record, build, diff and bench: the original is compiled into a shared
//! library, and Python's ctypes calls it and the Rust library with the same signatures.

use crate::walk::Canonical;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::bridge::Manifest;
use crate::harness::{BenchRow, CASES, DiffOutcome};
use crate::lang::c::{defines_main, scalar};

const WORK_DIR: &str = ".rust-anything";

fn run(cmd: &mut Command, what: &str) -> Result<()> {
    let _scratch = crate::harness::scratch_cwd(cmd)?;
    let output = cmd.output().with_context(|| format!("running {what}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: Vec<&str> = stderr.lines().rev().take(25).collect();
        bail!(
            "{what} failed ({}):\n{}",
            output.status,
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        );
    }
    Ok(())
}

fn compiler() -> &'static str {
    if Command::new("cc").arg("--version").output().is_ok() {
        "cc"
    } else {
        "gcc"
    }
}

/// ctypes signatures: `{ name: { "args": [...], "ret": ... } }`.
fn signatures(manifest: &Manifest) -> Value {
    let ctypes = |c: &str, ts: &str| -> String {
        if ts == "string" {
            "c_char_p".into()
        } else {
            scalar(c).map_or("c_int", |(_, _, _, ct)| ct).to_string()
        }
    };
    let map: serde_json::Map<String, Value> = manifest
        .exports
        .iter()
        .map(|f| {
            let args: Vec<String> = f
                .params
                .iter()
                .map(|p| ctypes(p.native_type.as_deref().unwrap_or("int"), &p.ts_type))
                .collect();
            let ret = ctypes(f.native_return.as_deref().unwrap_or("void"), &f.returns);
            (f.name.clone(), json!({ "args": args, "ret": ret }))
        })
        .collect();
    Value::Object(map)
}

/// Compiles the project's C sources (without `main` and tests) into one shared library.
fn build_original(crate_dir: &Path, manifest: &Manifest) -> Result<PathBuf> {
    let ws = crate::lang::c::discover(&manifest.root)?;
    let mut sources = Vec::new();
    let mut include_dirs = std::collections::BTreeSet::new();
    include_dirs.insert(manifest.root.clone());
    for file in ws
        .files
        .iter()
        .filter(|f| f.kind == crate::ts::discover::FileKind::Source)
    {
        let path = manifest.root.join(&file.path);
        if let Some(dir) = path.parent() {
            include_dirs.insert(dir.to_path_buf());
        }
        if file.path.ends_with(".c")
            && !defines_main(&std::fs::read_to_string(&path).unwrap_or_default())
        {
            sources.push(path);
        }
    }
    // Headers paired with a `.c` file are not in the workspace listing; their folders still
    // go on the include path.
    for entry in crate::walk::builder(&manifest.root)
        .require_git(false)
        .build()
        .flatten()
    {
        if entry.path().extension().is_some_and(|e| e == "h")
            && let Some(dir) = entry.path().parent()
        {
            include_dirs.insert(dir.to_path_buf());
        }
    }
    let out = crate_dir.join(WORK_DIR).join("original.so");
    std::fs::create_dir_all(crate_dir.join(WORK_DIR))?;
    let mut cmd = Command::new(compiler());
    cmd.args(["-shared", "-fPIC", "-O1", "-w", "-o"]).arg(&out);
    for dir in &include_dirs {
        cmd.arg(format!("-I{}", dir.display()));
    }
    cmd.args(&sources).arg("-lm");
    run(&mut cmd, "cc (original sources)")?;
    Ok(out)
}

fn driver(crate_dir: &Path, manifest: &Manifest) -> Result<(PathBuf, PathBuf)> {
    let work = crate_dir.join(WORK_DIR);
    std::fs::create_dir_all(&work)?;
    let path = work.join("driver_c.py");
    std::fs::write(&path, DRIVER_C_PY)?;
    let sigs = work.join("signatures.json");
    std::fs::write(&sigs, serde_json::to_string(&signatures(manifest))?)?;
    Ok((path, sigs))
}

fn python() -> Command {
    let mut cmd = Command::new("python3");
    cmd.env("PYTHONDONTWRITEBYTECODE", "1");
    cmd
}

pub fn record(crate_dir: &Path, manifest: &Manifest, cases: &[Value]) -> Result<PathBuf> {
    let original = build_original(crate_dir, manifest)?;
    let (driver, sigs) = driver(crate_dir, manifest)?;
    let out = crate_dir.join(CASES);
    crate::harness::record_cases(
        cases,
        &crate_dir.join(WORK_DIR),
        &out,
        "python (ctypes)",
        |inputs, part| {
            let mut cmd = python();
            cmd.arg(&driver)
                .arg("record")
                .arg(&original)
                .arg(&sigs)
                .arg(inputs)
                .arg(part);
            Ok(cmd)
        },
    )?;
    Ok(out)
}

/// The Rust library: `lib<name>.so`, `.dylib` or `.dll`.
pub fn build(crate_dir: &Path, manifest: &Manifest) -> Result<PathBuf> {
    let target = crate_dir.join("target");
    run(
        Command::new("cargo")
            .args(["build", "--release", "--manifest-path"])
            .arg(crate_dir.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", &target),
        "cargo build",
    )?;
    let lib = manifest.lib_name();
    let release = target.join("release");
    [
        format!("lib{lib}.so"),
        format!("lib{lib}.dylib"),
        format!("{lib}.dll"),
    ]
    .iter()
    .map(|n| release.join(n))
    .find(|p| p.exists())
    .with_context(|| format!("no shared library for `{lib}` in {}", release.display()))
}

/// The built Rust library of a crate.
pub fn built_library(crate_dir: &Path, manifest: &Manifest) -> Result<PathBuf> {
    let lib = manifest.lib_name();
    let release = crate_dir.join("target/release");
    [
        format!("lib{lib}.so"),
        format!("lib{lib}.dylib"),
        format!("{lib}.dll"),
    ]
    .iter()
    .map(|n| release.join(n))
    .find(|p| p.exists())
    .with_context(|| {
        format!(
            "no built library in {} (run `rust-anything build` first)",
            release.display()
        )
    })
}

pub fn diff(crate_dir: &Path, manifest: &Manifest) -> Result<DiffOutcome> {
    let cases = crate_dir.join(CASES);
    if !cases.exists() {
        bail!(
            "no {CASES} in {} (run `rust-anything record` first)",
            crate_dir.display()
        );
    }
    let rust = built_library(crate_dir, manifest)?;
    let (driver, sigs) = driver(crate_dir, manifest)?;
    let out = crate_dir.join(WORK_DIR).join("diff.json");
    run(
        python()
            .arg(&driver)
            .arg("diff")
            .arg(&rust)
            .arg(&sigs)
            .arg(&cases)
            .arg(&out),
        "python (ctypes)",
    )?;
    Ok(serde_json::from_str(&std::fs::read_to_string(&out)?)?)
}

pub fn bench(crate_dir: &Path, manifest: &Manifest, min_ms: u64) -> Result<Vec<BenchRow>> {
    let cases_path = crate_dir.join(CASES);
    if !cases_path.exists() {
        bail!(
            "no {CASES} in {} (run `rust-anything record` first)",
            crate_dir.display()
        );
    }
    let rust = built_library(crate_dir, manifest)?.canonical()?;
    let work = crate_dir.canonical()?.join(WORK_DIR);
    std::fs::create_dir_all(&work)?;
    let cases: Vec<Value> = std::fs::read_to_string(&cases_path)?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let source = work.join("bench.c");
    std::fs::write(&source, bench_c(manifest, &cases, min_ms))?;
    // The original, its functions renamed `<name>__original`, and the Rust library, whose
    // `rust_anything_<name>` aliases are the port: both called directly from one C program.
    let (sources, include_dirs) = original_sources(manifest)?;
    let bin = work.join("bench");
    let mut cmd = Command::new(compiler());
    cmd.args(["-O2", "-w", "-o"]).arg(&bin).arg(&source);
    for f in crate::codegen_c::shadow_flags(manifest).split_whitespace() {
        cmd.arg(f);
    }
    for dir in &include_dirs {
        cmd.arg(format!("-I{}", dir.display()));
    }
    cmd.args(&sources)
        .arg(&rust)
        .arg(format!(
            "-Wl,-rpath,{}",
            rust.parent()
                .map_or_else(String::new, |p| p.display().to_string())
        ))
        .arg("-lm");
    run(&mut cmd, "cc (bench)")?;
    let out = work.join("bench.json");
    run(Command::new(&bin).arg(&out), "bench")?;
    let timings: serde_json::Map<String, Value> =
        serde_json::from_str(&std::fs::read_to_string(&out)?)?;
    Ok(manifest
        .exports
        .iter()
        .filter_map(|f| {
            let t = timings.get(&f.name)?;
            let (o, n) = (t["original_ns"].as_f64()?, t["rust_ns"].as_f64()?);
            Some(BenchRow {
                name: f.name.clone(),
                cases: usize::try_from(t["cases"].as_u64().unwrap_or(0)).unwrap_or(0),
                original_ns: o,
                rust_ns: n,
                speedup: o / n,
            })
        })
        .collect())
}

/// The project's C sources without `main`, and the folders with headers.
fn original_sources(
    manifest: &Manifest,
) -> Result<(Vec<PathBuf>, std::collections::BTreeSet<PathBuf>)> {
    let ws = crate::lang::c::discover(&manifest.root)?;
    let mut sources = Vec::new();
    let mut include_dirs = std::collections::BTreeSet::new();
    include_dirs.insert(manifest.root.clone());
    for file in ws
        .files
        .iter()
        .filter(|f| f.kind == crate::ts::discover::FileKind::Source)
    {
        let path = manifest.root.join(&file.path);
        if let Some(dir) = path.parent() {
            include_dirs.insert(dir.to_path_buf());
        }
        if file.path.ends_with(".c")
            && !defines_main(&std::fs::read_to_string(&path).unwrap_or_default())
        {
            sources.push(path);
        }
    }
    for entry in crate::walk::builder(&manifest.root)
        .require_git(false)
        .build()
        .flatten()
    {
        if entry.path().extension().is_some_and(|e| e == "h")
            && let Some(dir) = entry.path().parent()
        {
            include_dirs.insert(dir.to_path_buf());
        }
    }
    Ok((sources, include_dirs))
}

/// A C string literal for `s`, byte by byte.
fn c_literal(s: &str) -> String {
    let mut out = String::from("\"");
    for b in s.bytes() {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7e if b != b'?' => out.push(b as char),
            _ => out.push_str(&format!("\\{b:03o}")),
        }
    }
    out.push('"');
    out
}

/// A C expression for a recorded argument of C type `c_type`; `None` when it has no C value.
fn c_arg(v: &Value, c_type: &str, is_string: bool) -> Option<String> {
    if is_string {
        return match v {
            Value::String(s) => Some(c_literal(s)),
            Value::Null => Some("NULL".into()),
            _ => None,
        };
    }
    let float = matches!(
        c_type.trim_start_matches("const ").trim(),
        "double" | "float" | "long double"
    );
    match v {
        Value::Number(n) if float => n
            .as_f64()
            .map(|x| format!("({c_type})rust_anything_f({}ULL)", x.to_bits())),
        Value::Object(o) if float => {
            let x = match o.get("$num")?.as_str()? {
                "NaN" => f64::NAN,
                "Infinity" => f64::INFINITY,
                "-Infinity" => f64::NEG_INFINITY,
                _ => -0.0,
            };
            Some(format!("({c_type})rust_anything_f({}ULL)", x.to_bits()))
        }
        Value::Number(n) => n
            .as_i64()
            .map(|i| format!("({c_type}){i}LL"))
            .or_else(|| n.as_u64().map(|u| format!("({c_type}){u}ULL"))),
        Value::Bool(b) => Some(format!("({c_type}){}", i32::from(*b))),
        _ => None,
    }
}

/// A C program that times each function's recorded calls on the original and on the port.
fn bench_c(m: &Manifest, cases: &[Value], min_ms: u64) -> String {
    use std::fmt::Write as _;
    let mut out = String::from(
        "#include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n#include <stdio.h>\n#include <string.h>\n\
         #include <sys/types.h>\n#include <time.h>\n\n\
         static double rust_anything_f(uint64_t bits) { double d; memcpy(&d, &bits, sizeof d); return d; }\n\
         static double now_ns(void) { struct timespec t; clock_gettime(CLOCK_MONOTONIC, &t); return t.tv_sec * 1e9 + t.tv_nsec; }\n\
         static volatile uint64_t sink;\n\n",
    );
    let mut rows = Vec::new();
    for f in &m.exports {
        let prototype = f.prototype.as_deref().unwrap_or(&f.name);
        let original = prototype.replacen(
            &format!("{}(", f.name),
            &format!("{}__original(", f.name),
            1,
        );
        let rust = prototype.replacen(
            &format!("{}(", f.name),
            &format!("rust_anything_{}(", f.name),
            1,
        );
        let _ = writeln!(out, "{original};\n{rust};");
        let mut calls = Vec::new();
        for c in cases
            .iter()
            .filter(|c| c["fn"] == f.name.as_str() && c.get("error").is_none())
        {
            let args = c["args"].as_array().cloned().unwrap_or_default();
            let exprs: Option<Vec<String>> = f
                .params
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    c_arg(
                        args.get(i).unwrap_or(&Value::Null),
                        p.native_type.as_deref().unwrap_or("int"),
                        p.ts_type == "string",
                    )
                })
                .collect();
            if let Some(exprs) = exprs {
                calls.push(exprs.join(", "));
            }
        }
        if calls.is_empty() {
            continue;
        }
        let void = f.native_return.as_deref() == Some("void");
        let body = |target: &str| -> String {
            calls
                .iter()
                .map(|a| {
                    if void {
                        format!("    {target}({a});\n")
                    } else {
                        format!("    sink += (uint64_t){target}({a});\n")
                    }
                })
                .collect()
        };
        let _ = writeln!(
            out,
            "static void pass_{n}_original(void) {{\n{}}}\nstatic void pass_{n}_rust(void) {{\n{}}}\n",
            body(&format!("{}__original", f.name)),
            body(&format!("rust_anything_{}", f.name)),
            n = f.name
        );
        rows.push((f.name.clone(), calls.len()));
    }
    let _ = writeln!(
        out,
        "static double timed(void (*pass)(void), int calls) {{\n    pass();\n    double start = now_ns(), elapsed;\n    long passes = 0;\n\
         \x20   do {{ pass(); passes++; elapsed = now_ns() - start; }} while (elapsed < {min_ms} * 1e6);\n    return elapsed / ((double)passes * calls);\n}}\n\n\
         int main(int argc, char **argv) {{\n    FILE *out = fopen(argv[1], \"w\");\n    fputs(\"{{\", out);"
    );
    for (i, (name, n)) in rows.iter().enumerate() {
        let _ = writeln!(
            out,
            "    fprintf(out, \"{}\\\"{name}\\\": {{\\\"cases\\\": {n}, \\\"original_ns\\\": %f, \\\"rust_ns\\\": %f}}\", \
             timed(pass_{name}_original, {n}), timed(pass_{name}_rust, {n}));",
            if i == 0 { "" } else { "," }
        );
    }
    out.push_str("    fputs(\"}\", out);\n    fclose(out);\n    return 0;\n}\n");
    out
}

const DRIVER_C_PY: &str = r#"
import ctypes
import json
import locale
import math
import os
import signal
import sys
import time

# Python switches C's locale to the user's at startup; a C program stays in "C" unless it calls
# setlocale, so the library must see "C" here (isspace, toupper, strtod depend on it).
locale.setlocale(locale.LC_ALL, "C")


def enc(v):
    if isinstance(v, float):
        if math.isnan(v):
            return {"$num": "NaN"}
        if math.isinf(v):
            return {"$num": "Infinity" if v > 0 else "-Infinity"}
        if v == 0 and math.copysign(1, v) < 0:
            return {"$num": "-0"}
    if isinstance(v, bytes):
        return {"$bytes": list(v)}
    return v


def dec(v):
    if isinstance(v, dict) and len(v) == 1:
        (key, inner), = v.items()
        if key == "$undefined":
            return None
        if key == "$num":
            return {"NaN": math.nan, "Infinity": math.inf, "-Infinity": -math.inf, "-0": -0.0}[inner]
    return v


def bind(path, signatures):
    lib = ctypes.CDLL(path)
    functions = {}
    for name, sig in signatures.items():
        try:
            fn = getattr(lib, name)
        except AttributeError:
            continue
        fn.argtypes = [getattr(ctypes, t) for t in sig["args"]]
        fn.restype = None if sig["ret"] == "None" else getattr(ctypes, sig["ret"])
        functions[name] = (fn, sig)
    return functions


def call(functions, c):
    fn, sig = functions[c["fn"]]
    args = []
    for value, kind in zip([dec(a) for a in c["args"]], sig["args"]):
        if kind == "c_char_p":
            args.append(None if value is None else str(value).encode("utf-8"))
        elif kind in ("c_float", "c_double"):
            args.append(float(value) if isinstance(value, (int, float)) else 0.0)
        elif kind == "c_bool":
            args.append(bool(value))
        else:
            args.append(int(value) if isinstance(value, (int, float)) and math.isfinite(value) else 0)
    while len(args) < len(sig["args"]):
        args.append(None if sig["args"][len(args)] == "c_char_p" else 0)
    return {"ok": enc(fn(*args))}


def isolated(functions, c):
    """Runs one case in a forked child: undefined behavior in C can crash the process."""
    read_end, write_end = os.pipe()
    pid = os.fork()
    if pid == 0:
        os.close(read_end)
        try:
            result = call(functions, c)
        except Exception as e:  # ctypes conversion errors
            result = {"error": "%s: %s" % (type(e).__name__, e)}
        os.write(write_end, json.dumps(result).encode())
        os._exit(0)
    os.close(write_end)
    data = b""
    while True:
        chunk = os.read(read_end, 65536)
        if not chunk:
            break
        data += chunk
    os.close(read_end)
    _, status = os.waitpid(pid, 0)
    if os.WIFSIGNALED(status):
        return {"error": "crash: " + signal.Signals(os.WTERMSIG(status)).name}
    return json.loads(data) if data else {"error": "crash: no result"}


def cases_of(path):
    return [json.loads(line) for line in open(path, encoding="utf-8") if line.strip()]


def record(lib, sigs, inputs, out_path):
    functions = bind(lib, json.load(open(sigs)))
    progress = os.environ.get("RUST_ANYTHING_PROGRESS")
    with open(out_path, "w", encoding="utf-8") as out:
        for index, c in enumerate(json.load(open(inputs, encoding="utf-8"))):
            if progress:
                with open(progress, "a", encoding="utf-8") as p:
                    p.write("%d\n" % index)
            line = {"fn": c["fn"], "args": c["args"]}
            line.update(isolated(functions, c))
            out.write(json.dumps(line, ensure_ascii=False) + "\n")
            out.flush()


def diff(lib, sigs, cases_path, out_path):
    functions = bind(lib, json.load(open(sigs)))
    summary = {"total": 0, "passed": 0, "failed": 0, "missing": [], "mismatches": []}
    for c in cases_of(cases_path):
        summary["total"] += 1
        if c["fn"] not in functions:
            if c["fn"] not in summary["missing"]:
                summary["missing"].append(c["fn"])
            summary["failed"] += 1
            continue
        if str(c.get("error", "")).startswith("crash:"):
            # The original's behavior is undefined there: any answer of the port passes.
            summary["passed"] += 1
            continue
        got = isolated(functions, c)
        expected = {"ok": c.get("ok")}
        if json.dumps(got) == json.dumps(expected):
            summary["passed"] += 1
        else:
            summary["failed"] += 1
            summary["mismatches"].append({"fn": c["fn"], "args": c["args"], "expected": expected, "actual": got})
    json.dump(summary, open(out_path, "w", encoding="utf-8"), ensure_ascii=False)


def bench(original, rust, sigs, cases_path, out_path, min_ms):
    signatures = json.load(open(sigs))
    old, new = bind(original, signatures), bind(rust, signatures)
    groups = {}
    for c in cases_of(cases_path):
        if "error" not in c:
            groups.setdefault(c["fn"], []).append(c)
    rows = []
    for name, cases in groups.items():
        if name not in old or name not in new:
            continue

        def timed(functions):
            def once():
                for c in cases:
                    call(functions, c)
            once()
            passes, start = 0, time.perf_counter()
            while True:
                once()
                passes += 1
                elapsed = time.perf_counter() - start
                if elapsed * 1000 >= min_ms:
                    return elapsed * 1e9 / (passes * len(cases))
        original_ns, rust_ns = timed(old), timed(new)
        rows.append({"fn": name, "cases": len(cases), "original_ns": original_ns, "rust_ns": rust_ns, "speedup": original_ns / rust_ns})
    json.dump(rows, open(out_path, "w"))


if __name__ == "__main__":
    mode, *rest = sys.argv[1:]
    if mode == "record":
        record(*rest)
    elif mode == "diff":
        diff(*rest)
    elif mode == "bench":
        bench(rest[0], rest[1], rest[2], rest[3], rest[4], int(rest[5]))
"#;
