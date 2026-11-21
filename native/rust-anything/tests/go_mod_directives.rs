mod common;

use std::path::Path;

use rust_anything::{bridge, harness};

#[test]
fn record_reads_tabbed_go_mod_directives_with_comments() {
    if !common::have("go") {
        return;
    }

    let temp = std::env::temp_dir().join(format!(
        "rust-anything-go-mod-directives {}",
        std::process::id()
    ));
    let result = record(&temp);
    std::fs::remove_dir_all(temp).unwrap();
    result.unwrap();
}

fn record(root: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(root)?;
    std::fs::write(
        root.join("go.mod"),
        "module\t\"example.com/textkit\" // module comment\n\ngo\t1.22 // version comment\n",
    )?;
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
    let manifest = bridge::Manifest::load(&out)?;
    assert_eq!(
        rust_anything::harness_go::import_path(&manifest)?,
        "example.com/textkit"
    );
    let recording = harness::record(&harness::RecordOptions {
        crate_dir: &out,
        count: 2,
        inputs: None,
        module: None,
        resolve_from: None,
        node_args: &[],
    })?;
    assert!(recording.cases > 0);
    let driver_mod = std::fs::read_to_string(out.join(".rust-anything/go-original/go.mod"))?;
    assert!(
        driver_mod.lines().any(|line| line == "go 1.22"),
        "{driver_mod}"
    );
    assert_eq!(recording.errors, 0);
    Ok(())
}
