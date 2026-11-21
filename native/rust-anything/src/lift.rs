//! `app lift`: once the front door answers every recorded case in Rust, remove the fallback, the
//! kill switches and the units' bindings, and prove the pure binary on the recorded cases.
//!
//! The lift is made on a copy of `rust/` and only copied back when every check passes, so a lift
//! that fails leaves the repository as it was.

use crate::walk::Canonical;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};
use regex::Regex;
use serde::Serialize;
use toml_edit::{DocumentMut, Item as TomlItem, value};

use crate::app::{self, ReplayOptions, ReplayOutcome};
use crate::front_door::{self, APP_DIR, AppKind, MAIN_RS, STATE_FILE, State};
use crate::front_door_http as http_door;
use crate::gate::{self, Check};

const BINDING_DEPS: [&str; 9] = [
    "napi",
    "napi-derive",
    "napi-build",
    "wasm-bindgen",
    "pyo3",
    "pyo3-build-config",
    "magnus",
    "rb-sys",
    "jni",
];
const GENERATED_FILES: [&str; 3] = ["build.rs", "index.cjs", "index.d.ts"];
const GENERATED_DIRS: [&str; 7] = ["python", "go", "ruby", "java", "include", "shadow", "pkg"];
const GENERATED_MODULES: [&str; 6] = ["js", "wire", "pyconv", "rbconv", "utf16", "pynum"];
const ASSET_DIRS: [&str; 9] = [
    "assets",
    "templates",
    "static",
    "data",
    "locales",
    "i18n",
    "public",
    "resources",
    "share",
];
const TARGET_TRIPLE: &str = r"\b(?:x86_64|aarch64|i686|i586|armv7|arm|thumbv7em|wasm32|wasm64|riscv64gc|powerpc64le|s390x|loongarch64)-[a-z0-9_]+(?:-[a-z0-9_]+){1,2}\b";

const LIFTED_MAIN: &str = "mod cli;
mod units;

use std::process::ExitCode;

fn main() -> ExitCode {
    let matches = cli::command().get_matches();
    let (command, sub) = matches.subcommand().unwrap_or((cli::ROOT, &matches));
    units::dispatch(command, sub).unwrap_or_else(|| {
        eprintln!(\"{}: no handler for `{command}`\", cli::ROOT);
        ExitCode::from(2)
    })
}
";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mark {
    Pass,
    Warn,
    Fail,
    Skip,
}

/// One line of the lift checklist.
#[derive(Debug, Serialize)]
pub struct Item {
    pub name: String,
    pub mark: Mark,
    pub detail: String,
}

impl Item {
    fn new(name: impl Into<String>, mark: Mark, detail: impl Into<String>) -> Self {
        Item {
            name: name.into(),
            mark,
            detail: detail.into(),
        }
    }

    fn from_check(prefix: &str, check: Check) -> Self {
        Item::new(
            format!("{prefix}: {}", check.name),
            if check.passed { Mark::Pass } else { Mark::Fail },
            check.detail,
        )
    }
}

pub struct LiftOptions<'a> {
    pub repo: &'a Path,
}

#[derive(Debug, Serialize)]
pub struct LiftOutcome {
    pub lifted: bool,
    /// Why the lift did not happen.
    pub blockers: Vec<String>,
    pub checklist: Vec<Item>,
    pub units: Vec<String>,
    /// Files the lift removed, relative to the repository.
    pub removed: Vec<String>,
    pub binary: Option<PathBuf>,
    /// The recorded cases against the lifted binary.
    pub replay: Option<ReplayOutcome>,
    /// The copy the lift was checked in, kept when a check failed.
    pub stage: Option<PathBuf>,
}

impl LiftOutcome {
    fn refused(blockers: Vec<String>, checklist: Vec<Item>, stage: Option<PathBuf>) -> Self {
        LiftOutcome {
            lifted: false,
            blockers,
            checklist,
            units: Vec::new(),
            removed: Vec::new(),
            binary: None,
            replay: None,
            stage,
        }
    }
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)?.flatten() {
        let name = entry.file_name();
        if name == "target" || name == ".git" {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &to.join(&name))?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), to.join(&name))?;
        }
    }
    Ok(())
}

fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if dir == root && (entry.file_name() == "target" || entry.file_name() == ".git") {
                continue;
            }
            match entry.file_type() {
                Ok(t) if t.is_dir() => stack.push(path),
                Ok(t) if t.is_file() => out.push(path),
                _ => {}
            }
        }
    }
    out.sort();
    out
}

