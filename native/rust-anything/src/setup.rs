//! `setup`: register rust-anything with the coding agents on this machine, in one command.
//!
//! For each agent it finds (or is told about), it adds the MCP server (`rust-anything mcp`) to
//! the agent's own configuration and, where the agent reads skills, installs the workflow skill.
//! Existing settings are kept: only the `rust-anything` entries are added or replaced, and a file
//! that does not parse is left alone with an error. `--remove` takes the entries out again.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::{Map, Value, json};

/// The server and skill name in every agent.
pub const NAME: &str = "rust-anything";

/// The workflow skill, as shipped in `skills/rust-anything/SKILL.md`.
pub const SKILL: &str = include_str!("../skills/rust-anything/SKILL.md");
pub(crate) const SKILL_FILES: &[(&str, &str)] = &[
    ("SKILL.md", SKILL),
    (
        "whole-app.md",
        include_str!("../skills/rust-anything/whole-app.md"),
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Agent {
    /// Claude Code: `~/.claude.json`, skill in `~/.claude/skills`.
    Claude,
    /// OpenAI Codex CLI: `~/.codex/config.toml`, skill in `~/.codex/skills`.
    Codex,
    /// Cursor: `~/.cursor/mcp.json`.
    Cursor,
    /// Gemini CLI: `~/.gemini/settings.json`.
    Gemini,
    /// OpenCode: `~/.config/opencode/opencode.json`, skill in `~/.config/opencode/skills`.
    #[value(name = "opencode")]
    OpenCode,
}

impl Agent {
    pub const ALL: [Agent; 5] = [
        Agent::Claude,
        Agent::Codex,
        Agent::Cursor,
        Agent::Gemini,
        Agent::OpenCode,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Agent::Claude => "Claude Code",
            Agent::Codex => "Codex",
            Agent::Cursor => "Cursor",
            Agent::Gemini => "Gemini CLI",
            Agent::OpenCode => "OpenCode",
        }
    }

    /// The agent's folder in the home directory, and its command.
    fn markers(self) -> (&'static str, &'static str) {
        match self {
            Agent::Claude => (".claude", "claude"),
            Agent::Codex => (".codex", "codex"),
            Agent::Cursor => (".cursor", "cursor"),
            Agent::Gemini => (".gemini", "gemini"),
            Agent::OpenCode => (".config/opencode", "opencode"),
        }
    }

    /// The settings file that holds the agent's MCP servers.
    pub(crate) fn config(self, home: &Path) -> PathBuf {
        home.join(match self {
            Agent::Claude => ".claude.json",
            Agent::Codex => ".codex/config.toml",
            Agent::Cursor => ".cursor/mcp.json",
            Agent::Gemini => ".gemini/settings.json",
            Agent::OpenCode => ".config/opencode/opencode.json",
        })
    }

    /// Where the workflow skill goes, for agents that read skills.
    pub(crate) fn skill_dir(self, home: &Path) -> Option<PathBuf> {
        match self {
            Agent::Claude => Some(home.join(".claude/skills").join(NAME)),
            Agent::Codex => Some(home.join(".codex/skills").join(NAME)),
            Agent::OpenCode => Some(home.join(".config/opencode/skills").join(NAME)),
            Agent::Cursor | Agent::Gemini => None,
        }
    }

    /// Whether the agent seems installed: its folder exists or its command is on `PATH`.
    pub(crate) fn present(self, home: &Path) -> bool {
        let (dir, command) = self.markers();
        home.join(dir).is_dir() || on_path(command)
    }
}

pub(crate) fn on_path(command: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| {
            let file = dir.join(command);
            file.is_file()
                || file.with_extension("exe").is_file()
                || file.with_extension("cmd").is_file()
        })
    })
}

pub struct Options<'a> {
    pub home: &'a Path,
    /// Agents to set up; empty means every agent found.
    pub agents: Vec<Agent>,
    /// The command the agents run; the current executable by default.
    pub command: PathBuf,
    pub remove: bool,
    pub dry_run: bool,
}

