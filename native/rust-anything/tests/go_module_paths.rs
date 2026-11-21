mod common;

use std::path::Path;

use rust_anything::{bridge, harness};

#[test]
fn record_resolves_versioned_go_modules_in_directories_with_spaces() {
    if !common::have("go") {
        return;
    }

    let temp = std::env::temp_dir().join(format!(
        "rust-anything go module paths {}",
        std::process::id()
    ));
    let result = ["example.com/textkit/v2", "gopkg.in/textkit.v1"]
        .into_iter()
        .enumerate()
        .try_for_each(|(i, module)| record(&temp.join(i.to_string()), module));
    std::fs::remove_dir_all(temp).unwrap();
    result.unwrap();
}

fn record(root: &Path, module: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(root)?;
    std::fs::write(root.join("go.mod"), format!("module {module}\n\ngo 1.21\n"))?;
    std::fs::write(
        root.join("textkit.go"),
        "package textkit\n\nfunc Upper(s string) string { return s }\n",
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