fn remove_empty_dirs(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|t| t.is_dir()) && entry.file_name() != "target" {
            remove_empty_dirs(&entry.path());
            let _ = std::fs::remove_dir(entry.path());
        }
    }
}

/// Makes `to` hold what `from` holds (its `target/` aside); returns the files removed from `to`.
fn sync(from: &Path, to: &Path) -> Result<Vec<PathBuf>> {
    let mut removed = Vec::new();
    for path in files_under(to) {
        let rel = path.strip_prefix(to).unwrap_or(&path);
        if !from.join(rel).exists() {
            std::fs::remove_file(&path)?;
            removed.push(path);
        }
    }
    copy_tree(from, to)?;
    remove_empty_dirs(to);
    Ok(removed)
}

/// Modules `port.rs` reaches through `crate::` and `super::`, transitively.
fn port_modules(src: &Path) -> BTreeSet<String> {
    let single = Regex::new(r"(?:crate|super)::([A-Za-z_][A-Za-z0-9_]*)").expect("valid regex");
    let group = Regex::new(r"(?:crate|super)::\{([^}]*)\}").expect("valid regex");
    let module_file = |name: &str| -> Option<PathBuf> {
        [
            src.join(format!("{name}.rs")),
            src.join(name).join("mod.rs"),
        ]
        .into_iter()
        .find(|p| p.is_file())
    };
    let mut keep = BTreeSet::new();
    let mut queue = vec![src.join("port.rs")];
    while let Some(file) = queue.pop() {
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let mut names: Vec<String> = single
            .captures_iter(&text)
            .map(|c| c[1].to_string())
            .collect();
        for c in group.captures_iter(&text) {
            names.extend(
                c[1].split(',')
                    .filter_map(|n| n.split("::").next())
                    .map(|n| n.trim().to_string()),
            );
        }
        for name in names {
            if name == "port" || keep.contains(&name) {
                continue;
            }
            if let Some(path) = module_file(&name) {
                keep.insert(name);
                queue.push(path);
            }
        }
    }
    keep
}

/// Turns a unit crate into a plain library: no `cdylib`, no binding dependencies, `lib.rs` that
/// only exposes `port` (and the modules it uses), and, for a crate `bridge` made, no generated
/// wrappers. Returns the paths it removed.
/// The crates under `rust_dir` that `roots` depend on by path, transitively, roots excluded.
fn path_dependencies(roots: &[PathBuf], rust_dir: &Path) -> Vec<PathBuf> {
    let mut seen: BTreeSet<PathBuf> = roots.iter().cloned().collect();
    let mut found = Vec::new();
    let mut queue: Vec<PathBuf> = roots.to_vec();
    while let Some(dir) = queue.pop() {
        let Ok(text) = std::fs::read_to_string(dir.join("Cargo.toml")) else {
            continue;
        };
        let Ok(doc) = text.parse::<DocumentMut>() else {
            continue;
        };
        for table in ["dependencies", "dev-dependencies", "build-dependencies"] {
            let Some(deps) = doc.get(table).and_then(TomlItem::as_table_like) else {
                continue;
            };
            for (_, dep) in deps.iter() {
                let Some(path) = dep.get("path").and_then(TomlItem::as_str) else {
                    continue;
                };
                let Ok(target) = dir.join(path).canonical() else {
                    continue;
                };
                if target.starts_with(rust_dir) && seen.insert(target.clone()) {
                    found.push(target.clone());
                    queue.push(target);
                }
            }
        }
    }
    found.sort();
    found
}

/// `lib.rs` of a stripped unit: `port` and the generated modules it uses, in the order rustfmt
/// keeps module declarations.
fn stripped_lib_rs(keep: &[String]) -> String {
    let mut modules: Vec<&str> = keep.iter().map(String::as_str).collect();
    modules.push("port");
    modules.sort_unstable();
    modules.dedup();
    modules.iter().map(|m| format!("pub mod {m};\n")).collect()
}

