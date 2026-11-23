//! `doctor`: check the toolchains, the agent setup and the MCP server on this machine, and say
//! the command that fixes each problem.
//!
//! Language runtimes are optional: each matters only for the languages being ported. Cargo, the
//! MCP server and the setup of every agent found on the machine are required.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Value, json};

use crate::setup::{self, Agent, NAME, SKILL_FILES};
use crate::walk::Canonical;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Missing,
    Outdated,
    /// Something named rust-anything is there, but it is not ours.
    Foreign,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Missing => "missing",
            Status::Outdated => "outdated",
            Status::Foreign => "foreign",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub name: String,
    pub status: Status,
    /// A problem here makes `doctor` exit 1; optional checks only matter for some languages.
    pub required: bool,
    pub detail: String,
    pub fix: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub checks: Vec<Check>,
}

impl Report {
    /// Whether every required check passed.
    pub fn ok(&self) -> bool {
        self.checks
            .iter()
            .all(|c| !c.required || c.status == Status::Ok)
    }
}

/// A version as `major.minor.patch`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u32, pub u32, pub u32);

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// The first `N[.N[.N]]` in a version banner (`v24.1.0`, `go version go1.22.3 ...`,
/// `openjdk version "21.0.2"`, `ruby 2.6.10p210`).
pub fn parse_version(text: &str) -> Option<Version> {
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let mut parts = text[start..]
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()?
        .split('.')
        .map(|p| p.parse::<u32>().ok());
    let major = parts.next()??;
    let minor = parts.next().flatten().unwrap_or(0);
    let patch = parts.next().flatten().unwrap_or(0);
    Some(Version(major, minor, patch))
}

/// Java's version, reading the old `1.8.0` scheme as 8.
pub fn parse_java_version(text: &str) -> Option<Version> {
    parse_version(text).map(|v| match v {
        Version(1, minor, patch) => Version(minor, patch, 0),
        v => v,
    })
}

/// A runtime's status from whether it ran, its version and the oldest version that works.
pub fn grade(ran: bool, version: Option<Version>, minimum: Option<Version>) -> Status {
    match (ran, version, minimum) {
        (false, _, _) => Status::Missing,
        (true, Some(v), Some(min)) if v < min => Status::Outdated,
        _ => Status::Ok,
    }
}

/// The state of something `setup` installs: an MCP entry or the skill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Install {
    Missing,
    Current,
    Outdated,
    Foreign,
}

impl From<Install> for Status {
    fn from(i: Install) -> Status {
        match i {
            Install::Missing => Status::Missing,
            Install::Current => Status::Ok,
            Install::Outdated => Status::Outdated,
            Install::Foreign => Status::Foreign,
        }
    }
}

/// An agent's `rust-anything` MCP server entry, as read from its settings.
#[derive(Debug, Default)]
pub struct Entry {
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: bool,
    /// OpenCode keeps a configured server out of the way with `disabled`; never set elsewhere.
    pub disabled: bool,
}

/// Ours and current when it runs `<exe> mcp`; outdated when it runs `mcp` on another binary.
pub fn entry_state(entry: Option<&Entry>, exe: &Path) -> Install {
    let Some(entry) = entry else {
        return Install::Missing;
    };
    let Some(command) = entry.command.as_deref() else {
        return Install::Foreign;
    };
    if entry.url || entry.args != ["mcp"] {
        return Install::Foreign;
    }
    if entry.disabled {
        return Install::Outdated;
    }
    let path = Path::new(command);
    if path == exe || path.canonical().is_ok_and(|p| p == exe) {
        Install::Current
    } else {
        Install::Outdated
    }
}

/// The installed skill files, in `SKILL_FILES` order, against the embedded copies.
pub fn skill_state(installed: &[Option<String>]) -> Install {
    let Some(main) = installed.first().and_then(Option::as_deref) else {
        return if installed.iter().all(Option::is_none) {
            Install::Missing
        } else {
            Install::Outdated
        };
    };
    if installed
        .iter()
        .zip(SKILL_FILES)
        .all(|(have, (_, want))| have.as_deref() == Some(*want))
    {
        Install::Current
    } else if main
        .lines()
        .take_while(|l| !l.is_empty())
        .any(|l| l.trim() == format!("name: {NAME}"))
    {
        Install::Outdated
    } else {
        Install::Foreign
    }
}

