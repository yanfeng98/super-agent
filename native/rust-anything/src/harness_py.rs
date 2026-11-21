//! The Python side of record, build, diff and bench: same cases, same markers, run by a
//! Python driver instead of Node.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::bridge::Manifest;
use crate::harness::{BenchRow, CASES, DiffOutcome};

const WORK_DIR: &str = ".rust-anything";

fn python() -> &'static str {
    if Command::new("python3")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        "python3"
    } else {
        "python"
    }
}

fn run(cmd: &mut Command) -> Result<()> {
    let _scratch = crate::harness::scratch_cwd(cmd)?;
    let output = cmd.output().context("running python")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: Vec<&str> = stderr.lines().rev().take(25).collect();
        bail!(
            "python failed ({}):\n{}",
            output.status,
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        );
    }
    Ok(())
}

fn driver(crate_dir: &Path) -> Result<PathBuf> {
    let work = crate_dir.join(WORK_DIR);
    std::fs::create_dir_all(&work)?;
    let path = work.join("driver.py");
    std::fs::write(&path, DRIVER_PY)?;
    Ok(path)
}

/// The folder the wrapper package lives in, and the package name.
pub fn package(crate_dir: &Path, manifest: &Manifest) -> (PathBuf, String) {
    (crate_dir.join("python"), manifest.lib_name())
}

pub fn record(crate_dir: &Path, cases: &[Value]) -> Result<PathBuf> {
    let driver = driver(crate_dir)?;
    let out = crate_dir.join(CASES);
    crate::harness::record_cases(
        cases,
        &crate_dir.join(WORK_DIR),
        &out,
        "python",
        |inputs, part| {
            let mut cmd = Command::new(python());
            cmd.arg(&driver)
                .arg("record")
                .arg(inputs)
                .arg(part)
                .env("PYTHONHASHSEED", "0")
                // Recording must not leave `__pycache__` folders in the source tree.
                .env("PYTHONDONTWRITEBYTECODE", "1");
            Ok(cmd)
        },
    )?;
    Ok(out)
}

