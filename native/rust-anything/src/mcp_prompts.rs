//! MCP prompts: the skill's playbooks as ready-to-send messages, for agents that load prompts
//! rather than skills.

use serde_json::{Value, json};

const SKILL: &str = include_str!("../skills/rust-anything/SKILL.md");
const WHOLE_APP: &str = include_str!("../skills/rust-anything/whole-app.md");

/// The `prompts/list` result.
pub fn list() -> Value {
    json!({ "prompts": [
        {
            "name": "port_unit",
            "title": "Port one unit to Rust",
            "description": "Port one unit of a repository to Rust: analyze, bridge, record, port, build, diff, shadow, gate.",
            "arguments": [
                { "name": "path", "description": "Repository root.", "required": true },
                { "name": "unit", "description": "Unit id from analyze; the best first port from `start_here` when omitted.", "required": false },
            ],
        },
        {
            "name": "port_whole_app",
            "title": "Port a whole app to Rust",
            "description": "Rewrite a whole app (CLI, server or library) as a pure Rust app: grill the decisions, record the app, front door, port every unit, lift.",
            "arguments": [
                { "name": "path", "description": "Repository root.", "required": true },
            ],
        },
    ] })
}

/// The `prompts/get` result, or the JSON-RPC invalid-params message.
pub fn get(name: &str, arguments: &Value) -> Result<Value, String> {
    let arg = |key: &str| arguments[key].as_str().filter(|s| !s.trim().is_empty());
    let path = arg("path").ok_or_else(|| format!("prompt `{name}` needs the `path` argument"))?;
    let (description, text) = match name {
        "port_unit" => {
            let target = match arg("unit") {
                Some(unit) => format!(
                    "Port the unit `{unit}` of the repository at {path} to Rust with rust-anything."
                ),
                None => format!(
                    "Port one unit of the repository at {path} to Rust with rust-anything: run \
                     analyze on it first and take the best first port from `start_here`."
                ),
            };
            (
                "Port one unit to Rust",
                format!(
                    "{target} Call the port done only when gate passes.\n\nFollow this guide:\n\n{}",
                    body(SKILL)
                ),
            )
        }
        "port_whole_app" => (
            "Port a whole app to Rust",
            format!(
                "Rewrite the app at {path} as a pure Rust app with rust-anything, keeping the \
                 original only as reference.\n\nFollow this guide:\n\n{}\n\nEach unit goes through \
                 the per-unit loop:\n\n{}",
                body(WHOLE_APP),
                body(SKILL)
            ),
        ),
        _ => return Err(format!("unknown prompt `{name}`")),
    };
    Ok(json!({
        "description": description,
        "messages": [{ "role": "user", "content": { "type": "text", "text": text } }],
    }))
}

fn body(markdown: &str) -> &str {
    markdown
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map_or(markdown, |(_, body)| body)
        .trim()
}
