//! npm packages and their Rust counterparts.
//!
//! The table is deliberately small and opinionated: it names the crate a port should reach
//! for first. A package missing here is reported as `unknown`, never guessed.

use serde::Serialize;

/// How well a Rust crate covers what the npm package does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fit {
    /// A maintained crate does the same job.
    Good,
    /// A crate covers the common case; some behavior needs checking or extra work.
    Partial,
    /// No Rust equivalent: keep this part in JS or call it across the bridge.
    None,
}

#[derive(Clone, Copy, Debug)]
pub struct CrateMatch {
    pub crates: &'static str,
    pub fit: Fit,
    pub note: &'static str,
    /// The table row that matched, within its language: two names with the same row are one
    /// library spelled two ways (`yaml` and `PyYAML`).
    pub row: usize,
}

/// The Rust counterpart of an npm package, if the table knows it.
pub fn lookup(package: &str) -> Option<CrateMatch> {
    crate::crates::find("ts", package)
}

/// The package part of a bare import specifier: `@scope/name/sub` -> `@scope/name`.
pub fn package_name(specifier: &str) -> &str {
    let mut parts = specifier.splitn(3, '/');
    let first = parts.next().unwrap_or(specifier);
    if first.starts_with('@') {
        match parts.next() {
            Some(second) => &specifier[..first.len() + 1 + second.len()],
            None => specifier,
        }
    } else {
        first
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_package_names() {
        assert_eq!(package_name("yaml"), "yaml");
        assert_eq!(package_name("svelte/compiler"), "svelte");
        assert_eq!(package_name("@x18n/core"), "@x18n/core");
        assert_eq!(package_name("@x18n/builtins/presets"), "@x18n/builtins");
    }

    #[test]
    fn knows_common_packages() {
        assert_eq!(lookup("json5").map(|m| m.fit), Some(Fit::Good));
        assert_eq!(lookup("jiti").map(|m| m.fit), Some(Fit::None));
        assert!(lookup("left-pad-but-new").is_none());
    }
}
