//! Step 6: turn a gated port into packages end users install, the way maturin and
//! `@napi-rs/cli` lay them out. Writes files only: it never builds for other platforms (the
//! generated GitHub Actions workflow does) and never publishes.
//!
//! - PyO3: a self-contained maturin project (abi3 wheel, importable under the original's module
//!   name) and a workflow that builds wheels for macOS, Linux (manylinux) and Windows.
//! - napi-rs: a main npm package whose loader picks `<name>-<platform>` from its
//!   `optionalDependencies` and falls back to the original JS, one package per platform holding
//!   one `.node`, and a workflow that builds and packs them.
//! - wasm-bindgen: one npm package with the built `pkg/`, the same on every platform.
//! - A CLI front door (`rust/app`) of an npm CLI: a main package whose command runs this
//!   platform's front door binary, with the original pinned as a dependency for every case the
//!   port does not cover, and one package per platform holding the binary (`cli.rs`).

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::bridge::{Kind, Manifest};

mod cli;

/// Written into the output folder; lists what the last run wrote, so a rerun replaces only that.
pub const STAMP: &str = "rust-anything-package.json";
const STAMP_SCHEMA: &str = "rust-anything/package@1";

pub struct Options<'a> {
    pub crate_dir: &'a Path,
    /// Package name on npm or PyPI; defaults to the crate name.
    pub name: Option<String>,
    /// Package version; defaults to the original package's.
    pub version: Option<String>,
    /// Output folder; defaults to `<crate>/package`.
    pub out: Option<PathBuf>,
    /// List what would be written, write nothing.
    pub dry_run: bool,
    /// napi: the module the loader requires when no binary fits the platform.
    pub fallback: Option<String>,
    /// CLI: the commands the package installs; defaults to the package name without its scope.
    pub bin: Vec<String>,
    /// CLI: the original npm package the front door falls back to, as `name@version`; defaults
    /// to the one in `node_modules` of the repository the front door belongs to.
    pub original: Option<String>,
}

/// What a package folder holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    Napi,
    Wasm,
    Pyo3,
    /// A front door's binary per platform behind an npm command.
    Cli,
}

impl From<Kind> for Layout {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Wasm => Layout::Wasm,
            Kind::Pyo3 => Layout::Pyo3,
            _ => Layout::Napi,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Outcome {
    pub kind: Layout,
    pub name: String,
    pub version: String,
    pub out: PathBuf,
    /// Python: the module the wheel installs (`import <module>`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    /// napi: the platform packages, `<name>-<key>`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<String>,
    /// CLI: the commands the package installs.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bin: Vec<String>,
    /// CLI: the original the package depends on, `name@version`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original: Option<String>,
    pub files: Vec<String>,
    pub dry_run: bool,
    pub notes: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Stamp {
    schema: String,
    kind: Layout,
    name: String,
    version: String,
    files: Vec<String>,
}

enum Content {
    Text(String),
    Copy(PathBuf),
}

/// A platform a napi port ships a binary for: the npm key napi-rs uses, the Rust target, and
/// the runner that builds it.
pub struct Platform {
    pub key: &'static str,
    pub target: &'static str,
    os: &'static str,
    cpu: &'static str,
    libc: Option<&'static str>,
    runner: &'static str,
    /// Whether the runner can load what it built (no cross-compiled check).
    native: bool,
}

pub const PLATFORMS: &[Platform] = &[
    Platform {
        key: "darwin-arm64",
        target: "aarch64-apple-darwin",
        os: "darwin",
        cpu: "arm64",
        libc: None,
        runner: "macos-latest",
        native: true,
    },
    Platform {
        key: "darwin-x64",
        target: "x86_64-apple-darwin",
        os: "darwin",
        cpu: "x64",
        libc: None,
        runner: "macos-latest",
        native: false,
    },
    Platform {
        key: "linux-x64-gnu",
        target: "x86_64-unknown-linux-gnu",
        os: "linux",
        cpu: "x64",
        libc: Some("glibc"),
        runner: "ubuntu-22.04",
        native: true,
    },
    Platform {
        key: "linux-arm64-gnu",
        target: "aarch64-unknown-linux-gnu",
        os: "linux",
        cpu: "arm64",
        libc: Some("glibc"),
        runner: "ubuntu-22.04-arm",
        native: true,
    },
    Platform {
        key: "win32-x64-msvc",
        target: "x86_64-pc-windows-msvc",
        os: "win32",
        cpu: "x64",
        libc: None,
        runner: "windows-latest",
        native: true,
    },
];

/// The napi key of the machine running rust-anything, if a package is built for it.
pub fn host_key() -> Option<&'static str> {
    let target = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") if cfg!(target_env = "gnu") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") if cfg!(target_env = "gnu") => "aarch64-unknown-linux-gnu",
        ("windows", "x86_64") if cfg!(target_env = "msvc") => "x86_64-pc-windows-msvc",
        _ => return None,
    };
    PLATFORMS.iter().find(|p| p.target == target).map(|p| p.key)
}

