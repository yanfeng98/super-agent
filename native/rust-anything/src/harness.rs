//! Steps 3 and 5's runtime half: record what the original code does, build the Rust module, and
//! replay the recorded cases against it.
//!
//! Both sides run in Node through small generated scripts, so the comparison sees exactly
//! what a JS caller sees. Values are encoded so `undefined`, `NaN`, `-0`, `Map`, `Set`,
//! `BigInt` and key order survive JSON.

use crate::walk::Canonical;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::bridge::{Kind, Manifest, RType, param_type};
use crate::ts::exports::LocalType;
use crate::ts::shape::{self, Shape};

pub const CASES: &str = "cases.jsonl";
pub(crate) const WORK_DIR: &str = ".rust-anything";
const COVERAGE_DIR: &str = "coverage";

/// Strings that break naive ports: empty, case, whitespace, non-ASCII, astral, combining.
const STRINGS: &[&str] = &[
    "",
    "a",
    "ab",
    "abc",
    "hello",
    "hello world",
    "Hello World",
    "HELLO",
    "  padded  ",
    "a.b.c",
    "ns:key.path",
    "a:b:c",
    ":",
    ".leading",
    "trailing.",
    "a..b",
    "src/a/b.ts",
    "./x/**/*.json",
    "key_name-1",
    "{name} has {count} items",
    "line1\nline2",
    "tab\tseparated",
    "ünïcödé",
    "straße",
    "İstanbul",
    "ΣΊΣΥΦΟΣ",
    "日本語のテキスト",
    "emoji 👍🏽 mix",
    "😀",
    "a😀b😀c",
    "e\u{301}",
    "\u{FEFF}bom",
];
const JSON_VALUES: &str =
    r#"[null, true, 0, "x", [], {}, {"a": "b"}, {"a": {"b": "c"}}, ["a", 1]]"#;

#[derive(Debug, Serialize)]
pub struct RecordOutcome {
    pub path: PathBuf,
    pub cases: usize,
    pub errors: usize,
    /// Cases from the semantic traps (added to the generated ones for every function).
    pub traps: usize,
    pub per_function: BTreeMap<String, usize>,
    /// Functions in files that use regexes: their string parameters also got the regex traps.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub regex_functions: Vec<String>,
    /// Inputs left out because the original ran past the per-case time limit on them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub timed_out: Vec<Value>,
    /// Lines of the original the cases ran (TS/JS, from V8's coverage).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<Value>,
    /// The clock and seed the original ran with (`--pin-clock`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<crate::pin::Pin>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DiffOutcome {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    /// Exports the Rust module does not provide.
    pub missing: Vec<String>,
    /// The first mismatches: `{ fn, args, expected, actual }`.
    pub mismatches: Vec<Value>,
    /// Mismatches `known-diffs.jsonl` explains, counted in `passed`.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub accepted: usize,
    /// `known-diffs.jsonl` entries no mismatch matched any more.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stale_known: Vec<Value>,
    /// Matches across the last changes to `port.rs`, and whether they stopped going up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trend: Option<crate::recording::Trend>,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

pub struct RecordOptions<'a> {
    pub crate_dir: &'a Path,
    /// Generated cases per function.
    pub count: usize,
    /// Extra cases, one `{"fn": name, "args": [...]}` JSON object per line, with `"kwargs": {...}`
    /// for Python and Ruby arguments passed by keyword.
    pub inputs: Option<&'a Path>,
    /// Import the original from this JS module instead of the TS source files.
    pub module: Option<&'a Path>,
    /// Resolve npm packages the original imports from this folder's `node_modules`.
    pub resolve_from: Option<&'a Path>,
    pub node_args: &'a [String],
}

/// The dataclasses a Python file can take: its own and its package's, so a type imported from
/// a sibling module (`from .models import Limits`) is found too.
fn python_types(root: &Path, file: &str) -> Vec<LocalType> {
    let dir = root.join(file).parent().map(Path::to_path_buf);
    let mut files = vec![root.join(file)];
    if let Some(dir) = dir
        && let Ok(entries) = std::fs::read_dir(dir)
    {
        let mut siblings: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "py") && !files.contains(p))
            .collect();
        siblings.sort();
        files.extend(siblings);
    }
    files
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .flat_map(|text| crate::lang::python::dataclass_types(&text))
        .collect()
}

/// Generates the cases a recording runs, shared by the host and on-device drivers.
fn generate_cases(
    opts: &RecordOptions,
    manifest: &Manifest,
) -> Result<(Vec<Value>, usize, Vec<String>)> {
    generate_cases_salted(opts, manifest, 0)
}

/// [`generate_cases`] with the generator's seeds mixed with `salt`. A non-zero salt gives fresh
/// inputs (`fuzz`): no traps and no `--inputs`, which the recording already has.
pub(crate) fn generate_cases_salted(
    opts: &RecordOptions,
    manifest: &Manifest,
    salt: u64,
) -> Result<(Vec<Value>, usize, Vec<String>)> {
    let mut cases: Vec<Value> = Vec::new();
    let module_for = |file: &str| -> Result<String> {
        let path = match opts.module {
            Some(m) => m
                .canonical()
                .with_context(|| format!("resolving {}", m.display()))?,
            None => manifest.root.join(file),
        };
        Ok(path.to_string_lossy().into_owned())
    };
    let mut scopes: BTreeMap<String, Vec<LocalType>> = BTreeMap::new();
    let mut regex_files: BTreeMap<String, bool> = BTreeMap::new();
    let mut regex_functions = Vec::new();
    let mut traps = 0;
    for f in &manifest.exports {
        let module = module_for(&f.file)?;
        if !scopes.contains_key(&f.file) {
            let text = std::fs::read_to_string(manifest.root.join(&f.file)).unwrap_or_default();
            regex_files.insert(
                f.file.clone(),
                crate::risks::uses_regex(manifest.language, &text),
            );
            let types = if manifest.language == crate::lang::Lang::Python {
                python_types(&manifest.root, &f.file)
            } else {
                crate::bridge::types_in_scope(&manifest.root, &f.file, &text)
            };
            scopes.insert(f.file.clone(), types);
        }
        let regex = regex_files[&f.file];
        if regex && f.params.iter().any(|p| param_type(p) == RType::Str) {
            regex_functions.push(f.name.clone());
        }
        let types = &scopes[&f.file];
        let mut all = generate_salted(f, opts.count, types, salt);
        let mut seen: std::collections::HashSet<String> = all
            .iter()
            .map(|a| serde_json::to_string(a).unwrap_or_default())
            .collect();
        let base: Vec<Value> = all
            .first()
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for args in trap_args(&f.params, &base, regex)
            .into_iter()
            .filter(|_| salt == 0)
        {
            let args = Value::Array(args);
            if seen.insert(serde_json::to_string(&args).unwrap_or_default()) {
                all.push(args);
                traps += 1;
            }
        }
        for (i, mut args) in all.into_iter().enumerate() {
            // Python callers pass ints where floats are expected, and int-ness can leak into
            // results: every fourth case passes integral `number` arguments as ints.
            if manifest.language == crate::lang::Lang::Python && i % 4 == 3 {
                intify(&f.params, &mut args);
            }
            let mut case = json!({ "fn": f.name, "module": module, "args": args });
            split_keyword_only(&f.params, &mut case);
            cases.push(case);
        }
    }
    for class in &manifest.classes {
        let module = module_for(&class.file)?;
        let text = std::fs::read_to_string(manifest.root.join(&class.file)).unwrap_or_default();
        let types = crate::bridge::types_in_scope(&manifest.root, &class.file, &text);
        for mut scenario in scenarios_salted(class, opts.count.max(20), &types, salt) {
            scenario["module"] = Value::String(module.clone());
            cases.push(scenario);
        }
    }
    if let Some(path) = opts.inputs.filter(|_| salt == 0) {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        for (i, line) in text
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
        {
            let mut case: Value = parse_deep(line).with_context(|| {
                format!(
                    "{}:{} (values use the record codec: {{\"$undefined\": true}}, {{\"$num\": \"NaN\"}}, \
                     and {{\"$utf16\": [code units]}} for strings with lone surrogates)",
                    path.display(),
                    i + 1
                )
            })?;
            if let Some(class) = case["class"].as_str() {
                let Some(c) = manifest.classes.iter().find(|c| c.name == class) else {
                    bail!(
                        "{}:{}: `{class}` is not a ported class of this unit",
                        path.display(),
                        i + 1
                    );
                };
                case["module"] = Value::String(module_for(&c.file)?);
                cases.push(case);
                continue;
            }
            let name = case["fn"].as_str().unwrap_or_default().to_string();
            let Some(f) = manifest.exports.iter().find(|f| f.name == name) else {
                bail!(
                    "{}:{}: `{name}` is not an export of this unit",
                    path.display(),
                    i + 1
                );
            };
            case["module"] = Value::String(module_for(&f.file)?);
            cases.push(case);
        }
    }

    Ok((cases, traps, regex_functions))
}

pub fn record(opts: &RecordOptions) -> Result<RecordOutcome> {
    let crate_dir = std::path::absolute(opts.crate_dir)?;
    let opts = &RecordOptions {
        crate_dir: &crate_dir,
        ..*opts
    };
    let manifest = Manifest::load(opts.crate_dir)?;
    let (cases, traps, regex_functions) = generate_cases(opts, &manifest)?;

    let timed_out_path = opts.crate_dir.join(WORK_DIR).join(TIMED_OUT);
    let _ = std::fs::remove_file(&timed_out_path);
    let coverage_dir = opts.crate_dir.join(WORK_DIR).join(COVERAGE_DIR);
    let _ = std::fs::remove_dir_all(&coverage_dir);
    let out = record_with(opts, &manifest, &cases)?;
    let coverage = crate::recording::node_coverage(&coverage_dir, &manifest);
    crate::recording::save_fingerprint(opts.crate_dir, &manifest, coverage.as_ref())?;
    let mut outcome = summarize(opts, &out, traps, regex_functions, coverage)?;
    outcome.pinned = crate::pin::load(opts.crate_dir)?.filter(|_| crate::pin::supported(&manifest));
    Ok(outcome)
}

