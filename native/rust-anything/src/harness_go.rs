//! The Go side of record, build, diff and bench. A generated driver calls the package's
//! functions by name: built against the original package to record, and against the cgo
//! wrapper over the Rust library to diff. Both builds are separate modules under `.rust-anything/`
//! with `replace` directives, so nothing is written into the original repository.

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
        let tail: Vec<&str> = stderr.lines().rev().take(30).collect();
        bail!(
            "{what} failed ({}):\n{}",
            output.status,
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        );
    }
    Ok(())
}

/// The folder of the unit's package.
fn unit_dir(m: &Manifest) -> PathBuf {
    let dir = m.root.join(m.unit.trim_end_matches('/'));
    if dir.is_file() {
        dir.parent().map(Path::to_path_buf).unwrap_or(dir)
    } else {
        dir
    }
}

/// The module that holds the unit: its folder, `go.mod` text and module path.
fn module_of(m: &Manifest) -> Result<(PathBuf, String, String)> {
    let mut dir = unit_dir(m);
    loop {
        let go_mod = dir.join("go.mod");
        if go_mod.exists() {
            let text = std::fs::read_to_string(&go_mod)?;
            let path = crate::lang::go::module_path(&text)
                .with_context(|| format!("no `module` line in {}", go_mod.display()))?;
            return Ok((dir, text, path));
        }
        if !dir.pop() {
            bail!("no go.mod above {}", unit_dir(m).display());
        }
    }
}