pub fn run() -> Report {
    let exe = std::env::current_exe()
        .and_then(|p| p.canonical())
        .unwrap_or_else(|_| PathBuf::from(NAME));
    let mut checks: Vec<Check> = std::thread::scope(|s| {
        let probes: Vec<_> = RUNTIMES.iter().map(|r| s.spawn(|| runtime(r))).collect();
        let wasm = s.spawn(wasm_target);
        let ndk = s.spawn(android_ndk);
        let android_targets = s.spawn(android_targets);
        let mcp = s.spawn(|| mcp_self_test(&exe));
        let mut checks: Vec<Check> = probes
            .into_iter()
            .map(|p| p.join().expect("a runtime probe panicked"))
            .collect();
        checks.push(wasm.join().expect("the wasm probe panicked"));
        checks.push(ndk.join().expect("the NDK probe panicked"));
        checks.push(
            android_targets
                .join()
                .expect("the Android targets probe panicked"),
        );
        checks.push(mcp.join().expect("the MCP self-test panicked"));
        checks
    });
    checks.extend(agents(&exe));
    Report { checks }
}

/// The command `doctor` gives on this platform to install or update a runtime, by its name.
pub(crate) fn install_command(name: &str) -> Option<String> {
    let r = RUNTIMES.iter().find(|r| r.name == name)?;
    runtime_fix(r, Status::Outdated, Platform::HERE)
}

/// The oldest version of a runtime that `doctor` accepts, by its name.
pub(crate) fn minimum(name: &str) -> Option<Version> {
    RUNTIMES.iter().find(|r| r.name == name)?.minimum
}

struct Runtime {
    name: &'static str,
    program: &'static str,
    args: &'static [&'static str],
    minimum: Option<Version>,
    required: bool,
    java: bool,
    /// Fix commands: macOS, Windows, elsewhere.
    fix: [&'static str; 3],
}

/// The C++ Build Tools, which Rust links with on Windows. Without `--add`, winget installs them
/// with no workload: no compiler, linker or Windows SDK. The workload's recommended tools build for
/// x64 and x86, so a build for Windows on ARM (npm has no binary for it) adds the ARM64 tools.
const BUILD_TOOLS: &str = if cfg!(all(windows, target_arch = "aarch64")) {
    BUILD_TOOLS_ARM64
} else {
    BUILD_TOOLS_X64
};
const BUILD_TOOLS_X64: &str = "winget install Microsoft.VisualStudio.2022.BuildTools --override \"--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended\"";
const BUILD_TOOLS_ARM64: &str = "winget install Microsoft.VisualStudio.2022.BuildTools --override \"--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --add Microsoft.VisualStudio.Component.VC.Tools.ARM64\"";
/// winget adds nothing to Build Tools that are already installed: an upgrade cannot add a workload.
const BUILD_TOOLS_INSTALLED: &str = "(Build Tools already installed: add \"Desktop development with C++\" in Visual Studio Installer)";