/// Runs `cases` through the original with the language's driver and writes `cases.jsonl`.
pub(crate) fn record_with(
    opts: &RecordOptions,
    manifest: &Manifest,
    cases: &[Value],
) -> Result<PathBuf> {
    Ok(if manifest.language == crate::lang::Lang::Python {
        crate::harness_py::record(opts.crate_dir, cases)?
    } else if manifest.kind == Kind::CAbi {
        crate::harness_c::record(opts.crate_dir, manifest, cases)?
    } else if manifest.language == crate::lang::Lang::Go {
        crate::harness_go::record(opts.crate_dir, manifest, cases)?
    } else if manifest.language == crate::lang::Lang::Ruby {
        crate::harness_ruby::record(opts.crate_dir, manifest, cases)?
    } else if manifest.language == crate::lang::Lang::Php {
        crate::harness_php::record(opts.crate_dir, manifest, cases)?
    } else if manifest.language == crate::lang::Lang::Csharp {
        crate::harness_csharp::record(opts.crate_dir, manifest, cases)?
    } else if manifest.language == crate::lang::Lang::Java {
        crate::harness_java::record(opts.crate_dir, manifest, cases)?
    } else if manifest.language == crate::lang::Lang::Kotlin {
        crate::harness_kotlin::record(opts.crate_dir, manifest, cases)?
    } else {
        record_node(opts, manifest, cases)?
    })
}

/// Records on a connected Android device: the cases are generated here, the original Kotlin
/// runs under ART, and the results come back as the same `cases.jsonl`.
pub fn record_android(opts: &RecordOptions, device: Option<&str>) -> Result<RecordOutcome> {
    let manifest = Manifest::load(opts.crate_dir)?;
    let (cases, traps, regex_functions) = generate_cases(opts, &manifest)?;
    let _ = std::fs::remove_file(opts.crate_dir.join(WORK_DIR).join(TIMED_OUT));
    let out = crate::android_device::record(opts.crate_dir, &manifest, &cases, device)?;
    crate::recording::save_fingerprint(opts.crate_dir, &manifest, None)?;
    summarize(opts, &out, traps, regex_functions, None)
}

fn summarize(
    opts: &RecordOptions,
    out: &Path,
    traps: usize,
    regex_functions: Vec<String>,
    coverage: Option<Value>,
) -> Result<RecordOutcome> {
    let text = std::fs::read_to_string(out)?;
    let mut per_function = BTreeMap::new();
    let mut errors = 0;
    let mut count = 0;
    for line in text.lines().filter(|l| !l.is_empty()) {
        let case: Value = parse_deep(line)?;
        count += 1;
        let class_error = case["results"]
            .as_array()
            .is_some_and(|r| r.iter().any(|x| x.get("error").is_some()));
        errors +=
            usize::from(case.get("error").is_some() || case.get("panic").is_some() || class_error);
        let name = case["fn"]
            .as_str()
            .or_else(|| case["class"].as_str())
            .unwrap_or_default();
        *per_function.entry(name.to_string()).or_default() += 1;
    }
    let timed_out_path = opts.crate_dir.join(WORK_DIR).join(TIMED_OUT);
    let timed_out: Vec<Value> = std::fs::read_to_string(&timed_out_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    Ok(RecordOutcome {
        path: out.to_path_buf(),
        cases: count,
        errors,
        traps,
        per_function,
        regex_functions,
        timed_out: timed_out
            .iter()
            .map(|c| serde_json::json!({ "fn": c["fn"].clone(), "args": c["args"].clone() }))
            .collect(),
        coverage,
        pinned: None,
    })
}

/// Runs the recorded inputs through Node: the original TS/JS, with the resolve hook.
fn record_node(opts: &RecordOptions, manifest: &Manifest, cases: &[Value]) -> Result<PathBuf> {
    let work = opts.crate_dir.join(WORK_DIR);
    std::fs::create_dir_all(&work)?;
    let script = work.join("record.mjs");
    std::fs::write(
        &script,
        format!("{CODEC_JS}\n{}\n{RECORD_JS}", crate::pin::PIN_JS),
    )?;
    let out = opts.crate_dir.join(CASES);
    // TS sources often import the way bundlers resolve (`./errors`, `./types`): a resolve hook
    // lets Node follow them, so the original runs as it is.
    std::fs::write(work.join("resolve.mjs"), RESOLVE_JS)?;
    std::fs::write(work.join("register.mjs"), REGISTER_JS)?;
    let register = work.canonical()?.join("register.mjs");
    let mut node_args = vec![format!("--import={}", file_url(&register))];
    node_args.extend(opts.node_args.iter().cloned());
    let aliases = alias_env(manifest);
    let resolve_from = opts
        .resolve_from
        .map(|dir| {
            dir.canonical()
                .with_context(|| format!("resolving {}", dir.display()))
        })
        .transpose()?;
    let record = |transform: bool| {
        record_cases(cases, &work, &out, "node", |inputs, part| {
            let mut cmd = Command::new("node");
            if transform {
                // Enums and parameter properties need Node to transform TS, not only strip types.
                cmd.args(transform_flag())
                    .arg("--no-warnings=ExperimentalWarning");
            }
            cmd.args(&node_args)
                .arg(&script)
                .arg(inputs)
                .arg(part)
                .env("NODE_V8_COVERAGE", work.join(COVERAGE_DIR))
                .env("RUST_ANYTHING_PATHS", &aliases)
                .env("RUST_ANYTHING_WORKSPACE", workspace_env(manifest));
            if let Some(dir) = &resolve_from {
                cmd.env("RUST_ANYTHING_RESOLVE_FROM", dir);
            }
            crate::pin::apply(&mut cmd, opts.crate_dir)?;
            crate::enable::original_only(&mut cmd, &Manifest::load(opts.crate_dir)?);
            Ok(cmd)
        })
    };
    if let Err(error) = record(false) {
        if !format!("{error:#}").contains("ERR_UNSUPPORTED_TYPESCRIPT_SYNTAX") {
            return Err(error);
        }
        record(true)?;
    }
    Ok(out)
}

fn transform_flag() -> Option<&'static str> {
    static SUPPORTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let supported = *SUPPORTED.get_or_init(|| {
        let mut cmd = Command::new("node");
        cmd.args(["--experimental-transform-types", "--no-warnings", "-e", "0"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        crate::process::status(&mut cmd, crate::process::QUICK).is_ok_and(|s| s.success())
    });
    supported.then_some("--experimental-transform-types")
}

/// Copies a freshly built library into place by renaming, so a process that has the old one
/// loaded keeps a valid file.
pub(crate) fn install(built: &Path, dest: &Path) -> Result<()> {
    let tmp = dest.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::copy(built, &tmp).with_context(|| format!("copying {}", built.display()))?;
    std::fs::rename(&tmp, dest).with_context(|| format!("installing {}", dest.display()))
}

/// The library `diff` loads, when it is older than the crate's sources: a forgotten `build`
/// would otherwise test the previous port.
fn stale_build(crate_dir: &Path, manifest: &Manifest) -> Option<PathBuf> {
    let built = match manifest.kind {
        Kind::Napi => crate_dir.join(format!("{}.node", manifest.lib_name())),
        Kind::Pyo3 => crate::harness_py::native_file(crate_dir, manifest),
        Kind::Ffi if manifest.language == crate::lang::Lang::Php => {
            crate::harness_php::built_file(crate_dir, manifest)?
        }
        Kind::Ffi if manifest.language == crate::lang::Lang::Csharp => {
            crate::harness_csharp::built_file(crate_dir, manifest)?
        }
        _ => return None,
    };
    sources_newer_than(crate_dir, &built).then_some(built)
}

/// True when `Cargo.toml` or any `.rs` file under `src/` is newer than `built`.
pub(crate) fn sources_newer_than(crate_dir: &Path, built: &Path) -> bool {
    let Ok(built_at) = std::fs::metadata(built).and_then(|m| m.modified()) else {
        return false;
    };
    let mut sources = vec![crate_dir.join("Cargo.toml")];
    sources.extend(
        walkdir_rs(&crate_dir.join("src"))
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "rs")),
    );
    sources
        .iter()
        .filter_map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
        .any(|changed| changed > built_at)
}

fn walkdir_rs(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walkdir_rs(&path));
        } else {
            out.push(path);
        }
    }
    out
}

/// Builds a JNI crate for Android: one library per ABI, installed into `jniLibs/`.
pub fn build_android(
    crate_dir: &Path,
    opts: &crate::android::Options,
) -> Result<crate::android::Report> {
    let manifest = Manifest::load(crate_dir)?;
    crate::android::build(crate_dir, &manifest, opts)
}

pub fn build(crate_dir: &Path) -> Result<PathBuf> {
    let manifest = Manifest::load(crate_dir)?;
    match manifest.kind {
        Kind::Pyo3 => crate::harness_py::build(crate_dir, &manifest),
        Kind::Ffi if manifest.language == crate::lang::Lang::Ruby => {
            crate::harness_ruby::build(crate_dir, &manifest)
        }
        Kind::Ffi if manifest.language == crate::lang::Lang::Php => {
            crate::harness_php::build(crate_dir, &manifest)
        }
        Kind::Ffi if manifest.language == crate::lang::Lang::Csharp => {
            crate::harness_csharp::build(crate_dir, &manifest)
        }
        Kind::CAbi | Kind::Ffi => crate::harness_c::build(crate_dir, &manifest),
        Kind::Napi => {
            let target = crate::cache::target_for(crate_dir)?;
            run(
                Command::new("cargo")
                    .args(["build", "--release", "--manifest-path"])
                    .arg(crate_dir.join("Cargo.toml"))
                    .env("CARGO_TARGET_DIR", &target.dir),
                "cargo build",
            )?;
            let lib = manifest.lib_name();
            let release = target.dir.join("release");
            let built = [
                format!("lib{lib}.so"),
                format!("lib{lib}.dylib"),
                format!("{lib}.dll"),
            ]
            .iter()
            .map(|name| release.join(name))
            .find(|p| p.exists())
            .with_context(|| format!("no native library for `{lib}` in {}", release.display()))?;
            let artifact = crate_dir.join(format!("{lib}.node"));
            install(&built, &artifact)?;
            Ok(artifact)
        }
        Kind::Wasm => {
            let target = crate::cache::target_for(crate_dir)?.dir;
            run(
                Command::new("cargo")
                    .args([
                        "build",
                        "--release",
                        "--target",
                        "wasm32-unknown-unknown",
                        "--manifest-path",
                    ])
                    .arg(crate_dir.join("Cargo.toml"))
                    .env("CARGO_TARGET_DIR", &target),
                "cargo build --target wasm32-unknown-unknown (needs `rustup target add wasm32-unknown-unknown`)",
            )?;
            // wasm-bindgen's CLI must match the crate's wasm-bindgen version exactly.
            let lock = std::fs::read_to_string(crate_dir.join("Cargo.lock")).unwrap_or_default();
            let wanted = locked_version(&lock, "wasm-bindgen")
                .context("wasm-bindgen missing from Cargo.lock")?;
            let installed = Command::new("wasm-bindgen")
                .arg("--version")
                .output()
                .ok()
                .map(|o| {
                    String::from_utf8_lossy(&o.stdout)
                        .trim()
                        .rsplit(' ')
                        .next()
                        .unwrap_or_default()
                        .to_string()
                });
            if installed.as_deref() != Some(wanted.as_str()) {
                bail!(
                    "the crate uses wasm-bindgen {wanted}; the CLI must match:\n  cargo install wasm-bindgen-cli --version {wanted} --locked{}",
                    installed
                        .map(|v| format!(" (found {v})"))
                        .unwrap_or_default()
                );
            }
            let lib = manifest.lib_name();
            let wasm = target
                .join("wasm32-unknown-unknown/release")
                .join(format!("{lib}.wasm"));
            run(
                Command::new("wasm-bindgen")
                    .args(["--target", "nodejs", "--out-dir"])
                    .arg(crate_dir.join("pkg"))
                    .arg(&wasm),
                "wasm-bindgen",
            )?;
            std::fs::write(
                crate_dir.join("pkg").join("package.json"),
                "{\n  \"type\": \"commonjs\"\n}\n",
            )?;
            Ok(crate_dir.join("pkg").join(format!("{lib}.js")))
        }
    }
}