pub fn package(opts: &Options) -> Result<Outcome> {
    let crate_dir = std::path::absolute(opts.crate_dir)?;
    if cli::is_front_door(&crate_dir) {
        return cli::package(&crate_dir, opts);
    }
    let manifest = Manifest::load(&crate_dir)?;
    let out = match &opts.out {
        Some(out) => std::path::absolute(out)?,
        None => crate_dir.join("package"),
    };
    let name = match &opts.name {
        Some(name) => name.clone(),
        None => manifest.crate_name.clone(),
    };
    let mut notes = Vec::new();
    let version = match &opts.version {
        Some(v) => v.clone(),
        None => original_version(&manifest).unwrap_or_else(|| {
            let v = cargo_version(&crate_dir).unwrap_or_else(|| "0.1.0".into());
            notes.push(format!(
                "No version found in the original package; using {v}. Pass --version to set one."
            ));
            v
        }),
    };
    let mut files: Vec<(String, Content)> = Vec::new();
    let mut module = None;
    let mut platforms = Vec::new();
    match manifest.kind {
        Kind::Pyo3 => {
            let m = python_module(&manifest);
            python_files(&crate_dir, &manifest, &name, &version, &m, &out, &mut files)?;
            notes.push(format!(
                "pip install {0} builds the wheel locally (maturin, from PyPI); `maturin build \
                 --release` in {0} writes it to target/wheels. The wheel installs `import {m}`.",
                out.display()
            ));
            module = Some(m);
        }
        Kind::Napi => {
            validate_npm_name(&name)?;
            platforms = PLATFORMS
                .iter()
                .map(|p| format!("{name}-{}", p.key))
                .collect();
            napi_files(
                &crate_dir, &manifest, &name, &version, opts, &out, &mut files, &mut notes,
            )?;
        }
        Kind::Wasm => {
            validate_npm_name(&name)?;
            wasm_files(&crate_dir, &manifest, &name, &version, &mut files)?;
            notes.push(format!(
                "npm pack {} gives one tarball that runs on every platform.",
                out.display()
            ));
        }
        Kind::CAbi | Kind::Ffi => bail!(
            "package covers napi-rs, wasm-bindgen and PyO3 crates; this one is {:?}",
            manifest.kind
        ),
    }
    if matches!(manifest.kind, Kind::Pyo3 | Kind::Napi) {
        let (workflow, note) = workflow_file(&manifest, &name, &version, &crate_dir, &out)?;
        files.push(workflow);
        notes.push(note);
    }
    notes.push("Nothing was published.".into());
    let listed: Vec<String> = files.iter().map(|(p, _)| p.clone()).collect();
    if !opts.dry_run {
        write(&out, manifest.kind.into(), &name, &version, files)?;
    }
    Ok(Outcome {
        kind: manifest.kind.into(),
        name,
        version,
        out,
        module,
        platforms,
        bin: Vec::new(),
        original: None,
        files: listed,
        dry_run: opts.dry_run,
        notes,
    })
}