const RUNTIMES: &[Runtime] = &[
    Runtime {
        name: "cargo",
        program: "cargo",
        args: &["--version"],
        minimum: Some(Version(1, 97, 0)),
        required: true,
        java: false,
        fix: [
            "rustup update stable",
            "rustup update stable",
            "rustup update stable",
        ],
    },
    Runtime {
        name: "node",
        program: "node",
        args: &["--version"],
        minimum: Some(Version(22, 6, 0)),
        required: false,
        java: false,
        fix: [
            "brew install node@24",
            "winget install OpenJS.NodeJS",
            "fnm install 24 && fnm default 24",
        ],
    },
    Runtime {
        name: "python3",
        program: if cfg!(windows) { "python" } else { "python3" },
        args: &["--version"],
        minimum: Some(Version(3, 9, 0)),
        required: false,
        java: false,
        fix: [
            "brew install python@3.13",
            "winget install Python.Python.3.13",
            "sudo apt install python3",
        ],
    },
    Runtime {
        name: "cc",
        program: "cc",
        args: &["--version"],
        minimum: None,
        required: false,
        java: false,
        fix: [
            "xcode-select --install",
            BUILD_TOOLS,
            "sudo apt install build-essential",
        ],
    },
    Runtime {
        name: "go",
        program: "go",
        args: &["version"],
        minimum: Some(Version(1, 22, 0)),
        required: false,
        java: false,
        fix: [
            "brew install go",
            "winget install GoLang.Go",
            "install Go 1.22+ from https://go.dev/dl",
        ],
    },
    Runtime {
        name: "ruby",
        program: "ruby",
        args: &["--version"],
        minimum: Some(Version(3, 0, 0)),
        required: false,
        java: false,
        fix: [
            "brew install ruby && export PATH=\"$(brew --prefix ruby)/bin:$PATH\"",
            "winget install RubyInstallerTeam.RubyWithDevKit.3.4",
            "sudo apt install ruby-full",
        ],
    },
    Runtime {
        name: "java",
        program: "java",
        args: &["-version"],
        minimum: Some(Version(17, 0, 0)),
        required: false,
        java: true,
        fix: [
            "brew install openjdk@21",
            "winget install EclipseAdoptium.Temurin.21.JDK",
            "sudo apt install openjdk-21-jdk",
        ],
    },
    Runtime {
        name: "javac",
        program: "javac",
        args: &["-version"],
        minimum: Some(Version(17, 0, 0)),
        required: false,
        java: true,
        fix: [
            "brew install openjdk@21",
            "winget install EclipseAdoptium.Temurin.21.JDK",
            "sudo apt install openjdk-21-jdk",
        ],
    },
    Runtime {
        name: "kotlinc",
        program: "kotlinc",
        args: &["-version"],
        minimum: None,
        required: false,
        java: false,
        fix: [
            "brew install kotlin",
            "winget install JetBrains.Kotlin",
            "sdk install kotlin",
        ],
    },
    Runtime {
        name: "php",
        program: "php",
        args: &["--version"],
        minimum: Some(Version(8, 0, 0)),
        required: false,
        java: false,
        fix: [
            "brew install php",
            "winget install PHP.PHP.8.4",
            "sudo apt install php-cli",
        ],
    },
    Runtime {
        name: "dotnet",
        program: "dotnet",
        args: &["--version"],
        minimum: None,
        required: false,
        java: false,
        fix: [
            "brew install --cask dotnet-sdk",
            "winget install Microsoft.DotNet.SDK.8",
            "install the .NET SDK from https://dotnet.microsoft.com/download",
        ],
    },
    Runtime {
        name: "wasm-bindgen",
        program: "wasm-bindgen",
        args: &["--version"],
        minimum: None,
        required: false,
        java: false,
        fix: [
            "cargo install wasm-bindgen-cli --version <the crate's wasm-bindgen version> --locked",
            "cargo install wasm-bindgen-cli --version <the crate's wasm-bindgen version> --locked",
            "cargo install wasm-bindgen-cli --version <the crate's wasm-bindgen version> --locked",
        ],
    },
];

/// Where a fix runs: [`Runtime::fix`] has one command for each, in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Platform {
    MacOs,
    Windows,
    Other,
}

impl Platform {
    const HERE: Platform = if cfg!(target_os = "macos") {
        Platform::MacOs
    } else if cfg!(windows) {
        Platform::Windows
    } else {
        Platform::Other
    };