/// Per-function timing of the original against the Rust module.
/// What `bench` concludes: the geometric-mean speedup and what to do.
#[derive(Debug, Serialize)]
pub struct Verdict {
    pub speedup: f64,
    pub faster: Vec<String>,
    pub slower: Vec<String>,
    pub advice: String,
}

/// Faster means at least 1.1x, slower below 0.9x; between is a tie (timing noise).
pub fn verdict(rows: &[BenchRow], lang: crate::lang::Lang) -> Verdict {
    let faster: Vec<String> = rows
        .iter()
        .filter(|r| r.speedup >= 1.1)
        .map(|r| r.name.clone())
        .collect();
    let slower: Vec<String> = rows
        .iter()
        .filter(|r| r.speedup < 0.9)
        .map(|r| r.name.clone())
        .collect();
    let speedup = if rows.is_empty() {
        1.0
    } else {
        (rows.iter().map(|r| r.speedup.max(1e-9).ln()).sum::<f64>() / rows.len() as f64).exp()
    };
    let advice = if rows.is_empty() {
        "Nothing to compare: record cases that do not throw first.".to_string()
    } else if slower.is_empty() && faster.len() == rows.len() {
        "Ship it: every function is faster in Rust, call included.".to_string()
    } else if slower.is_empty() {
        "Ship it: no function is slower in Rust; some only tie.".to_string()
    } else if lang == crate::lang::Lang::C {
        // C links the port directly: there is no crossing to blame.
        format!(
            "No call overhead in C: these are slower as Rust code. Profile and optimize them, or keep them in C: {}.",
            slower.join(", ")
        )
    } else if speedup >= 1.1 {
        format!(
            "Ship the faster functions. Keep the slower ones in the original (`bridge --keep`) or move their callers to Rust too: {}.",
            slower.join(", ")
        )
    } else {
        "Do not ship this unit for speed: calling across costs more than the work these functions do. \
         Port a bigger unit (these functions together with their callers), so one call does more work."
            .to_string()
    };
    Verdict {
        speedup,
        faster,
        slower,
        advice,
    }
}

/// Rows from two `{ name: { cases, ns } }` timing maps: the original's and the port's.
pub fn bench_rows(
    m: &Manifest,
    original: &serde_json::Map<String, Value>,
    rust: &serde_json::Map<String, Value>,
) -> Vec<BenchRow> {
    m.exports
        .iter()
        .filter_map(|f| {
            let (old, new) = (original.get(&f.name)?, rust.get(&f.name)?);
            let (o, n) = (old["ns"].as_f64()?, new["ns"].as_f64()?);
            Some(BenchRow {
                name: f.name.clone(),
                cases: usize::try_from(old["cases"].as_u64().unwrap_or(0)).unwrap_or(0),
                original_ns: o,
                rust_ns: n,
                speedup: o / n,
            })
        })
        .collect()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchRow {
    #[serde(rename = "fn")]
    pub name: String,
    pub cases: usize,
    /// Nanoseconds per call.
    pub original_ns: f64,
    pub rust_ns: f64,
    /// `original_ns / rust_ns`: above 1 means Rust is faster, bridge included.
    pub speedup: f64,
}

pub struct BenchOptions<'a> {
    pub crate_dir: &'a Path,
    /// Minimum measured time per function and side, in milliseconds.
    pub min_ms: u64,
    pub resolve_from: Option<&'a Path>,
    pub node_args: &'a [String],
}

/// Times the recorded cases against the original and the built Rust module, bridge included.
pub fn bench(opts: &BenchOptions) -> Result<Vec<BenchRow>> {
    let crate_dir = std::path::absolute(opts.crate_dir)?;
    let opts = &BenchOptions {
        crate_dir: &crate_dir,
        ..*opts
    };
    let manifest = Manifest::load(opts.crate_dir)?;
    if manifest.language == crate::lang::Lang::Python {
        return crate::harness_py::bench(opts.crate_dir, &manifest, opts.min_ms);
    }
    if manifest.kind == Kind::CAbi {
        return crate::harness_c::bench(opts.crate_dir, &manifest, opts.min_ms);
    }
    if manifest.language == crate::lang::Lang::Go {
        return crate::harness_go::bench(opts.crate_dir, &manifest, opts.min_ms);
    }
    if manifest.language == crate::lang::Lang::Ruby {
        return crate::harness_ruby::bench(opts.crate_dir, &manifest, opts.min_ms);
    }
    if manifest.language == crate::lang::Lang::Php {
        return crate::harness_php::bench(opts.crate_dir, &manifest, opts.min_ms);
    }
    if manifest.language == crate::lang::Lang::Csharp {
        return crate::harness_csharp::bench(opts.crate_dir, &manifest, opts.min_ms);
    }
    if manifest.language == crate::lang::Lang::Java {
        return crate::harness_java::bench(opts.crate_dir, &manifest, opts.min_ms);
    }
    if manifest.language == crate::lang::Lang::Kotlin {
        return crate::harness_kotlin::bench(opts.crate_dir, &manifest, opts.min_ms);
    }
    let cases = opts.crate_dir.join(CASES);
    if !cases.exists() {
        bail!(
            "no {CASES} in {} (run `rust-anything record` first)",
            opts.crate_dir.display()
        );
    }
    let modules: BTreeMap<String, String> = manifest
        .exports
        .iter()
        .map(|f| {
            (
                f.name.clone(),
                manifest.root.join(&f.file).to_string_lossy().into_owned(),
            )
        })
        .collect();
    let work = opts.crate_dir.join(WORK_DIR);
    std::fs::create_dir_all(&work)?;
    std::fs::write(work.join("resolve.mjs"), RESOLVE_JS)?;
    std::fs::write(work.join("register.mjs"), REGISTER_JS)?;
    let class_modules: BTreeMap<String, String> = manifest
        .classes
        .iter()
        .map(|c| {
            (
                c.name.clone(),
                manifest.root.join(&c.file).to_string_lossy().into_owned(),
            )
        })
        .collect();
    let async_functions: Vec<&str> = manifest
        .exports
        .iter()
        .filter(|f| f.is_async)
        .map(|f| f.name.as_str())
        .collect();
    let modules_path = work.join("bench-modules.json");
    std::fs::write(
        &modules_path,
        serde_json::to_string(&json!({
            "functions": modules,
            "classes": class_modules,
            "asyncFunctions": async_functions,
        }))?,
    )?;
    let script = work.join("bench.mjs");
    std::fs::write(&script, format!("{CODEC_JS}\n{BENCH_JS}"))?;
    let out = work.join("bench.json");
    let native = opts.crate_dir.join("index.cjs").canonical()?;
    let register = work.canonical()?.join("register.mjs");
    let mut cmd = Command::new("node");
    crate::enable::original_only(&mut cmd, &manifest);
    cmd.args(transform_flag())
        .arg("--no-warnings")
        .arg(format!("--import={}", file_url(&register)))
        .args(opts.node_args)
        .arg(&script)
        .arg(&modules_path)
        .arg(&native)
        .arg(&cases)
        .arg(&out)
        .arg(opts.min_ms.to_string())
        .env("RUST_ANYTHING_PATHS", alias_env(&manifest))
        .env("RUST_ANYTHING_WORKSPACE", workspace_env(&manifest));
    if let Some(dir) = opts.resolve_from {
        cmd.env("RUST_ANYTHING_RESOLVE_FROM", dir.canonical()?);
    }
    run(&mut cmd, "node")?;
    Ok(serde_json::from_str(&std::fs::read_to_string(&out)?)?)
}

pub struct DiffOptions<'a> {
    pub crate_dir: &'a Path,
    /// The module to test; defaults to the crate's `index.cjs`.
    pub module: Option<&'a Path>,
    /// Compare objects regardless of key order.
    pub ignore_key_order: bool,
    pub node_args: &'a [String],
    /// Floats match within this tolerance; the one saved by `record` when `None`.
    pub float_tolerance: Option<f64>,
}

/// The first mismatches a diff reports.
const SHOWN_MISMATCHES: usize = 20;

pub fn diff(opts: &DiffOptions) -> Result<DiffOutcome> {
    diff_after(opts, diff_exact(opts)?)
}

/// Diffs on a connected Android device, using the port built for the device's ABI.
pub fn diff_android(
    opts: &DiffOptions,
    device: Option<&str>,
    jni_libs: Option<&Path>,
) -> Result<DiffOutcome> {
    let manifest = Manifest::load(opts.crate_dir)?;
    diff_after(
        opts,
        crate::android_device::diff(opts.crate_dir, &manifest, device, jni_libs)?,
    )
}

fn diff_after(opts: &DiffOptions, exact: DiffOutcome) -> Result<DiffOutcome> {
    let mut outcome = settle(opts, exact)?;
    outcome.mismatches.truncate(SHOWN_MISMATCHES);
    anyhow::ensure!(
        outcome.total > 0,
        "no recorded cases to compare; run `rust-anything record` first"
    );
    if opts.module.is_none() {
        outcome.trend = crate::recording::track(opts.crate_dir, outcome.passed, outcome.total);
    }
    Ok(outcome)
}