fn write(
    out: &Path,
    kind: Layout,
    name: &str,
    version: &str,
    files: Vec<(String, Content)>,
) -> Result<()> {
    let stamp_path = out.join(STAMP);
    if stamp_path.exists() {
        let text = std::fs::read_to_string(&stamp_path)?;
        let old: Stamp = serde_json::from_str(&text)
            .with_context(|| format!("parsing {}", stamp_path.display()))?;
        for file in old.files {
            let path = out.join(&file);
            if safe_relative(&file) && path.is_file() {
                std::fs::remove_file(&path)?;
            }
        }
    } else if out.exists() && std::fs::read_dir(out)?.next().is_some() {
        bail!(
            "{} is not empty and was not written by `rust-anything package`; pass --out with an \
             empty or new folder",
            out.display()
        );
    }
    for (path, content) in &files {
        let target = out.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match content {
            Content::Text(text) => std::fs::write(&target, text),
            Content::Copy(from) => std::fs::copy(from, &target).map(|_| ()),
        }
        .with_context(|| format!("writing {}", target.display()))?;
    }
    let stamp = Stamp {
        schema: STAMP_SCHEMA.into(),
        kind,
        name: name.into(),
        version: version.into(),
        files: files.into_iter().map(|(p, _)| p).collect(),
    };
    std::fs::write(&stamp_path, serde_json::to_string_pretty(&stamp)? + "\n")?;
    Ok(())
}

fn safe_relative(path: &str) -> bool {
    Path::new(path)
        .components()
        .all(|c| matches!(c, Component::Normal(_)))
}

fn validate_npm_name(name: &str) -> Result<()> {
    // A scoped name is exactly `@scope/name`, and npm applies the `.`/`_` rule to each segment:
    // `@x18n/.hidden`, `@x18n/_private`, `@x18n/fuzzy/extra`, `@x18n` and `@/fuzzy` are all
    // refused at publish time, so they are refused here instead of in the generated package.
    let (scope, bare) = match name.strip_prefix('@') {
        Some(rest) => match rest.split_once('/') {
            Some((scope, pkg)) => (Some(scope), pkg),
            None => (None, ""),
        },
        None => (None, name),
    };
    let segment = |s: &str| !s.is_empty() && !s.starts_with(['.', '_']) && !s.contains(['/', '@']);
    let ok = segment(bare)
        && scope.is_none_or(segment)
        && name.len() <= 214
        && name.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.' | '_' | '@' | '/')
        });
    if !ok {
        bail!("`{name}` is not a valid npm package name (lowercase, URL-safe); pass --name");
    }
    Ok(())
}

/// The version of the package the unit belongs to: the nearest `package.json` for JS, the
/// `pyproject.toml` (or `setup.cfg`, or `__version__`) for Python.
pub fn original_version(manifest: &Manifest) -> Option<String> {
    let root = &manifest.root;
    let unit = root.join(manifest.unit.trim_end_matches('/'));
    match manifest.language {
        crate::lang::Lang::Python => python_version(root, &unit),
        _ => {
            let mut dir = unit.parent()?.to_path_buf();
            loop {
                let pkg = dir.join("package.json");
                if let Ok(text) = std::fs::read_to_string(&pkg)
                    && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
                    && let Some(v) = value["version"].as_str()
                {
                    return Some(v.to_string());
                }
                if dir == *root || !dir.pop() || !dir.starts_with(root) {
                    return None;
                }
            }
        }
    }
}

fn python_version(root: &Path, unit: &Path) -> Option<String> {
    if let Ok(text) = std::fs::read_to_string(root.join("pyproject.toml"))
        && let Ok(doc) = text.parse::<toml_edit::DocumentMut>()
    {
        let v = doc
            .get("project")
            .and_then(|p| p.get("version"))
            .or_else(|| {
                doc.get("tool")
                    .and_then(|t| t.get("poetry"))
                    .and_then(|p| p.get("version"))
            })
            .and_then(|v| v.as_str());
        if let Some(v) = v {
            return Some(v.to_string());
        }
    }
    if let Ok(text) = std::fs::read_to_string(root.join("setup.cfg")) {
        let mut in_metadata = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_metadata = line == "[metadata]";
            } else if in_metadata
                && let Some((key, value)) = line.split_once('=')
                && key.trim() == "version"
                && !value.trim().starts_with("attr:")
            {
                return Some(value.trim().to_string());
            }
        }
    }
    let init = if unit.is_dir() {
        unit.join("__init__.py")
    } else {
        unit.parent()?.join("__init__.py")
    };
    let text = std::fs::read_to_string(init).ok()?;
    text.lines().find_map(|l| {
        let rest = l.trim().strip_prefix("__version__")?.trim_start();
        let rest = rest.strip_prefix('=')?.trim();
        let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        rest[1..].split(quote).next().map(str::to_string)
    })
}

fn cargo_version(crate_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(crate_dir.join("Cargo.toml")).ok()?;
    let doc = text.parse::<toml_edit::DocumentMut>().ok()?;
    Some(doc.get("package")?.get("version")?.as_str()?.to_string())
}