    fn pick(self, fix: &[&'static str; 3]) -> &'static str {
        fix[self as usize]
    }
}

/// The commands that install or update `r` on `platform`, one per line, to run in order; `None`
/// when it is fine.
fn runtime_fix(r: &Runtime, status: Status, platform: Platform) -> Option<String> {
    let fix = match (status, r.name) {
        (Status::Ok, _) => return None,
        (Status::Outdated, "cargo") => "rustup update stable".to_string(),
        // rustup's script needs a Unix shell. winget runs `rustup-init -y`, which skips the Build
        // Tools with a warning, so they come first, on a line of their own: `cmd` does not split
        // commands at `;`.
        (Status::Missing, "cargo") if platform == Platform::Windows => {
            format!("{BUILD_TOOLS}\nwinget install Rustlang.Rustup")
        }
        (Status::Missing, "cargo") => {
            "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh".to_string()
        }
        _ => platform.pick(&r.fix).to_string(),
    };
    Some(if fix.starts_with(BUILD_TOOLS) {
        format!("{fix}\n{BUILD_TOOLS_INSTALLED}")
    } else {
        fix
    })
}

/// The program's banner (stdout, else stderr), or `None` when it did not run.
fn probe(program: &str, args: &[&str]) -> Option<String> {
    let out =
        crate::process::output(Command::new(program).args(args), crate::process::QUICK).ok()?;
    if !out.success() {
        return None;
    }
    let text = out.stdout_text();
    let text = if text.trim().is_empty() {
        out.stderr_text()
    } else {
        text
    };
    Some(text.lines().next().unwrap_or_default().trim().to_string())
}

fn which(program: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(program))
        .find(|p| p.is_file())
}

fn runtime(r: &Runtime) -> Check {
    let banner = probe(r.program, r.args);
    let version = banner.as_deref().and_then(if r.java {
        parse_java_version
    } else {
        parse_version
    });
    let status = grade(banner.is_some(), version, r.minimum);
    let mut detail = match (&banner, r.minimum) {
        (None, _) => format!("`{}` not found or did not run", r.program),
        (Some(b), Some(min)) if status == Status::Outdated => {
            format!("{b}; rust-anything needs {}.{}+", min.0, min.1)
        }
        (Some(b), _) => b.clone(),
    };
    if r.name == "node" && status == Status::Ok && version.is_some_and(|v| v.0 < 24) {
        detail.push_str("; works, but `profile` needs Node 24");
    }
    if r.name == "ruby" && status == Status::Outdated {
        detail.push_str("; magnus/rb-sys extensions do not build on Ruby 2");
        if cfg!(target_os = "macos") && which("ruby").is_some_and(|p| p.starts_with("/usr/bin")) {
            detail.push_str(" (this is macOS's system Ruby)");
        }
    }
    Check {
        name: r.name.to_string(),
        status,
        required: r.required,
        detail,
        fix: runtime_fix(r, status, Platform::HERE),
    }
}

fn wasm_target() -> Check {
    const TARGET: &str = "wasm32-unknown-unknown";
    let installed = crate::process::output(
        Command::new("rustup").args(["target", "list", "--installed"]),
        crate::process::QUICK,
    )
    .ok()
    .filter(|o| o.success())
    .map(|o| o.stdout_text());
    let (status, detail) = match installed {
        None => (Status::Missing, "`rustup` not found or did not run".into()),
        Some(list) if list.lines().any(|l| l.trim() == TARGET) => {
            (Status::Ok, format!("{TARGET} installed"))
        }
        Some(_) => (Status::Missing, format!("{TARGET} not installed")),
    };
    Check {
        name: TARGET.into(),
        status,
        required: false,
        detail,
        fix: (status != Status::Ok).then(|| format!("rustup target add {TARGET}")),
    }
}

/// The Android NDK `build --profile android` links with.
fn android_ndk() -> Check {
    let (status, detail) = match crate::android::ndk_root(None) {
        Ok(path) => (Status::Ok, path.display().to_string()),
        Err(e) => (Status::Missing, format!("{e:#}")),
    };
    Check {
        name: "Android NDK".into(),
        status,
        required: false,
        detail,
        fix: (status != Status::Ok).then(|| {
            "sdkmanager \"ndk;28.0.13004108\"  (or set ANDROID_NDK_HOME to an installed NDK)"
                .to_string()
        }),
    }
}

/// The Rust targets `build --profile android` needs.
fn android_targets() -> Check {
    let wanted: Vec<&str> = crate::android::DEFAULT_ABIS
        .iter()
        .map(|a| a.triple())
        .collect();
    let installed = crate::process::output(
        Command::new("rustup").args(["target", "list", "--installed"]),
        crate::process::QUICK,
    )
    .ok()
    .filter(|o| o.success())
    .map(|o| o.stdout_text());
    let (status, detail, fix) = match installed {
        None => (
            Status::Missing,
            "`rustup` not found or did not run".into(),
            "install Rust with rustup (https://rustup.rs) to add Android targets".to_string(),
        ),
        Some(list) => {
            let missing: Vec<&str> = wanted
                .iter()
                .copied()
                .filter(|t| !list.lines().any(|l| l.trim() == *t))
                .collect();
            if missing.is_empty() {
                (
                    Status::Ok,
                    format!("{} installed", wanted.join(", ")),
                    String::new(),
                )
            } else {
                (
                    Status::Missing,
                    format!("not installed: {}", missing.join(", ")),
                    format!("rustup target add {}", missing.join(" ")),
                )
            }
        }
    };
    Check {
        name: "Android Rust targets".into(),
        status,
        required: false,
        detail,
        fix: (status != Status::Ok).then_some(fix),
    }
}

/// Starts `<exe> mcp`, asks for `initialize` and `tools/list`, and expects both answers in 10 s.
fn mcp_self_test(exe: &Path) -> Check {
    let (status, detail) = match talk_to_mcp(exe, Duration::from_secs(10)) {
        Ok(tools) => (
            Status::Ok,
            format!("`{} mcp` answered with {tools} tools", exe.display()),
        ),
        Err(e) => (Status::Missing, format!("`{} mcp`: {e}", exe.display())),
    };
    Check {
        name: "mcp server".into(),
        status,
        required: true,
        detail,
        fix: (status != Status::Ok).then(|| {
            format!(
                "run `{} mcp` by hand and look at what it prints; reinstall with `cargo install rust-anything`",
                quoted(exe)
            )
        }),
    }
}

fn talk_to_mcp(exe: &Path, limit: Duration) -> Result<usize, String> {
    let mut child = Command::new(exe)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start: {e}"))?;
    let requests = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "rust-anything-doctor", "version": env!("CARGO_PKG_VERSION")}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    ];
    if let Some(mut stdin) = child.stdin.take() {
        for r in &requests {
            let _ = writeln!(stdin, "{r}");
        }
    }
    let (tx, rx) = mpsc::channel();
    if let Some(stdout) = child.stdout.take() {
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
    }
    let deadline = Instant::now() + limit;
    let mut init = None;
    let mut list = None;
    let outcome = loop {
        if let (Some(_), Some(_)) = (&init, &list) {
            break Ok(());
        }
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(line) => {
                let Ok(reply) = serde_json::from_str::<Value>(&line) else {
                    break Err(format!("printed a line that is not JSON: {line}"));
                };
                match reply["id"].as_i64() {
                    Some(1) => init = Some(reply),
                    Some(2) => list = Some(reply),
                    _ => {}
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                break Err(format!("no answer within {}s", limit.as_secs()));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break Err("exited before answering".to_string());
            }
        }
    };
    let _ = child.kill();
    let _ = child.wait();
    outcome?;
    let init = init.unwrap_or_default();
    let list = list.unwrap_or_default();
    if !init["result"]["protocolVersion"].is_string() {
        return Err(format!("`initialize` gave no protocol version: {init}"));
    }
    match list["result"]["tools"].as_array() {
        Some(tools) if !tools.is_empty() => Ok(tools.len()),
        _ => Err(format!("`tools/list` listed no tools: {list}")),
    }
}

