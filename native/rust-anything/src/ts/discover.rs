//! Finds the packages and source files of a JS/TS repository.

use crate::walk::Canonical;
use std::path::Path;

use anyhow::{Context, Result};

/// A folder with a `package.json`.
#[derive(Clone, Debug)]
pub struct Package {
    pub name: String,
    /// Path relative to the root, `/`-separated; empty for the root.
    pub dir: String,
    /// `dir/src` when it exists, else `dir`.
    pub src_root: String,
    /// Runtime dependencies (`dependencies`, `peerDependencies`, `optionalDependencies`).
    pub deps: Vec<String>,
    pub dev_deps: Vec<String>,
    /// Namespaces the package's own code declares (PHP's PSR-4 `autoload` and `autoload-dev`
    /// prefixes); empty for the other languages.
    pub namespaces: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Source,
    Test,
    /// Components and single-file components: flagged, not ported.
    Ui,
}

#[derive(Clone, Debug)]
pub struct SourceFile {
    /// Path relative to the root, `/`-separated.
    pub path: String,
    pub package: usize,
    pub kind: FileKind,
}

#[derive(Debug)]
pub struct Workspace {
    pub packages: Vec<Package>,
    pub files: Vec<SourceFile>,
}

/// Folders that never hold product code.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    "out",
    "target",
    "coverage",
    "fixtures",
    "__fixtures__",
    "__snapshots__",
    "playground",
    "examples",
];
pub(crate) const SOURCE_EXTS: &[&str] = &["ts", "mts", "cts", "js", "mjs", "cjs"];
pub(crate) const UI_EXTS: &[&str] = &["tsx", "jsx", "vue", "svelte", "astro"];
const TEST_DIRS: &[&str] = &["test", "tests", "__tests__", "e2e"];

pub fn discover(root: &Path) -> Result<Workspace> {
    let mut manifests = Vec::new();
    let mut candidates = Vec::new();
    let walker = crate::walk::filtered(root, |entry| {
        !(entry.file_type().is_some_and(|t| t.is_dir())
            && SKIP_DIRS.contains(&entry.file_name().to_string_lossy().as_ref()))
    })
    .hidden(true)
    .require_git(false)
    .build();
    for entry in walker {
        let entry = entry?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if rel == "package.json" || rel.ends_with("/package.json") {
            manifests.push(rel);
        } else if let Some(kind) = classify(&rel) {
            candidates.push((rel, kind));
        }
    }

    let mut packages = manifests
        .iter()
        .map(|rel| read_package(root, rel))
        .collect::<Result<Vec<_>>>()?;
    if !packages.iter().any(|p| p.dir.is_empty()) {
        let name = root
            .canonical()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "root".into());
        packages.push(Package {
            src_root: if root.join("src").is_dir() {
                "src".into()
            } else {
                String::new()
            },
            name,
            dir: String::new(),
            deps: Vec::new(),
            dev_deps: Vec::new(),
            namespaces: Vec::new(),
        });
    }
    packages.sort_by(|a, b| a.dir.cmp(&b.dir));

    candidates.sort_by(|a, b| a.0.cmp(&b.0));
    let files = candidates
        .into_iter()
        .map(|(path, kind)| SourceFile {
            package: owning_package(&packages, &path),
            path,
            kind,
        })
        .collect();
    Ok(Workspace { packages, files })
}

/// What a file is, or `None` when it is not code we analyze.
fn classify(rel: &str) -> Option<FileKind> {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let ext = name.rsplit_once('.').map(|(_, e)| e)?;
    if name.ends_with(".d.ts") || name.ends_with(".d.mts") || name.ends_with(".d.cts") {
        return None;
    }
    // Tool configs (`vite.config.ts`, `eslint.config.js`) are not product code.
    if name.contains(".config.") || name.starts_with('.') {
        return None;
    }
    let is_test = name.contains(".test.")
        || name.contains(".spec.")
        || rel.split('/').any(|part| TEST_DIRS.contains(&part));
    if SOURCE_EXTS.contains(&ext) {
        Some(if is_test {
            FileKind::Test
        } else {
            FileKind::Source
        })
    } else if UI_EXTS.contains(&ext) {
        Some(if is_test {
            FileKind::Test
        } else {
            FileKind::Ui
        })
    } else {
        None
    }
}

fn read_package(root: &Path, rel: &str) -> Result<Package> {
    let text = std::fs::read_to_string(root.join(rel)).with_context(|| format!("reading {rel}"))?;
    let json: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {rel}"))?;
    let dir = rel
        .strip_suffix("package.json")
        .unwrap_or("")
        .trim_end_matches('/')
        .to_string();
    let keys = |field: &str| -> Vec<String> {
        json.get(field)
            .and_then(|v| v.as_object())
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default()
    };
    let mut deps = keys("dependencies");
    deps.extend(keys("peerDependencies"));
    deps.extend(keys("optionalDependencies"));
    let src = join(&dir, "src");
    Ok(Package {
        name: json.get("name").and_then(|v| v.as_str()).map_or_else(
            || {
                if dir.is_empty() {
                    "root".into()
                } else {
                    dir.clone()
                }
            },
            str::to_owned,
        ),
        src_root: if root.join(&src).is_dir() {
            src
        } else {
            dir.clone()
        },
        dir,
        deps,
        dev_deps: keys("devDependencies"),
        namespaces: Vec::new(),
    })
}

/// The package whose folder is the longest prefix of `path`.
fn owning_package(packages: &[Package], path: &str) -> usize {
    packages
        .iter()
        .enumerate()
        .filter(|(_, p)| p.dir.is_empty() || path.starts_with(&format!("{}/", p.dir)))
        .max_by_key(|(_, p)| p.dir.len())
        .map_or(0, |(i, _)| i)
}

pub fn join(dir: &str, rest: &str) -> String {
    if dir.is_empty() {
        rest.to_string()
    } else {
        format!("{dir}/{rest}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_files() {
        assert_eq!(classify("src/a.ts"), Some(FileKind::Source));
        assert_eq!(classify("src/a.test.ts"), Some(FileKind::Test));
        assert_eq!(classify("test/_utils.ts"), Some(FileKind::Test));
        assert_eq!(classify("src/App.vue"), Some(FileKind::Ui));
        assert_eq!(classify("src/types.d.ts"), None);
        assert_eq!(classify("vite.config.ts"), None);
        assert_eq!(classify("README.md"), None);
    }
}