pub fn strip_unit(dir: &Path) -> Result<Vec<PathBuf>> {
    let cargo = dir.join("Cargo.toml");
    let mut doc: DocumentMut = std::fs::read_to_string(&cargo)
        .with_context(|| format!("reading {}", cargo.display()))?
        .parse()
        .with_context(|| format!("parsing {}", cargo.display()))?;
    if let Some(lib) = doc.get_mut("lib").and_then(TomlItem::as_table_mut) {
        lib.remove("crate-type");
        if lib.is_empty() {
            doc.remove("lib");
        }
    }
    for table in ["dependencies", "build-dependencies"] {
        if let Some(deps) = doc.get_mut(table).and_then(TomlItem::as_table_like_mut) {
            for dep in BINDING_DEPS {
                deps.remove(dep);
            }
        }
    }
    if doc
        .get("build-dependencies")
        .and_then(TomlItem::as_table_like)
        .is_some_and(|t| t.is_empty())
    {
        doc.remove("build-dependencies");
    }
    if let Some(package) = doc.get_mut("package").and_then(TomlItem::as_table_mut) {
        package.remove("build");
    }
    std::fs::write(&cargo, doc.to_string())?;

    let mut removed = Vec::new();
    if !dir.join(crate::bridge::MANIFEST).exists() {
        return Ok(removed);
    }
    let src = dir.join("src");
    let keep = port_modules(&src);
    let kept: Vec<String> = keep.iter().cloned().collect();
    std::fs::write(src.join("lib.rs"), stripped_lib_rs(&kept))?;
    for file in GENERATED_FILES {
        let path = dir.join(file);
        if path.is_file() {
            std::fs::remove_file(&path)?;
            removed.push(path);
        }
    }
    for sub in GENERATED_DIRS {
        let path = dir.join(sub);
        if path.is_dir() {
            removed.extend(files_under(&path));
            std::fs::remove_dir_all(&path)?;
        }
    }
    for module in GENERATED_MODULES {
        let path = src.join(format!("{module}.rs"));
        if !keep.contains(module) && path.is_file() {
            std::fs::remove_file(&path)?;
            removed.push(path);
        }
    }
    for entry in std::fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "node") && path.is_file() {
            std::fs::remove_file(&path)?;
            removed.push(path);
        }
    }
    Ok(removed)
}

fn lifted_units_rs(state: &State) -> String {
    let mut out = String::from(
        "//! Written by `rust-anything app lift`: every command is answered in Rust.\n\n\
         use std::process::ExitCode;\n\n\
         use clap::ArgMatches;\n\n",
    );
    for unit in &state.units {
        out += &format!("mod {};\n", front_door::ident(&unit.name));
    }
    out += "\npub fn dispatch(command: &str, matches: &ArgMatches) -> Option<ExitCode> {\n    match command {\n";
    for unit in &state.units {
        let pattern: Vec<String> = unit.commands.iter().map(|c| format!("{c:?}")).collect();
        out += &format!(
            "        {} => Some({}::run(command, matches)),\n",
            pattern.join(" | "),
            front_door::ident(&unit.name)
        );
    }
    out += "        _ => None,\n    }\n}\n";
    out
}

fn cases_test_rs(case_bin: &str, binary: &str, unordered: &[&String]) -> String {
    let skips: String = unordered
        .iter()
        .map(|case| format!("\n        .skip(\"../app-cases/{case}.toml\")"))
        .collect();
    format!(
        "#[test]\n\
         fn recorded_cases() {{\n    \
             trycmd::TestCases::new()\n        \
                 .register_bin(\n            \
                     {case_bin:?},\n            \
                     std::path::Path::new(env!(\"CARGO_BIN_EXE_{binary}\")),\n        \
                 )\n        \
                 .case(\"../app-cases/*.toml\"){skips};\n\
         }}\n"
    )
}

fn same_code(a: &str, b: &str) -> bool {
    let squash = |s: &str| s.split_whitespace().collect::<String>();
    squash(a) == squash(b)
}

fn generated_mains(state: &State) -> Vec<String> {
    match state.kind {
        AppKind::Cli => vec![MAIN_RS.to_string()],
        AppKind::Http => http_door::main_rs_versions(),
        AppKind::Library => Vec::new(),
    }
}

fn is_generated_main(code: &str, state: &State) -> bool {
    generated_mains(state).iter().any(|m| same_code(code, m))
}

fn ship_settings(doc: &mut DocumentMut) {
    if doc["package"].get("rust-version").is_none() {
        doc["package"]["rust-version"] = value(crate::MIN_RUST);
    }
    let release = doc["profile"]["release"].or_insert(toml_edit::table());
    if let Some(release) = release.as_table_like_mut() {
        if release.get("lto").is_none() {
            release.insert("lto", value(true));
        }
        if release.get("codegen-units").is_none() {
            release.insert("codegen-units", value(1));
        }
    }
    if let Some(profile) = doc["profile"].as_table_mut() {
        profile.set_implicit(true);
    }
}