/// Applies the float tolerance and `known-diffs.jsonl` to a driver's outcome, keeping every
/// mismatch.
pub(crate) fn settle(opts: &DiffOptions, exact: DiffOutcome) -> Result<DiffOutcome> {
    let mut outcome = exact;
    let manifest = Manifest::load(opts.crate_dir).ok();
    let saved = manifest.as_ref().and_then(|m| m.float_tolerance);
    if let Some(t) = opts.float_tolerance.or(saved) {
        let preserve_key_order = opts.module.is_some()
            || manifest.as_ref().is_none_or(|m| {
                matches!(
                    m.language,
                    crate::lang::Lang::Ts | crate::lang::Lang::Python
                )
            });
        outcome = crate::tolerance::apply(
            outcome,
            crate::tolerance::check(t)?,
            opts.ignore_key_order || !preserve_key_order,
        );
    }
    Ok(crate::recording::apply_known(
        outcome,
        &crate::recording::known_diffs(opts.crate_dir)?,
    ))
}

pub(crate) fn diff_exact(opts: &DiffOptions) -> Result<DiffOutcome> {
    let crate_dir = std::path::absolute(opts.crate_dir)?;
    let opts = &DiffOptions {
        crate_dir: &crate_dir,
        ..*opts
    };
    if opts.module.is_none()
        && let Ok(manifest) = Manifest::load(opts.crate_dir)
        && let Some(built) = stale_build(opts.crate_dir, &manifest)
    {
        bail!(
            "{} is older than the crate's sources, so diff would test the previous port; run \
             `rust-anything build {}` first",
            built.display(),
            opts.crate_dir.display()
        );
    }
    if let Ok(manifest) = Manifest::load(opts.crate_dir)
        && manifest.language == crate::lang::Lang::Python
        && opts.module.is_none()
    {
        return crate::harness_py::diff(opts.crate_dir, &manifest, opts.ignore_key_order);
    }
    if let Ok(manifest) = Manifest::load(opts.crate_dir)
        && manifest.kind == Kind::CAbi
        && opts.module.is_none()
    {
        return crate::harness_c::diff(opts.crate_dir, &manifest);
    }
    if let Ok(manifest) = Manifest::load(opts.crate_dir)
        && manifest.language == crate::lang::Lang::Go
        && opts.module.is_none()
    {
        return crate::harness_go::diff(opts.crate_dir, &manifest);
    }
    if let Ok(manifest) = Manifest::load(opts.crate_dir)
        && manifest.language == crate::lang::Lang::Csharp
        && opts.module.is_none()
    {
        return crate::harness_csharp::diff(opts.crate_dir, &manifest);
    }
    if let Ok(manifest) = Manifest::load(opts.crate_dir)
        && manifest.language == crate::lang::Lang::Ruby
        && opts.module.is_none()
    {
        return crate::harness_ruby::diff(opts.crate_dir, &manifest);
    }
    if let Ok(manifest) = Manifest::load(opts.crate_dir)
        && manifest.language == crate::lang::Lang::Php
        && opts.module.is_none()
    {
        return crate::harness_php::diff(opts.crate_dir, &manifest);
    }
    if let Ok(manifest) = Manifest::load(opts.crate_dir)
        && manifest.language == crate::lang::Lang::Java
        && opts.module.is_none()
    {
        return crate::harness_java::diff(opts.crate_dir, &manifest);
    }
    if let Ok(manifest) = Manifest::load(opts.crate_dir)
        && manifest.language == crate::lang::Lang::Kotlin
        && opts.module.is_none()
    {
        return crate::harness_kotlin::diff(opts.crate_dir, &manifest);
    }
    let cases = opts.crate_dir.join(CASES);
    if !cases.exists() {
        bail!(
            "no {CASES} in {} (run `rust-anything record` first)",
            opts.crate_dir.display()
        );
    }
    let module = match opts.module {
        Some(m) => m.to_path_buf(),
        None => opts.crate_dir.join("index.cjs"),
    };
    let module = module
        .canonical()
        .with_context(|| format!("resolving {}", module.display()))?;
    let work = opts.crate_dir.join(WORK_DIR);
    std::fs::create_dir_all(&work)?;
    let script = work.join("diff.mjs");
    std::fs::write(
        &script,
        format!("{CODEC_JS}\n{}\n{DIFF_JS}", crate::pin::PIN_JS),
    )?;
    let out = work.join("diff.json");
    let flag = if opts.ignore_key_order {
        "sort"
    } else {
        "keep"
    };
    let mut cmd = Command::new("node");
    cmd.args(opts.node_args)
        .arg(&script)
        .args([&module, &cases, &out])
        .arg(flag);
    crate::pin::apply(&mut cmd, opts.crate_dir)?;
    crate::enable::original_only(&mut cmd, &Manifest::load(opts.crate_dir)?);
    run(&mut cmd, "node")?;
    let text = std::fs::read_to_string(&out)?;
    Ok(serde_json::from_str(&text)?)
}

/// The version of `package` in a Cargo.lock.
fn locked_version(lock: &str, package: &str) -> Option<String> {
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() == format!("name = \"{package}\"") {
            let version = lines
                .next()?
                .trim()
                .strip_prefix("version = \"")?
                .strip_suffix('"')?;
            return Some(version.to_string());
        }
    }
    None
}

/// Deterministic inputs for one function: edge cases first, then seeded random mixes.
pub fn generate(
    f: &crate::ts::exports::ExportedFn,
    count: usize,
    types: &[LocalType],
) -> Vec<Value> {
    generate_salted(f, count, types, 0)
}

/// [`generate`] from another seed: `salt` 0 gives the recorded inputs, any other value fresh ones.
pub fn generate_salted(
    f: &crate::ts::exports::ExportedFn,
    count: usize,
    types: &[LocalType],
    salt: u64,
) -> Vec<Value> {
    let mut rng = Rng::new(
        f.name.bytes().fold(0x9E37_79B9_7F4A_7C15, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x100_0000_01B3)
        }) ^ salt,
    );
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let mut attempts = 0;
    while out.len() < count && attempts < count * 20 {
        attempts += 1;
        let args = args_for(&f.params, types, &mut rng);
        let key = serde_json::to_string(&args).unwrap_or_default();
        if seen.insert(key) {
            out.push(Value::Array(args));
        }
    }
    out
}

/// The semantic traps for a value of type `t`: what most often behaves differently once ported
/// (string length and case, signed division and overflow, float printing, map and sort order).
fn trap_values(t: &RType) -> Vec<Value> {
    match t {
        RType::Str => crate::app::STRING_TRAPS
            .iter()
            .chain(
                [
                    "ΣΊΣΥΦΟΣ",
                    "a😀b",
                    "\u{FEFF}bom",
                    "a²b x½ y①",
                    "aⅠb Ⅻ",
                    "a𐐀b 𝐀bc",
                    "कि ٣٤",
                ]
                .iter(),
            )
            .map(|s| json!(s))
            .collect(),
        RType::Int => [
            -7,
            -1,
            0,
            2_147_483_648,
            -2_147_483_649,
            9_007_199_254_740_993,
            i64::MAX,
            i64::MIN,
        ]
        .iter()
        .map(|n| json!(n))
        .collect(),
        RType::F64 => {
            let mut out: Vec<Value> = [
                -7.5,
                0.1,
                0.30000000000000004,
                1e21,
                1e-7,
                5e-324,
                1.797_693_134_862_315_7e308,
                9_007_199_254_740_992.0,
            ]
            .iter()
            .map(|n| json!(n))
            .collect();
            out.push(json!({ "$num": "-0" }));
            out
        }
        RType::Vec(inner) => match inner.as_ref() {
            RType::Str => vec![json!([]), json!(["b", "a", "B", "é", "10", "9", "a"])],
            RType::Int => vec![json!([]), json!([-7, 3, 0, i64::MAX, -1])],
            RType::F64 => vec![json!([]), json!([0.1, 0.2, -0.5, 1e21])],
            _ => vec![json!([])],
        },
        RType::Opt(inner) => {
            let mut out = vec![json!({ "$undefined": true })];
            out.extend(trap_values(inner));
            out
        }
        RType::Json => crate::app::JSON_TRAPS
            .iter()
            .filter_map(|t| serde_json::from_str(t).ok())
            .chain([json!([3, 1, 2]), json!({})])
            .collect(),
        RType::Bool | RType::Unit => Vec::new(),
    }
}

/// Every trap value in every parameter, one at a time, the other arguments as in `base`.
/// Structured parameters (objects with a known shape) are left as generated.
pub fn trap_args(
    params: &[crate::ts::exports::Param],
    base: &[Value],
    regex: bool,
) -> Vec<Vec<Value>> {
    let mut out = Vec::new();
    let params: Vec<&crate::ts::exports::Param> = params
        .iter()
        .filter(|p| !p.ts_type.starts_with("len:"))
        .collect();
    if base.len() < params.iter().filter(|p| !p.rest).count() {
        return out;
    }
    for (i, p) in params.iter().enumerate() {
        if p.rest || i >= base.len() {
            continue;
        }
        if let Some(values) = c_pointer_traps(&p.ts_type) {
            for v in values {
                if base[i] != v {
                    let mut args = base.to_vec();
                    args[i] = v;
                    out.push(args);
                }
            }
            continue;
        }
        let t = param_type(p);
        let structured = t.contains_json() && !matches!(t, RType::Json);
        let any = matches!(p.ts_type.trim(), "any" | "unknown" | "object" | "Object");
        if structured || (matches!(t, RType::Json) && !any) {
            continue;
        }
        let mut values = trap_values(&t);
        if regex && t == RType::Str {
            values.extend(crate::risks::REGEX_TRAPS.iter().map(|s| json!(s)));
        }
        for v in values {
            if base[i] != v {
                let mut args = base.to_vec();
                args[i] = v;
                out.push(args);
            }
        }
    }
    out
}