pub fn build(crate_dir: &Path, manifest: &Manifest) -> Result<PathBuf> {
    let target = crate_dir.join("target");
    let output = Command::new("cargo")
        .args(["build", "--release", "--manifest-path"])
        .arg(crate_dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .context("running cargo build")?;
    if !output.status.success() {
        bail!(
            "cargo build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let lib = manifest.lib_name();
    let release = target.join("release");
    let built = [
        format!("lib{lib}.so"),
        format!("lib{lib}.dylib"),
        format!("{lib}.dll"),
    ]
    .iter()
    .map(|n| release.join(n))
    .find(|p| p.exists())
    .with_context(|| format!("no extension module for `{lib}` in {}", release.display()))?;
    let (dir, name) = package(crate_dir, manifest);
    let package_dir = dir.join(&name);
    std::fs::create_dir_all(&package_dir)?;
    crate::harness::install(&built, &native_file(crate_dir, manifest))?;
    Ok(package_dir)
}

/// The built extension module inside the Python package.
pub(crate) fn native_file(crate_dir: &Path, manifest: &Manifest) -> PathBuf {
    let (dir, name) = package(crate_dir, manifest);
    dir.join(name).join(if cfg!(windows) {
        "_native.pyd"
    } else {
        "_native.abi3.so"
    })
}

pub fn diff(crate_dir: &Path, manifest: &Manifest, ignore_key_order: bool) -> Result<DiffOutcome> {
    let cases = crate_dir.join(CASES);
    if !cases.exists() {
        bail!(
            "no {CASES} in {} (run `rust-anything record` first)",
            crate_dir.display()
        );
    }
    let driver = driver(crate_dir)?;
    let (dir, name) = package(crate_dir, manifest);
    let out = crate_dir.join(WORK_DIR).join("diff.json");
    run(Command::new(python())
        .arg(&driver)
        .arg("diff")
        .arg(&cases)
        .arg(&out)
        .arg(dir)
        .arg(name)
        .arg(if ignore_key_order { "sort" } else { "keep" })
        .env("PYTHONHASHSEED", "0")
        // Recording must not leave `__pycache__` folders in the source tree.
        .env("PYTHONDONTWRITEBYTECODE", "1"))?;
    Ok(serde_json::from_str(&std::fs::read_to_string(&out)?)?)
}

pub fn bench(crate_dir: &Path, manifest: &Manifest, min_ms: u64) -> Result<Vec<BenchRow>> {
    let cases = crate_dir.join(CASES);
    if !cases.exists() {
        bail!(
            "no {CASES} in {} (run `rust-anything record` first)",
            crate_dir.display()
        );
    }
    let driver = driver(crate_dir)?;
    let (dir, name) = package(crate_dir, manifest);
    let modules: serde_json::Map<String, Value> = manifest
        .exports
        .iter()
        .map(|f| {
            (
                f.name.clone(),
                Value::String(manifest.root.join(&f.file).to_string_lossy().into_owned()),
            )
        })
        .chain(manifest.classes.iter().map(|c| {
            (
                c.name.clone(),
                Value::String(manifest.root.join(&c.file).to_string_lossy().into_owned()),
            )
        }))
        .collect();
    let modules_path = crate_dir.join(WORK_DIR).join("bench-modules.json");
    std::fs::write(&modules_path, serde_json::to_string(&modules)?)?;
    let out = crate_dir.join(WORK_DIR).join("bench.json");
    run(Command::new(python())
        .arg(&driver)
        .arg("bench")
        .arg(&cases)
        .arg(&out)
        .arg(dir)
        .arg(name)
        .arg(&modules_path)
        .arg(min_ms.to_string())
        .env("PYTHONHASHSEED", "0")
        // Recording must not leave `__pycache__` folders in the source tree.
        .env("PYTHONDONTWRITEBYTECODE", "1"))?;
    Ok(serde_json::from_str(&std::fs::read_to_string(&out)?)?)
}

/// record / diff / bench for Python. Cases and markers match the Node scripts.
pub const DRIVER_PY: &str = r#"
import dataclasses
import importlib
import json
import math
import os
import sys
import time

BIG = 2 ** 53


def enc(v, stack=()):
    if v is None or isinstance(v, (bool, str)):
        return v
    if isinstance(v, int):
        return v if -BIG <= v <= BIG else {"$int": str(v)}
    if isinstance(v, float):
        if math.isnan(v):
            return {"$num": "NaN"}
        if math.isinf(v):
            return {"$num": "Infinity" if v > 0 else "-Infinity"}
        if v == 0 and math.copysign(1, v) < 0:
            return {"$num": "-0"}
        # 1.0 and 1 are different Python values: keep floats distinguishable.
        return {"$float": repr(v)} if v.is_integer() else v
    if any(v is s for s in stack):
        return {"$cycle": True}
    stack = stack + (v,)
    if isinstance(v, list):
        return [enc(x, stack) for x in v]
    if isinstance(v, tuple):
        return {"$tuple": [enc(x, stack) for x in v]}
    if isinstance(v, (set, frozenset)):
        return {"$set": sorted((enc(x, stack) for x in v), key=json.dumps)}
    if isinstance(v, (bytes, bytearray)):
        return {"$bytes": list(v)}
    if isinstance(v, dict):
        if all(isinstance(k, str) for k in v):
            return {k: enc(x, stack) for k, x in v.items()}
        return {"$dict": [[enc(k, stack), enc(x, stack)] for k, x in v.items()]}
    if hasattr(v, "__dict__"):
        return {"$object": type(v).__name__, "attrs": enc(vars(v), stack)}
    return {"$repr": type(v).__name__}


def dec(v):
    if isinstance(v, list):
        return [dec(x) for x in v]
    if not isinstance(v, dict):
        return v
    if len(v) == 1:
        (key, inner), = v.items()
        if key == "$undefined":
            return None
        if key == "$num":
            return {"NaN": math.nan, "Infinity": math.inf, "-Infinity": -math.inf, "-0": -0.0}[inner]
        if key == "$float":
            return float(inner)
        if key == "$int":
            return int(inner)
        if key == "$tuple":
            return tuple(dec(x) for x in inner)
        if key == "$set":
            return {dec(x) for x in inner}
        if key == "$bytes":
            return bytes(inner)
        if key == "$dict":
            return {dec(k): dec(x) for k, x in inner}
    if set(v) == {"$object", "attrs"} and isinstance(v["attrs"], dict):
        cls = dataclass_named(v["$object"])
        if cls is not None:
            return cls(**{k: dec(x) for k, x in v["attrs"].items()})
    return {k: dec(x) for k, x in v.items()}


def dataclass_named(name):
    """A dataclass the loaded modules define, for building a recorded `$object` argument."""
    for module in list(sys.modules.values()):
        cls = getattr(module, name, None)
        if isinstance(cls, type) and dataclasses.is_dataclass(cls):
            return cls
    return None


def canon(v, sort):
    return json.dumps(v, sort_keys=sort, ensure_ascii=False)


def load(path):
    """Imports a source file with its package context, so relative imports work."""
    path = os.path.abspath(path)
    directory, filename = os.path.split(path)
    parts = [os.path.splitext(filename)[0]]
    while os.path.exists(os.path.join(directory, "__init__.py")):
        directory, package = os.path.split(directory)
        parts.insert(0, package)
    if parts[-1] == "__init__":
        parts.pop()
    if directory not in sys.path:
        sys.path.insert(0, directory)
    return importlib.import_module(".".join(parts))


def error_text(e):
    return f"{type(e).__name__}: {e}"


def call_args(args):
    """Trailing `undefined` arguments are omitted, so Python defaults apply."""
    args = list(args)
    while args and args[-1] == {"$undefined": True}:
        args.pop()
    return [dec(a) for a in args]


def run_case(mod, c):
    if c.get("class"):
        cls = getattr(mod, c["class"])
        results = []
        try:
            instance = cls(*call_args(c["ctor"]))
            results.append({"ok": True})
        except (KeyboardInterrupt, SystemExit):
            raise
        except BaseException as e:  # noqa: BLE001
            results.append({"error": error_text(e)})
            return {"results": results}
        for call in c["calls"]:
            try:
                results.append({"ok": enc(getattr(instance, call["method"])(*call_args(call["args"])))})
            except (KeyboardInterrupt, SystemExit):
                raise
            except BaseException as e:  # noqa: BLE001
                results.append({"error": error_text(e)})
        return {"results": results}
    kwargs = {k: dec(v) for k, v in c.get("kwargs", {}).items()}
    try:
        return {"ok": enc(getattr(mod, c["fn"])(*call_args(c["args"]), **kwargs))}
    except (KeyboardInterrupt, SystemExit):
        raise
    except BaseException as e:  # noqa: BLE001
        return {"error": error_text(e)}


def record(inputs_path, out_path):
    cases = json.load(open(inputs_path, encoding="utf-8"))
    modules = {}
    progress = os.environ.get("RUST_ANYTHING_PROGRESS")
    with open(out_path, "w", encoding="utf-8") as out:
        for index, c in enumerate(cases):
            if progress:
                with open(progress, "a", encoding="utf-8") as p:
                    p.write("%d\n" % index)
            if c["module"] not in modules:
                modules[c["module"]] = load(c["module"])
            result = run_case(modules[c["module"]], c)
            line = {k: c[k] for k in ("fn", "class", "ctor", "calls", "args", "kwargs") if k in c}
            line.update(result)
            out.write(json.dumps(line, ensure_ascii=False) + "\n")
            out.flush()


def native(package_dir, name):
    sys.path.insert(0, package_dir)
    return importlib.import_module(name)


def error_type(text):
    return text.split(":", 1)[0]


def same(expected, actual, sort):
    """Exception types are part of a Python contract and must match; a Rust `TypeError` means
    the bindings rejected the arguments, which agrees with any error from the original."""
    if "error" in expected or "error" in actual:
        if not ("error" in expected and "error" in actual):
            return False
        return error_type(actual["error"]) in (error_type(expected["error"]), "TypeError")
    return canon(expected.get("ok"), sort) == canon(actual.get("ok"), sort)


def diff(cases_path, out_path, package_dir, name, key_order):
    mod = native(package_dir, name)
    sort = key_order == "sort"
    summary = {"total": 0, "passed": 0, "failed": 0, "missing": [], "mismatches": []}
    for line in open(cases_path, encoding="utf-8"):
        if not line.strip():
            continue
        c = json.loads(line)
        summary["total"] += 1
        name_ = c.get("class") or c["fn"]
        if not hasattr(mod, name_):
            if name_ not in summary["missing"]:
                summary["missing"].append(name_)
            summary["failed"] += 1
            continue
        got = run_case(mod, c)
        if c.get("class"):
            want, have = c["results"], got["results"]
            ok = len(want) == len(have) and all(same(w, h, sort) for w, h in zip(want, have))
            expected, actual = want, have
        else:
            expected = {"error": c["error"]} if "error" in c else {"ok": c.get("ok")}
            ok = same(expected, got, sort)
            actual = got
        if ok:
            summary["passed"] += 1
        else:
            summary["failed"] += 1
            summary["mismatches"].append({"fn": name_, "args": c.get("args", [c.get("ctor"), c.get("calls")]), "expected": expected, "actual": actual})
    json.dump(summary, open(out_path, "w", encoding="utf-8"), ensure_ascii=False)


def bench(cases_path, out_path, package_dir, name, modules_path, min_ms):
    new = native(package_dir, name)
    sources = json.load(open(modules_path, encoding="utf-8"))
    groups = {}
    for line in open(cases_path, encoding="utf-8"):
        if line.strip():
            c = json.loads(line)
            groups.setdefault(c.get("class") or c["fn"], []).append(c)
    rows = []
    for key, cases in groups.items():
        if key not in sources or not hasattr(new, key):
            continue
        old = load(sources[key])

        def timed(mod):
            if cases[0].get("class"):
                def once():
                    for c in cases:
                        run_case(mod, c)
            else:
                # Arguments are decoded once: only the calls are timed.
                fn = getattr(mod, key)
                prepared = [(call_args(c["args"]), {k: dec(v) for k, v in c.get("kwargs", {}).items()}) for c in cases]

                def once():
                    for args, kwargs in prepared:
                        try:
                            fn(*args, **kwargs)
                        except (KeyboardInterrupt, SystemExit):
                            raise
                        except BaseException:  # noqa: BLE001
                            pass
            once()
            passes, start = 0, time.perf_counter()
            while True:
                once()
                passes += 1
                elapsed = time.perf_counter() - start
                if elapsed * 1000 >= min_ms:
                    return elapsed * 1e9 / (passes * len(cases))
        original_ns, rust_ns = timed(old), timed(new)
        label = f"class {key}" if cases[0].get("class") else key
        rows.append({"fn": label, "cases": len(cases), "original_ns": original_ns, "rust_ns": rust_ns, "speedup": original_ns / rust_ns})
    json.dump(rows, open(out_path, "w", encoding="utf-8"))


if __name__ == "__main__":
    mode, *rest = sys.argv[1:]
    if mode == "record":
        record(*rest)
    elif mode == "diff":
        diff(*rest)
    elif mode == "bench":
        bench(rest[0], rest[1], rest[2], rest[3], rest[4], int(rest[5]))
"#;
