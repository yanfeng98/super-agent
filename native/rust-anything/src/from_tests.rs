//! `record --from-tests -- <test command>`: runs the original's own test suite and keeps every
//! call it makes to the unit's exported functions as recorded inputs, so the cases cover what
//! the project's authors thought worth testing, not only generated values.
//!
//! TS/JS: test runners (Jest, Vitest, ts-jest) load sources through their own module systems, so
//! no Node hook sees the calls. The unit's files get a few lines appended for the run instead,
//! wrapping each exported function declaration, and are restored from a backup afterwards (also
//! at the start of the next run, if a run was killed). Python: an import hook on `PYTHONPATH`
//! wraps the functions when their module loads; no file changes.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::bridge::Manifest;
use crate::lang::Lang;
use crate::walk::Canonical;

/// Captured calls kept per function: enough to cover a test suite, small enough to record.
const PER_FUNCTION: usize = 1000;
pub const TEST_INPUTS: &str = "test-inputs.jsonl";
const BACKUP: &str = ".rust-anything-backup";

#[derive(Debug)]
pub struct Capture {
    /// The captured inputs, one `{"fn", "args"}` per line (with `"kwargs"` for Python arguments
    /// passed by keyword), in the `--inputs` format.
    pub inputs: PathBuf,
    pub per_function: BTreeMap<String, usize>,
    /// Exports the capture cannot see (not a function declaration), with why.
    pub skipped: Vec<String>,
    pub tests_passed: bool,
}

pub fn capture(crate_dir: &Path, command: &[String]) -> Result<Capture> {
    let m = Manifest::load(crate_dir)?;
    let Some((program, args)) = command.split_first() else {
        bail!("--from-tests needs the test command after `--`, e.g. `-- npx jest`");
    };
    let raw = std::path::absolute(crate_dir.join("test-calls.raw.jsonl"))?;
    let _ = std::fs::remove_file(&raw);
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(&m.root)
        .stdout(std::io::stderr())
        .env("RUST_ANYTHING_CAPTURE", &raw)
        .env("CI", "1");
    let mut skipped = Vec::new();
    let status = match m.language {
        Lang::Ts => {
            let guard = instrument_js(&m, &mut skipped)?;
            let status = crate::process::status(&mut cmd, crate::process::TESTS);
            drop(guard);
            status
        }
        Lang::Python => {
            let hook = crate_dir.join("target").join("from-tests");
            std::fs::create_dir_all(&hook)?;
            std::fs::write(hook.join("sitecustomize.py"), python_hook())?;
            let mut want: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for f in &m.exports {
                let path = m.root.join(&f.file).canonical()?;
                want.entry(path.to_string_lossy().into_owned())
                    .or_default()
                    .push(f.name.clone());
            }
            let path = std::env::join_paths(
                std::iter::once(std::path::absolute(&hook)?).chain(
                    std::env::var_os("PYTHONPATH")
                        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
                        .unwrap_or_default(),
                ),
            )?;
            cmd.env("PYTHONPATH", path)
                .env("RUST_ANYTHING_CAPTURE_FUNCS", serde_json::to_string(&want)?)
                .env("PYTHONDONTWRITEBYTECODE", "1");
            crate::process::status(&mut cmd, crate::process::TESTS)
        }
        other => bail!(
            "--from-tests captures TS/JS and Python test suites; this unit is {}",
            other.name()
        ),
    }
    .with_context(|| format!("running `{}`", command.join(" ")))?;

    let text = std::fs::read_to_string(&raw).unwrap_or_default();
    let _ = std::fs::remove_file(&raw);
    let exported: HashSet<&str> = m.exports.iter().map(|f| f.name.as_str()).collect();
    let mut seen = HashSet::new();
    let mut per_function: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = String::new();
    for line in text.lines() {
        let Ok(call) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(name) = call["fn"].as_str() else {
            continue;
        };
        if !exported.contains(name) || !call["args"].is_array() || !seen.insert(line.to_string()) {
            continue;
        }
        let n = per_function.entry(name.to_string()).or_default();
        if *n >= PER_FUNCTION {
            continue;
        }
        *n += 1;
        out.push_str(line);
        out.push('\n');
    }
    let inputs = crate_dir.join(TEST_INPUTS);
    std::fs::write(&inputs, out).with_context(|| format!("writing {}", inputs.display()))?;
    Ok(Capture {
        inputs,
        per_function,
        skipped,
        tests_passed: status.success(),
    })
}

