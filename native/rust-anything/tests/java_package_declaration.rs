mod common;

use std::path::Path;

use rust_anything::{bridge, harness};

#[test]
fn records_java_with_comments_in_package_declarations() {
    if !common::have("javac") || !common::have("java") {
        return;
    }

    let temp = std::env::temp_dir().join(format!(
        "rust-anything-java-package-declaration {}",
        std::process::id()
    ));
    let result = record(&temp);
    std::fs::remove_dir_all(temp).unwrap();
    result.unwrap();
}

fn record(root: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(root)?;
    std::fs::write(
        root.join("Sample.java"),
        "package com. /* package comment */ measured; // trailing comment\npublic class Sample { public static int twice(int n) { return n * 2; } }\n",
    )?;
    let out = root.join("port");
    bridge::bridge(&bridge::Options {
        root,
        unit: "Sample.java",
        kind: None,
        out: Some(out.clone()),
        name: Some("sample-rs".into()),
        force: false,
        keep: Vec::new(),
        lang: Some(rust_anything::lang::Lang::Java),
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