fn quoted(path: &Path) -> String {
    let text = path.display().to_string();
    if text.contains(' ') {
        format!("\"{text}\"")
    } else {
        text
    }
}

fn slug(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "claude",
        Agent::Codex => "codex",
        Agent::Cursor => "cursor",
        Agent::Gemini => "gemini",
        Agent::OpenCode => "opencode",
    }
}

fn agents(exe: &Path) -> Vec<Check> {
    let home = match setup::home() {
        Ok(h) => h,
        Err(e) => {
            return vec![Check {
                name: "agents".into(),
                status: Status::Missing,
                required: false,
                detail: format!("{e:#}"),
                fix: Some("set HOME".into()),
            }];
        }
    };
    let found: Vec<Agent> = Agent::ALL
        .into_iter()
        .filter(|a| a.present(&home))
        .collect();
    if found.is_empty() {
        return vec![Check {
            name: "agents".into(),
            status: Status::Missing,
            required: false,
            detail: "no coding agent found (Claude Code, Codex, Cursor, Gemini CLI, OpenCode); the CLI works without one".into(),
            fix: Some(format!("{} setup --agent claude", quoted(exe))),
        }];
    }
    let setup_fix = |agent: Agent| format!("{} setup --agent {}", quoted(exe), slug(agent));
    let mut checks = Vec::new();
    for agent in found {
        let config = agent.config(&home);
        let (state, detail) = match read_entry(agent, &config) {
            Err(e) => (Install::Foreign, e),
            Ok(entry) => {
                let state = entry_state(entry.as_ref(), exe);
                let detail = match (state, &entry) {
                    (Install::Missing, _) => format!("no `{NAME}` server in {}", config.display()),
                    (_, Some(Entry { disabled: true, .. })) => {
                        format!("`{NAME}` in {} is disabled", config.display())
                    }
                    (
                        _,
                        Some(Entry {
                            command: Some(c), ..
                        }),
                    ) => {
                        format!("{} runs `{c}`", config.display())
                    }
                    _ => format!(
                        "`{NAME}` in {} is not `{} mcp`",
                        config.display(),
                        exe.display()
                    ),
                };
                (state, detail)
            }
        };
        checks.push(Check {
            name: format!("{} mcp", agent.name()),
            status: state.into(),
            required: true,
            detail,
            fix: match state {
                Install::Current => None,
                Install::Missing | Install::Outdated => Some(setup_fix(agent)),
                Install::Foreign => Some(format!(
                    "fix or remove the `{NAME}` entry in {}, then run `{}`",
                    config.display(),
                    setup_fix(agent)
                )),
            },
        });
        if let Some(dir) = agent.skill_dir(&home) {
            let installed: Vec<Option<String>> = SKILL_FILES
                .iter()
                .map(|(name, _)| std::fs::read_to_string(dir.join(name)).ok())
                .collect();
            let state = skill_state(&installed);
            let skill_md = dir.join("SKILL.md");
            checks.push(Check {
                name: format!("{} skill", agent.name()),
                status: state.into(),
                required: true,
                detail: match state {
                    Install::Missing => format!("{} not installed", skill_md.display()),
                    Install::Current => format!("{} matches this version", skill_md.display()),
                    Install::Outdated => {
                        format!("{} is from another version", skill_md.display())
                    }
                    Install::Foreign => {
                        format!("{} is not rust-anything's skill", skill_md.display())
                    }
                },
                fix: match state {
                    Install::Current => None,
                    Install::Missing | Install::Outdated => Some(setup_fix(agent)),
                    Install::Foreign => Some(format!(
                        "move {} aside, then run `{}`",
                        dir.display(),
                        setup_fix(agent)
                    )),
                },
            });
        }
    }
    checks
}