/// Edge cases for C pointer parameters: buffers as hex text (`bytes`, `buffer`) or a size of
/// zeroed bytes (`buffer`), the starting value behind an `out` pointer, a `char **` that starts
/// NULL, unwritten or at offset 0 (`outptr`), and structs of zeros, ones, minus ones, minimums
/// and maximums, plus each field zero with the others one (a zero denominator).
fn c_pointer_traps(ts_type: &str) -> Option<Vec<Value>> {
    Some(match ts_type {
        "bytes" => vec![
            json!(""),
            json!("00"),
            json!("ff"),
            json!("616263"),
            json!((0..=255u8).map(|b| format!("{b:02x}")).collect::<String>()),
            json!("ff".repeat(64)),
        ],
        "buffer" => vec![
            json!(0),
            json!(1),
            json!(3),
            json!(64),
            json!("ff".repeat(16)),
        ],
        "out" => vec![json!(0), json!(-1), json!(2_147_483_647)],
        t if t.starts_with("outptr:") => vec![json!(true), Value::Null, json!(0)],
        t => {
            let s = crate::codegen_c::c_struct(t)?;
            let field = |pick: &dyn Fn(&[Value]) -> Value, zero: Option<usize>| {
                let fields = s.fields.iter().enumerate().map(|(i, (ty, name))| {
                    let edges = c_field_edges(ty);
                    let v = if zero == Some(i) {
                        edges[0].clone()
                    } else {
                        pick(&edges)
                    };
                    (name.clone(), v)
                });
                Value::Object(fields.collect())
            };
            let mut out: Vec<Value> = (0..5)
                .map(|k| field(&|e: &[Value]| e[k.min(e.len() - 1)].clone(), None))
                .collect();
            out.extend((0..s.fields.len()).map(|i| field(&|e: &[Value]| e[1].clone(), Some(i))));
            out.dedup();
            out
        }
    })
}

/// Edge values for a C scalar struct field: zero, one, then minus one, the minimum and the
/// maximum where the type has them (non-finite values for floating point).
fn c_field_edges(ty: &str) -> Vec<Value> {
    let ty = ty.trim();
    if ty.contains("float") || ty.contains("double") {
        return vec![
            json!(0.0),
            json!(1.0),
            json!(-1.0),
            json!({ "$num": "NaN" }),
            json!({ "$num": "Infinity" }),
            json!({ "$num": "-Infinity" }),
            json!(0.5),
            json!(1e300),
        ];
    }
    if ty == "bool" || ty == "_Bool" {
        return vec![json!(false), json!(true)];
    }
    let unsigned = ty.starts_with('u') || ty.starts_with("unsigned") || ty == "size_t";
    let bits = if ty.contains('8') || ty.ends_with("char") {
        8
    } else if ty.contains("16") || ty.contains("short") {
        16
    } else if ty.contains("64") || ty.contains("long") || ty.contains("size") || ty.contains("ptr")
    {
        64
    } else {
        32
    };
    if unsigned {
        let max = if bits == 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        vec![json!(0), json!(1), json!(max), json!(max - 1), json!(2)]
    } else {
        let max = i64::MAX >> (64 - bits);
        vec![json!(0), json!(1), json!(-1), json!(-max - 1), json!(max)]
    }
}

/// A random value for a C pointer parameter (see `c_pointer_traps`).
fn c_pointer_value(ts_type: &str, rng: &mut Rng) -> Value {
    const SIZES: [usize; 12] = [0, 1, 2, 3, 4, 5, 8, 16, 17, 64, 100, 1024];
    match ts_type {
        "bytes" => {
            let n = SIZES[rng.below(SIZES.len())];
            let text = rng.below(2) == 0;
            json!(
                (0..n)
                    .map(|_| {
                        let b = if text {
                            0x20 + rng.below(0x5f)
                        } else {
                            rng.below(256)
                        };
                        format!("{b:02x}")
                    })
                    .collect::<String>()
            )
        }
        "buffer" => json!(SIZES[rng.below(SIZES.len())]),
        t if t.starts_with("outptr:") => [json!(true), Value::Null, json!(0)][rng.below(3)].clone(),
        t if let Some(s) = crate::codegen_c::c_struct(t) => Value::Object(
            s.fields
                .iter()
                .map(|(ty, name)| {
                    let edges = c_field_edges(ty);
                    let small: [i32; 14] =
                        [0, 1, -1, 2, 3, 7, 12, 25, 30, 1000, 1001, 90000, -5, -48000];
                    let v = match rng.below(10) {
                        0..=2 => edges[rng.below(edges.len())].clone(),
                        _ if edges.len() == 2 => edges[rng.below(2)].clone(),
                        _ if ty.contains("float") || ty.contains("double") => {
                            json!(f64::from(small[rng.below(small.len())]) / 4.0)
                        }
                        _ if edges[2] != json!(-1) => {
                            json!(small[rng.below(small.len())].unsigned_abs())
                        }
                        _ => json!(small[rng.below(small.len())]),
                    };
                    (name.clone(), v)
                })
                .collect(),
        ),
        _ => json!([0, 1, -1, 7, 1000, 65536][rng.below(6)]),
    }
}

/// One argument list for `params`: values that fit, edge cases, and sometimes omitted optionals.
fn args_for(
    params: &[crate::ts::exports::Param],
    types: &[LocalType],
    rng: &mut Rng,
) -> Vec<Value> {
    let mut args: Vec<Value> = Vec::new();
    for p in params {
        match p.ts_type.as_str() {
            t if t.starts_with("len:") => continue,
            t if matches!(t, "bytes" | "buffer" | "out")
                || t.starts_with("outptr:")
                || t.starts_with("struct:") =>
            {
                args.push(c_pointer_value(&p.ts_type, rng));
                continue;
            }
            _ => {}
        }
        let t = param_type(p);
        // Structured parameters: values that fit the TS type, and now and then a value
        // that does not, so error paths get recorded too.
        let structured = |rng: &mut Rng| {
            if rng.below(100) < 15 {
                value(&RType::Json, rng)
            } else {
                shaped(&shape::parse(&p.ts_type), types, rng, 0)
            }
        };
        if p.rest {
            let inner = match t {
                RType::Vec(inner) => *inner,
                other => other,
            };
            for _ in 0..rng.below(3) {
                args.push(value(&inner, rng));
            }
        } else if t.contains_json() {
            let v = structured(rng);
            args.push(if p.optional && rng.below(5) == 0 {
                json!({ "$undefined": true })
            } else {
                v
            });
        } else {
            args.push(value(&t, rng));
        }
    }
    // Search-like functions rarely hit on random text: often derive one string from another.
    let strings: Vec<usize> = (0..args.len()).filter(|&i| args[i].is_string()).collect();
    if strings.len() >= 2 && rng.below(10) < 5 {
        let from = strings[rng.below(strings.len())];
        let to = strings[rng.below(strings.len())];
        if from != to {
            let source: Vec<char> = args[from].as_str().unwrap_or_default().chars().collect();
            if !source.is_empty() {
                let start = rng.below(source.len());
                let end = start + 1 + rng.below(source.len() - start);
                let mut piece: String = source[start..end].iter().collect();
                if rng.below(4) == 0 {
                    piece = piece.to_uppercase();
                }
                args[to] = Value::String(piece);
            }
        }
    }
    // Trailing optional arguments: sometimes omit them, the way JS callers do.
    while args.last().is_some_and(|a| a.get("$undefined").is_some()) && rng.below(2) == 0 {
        args.pop();
    }
    args
}

/// Class scenarios: construct, then call a random sequence of methods.
pub fn scenarios(
    class: &crate::ts::exports::ExportedClass,
    count: usize,
    types: &[LocalType],
) -> Vec<Value> {
    scenarios_salted(class, count, types, 0)
}

/// [`scenarios`] from another seed, as [`generate_salted`].
pub fn scenarios_salted(
    class: &crate::ts::exports::ExportedClass,
    count: usize,
    types: &[LocalType],
    salt: u64,
) -> Vec<Value> {
    let mut rng = Rng::new(
        class.name.bytes().fold(0x2545_F491_4F6C_DD1D, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x100_0000_01B3)
        }) ^ salt,
    );
    (0..count)
        .map(|_| {
            let ctor = args_for(&class.constructor, types, &mut rng);
            let steps = if class.methods.is_empty() {
                0
            } else {
                1 + rng.below(12)
            };
            // Stateful classes need calls that meet again (`set(k)` then `get(k)`): string
            // arguments often reuse a string seen earlier in the scenario.
            let mut seen: Vec<Value> = ctor.iter().filter(|v| v.is_string()).cloned().collect();
            let calls: Vec<Value> = (0..steps)
                .map(|_| {
                    let m = &class.methods[rng.below(class.methods.len())];
                    let mut args = args_for(&m.params, types, &mut rng);
                    for arg in &mut args {
                        if arg.is_string() {
                            if !seen.is_empty() && rng.below(2) == 0 {
                                *arg = seen[rng.below(seen.len())].clone();
                            } else {
                                seen.push(arg.clone());
                            }
                        }
                    }
                    json!({ "method": m.name, "args": args })
                })
                .collect();
            json!({ "class": class.name, "ctor": ctor, "calls": calls })
        })
        .collect()
}

/// Property names for generated objects: plain, nested-looking, integer-like (JS orders those
/// first), non-ASCII.
const KEYS: &[&str] = &[
    "a", "b", "nav", "home", "title", "2", "10", "ünï", "😀", "a.b",
];

/// A value that fits a TS shape. Named types resolve against `types`; recursion stops at a depth.
fn shaped(s: &Shape, types: &[LocalType], rng: &mut Rng, depth: usize) -> Value {
    match s {
        Shape::Str => value(&RType::Str, rng),
        Shape::Num => value(&RType::F64, rng),
        Shape::Int => value(&RType::Int, rng),
        Shape::Bool => value(&RType::Bool, rng),
        Shape::Null => Value::Null,
        Shape::Undefined => json!({ "$undefined": true }),
        Shape::LitStr(text) => Value::String(text.clone()),
        Shape::LitNum(n) => json!(n),
        Shape::LitBool(b) => Value::Bool(*b),
        Shape::Union(items) if !items.is_empty() => {
            shaped(&items[rng.below(items.len())], types, rng, depth)
        }
        Shape::Array(item) if depth < 6 => Value::Array(
            (0..rng.below(4))
                .map(|_| shaped(item, types, rng, depth + 1))
                .collect(),
        ),
        Shape::Tuple(items) => Value::Array(
            items
                .iter()
                .map(|i| shaped(i, types, rng, depth + 1))
                .collect(),
        ),
        Shape::Object(props, index) if depth < 6 => {
            let mut out = serde_json::Map::new();
            for (name, optional, prop) in props {
                if *optional && rng.below(4) == 0 {
                    continue;
                }
                out.insert(name.clone(), shaped(prop, types, rng, depth + 1));
            }
            if let Some(item) = index {
                for _ in 0..rng.below(4) {
                    out.insert(
                        KEYS[rng.below(KEYS.len())].to_string(),
                        shaped(item, types, rng, depth + 1),
                    );
                }
            }
            Value::Object(out)
        }
        Shape::Ref(name, args) if depth < 6 => {
            shaped(&shape::resolve(name, args, types), types, rng, depth + 1)
        }
        Shape::Any => value(&RType::Json, rng),
        // Too deep, or empty: a leaf ends the recursion.
        _ => Value::Null,
    }
}

