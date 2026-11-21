mod common;

use std::{path::Path, process::Command};

use rust_anything::{
    codegen_java::{DRIVER_JAVA, JSON_JAVA},
    harness::DiffOutcome,
};

#[test]
fn java_replay_distinguishes_null_from_a_rejected_argument() {
    if !common::have("java") || !common::have("javac") {
        return;
    }

    let root = std::env::temp_dir().join(format!("rust-anything-java-null-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let result = replay(&root);
    std::fs::remove_dir_all(root).unwrap();

    let diff = result.unwrap();
    assert_eq!((diff.passed, diff.failed), (1, 1));
    assert_eq!(diff.mismatches[0]["actual"]["invalid"], true);
}

fn replay(root: &Path) -> anyhow::Result<DiffOutcome> {
    std::fs::write(root.join("RustAnythingDriver.java"), DRIVER_JAVA)?;
    std::fs::write(root.join("RustAnythingJson.java"), JSON_JAVA)?;
    std::fs::write(
        root.join("Port.java"),
        "public class Port { public static String value(int input) { return null; } }",
    )?;
    std::fs::write(root.join("targets.json"), r#"{"value":["Port","value"]}"#)?;
    std::fs::write(
        root.join("cases.jsonl"),
        r#"{"fn":"value","args":[1],"ok":null}
{"fn":"value","args":["previously accepted"],"ok":null}
"#,
    )?;

    run(Command::new("javac").current_dir(root).args([
        "-d",
        ".",
        "RustAnythingDriver.java",
        "RustAnythingJson.java",
        "Port.java",
    ]))?;
    run(Command::new("java").current_dir(root).args([
        "-cp",
        ".",
        "rustanything.RustAnythingDriver",
        "diff",
        "targets.json",
        "cases.jsonl",
        "diff.json",
    ]))?;
    Ok(serde_json::from_slice(&std::fs::read(
        root.join("diff.json"),
    )?)?)
}

fn run(cmd: &mut Command) -> anyhow::Result<()> {
    let output = cmd.output()?;
    anyhow::ensure!(
        output.status.success(),
        "{cmd:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