/// The agent's `rust-anything` entry; `Err` when the settings file cannot be read as settings.
fn read_entry(agent: Agent, path: &Path) -> Result<Option<Entry>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("reading {}: {e}", path.display())),
    };
    if text.trim().is_empty() {
        return Ok(None);
    }
    if agent == Agent::Codex {
        let doc: toml_edit::DocumentMut = text
            .parse()
            .map_err(|_| format!("{} is not valid TOML", path.display()))?;
        let Some(server) = doc
            .get("mcp_servers")
            .and_then(|s| s.get(NAME))
            .and_then(toml_edit::Item::as_table_like)
        else {
            return Ok(None);
        };
        return Ok(Some(Entry {
            command: server
                .get("command")
                .and_then(|c| c.as_str())
                .map(String::from),
            args: server
                .get("args")
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            url: server.get("url").is_some(),
            disabled: false,
        }));
    }
    if agent == Agent::OpenCode {
        let doc: Value = serde_json::from_str(&text)
            .map_err(|_| format!("{} is not valid JSON", path.display()))?;
        // Current OpenCode keeps servers under `mcp.servers`; older files list them in `mcp`.
        let mcp = &doc["mcp"];
        let servers = if mcp.get("servers").is_some() {
            &mcp["servers"]
        } else {
            mcp
        };
        let Some(server) = servers.get(NAME) else {
            return Ok(None);
        };
        // OpenCode's local server takes the whole argv in `command`.
        let mut argv = server["command"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)));
        return Ok(Some(Entry {
            command: argv.as_mut().and_then(|a| a.next()),
            args: argv.into_iter().flatten().collect(),
            url: server.get("url").is_some(),
            disabled: server["disabled"].as_bool() == Some(true),
        }));
    }
    let doc: Value =
        serde_json::from_str(&text).map_err(|_| format!("{} is not valid JSON", path.display()))?;
    let Some(server) = doc["mcpServers"].get(NAME) else {
        return Ok(None);
    };
    Ok(Some(Entry {
        command: server["command"].as_str().map(String::from),
        args: server["args"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        url: server.get("url").is_some(),
        disabled: false,
    }))
}

