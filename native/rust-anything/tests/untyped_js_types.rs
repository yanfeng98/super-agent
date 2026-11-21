mod common;

use std::path::{Path, PathBuf};

use rust_anything::{bridge, harness};

fn repo(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "rust-anything-untyped-js-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    for (path, text) in files {
        std::fs::write(root.join(path), text).unwrap();
    }
    root
}

fn bridged(root: &Path) -> anyhow::Result<(PathBuf, String)> {
    let out = root.join("port");
    bridge::bridge(&bridge::Options {
        root,
        unit: "m.mjs",
        kind: None,
        out: Some(out.clone()),
        name: Some("m-proof".into()),
        force: false,
        keep: Vec::new(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })?;
    let port = std::fs::read_to_string(out.join("src/port.rs"))?;
    Ok((out, port))
}

#[test]
fn jsdoc_types_untyped_parameters() {
    let root = repo(
        "jsdoc",
        &[(
            "m.mjs",
            "/**\n * @param {string} source\n * @param {string} [delimiter]\n * @returns {string[]}\n */\n\
             export function parse(source, delimiter = \",\") {\n  return source.split(delimiter);\n}\n",
        )],
    );
    let result = (|| -> anyhow::Result<()> {
        let (out, port) = bridged(&root)?;
        assert!(
            port.contains(
                "pub fn parse(source: String, delimiter: Option<String>) -> Result<Vec<String>, String>"
            ),
            "{port}"
        );
        if common::have("node") {
            let recording = harness::record(&harness::RecordOptions {
                crate_dir: &out,
                count: 50,
                inputs: None,
                module: None,
                resolve_from: None,
                node_args: &[],
            })?;
            assert!(recording.cases > 0);
            assert_eq!(recording.errors, 0);
        }
        Ok(())
    })();
    std::fs::remove_dir_all(&root).unwrap();
    result.unwrap();
}

#[test]
fn a_sibling_declaration_file_types_a_js_module() {
    let root = repo(
        "dts",
        &[
            (
                "m.mjs",
                "/** @param {number} text */\n\
                 export function count(text, needle) { return text.split(needle).length - 1; }\n\
                 export const flag = (x) => !!x;\n",
            ),
            (
                "m.d.mts",
                "export declare function count(text: string, needle?: string | null): number;\n\
                 export declare function flag(x: boolean): boolean;\n",
            ),
        ],
    );
    let result = bridged(&root);
    std::fs::remove_dir_all(&root).unwrap();
    let (_, port) = result.unwrap();
    assert!(
        port.contains("pub fn count(text: String, needle: Option<String>) -> Result<f64, String>"),
        "{port}"
    );
    assert!(
        port.contains("pub fn flag(x: bool) -> Result<bool, String>"),
        "{port}"
    );
}