fn value(t: &RType, rng: &mut Rng) -> Value {
    match t {
        RType::Str => {
            let a = STRINGS[rng.below(STRINGS.len())];
            if rng.below(5) == 0 {
                Value::String(format!("{a}{}", STRINGS[rng.below(STRINGS.len())]))
            } else {
                Value::String(a.into())
            }
        }
        RType::Int => {
            // Small, signed, and the edges of 32 and 64 bits.
            let n: [i64; 12] = [
                0,
                1,
                -1,
                2,
                7,
                42,
                100,
                -100,
                255,
                i64::from(i32::MAX),
                i64::from(i32::MIN),
                i64::MAX,
            ];
            json!(n[rng.below(n.len())])
        }
        RType::F64 => {
            let n = [
                0.0,
                1.0,
                -1.0,
                2.0,
                3.0,
                10.0,
                0.5,
                -2.5,
                42.0,
                100.0,
                1000.0,
                1e6,
                2_147_483_648.0,
                9_007_199_254_740_992.0,
            ];
            match rng.below(n.len() + 4) {
                i if i < n.len() => json!(n[i]),
                i if i == n.len() => json!({ "$num": "-0" }),
                i if i == n.len() + 1 => json!({ "$num": "NaN" }),
                i if i == n.len() + 2 => json!({ "$num": "Infinity" }),
                _ => json!({ "$num": "-Infinity" }),
            }
        }
        RType::Bool => Value::Bool(rng.below(2) == 0),
        RType::Unit => json!({ "$undefined": true }),
        RType::Vec(inner) => Value::Array((0..rng.below(4)).map(|_| value(inner, rng)).collect()),
        RType::Opt(inner) => {
            if rng.below(4) == 0 {
                json!({ "$undefined": true })
            } else {
                value(inner, rng)
            }
        }
        RType::Json => {
            let values: Vec<Value> = serde_json::from_str(JSON_VALUES).unwrap_or_default();
            values
                .get(rng.below(values.len().max(1)))
                .cloned()
                .unwrap_or(Value::Null)
        }
    }
}

/// xorshift64*: deterministic, so a recording is reproducible.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let x = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        usize::try_from(x >> 33).unwrap_or(0) % n
    }
}

/// How long one recorded case may run before the original is stopped on it:
/// `RUST_ANYTHING_CASE_TIMEOUT` seconds, 10 by default.
fn case_timeout() -> std::time::Duration {
    let secs = std::env::var("RUST_ANYTHING_CASE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s > 0.0)
        .unwrap_or(10.0);
    std::time::Duration::from_secs_f64(secs)
}

/// Runs a record driver over `cases` with a watchdog. The driver appends the index of each case
/// it starts to `RUST_ANYTHING_PROGRESS` and flushes each recorded line. A case that runs past
/// the time limit (an input that makes the original allocate or loop for ever) stops the driver;
/// the lines recorded so far are kept, that input is left out, and the driver goes on with the
/// rest. Returns the inputs left out.
pub(crate) fn record_cases(
    cases: &[Value],
    work: &Path,
    out: &Path,
    what: &str,
    mut command: impl FnMut(&Path, &Path) -> Result<Command>,
) -> Result<Vec<Value>> {
    use std::process::Stdio;
    use std::time::Instant;
    std::fs::create_dir_all(work)?;
    let timeout = case_timeout();
    let inputs = work.join("record-inputs.json");
    let part = work.join("record-part.jsonl");
    let progress = work.join("record-progress");
    let stderr_path = work.join("record-stderr");
    let mut recorded = String::new();
    let mut timed_out = Vec::new();
    let mut start = 0;
    while start < cases.len() {
        std::fs::write(&inputs, serde_json::to_string(&cases[start..])?)?;
        std::fs::write(&progress, "")?;
        let _ = std::fs::remove_file(&part);
        let mut cmd = command(&inputs, &part)?;
        cmd.env("RUST_ANYTHING_PROGRESS", &progress)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(&stderr_path)?);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            cmd.process_group(0);
        }
        let _scratch = scratch_cwd(&mut cmd)?;
        let mut child = cmd.spawn().with_context(|| format!("running {what}"))?;
        let mut current: Option<(usize, Instant)> = None;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break Some(status);
            }
            let started = std::fs::read_to_string(&progress).ok().and_then(|p| {
                p.lines()
                    .last()
                    .and_then(|l| l.trim().parse::<usize>().ok())
            });
            match (started, current) {
                (Some(i), Some((j, _))) if i == j => {}
                (Some(i), _) => current = Some((i, Instant::now())),
                (None, _) => {}
            }
            if let Some((_, since)) = current
                && since.elapsed() > timeout
            {
                #[cfg(unix)]
                {
                    let _ = Command::new("/bin/kill")
                        .args(["-KILL", "--", &format!("-{}", child.id())])
                        .stderr(Stdio::null())
                        .status();
                }
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        let written = std::fs::read_to_string(&part).unwrap_or_default();
        let complete = written.rfind('\n').map_or("", |end| &written[..=end]);
        recorded.push_str(complete);
        match (status, current) {
            (Some(status), _) if status.success() => break,
            (Some(status), _) => {
                let stderr = std::fs::read_to_string(&stderr_path).unwrap_or_default();
                let tail: Vec<&str> = stderr.lines().rev().take(25).collect();
                bail!(
                    "{what} failed ({status}):\n{}",
                    tail.into_iter().rev().collect::<Vec<_>>().join("\n")
                );
            }
            (None, Some((i, _))) => {
                timed_out.push(cases[start + i].clone());
                start += i + 1;
            }
            (None, None) => unreachable!("the watchdog only stops a driver inside a case"),
        }
    }
    std::fs::write(out, recorded)?;
    std::fs::write(work.join(TIMED_OUT), serde_json::to_string(&timed_out)?)?;
    Ok(timed_out)
}

/// Inputs the last `record` left out because the original ran past the time limit on them.
pub(crate) const TIMED_OUT: &str = "timed-out.json";

pub(crate) struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn scratch_cwd(cmd: &mut Command) -> Result<Option<Scratch>> {
    if cmd.get_current_dir().is_some() {
        return Ok(None);
    }
    let dir = crate::app::temp_dir("unit")?;
    cmd.current_dir(&dir);
    Ok(Some(Scratch(dir)))
}

fn run(cmd: &mut Command, what: &str) -> Result<()> {
    let _scratch = scratch_cwd(cmd)?;
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

/// Python keyword-only parameters (after `*`) are passed by name: they move from `args` into
/// `kwargs`, and an omitted (`undefined`) one is left out.
/// Integral float arguments of `number` parameters, as ints.
fn intify(params: &[crate::ts::exports::Param], args: &mut Value) {
    let Some(items) = args.as_array_mut() else {
        return;
    };
    for (p, v) in params.iter().zip(items.iter_mut()) {
        if p.ts_type == "number"
            && let Some(x) = v.as_f64()
            && x.fract() == 0.0
            && x.abs() < 1e15
            && !(x == 0.0 && x.is_sign_negative())
        {
            *v = json!(x as i64);
        }
    }
}

fn split_keyword_only(params: &[crate::ts::exports::Param], case: &mut Value) {
    if !params.iter().any(|p| p.keyword_only) {
        return;
    }
    let Some(args) = case["args"].as_array().cloned() else {
        return;
    };
    let mut positional = Vec::new();
    let mut kwargs = serde_json::Map::new();
    for (i, value) in args.into_iter().enumerate() {
        match params.get(i) {
            Some(p) if p.keyword_only => {
                if value.get("$undefined").is_none() {
                    kwargs.insert(p.name.clone(), value);
                }
            }
            _ => positional.push(value),
        }
    }
    case["args"] = Value::Array(positional);
    case["kwargs"] = Value::Object(kwargs);
}

/// Workspace packages as `{ name: absolute source folder }` for the resolve hook: a monorepo
/// imports its own packages by name, and their build output may not exist.
fn workspace_env(manifest: &Manifest) -> String {
    let packages: BTreeMap<String, String> = crate::ts::discover::discover(&manifest.root)
        .map(|ws| {
            ws.packages
                .into_iter()
                .filter(|p| !p.dir.is_empty())
                .map(|p| {
                    let dir = manifest.root.join(&p.dir);
                    let src = if dir.join("src").is_dir() {
                        dir.join("src")
                    } else {
                        dir
                    };
                    (p.name, src.to_string_lossy().into_owned())
                })
                .collect()
        })
        .unwrap_or_default();
    serde_json::to_string(&packages).unwrap_or_else(|_| "{}".into())
}

/// The tsconfig `paths` of the unit's files, as JSON for the resolve hook: `[[pattern, [absolute targets]]]`.
fn alias_env(manifest: &Manifest) -> String {
    let mut entries: Vec<(String, Vec<String>)> = Vec::new();
    let files = manifest
        .exports
        .iter()
        .map(|f| &f.file)
        .chain(manifest.classes.iter().map(|c| &c.file));
    for file in files {
        let dir = file.rsplit_once('/').map_or("", |(d, _)| d);
        for (pattern, targets) in
            crate::ts::tsconfig::PathAliases::nearest(&manifest.root, dir).entries
        {
            if entries.iter().all(|(p, _)| *p != pattern) {
                let absolute = targets
                    .iter()
                    .map(|t| manifest.root.join(t).to_string_lossy().into_owned())
                    .collect();
                entries.push((pattern, absolute));
            }
        }
    }
    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into())
}

fn file_url(path: &Path) -> String {
    use std::fmt::Write as _;

    let text = path.to_string_lossy();
    #[cfg(windows)]
    let text = text.replace('\\', "/");
    let mut url = if text.starts_with('/') {
        String::from("file://")
    } else {
        String::from("file:///")
    };
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b':' | b'-' | b'_' | b'.' | b'~') {
            url.push(char::from(byte));
        } else {
            let _ = write!(url, "%{byte:02X}");
        }
    }
    url
}

const REGISTER_JS: &str =
    "import { register } from 'node:module'\nregister('./resolve.mjs', import.meta.url)\n";

/// Node resolve hook: bundler-style relative imports, and npm packages from
/// `RUST_ANYTHING_RESOLVE_FROM` when the source tree has none installed.
const RESOLVE_JS: &str = r#"
import { existsSync, statSync } from 'node:fs'
import { fileURLToPath, pathToFileURL } from 'node:url'