/// The report for people: one line per check, the fix under each problem.
pub fn text(r: &Report) -> String {
    let mut out = String::new();
    let width = r.checks.iter().map(|c| c.name.len()).max().unwrap_or(0);
    for c in &r.checks {
        let optional = if c.required || c.status == Status::Ok {
            ""
        } else {
            " (optional)"
        };
        out.push_str(&format!(
            "{:<9} {:<width$}  {}{optional}\n",
            c.status.label(),
            c.name,
            c.detail
        ));
        if let Some(fix) = &c.fix {
            // Further lines of a fix line up under the first.
            for (i, line) in fix.lines().enumerate() {
                let label = if i == 0 { "fix:" } else { "" };
                out.push_str(&format!("{:<9} {:<width$}  {label:<4} {line}\n", "", ""));
            }
        }
    }
    out.push_str(if r.ok() {
        "\nEverything required is in place. Language runtimes and wasm are optional: each matters only for the languages you port.\n"
    } else {
        "\nSomething required is missing: cargo, the MCP server, or the setup of an agent found here. Run the fix commands above.\n"
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_command_is_doctor_fix_for_a_runtime() {
        let ruby = RUNTIMES.iter().find(|r| r.name == "ruby").unwrap();
        assert_eq!(
            install_command("ruby"),
            runtime_fix(ruby, Status::Outdated, Platform::HERE)
        );
        assert!(install_command("cobol").is_none());
        assert_eq!(minimum("ruby"), ruby.minimum);
        assert!(minimum("cobol").is_none());
    }

    #[test]
    fn versions_are_read_from_every_banner() {
        let cases = [
            ("v24.1.0", Version(24, 1, 0)),
            ("Python 3.12.1", Version(3, 12, 1)),
            ("go version go1.22.3 darwin/arm64", Version(1, 22, 3)),
            (
                "ruby 2.6.10p210 (2022-04-12 revision 67958) [universal.arm64e-darwin26]",
                Version(2, 6, 10),
            ),
            ("openjdk version \"21.0.8\" 2025-07-15", Version(21, 0, 8)),
            ("javac 17", Version(17, 0, 0)),
            ("PHP 8.3.4 (cli) (built: Mar 12 2024)", Version(8, 3, 4)),
            ("cargo 1.97.0 (abcdef 2026-01-01)", Version(1, 97, 0)),
            ("8.0.100", Version(8, 0, 100)),
        ];
        for (text, want) in cases {
            assert_eq!(parse_version(text), Some(want), "{text}");
        }
        assert_eq!(parse_version("no digits"), None);
        assert_eq!(
            parse_java_version("java version \"1.8.0_281\""),
            Some(Version(8, 0, 0))
        );
        assert_eq!(
            parse_java_version("openjdk version \"17.0.2\""),
            Some(Version(17, 0, 2))
        );
    }

    #[test]
    fn runtimes_are_graded_against_their_minimum() {
        let min = Some(Version(3, 0, 0));
        assert_eq!(grade(false, None, min), Status::Missing);
        assert_eq!(grade(true, Some(Version(2, 6, 10)), min), Status::Outdated);
        assert_eq!(grade(true, Some(Version(3, 0, 0)), min), Status::Ok);
        assert_eq!(grade(true, Some(Version(3, 4, 1)), min), Status::Ok);
        assert_eq!(grade(true, None, min), Status::Ok);
        assert_eq!(grade(true, Some(Version(1, 0, 0)), None), Status::Ok);
        assert!(Version(22, 6, 0) > Version(22, 5, 9));
    }

    #[test]
    fn install_states_map_to_statuses() {
        assert_eq!(Status::from(Install::Missing), Status::Missing);
        assert_eq!(Status::from(Install::Current), Status::Ok);
        assert_eq!(Status::from(Install::Outdated), Status::Outdated);
        assert_eq!(Status::from(Install::Foreign), Status::Foreign);
        assert_eq!(
            serde_json::to_value(Status::Outdated).unwrap(),
            json!("outdated")
        );
    }

    #[test]
    fn mcp_entries_are_ours_only_when_they_run_this_binary() {
        let exe = Path::new("/opt/bin/rust-anything");
        let entry = |command: &str, args: &[&str], url: bool| Entry {
            command: Some(command.into()),
            args: args.iter().map(|a| a.to_string()).collect(),
            url,
            disabled: false,
        };
        assert_eq!(entry_state(None, exe), Install::Missing);
        assert_eq!(
            entry_state(Some(&entry("/opt/bin/rust-anything", &["mcp"], false)), exe),
            Install::Current
        );
        assert_eq!(
            entry_state(Some(&entry("/old/rust-anything", &["mcp"], false)), exe),
            Install::Outdated
        );
        assert_eq!(
            entry_state(
                Some(&entry("/opt/bin/rust-anything", &["serve"], false)),
                exe
            ),
            Install::Foreign
        );
        assert_eq!(
            entry_state(Some(&entry("/opt/bin/rust-anything", &["mcp"], true)), exe),
            Install::Foreign
        );
        assert_eq!(entry_state(Some(&Entry::default()), exe), Install::Foreign);
        // A server OpenCode keeps disabled is not current.
        let mut disabled = entry("/opt/bin/rust-anything", &["mcp"], false);
        disabled.disabled = true;
        assert_eq!(entry_state(Some(&disabled), exe), Install::Outdated);
    }

    #[test]
    fn opencode_entries_come_from_mcp_servers() {
        let home = std::env::temp_dir().join(format!(
            "rust-anything-doctor-opencode-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join(".config/opencode")).unwrap();
        let config = home.join(".config/opencode/opencode.json");
        std::fs::write(
            &config,
            r#"{"mcp":{"servers":{"rust-anything":{"type":"local","command":["/opt/bin/rust-anything","mcp"]}}}}"#,
        )
        .unwrap();
        let exe = Path::new("/opt/bin/rust-anything");
        let entry = read_entry(Agent::OpenCode, &config).unwrap();
        assert_eq!(entry_state(entry.as_ref(), exe), Install::Current);
        assert_eq!(
            entry_state(entry.as_ref(), Path::new("/elsewhere/rust-anything")),
            Install::Outdated
        );
        std::fs::write(
            &config,
            r#"{"mcp":{"servers":{"rust-anything":{"type":"local","command":["/opt/bin/rust-anything","mcp"],"disabled":true}}}}"#,
        )
        .unwrap();
        let entry = read_entry(Agent::OpenCode, &config).unwrap();
        assert_eq!(entry_state(entry.as_ref(), exe), Install::Outdated);
        // Older files list servers in `mcp` itself.
        std::fs::write(
            &config,
            r#"{"mcp":{"rust-anything":{"type":"local","command":["/opt/bin/rust-anything","mcp"]}}}"#,
        )
        .unwrap();
        let entry = read_entry(Agent::OpenCode, &config).unwrap();
        assert_eq!(entry_state(entry.as_ref(), exe), Install::Current);
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn skills_are_current_outdated_or_foreign() {
        let ours: Vec<Option<String>> = SKILL_FILES
            .iter()
            .map(|(_, t)| Some(t.to_string()))
            .collect();
        assert_eq!(skill_state(&ours), Install::Current);
        assert_eq!(
            skill_state(&vec![None; SKILL_FILES.len()]),
            Install::Missing
        );
        let mut old = ours.clone();
        old[0] = Some(format!(
            "---\nname: {NAME}\ndescription: old\n---\n\n# Old\n"
        ));
        assert_eq!(skill_state(&old), Install::Outdated);
        let mut partial = ours.clone();
        partial[1] = None;
        assert_eq!(skill_state(&partial), Install::Outdated);
        let mut foreign = ours;
        foreign[0] = Some("---\nname: something-else\n---\n".into());
        assert_eq!(skill_state(&foreign), Install::Foreign);
    }

    #[test]
    fn only_required_problems_fail_the_report() {
        let check = |status, required| Check {
            name: "x".into(),
            status,
            required,
            detail: String::new(),
            fix: None,
        };
        let optional = Report {
            checks: vec![check(Status::Ok, true), check(Status::Missing, false)],
        };
        assert!(optional.ok());
        let required = Report {
            checks: vec![check(Status::Outdated, true)],
        };
        assert!(!required.ok());
    }

    fn runtime_named(name: &str) -> &'static Runtime {
        RUNTIMES.iter().find(|r| r.name == name).unwrap()
    }

    #[test]
    fn fixes_follow_the_platform() {
        let cc = runtime_named("cc");
        let fix = |platform| runtime_fix(cc, Status::Missing, platform);
        assert_eq!(
            fix(Platform::MacOs).as_deref(),
            Some("xcode-select --install")
        );
        assert_eq!(
            fix(Platform::Windows),
            Some(format!("{BUILD_TOOLS}\n{BUILD_TOOLS_INSTALLED}"))
        );
        assert_eq!(
            fix(Platform::Other).as_deref(),
            Some("sudo apt install build-essential")
        );
        assert_eq!(runtime_fix(cc, Status::Ok, Platform::Windows), None);
    }

    #[test]
    fn windows_fixes_need_no_unix_shell() {
        for r in RUNTIMES {
            for status in [Status::Missing, Status::Outdated] {
                let fix = runtime_fix(r, status, Platform::Windows).unwrap();
                assert!(!fix.contains("| sh"), "{}: {fix}", r.name);
            }
        }
    }

    #[test]
    fn windows_build_tools_fixes_add_the_cpp_workload() {
        for name in ["cargo", "cc"] {
            let fix = runtime_fix(runtime_named(name), Status::Missing, Platform::Windows).unwrap();
            assert!(
                fix.contains("--add Microsoft.VisualStudio.Workload.VCTools"),
                "{name}: {fix}"
            );
        }
    }

    #[test]
    fn windows_cargo_fix_has_one_command_per_line() {
        let fix = runtime_fix(runtime_named("cargo"), Status::Missing, Platform::Windows).unwrap();
        assert_eq!(
            fix.lines().collect::<Vec<_>>(),
            [
                BUILD_TOOLS,
                "winget install Rustlang.Rustup",
                BUILD_TOOLS_INSTALLED
            ]
        );
    }

    #[test]
    fn windows_on_arm_adds_the_arm64_tools() {
        let workload = "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended";
        assert!(BUILD_TOOLS_X64.contains(workload));
        assert!(!BUILD_TOOLS_X64.contains("ARM64"));
        assert!(BUILD_TOOLS_ARM64.contains(workload));
        assert!(
            BUILD_TOOLS_ARM64.ends_with(" --add Microsoft.VisualStudio.Component.VC.Tools.ARM64\"")
        );
        assert_eq!(
            BUILD_TOOLS == BUILD_TOOLS_ARM64,
            cfg!(all(windows, target_arch = "aarch64"))
        );
    }

    #[test]
    fn fixes_of_several_lines_line_up() {
        let report = Report {
            checks: vec![Check {
                name: "cargo".into(),
                status: Status::Missing,
                required: true,
                detail: "not found".into(),
                fix: Some("first\nsecond".into()),
            }],
        };
        let out = text(&report);
        let lines: Vec<_> = out.lines().collect();
        assert!(lines[1].trim_start().starts_with("fix: first"), "{out}");
        assert_eq!(lines[1].find("first"), lines[2].find("second"), "{out}");
    }
}
