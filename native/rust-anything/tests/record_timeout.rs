//! `record` on an original that never returns for some inputs (#80): the case is stopped after
//! the time limit, the other cases are kept, and the stuck inputs are reported.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "rust-anything-timeout-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for (path, text) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    root
}

/// Bridges and records `unit` with the CLI, with a one-second limit per case. Returns what
/// `record` printed and the recorded cases.
fn record_stuck(root: &Path, unit: &str) -> (String, Vec<serde_json::Value>) {
    let cli = env!("CARGO_BIN_EXE_rust-anything");
    let crate_dir = root.join("port");
    let bridged = Command::new(cli)
        .arg("bridge")
        .arg(root)
        .arg(unit)
        .arg("--out")
        .arg(&crate_dir)
        .output()
        .unwrap();
    assert!(bridged.status.success(), "{bridged:?}");
    let recorded = Command::new(cli)
        .arg("record")
        .arg(&crate_dir)
        .env("RUST_ANYTHING_CASE_TIMEOUT", "1")
        .output()
        .unwrap();
    assert!(recorded.status.success(), "{recorded:?}");
    let cases = std::fs::read_to_string(crate_dir.join("cases.jsonl"))
        .unwrap()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    (
        String::from_utf8_lossy(&recorded.stdout).into_owned(),
        cases,
    )
}

fn check((printed, cases): &(String, Vec<serde_json::Value>)) {
    assert!(printed.contains("Left out "), "{printed}");
    assert!(cases.len() > 5, "{printed}");
    let hangs =
        |v: &serde_json::Value| v.as_f64().is_some_and(|n| n > 1000.0) || v["$num"] == "Infinity";
    assert!(
        cases.iter().all(|c| !hangs(&c["args"][0])),
        "a huge input was recorded: {cases:?}"
    );
}

#[test]
fn python_cases_that_never_return_are_left_out() {
    if !common::have("python3") {
        return;
    }
    let root = project(
        "py",
        &[
            (
                "pyproject.toml",
                "[project]\nname = \"spin\"\nversion = \"0.1.0\"\n",
            ),
            (
                "spin.py",
                "def spin(n: int) -> int:\n    while n > 1000:\n        pass\n    return n * 2\n",
            ),
        ],
    );
    let recorded = record_stuck(&root, "spin.py");
    std::fs::remove_dir_all(&root).unwrap();
    check(&recorded);
}

#[test]
fn node_cases_that_never_return_are_left_out() {
    if !common::have("node") {
        return;
    }
    let root = project(
        "ts",
        &[
            ("package.json", r#"{"name":"spin","version":"0.1.0"}"#),
            (
                "src/spin.ts",
                "export function spin(n: number): number {\n  while (n > 1000) {}\n  return n * 2;\n}\n",
            ),
        ],
    );
    let recorded = record_stuck(&root, "src/spin.ts");
    std::fs::remove_dir_all(&root).unwrap();
    check(&recorded);
}