/// The dotted module the original unit is imported as: `src/textkit/` is `textkit`,
/// `src/textkit/words.py` is `textkit.words`. Folders above count while they are packages
/// (hold `__init__.py`); without the sources, a leading `src/` or `lib/` is dropped.
pub fn python_module(manifest: &Manifest) -> String {
    let unit = manifest.unit.trim_end_matches('/');
    let unit = unit.strip_suffix("/__init__.py").unwrap_or(unit);
    let unit = unit.strip_suffix(".py").unwrap_or(unit);
    let parts: Vec<&str> = unit.split('/').filter(|p| !p.is_empty()).collect();
    let root = &manifest.root;
    let mut start = parts.len().saturating_sub(1);
    if root.join(unit).exists() || root.join(format!("{unit}.py")).exists() {
        while start > 0
            && root
                .join(parts[..start].join("/"))
                .join("__init__.py")
                .exists()
        {
            start -= 1;
        }
    } else {
        start = usize::from(matches!(parts.first(), Some(&"src" | &"lib")) && parts.len() > 1);
    }
    parts[start..].join(".")
}

#[allow(clippy::too_many_arguments)]
fn python_files(
    crate_dir: &Path,
    manifest: &Manifest,
    name: &str,
    version: &str,
    module: &str,
    out: &Path,
    files: &mut Vec<(String, Content)>,
) -> Result<()> {
    if out.starts_with(crate_dir.join("src")) || out.starts_with(crate_dir.join("python")) {
        bail!("--out cannot be inside the crate's src/ or python/ folder");
    }
    let (wrapper_dir, wrapper_name) = crate::harness_py::package(crate_dir, manifest);
    let wrapper = wrapper_dir.join(&wrapper_name);
    if !wrapper.join("__init__.py").exists() {
        bail!(
            "{} has no __init__.py; run `rust-anything refresh {}`",
            wrapper.display(),
            crate_dir.display()
        );
    }
    let cargo =
        std::fs::read_to_string(crate_dir.join("Cargo.toml")).context("reading Cargo.toml")?;
    let mut doc: toml_edit::DocumentMut = cargo.parse().context("parsing Cargo.toml")?;
    if !doc.contains_key("workspace") {
        doc.insert("workspace", toml_edit::Item::Table(toml_edit::Table::new()));
    }
    files.push(("Cargo.toml".into(), Content::Text(doc.to_string())));
    for extra in ["Cargo.lock", "build.rs"] {
        if crate_dir.join(extra).exists() {
            files.push((extra.into(), Content::Copy(crate_dir.join(extra))));
        }
    }
    for file in tree(&crate_dir.join("src"))? {
        files.push((
            format!("src/{}", slash(&file)),
            Content::Copy(crate_dir.join("src").join(&file)),
        ));
    }
    let module_dir = format!("python/{}", module.replace('.', "/"));
    for file in tree(&wrapper)? {
        let file_name = file.file_name().unwrap_or_default().to_string_lossy();
        if file_name.starts_with("_native.")
            || file.components().any(|c| c.as_os_str() == "__pycache__")
        {
            continue;
        }
        files.push((
            format!("{module_dir}/{}", slash(&file)),
            Content::Copy(wrapper.join(&file)),
        ));
    }
    files.push((
        "pyproject.toml".into(),
        Content::Text(pyproject(manifest, name, version, module)),
    ));
    files.push((
        ".gitignore".into(),
        Content::Text("target/\ndist/\n*.so\n*.pyd\n__pycache__/\n".into()),
    ));
    Ok(())
}

fn pyproject(manifest: &Manifest, name: &str, version: &str, module: &str) -> String {
    format!(
        "[build-system]\nrequires = [\"maturin>=1.7,<2\"]\nbuild-backend = \"maturin\"\n\n\
         [project]\nname = \"{name}\"\nversion = \"{version}\"\n\
         description = \"Rust port of `{unit}`, generated by rust-anything\"\n\
         requires-python = \">=3.9\"\n\
         classifiers = [\"Programming Language :: Rust\", \"Programming Language :: Python :: Implementation :: CPython\"]\n\n\
         [tool.maturin]\nbindings = \"pyo3\"\nmodule-name = \"{module}._native\"\npython-source = \"python\"\n",
        unit = manifest.unit
    )
}