const extra = process.env.RUST_ANYTHING_RESOLVE_FROM
// tsconfig `paths`: exact keys first, then wildcard keys by longest prefix.
const aliasPriority = pattern => pattern.includes('*') ? pattern.indexOf('*') : Number.POSITIVE_INFINITY
const aliases = JSON.parse(process.env.RUST_ANYTHING_PATHS || '[]')
  .sort(([a], [b]) => aliasPriority(b) - aliasPriority(a))
// Workspace packages by name: `@x18n/core` and `@x18n/core/sub` resolve to source.
const workspace = JSON.parse(process.env.RUST_ANYTHING_WORKSPACE || '{}')
const workspaceTargets = (specifier) => {
  for (const [name, src] of Object.entries(workspace)) {
    if (specifier === name)
      return [`${src}/index`]
    if (specifier.startsWith(`${name}/`))
      return [`${src}/${specifier.slice(name.length + 1)}`, `${src}/${specifier.slice(name.length + 1)}/index`]
  }
  return []
}
const aliasTargets = (specifier) => {
  for (const [pattern, targets] of aliases) {
    const [prefix, suffix = ''] = pattern.split('*')
    const exact = !pattern.includes('*')
    if (exact ? specifier === pattern : specifier.startsWith(prefix) && specifier.endsWith(suffix) && specifier.length >= prefix.length + suffix.length) {
      const star = exact ? '' : specifier.slice(prefix.length, specifier.length - suffix.length)
      return targets.map(t => t.replace('*', () => star))
    }
  }
  return []
}
const suffixes = ['.ts', '.tsx', '.mts', '.js', '.mjs', '/index.ts', '/index.mts', '/index.js']
const sourceCandidates = path => {
  const exact = path.match(/\.(mjs|cjs|jsx)$/)
  if (exact) {
    const replacement = { mjs: 'mts', cjs: 'cts', jsx: 'tsx' }[exact[1]]
    return [path, path.slice(0, -exact[1].length) + replacement]
  }
  const stem = path.replace(/\.js$/, '')
  return [stem, ...suffixes.map(suffix => stem + suffix)]
}
const isFile = p => existsSync(p) && statSync(p).isFile()

export async function resolve(specifier, context, next) {
  // Aliases first: `@/x` is not even a valid package name for Node.
  for (const target of [...aliasTargets(specifier), ...workspaceTargets(specifier)]) {
    for (const candidate of sourceCandidates(target)) {
      if (isFile(candidate))
        return next(pathToFileURL(candidate).href, context)
    }
  }
  try {
    return await next(specifier, context)
  }
  catch (error) {
    const missing = error?.code === 'ERR_MODULE_NOT_FOUND' || error?.code === 'ERR_UNSUPPORTED_DIR_IMPORT'
    if (!missing || !context.parentURL)
      throw error
    if (specifier.startsWith('.') || specifier.startsWith('/')) {
      const path = fileURLToPath(new URL(specifier, context.parentURL))
      for (const candidate of sourceCandidates(path)) {
        if (isFile(candidate))
          return next(pathToFileURL(candidate).href, context)
      }
    }
    else if (extra) {
      return next(specifier, { ...context, parentURL: pathToFileURL(`${extra}/`).href })
    }
    throw error
  }
}
"#;

/// Encoding shared by the record, diff and shadow scripts. Lossless for what a JS caller
/// can observe: `undefined`, `NaN`, `-0`, BigInt, Map/Set order, key order, lone surrogates.
pub const CODEC_JS: &str = r#"
const enc = (v, stack = []) => {
  if (v === undefined) return { $undefined: true }
  if (typeof v === 'number') {
    if (Number.isNaN(v)) return { $num: 'NaN' }
    if (!Number.isFinite(v)) return { $num: v > 0 ? 'Infinity' : '-Infinity' }
    if (Object.is(v, -0)) return { $num: '-0' }
    return v
  }
  // Strings JS split inside a surrogate pair are not valid Unicode: keep their code units.
  if (typeof v === 'string' && !v.isWellFormed()) return { $utf16: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) }
  if (typeof v === 'bigint') return { $bigint: String(v) }
  if (typeof v === 'function') return { $function: v.name }
  if (typeof v === 'symbol') return { $symbol: String(v.description) }
  if (v === null || typeof v !== 'object') return v
  if (stack.includes(v)) return { $cycle: true }
  const next = [...stack, v]
  if (Array.isArray(v)) return v.map(x => enc(x, next))
  if (typeof Buffer !== 'undefined' && Buffer.isBuffer(v)) return { $buffer: [...v] }
  if (v instanceof Uint8Array) return { $bytes: [...v] }
  if (ArrayBuffer.isView(v) && !(v instanceof DataView)) return { $typed: { kind: v.constructor.name, values: [...v].map(x => enc(x, next)) } }
  if (v instanceof Map) return { $map: [...v].map(([k, x]) => [enc(k, next), enc(x, next)]) }
  if (v instanceof Set) return { $set: [...v].map(x => enc(x, next)) }
  if (v instanceof Date) return { $date: Number.isNaN(v.getTime()) ? 'Invalid' : v.toISOString() }
  if (v instanceof RegExp) return { $regexp: String(v) }
  if (v instanceof Error) return { $error: v.message }
  const plain = Object.fromEntries(Object.keys(v).map(k => [k, enc(v[k], next)]))
  return looksLikeMarker(plain) ? { $object: plain } : plain
}
const MARKERS = ['$undefined', '$num', '$bigint', '$utf16', '$map', '$set', '$date', '$bytes', '$buffer', '$typed', '$object']
const looksLikeMarker = (v) => {
  const keys = Object.keys(v)
  return keys.length === 1 && MARKERS.includes(keys[0])
}
const dec = (v) => {
  if (Array.isArray(v)) return v.map(dec)
  if (v === null || typeof v !== 'object') return v
  if (!looksLikeMarker(v)) return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, dec(x)]))
  if ('$object' in v) return Object.fromEntries(Object.entries(v.$object).map(([k, x]) => [k, dec(x)]))
  if ('$undefined' in v) return undefined
  if ('$num' in v) return Number(v.$num)
  if ('$bigint' in v) return BigInt(v.$bigint)
  if ('$utf16' in v) {
    let text = ''
    for (let i = 0; i < v.$utf16.length; i += 8192)
      text += String.fromCharCode(...v.$utf16.slice(i, i + 8192))
    return text
  }
  if ('$map' in v) return new Map(v.$map.map(([k, x]) => [dec(k), dec(x)]))
  if ('$set' in v) return new Set(v.$set.map(dec))
  if ('$date' in v) return new Date(v.$date)
  if ('$bytes' in v) return Uint8Array.from(v.$bytes)
  if ('$typed' in v) return globalThis[v.$typed.kind].from(v.$typed.values.map(dec))
  return Buffer.from(v.$buffer)
}
const canon = (v, sort) => {
  if (Array.isArray(v)) return v.map(x => canon(x, sort))
  if (v === null || typeof v !== 'object') return v
  const keys = Object.keys(v)
  if (sort) keys.sort()
  return Object.fromEntries(keys.map(k => [k, canon(v[k], sort)]))
}
const same = (a, b, sort = false) => JSON.stringify(canon(a, sort)) === JSON.stringify(canon(b, sort))
"#;