/// The inputs file `record` takes: the captured calls, after `own` inputs when there are some.
pub fn with_own_inputs(crate_dir: &Path, own: Option<&Path>, captured: &Path) -> Result<PathBuf> {
    let Some(own) = own else {
        return Ok(captured.to_path_buf());
    };
    let mut text =
        std::fs::read_to_string(own).with_context(|| format!("reading {}", own.display()))?;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text += &std::fs::read_to_string(captured)?;
    let combined = crate_dir.join("target").join("record-inputs.jsonl");
    std::fs::create_dir_all(crate_dir.join("target"))?;
    std::fs::write(&combined, text)?;
    Ok(combined)
}

/// Restores the instrumented files when dropped.
struct Restore(Vec<PathBuf>);

impl Drop for Restore {
    fn drop(&mut self) {
        for file in &self.0 {
            restore(file);
        }
    }
}

fn backup_of(file: &Path) -> PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(BACKUP);
    PathBuf::from(name)
}

fn restore(file: &Path) {
    let backup = backup_of(file);
    if std::fs::copy(&backup, file).is_ok() {
        let _ = std::fs::remove_file(backup);
    }
}

fn instrument_js(m: &Manifest, skipped: &mut Vec<String>) -> Result<Restore> {
    let mut by_file: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for f in &m.exports {
        by_file.entry(&f.file).or_default().push(&f.name);
    }
    let mut guard = Restore(Vec::new());
    let capture = JS_CAPTURE.replace("__CODEC__", &js_codec());
    for (file, names) in by_file {
        let path = m.root.join(file);
        restore(&path);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let wrappable: Vec<&str> = names
            .into_iter()
            .filter(|name| {
                let declared = regex::Regex::new(&format!(
                    r"\bfunction\s*\*?\s*{}\s*[<(]",
                    regex::escape(name)
                ))
                .is_ok_and(|re| re.is_match(&text));
                if !declared {
                    skipped.push(format!(
                        "{name} ({file}): not a function declaration, so its binding cannot be wrapped"
                    ));
                }
                declared
            })
            .collect();
        if wrappable.is_empty() {
            continue;
        }
        std::fs::copy(&path, backup_of(&path))
            .with_context(|| format!("backing up {}", path.display()))?;
        guard.0.push(path.clone());
        let ts = file.ends_with(".ts") || file.ends_with(".tsx") || file.ends_with(".mts");
        let wraps: String = wrappable
            .iter()
            .map(|n| format!("  {n} = __raWrap('{n}', {n})\n"))
            .collect();
        let instrumented = format!(
            "{}{text}\n{}",
            if ts { "// @ts-nocheck\n" } else { "" },
            capture.replace("__WRAPS__", &wraps)
        );
        std::fs::write(&path, instrumented)
            .with_context(|| format!("instrumenting {}", path.display()))?;
    }
    Ok(guard)
}

const JS_CAPTURE: &str = r#"// rust-anything record --from-tests: removed when the tests finish.
/* c8 ignore start */
/* istanbul ignore next */
{
  const __raFs = globalThis.process?.getBuiltinModule?.('node:fs')
  const __raOut = globalThis.process?.env?.RUST_ANYTHING_CAPTURE
  const __raCount = new globalThis.Map()
  // The built-in name of a typed array, which a subclass cannot change.
  const __raTypedName = globalThis.Object.getOwnPropertyDescriptor(globalThis.Object.getPrototypeOf(globalThis.Uint8Array.prototype), globalThis.Symbol.toStringTag).get
__CODEC__
  const __raWrap = (name, orig) => {
    if (!__raFs || !__raOut || typeof orig !== 'function') return orig
    const wrapped = function (...args) {
      const n = __raCount.get(name) ?? 0
      if (n < 5000) {
        __raCount.set(name, n + 1)
        try { __raFs.appendFileSync(__raOut, globalThis.JSON.stringify({ fn: name, args: globalThis.Array.from(args, a => __raEnc(a)) }) + '\n') } catch {}
      }
      return new.target ? globalThis.Reflect.construct(orig, args, new.target) : orig.apply(this, args)
    }
    if (typeof exports === 'object' && exports !== null) {
      for (const key of [name, 'default']) if (exports[key] === orig) exports[key] = wrapped
    }
    return wrapped
  }
__WRAPS__}
/* c8 ignore stop */
"#;

