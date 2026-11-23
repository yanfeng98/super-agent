//! The Ruby side of record, build, diff and bench. A generated driver calls the module
//! functions by name: with the original files loaded to record, and with the Fiddle wrapper
//! over the Rust library loaded to diff.

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

/// The wrapper file, relative to the crate.
pub fn wrapper_path(m: &Manifest) -> String {
    format!("ruby/{}.rb", m.lib_name())
}

/// The files to load: the unit's sources, or the wrapper.
fn files(crate_dir: &Path, m: &Manifest, rust: bool) -> Result<Vec<PathBuf>> {
    if rust {
        return Ok(vec![crate_dir.join(wrapper_path(m)).canonical()?]);
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
    run(&mut cmd, &format!("ruby driver ({mode})"))
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
    let script = work.join("driver.rb");
    std::fs::write(&script, crate::codegen_ruby::driver())?;
    let targets = work.join("targets.json");
    std::fs::write(&targets, crate::codegen_ruby::targets(m).to_string())?;
    let mut cmd = Command::new("ruby");
    // Source and data are UTF-8 whatever the machine's locale says.
    cmd.args(["-E", "UTF-8:UTF-8"]);
    // The original's `require "name"` finds its own files through each package's lib folder.
    if !rust && let Ok(ws) = crate::lang::ruby::discover(&m.root) {
        for p in &ws.packages {
            cmd.arg(format!("-I{}", m.root.join(&p.src_root).display()));
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

pub fn record(crate_dir: &Path, m: &Manifest, cases: &[Value]) -> Result<PathBuf> {
    let out = crate_dir.join(CASES);
    let pending = crate_dir.join(WORK_DIR).join("recording.jsonl");
    crate::harness::record_cases(
        cases,
        &crate_dir.join(WORK_DIR),
        &pending,
        "ruby driver (record)",
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
             (a binary String result?); kept the previous {CASES}"
        );
    }
    std::fs::rename(&pending, &out).with_context(|| format!("publishing {}", out.display()))?;
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

/// Builds the extension and puts it beside the wrapper as `ruby/<lib>_native.so`.
pub fn build(crate_dir: &Path, m: &Manifest) -> Result<PathBuf> {
    let lib = crate::harness_c::build(crate_dir, m).map_err(explain_old_ruby)?;
    let ext = if cfg!(target_os = "macos") {
        "bundle"
    } else {
        "so"
    };
    let target = crate_dir.join(format!("ruby/{}_native.{ext}", m.lib_name()));
    std::fs::create_dir_all(crate_dir.join("ruby"))?;
    std::fs::copy(&lib, &target)?;
    Ok(target)
}

/// rb-sys's error for a Ruby it has no stable API for: older than it supports, or newer.
const NO_STABLE_API: &str = "Stable API is needed but could not find a candidate";

/// rb-sys answers a Ruby it has no stable API for with a hint about its own features. When the
/// Ruby it built against (`RUBY`, else `ruby`) is older than `doctor`'s minimum, this adds that
/// cause and doctor's fix. Only a failed build gets it, so a Ruby chosen through `RUBY` or a
/// version manager never stops a build that would work.
fn explain_old_ruby(err: anyhow::Error) -> anyhow::Error {
    if !err.to_string().contains(NO_STABLE_API) {
        return err;
    }
    let mut cmd = version_probe();
    // Not the caller's folder: rb-sys runs Ruby from its own, where a version manager's
    // `.ruby-version` of the project does not apply.
    let Ok(_scratch) = crate::harness::scratch_cwd(&mut cmd) else {
        return err;
    };
    let banner = crate::process::output(&mut cmd, crate::process::QUICK)
        .ok()
        .filter(|out| out.success())
        .map(|out| out.stdout_text());
    old_ruby_note(err, banner.as_deref(), std::env::var_os("RUBY").is_some())
}

/// `ruby --version` for the Ruby rb-sys asks: `RUBY`, else `ruby`, without `RUBYOPT` as rb-sys
/// runs it (an option the old Ruby rejects would make it fail before printing its version).
fn version_probe() -> Command {
    let mut cmd = Command::new(std::env::var_os("RUBY").unwrap_or_else(|| "ruby".into()));
    cmd.arg("--version").env_remove("RUBYOPT");
    cmd
}

/// `err` with the cause and doctor's fix when `banner` (`ruby --version`) is below the minimum.
/// When `RUBY` chose that Ruby, installing another is not enough: `RUBY` has to change too.
fn old_ruby_note(err: anyhow::Error, banner: Option<&str>, ruby_var: bool) -> anyhow::Error {
    let (Some(banner), Some(min)) = (banner.map(str::trim), crate::doctor::minimum("ruby")) else {
        return err;
    };
    if !crate::doctor::parse_version(banner).is_some_and(|v| v < min) {
        return err;
    }
    let system = if cfg!(target_os = "macos") {
        " (macOS's system Ruby is 2.6)"
    } else {
        ""
    };
    let fix = crate::doctor::install_command("ruby")
        .map(|fix| format!("\nfix: {fix}"))
        .unwrap_or_default();
    let ruby_var = if ruby_var {
        "\n`RUBY` is set, and rb-sys uses it before `ruby` on PATH: point it at the new Ruby, or unset it"
    } else {
        ""
    };
    anyhow::anyhow!(
        "{err}\n\n{banner}: rust-anything needs Ruby {}.{}+{system}{fix}{ruby_var}",
        min.0,
        min.1
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed() -> anyhow::Error {
        anyhow::anyhow!(
            "cargo build failed (exit status: 101):\n  Failed to setup stable API: {NO_STABLE_API}. \
             Try enabling the `stable-api-compiled-fallback` feature in rb-sys."
        )
    }

    #[test]
    fn a_ruby_below_the_minimum_gets_the_cause_and_the_fix() {
        let banner = "ruby 2.6.10p210 (2022-04-12 revision 67958) [universal.arm64e-darwin26]";
        let explained = old_ruby_note(failed(), Some(banner), false).to_string();
        assert!(explained.starts_with("cargo build failed"), "{explained}");
        assert!(
            explained.contains(&format!("\n\n{banner}: rust-anything needs Ruby 3.0+")),
            "{explained}"
        );
        let fix = crate::doctor::install_command("ruby").unwrap();
        assert!(explained.ends_with(&format!("\nfix: {fix}")), "{explained}");
    }

    #[test]
    fn a_newer_or_unknown_ruby_keeps_rb_sys_error() {
        let plain = failed().to_string();
        let newer = "ruby 4.1.0preview1 (2026-12-25 master 0123456789) +PRISM [arm64-darwin25]";
        assert_eq!(
            old_ruby_note(failed(), Some(newer), false).to_string(),
            plain
        );
        assert_eq!(old_ruby_note(failed(), None, true).to_string(), plain);
    }

    #[test]
    fn a_ruby_chosen_through_ruby_var_says_to_change_it() {
        let banner = "ruby 2.6.10p210 (2022-04-12 revision 67958) [universal.arm64e-darwin26]";
        let explained = old_ruby_note(failed(), Some(banner), true).to_string();
        let fix = crate::doctor::install_command("ruby").unwrap();
        assert!(
            explained.ends_with(&format!(
                "\nfix: {fix}\n`RUBY` is set, and rb-sys uses it before `ruby` on PATH: \
                 point it at the new Ruby, or unset it"
            )),
            "{explained}"
        );
    }

    #[test]
    fn the_version_probe_drops_rubyopt_as_rb_sys_does() {
        let probe = version_probe();
        assert_eq!(probe.get_args().collect::<Vec<_>>(), ["--version"]);
        assert!(
            probe
                .get_envs()
                .any(|(key, value)| key == "RUBYOPT" && value.is_none()),
            "{probe:?}"
        );
    }

    #[test]
    fn other_build_failures_are_left_alone() {
        let failed = anyhow::anyhow!("cargo build failed (exit status: 101):\nerror[E0308]");
        assert_eq!(
            explain_old_ruby(failed).to_string(),
            "cargo build failed (exit status: 101):\nerror[E0308]"
        );
    }
}
