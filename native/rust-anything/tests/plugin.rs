use std::path::Path;

fn read(path: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}

#[test]
fn the_plugin_ships_the_same_skill_and_mcp_server() {
    for dir in ["plugin", "directory"] {
        for file in ["SKILL.md", "whole-app.md"] {
            assert_eq!(
                read(&format!("{dir}/skills/rust-anything/{file}")),
                read(&format!("skills/rust-anything/{file}")),
                "{dir}/skills/rust-anything/{file} differs from skills/rust-anything/{file}"
            );
        }
    }
    assert_eq!(read("plugin/.mcp.json"), read(".mcp.json"));
}

#[test]
fn the_plugin_version_is_the_crate_version() {
    for dir in ["plugin", "directory"] {
        let plugin: serde_json::Value =
            serde_json::from_str(&read(&format!("{dir}/.claude-plugin/plugin.json"))).unwrap();
        assert_eq!(plugin["version"], env!("CARGO_PKG_VERSION"), "{dir}");
    }
}