/// One change: which agent, which file, and what happened.
#[derive(Debug, Serialize)]
pub struct Change {
    pub agent: Agent,
    pub file: PathBuf,
    /// `added`, `updated`, `unchanged`, `removed` or `absent`.
    pub action: &'static str,
    /// The copy of the file as it was, when an existing file was changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub command: PathBuf,
    pub agents: Vec<Agent>,
    pub changes: Vec<Change>,
    pub dry_run: bool,
}

/// The home directory: `HOME`, or `USERPROFILE` on Windows.
pub fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .context("no home directory (HOME is not set)")
}

pub fn setup(opts: &Options) -> Result<Report> {
    let agents: Vec<Agent> = if opts.agents.is_empty() {
        Agent::ALL
            .into_iter()
            .filter(|a| a.present(opts.home))
            .collect()
    } else {
        opts.agents.clone()
    };
    if agents.is_empty() {
        bail!(
            "no coding agent found (looked for ~/.claude, ~/.codex, ~/.cursor, ~/.gemini, ~/.config/opencode and their commands); \
             name one with --agent claude|codex|cursor|gemini|opencode"
        );
    }
    let command = opts.command.to_string_lossy().into_owned();
    let mut changes = Vec::new();
    for &agent in &agents {
        let config = agent.config(opts.home);
        changes.push(match agent {
            Agent::Codex => codex_server(agent, &config, &command, opts)?,
            Agent::OpenCode => opencode_server(agent, &config, &command, opts)?,
            _ => json_server(agent, &config, &command, opts, agent == Agent::Claude)?,
        });
        if let Some(dir) = agent.skill_dir(opts.home) {
            changes.push(skill(agent, &dir, opts)?);
        }
    }
    Ok(Report {
        command: opts.command.clone(),
        agents,
        changes,
        dry_run: opts.dry_run,
    })
}

/// Copies an existing settings file to `<file>.rust-anything.bak` before it is changed.
fn backup(path: &Path) -> Result<Option<PathBuf>> {
    if !path.is_file() {
        return Ok(None);
    }
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".rust-anything.bak");
    let copy = path.with_file_name(name);
    std::fs::copy(path, &copy).with_context(|| format!("backing up {}", path.display()))?;
    Ok(Some(copy))
}

/// Writes `text` to `path` through a temporary file, so a failed write never leaves half a file.
/// A symlinked file (a dotfiles repo) is written at its target, and an existing file keeps its
/// permissions (a settings file holding tokens stays private).
fn write(path: &Path, text: &str) -> Result<()> {
    let path = &std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let tmp = path.with_extension("rust-anything.tmp");
    let mut file =
        std::fs::File::create(&tmp).with_context(|| format!("writing {}", tmp.display()))?;
    if let Ok(meta) = std::fs::metadata(path) {
        file.set_permissions(meta.permissions())
            .with_context(|| format!("writing {}", tmp.display()))?;
    }
    std::io::Write::write_all(&mut file, text.as_bytes())
        .with_context(|| format!("writing {}", tmp.display()))?;
    drop(file);
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
}

/// The `mcpServers` entry of a JSON settings file (Claude Code, Cursor, Gemini CLI).
fn json_server(
    agent: Agent,
    path: &Path,
    command: &str,
    opts: &Options,
    stdio_type: bool,
) -> Result<Change> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let mut doc: Value = match text.as_deref().map(str::trim) {
        None | Some("") => json!({}),
        Some(t) => serde_json::from_str(t).with_context(|| {
            format!(
                "{} is not valid JSON; fix it or add the server by hand",
                path.display()
            )
        })?,
    };
    let Some(root) = doc.as_object_mut() else {
        bail!(
            "{} is not a JSON object; add the server by hand",
            path.display()
        );
    };
    let servers = root.entry("mcpServers").or_insert_with(|| json!({}));
    let Some(servers) = servers.as_object_mut() else {
        bail!(
            "`mcpServers` in {} is not an object; add the server by hand",
            path.display()
        );
    };
    let action = if opts.remove {
        if servers.shift_remove(NAME).is_some() {
            "removed"
        } else {
            "absent"
        }
    } else {
        let existing = servers.get(NAME);
        let mut entry = existing
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        entry.insert("command".into(), json!(command));
        entry.insert("args".into(), json!(["mcp"]));
        if stdio_type || entry.contains_key("type") {
            entry.insert("type".into(), json!("stdio"));
        }
        if stdio_type {
            entry.entry("env").or_insert_with(|| json!({}));
        }
        entry.shift_remove("url");
        let entry = Value::Object(entry);
        if existing == Some(&entry) {
            "unchanged"
        } else {
            let action = if existing.is_some() {
                "updated"
            } else {
                "added"
            };
            servers.insert(NAME.into(), entry);
            action
        }
    };
    let mut saved = None;
    if !opts.dry_run && matches!(action, "added" | "updated" | "removed") {
        saved = backup(path)?;
        write(path, &(serde_json::to_string_pretty(&doc)? + "\n"))?;
    }
    Ok(Change {
        agent,
        file: path.to_path_buf(),
        action,
        backup: saved,
    })
}