fn lift_library(stage_app: &Path, state: &mut State) -> Result<()> {
    state.lifted = true;
    let _ = std::fs::remove_file(stage_app.join(crate::library::CARGO_CONFIG));
    let lib = stage_app.join("src/lib.rs");
    std::fs::write(&lib, crate::library::lib_rs(state, true))?;
    front_door::rustfmt(&lib);
    std::fs::write(
        stage_app.join(STATE_FILE),
        serde_json::to_string_pretty(state)? + "\n",
    )?;
    let cargo = stage_app.join("Cargo.toml");
    let mut doc: DocumentMut = std::fs::read_to_string(&cargo)?.parse()?;
    doc["package"]["description"] = value(format!(
        "`{}` in Rust, lifted by rust-anything from its units' ports",
        state.name
    ));
    ship_settings(&mut doc);
    std::fs::write(&cargo, doc.to_string())?;
    Ok(())
}

fn lift_app(
    stage_app: &Path,
    state: &mut State,
    case_bin: &str,
    unordered: &[&String],
) -> Result<()> {
    if state.kind == AppKind::Library {
        return lift_library(stage_app, state);
    }
    let http = state.kind == AppKind::Http;
    let main = stage_app.join("src/main.rs");
    if is_generated_main(&std::fs::read_to_string(&main)?, state) {
        let lifted = if http {
            http_door::lifted_main_rs()
        } else {
            LIFTED_MAIN.to_string()
        };
        std::fs::write(&main, lifted)?;
    }
    let _ = std::fs::remove_file(stage_app.join("src/original.rs"));
    let cli = stage_app.join("src/cli.rs");
    if let Ok(text) = std::fs::read_to_string(&cli) {
        std::fs::write(
            &cli,
            text.replace(
                "/// Routes argv to a command. Anything this parser rejects runs the original, so it never\n/// has to match every way the original accepts its arguments.\n",
                "/// Parses argv into a command and its arguments.\n",
            ),
        )?;
    }
    state.lifted = true;
    let units = stage_app.join("src/units/mod.rs");
    let routing = if http {
        http_door::units_rs(state, true)
    } else {
        lifted_units_rs(state)
    };
    std::fs::write(&units, routing)?;
    front_door::rustfmt(&units);
    std::fs::write(
        stage_app.join(STATE_FILE),
        serde_json::to_string_pretty(state)? + "\n",
    )?;

    let cargo = stage_app.join("Cargo.toml");
    let mut doc: DocumentMut = std::fs::read_to_string(&cargo)?.parse()?;
    let binary = doc["package"]["name"]
        .as_str()
        .context("rust/app/Cargo.toml has no package name")?
        .to_string();
    if let Some(description) = doc["package"].get("description").and_then(|d| d.as_str())
        && description.starts_with("Front door for")
    {
        doc["package"]["description"] = value(format!(
            "`{}` in Rust, lifted by rust-anything from its front door",
            state.name
        ));
    }
    ship_settings(&mut doc);
    if http {
        if let Some(deps) = doc
            .get_mut("dependencies")
            .and_then(TomlItem::as_table_like_mut)
        {
            deps.remove("reqwest");
        }
        std::fs::write(&cargo, doc.to_string())?;
        front_door::rustfmt(&main);
        return Ok(());
    }
    doc["dev-dependencies"]["trycmd"] = value("1");
    std::fs::write(&cargo, doc.to_string())?;
    std::fs::create_dir_all(stage_app.join("tests"))?;
    let test = stage_app.join("tests/app_cases.rs");
    std::fs::write(&test, cases_test_rs(case_bin, &binary, unordered))?;
    front_door::rustfmt(&test);
    front_door::rustfmt(&main);
    Ok(())
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn target_items(repo: &Path, stage_app: &Path, target_dir: &Path) -> Vec<Item> {
    let decisions = repo.join("rust/DECISIONS.md");
    let Ok(text) = std::fs::read_to_string(&decisions) else {
        return vec![Item::new(
            "targets",
            Mark::Skip,
            "no rust/DECISIONS.md: only the host target was built",
        )];
    };
    let triple = Regex::new(TARGET_TRIPLE).expect("valid regex");
    let targets: BTreeSet<&str> = triple.find_iter(&text).map(|m| m.as_str()).collect();
    if targets.is_empty() {
        return vec![Item::new(
            "targets",
            Mark::Skip,
            "rust/DECISIONS.md names no target triple: only the host target was built",
        )];
    }
    let host = command_output("rustc", &["-vV"]).and_then(|v| {
        v.lines()
            .find_map(|l| l.strip_prefix("host: ").map(str::to_string))
    });
    let installed: Vec<String> = command_output("rustup", &["target", "list", "--installed"])
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default();
    targets
        .into_iter()
        .map(|target| {
            let name = format!("target {target}");
            if host.as_deref() == Some(target) {
                return Item::new(name, Mark::Pass, "the host: built and tested above");
            }
            if !installed.iter().any(|t| t == target) {
                return Item::new(
                    name,
                    Mark::Skip,
                    format!("not installed here: `rustup target add {target}` to check it"),
                );
            }
            let out = Command::new("cargo")
                .args(["check", "--quiet", "--target", target, "--manifest-path"])
                .arg(stage_app.join("Cargo.toml"))
                .env("CARGO_TARGET_DIR", target_dir)
                .output();
            match out {
                Ok(o) if o.status.success() => {
                    Item::new(name, Mark::Pass, "`cargo check` passes (not linked)")
                }
                Ok(o) => Item::new(
                    name,
                    Mark::Fail,
                    String::from_utf8_lossy(&o.stderr)
                        .lines()
                        .rev()
                        .take(10)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
                Err(e) => Item::new(name, Mark::Fail, e.to_string()),
            }
        })
        .collect()
}

fn asset_items(repo: &Path, sources: &[PathBuf]) -> Vec<Item> {
    let reads =
        Regex::new(r#"(?:read_to_string|read|read_dir|read_to_end|File::open|open)\(\s*"([^"]+)""#)
            .expect("valid regex");
    let mut items = Vec::new();
    let mut included = String::new();
    for root in sources {
        for file in files_under(root) {
            if file.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            for line in text.lines().filter(|l| l.contains("include_")) {
                included += line;
            }
            for c in reads.captures_iter(&text) {
                items.push(Item::new(
                    "assets",
                    Mark::Warn,
                    format!(
                        "{} reads `{}` at run time from the working directory: embed it with include_str!/include_bytes! or ship it with the binary",
                        file.file_name().unwrap_or_default().to_string_lossy(),
                        &c[1]
                    ),
                ));
            }
        }
    }
    for dir in ASSET_DIRS {
        if repo.join(dir).is_dir() && !included.contains(dir) {
            items.push(Item::new(
                "assets",
                Mark::Warn,
                format!(
                    "the original has `{dir}/` and no include_str!/include_bytes! names it: embed what the app reads, or ship it with the binary"
                ),
            ));
        }
    }
    if items.is_empty() {
        items.push(Item::new(
            "assets",
            Mark::Pass,
            "no file read at run time from a fixed path, and no asset folder in the original",
        ));
    }
    items
}

pub fn lift(opts: &LiftOptions) -> Result<LiftOutcome> {
    let repo = opts
        .repo
        .canonical()
        .with_context(|| format!("no folder {}", opts.repo.display()))?;
    let mut state = State::load(&repo)?;
    if state.lifted {
        bail!("{} is already lifted", repo.join(APP_DIR).display());
    }
    let library = state.kind == AppKind::Library;
    let config = app::Config::load(&repo)?;
    ensure!(
        library || config.is_some(),
        "no recorded cases: run `app record` first"
    );
    let case_bin = config
        .as_ref()
        .map_or_else(|| state.name.clone(), |c| c.name.clone());
    let key_order = config
        .as_ref()
        .is_some_and(|c| c.normalize.iter().any(|n| n == "key-order"));
    let unordered: Vec<&String> = config
        .as_ref()
        .map(|c| c.unordered.keys().collect())
        .unwrap_or_default();
    let app_dir = repo.join(APP_DIR);
    let rust_dir = repo.join("rust");
    let mut blockers = Vec::new();
    if state.units.is_empty() {
        blockers.push("no unit is wired yet: run `rust-anything app wire` first".to_string());
    }
    let mut units = Vec::new();
    for unit in &state.units {
        let dir = app_dir
            .join(&unit.path)
            .canonical()
            .with_context(|| format!("unit crate {} not found", unit.path))?;
        if !dir.starts_with(&rust_dir) {
            blockers.push(format!(
                "{}: lift works on unit crates under rust/, and this one is at {}",
                unit.crate_name,
                dir.display()
            ));
        }
        let glue = app_dir
            .join("src/units")
            .join(format!("{}.rs", front_door::ident(&unit.name)));
        let glue_text = std::fs::read_to_string(&glue).unwrap_or_default();
        if !library && front_door::is_stub(&glue_text) {
            blockers.push(format!(
                "{}: the glue in {} is still the generated stub",
                unit.name,
                glue.display()
            ));
        } else if !library && glue_text.contains("original::") {
            blockers.push(format!(
                "{}: the glue in {} still hands cases to the original (`original::`): answer them in Rust, then lift again",
                unit.name,
                glue.display()
            ));
        }
        units.push((unit.clone(), dir));
    }
    let main = std::fs::read_to_string(app_dir.join("src/main.rs")).unwrap_or_default();
    if !library && !is_generated_main(&main, &state) && main.contains("original::") {
        blockers.push(
            "src/main.rs was changed and still calls the original: remove `mod original`, `original::report_fallbacks()` and `original::run` by hand, then lift again"
                .to_string(),
        );
    }

    let target_dir = app_dir.join("target");
    let before = if library {
        crate::library::cargo_build_lib(&app_dir, Some(&target_dir))
            .and_then(|()| crate::library::replay(&repo, &state, None))
    } else {
        front_door::cargo_build(&app_dir, Some(&target_dir)).and_then(|exe| {
            app::replay(&ReplayOptions {
                case: None,
                repo: &repo,
                build: Some(&app::shell_quote(&exe.to_string_lossy())),
                timeout_secs: None,
                port: None,
            })
        })
    };
    let replay_before = match &before {
        Ok(r) => Some((r.passed, r.total)),
        Err(_) => None,
    };
    match before {
        Err(e) => blockers.push(format!("the front door does not build: {e:#}")),
        Ok(replay) => {
            if replay.passed != replay.total {
                blockers.push(format!(
                    "{} of {} cases fail against the front door (see {})",
                    replay.total - replay.passed,
                    replay.total,
                    replay.report.display()
                ));
            }
            match &replay.fallback {
                None => blockers.push(
                    "the front door does not report its fallbacks (src/original.rs changed?)"
                        .to_string(),
                ),
                Some(f) if f.cases > 0 => {
                    let mut names = f.names.iter().take(10).cloned().collect::<Vec<_>>();
                    if f.names.len() > 10 {
                        names.push(format!("and {} more", f.names.len() - 10));
                    }
                    blockers.push(if library {
                        format!(
                            "{} of {} exports are still only in the original ({}): wire their units",
                            f.cases,
                            f.total,
                            names.join(", ")
                        )
                    } else {
                        format!(
                            "{} of {} cases still fall back to the original ({}): wire units for them",
                            f.cases,
                            f.total,
                            names.join(", ")
                        )
                    });
                }
                Some(_) => {}
            }
        }
    }
    for (unit, dir) in &units {
        let checks = if dir.join(crate::bridge::MANIFEST).exists() {
            gate::gate(dir, &[])?.checks
        } else {
            let mut checks = gate::rust_checks(dir, &dir.join("target"));
            checks.push(gate::ported_check(dir));
            checks
        };
        let failed: Vec<&str> = checks
            .iter()
            .filter(|c| !c.passed)
            .map(|c| c.name)
            .collect();
        if !failed.is_empty() {
            blockers.push(format!(
                "{}: gate fails ({}): run `rust-anything gate {}`",
                unit.crate_name,
                failed.join(", "),
                dir.display()
            ));
        }
    }
    if !blockers.is_empty() {
        return Ok(LiftOutcome::refused(blockers, Vec::new(), None));
    }

    let stage = std::env::temp_dir().join(format!(
        "rust-anything-lift-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let stage_rust = stage.join("rust");
    copy_tree(&app_dir, &stage_rust.join("app"))?;
    if app::cases_dir(&repo).is_dir() {
        copy_tree(&app::cases_dir(&repo), &stage_rust.join("app-cases"))?;
    }
    let mut staged_units = Vec::new();
    for (unit, dir) in &units {
        let rel = dir.strip_prefix(&rust_dir).unwrap_or(dir);
        let staged = stage_rust.join(rel);
        copy_tree(dir, &staged)?;
        strip_unit(&staged)?;
        staged_units.push((unit.clone(), dir.clone(), staged));
    }
    let unit_dirs: Vec<PathBuf> = units.iter().map(|(_, dir)| dir.clone()).collect();
    let mut staged_deps = Vec::new();
    for dir in path_dependencies(&unit_dirs, &rust_dir) {
        if dir == app_dir.canonical().unwrap_or_else(|_| app_dir.clone()) {
            continue;
        }
        let rel = dir.strip_prefix(&rust_dir).unwrap_or(&dir);
        let staged = stage_rust.join(rel);
        copy_tree(&dir, &staged)?;
        strip_unit(&staged)?;
        staged_deps.push((dir, staged));
    }
    let stage_app = stage_rust.join("app");
    lift_app(&stage_app, &mut state, &case_bin, &unordered)?;

    let mut checklist = Vec::new();
    for (unit, dir, staged) in &staged_units {
        for check in gate::rust_checks(staged, &dir.join("target")) {
            checklist.push(Item::from_check(&unit.crate_name, check));
        }
    }
    for (dir, staged) in &staged_deps {
        let name = dir
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        for check in gate::rust_checks(staged, &dir.join("target")) {
            checklist.push(Item::from_check(&name, check));
        }
    }
    for check in gate::rust_checks(&stage_app, &target_dir) {
        checklist.push(Item::from_check("rust/app", check));
    }
    checklist.push(Item::new(
        "replay in CI",
        if state.kind != AppKind::Cli || key_order || !unordered.is_empty() {
            Mark::Warn
        } else {
            Mark::Pass
        },
        if library {
            "the recorded cases ran through the units' bindings, which the lift removes: `cargo test` on the crates is what CI keeps; turn the cases that matter into Rust tests"
        } else if state.kind == AppKind::Http {
            "trycmd runs command lines only: run `rust-anything app replay <repo> --build <the binary>` in CI for the HTTP cases"
        } else if !unordered.is_empty() {
            "rust/app/tests/app_cases.rs runs the cases with trycmd on `cargo test`, which compares line order exactly, so it skips the cases whose order the original's own runs disagreed on: also run `rust-anything app replay` in CI for them"
        } else if key_order {
            "rust/app/tests/app_cases.rs runs the cases with trycmd on `cargo test`, which compares JSON as text: also run `rust-anything app replay` in CI for the key-order normalizer"
        } else {
            "rust/app/tests/app_cases.rs runs the recorded cases with trycmd on `cargo test`"
        },
    ));
    checklist.extend(target_items(&repo, &stage_app, &target_dir));
    let mut sources = vec![stage_app.join("src")];
    sources.extend(staged_units.iter().map(|(_, _, s)| s.join("src")));
    checklist.extend(asset_items(&repo, &sources));

    if library {
        let references = crate::library::original_references(&repo, &state);
        checklist.push(if references.is_empty() {
            Item::new(
                "original package",
                Mark::Pass,
                "nothing in the original names the units' bindings or shadows",
            )
        } else {
            Item::new(
                "original package",
                Mark::Warn,
                format!(
                    "these files use the units' bindings or shadows, which the lift removes; the original stays as reference, so point them back at its own modules: {}",
                    references.join(", ")
                ),
            )
        });
        let unchanged = staged_units.iter().all(|(_, dir, staged)| {
            std::fs::read(dir.join("src/port.rs")).ok()
                == std::fs::read(staged.join("src/port.rs")).ok()
        });
        let built = crate::library::cargo_build_lib(&stage_app, Some(&target_dir));
        let (passed, total) = replay_before.unwrap_or((0, 0));
        checklist.push(Item::new(
            "replay",
            if built.is_ok() && unchanged && passed == total {
                Mark::Pass
            } else {
                Mark::Fail
            },
            match built {
                Err(e) => format!("{e:#}"),
                Ok(()) => format!(
                    "{passed}/{total} recorded cases matched through the bindings before the lift; the lift leaves every port.rs as it was, and the whole API builds as one Rust library"
                ),
            },
        ));
    }
    let built = if library {
        Ok(PathBuf::new())
    } else {
        front_door::cargo_build(&stage_app, Some(&target_dir))
    };
    let replay = match built {
        Ok(_) if library => None,
        Err(e) => {
            checklist.push(Item::new("build", Mark::Fail, format!("{e:#}")));
            None
        }
        Ok(exe) => {
            let replay = app::replay(&ReplayOptions {
                case: None,
                repo: &repo,
                build: Some(&app::shell_quote(&exe.to_string_lossy())),
                timeout_secs: None,
                port: None,
            })?;
            checklist.push(Item::new(
                "replay",
                if replay.passed == replay.total {
                    Mark::Pass
                } else {
                    Mark::Fail
                },
                format!(
                    "{}/{} cases pass against the lifted binary, fallback rate 0 (no fallback path left)",
                    replay.passed, replay.total
                ),
            ));
            Some(replay)
        }
    };
    let failed: Vec<String> = checklist
        .iter()
        .filter(|i| i.mark == Mark::Fail)
        .map(|i| format!("{}: {}", i.name, i.detail))
        .collect();
    if !failed.is_empty() {
        let mut outcome = LiftOutcome::refused(failed, checklist, Some(stage));
        outcome.replay = replay;
        return Ok(outcome);
    }

    let mut removed = Vec::new();
    removed.extend(sync(&stage_app, &app_dir)?);
    for (_, dir, staged) in &staged_units {
        removed.extend(sync(staged, dir)?);
    }
    for (dir, staged) in &staged_deps {
        removed.extend(sync(staged, dir)?);
    }
    let _ = std::fs::remove_dir_all(&stage);
    let binary = if library {
        crate::library::cargo_build_lib(&app_dir, Some(&target_dir))?;
        None
    } else {
        Some(front_door::cargo_build(&app_dir, Some(&target_dir))?)
    };
    Ok(LiftOutcome {
        lifted: true,
        blockers: Vec::new(),
        checklist,
        units: state.units.iter().map(|u| u.crate_name.clone()).collect(),
        removed: removed
            .iter()
            .map(|p| {
                p.strip_prefix(&repo)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .into_owned()
            })
            .collect(),
        binary,
        replay,
        stage: None,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn stripped_lib_rs_lists_modules_as_rustfmt_orders_them() {
        assert_eq!(
            super::stripped_lib_rs(&["utf16".into(), "js".into()]),
            "pub mod js;\npub mod port;\npub mod utf16;\n"
        );
    }

    #[test]
    fn path_dependencies_are_found_transitively_under_rust() {
        let root = crate::app::temp_dir("lift-deps").unwrap();
        let rust = root.join("rust");
        let write = |name: &str, deps: &str| {
            let dir = rust.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("Cargo.toml"),
                format!("[package]\nname = \"{name}\"\n\n[dependencies]\n{deps}"),
            )
            .unwrap();
        };
        std::fs::create_dir_all(root.join("outside")).unwrap();
        write(
            "unit-rs",
            "shared = { path = \"../shared\" }\nother = { path = \"../../outside\" }\nserde = \"1\"\n",
        );
        write("shared", "leaf = { path = \"../leaf\" }\n");
        write("leaf", "");
        let rust = rust.canonical().unwrap();
        let found = super::path_dependencies(&[rust.join("unit-rs")], &rust);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(found, [rust.join("leaf"), rust.join("shared")]);
    }

    use super::*;

    #[test]
    fn port_modules_follow_crate_paths() {
        let dir =
            std::env::temp_dir().join(format!("rust-anything-lift-mods-{}", std::process::id()));
        let src = dir.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("port.rs"),
            "use crate::utf16::len;\nuse crate::{helpers, Thing};\n",
        )
        .unwrap();
        std::fs::write(src.join("utf16.rs"), "").unwrap();
        std::fs::write(src.join("helpers.rs"), "use super::deep;\n").unwrap();
        std::fs::write(src.join("deep.rs"), "").unwrap();
        std::fs::write(src.join("wire.rs"), "").unwrap();
        let keep: Vec<String> = port_modules(&src).into_iter().collect();
        assert_eq!(keep, ["deep", "helpers", "utf16"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn lifted_routing_has_no_kill_switch() {
        let state = State {
            schema: String::new(),
            name: "tcli".into(),
            commands: vec!["words".into()],
            units: vec![front_door::WiredUnit {
                name: "wordcount".into(),
                crate_name: "wordcount-rs".into(),
                path: "../wordcount-rs".into(),
                commands: vec!["words".into(), "tcli".into()],
                switch: "RUST_ANYTHING_UNIT_WORDCOUNT".into(),
                routes: Vec::new(),
                unit: None,
                shadow: None,
                kept: Vec::new(),
            }],
            lifted: true,
            kind: AppKind::Cli,
            routes: Vec::new(),
            api: Vec::new(),
        };
        let code = lifted_units_rs(&state);
        assert!(code.contains("\"words\" | \"tcli\" => Some(wordcount::run(command, matches)),"));
        assert!(!code.contains("RUST_ANYTHING_UNIT"));
        assert!(!LIFTED_MAIN.contains("original"));
    }
}