/// What the capture hook changes in the `enc` of `CODEC_JS`: a value `dec` cannot give back
/// throws, so the call is left out. That is a function, symbol, cycle, RegExp or Error, an
/// object `dec` would rebuild as a plain one although it is not (a `URL`, an `ArrayBuffer`, a
/// boxed string, anything with `toJSON`, or a RegExp, Error, Map, Set or Date from another
/// realm), and a typed array keeps its built-in kind (a subclass is not a global `dec` can find).
/// Array holes stay `undefined`, as `JSON.stringify` saw them before. A class instance is still
/// captured as its own fields, as before.
const JS_CAPTURE_ENC: &[(&str, &str)] = &[
    ("return { $function: v.name }", NOT_DATA),
    ("return { $symbol: String(v.description) }", NOT_DATA),
    ("return { $cycle: true }", NOT_DATA),
    ("return { $regexp: String(v) }", NOT_DATA),
    ("return { $error: v.message }", NOT_DATA),
    (
        "if (Array.isArray(v)) return v.map(x => enc(x, next))",
        "if (Array.isArray(v)) return Array.from(v, x => enc(x, next))",
    ),
    ("kind: v.constructor.name,", "kind: __raTypedName.call(v),"),
    (
        "const plain = Object.fromEntries(",
        "if (Object.prototype.toString.call(v) !== '[object Object]' || typeof v.toJSON === 'function') throw new Error('not data')\n  const plain = Object.fromEntries(",
    ),
];

const NOT_DATA: &str = "throw new Error('not data')";

/// The `enc` of `CODEC_JS`, which `record` uses, for the capture hook, changed as
/// `JS_CAPTURE_ENC` says. It runs inside the user's module: its names take a `__ra` prefix so
/// they cannot shadow a wrapped function of the same name, and the built-ins it uses are read
/// from `globalThis` so the module's own names cannot shadow them. Strings and comments are left
/// as they are.
fn js_codec() -> String {
    let codec = crate::harness::CODEC_JS;
    let end = codec.find("\nconst dec = ").unwrap_or(codec.len());
    let enc = JS_CAPTURE_ENC
        .iter()
        .fold(codec[..end].to_string(), |enc, (from, to)| {
            enc.replace(from, to)
        });
    js_names()
        .replace_all(&enc, |c: &regex::Captures| {
            match c.get(1).map(|m| m.as_str()) {
                None => c[0].to_string(),
                Some("enc") => "__raEnc".to_string(),
                Some("MARKERS") => "__raMarkers".to_string(),
                Some("looksLikeMarker") => "__raLooksLikeMarker".to_string(),
                Some(global) => format!("globalThis.{global}"),
            }
        })
        .into_owned()
}

/// The built-ins the codec uses.
const JS_GLOBALS: &str =
    "Array|ArrayBuffer|Buffer|DataView|Date|Error|Map|Number|Object|RegExp|Set|String|Uint8Array";

/// The codec's own names and the built-ins it uses, outside strings and `//` comments (those
/// match whole, without a group).
fn js_names() -> regex::Regex {
    regex::Regex::new(&format!(
        r"'[^']*'|//[^\n]*|\b(enc|MARKERS|looksLikeMarker|{JS_GLOBALS})\b"
    ))
    .expect("valid regex")
}

fn python_hook() -> String {
    let driver = crate::harness_py::DRIVER_PY;
    let enc = driver
        .find("def enc(")
        .and_then(|start| {
            driver[start..]
                .find("\ndef dec(")
                .map(|end| &driver[start..start + end])
        })
        .unwrap_or("def enc(v, stack=()):\n    return v\n");
    format!("{PY_HOOK_HEAD}\n{enc}\n{PY_HOOK_TAIL}")
}

