//! The libraries each source language imports and the Rust crates that replace them, from the
//! data table `crates.toml`: adding a mapping is a few lines there, not code.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::ts::deps::{CrateMatch, Fit};

const TABLE: &str = include_str!("crates.toml");

struct Entry {
    names: Vec<String>,
    prefixes: Vec<String>,
    crates: String,
    fit: Fit,
    note: String,
}

fn table() -> &'static BTreeMap<String, Vec<Entry>> {
    static PARSED: OnceLock<BTreeMap<String, Vec<Entry>>> = OnceLock::new();
    PARSED.get_or_init(|| parse(TABLE).unwrap_or_default())
}

fn parse(text: &str) -> Result<BTreeMap<String, Vec<Entry>>, String> {
    let doc: toml_edit::DocumentMut = text.parse().map_err(|e| format!("crates.toml: {e}"))?;
    let strings = |t: &toml_edit::Table, key: &str| -> Vec<String> {
        t.get(key)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut out = BTreeMap::new();
    for (lang, item) in doc.iter() {
        let tables = item
            .as_array_of_tables()
            .ok_or_else(|| format!("crates.toml: `{lang}` must be a list of [[{lang}]] entries"))?;
        let mut entries = Vec::new();
        for t in tables.iter() {
            let text = |key: &str| t.get(key).and_then(|v| v.as_str()).unwrap_or_default();
            let fit = match text("fit") {
                "good" => Fit::Good,
                "partial" => Fit::Partial,
                "none" => Fit::None,
                other => return Err(format!("crates.toml: [[{lang}]] fit `{other}`")),
            };
            entries.push(Entry {
                names: strings(t, "names"),
                prefixes: strings(t, "prefixes"),
                crates: text("crates").to_string(),
                fit,
                note: text("note").to_string(),
            });
        }
        out.insert(lang.to_string(), entries);
    }
    Ok(out)
}

/// The first entry for `lang` whose names hold `name` or whose prefixes start it.
pub fn find(lang: &str, name: &str) -> Option<CrateMatch> {
    table()
        .get(lang)?
        .iter()
        .find(|e| {
            e.names.iter().any(|n| n == name)
                || e.prefixes.iter().any(|p| name.starts_with(p.as_str()))
        })
        .map(|e| CrateMatch {
            crates: &e.crates,
            fit: e.fit,
            note: &e.note,
        })
}

/// The longest configured prefix of `lang` that starts `name`, if any.
pub fn longest_prefix(lang: &str, name: &str) -> Option<&'static str> {
    table()
        .get(lang)?
        .iter()
        .flat_map(|e| e.prefixes.iter().map(String::as_str))
        .filter(|p| name.starts_with(p))
        .max_by_key(|p| p.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_parses_and_every_entry_names_something() {
        let parsed = parse(TABLE).unwrap();
        assert_eq!(parsed.len(), 9);
        for (lang, entries) in &parsed {
            for e in entries {
                assert!(
                    !e.names.is_empty() || !e.prefixes.is_empty(),
                    "{lang}: {}",
                    e.crates
                );
                assert!(
                    !e.crates.is_empty() || e.fit == Fit::None,
                    "{lang}: {:?}",
                    e.names
                );
            }
        }
    }

    #[test]
    fn the_first_match_wins() {
        assert_eq!(find("ts", "json5").map(|m| m.crates), Some("json5"));
        assert_eq!(
            find("java", "org.slf4j.Logger").map(|m| m.crates),
            Some("tracing")
        );
        assert!(find("ts", "left-pad-but-new").is_none());
        assert!(find("nope", "json5").is_none());
    }

    #[test]
    fn ffmpeg_bindings_map_in_every_language() {
        for (lang, name) in [
            ("ts", "fluent-ffmpeg"),
            ("python", "av"),
            ("c", "<libavcodec/avcodec.h>"),
            ("go", "github.com/asticode/go-astiav"),
            ("java", "org.bytedeco.ffmpeg.global.avcodec"),
        ] {
            let m = find(lang, name).unwrap_or_else(|| panic!("{lang}: {name}"));
            assert_eq!(
                (m.crates, m.fit),
                ("ffmpeg-next", Fit::Partial),
                "{lang}: {name}"
            );
        }
        assert!(find("c", "<libavcodecx.h>").is_none());
    }
}
