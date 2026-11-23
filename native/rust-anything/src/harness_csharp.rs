//! The C# side of record, build, diff and bench. A generated console project compiles the unit's
//! project sources with a driver that calls the static methods by reflection: the originals to
//! record, and the P/Invoke wrapper over the Rust library to diff.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::bridge::Manifest;
use crate::harness::{BenchRow, CASES, DiffOutcome};
use crate::ts::discover::FileKind;

const WORK_DIR: &str = ".rust-anything";

fn quiet(cmd: &mut Command) -> &mut Command {
    cmd.env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1")
}

fn run(cmd: &mut Command, what: &str) -> Result<()> {
    let _scratch = crate::harness::scratch_cwd(cmd)?;
    let output = quiet(cmd)
        .output()
        .with_context(|| format!("running {what} (is `dotnet` on PATH?)"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let tail: Vec<&str> = stderr
            .lines()
            .chain(stdout.lines())
            .filter(|l| !l.trim().is_empty())
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

/// `net<major>.0` of the installed SDK.
fn framework() -> Result<String> {
    static FRAMEWORK: OnceLock<Option<String>> = OnceLock::new();
    FRAMEWORK
        .get_or_init(|| {
            let out = quiet(&mut Command::new("dotnet"))
                .arg("--version")
                .output()
                .ok()?;
            let version = String::from_utf8_lossy(&out.stdout);
            let major = version.trim().split('.').next()?.parse::<u32>().ok()?;
            Some(format!("net{major}.0"))
        })
        .clone()
        .context("`dotnet --version` did not answer: the .NET SDK is needed to record and diff C#")
}

/// The non-test sources of the projects holding the unit's exports, and the first project file.
fn sources(m: &Manifest) -> Result<(Vec<PathBuf>, Option<String>)> {
    let ws = crate::lang::csharp::discover(&m.root)?;
    let mut packages: Vec<usize> = Vec::new();
    for f in m.exports.iter().map(|f| &f.file) {
        if let Some(file) = ws.files.iter().find(|s| &s.path == f)
            && !packages.contains(&file.package)
        {
            packages.push(file.package);
        }
    }
    let mut files: Vec<PathBuf> = ws
        .files
        .iter()
        .filter(|f| f.kind == FileKind::Source && packages.contains(&f.package))
        .map(|f| m.root.join(&f.path))
        .collect();
    files.sort();
    let project = packages.first().and_then(|&i| {
        let dir = m.root.join(&ws.packages[i].dir);
        std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|e| e == "csproj"))
            .and_then(|p| std::fs::read_to_string(p).ok())
    });
    Ok((files, project))
}

/// A built driver: its assembly, and whether it calls the Rust wrapper.
struct Driver {
    dll: PathBuf,
    rust: bool,
}

/// Builds the driver (with the wrapper when `rust`).
fn prepare(crate_dir: &Path, m: &Manifest, rust: bool) -> Result<Driver> {
    let dir = crate_dir
        .join(WORK_DIR)
        .join(if rust { "cs-rust" } else { "cs-original" });
    std::fs::create_dir_all(&dir)?;
    let (mut files, project) = sources(m)?;
    if rust {
        files.push(
            crate_dir
                .join(crate::codegen_csharp::wrapper_path(m))
                .canonicalize()
                .context("the C# wrapper is missing (run `rust-anything refresh`)")?,
        );
    }
    std::fs::write(dir.join("Driver.cs"), crate::codegen_csharp::driver())?;
    std::fs::write(
        dir.join("Driver.csproj"),
        crate::codegen_csharp::project(&framework()?, &files, project.as_deref()),
    )?;
    let out = dir.join("bin");
    run(
        Command::new("dotnet")
            .arg("build")
            .arg(dir.join("Driver.csproj"))
            .args([
                "-c",
                "Release",
                "--nologo",
                "-v",
                "q",
                "-clp:ErrorsOnly",
                "-o",
            ])
            .arg(&out),
        if rust {
            "dotnet build (driver with the Rust wrapper)"
        } else {
            "dotnet build (driver with the original)"
        },
    )?;
    Ok(Driver {
        dll: out.join("RustAnythingDriver.dll"),
        rust,
    })
}

fn write_targets(crate_dir: &Path, m: &Manifest) -> Result<PathBuf> {
    let work = crate_dir.join(WORK_DIR);
    std::fs::create_dir_all(&work)?;
    let targets = work.join("targets.json");
    std::fs::write(&targets, crate::codegen_csharp::targets(m).to_string())?;
    Ok(targets)
}

fn driver_command(
    crate_dir: &Path,
    m: &Manifest,
    driver: &Driver,
    mode: &str,
    cases: &Path,
    out: &Path,
    min_ms: u64,
) -> Result<Command> {
    let targets = write_targets(crate_dir, m)?;
    let mut cmd = Command::new("dotnet");
    quiet(&mut cmd)
        .arg(&driver.dll)
        .arg(mode)
        .arg(&targets)
        .arg(cases)
        .arg(out)
        .arg(min_ms.to_string());
    if driver.rust {
        let lib = built_file(crate_dir, m).with_context(|| {
            format!(
                "no built library in {} (run `rust-anything build {}` first)",
                crate_dir.join("cs").display(),
                crate_dir.display()
            )
        })?;
        cmd.arg(m.lib_name()).arg(lib.canonicalize()?);
    }
    Ok(cmd)
}

pub fn record(crate_dir: &Path, m: &Manifest, cases: &[Value]) -> Result<PathBuf> {
    let driver = prepare(crate_dir, m, false)?;
    let out = crate_dir.join(CASES);
    let pending = crate_dir.join(WORK_DIR).join("recording.jsonl");
    crate::harness::record_cases(
        cases,
        &crate_dir.join(WORK_DIR),
        &pending,
        "C# driver (record)",
        |inputs, part| driver_command(crate_dir, m, &driver, "record", inputs, part, 0),
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
            "no case of {name} was recorded: none of its inputs converts to the parameter types \
             (pass cases that fit with `--inputs`); kept the previous {CASES}"
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
    .map(|n| crate_dir.join("cs").join(n))
    .find(|p| p.exists())
}

pub fn diff(crate_dir: &Path, m: &Manifest) -> Result<DiffOutcome> {
    let cases = need_cases(crate_dir)?;
    if built_file(crate_dir, m).is_none() {
        bail!(
            "no built library in {} (run `rust-anything build {}` first)",
            crate_dir.join("cs").display(),
            crate_dir.display()
        );
    }
    let driver = prepare(crate_dir, m, true)?;
    let out = crate_dir.join(WORK_DIR).join("diff.json");
    let mut cmd = driver_command(crate_dir, m, &driver, "diff", &cases, &out, 0)?;
    run(&mut cmd, "C# driver (diff)")?;
    Ok(serde_json::from_str(&std::fs::read_to_string(&out)?)?)
}

pub fn bench(crate_dir: &Path, m: &Manifest, min_ms: u64) -> Result<Vec<BenchRow>> {
    let cases = need_cases(crate_dir)?;
    let mut timings = Vec::new();
    for rust in [false, true] {
        let driver = prepare(crate_dir, m, rust)?;
        let out = crate_dir.join(WORK_DIR).join(if rust {
            "bench-rust.json"
        } else {
            "bench-original.json"
        });
        let mut cmd = driver_command(crate_dir, m, &driver, "bench", &cases, &out, min_ms)?;
        run(&mut cmd, "C# driver (bench)")?;
        timings.push(serde_json::from_str(&std::fs::read_to_string(&out)?)?);
    }
    Ok(crate::harness::bench_rows(m, &timings[0], &timings[1]))
}

pub fn build(crate_dir: &Path, m: &Manifest) -> Result<PathBuf> {
    let lib = crate::harness_c::build(crate_dir, m)?;
    let name = lib.file_name().context("library has no file name")?;
    std::fs::create_dir_all(crate_dir.join("cs"))?;
    let target = crate_dir.join("cs").join(name);
    std::fs::copy(&lib, &target)?;
    Ok(target)
}
