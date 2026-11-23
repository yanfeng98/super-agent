//! `fuzz` finds what the recording missed, and `record --pin-clock` makes clock and random
//! numbers recordable. Both build a real napi port, so they need `node` and cargo.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use rust_anything::bridge::{self, Kind};
use rust_anything::harness::{self, DiffOptions, RecordOptions};

const CLI: &str = env!("CARGO_BIN_EXE_rust-anything");

/// A TS repository with one file, bridged with napi into `<repo>/rust/unit`.
fn bridged(name: &str, file: &str, source: &str, port: &str) -> (PathBuf, PathBuf) {
    let repo = std::env::temp_dir().join(format!("rust-anything-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::fs::write(
        repo.join("package.json"),
        format!(r#"{{ "name": "{name}", "private": true, "type": "module" }}"#),
    )
    .unwrap();
    std::fs::write(repo.join("src").join(file), source).unwrap();
    let dir = repo.join("rust/unit");
    bridge::bridge(&bridge::Options {
        root: &repo,
        unit: &format!("src/{file}"),
        kind: Some(Kind::Napi),
        out: Some(dir.clone()),
        name: None,
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();
    std::fs::write(dir.join("src/port.rs"), port).unwrap();
    bridge::refresh(&dir).unwrap();
    (repo, dir)
}

fn record(dir: &Path, count: usize) -> harness::RecordOutcome {
    harness::record(&RecordOptions {
        crate_dir: dir,
        count,
        inputs: None,
        module: None,
        resolve_from: None,
        node_args: &[],
    })
    .unwrap()
}

fn diff(dir: &Path) -> harness::DiffOutcome {
    harness::diff(&DiffOptions {
        crate_dir: dir,
        module: None,
        ignore_key_order: false,
        float_tolerance: None,
        node_args: &[],
    })
    .unwrap()
}

fn behavior(dir: &Path) -> bool {
    let gate = rust_anything::gate::gate(dir, &[]).unwrap();
    gate.checks
        .iter()
        .find(|c| c.name == "behavior")
        .unwrap_or_else(|| panic!("no behavior check: {:?}", gate.checks))
        .passed
}

#[test]
fn fuzz_finds_a_mismatch_the_recording_missed_and_keep_makes_gate_fail() {
    if !common::have("node") {
        return;
    }
    let (repo, dir) = bridged(
        "fuzz",
        "calc.ts",
        "export function sum3(a: number, b: number, c: number): number {\n  return a + b + c\n}\n",
        "pub fn sum3(a: f64, b: f64, c: f64) -> Result<f64, String> {\n    \
         if a > 1.0 && b > 1.0 && c > 1.0 {\n        Ok(a + b)\n    } else {\n        \
         Ok(a + b + c)\n    }\n}\n",
    );
    record(&dir, 2);
    harness::build(&dir).unwrap();
    let recorded = diff(&dir);
    assert_eq!(recorded.failed, 0, "{:?}", recorded.mismatches);
    assert!(behavior(&dir), "the recorded cases all match");
    let before = std::fs::read_to_string(dir.join(harness::CASES)).unwrap();

    let fuzz = |extra: &[&str]| {
        let out = Command::new(CLI)
            .args(["fuzz", "--seed", "7", "--cases", "100", "--json"])
            .args(extra)
            .arg(&dir)
            .output()
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr)));
        (out.status.code(), report)
    };
    let (code, found) = fuzz(&[]);
    assert_eq!(code, Some(1), "{found}");
    assert_eq!(found["seed"], 7);
    let failed = found["failed"].as_u64().unwrap();
    assert!(failed > 0, "{found}");
    assert!(
        found["mismatches"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["fn"] == "sum3"),
        "{found}"
    );
    assert_eq!(found["kept"], 0);
    assert_eq!(
        std::fs::read_to_string(dir.join(harness::CASES)).unwrap(),
        before,
        "fuzz without --keep leaves the recording as it was"
    );

    let (_, again) = fuzz(&[]);
    assert_eq!(again["failed"], found["failed"], "a seed replays");

    let (code, kept) = fuzz(&["--keep"]);
    assert_eq!(code, Some(1));
    assert_eq!(kept["kept"].as_u64().unwrap(), failed, "{kept}");
    let after = std::fs::read_to_string(dir.join(harness::CASES)).unwrap();
    assert!(after.starts_with(&before));
    assert_eq!(
        after.lines().count(),
        before.lines().count() + usize::try_from(failed).unwrap()
    );
    assert!(!behavior(&dir), "gate checks the kept cases");
    std::fs::remove_dir_all(&repo).unwrap();
}

#[test]
fn pinned_clock_and_random_numbers_are_recorded_and_matched_by_a_rust_port() {
    if !common::have("node") {
        return;
    }
    let (repo, dir) = bridged(
        "pinned",
        "stamp.ts",
        "export function stamp(utc: boolean): string {\n  \
         const id = Math.floor(Math.random() * 1000000)\n  \
         return `${utc}@${Date.now()}/${new Date().getTime()}#${id}`\n}\n\n\
         export function roll(sides: number): number {\n  \
         return Math.floor(Math.random() * sides) + Math.floor(Math.random() * sides)\n}\n",
        "mod pinned;\n\n\
         pub fn stamp(utc: bool) -> Result<String, String> {\n    \
         let id = (pinned::random() * 1_000_000.0).floor();\n    \
         Ok(format!(\"{utc}@{}/{}#{id}\", pinned::now_ms(), pinned::now_ms()))\n}\n\n\
         pub fn roll(sides: f64) -> Result<f64, String> {\n    \
         let first = (pinned::random() * sides).floor();\n    \
         Ok(first + (pinned::random() * sides).floor())\n}\n",
    );
    let helper = rust_anything::pin::set(&dir, true).unwrap();
    assert_eq!(helper, Some(dir.join(rust_anything::pin::HELPER)));
    let first = record(&dir, 20);
    assert_eq!(first.pinned, Some(rust_anything::pin::Pin::default()));
    let cases = std::fs::read_to_string(dir.join(harness::CASES)).unwrap();
    assert!(cases.contains("@1700000000000/1700000000000#"), "{cases}");
    record(&dir, 20);
    assert_eq!(
        std::fs::read_to_string(dir.join(harness::CASES)).unwrap(),
        cases,
        "a pinned recording is the same every time"
    );

    harness::build(&dir).unwrap();
    let matched = diff(&dir);
    assert_eq!(matched.failed, 0, "{:?}", matched.mismatches);
    assert!(matched.total > 20);
    let fmt = Command::new("cargo")
        .arg("fmt")
        .current_dir(&dir)
        .status()
        .unwrap();
    assert!(fmt.success());
    let gate = rust_anything::gate::gate(&dir, &[]).unwrap();
    assert!(gate.passed, "{:?}", gate.checks);

    rust_anything::pin::set(&dir, false).unwrap();
    assert!(!dir.join(rust_anything::pin::FILE).exists());
    std::fs::remove_dir_all(&repo).unwrap();
}
