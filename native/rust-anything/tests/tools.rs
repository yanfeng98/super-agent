//! The MCP tools and the CLI are two doors to the same commands: every tool parameter has a CLI
//! flag or argument and every CLI option has a tool parameter, and `docs/tools.md` is generated
//! from the tool list (`UPDATE_TOOLS_DOC=1 cargo test --test tools` rewrites it).

use std::collections::BTreeSet;
use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::Value;

const CLI: &str = env!("CARGO_BIN_EXE_rust-anything");

/// Options only the terminal needs: how output is printed, and `analyze`'s output file.
fn cli_only(tool: &str, option: &str) -> bool {
    option == "json"
        || (tool == "analyze" && option == "out")
        || (tool == "map" && matches!(option, "write" | "check"))
}

fn tools() -> Vec<Value> {
    let mut child = Command::new(CLI)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
"#,
        )
        .unwrap();
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let list = text
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|m| m["id"] == 2)
        .expect("tools/list answered");
    list["result"]["tools"].as_array().unwrap().clone()
}

fn subcommand(tool: &str) -> Vec<String> {
    match tool.strip_prefix("app_") {
        Some(rest) => vec!["app".into(), rest.replace('_', "-")],
        None => vec![tool.into()],
    }
}

fn canonical(name: &str) -> String {
    let name = name.to_lowercase().replace('-', "_");
    let name = name.strip_prefix("no_").unwrap_or(&name).to_string();
    match name.as_str() {
        "crate_dir" => "crate".into(),
        "node_arg" | "node_args" => "node_args".into(),
        "from_tests" => "test_command".into(),
        "command" | "commands" => "commands".into(),
        "route" | "routes" => "routes".into(),
        "arg" | "args" => "args".into(),
        _ => name,
    }
}

/// The CLI's flags and arguments for a subcommand, read from its `--help`.
fn cli_options(tool: &str, sub: &[String]) -> BTreeSet<String> {
    let out = Command::new(CLI).args(sub).arg("--help").output().unwrap();
    let help = String::from_utf8(out.stdout).unwrap();
    let flags = regex::Regex::new(r"(?m)^\s+(?:-[a-zA-Z], )?--([a-z][a-z-]*)").unwrap();
    let args = regex::Regex::new(r"[<\[]([A-Z][A-Z_]*)[>\].]").unwrap();
    let usage = help.lines().find(|l| l.starts_with("Usage:")).unwrap_or("");
    flags
        .captures_iter(&help)
        .map(|c| c[1].to_string())
        .filter(|f| f != "help" && (f != "version" || help.contains("--version <")))
        .chain(
            args.captures_iter(usage)
                .map(|c| c[1].to_string())
                .filter(|a| a != "OPTIONS"),
        )
        .map(|n| canonical(&n))
        .filter(|n| !cli_only(tool, n))
        .collect()
}

#[test]
fn every_tool_parameter_has_a_cli_option_and_back() {
    let mut problems = Vec::new();
    for tool in tools() {
        let name = tool["name"].as_str().unwrap();
        let sub = subcommand(name);
        let cli = cli_options(name, &sub);
        let mcp: BTreeSet<String> = tool["inputSchema"]["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(|k| canonical(k))
            .collect();
        for missing in mcp.difference(&cli) {
            problems.push(format!(
                "{name}: `{missing}` has no CLI option in `rust-anything {}`",
                sub.join(" ")
            ));
        }
        for missing in cli.difference(&mcp) {
            problems.push(format!(
                "{name}: CLI option `{missing}` is not a tool parameter"
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

fn doc(tools: &[Value]) -> String {
    let mut out = String::from(
        "# Tools\n\nGenerated from the MCP server's tool list by `tests/tools.rs`; do not edit by hand \
         (`UPDATE_TOOLS_DOC=1 cargo test --test tools` rewrites it). Every tool is also a CLI \
         command with the same options.\n",
    );
    for tool in tools {
        let name = tool["name"].as_str().unwrap_or_default();
        let title = tool["title"].as_str().unwrap_or(name);
        out.push_str(&format!(
            "\n## {title}\n\nTool `{name}` · CLI `rust-anything {}`\n\n{}\n",
            subcommand(name).join(" "),
            tool["description"].as_str().unwrap_or_default()
        ));
        let hints = &tool["annotations"];
        let mut marks = Vec::new();
        if hints["readOnlyHint"] == true {
            marks.push("Read-only");
        }
        if hints["destructiveHint"] == true {
            marks.push(if marks.is_empty() {
                "May overwrite files"
            } else {
                "may overwrite files"
            });
        }
        if hints["openWorldHint"] == true {
            marks.push(if marks.is_empty() {
                "Runs the original's code or commands"
            } else {
                "runs the original's code or commands"
            });
        }
        if !marks.is_empty() {
            out.push_str(&format!("\n{}.\n", marks.join(", ")));
        }
        let required: BTreeSet<&str> = tool["inputSchema"]["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let props = tool["inputSchema"]["properties"].as_object();
        if props.is_some_and(|p| !p.is_empty()) {
            out.push_str(
                "\n| Parameter | Type | Required | Description |\n| --- | --- | --- | --- |\n",
            );
            for (key, p) in props.into_iter().flatten() {
                let ty = match &p["type"] {
                    Value::String(t) if t == "array" => {
                        format!("{}[]", p["items"]["type"].as_str().unwrap_or("value"))
                    }
                    Value::String(t) => t.clone(),
                    _ => "value".into(),
                };
                out.push_str(&format!(
                    "| `{key}` | {ty} | {} | {} |\n",
                    if required.contains(key.as_str()) {
                        "yes"
                    } else {
                        ""
                    },
                    p["description"]
                        .as_str()
                        .unwrap_or_default()
                        .replace('|', "\\|")
                ));
            }
        }
    }
    out
}

#[test]
fn docs_tools_md_matches_the_tool_list() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/tools.md");
    let want = doc(&tools());
    if std::env::var_os("UPDATE_TOOLS_DOC").is_some() {
        std::fs::write(&path, &want).unwrap();
        return;
    }
    let have = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        have == want,
        "docs/tools.md is out of date: run `UPDATE_TOOLS_DOC=1 cargo test --test tools`"
    );
}