/// The map of servers in OpenCode's `mcp` object. Current OpenCode keeps them under
/// `mcp.servers`; a `mcp` object that already lists servers by name (the shape older files
/// use) is kept as it is, so the two shapes never end up mixed in one file.
fn opencode_servers<'a>(
    mcp: &'a mut Map<String, Value>,
    path: &Path,
) -> Result<&'a mut Map<String, Value>> {
    if mcp.is_empty() || matches!(mcp.get("servers"), Some(Value::Object(_))) {
        let servers = mcp.entry("servers").or_insert_with(|| json!({}));
        return servers.as_object_mut().with_context(|| {
            format!(
                "`mcp.servers` in {} is not an object; add the server by hand",
                path.display()
            )
        });
    }
    Ok(mcp)
}

/// `mcp.servers.rust-anything` in OpenCode's `opencode.json`: a local server whose `command`
/// is the whole argv, so the executable and `mcp` go in one array.
fn opencode_server(agent: Agent, path: &Path, command: &str, opts: &Options) -> Result<Change> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let mut doc: Value = match text.as_deref().map(str::trim) {
        None | Some("") => json!({}),
        Some(t) => serde_json::from_str(t).with_context(|| {
            format!(
                "{} is not valid JSON; fix it or add the server by hand",
                path.display()
            )
        })?,
    };
    let Some(root) = doc.as_object_mut() else {
        bail!(
            "{} is not a JSON object; add the server by hand",
            path.display()
        );
    };
    let mcp = root.entry("mcp").or_insert_with(|| json!({}));
    let Some(mcp) = mcp.as_object_mut() else {
        bail!(
            "`mcp` in {} is not an object; add the server by hand",
            path.display()
        );
    };
    let servers = opencode_servers(mcp, path)?;
    let action = if opts.remove {
        if servers.shift_remove(NAME).is_some() {
            "removed"
        } else {
            "absent"
        }
    } else {
        let existing = servers.get(NAME);
        let mut entry = existing
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        entry.insert("type".into(), json!("local"));
        entry.insert("command".into(), json!([command, "mcp"]));
        // OpenCode keeps a server out of the way with `disabled`, and older files wrote the
        // opposite `enabled`: setting one up means neither key.
        entry.shift_remove("disabled");
        entry.shift_remove("enabled");
        entry.shift_remove("url");
        let entry = Value::Object(entry);
        if existing == Some(&entry) {
            "unchanged"
        } else {
            let action = if existing.is_some() {
                "updated"
            } else {
                "added"
            };
            servers.insert(NAME.into(), entry);
            action
        }
    };
    let mut saved = None;
    if !opts.dry_run && matches!(action, "added" | "updated" | "removed") {
        saved = backup(path)?;
        write(path, &(serde_json::to_string_pretty(&doc)? + "\n"))?;
    }
    Ok(Change {
        agent,
        file: path.to_path_buf(),
        action,
        backup: saved,
    })
}