#[allow(clippy::too_many_arguments)]
fn napi_files(
    crate_dir: &Path,
    manifest: &Manifest,
    name: &str,
    version: &str,
    opts: &Options,
    out: &Path,
    files: &mut Vec<(String, Content)>,
    notes: &mut Vec<String>,
) -> Result<()> {
    let lib = manifest.lib_name();
    let index = std::fs::read_to_string(crate_dir.join("index.cjs")).with_context(|| {
        format!(
            "reading index.cjs (run `rust-anything refresh {}`)",
            crate_dir.display()
        )
    })?;
    let load = format!("require('./{lib}.node')");
    if !index.contains(&load) {
        bail!(
            "index.cjs does not load `./{lib}.node`; run `rust-anything refresh {}` to regenerate it",
            crate_dir.display()
        );
    }
    files.push((
        "binding.cjs".into(),
        Content::Text(index.replacen(&load, "require('./native.cjs').load().binding", 1)),
    ));
    files.push(("native.cjs".into(), Content::Text(native_cjs(name, &lib))));

    let mut shipped = vec!["index.cjs", "binding.cjs", "native.cjs"];
    let fallback = match &opts.fallback {
        Some(spec) => Some(spec.clone()),
        None => match original_fallback(manifest) {
            Ok((file, path)) => {
                files.push((format!("fallback/{file}"), Content::Copy(path)));
                shipped.push("fallback/");
                Some(format!("./fallback/{file}"))
            }
            Err(why) => {
                notes.push(format!(
                    "No fallback: {why}. On a platform without a binary the package throws; pass \
                     --fallback <module> (e.g. the original npm package) to load it instead."
                ));
                None
            }
        },
    };
    files.push((
        "index.cjs".into(),
        Content::Text(index_cjs(name, fallback.as_deref())),
    ));

    let mut optional = serde_json::Map::new();
    for p in PLATFORMS {
        let pkg = format!("{name}-{}", p.key);
        optional.insert(pkg.clone(), json!(version));
        let node = format!("{lib}.{}.node", p.key);
        let mut platform = json!({
            "name": pkg,
            "version": version,
            "description": format!("{} binary of {name}, generated by rust-anything", p.target),
            "main": node,
            "files": [node],
            "os": [p.os],
            "cpu": [p.cpu],
        });
        if let Some(libc) = p.libc {
            platform["libc"] = json!([libc]);
        }
        files.push((
            format!("npm/{}/package.json", p.key),
            Content::Text(serde_json::to_string_pretty(&platform)? + "\n"),
        ));
    }
    let main = json!({
        "name": name,
        "version": version,
        "description": format!("Rust port of `{}`, generated by rust-anything", manifest.unit),
        "main": "index.cjs",
        "files": shipped,
        "optionalDependencies": optional,
    });
    files.push((
        "package.json".into(),
        Content::Text(serde_json::to_string_pretty(&main)? + "\n"),
    ));
    files.push((
        ".gitignore".into(),
        Content::Text("target/\ntarballs/\nnpm-built/\n*.node\n*.tgz\n".into()),
    ));

    let built = crate_dir.join(format!("{lib}.node"));
    match host_key() {
        Some(key) if built.exists() => {
            files.push((
                format!("npm/{key}/{lib}.{key}.node"),
                Content::Copy(built.clone()),
            ));
            if crate::harness::sources_newer_than(crate_dir, &built) {
                notes.push(format!(
                    "{} is older than the crate's sources; run `rust-anything build {}` and \
                     package again",
                    built.display(),
                    crate_dir.display()
                ));
            }
            notes.push(format!(
                "This machine's binary is in npm/{key}/. To try it: npm pack {0}/npm/{key} and \
                 npm pack {0}, then npm install both tarballs.",
                out.display()
            ));
        }
        Some(_) => notes.push(format!(
            "No {lib}.node in the crate: run `rust-anything build {}` and package again to try \
             the package on this machine.",
            crate_dir.display()
        )),
        None => {}
    }
    Ok(())
}

