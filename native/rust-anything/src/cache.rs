//! One cargo target folder for every port and bridge, so dependencies compile once.
//!
//! The folder is `CARGO_TARGET_DIR` when set, else `rust-anything/target` in the user's cache
//! folder. Cargo names a crate's library after its package name alone, so two crates with one
//! name would overwrite each other's artifact there. Each package name therefore belongs to one
//! crate folder (`owners.json`, written under a file lock); another crate with that name builds
//! into its own `target/` as before. Built libraries are copied to the crate's own
//! `target/release`, where every loader reads them. Front doors keep their own `target/`.

use std::collections::BTreeMap;
use std::io::{Read as _, Seek as _, Write as _};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

const OWNERS: &str = "owners.json";
const LOCK: &str = "owners.lock";

/// The shared target folder in effect.
pub fn shared_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("CARGO_TARGET_DIR").filter(|d| !d.is_empty()) {
        let dir = PathBuf::from(dir);
        return std::path::absolute(&dir).unwrap_or(dir);
    }
    user_cache().join("rust-anything").join("target")
}

fn user_cache() -> PathBuf {
    let home = || {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map_or_else(std::env::temp_dir, PathBuf::from)
    };
    if cfg!(windows) {
        return std::env::var_os("LOCALAPPDATA").map_or_else(home, PathBuf::from);
    }
    if cfg!(target_os = "macos") {
        return home().join("Library").join("Caches");
    }
    std::env::var_os("XDG_CACHE_HOME")
        .filter(|d| !d.is_empty())
        .map_or_else(|| home().join(".cache"), PathBuf::from)
}

/// Where cargo builds this crate: the shared folder, or the crate's own `target/` when another
/// crate with the same package name owns that name there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub dir: PathBuf,
    pub shared: bool,
}

pub fn target_for(crate_dir: &Path) -> Result<Target> {
    target_in(&shared_dir(), crate_dir)
}

pub fn target_in(shared: &Path, crate_dir: &Path) -> Result<Target> {
    let own = Target {
        dir: crate_dir.join("target"),
        shared: false,
    };
    let Some(name) = package_name(crate_dir) else {
        return Ok(own);
    };
    let me = std::path::absolute(crate_dir)?;
    let me = me.canonicalize().unwrap_or(me);
    std::fs::create_dir_all(shared).with_context(|| format!("creating {}", shared.display()))?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(shared.join(LOCK))
        .with_context(|| format!("opening {}", shared.join(LOCK).display()))?;
    lock.lock()?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(shared.join(OWNERS))?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    let mut owners: BTreeMap<String, PathBuf> = serde_json::from_str(&text).unwrap_or_default();
    owners.retain(|n, dir| package_name(dir).as_deref() == Some(n.as_str()));
    let owner = owners.entry(name).or_insert_with(|| me.clone()).clone();
    let text = serde_json::to_string_pretty(&owners)? + "\n";
    file.set_len(0)?;
    file.rewind()?;
    file.write_all(text.as_bytes())?;
    drop(file);
    lock.unlock()?;
    Ok(if owner == me {
        Target {
            dir: shared.to_path_buf(),
            shared: true,
        }
    } else {
        own
    })
}

fn package_name(crate_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(crate_dir.join("Cargo.toml")).ok()?;
    let doc: toml_edit::DocumentMut = text.parse().ok()?;
    doc.get("package")?
        .get("name")?
        .as_str()
        .map(str::to_string)
}

/// Copies the crate's built libraries (`lib<name>.so`, `.dylib`, `.a`, `<name>.dll`, ...) from
/// `release` into the crate's own `target/release`, the path every loader reads. Returns that
/// folder.
pub fn publish(target: &Target, crate_dir: &Path, lib: &str, release: &Path) -> Result<PathBuf> {
    let own = crate_dir.join("target").join("release");
    if !target.shared {
        return Ok(own);
    }
    std::fs::create_dir_all(&own)?;
    for name in [
        format!("lib{lib}.so"),
        format!("lib{lib}.dylib"),
        format!("lib{lib}.a"),
        format!("{lib}.dll"),
        format!("{lib}.dll.lib"),
        format!("{lib}.lib"),
    ] {
        let from = release.join(&name);
        if from.is_file() {
            crate::harness::install(&from, &own.join(&name))?;
        }
    }
    Ok(own)
}