/// `[mcp_servers.rust-anything]` in Codex's `config.toml`, keeping the rest of the file as is.
fn codex_server(agent: Agent, path: &Path, command: &str, opts: &Options) -> Result<Change> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let mut doc: toml_edit::DocumentMut = text.parse().with_context(|| {
        format!(
            "{} is not valid TOML; fix it or add the server by hand",
            path.display()
        )
    })?;
    let servers = doc
        .entry("mcp_servers")
        .or_insert_with(|| {
            let mut t = toml_edit::Table::new();
            t.set_implicit(true);
            toml_edit::Item::Table(t)
        })
        .as_table_mut()
        .with_context(|| format!("`mcp_servers` in {} is not a table", path.display()))?;
    let same = servers
        .get(NAME)
        .and_then(|t| t.as_table_like())
        .is_some_and(|t| {
            t.get("command").and_then(|c| c.as_str()) == Some(command)
                && t.get("url").is_none()
                && t.get("args").and_then(|a| a.as_array()).is_some_and(|a| {
                    a.iter().map(|v| v.as_str()).collect::<Vec<_>>() == [Some("mcp")]
                })
        });
    let action = if opts.remove {
        if servers.remove(NAME).is_some() {
            "removed"
        } else {
            "absent"
        }
    } else if same {
        "unchanged"
    } else {
        let had = servers.contains_key(NAME);
        // An entry written inline (`rust-anything = { ... }`) keeps its other keys too.
        let mut entry = match servers.get(NAME) {
            Some(toml_edit::Item::Table(t)) => t.clone(),
            Some(toml_edit::Item::Value(toml_edit::Value::InlineTable(t))) => {
                t.clone().into_table()
            }
            _ => toml_edit::Table::new(),
        };
        entry["command"] = toml_edit::value(command);
        let mut args = toml_edit::Array::new();
        args.push("mcp");
        entry["args"] = toml_edit::value(args);
        entry.remove("url");
        servers.insert(NAME, toml_edit::Item::Table(entry));
        if had { "updated" } else { "added" }
    };
    let mut saved = None;
    if !opts.dry_run && matches!(action, "added" | "updated" | "removed") {
        saved = backup(path)?;
        write(path, &doc.to_string())?;
    }
    Ok(Change {
        agent,
        file: path.to_path_buf(),
        action,
        backup: saved,
    })
}

/// The workflow skill in an agent's skills folder.
fn skill(agent: Agent, dir: &Path, opts: &Options) -> Result<Change> {
    let current: Vec<Option<String>> = SKILL_FILES
        .iter()
        .map(|(name, _)| std::fs::read_to_string(dir.join(name)).ok())
        .collect();
    let action = if opts.remove {
        if current.iter().any(Option::is_some) {
            "removed"
        } else {
            "absent"
        }
    } else if current.iter().all(Option::is_none) {
        "added"
    } else if current
        .iter()
        .zip(SKILL_FILES)
        .all(|(have, (_, want))| have.as_deref() == Some(*want))
    {
        "unchanged"
    } else {
        "updated"
    };
    if !opts.dry_run {
        for (name, text) in SKILL_FILES {
            let path = dir.join(name);
            match action {
                "added" | "updated" => write(&path, text)?,
                "removed" if path.exists() => {
                    std::fs::remove_file(&path)
                        .with_context(|| format!("removing {}", path.display()))?;
                }
                _ => {}
            }
        }
        if action == "removed" {
            // The folder is ours; leave it only if someone put other files in it.
            let _ = std::fs::remove_dir(dir);
        }
    }
    Ok(Change {
        agent,
        file: dir.join("SKILL.md"),
        action,
        backup: None,
    })
}