/// The original unit as a file the package can ship and require: one JS file that imports
/// nothing.
fn original_fallback(manifest: &Manifest) -> Result<(String, PathBuf), String> {
    let path = manifest.root.join(&manifest.unit);
    if !path.is_file() {
        return Err(format!(
            "the original `{}` is not a single file in {}",
            manifest.unit,
            manifest.root.display()
        ));
    }
    let file = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    if ["ts", "mts", "cts", "tsx"]
        .iter()
        .any(|e| file.ends_with(&format!(".{e}")))
    {
        return Err(format!(
            "`{file}` is TypeScript, and Node does not strip types under node_modules"
        ));
    }
    if !["js", "cjs", "mjs"]
        .iter()
        .any(|e| file.ends_with(&format!(".{e}")))
    {
        return Err(format!("`{file}` is not a JS file"));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let imports = text.lines().any(|l| {
        let l = l.trim_start();
        l.starts_with("import ")
            || l.starts_with("import{")
            || (l.starts_with("export ") && l.contains(" from "))
            || l.contains("require(")
            || l.contains("import(")
    });
    if imports {
        return Err(format!("`{file}` imports other modules"));
    }
    Ok((file, path))
}

fn index_cjs(name: &str, fallback: Option<&str>) -> String {
    let otherwise = match fallback {
        Some(spec) => format!("  module.exports = require({})\n", js_string(spec)),
        None => format!(
            "  throw new Error(\n    `{name} has no Rust build for ${{key}} (built for: ${{supported.join(', ')}})` +\n      (errors.length ? `:\\n${{errors.join('\\n')}}` : '')\n  )\n"
        ),
    };
    format!(
        "'use strict'\n// Generated by `rust-anything package`: the Rust port for this platform, or the original.\n\
         const {{ load, supported }} = require('./native.cjs')\nconst {{ binding, key, errors }} = load()\n\
         if (binding && process.env.RUST_ANYTHING_MODE !== 'old') {{\n  module.exports = require('./binding.cjs')\n}} else {{\n{otherwise}}}\n"
    )
}

fn native_cjs(name: &str, lib: &str) -> String {
    let keys: Vec<String> = PLATFORMS.iter().map(|p| format!("'{}'", p.key)).collect();
    format!(
        r#"'use strict'
// Generated by `rust-anything package`: finds the binary for this platform.
const {{ platform, arch }} = process

function abi() {{
  if (platform === 'win32') return '-msvc'
  if (platform !== 'linux') return ''
  try {{
    return process.report.getReport().header.glibcVersionRuntime ? '-gnu' : '-musl'
  }} catch {{
    return '-gnu'
  }}
}}

const key = `${{platform}}-${{arch}}${{abi()}}`
const supported = [{keys}]
let loaded = null

function load() {{
  if (loaded) return loaded
  const errors = []
  let binding = null
  if (supported.includes(key)) {{
    for (const id of [`./npm/${{key}}/{lib}.${{key}}.node`, `{name}-${{key}}`]) {{
      try {{
        binding = require(id)
        break
      }} catch (e) {{
        errors.push(`${{id}}: ${{e.message.split('\n')[0]}}`)
      }}
    }}
  }}
  loaded = {{ binding, key, errors }}
  return loaded
}}

module.exports = {{ load, key, supported }}
"#,
        keys = keys.join(", ")
    )
}

fn js_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "''".into())
}

fn wasm_files(
    crate_dir: &Path,
    manifest: &Manifest,
    name: &str,
    version: &str,
    files: &mut Vec<(String, Content)>,
) -> Result<()> {
    let pkg = crate_dir.join("pkg");
    let lib = manifest.lib_name();
    if !pkg.join(format!("{lib}.js")).exists() {
        bail!(
            "no wasm build in {}; run `rust-anything build {}` first",
            pkg.display(),
            crate_dir.display()
        );
    }
    files.push((
        "index.cjs".into(),
        Content::Copy(crate_dir.join("index.cjs")),
    ));
    for file in tree(&pkg)? {
        let file_name = file.file_name().unwrap_or_default().to_string_lossy();
        if file_name == ".gitignore" || file_name == "package.json" {
            continue;
        }
        files.push((
            format!("pkg/{}", slash(&file)),
            Content::Copy(pkg.join(&file)),
        ));
    }
    let main = json!({
        "name": name,
        "version": version,
        "description": format!("Rust port of `{}` (WebAssembly), generated by rust-anything", manifest.unit),
        "main": "index.cjs",
        "types": format!("pkg/{lib}.d.ts"),
        "files": ["index.cjs", "pkg/"],
    });
    files.push((
        "package.json".into(),
        Content::Text(serde_json::to_string_pretty(&main)? + "\n"),
    ));
    Ok(())
}

