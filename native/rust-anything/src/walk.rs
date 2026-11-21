//! Directory walks that read `.gitignore` the way git does: from the repository root down to the
//! scanned folder and below it, never from folders above the repository (a `*` in a home-folder
//! `.gitignore` would otherwise hide every file).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ignore::gitignore::Gitignore;
use ignore::{DirEntry, WalkBuilder};

/// `.gitignore` files in the folders between the repository root and the scanned folder.
struct Outer {
    root: PathBuf,
    base: PathBuf,
    matchers: Vec<Gitignore>,
}

impl Outer {
    fn new(root: &Path) -> Option<Self> {
        let base = root.canonical().ok()?;
        if base.join(".git").exists() {
            return None;
        }
        let repo = base.ancestors().skip(1).find(|d| d.join(".git").exists())?;
        let mut matchers: Vec<Gitignore> = base
            .ancestors()
            .skip(1)
            .take_while(|d| d.starts_with(repo))
            .filter_map(|d| {
                let file = d.join(".gitignore");
                file.is_file().then(|| Gitignore::new(file).0)
            })
            .collect();
        matchers.reverse();
        (!matchers.is_empty()).then(|| Self {
            root: root.to_path_buf(),
            base,
            matchers,
        })
    }

    fn ignores(&self, entry: &DirEntry) -> bool {
        if entry.depth() == 0 {
            return false;
        }
        let rel = entry
            .path()
            .strip_prefix(&self.root)
            .unwrap_or(entry.path());
        let path = self.base.join(rel);
        let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
        for m in self.matchers.iter().rev() {
            let found = m.matched(&path, is_dir);
            if found.is_ignore() {
                return true;
            }
            if found.is_whitelist() {
                return false;
            }
        }
        false
    }
}

/// `canonicalize` without Windows' `\\?\` verbatim prefix, which `java -cp`, `-D` library
/// paths and other tools reject. Elsewhere it is `canonicalize` itself.
pub trait Canonical {
    fn canonical(&self) -> std::io::Result<PathBuf>;
}

impl Canonical for Path {
    fn canonical(&self) -> std::io::Result<PathBuf> {
        let path = self.canonicalize()?;
        Ok(match path.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
            Some(rest) if !rest.starts_with(r"UNC\") => PathBuf::from(rest),
            _ => path,
        })
    }
}

/// A crate `bridge` generated inside the project: its wrappers reuse the original's names, so a
/// source walk that entered it would find every class or module twice.
fn bridge_crate(entry: &DirEntry) -> bool {
    entry.depth() > 0
        && entry.file_type().is_some_and(|t| t.is_dir())
        && entry.path().join(crate::bridge::MANIFEST).is_file()
}

/// A walk of `root` that keeps an entry when `keep` says so and git would not ignore it.
pub fn filtered<F>(root: &Path, keep: F) -> WalkBuilder
where
    F: Fn(&DirEntry) -> bool + Send + Sync + 'static,
{
    let mut walk = WalkBuilder::new(root);
    walk.parents(false);
    let outer = Outer::new(root).map(Arc::new);
    walk.filter_entry(move |e| {
        keep(e) && !bridge_crate(e) && !outer.as_ref().is_some_and(|o| o.ignores(e))
    });
    walk
}

/// A walk of `root` that skips only what git would ignore.
pub fn builder(root: &Path) -> WalkBuilder {
    filtered(root, |_| true)
}

/// Files with one of `extensions` under `root` that ignore rules exclude, outside folders that
/// never hold source (dependencies, build output, version control).
pub fn ignored_files(root: &Path, extensions: &[&str]) -> usize {
    const SKIP: [&str; 6] = ["node_modules", "target", "vendor", ".git", "dist", "build"];
    let has_ext = |p: &Path| {
        p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| extensions.contains(&e))
    };
    let all = WalkBuilder::new(root)
        .standard_filters(false)
        .hidden(true)
        .filter_entry(|e| !SKIP.contains(&e.file_name().to_string_lossy().as_ref()))
        .build()
        .flatten()
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()) && has_ext(e.path()))
        .count();
    let kept = builder(root)
        .hidden(true)
        .require_git(false)
        .build()
        .flatten()
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()) && has_ext(e.path()))
        .filter(|e| {
            !e.path()
                .components()
                .any(|c| SKIP.contains(&c.as_os_str().to_string_lossy().as_ref()))
        })
        .count();
    all.saturating_sub(kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(root: &Path) -> Vec<String> {
        let mut out: Vec<String> = builder(root)
            .hidden(true)
            .require_git(false)
            .build()
            .flatten()
            .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
            .map(|e| {
                e.path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn ignore_files_above_the_repository_are_not_read() {
        let tmp = std::env::temp_dir().join(format!("ra-walk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let repo = tmp.join("home/proj");
        std::fs::create_dir_all(repo.join("src/gen")).unwrap();
        std::fs::write(tmp.join("home/.gitignore"), "*\n").unwrap();
        std::fs::write(repo.join("src/a.ts"), "").unwrap();
        std::fs::write(repo.join("src/gen/b.ts"), "").unwrap();
        assert_eq!(
            files(&repo),
            ["src/a.ts", "src/gen/b.ts"],
            "not a repository"
        );

        std::fs::create_dir(repo.join(".git")).unwrap();
        std::fs::write(repo.join(".gitignore"), "/src/gen/\n").unwrap();
        assert_eq!(files(&repo), ["src/a.ts"], "repository root");
        assert_eq!(
            files(&repo.join("src")),
            ["a.ts"],
            "folder inside the repository"
        );

        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn bridge_crates_inside_the_project_are_not_source() {
        let tmp = std::env::temp_dir().join(format!("ra-walk-bridge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).unwrap();
        std::fs::create_dir_all(tmp.join("rust/x-rs/java")).unwrap();
        std::fs::write(tmp.join("src/A.java"), "").unwrap();
        std::fs::write(tmp.join("rust/x-rs/rust-anything.json"), "{}").unwrap();
        std::fs::write(tmp.join("rust/x-rs/java/A.java"), "").unwrap();
        assert_eq!(files(&tmp), ["src/A.java"]);
        assert!(
            files(&tmp.join("rust/x-rs")).contains(&"java/A.java".to_string()),
            "a walk of the crate itself still sees its files"
        );
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