const RECORD_JS: &str = r#"
import { appendFileSync, readFileSync, writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

const [inputsPath, outPath] = process.argv.slice(2)
const inputs = JSON.parse(readFileSync(inputsPath, 'utf8'))
const modules = new Map()
const progress = process.env.RUST_ANYTHING_PROGRESS
writeFileSync(outPath, '')
const lines = { push: line => appendFileSync(outPath, `${line}\n`) }
// A class scenario: construct, then call each method in turn; every step's result is recorded.
const runScenario = async (Class, c) => {
  const results = []
  let instance
  try {
    instance = new Class(...c.ctor.map(dec))
    results.push({ ok: true })
  }
  catch (error) {
    results.push({ error: String(error?.message ?? error) })
    return results
  }
  for (const call of c.calls) {
    try {
      results.push({ ok: enc(await instance[call.method](...call.args.map(dec))) })
    }
    catch (error) {
      results.push({ error: String(error?.message ?? error) })
    }
  }
  return results
}
for (const [index, c] of inputs.entries()) {
  if (progress) appendFileSync(progress, `${index}\n`)
  let mod = modules.get(c.module)
  if (!mod) {
    mod = await import(pathToFileURL(c.module).href)
    modules.set(c.module, mod)
  }
  if (c.class) {
    const Class = mod[c.class] ?? mod.default?.[c.class]
    if (typeof Class !== 'function') {
      console.error(`${c.class} is not a class exported by ${c.module}`)
      process.exit(2)
    }
    pinCase(index)
    lines.push(JSON.stringify({ class: c.class, ctor: c.ctor, calls: c.calls, results: await runScenario(Class, c) }))
    continue
  }
  const fn = mod[c.fn] ?? mod.default?.[c.fn]
  if (typeof fn !== 'function') {
    console.error(`${c.fn} is not a function exported by ${c.module}`)
    process.exit(2)
  }
  pinCase(index)
  try {
    const value = await fn(...c.args.map(dec))
    lines.push(JSON.stringify({ fn: c.fn, args: c.args, ok: enc(value) }))
  }
  catch (error) {
    lines.push(JSON.stringify({ fn: c.fn, args: c.args, error: String(error?.message ?? error) }))
  }
}
"#;

const BENCH_JS: &str = r#"
import { readFileSync, writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

const [modulesPath, nativePath, casesPath, outPath, minMs] = process.argv.slice(2)
const { functions: modules, classes: classModules, asyncFunctions } = JSON.parse(readFileSync(modulesPath, 'utf8'))
const loaded = await import(pathToFileURL(nativePath).href)
const native = { ...(loaded.default && typeof loaded.default === 'object' ? loaded.default : {}), ...loaded }
const cases = readFileSync(casesPath, 'utf8').split('\n').filter(Boolean).map(l => JSON.parse(l))
const byFn = new Map()
const scenarios = new Map()
for (const c of cases.filter(c => c.class)) {
  if (!scenarios.has(c.class)) scenarios.set(c.class, [])
  scenarios.get(c.class).push(c)
}
for (const c of cases.filter(c => !c.class)) {
  if (!byFn.has(c.fn)) byFn.set(c.fn, [])
  byFn.get(c.fn).push(c.args.map(dec))
}
// Time whole passes over the cases until `minMs` has passed; errors count as calls.
const time = (fn, argsList) => {
  const pass = () => {
    for (const args of argsList) {
      try { fn(...args) } catch {}
    }
  }
  pass()
  let passes = 0
  const start = performance.now()
  do { pass(); passes++ } while (performance.now() - start < Number(minMs))
  return (performance.now() - start) * 1e6 / (passes * argsList.length)
}
const timeAsync = async (fn, argsList) => {
  const pass = async () => {
    for (const args of argsList) {
      try { await fn(...args) } catch {}
    }
  }
  await pass()
  let passes = 0
  const start = performance.now()
  do { await pass(); passes++ } while (performance.now() - start < Number(minMs))
  return (performance.now() - start) * 1e6 / (passes * argsList.length)
}
const rows = []
for (const [name, argsList] of byFn) {
  const original = (await import(pathToFileURL(modules[name]).href))[name]
  const rust = native[name]
  if (typeof original !== 'function' || typeof rust !== 'function') continue
  const timer = asyncFunctions.includes(name) ? timeAsync : time
  const originalNs = await timer(original, argsList)
  const rustNs = await timer(rust, argsList)
  rows.push({ fn: name, cases: argsList.length, original_ns: originalNs, rust_ns: rustNs, speedup: originalNs / rustNs })
}
// Classes: time whole scenarios (construct, then every call).
const play = (Class, list) => () => {
  for (const c of list) {
    try {
      const instance = new Class(...c.ctor.map(dec))
      for (const call of c.calls) {
        try { instance[call.method](...call.args.map(dec)) } catch {}
      }
    } catch {}
  }
}
for (const [name, list] of scenarios) {
  const original = (await import(pathToFileURL(classModules[name]).href))[name]
  const rust = native[name]
  if (typeof original !== 'function' || typeof rust !== 'function') continue
  const timeAll = (fn) => {
    fn()
    let passes = 0
    const start = performance.now()
    do { fn(); passes++ } while (performance.now() - start < Number(minMs))
    return (performance.now() - start) * 1e6 / (passes * list.length)
  }
  const originalNs = timeAll(play(original, list))
  const rustNs = timeAll(play(rust, list))
  rows.push({ fn: `class ${name}`, cases: list.length, original_ns: originalNs, rust_ns: rustNs, speedup: originalNs / rustNs })
}
writeFileSync(outPath, JSON.stringify(rows))
"#;

const DIFF_JS: &str = r#"
import { readFileSync, writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

const [modulePath, casesPath, outPath, keyOrder] = process.argv.slice(2)
const loaded = await import(pathToFileURL(modulePath).href)
const mod = { ...(loaded.default && typeof loaded.default === 'object' ? loaded.default : {}), ...loaded }
const cases = readFileSync(casesPath, 'utf8').split('\n').filter(Boolean).map(l => JSON.parse(l))
const summary = { total: cases.length, passed: 0, failed: 0, missing: [], mismatches: [] }
const scenario = async (Class, c) => {
  const results = []
  let instance
  try {
    instance = new Class(...c.ctor.map(dec))
    results.push({ ok: true })
  }
  catch (error) {
    results.push({ error: String(error?.message ?? error) })
    return results
  }
  for (const call of c.calls) {
    try {
      results.push({ ok: enc(await instance[call.method](...call.args.map(dec))) })
    }
    catch (error) {
      results.push({ error: String(error?.message ?? error) })
    }
  }
  return results
}
const stepsMatch = (want, got) => want.length === got.length && want.every((w, i) =>
  ('error' in w && 'error' in got[i] && !got[i].error.startsWith('Rust panic:')) || ('ok' in w && 'ok' in got[i] && same(w.ok, got[i].ok, keyOrder === 'sort')))
for (const [index, c] of cases.entries()) {
  pinCase(index)
  if (c.class) {
    const Class = mod[c.class]
    if (typeof Class !== 'function') {
      if (!summary.missing.includes(c.class)) summary.missing.push(c.class)
      summary.failed++
      continue
    }
    const got = await scenario(Class, c)
    if (stepsMatch(c.results, got)) {
      summary.passed++
    }
    else {
      summary.failed++
      summary.mismatches.push({ fn: c.class, args: [c.ctor, c.calls], expected: c.results, actual: got })
    }
    continue
  }
  const fn = mod[c.fn]
  let actual
  if (typeof fn !== 'function') {
    if (!summary.missing.includes(c.fn)) summary.missing.push(c.fn)
    actual = { missing: true }
  }
  else {
    try {
      actual = { ok: enc(await fn(...c.args.map(dec))) }
    }
    catch (error) {
      actual = { error: String(error?.message ?? error) }
    }
  }
  const expected = 'error' in c ? { error: c.error } : { ok: c.ok }
  // Both sides throwing counts as agreement: error messages are not part of the contract.
  const panicked = 'error' in actual && actual.error.startsWith('Rust panic:')
  const pass = !panicked && ('error' in expected && 'error' in actual) || ('ok' in expected && 'ok' in actual && same(expected.ok, actual.ok, keyOrder === 'sort'))
  if (pass) {
    summary.passed++
  }
  else {
    summary.failed++
    summary.mismatches.push({ fn: c.fn, args: c.args, expected, actual })
  }
}
writeFileSync(outPath, JSON.stringify(summary, null, 2))
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ts::exports::{ExportedFn, Param};

    #[test]
    fn the_js_codec_keeps_marker_shaped_data_and_typed_arrays() {
        let script = format!(
            "{CODEC_JS}\n{}",
            r#"
const assert = (ok, what) => { if (!ok) { console.error(what); process.exit(1) } }
const round = v => dec(JSON.parse(JSON.stringify(enc(v))))
const plain = [{ $date: '2024-01-01T00:00:00.000Z' }, { $undefined: true }, { $bigint: '7' }, { $object: 1 }]
for (const v of plain) assert(JSON.stringify(round(v)) === JSON.stringify(v), 'plain ' + JSON.stringify(v))
const wide = dec({ $date: '2024-01-01T00:00:00.000Z', tz: 'UTC' })
assert(wide.tz === 'UTC' && typeof wide.$date === 'string', 'sibling keys are data')
assert(round(new Date(0)) instanceof Date, 'dates still decode')
for (const T of [Float64Array, Float32Array, Int32Array, Uint8ClampedArray, BigInt64Array]) {
  const v = T === BigInt64Array ? new T([1n, -2n]) : new T([1, 2])
  const back = round(v)
  assert(back instanceof T && back.length === 2 && back[1] === v[1], 'typed ' + T.name)
}
assert(JSON.stringify(enc(new Float64Array([1, 2]))) !== JSON.stringify(enc(new Int32Array([1, 2]))), 'kinds differ')
assert(round(new Uint8Array([1])) instanceof Uint8Array, 'bytes stay bytes')
"#
        );
        let Ok(out) = Command::new("node")
            .args(["--input-type=module", "-e", &script])
            .output()
        else {
            return;
        };
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn f(params: &[(&str, &str)]) -> ExportedFn {
        ExportedFn {
            name: "f".into(),
            file: "a.ts".into(),
            line: 1,
            params: params
                .iter()
                .map(|(n, t)| Param {
                    name: (*n).into(),
                    ts_type: (*t).into(),
                    optional: false,
                    rest: false,
                    ..Default::default()
                })
                .collect(),
            returns: "number".into(),
            is_async: false,
            is_generator: false,
            type_params: Vec::new(),
            uses_instanceof: false,
            ..Default::default()
        }
    }

    #[test]
    fn reads_case_lines_deeper_than_128() {
        let deep = format!("{}1{}", "[".repeat(300), "]".repeat(300));
        assert!(serde_json::from_str::<Value>(&deep).is_err());
        assert!(parse_deep(&deep).is_ok());
        assert!(parse_deep("[1] x").is_err());
    }

    #[test]
    fn generates_deterministic_distinct_inputs() {
        let a = generate(&f(&[("q", "string"), ("t", "string")]), 50, &[]);
        let b = generate(&f(&[("q", "string"), ("t", "string")]), 50, &[]);
        assert_eq!(a, b);
        assert_eq!(a.len(), 50);
        let unique: std::collections::HashSet<String> = a.iter().map(|v| v.to_string()).collect();
        assert_eq!(unique.len(), 50);
        assert!(
            a.iter()
                .any(|args| args.to_string().contains('😀') || args.to_string().contains('👍'))
        );
    }

    #[test]
    fn generates_values_that_fit_object_types() {
        let types = crate::ts::exports::local_types(
            "m.ts",
            "export interface Edit { readonly range: { start: number, end: number }, readonly text: string }\n",
        );
        let cases = generate(&f(&[("edits", "readonly Edit[]")]), 60, &types);
        let fitting = cases
            .iter()
            .filter_map(|args| args[0].as_array())
            .flatten()
            .filter(|e| e["range"]["start"].is_number() || e["range"]["start"].is_object())
            .count();
        assert!(fitting > 20, "{fitting}");
    }

    #[test]
    fn covers_number_edge_cases() {
        let cases = generate(&f(&[("n", "number")]), 18, &[]);
        let text = serde_json::to_string(&cases).unwrap();
        assert!(text.contains("NaN") && text.contains("-0"));
    }
}

#[cfg(test)]
mod verdict_tests {
    use super::*;

    fn row(name: &str, speedup: f64) -> BenchRow {
        BenchRow {
            name: name.into(),
            cases: 1,
            original_ns: speedup,
            rust_ns: 1.0,
            speedup,
        }
    }

    #[test]
    fn advice_follows_the_numbers() {
        let ts = crate::lang::Lang::Ts;
        assert!(
            verdict(&[row("a", 3.0), row("b", 2.0)], ts)
                .advice
                .starts_with("Ship it: every")
        );
        assert!(
            verdict(&[row("a", 3.0), row("b", 1.0)], ts)
                .advice
                .starts_with("Ship it: no function")
        );
        let mixed = verdict(&[row("a", 4.0), row("b", 0.5)], ts);
        assert!(mixed.advice.contains("Keep the slower ones") && mixed.advice.ends_with("b."));
        assert!(
            verdict(&[row("a", 0.2), row("b", 0.3)], ts)
                .advice
                .starts_with("Do not ship")
        );
        assert!(
            verdict(&[row("a", 0.5)], crate::lang::Lang::C)
                .advice
                .starts_with("No call overhead")
        );
    }
}

/// Parses one case line: recorded values (a pretty-printer's document trees) nest deeper than
/// serde_json's default limit of 128.
pub(crate) fn parse_deep(text: &str) -> serde_json::Result<Value> {
    let mut de = serde_json::Deserializer::from_str(text);
    de.disable_recursion_limit();
    let value = Value::deserialize(&mut de)?;
    de.end()?;
    Ok(value)
}
