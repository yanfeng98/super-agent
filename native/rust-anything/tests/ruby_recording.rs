mod common;

use rust_anything::{bridge, harness};

#[test]
fn ruby_recording_fails_on_unencodable_results_and_preserves_cases() {
    if !common::have_with(
        "ruby",
        &["-e", "exit(RUBY_VERSION.to_i >= 3 ? 0 : 1)"],
        "3 or newer was not found",
    ) {
        return;
    }

    let root =
        std::env::temp_dir().join(format!("rust-anything-ruby-record-{}", std::process::id()));
    let lib = root.join("lib");
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(root.join("textkit.gemspec"), "").unwrap();
    let source = lib.join("textkit.rb");
    std::fs::write(
        &source,
        "module Textkit\n  def self.value(s)\n    s\n  end\nend\n",
    )
    .unwrap();
    let port = root.join("port");
    bridge::bridge(&bridge::Options {
        root: &root,
        unit: "lib/textkit.rb",
        kind: None,
        out: Some(port.clone()),
        name: None,
        force: false,
        keep: Vec::new(),
        lang: Some(rust_anything::lang::Lang::Ruby),
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();
    let opts = harness::RecordOptions {
        crate_dir: &port,
        count: 2,
        inputs: None,
        module: None,
        resolve_from: None,
        node_args: &[],
    };
    let first = harness::record(&opts).unwrap();
    assert!(first.cases > 0);
    let original = std::fs::read(&first.path).unwrap();
    assert_eq!(first.errors, 0, "{}", String::from_utf8_lossy(&original));
    std::fs::write(
        &source,
        "module Textkit\n  def self.value(s)\n    [255].pack('C').force_encoding('UTF-8')\n  end\nend\n",
    )
    .unwrap();
    let result = harness::record(&opts);
    let retained = std::fs::read(&first.path).unwrap();
    std::fs::remove_dir_all(root).unwrap();

    let err = result.unwrap_err();
    assert!(
        format!("{err:#}").contains("cannot encode recorded result"),
        "{err:#}"
    );
    assert_eq!(retained, original);
}