/// The GitHub Actions workflow, written inside the output folder; GitHub runs workflows only
/// from the repository's `.github/workflows`, so the note says where to move it.
fn workflow_file(
    manifest: &Manifest,
    name: &str,
    version: &str,
    crate_dir: &Path,
    out: &Path,
) -> Result<((String, Content), String)> {
    let repo = git_root(out);
    let out_rel = repo
        .as_deref()
        .and_then(|r| relative(r, out))
        .unwrap_or_else(|| ".".into());
    let slug: String = name
        .trim_start_matches('@')
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let (file, text) = match manifest.kind {
        Kind::Pyo3 => (
            format!("{slug}-wheels.yml"),
            wheels_yml(name, version, &python_module(manifest), &out_rel),
        ),
        _ => {
            let crate_rel = relative(out, crate_dir).context("relating the crate to --out")?;
            (
                format!("{slug}-npm.yml"),
                npm_yml(name, version, &manifest.lib_name(), &out_rel, &crate_rel),
            )
        }
    };
    let path = format!(".github/workflows/{file}");
    let note = match &repo {
        Some(repo) => format!(
            "CI: move {path} to {}/.github/workflows/ (GitHub runs workflows only from there); it \
             builds every platform and uploads the packages as artifacts.",
            repo.display()
        ),
        None => format!(
            "CI: {path} assumes {} is the repository root; move it to the repository's \
             .github/workflows/ and set `working-directory` to this folder.",
            out.display()
        ),
    };
    Ok(((path, Content::Text(text)), note))
}

