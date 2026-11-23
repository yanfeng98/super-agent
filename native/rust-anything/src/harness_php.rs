use crate::walk::Canonical;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::bridge::Manifest;
use crate::harness::{BenchRow, CASES, DiffOutcome};

const WORK_DIR: &str = ".rust-anything";

fn run(cmd: &mut Command, what: &str) -> Result<()> {
    let _scratch = crate::harness::scratch_cwd(cmd)?;
    let output = cmd.output().with_context(|| format!("running {what}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let tail: Vec<&str> = stderr
            .lines()
            .chain(stdout.lines())
            .rev()
            .take(30)
            .collect();
        bail!(
            "{what} failed ({}):\n{}",
            output.status,
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        );
    }
    Ok(())
}

fn files(crate_dir: &Path, m: &Manifest, rust: bool) -> Result<Vec<PathBuf>> {
    if rust {
        return Ok(vec![
            crate_dir
                .join(crate::codegen_php::wrapper_path(m))
                .canonical()?,
        ]);
    }
    let mut files: Vec<PathBuf> = m
        .exports
        .iter()
        .map(|f| m.root.join(&f.file))
        .chain(m.js_only.iter().map(|j| m.root.join(&j.file)))
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

fn driver_command(
    crate_dir: &Path,
    m: &Manifest,
    mode: &str,
    cases: &Path,
    out: &Path,
    min_ms: u64,
    rust: bool,
) -> Result<Command> {
    let work = crate_dir.join(WORK_DIR);
    std::fs::create_dir_all(&work)?;
    let script = work.join("driver.php");
    std::fs::write(&script, crate::codegen_php::driver())?;
    let targets = work.join("targets.json");
    std::fs::write(&targets, crate::codegen_php::targets(m).to_string())?;
    let mut cmd = Command::new("php");
    cmd.args(["-d", "memory_limit=-1", "-d", "display_errors=stderr"]);
    if rust {
        cmd.args(["-d", "ffi.enable=1"]);
    } else {
        let autoload = m.root.join("vendor/autoload.php");
        if autoload.is_file() {
            cmd.env("RUST_ANYTHING_PHP_AUTOLOAD", autoload);
        }
    }
    cmd.arg(&script)
        .arg(mode)
        .arg(&targets)
        .arg(cases)
        .arg(out)
        .arg(min_ms.to_string())
        .args(files(crate_dir, m, rust)?);
    Ok(cmd)
}

fn driver(
    crate_dir: &Path,
    m: &Manifest,
    mode: &str,
    cases: &Path,
    out: &Path,
    min_ms: u64,
    rust: bool,
) -> Result<()> {
    let mut cmd = driver_command(crate_dir, m, mode, cases, out, min_ms, rust)?;
    run(&mut cmd, &format!("php driver ({mode})"))
}

pub fn record(crate_dir: &Path, m: &Manifest, cases: &[Value]) -> Result<PathBuf> {
    let out = crate_dir.join(CASES);
    let pending = crate_dir.join(WORK_DIR).join("recording.jsonl");
    crate::harness::record_cases(
        cases,
        &crate_dir.join(WORK_DIR),
        &pending,
        "php driver (record)",
        |inputs, part| driver_command(crate_dir, m, "record", inputs, part, 0, false),
    )?;
    let recorded: std::collections::HashSet<String> = std::fs::read_to_string(&pending)?
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|c| c["fn"].as_str().map(str::to_string))
        .collect();
    let mut asked: Vec<&str> = cases.iter().filter_map(|c| c["fn"].as_str()).collect();
    asked.sort_unstable();
    asked.dedup();
    if let Some(name) = asked.into_iter().find(|name| !recorded.contains(*name)) {
        bail!(
            "cannot encode recorded result for {name}: no case could be written as JSON \
             (a binary string result?); kept the previous {CASES}"
        );
    }
    std::fs::rename(&pending, &out).with_context(|| format!("publishing {}", out.display()))?;
    Ok(out)
}

fn need_cases(crate_dir: &Path) -> Result<PathBuf> {
    let cases = crate_dir.join(CASES);
    if !cases.exists() {
        bail!(
            "no {CASES} in {} (run `rust-anything record` first)",
            crate_dir.display()
        );
    }
    Ok(cases)
}

pub fn built_file(crate_dir: &Path, m: &Manifest) -> Option<PathBuf> {
    let lib = m.lib_name();
    [
        format!("lib{lib}.dylib"),
        format!("lib{lib}.so"),
        format!("{lib}.dll"),
    ]
    .into_iter()
    .map(|n| crate_dir.join("php").join(n))
    .find(|p| p.exists())
}

pub fn diff(crate_dir: &Path, m: &Manifest) -> Result<DiffOutcome> {
    let cases = need_cases(crate_dir)?;
    if built_file(crate_dir, m).is_none() {
        bail!(
            "no built library in {} (run `rust-anything build {}` first)",
            crate_dir.join("php").display(),
            crate_dir.display()
        );
    }
    let out = crate_dir.join(WORK_DIR).join("diff.json");
    driver(crate_dir, m, "diff", &cases, &out, 0, true)?;
    Ok(serde_json::from_str(&std::fs::read_to_string(&out)?)?)
}

pub fn bench(crate_dir: &Path, m: &Manifest, min_ms: u64) -> Result<Vec<BenchRow>> {
    let cases = need_cases(crate_dir)?;
    let mut timings = Vec::new();
    for rust in [false, true] {
        let out = crate_dir.join(WORK_DIR).join(if rust {
            "bench-rust.json"
        } else {
            "bench-original.json"
        });
        driver(crate_dir, m, "bench", &cases, &out, min_ms, rust)?;
        timings.push(serde_json::from_str(&std::fs::read_to_string(&out)?)?);
    }
    Ok(crate::harness::bench_rows(m, &timings[0], &timings[1]))
}

pub fn build(crate_dir: &Path, m: &Manifest) -> Result<PathBuf> {
    let lib = crate::harness_c::build(crate_dir, m)?;
    let name = lib.file_name().context("library has no file name")?;
    std::fs::create_dir_all(crate_dir.join("php"))?;
    let target = crate_dir.join("php").join(name);
    std::fs::copy(&lib, &target)?;
    Ok(target)
}