#[derive(Debug, Serialize)]
pub struct Cleaned {
    pub dir: PathBuf,
    pub bytes: u64,
    pub removed: bool,
    /// `CARGO_TARGET_DIR` named the folder: other cargo projects may build there too.
    pub from_env: bool,
}

/// Removes the shared target folder (or only measures it with `dry_run`). A folder named by
/// `CARGO_TARGET_DIR` is removed only with `force`, since other projects may use it.
pub fn clean(dry_run: bool, force: bool) -> Result<Cleaned> {
    let dir = shared_dir();
    let from_env = std::env::var_os("CARGO_TARGET_DIR").is_some_and(|d| !d.is_empty());
    let bytes = size(&dir);
    let removed = !dry_run && (!from_env || force) && dir.exists();
    if removed {
        std::fs::remove_dir_all(&dir).with_context(|| format!("removing {}", dir.display()))?;
    }
    Ok(Cleaned {
        dir,
        bytes,
        removed,
        from_env,
    })
}

fn size(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    std::fs::read_dir(path)
        .map(|entries| entries.flatten().map(|e| size(&e.path())).sum())
        .unwrap_or(0)
}

pub fn human(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rust-anything-cache-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    fn krate(dir: &Path, name: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
        )
        .unwrap();
    }

    #[test]
    fn a_package_name_builds_in_the_shared_folder_for_one_crate_only() {
        let root = scratch("owners");
        let shared = root.join("shared");
        let (a, b, c) = (root.join("a"), root.join("b"), root.join("c"));
        krate(&a, "fuzzy-rs");
        krate(&b, "fuzzy-rs");
        krate(&c, "slug-rs");

        let first = target_in(&shared, &a).unwrap();
        assert_eq!(
            (first.dir.as_path(), first.shared),
            (shared.as_path(), true)
        );
        assert_eq!(target_in(&shared, &a).unwrap(), first);
        let copy = target_in(&shared, &b).unwrap();
        assert_eq!((copy.dir, copy.shared), (b.join("target"), false));
        assert!(target_in(&shared, &c).unwrap().shared);

        std::fs::remove_dir_all(&a).unwrap();
        assert!(
            target_in(&shared, &b).unwrap().shared,
            "a's name is free again"
        );
        let owners = std::fs::read_to_string(shared.join(OWNERS)).unwrap();
        assert!(
            !owners.contains(&a.to_string_lossy().into_owned()),
            "{owners}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_crate_without_a_manifest_keeps_its_own_target() {
        let root = scratch("no-manifest");
        let t = target_in(&root.join("shared"), &root.join("x")).unwrap();
        assert_eq!((t.dir, t.shared), (root.join("x/target"), false));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn publish_copies_the_libraries_to_the_crates_own_folder() {
        let root = scratch("publish");
        let release = root.join("shared/release");
        std::fs::create_dir_all(&release).unwrap();
        std::fs::write(release.join("libfuzzy_rs.a"), "a").unwrap();
        std::fs::write(release.join("libfuzzy_rs.dylib"), "d").unwrap();
        std::fs::write(release.join("libother.dylib"), "o").unwrap();
        let crate_dir = root.join("crate");
        let target = Target {
            dir: root.join("shared"),
            shared: true,
        };
        let own = publish(&target, &crate_dir, "fuzzy_rs", &release).unwrap();
        assert_eq!(own, crate_dir.join("target/release"));
        assert_eq!(
            std::fs::read_to_string(own.join("libfuzzy_rs.dylib")).unwrap(),
            "d"
        );
        assert!(own.join("libfuzzy_rs.a").is_file());
        assert!(!own.join("libother.dylib").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sizes_read_as_people_write_them() {
        assert_eq!(human(512), "512 B");
        assert_eq!(human(1536), "1.5 KB");
        assert_eq!(human(3 * 1024 * 1024 * 1024), "3.0 GB");
    }
}