fn wheels_yml(name: &str, version: &str, module: &str, dir: &str) -> String {
    format!(
        r#"# Generated by `rust-anything package`: abi3 wheels of {name} {version} (one per platform,
# every CPython from 3.9 on) and an sdist, uploaded as artifacts. Nothing is published.
name: {name} wheels

on:
  workflow_dispatch:
  push:
    tags: ['v*']

defaults:
  run:
    shell: bash
    working-directory: {dir}

jobs:
  wheels:
    name: ${{{{ matrix.target }}}}
    runs-on: ${{{{ matrix.os }}}}
    strategy:
      fail-fast: false
      matrix:
        include:
          - {{ os: macos-latest, target: aarch64-apple-darwin, check: true }}
          - {{ os: macos-latest, target: x86_64-apple-darwin, check: false }}
          - {{ os: ubuntu-latest, target: x86_64-unknown-linux-gnu, check: true }}
          - {{ os: ubuntu-latest, target: aarch64-unknown-linux-gnu, check: false }}
          - {{ os: windows-latest, target: x86_64-pc-windows-msvc, check: true }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.12'
      - uses: PyO3/maturin-action@v1
        with:
          target: ${{{{ matrix.target }}}}
          manylinux: auto
          args: --release --out dist
          working-directory: {dir}
      - name: Import check
        if: matrix.check
        run: |
          python -m pip install {name} --no-index --find-links dist --force-reinstall
          python -c "import {module}"
      - uses: actions/upload-artifact@v4
        with:
          name: wheel-${{{{ matrix.target }}}}
          path: {dir}/dist

  sdist:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: PyO3/maturin-action@v1
        with:
          command: sdist
          args: --out dist
          working-directory: {dir}
      - uses: actions/upload-artifact@v4
        with:
          name: sdist
          path: {dir}/dist
"#
    )
}

fn npm_yml(name: &str, version: &str, lib: &str, dir: &str, crate_rel: &str) -> String {
    let matrix: String = PLATFORMS
        .iter()
        .map(|p| {
            let file = match p.os {
                "darwin" => format!("lib{lib}.dylib"),
                "linux" => format!("lib{lib}.so"),
                _ => format!("{lib}.dll"),
            };
            format!(
                "          - {{ key: {}, target: {}, os: {}, lib: {file}, check: {} }}\n",
                p.key, p.target, p.runner, p.native
            )
        })
        .collect();
    format!(
        r#"# Generated by `rust-anything package`: the {name} {version} npm packages (one per
# platform, plus the main one), packed and uploaded as artifacts. Nothing is published.
name: {name} npm packages

on:
  workflow_dispatch:
  push:
    tags: ['v*']

defaults:
  run:
    shell: bash
    working-directory: {dir}

jobs:
  build:
    name: ${{{{ matrix.key }}}}
    runs-on: ${{{{ matrix.os }}}}
    strategy:
      fail-fast: false
      matrix:
        include:
{matrix}    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
      - run: rustup target add ${{{{ matrix.target }}}}
      - run: cargo build --release --target ${{{{ matrix.target }}}} --manifest-path {crate_rel}/Cargo.toml --target-dir target
      - run: cp target/${{{{ matrix.target }}}}/release/${{{{ matrix.lib }}}} npm/${{{{ matrix.key }}}}/{lib}.${{{{ matrix.key }}}}.node
      - name: Load check
        if: matrix.check
        run: node -e "const r = require('./native.cjs').load(); if (!r.binding) throw new Error(r.errors.join('\n'))"
      - uses: actions/upload-artifact@v4
        with:
          name: npm-${{{{ matrix.key }}}}
          path: {dir}/npm/${{{{ matrix.key }}}}/*.node

  pack:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
      - uses: actions/download-artifact@v4
        with:
          pattern: npm-*
          path: {dir}/npm-built
      - run: |
          for built in npm-built/npm-*; do cp "$built"/*.node "npm/${{built##*/npm-}}/"; done
          mkdir -p tarballs
          for platform in npm/*/; do npm pack "./$platform" --pack-destination tarballs; done
          npm pack --pack-destination tarballs
      - uses: actions/upload-artifact@v4
        with:
          name: npm-tarballs
          path: {dir}/tarballs
"#
    )
}

fn git_root(from: &Path) -> Option<PathBuf> {
    let mut dir = from.to_path_buf();
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// `to` relative to `from`, with `/` separators; `.` when they are the same folder.
fn relative(from: &Path, to: &Path) -> Option<String> {
    let from: Vec<Component> = from.components().collect();
    let to: Vec<Component> = to.components().collect();
    if from.first() != to.first() {
        return None;
    }
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = vec!["..".into(); from.len() - common];
    parts.extend(
        to[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().to_string()),
    );
    Some(if parts.is_empty() {
        ".".into()
    } else {
        parts.join("/")
    })
}

/// Every file under `dir`, relative to it, sorted.
fn tree(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(rel) = stack.pop() {
        let entries = std::fs::read_dir(dir.join(&rel))
            .with_context(|| format!("reading {}", dir.display()))?;
        for entry in entries {
            let entry = entry?;
            let path = rel.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn slash(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths() {
        assert_eq!(
            relative(Path::new("/r/a/package"), Path::new("/r/a")).as_deref(),
            Some("..")
        );
        assert_eq!(
            relative(Path::new("/r"), Path::new("/r/rust/x/package")).as_deref(),
            Some("rust/x/package")
        );
        assert_eq!(
            relative(Path::new("/r"), Path::new("/r")).as_deref(),
            Some(".")
        );
    }

    #[test]
    fn npm_names() {
        assert!(validate_npm_name("fuzzy-rs").is_ok());
        assert!(validate_npm_name("@x18n/fuzzy").is_ok());
        assert!(validate_npm_name("Fuzzy").is_err());
        assert!(validate_npm_name("@x18n/").is_err());
        // npm applies the `.`/`_` rule to each segment, and a scope is `@scope/name` exactly.
        assert!(validate_npm_name("@x18n/.hidden").is_err());
        assert!(validate_npm_name("@x18n/_private").is_err());
        assert!(validate_npm_name("@x18n/fuzzy/extra").is_err());
        assert!(validate_npm_name("@x18n").is_err());
        assert!(validate_npm_name("@/fuzzy").is_err());
        assert!(validate_npm_name("fuzzy/extra").is_err());
        assert!(validate_npm_name("fuzzy@1").is_err());
        assert!(validate_npm_name("@x18n/fuzzy@1").is_err());
    }

    #[test]
    fn every_platform_has_a_unique_key() {
        let mut keys: Vec<&str> = PLATFORMS.iter().map(|p| p.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), PLATFORMS.len());
    }

    #[test]
    fn stamp_paths_stay_inside() {
        assert!(safe_relative("npm/darwin-arm64/package.json"));
        assert!(!safe_relative("../x"));
        assert!(!safe_relative("/etc/passwd"));
    }
}
