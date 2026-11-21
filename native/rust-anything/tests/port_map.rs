use std::path::Path;

use rust_anything::bridge::{self, Manifest};
use rust_anything::{port_map, recording};

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), &dest).unwrap();
        }
    }
}

#[test]
fn map_lists_ports_and_marks_upstream_changes() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini");
    let repo = std::env::temp_dir().join(format!("rust-anything-map-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo);
    copy_dir(&fixture, &repo);
    let unit = "packages/a/src/util.ts";
    let crate_dir = repo.join("rust/util-rs");
    bridge::bridge(&bridge::Options {
        root: &repo,
        unit,
        kind: None,
        out: Some(crate_dir.clone()),
        name: None,
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();

    let map = port_map::map(&repo, &[]).unwrap();
    assert_eq!(map.ports.len(), 1, "{map:#?}");
    let port = &map.ports[0];
    assert_eq!(port.unit, unit);
    assert_eq!(port.crate_dir, "rust/util-rs");
    assert!(!port.in_rust.is_empty());
    assert_eq!(port.cases, 0);
    assert!(port.last_diff.is_none());
    assert!(port.changed_upstream.is_none());
    assert!(map.analyze_error.is_none(), "{:?}", map.analyze_error);
    assert!(map.not_ported.iter().all(|u| u.unit != unit));
    assert_eq!(map.changed(), 0);

    let m = Manifest::load(&crate_dir).unwrap();
    recording::save_fingerprint(&crate_dir, &m, None).unwrap();
    let source = repo.join(unit);
    let text = std::fs::read_to_string(&source).unwrap();
    std::fs::write(&source, format!("{text}\nexport const added = 1;\n")).unwrap();

    let map = port_map::map(&repo, &[]).unwrap();
    assert_eq!(map.changed(), 1);
    let md = port_map::markdown(&map);
    assert!(md.contains("**1 changed upstream**"), "{md}");
    assert!(md.contains("`rust-anything record rust/util-rs`"), "{md}");

    let outside =
        std::env::temp_dir().join(format!("rust-anything-map-out-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&outside);
    bridge::bridge(&bridge::Options {
        root: &repo,
        unit,
        kind: None,
        out: Some(outside.clone()),
        name: Some("util-outside".into()),
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();
    assert_eq!(port_map::map(&repo, &[]).unwrap().ports.len(), 1);
    assert_eq!(
        port_map::map(&repo, std::slice::from_ref(&outside))
            .unwrap()
            .ports
            .len(),
        2
    );

    std::fs::remove_dir_all(&repo).unwrap();
    std::fs::remove_dir_all(&outside).unwrap();
}