/// The report for people.
pub fn text(r: &Report) -> String {
    let mut out = String::new();
    let verb = if r.dry_run { "Would set up" } else { "Set up" };
    let names: Vec<&str> = r.agents.iter().map(|a| a.name()).collect();
    out.push_str(&format!("{verb} rust-anything for {}\n", names.join(", ")));
    for c in &r.changes {
        out.push_str(&format!(
            "  {:<10} {:<9} {}\n",
            c.agent.name(),
            c.action,
            c.file.display()
        ));
        if let Some(b) = &c.backup {
            out.push_str(&format!("  {:<10} {:<9} {}\n", "", "backup", b.display()));
        }
    }
    if !r.dry_run
        && r.changes
            .iter()
            .any(|c| c.action != "removed" && c.action != "absent")
    {
        out.push_str(&format!(
            "\nThe agents run `{} mcp`. Restart them, then ask, for example:\n  \
             \"Profile this repo's slowest workflow with rust-anything and move the hot loop to Rust.\"\n",
            r.command.display()
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rust-anything-setup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn opts(home: &Path, agents: Vec<Agent>) -> Options<'_> {
        Options {
            home,
            agents,
            command: PathBuf::from("/opt/bin/rust-anything"),
            remove: false,
            dry_run: false,
        }
    }

    #[test]
    fn adds_keeps_and_removes_entries() {
        let home = temp_home("all");
        // Existing settings that must survive, in their own order.
        std::fs::write(
            home.join(".claude.json"),
            r#"{"zeta": 1, "mcpServers": {"other": {"command": "x"}}, "alpha": true}"#,
        )
        .unwrap();
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        std::fs::write(
            home.join(".codex/config.toml"),
            "model = \"o4\"\n\n[mcp_servers.other]\ncommand = \"x\"\n",
        )
        .unwrap();

        let all = Agent::ALL.to_vec();
        let r = setup(&opts(&home, all.clone())).unwrap();
        assert!(
            r.changes.iter().all(|c| c.action == "added"),
            "{:?}",
            r.changes
        );

        let claude: Value =
            serde_json::from_str(&std::fs::read_to_string(home.join(".claude.json")).unwrap())
                .unwrap();
        assert_eq!(
            claude["mcpServers"]["rust-anything"]["args"],
            json!(["mcp"])
        );
        assert_eq!(claude["mcpServers"]["other"]["command"], "x");
        // The existing file was copied before the change; new files need no copy.
        let saved = r.changes[0]
            .backup
            .clone()
            .expect("an existing file is backed up");
        assert!(
            std::fs::read_to_string(saved)
                .unwrap()
                .starts_with(r#"{"zeta": 1"#)
        );
        assert!(r.changes.iter().any(|c| c.backup.is_none()));
        let keys: Vec<&String> = claude.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["zeta", "mcpServers", "alpha"], "key order kept");
        let codex = std::fs::read_to_string(home.join(".codex/config.toml")).unwrap();
        assert!(codex.starts_with("model = \"o4\""), "{codex}");
        assert!(codex.contains("[mcp_servers.rust-anything]\ncommand = \"/opt/bin/rust-anything\"\nargs = [\"mcp\"]"), "{codex}");
        assert!(codex.contains("[mcp_servers.other]"));
        assert_eq!(
            std::fs::read_to_string(home.join(".claude/skills/rust-anything/SKILL.md")).unwrap(),
            SKILL
        );
        assert!(
            home.join(".codex/skills/rust-anything/whole-app.md")
                .exists()
        );
        let cursor: Value =
            serde_json::from_str(&std::fs::read_to_string(home.join(".cursor/mcp.json")).unwrap())
                .unwrap();
        assert_eq!(
            cursor["mcpServers"]["rust-anything"]["command"],
            "/opt/bin/rust-anything"
        );
        assert!(home.join(".gemini/settings.json").exists());

        // Running again changes nothing.
        let again = setup(&opts(&home, all.clone())).unwrap();
        assert!(
            again.changes.iter().all(|c| c.action == "unchanged"),
            "{:?}",
            again.changes
        );

        // A moved binary updates the entries.
        let mut moved = opts(&home, vec![Agent::Codex]);
        moved.command = PathBuf::from("/usr/local/bin/rust-anything");
        let r = setup(&moved).unwrap();
        assert_eq!(r.changes[0].action, "updated");

        // Remove takes out only our entries.
        let mut remove = opts(&home, all);
        remove.remove = true;
        setup(&remove).unwrap();
        let claude: Value =
            serde_json::from_str(&std::fs::read_to_string(home.join(".claude.json")).unwrap())
                .unwrap();
        assert!(claude["mcpServers"].get("rust-anything").is_none());
        assert_eq!(claude["mcpServers"]["other"]["command"], "x");
        assert!(!home.join(".claude/skills/rust-anything").exists());
        let codex = std::fs::read_to_string(home.join(".codex/config.toml")).unwrap();
        assert!(
            !codex.contains("rust-anything") && codex.contains("[mcp_servers.other]"),
            "{codex}"
        );
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn reinstall_preserves_custom_options_for_the_same_server() {
        let home = temp_home("custom-entry");
        let cursor = home.join(".cursor/mcp.json");
        let codex = home.join(".codex/config.toml");
        std::fs::create_dir_all(cursor.parent().unwrap()).unwrap();
        std::fs::create_dir_all(codex.parent().unwrap()).unwrap();
        std::fs::write(&cursor, r#"{"mcpServers":{"rust-anything":{"command":"old","args":["mcp"],"env":{"MODE":"custom"}}}}"#).unwrap();
        std::fs::write(&codex, "[mcp_servers.rust-anything]\ncommand = \"old\"\nargs = [\"mcp\"]\ntool_timeout_sec = 120\n").unwrap();

        let options = opts(&home, vec![Agent::Cursor, Agent::Codex]);
        setup(&options).unwrap();
        let cursor: Value =
            serde_json::from_str(&std::fs::read_to_string(cursor).unwrap()).unwrap();
        let codex: toml_edit::DocumentMut =
            std::fs::read_to_string(codex).unwrap().parse().unwrap();
        let again = setup(&options).unwrap();
        std::fs::remove_dir_all(home).unwrap();
        assert_eq!(cursor["mcpServers"][NAME]["env"]["MODE"], "custom");
        assert_eq!(
            cursor["mcpServers"][NAME]["command"],
            "/opt/bin/rust-anything"
        );
        assert_eq!(
            codex["mcp_servers"][NAME]["tool_timeout_sec"].as_integer(),
            Some(120)
        );
        assert!(again.changes.iter().all(|c| c.action == "unchanged"));
    }

    #[test]
    fn reinstall_keeps_an_inline_codex_entry() {
        let home = temp_home("codex-inline");
        let codex = home.join(".codex/config.toml");
        std::fs::create_dir_all(codex.parent().unwrap()).unwrap();
        std::fs::write(
            &codex,
            "[mcp_servers]\nrust-anything = { command = \"old\", args = [\"mcp\"], env = { MODE = \"custom\" } }\n",
        )
        .unwrap();

        let options = opts(&home, vec![Agent::Codex]);
        setup(&options).unwrap();
        let doc: toml_edit::DocumentMut = std::fs::read_to_string(&codex).unwrap().parse().unwrap();
        let again = setup(&options).unwrap();
        std::fs::remove_dir_all(home).unwrap();
        assert_eq!(
            doc["mcp_servers"][NAME]["env"]["MODE"].as_str(),
            Some("custom")
        );
        assert_eq!(
            doc["mcp_servers"][NAME]["command"].as_str(),
            Some("/opt/bin/rust-anything")
        );
        assert!(again.changes.iter().all(|c| c.action == "unchanged"));
    }

    #[test]
    fn opencode_gets_a_local_server_and_the_skill() {
        let home = temp_home("opencode");
        let config = home.join(".config/opencode/opencode.json");
        std::fs::create_dir_all(config.parent().unwrap()).unwrap();
        // Other settings, another server and key order that must all survive.
        std::fs::write(
            &config,
            r#"{"$schema":"https://opencode.ai/config.json","mcp":{"servers":{"other":{"type":"local","command":["x"]}}},"model":"a/b"}"#,
        )
        .unwrap();

        let mut options = opts(&home, vec![Agent::OpenCode]);
        setup(&options).unwrap();
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
        let entry = &doc["mcp"]["servers"][NAME];
        assert_eq!(entry["type"], "local");
        assert_eq!(entry["command"], json!(["/opt/bin/rust-anything", "mcp"]));
        assert!(entry.get("disabled").is_none(), "{entry}");
        assert_eq!(doc["mcp"]["servers"]["other"]["command"], json!(["x"]));
        assert_eq!(doc["model"], "a/b");
        let keys: Vec<&String> = doc.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["$schema", "mcp", "model"], "key order kept");
        assert_eq!(
            std::fs::read_to_string(home.join(".config/opencode/skills/rust-anything/SKILL.md"))
                .unwrap(),
            SKILL
        );

        // Running again changes nothing.
        let again = setup(&options).unwrap();
        assert!(
            again.changes.iter().all(|c| c.action == "unchanged"),
            "{:?}",
            again.changes
        );

        // A disabled server is turned back on.
        let mut doc: Value =
            serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
        doc["mcp"]["servers"][NAME]["disabled"] = json!(true);
        std::fs::write(&config, serde_json::to_string(&doc).unwrap()).unwrap();
        let r = setup(&options).unwrap();
        assert_eq!(r.changes[0].action, "updated");
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
        assert!(doc["mcp"]["servers"][NAME].get("disabled").is_none());

        // A moved binary updates the entry.
        options.command = PathBuf::from("/usr/local/bin/rust-anything");
        let r = setup(&options).unwrap();
        assert_eq!(r.changes[0].action, "updated");

        // Remove takes out only ours.
        options.remove = true;
        setup(&options).unwrap();
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
        assert!(doc["mcp"]["servers"].get(NAME).is_none());
        assert_eq!(doc["mcp"]["servers"]["other"]["command"], json!(["x"]));
        assert!(!home.join(".config/opencode/skills/rust-anything").exists());

        // A file that lists servers in `mcp` by name (the older shape) keeps that shape.
        let flat = temp_home("opencode-flat");
        let flat_config = flat.join(".config/opencode/opencode.json");
        std::fs::create_dir_all(flat_config.parent().unwrap()).unwrap();
        std::fs::write(
            &flat_config,
            r#"{"mcp":{"other":{"type":"local","command":["x"]},"rust-anything":{"type":"local","command":["/old/rust-anything","mcp"],"enabled":true}}}"#,
        )
        .unwrap();
        setup(&opts(&flat, vec![Agent::OpenCode])).unwrap();
        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(&flat_config).unwrap()).unwrap();
        let entry = &doc["mcp"][NAME];
        assert_eq!(entry["command"], json!(["/opt/bin/rust-anything", "mcp"]));
        assert!(entry.get("enabled").is_none(), "{entry}");
        assert!(entry.get("disabled").is_none(), "{entry}");
        assert!(
            doc["mcp"].get("servers").is_none(),
            "shapes never mix: {doc}"
        );
        assert_eq!(doc["mcp"]["other"]["command"], json!(["x"]));
        std::fs::remove_dir_all(&flat).unwrap();

        // A file that does not parse is left alone with an error.
        std::fs::write(&config, "{ not json").unwrap();
        let err = setup(&opts(&home, vec![Agent::OpenCode])).unwrap_err();
        assert!(format!("{err:#}").contains("not valid JSON"), "{err:#}");
        assert_eq!(std::fs::read_to_string(&config).unwrap(), "{ not json");
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn keeps_a_symlinked_settings_file_and_its_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let home = temp_home("link");
        let dotfiles = home.join("dotfiles");
        std::fs::create_dir_all(&dotfiles).unwrap();
        let real = dotfiles.join("claude.json");
        std::fs::write(&real, r#"{"token": "secret"}"#).unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::os::unix::fs::symlink(&real, home.join(".claude.json")).unwrap();

        setup(&opts(&home, vec![Agent::Claude])).unwrap();
        let link = std::fs::symlink_metadata(home.join(".claude.json")).unwrap();
        assert!(link.file_type().is_symlink(), "the link is kept");
        let text = std::fs::read_to_string(&real).unwrap();
        assert!(text.contains("rust-anything"), "{text}");
        let mode = std::fs::metadata(&real).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "permissions kept");
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn finds_agents_and_never_breaks_a_bad_file() {
        let home = temp_home("detect");
        std::fs::create_dir_all(home.join(".cursor")).unwrap();
        std::fs::write(home.join(".cursor/mcp.json"), "{ not json").unwrap();
        // Only Cursor's folder exists (the commands may be on PATH on a developer machine).
        let found: Vec<Agent> = Agent::ALL
            .into_iter()
            .filter(|a| a.present(&home))
            .collect();
        assert!(found.contains(&Agent::Cursor));
        let err = setup(&opts(&home, vec![Agent::Cursor])).unwrap_err();
        assert!(format!("{err:#}").contains("not valid JSON"), "{err:#}");
        assert_eq!(
            std::fs::read_to_string(home.join(".cursor/mcp.json")).unwrap(),
            "{ not json"
        );

        // A dry run writes nothing.
        let mut dry = opts(&home, vec![Agent::Gemini]);
        dry.dry_run = true;
        let r = setup(&dry).unwrap();
        assert_eq!(r.changes[0].action, "added");
        assert!(!home.join(".gemini").exists());
        std::fs::remove_dir_all(&home).unwrap();
    }
}