/// The import path of the unit's package.
pub fn import_path(m: &Manifest) -> Result<String> {
    let (dir, _, module) = module_of(m)?;
    let rel = unit_dir(m)
        .strip_prefix(&dir)
        .map(|r| r.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    Ok(if rel.is_empty() {
        module
    } else {
        format!("{module}/{rel}")
    })
}

fn go_version(go_mod: &str) -> &str {
    crate::lang::go::mod_directive(go_mod, "go").unwrap_or("1.21")
}

/// The wrapper module's path.
pub fn wrapper_module(m: &Manifest) -> String {
    format!("rust-anything.local/{}", m.lib_name())
}

pub fn wrapper_go_mod(m: &Manifest) -> String {
    format!("module {}\n\ngo 1.21\n", wrapper_module(m))
}

fn go() -> Command {
    let mut cmd = Command::new("go");
    cmd.env("GOWORK", "off").env("GOFLAGS", "-mod=mod");
    cmd
}

/// A local replacement still requires a version compatible with the module path.
fn local_module_version(module: &str) -> String {
    let suffix = if module.starts_with("gopkg.in/") {
        module.trim_end_matches("-unstable").rsplit_once(".v")
    } else {
        module.rsplit_once("/v")
    };
    suffix
        .and_then(|(_, major)| major.parse::<u64>().ok())
        .map_or_else(|| "v0.0.0".into(), |major| format!("v{major}.0.0"))
}

/// Writes and builds a driver module for the original (`rust: false`) or the wrapper.
fn build_driver(crate_dir: &Path, m: &Manifest, rust: bool) -> Result<PathBuf> {
    let crate_dir = &crate_dir.canonical()?;
    let (module_dir, go_mod, module) = module_of(m)?;
    let name = if rust { "go-rust" } else { "go-original" };
    let dir = crate_dir.join(WORK_DIR).join(name);
    std::fs::create_dir_all(&dir)?;
    let (target_module, target_dir, import) = if rust {
        (
            wrapper_module(m),
            crate_dir.join("go").canonical()?,
            wrapper_module(m),
        )
    } else {
        (module.clone(), module_dir.clone(), import_path(m)?)
    };
    let driver_module = format!("{target_module}/rust-anything-driver");
    let mut text = format!(
        "module {driver_module}\n\ngo {}\n\nrequire {target_module} {}\n\nreplace {target_module} => {}\n",
        go_version(&go_mod),
        local_module_version(&target_module),
        serde_json::to_string(&target_dir.to_str().context("Go module path is not UTF-8")?)?
    );
    if rust {
        // The wrapper copies the original's types only; it needs nothing from the original module.
        text.push('\n');
    }
    std::fs::write(dir.join("go.mod"), text)?;
    // The original's checksums let its dependencies resolve without editing its go.sum.
    if let Ok(sum) = std::fs::read_to_string(module_dir.join("go.sum")) {
        std::fs::write(dir.join("go.sum"), sum)?;
    }
    std::fs::write(dir.join("main.go"), crate::codegen_go::driver(m, &import))?;
    if rust {
        let stale = crate_dir
            .join("target/release")
            .join(format!("lib{}.a", m.lib_name()));
        if stale.is_file() {
            std::fs::remove_file(&stale)
                .with_context(|| format!("removing {}", stale.display()))?;
        }
    }
    let bin = dir.join("driver");
    let mut command = go();
    command
        .args(["build", "-o"])
        .arg(&bin)
        .arg(".")
        .current_dir(&dir);
    if rust {
        command.env("CGO_ENABLED", "1");
    }
    run(
        &mut command,
        if rust {
            "go build (driver over the Rust wrapper)"
        } else {
            "go build (driver over the original package)"
        },
    )?;
    Ok(bin)
}

/// The driver built over the Rust wrapper loads the port's shared library. Windows finds a DLL
/// through `PATH`, not a path recorded in the binary, so its folder goes first there.
fn driver_command(crate_dir: &Path, bin: &Path) -> Command {
    let mut cmd = Command::new(bin);
    if cfg!(windows) {
        let mut dirs = vec![crate_dir.join("target").join("release")];
        dirs.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        if let Ok(path) = std::env::join_paths(dirs) {
            cmd.env("PATH", path);
        }
    }
    cmd
}

pub fn record(crate_dir: &Path, m: &Manifest, cases: &[Value]) -> Result<PathBuf> {
    let bin = build_driver(crate_dir, m, false)?;
    let slim: Vec<Value> = cases
        .iter()
        .map(|c| serde_json::json!({ "fn": c["fn"], "args": c["args"] }))
        .collect();
    let out = crate_dir.join(CASES);
    crate::harness::record_cases(
        &slim,
        &crate_dir.join(WORK_DIR),
        &out,
        "Go driver (record)",
        |inputs, part| {
            let mut cmd = Command::new(&bin);
            cmd.arg("record").arg(inputs).arg(part);
            Ok(cmd)
        },
    )?;
    Ok(out)
}

pub fn diff(crate_dir: &Path, m: &Manifest) -> Result<DiffOutcome> {
    let cases = crate_dir.join(CASES);
    if !cases.exists() {
        bail!(
            "no {CASES} in {} (run `rust-anything record` first)",
            crate_dir.display()
        );
    }
    let bin = build_driver(crate_dir, m, true)?;
    let out = crate_dir.join(WORK_DIR).join("diff.json");
    run(
        driver_command(crate_dir, &bin)
            .arg("diff")
            .arg(&cases)
            .arg(&out),
        "Go driver (diff)",
    )?;
    Ok(serde_json::from_str(&std::fs::read_to_string(&out)?)?)
}

pub fn bench(crate_dir: &Path, m: &Manifest, min_ms: u64) -> Result<Vec<BenchRow>> {
    let cases = crate_dir.join(CASES);
    if !cases.exists() {
        bail!(
            "no {CASES} in {} (run `rust-anything record` first)",
            crate_dir.display()
        );
    }
    let mut timings = Vec::new();
    for rust in [false, true] {
        let bin = build_driver(crate_dir, m, rust)?;
        let out = crate_dir.join(WORK_DIR).join(if rust {
            "bench-rust.json"
        } else {
            "bench-original.json"
        });
        run(
            driver_command(crate_dir, &bin)
                .arg("bench")
                .arg(&cases)
                .arg(&out)
                .arg(min_ms.to_string()),
            "Go driver (bench)",
        )?;
        let parsed: serde_json::Map<String, Value> =
            serde_json::from_str(&std::fs::read_to_string(&out)?)?;
        timings.push(parsed);
    }
    Ok(crate::harness::bench_rows(m, &timings[0], &timings[1]))
}
