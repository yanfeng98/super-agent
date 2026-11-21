mod common;

use rust_anything::{bridge, harness};

#[test]
fn records_go_internal_packages() {
    if !common::have("go") {
        return;
    }

    let root =
        std::env::temp_dir().join(format!("rust-anything-go-internal-{}", std::process::id()));
    let unit = root.join("internal/textkit");
    std::fs::create_dir_all(&unit).unwrap();
    std::fs::write(root.join("go.mod"), "module example.com/app\n\ngo 1.21\n").unwrap();
    std::fs::write(
        unit.join("textkit.go"),
        "package textkit\n\nfunc Upper(s string) string { return s }\n",
    )
    .unwrap();
    let port = root.join("port");
    bridge::bridge(&bridge::Options {
        root: &root,
        unit: "internal/textkit/",
        kind: None,
        out: Some(port.clone()),
        name: None,
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();
    let result = harness::record(&harness::RecordOptions {
        crate_dir: &port,
        count: 2,
        inputs: None,
        module: None,
        resolve_from: None,
        node_args: &[],
    });
    std::fs::remove_dir_all(root).unwrap();

    let recording = result.unwrap();
    assert!(recording.cases > 0);
    assert_eq!(recording.errors, 0);
}