const PY_HOOK_HEAD: &str = r#"# rust-anything record --from-tests: wraps the unit's functions when their module loads.
import functools
import importlib.abc
import importlib.machinery
import inspect
import json
import math
import os
import sys

BIG = 2 ** 53
"#;

const PY_HOOK_TAIL: &str = r#"
_OUT = os.environ.get("RUST_ANYTHING_CAPTURE")
_WANT = json.loads(os.environ.get("RUST_ANYTHING_CAPTURE_FUNCS") or "{}")
_COUNT = {}


def _wrap(name, orig):
    try:
        sig = inspect.signature(orig)
    except (TypeError, ValueError):
        sig = None

    @functools.wraps(orig)
    def wrapped(*args, **kwargs):
        n = _COUNT.get(name, 0)
        if n < 5000 and sig is not None:
            try:
                bound = sig.bind(*args, **kwargs)
                case = {"fn": name, "args": enc(list(bound.args))}
                # Keyword-only arguments, and those named after a skipped default.
                if bound.kwargs:
                    case["kwargs"] = {k: enc(v) for k, v in bound.kwargs.items()}
                line = json.dumps(case, ensure_ascii=False, allow_nan=False)
                with open(_OUT, "a", encoding="utf-8") as f:
                    f.write(line + "\n")
                _COUNT[name] = n + 1
            except Exception:
                pass
        return orig(*args, **kwargs)

    return wrapped


class _Loader(importlib.abc.Loader):
    def __init__(self, inner, names):
        self.inner = inner
        self.names = names

    def create_module(self, spec):
        return self.inner.create_module(spec)

    def exec_module(self, module):
        self.inner.exec_module(module)
        for name in self.names:
            orig = getattr(module, name, None)
            if callable(orig):
                setattr(module, name, _wrap(name, orig))


class _Finder(importlib.abc.MetaPathFinder):
    def find_spec(self, fullname, path, target=None):
        spec = importlib.machinery.PathFinder.find_spec(fullname, path)
        if spec is None or spec.origin is None or spec.loader is None:
            return None
        names = _WANT.get(os.path.realpath(spec.origin))
        if not names:
            return None
        spec.loader = _Loader(spec.loader, names)
        return spec


if _OUT and _WANT:
    sys.meta_path.insert(0, _Finder())
"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// `CODEC_JS` and the capture hook's copy of its `enc` must not drift apart in silence.
    #[test]
    fn the_capture_codec_is_the_record_codec_with_its_changes() {
        let codec = crate::harness::CODEC_JS;
        let end = codec.find("\nconst dec = ").expect("CODEC_JS defines dec");
        for (from, _) in JS_CAPTURE_ENC {
            let found = codec[..end].matches(from).count();
            assert_eq!(found, 1, "enc no longer has {from:?}");
        }
        let capture = js_codec();
        for marker in ["$function", "$symbol", "$cycle", "$regexp", "$error"] {
            assert!(!capture.contains(marker), "{marker} is written:\n{capture}");
        }
        // Every top-level name is prefixed, and nothing after `enc` came along.
        let declared: Vec<&str> = capture
            .lines()
            .filter_map(|l| l.strip_prefix("const "))
            .filter_map(|l| l.split([' ', '=']).next())
            .collect();
        let prefixed = ["__raEnc", "__raMarkers", "__raLooksLikeMarker"];
        assert_eq!(declared, prefixed, "{capture}");
        // Built-ins outside strings and comments are read from `globalThis`; strings stay.
        let globals = regex::Regex::new(&format!(
            r"'[^']*'|//[^\n]*|(globalThis\.)?\b(enc|{JS_GLOBALS})\b"
        ))
        .unwrap();
        for c in globals.captures_iter(&capture) {
            let bare = c.get(2).is_some() && c.get(1).is_none();
            assert!(!bare, "bare {}:\n{capture}", &c[0]);
        }
        assert!(capture.contains("'[object Object]'"), "{capture}");
    }
}
