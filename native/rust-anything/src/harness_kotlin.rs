//! The Kotlin side of record, build, diff and bench.
//!
//! Mirrors the Java harness, but compiles the original Kotlin sources with `kotlinc` into a
//! fat jar (`-include-runtime`, so the Kotlin stdlib is on the runtime classpath) and then
//! compiles the generated Java reflection driver and wrappers against that jar. Kotlin
//! static methods are callable from Java, so the same `rustanything.RustAnythingDriver`
//! is used for record, diff and bench.

use crate::walk::Canonical;
use std::ffi::OsString;
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
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let tail: Vec<&str> = text
            .lines()
            .filter(|l| !l.starts_with("Picked up JAVA_TOOL_OPTIONS"))
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

fn java_files(dir: &Path) -> Vec<PathBuf> {
    files_with_ext(dir, "java")
}

fn files_with_ext(dir: &Path, ext: &str) -> Vec<PathBuf> {
    crate::walk::builder(dir)
        .require_git(false)
        .build()
        .flatten()
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|e| e == ext))
        .collect()
}

/// Classpath for a generated driver run: generated classes, plus the original Kotlin
/// fat jar when recording (the Rust side needs no Kotlin sources).
struct Classpath {
    classes: PathBuf,
    original_jar: Option<PathBuf>,
}

impl Classpath {
    fn value(&self) -> Result<OsString> {
        let mut parts = vec![self.classes.as_path()];
        if let Some(jar) = &self.original_jar {
            parts.push(jar.as_path());
        }
        std::env::join_paths(parts).context("building Java classpath")
    }
}

fn needs_android_sdk(path: &Path) -> bool {
    path.components()
        .any(|c| c.as_os_str() == std::ffi::OsStr::new("androidTest"))
        || std::fs::read_to_string(path).is_ok_and(|text| {
            text.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with("import android.")
                    || line.starts_with("import androidx.")
                    || line.starts_with("import com.google.android.")
            })
        })
}

/// Compiles the support driver and either the original Kotlin sources (`rust: false`) or
/// the generated Java JNI wrappers (`rust: true`).
fn compile(crate_dir: &Path, m: &Manifest, rust: bool) -> Result<Classpath> {
    let work = crate_dir.canonical()?.join(WORK_DIR);
    let support = work.join("kotlin-support/rust-anything");
    std::fs::create_dir_all(&support)?;
    std::fs::write(
        support.join("RustAnythingJson.java"),
        crate::codegen_kotlin::JSON_JAVA,
    )?;
    std::fs::write(
        support.join("RustAnythingDriver.java"),
        crate::codegen_kotlin::DRIVER_JAVA,
    )?;

    let classes = work.join(if rust {
        "kotlin-rust"
    } else {
        "kotlin-original"
    });
    if classes.exists() {
        std::fs::remove_dir_all(&classes)?;
    }
    std::fs::create_dir_all(&classes)?;

    let mut original_jar = None;
    let mut cp = Vec::new();
    if !rust {
        let jar = work.join("kotlin-original.jar");
        let ws = crate::lang::kotlin::discover(&m.root)?;
        let all: Vec<PathBuf> = ws
            .files
            .iter()
            .filter(|f| f.kind == crate::ts::discover::FileKind::Source)
            .map(|f| m.root.join(&f.path))
            .collect();
        // Compile the unit being ported plus its local dependencies: an `app/Api.kt` may
        // import `util/Helpers.kt` outside the unit. Keep Android-only sources (Activities,
        // Compose, androidx imports) out, since they need an SDK classpath the host lacks.
        let unit = m.root.join(&m.unit);
        let sources: Vec<PathBuf> = all
            .iter()
            .filter(|p| p.starts_with(&unit) || !needs_android_sdk(p))
            .cloned()
            .collect();
        let sources = if sources.is_empty() { all } else { sources };
        let mut cmd = Command::new("kotlinc");
        cmd.arg("-include-runtime")
            .arg("-d")
            .arg(&jar)
            .args(&sources);
        run(&mut cmd, "kotlinc (original sources)")?;
        original_jar = Some(jar.clone());
        cp.push(jar);
    }

    let mut sources = java_files(&work.join("kotlin-support"));
    if rust {
        sources.extend(
            java_files(&crate_dir.join("java"))
                .into_iter()
                .filter(|p| !p.ends_with("rustanything/RustAnythingJson.java")),
        );
    }
    let mut javac = Command::new("javac");
    javac.arg("-encoding").arg("UTF-8").arg("-nowarn");
    if !cp.is_empty() {
        javac
            .arg("-cp")
            .arg(std::env::join_paths(&cp).context("building Java classpath")?);
    }
    javac.arg("-d").arg(&classes).args(&sources);
    run(
        &mut javac,
        if rust {
            "javac (driver and wrapper classes)"
        } else {
            "javac (driver against original Kotlin classes)"
        },
    )?;
    Ok(Classpath {
        classes,
        original_jar,
    })
}

/// Java class files for a dexed on-device run: the support driver plus either the original
/// Kotlin fat jar (`rust: false`) or the generated JNI wrappers (`rust: true`).
pub(crate) fn android_classes(crate_dir: &Path, m: &Manifest, rust: bool) -> Result<Vec<PathBuf>> {
    let cp = compile(crate_dir, m, rust)?;
    let mut files = files_with_ext(&cp.classes, "class");
    if let Some(jar) = cp.original_jar {
        files.push(jar);
    }
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
    let cp = compile(crate_dir, m, rust)?;
    let work = crate_dir.join(WORK_DIR);
    let targets = work.join("targets.json");
    std::fs::write(&targets, crate::codegen_kotlin::targets(m).to_string())?;
    let mut cmd = Command::new("java");
    cmd.arg("-XX:-UsePerfData");
    cmd.arg("-Dfile.encoding=UTF-8");
    if rust {
        let lib = crate::harness_c::built_library(crate_dir, m)?;
        cmd.arg(format!(
            "-Drust-anything.lib.{}={}",
            m.lib_name(),
            lib.canonical()?.display()
        ));
    }
    cmd.arg("-cp")
        .arg(cp.value()?)
        .arg("rustanything.RustAnythingDriver")
        .arg(mode)
        .arg(&targets)
        .arg(cases)
        .arg(out)
        .arg(min_ms.to_string());
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
    run(&mut cmd, &format!("java driver ({mode})"))
}

pub fn record(crate_dir: &Path, m: &Manifest, cases: &[Value]) -> Result<PathBuf> {
    std::fs::create_dir_all(crate_dir.join(WORK_DIR))?;
    let out = crate_dir.join(CASES);
    crate::harness::record_cases(
        cases,
        &crate_dir.join(WORK_DIR),
        &out,
        "java driver (record)",
        |inputs, part| driver_command(crate_dir, m, "record", inputs, part, 0, false),
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
    let out = crate_dir.join(WORK_DIR).join("diff.json");
    driver(crate_dir, m, "diff", &cases, &out, 0, true)?;
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
