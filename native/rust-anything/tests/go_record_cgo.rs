mod common;

use std::{path::Path, process::Command};

use rust_anything::{bridge, harness};

#[test]
fn record_uses_cgo_when_available_for_the_original_module() {
    if !common::have("go") {
        return;
    }
    let Ok(cgo) = Command::new("go").args(["env", "CGO_ENABLED"]).output() else {
        return;
    };
    if !cgo.status.success() || String::from_utf8_lossy(&cgo.stdout).trim() != "1" {
        return;
    }

    let temp = std::env::temp_dir().join(format!("rust-anything-go-cgo-{}", std::process::id()));
    let result = record(&temp);
    std::fs::remove_dir_all(temp).unwrap();
    result.unwrap();
}

fn record(root: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(root)?;
    std::fs::write(
        root.join("go.mod"),
        "module example.com/textkit\n\ngo 1.22\n",
    )?;
    std::fs::write(
        root.join("textkit.go"),
        "package textkit\n\n// static int twice(int x) { return x * 2; }\nimport \"C\"\n\nfunc Twice(x int) int { return int(C.twice(C.int(x))) }\n",
    )?;
    let out = root.join("port");
    bridge::bridge(&bridge::Options {
        root,
        unit: "textkit.go",
        kind: None,
        out: Some(out.clone()),
        name: Some("textkit-rs".into()),
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })?;
    let recording = harness::record(&harness::RecordOptions {
        crate_dir: &out,
        count: 2,
        inputs: None,
        module: None,
        resolve_from: None,
        node_args: &[],
    })?;
    assert!(recording.cases > 0);
    assert_eq!(recording.errors, 0);
    Ok(())
}
