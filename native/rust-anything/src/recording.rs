//! What a recording was made from, and the differences from it a port is allowed to keep.
//!
//! `record` saves a fingerprint of the original's unit (`recorded-from.json`): when the original
//! changes later, `diff` and `gate` say the recording is stale instead of passing against old
//! behavior. `known-diffs.jsonl` lists recorded cases where the port is meant to differ, each
//! with the Rust result it gives and why; any other difference still fails, and so does an entry
//! that no longer happens, so the list cannot outlive the differences it explains.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::bridge::Manifest;
use crate::harness::DiffOutcome;

pub const RECORDED_FROM: &str = "recorded-from.json";
pub const KNOWN_DIFFS: &str = "known-diffs.jsonl";

/// FNV-1a over the unit's files (relative path and contents, in path order): stable across Rust
/// versions and platforms, unlike `DefaultHasher`.
pub fn fingerprint(m: &Manifest) -> Option<(String, usize)> {
    let unit = m.root.join(&m.unit);
    let mut files: Vec<std::path::PathBuf> = if unit.is_file() {
        vec![unit.clone()]
    } else if unit.is_dir() {
        crate::walk::filtered(&unit, |e| {
            !matches!(
                e.file_name().to_str(),
                Some("node_modules" | "target" | ".git" | "__pycache__")
            )
        })
        .build()
        .flatten()
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .map(|e| e.into_path())
        .collect()
    } else {
        return None;
    };
    files.sort();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for b in bytes {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for file in &files {
        let relative = file.strip_prefix(&m.root).unwrap_or(file);
        eat(relative.to_string_lossy().replace('\\', "/").as_bytes());
        eat(&[0]);
        eat(&std::fs::read(file).ok()?);
        eat(&[0]);
    }
    Some((format!("{hash:016x}"), files.len()))
}

/// Saves the fingerprint of the original next to `cases.jsonl`, with how much of it the recorded
/// cases ran when that was measured.
pub fn save_fingerprint(crate_dir: &Path, m: &Manifest, coverage: Option<&Value>) -> Result<()> {
    let path = crate_dir.join(RECORDED_FROM);
    let Some((hash, files)) = fingerprint(m) else {
        return Ok(());
    };
    let mut saved = json!({ "unit": m.unit, "files": files, "fingerprint": hash });
    if let Some(c) = coverage {
        saved["coverage"] = c.clone();
    }
    std::fs::write(&path, serde_json::to_string_pretty(&saved)? + "\n")
        .with_context(|| format!("writing {}", path.display()))
}

/// The coverage `record` saved: `{"lines", "reached", "files": [{"file", "lines", "reached",
/// "missed"}]}`.
pub fn saved_coverage(crate_dir: &Path) -> Option<Value> {
    let saved: Value =
        serde_json::from_str(&std::fs::read_to_string(crate_dir.join(RECORDED_FROM)).ok()?).ok()?;
    saved.get("coverage").filter(|c| c.is_object()).cloned()
}

/// One line for a person: how much of the original the recorded cases ran, and where they
/// did not reach.
pub fn coverage_line(c: &Value) -> Option<String> {
    let lines = c["lines"].as_u64()?;
    let reached = c["reached"].as_u64()?;
    if lines == 0 {
        return None;
    }
    let mut files: Vec<&Value> = c["files"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| f["missed"].as_str().is_some_and(|m| !m.is_empty()))
        .collect();
    files.sort_by_key(|f| {
        std::cmp::Reverse(f["lines"].as_u64().unwrap_or(0) - f["reached"].as_u64().unwrap_or(0))
    });
    let mut missed: Vec<String> = files
        .iter()
        .take(4)
        .map(|f| {
            format!(
                "{}: {}",
                f["file"].as_str().unwrap_or("?"),
                f["missed"].as_str().unwrap_or_default()
            )
        })
        .collect();
    if files.len() > 4 {
        missed.push(format!(
            "{} more files, listed in {RECORDED_FROM}",
            files.len() - 4
        ));
    }
    let mut line = format!(
        "the recorded cases ran {reached} of the original's {lines} lines ({:.0}%)",
        reached as f64 * 100.0 / lines as f64
    );
    if !missed.is_empty() {
        line.push_str(&format!(
            "; never ran: {} (add inputs that reach them, e.g. `record --from-tests`)",
            missed.join("; ")
        ));
    }
    Some(line)
}

/// Line coverage of the unit's files from the V8 coverage Node writes to `dir`
/// (`NODE_V8_COVERAGE`). A line counts when it has code; it was reached when any of its code ran
/// in any of the record runs. Files Node transformed (not only stripped of types) are left out,
/// since V8's offsets then point into the transformed text.
pub fn node_coverage(dir: &Path, m: &Manifest) -> Option<Value> {
    let mut unit: Vec<std::path::PathBuf> = m
        .exports
        .iter()
        .map(|f| m.root.join(&f.file))
        .chain(m.classes.iter().map(|c| m.root.join(&c.file)))
        .collect();
    unit.sort();
    unit.dedup();
    let root = m.root.canonicalize().unwrap_or_else(|_| m.root.clone());
    let mut files: Vec<std::path::PathBuf> = crate::gate::with_local_imports(&m.root, unit)
        .into_iter()
        .filter_map(|p| p.canonicalize().ok())
        .collect();
    files.sort();
    files.dedup();
    let reports: Vec<Value> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| serde_json::from_str(&std::fs::read_to_string(e.path()).ok()?).ok())
        .collect();
    if reports.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    let (mut total, mut total_reached) = (0u64, 0u64);
    for path in files {
        let file = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let real = &path;
        let units: usize = text.encode_utf16().count();
        let mut ran = vec![false; units];
        let mut seen = false;
        for report in &reports {
            let transformed = report["source-map-cache"]
                .as_object()
                .is_some_and(|c| c.keys().any(|url| same_file(url, real)));
            if transformed {
                continue;
            }
            for script in report["result"].as_array().into_iter().flatten() {
                if !script["url"].as_str().is_some_and(|u| same_file(u, real)) {
                    continue;
                }
                seen = true;
                let mut counts = vec![0u64; units];
                for f in script["functions"].as_array().into_iter().flatten() {
                    for r in f["ranges"].as_array().into_iter().flatten() {
                        let (Some(s), Some(e), Some(n)) = (
                            r["startOffset"].as_u64(),
                            r["endOffset"].as_u64(),
                            r["count"].as_u64(),
                        ) else {
                            continue;
                        };
                        let (s, e) = (s as usize, (e as usize).min(units));
                        if s < e {
                            counts[s..e].fill(n);
                        }
                    }
                }
                for (r, c) in ran.iter_mut().zip(&counts) {
                    *r |= *c > 0;
                }
            }
        }
        if !seen {
            continue;
        }
        let mut lines: Vec<(usize, bool, bool)> = Vec::new();
        let (mut line, mut code, mut hit) = (1usize, false, false);
        let mut at = 0usize;
        for ch in text.chars() {
            if ch == '\n' {
                lines.push((line, code, hit));
                (line, code, hit) = (line + 1, false, false);
            } else if !ch.is_whitespace() {
                code = true;
                hit |= ran.get(at).copied().unwrap_or(false);
            }
            at += ch.len_utf16();
        }
        lines.push((line, code, hit));
        let code_lines: Vec<(usize, bool)> = lines
            .into_iter()
            .filter(|(_, code, _)| *code)
            .map(|(n, _, hit)| (n, hit))
            .collect();
        let reached = code_lines.iter().filter(|(_, hit)| *hit).count() as u64;
        total += code_lines.len() as u64;
        total_reached += reached;
        out.push(json!({
            "file": file,
            "lines": code_lines.len(),
            "reached": reached,
            "missed": spans(code_lines.iter().filter(|(_, hit)| !hit).map(|(n, _)| *n)),
        }));
    }
    (!out.is_empty()).then(|| json!({ "lines": total, "reached": total_reached, "files": out }))
}

fn same_file(url: &str, real: &Path) -> bool {
    url.strip_prefix("file://")
        .map(|p| {
            percent_encoding::percent_decode_str(p)
                .decode_utf8_lossy()
                .into_owned()
        })
        .is_some_and(|p| Path::new(&p) == real)
}

fn spans(lines: impl Iterator<Item = usize>) -> String {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for n in lines {
        match out.last_mut() {
            Some((_, end)) if *end + 1 == n => *end = n,
            _ => out.push((n, n)),
        }
    }
    out.iter()
        .map(|(a, b)| {
            if a == b {
                a.to_string()
            } else {
                format!("{a}-{b}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Why the recording no longer describes the original, if it does not: the unit's files changed
/// since `record`. `None` when they did not, or when there is nothing to compare (a recording
/// made before fingerprints, or the original not at hand).
pub fn stale(crate_dir: &Path, m: &Manifest) -> Option<String> {
    let saved: Value =
        serde_json::from_str(&std::fs::read_to_string(crate_dir.join(RECORDED_FROM)).ok()?).ok()?;
    let (now, _) = fingerprint(m)?;
    (saved["fingerprint"].as_str()? != now).then(|| {
        format!(
            "{} changed since it was recorded: re-run `rust-anything record {}` so the port is \
             checked against what the original does now",
            m.unit,
            crate_dir.display()
        )
    })
}

const HISTORY: &str = ".rust-anything/diff-history.jsonl";
/// Changes to `port.rs` in a row that gain no matching case before `diff` calls the port stalled.
pub const STALL_AFTER: usize = 3;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Trend {
    /// Cases matching after each change to `port.rs`, oldest first (the last ten).
    pub passed: Vec<usize>,
    pub total: usize,
    /// The last `STALL_AFTER` changes gained nothing while cases still fail.
    pub stalled: bool,
}

/// Records this diff when `port.rs` changed since the last one, and returns the trend. Rerunning
/// `diff` on an unchanged port is not another attempt, so it adds nothing.
pub fn track(crate_dir: &Path, passed: usize, total: usize) -> Option<Trend> {
    let port = std::fs::read(crate_dir.join("src/port.rs")).ok()?;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in &port {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    let hash = format!("{hash:016x}");
    let path = crate_dir.join(HISTORY);
    let mut runs: Vec<Value> = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .filter(|r: &Value| r["total"].as_u64() == Some(total as u64))
        .collect();
    let entry = json!({ "port": hash, "passed": passed, "total": total });
    match runs.last_mut() {
        Some(last) if last["port"] == entry["port"] => *last = entry,
        _ => runs.push(entry),
    }
    let keep = runs.len().saturating_sub(10);
    runs.drain(..keep);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let text: String = runs.iter().map(|r| format!("{r}\n")).collect();
    let _ = std::fs::write(&path, text);
    Some(trend(
        runs.iter()
            .filter_map(|r| r["passed"].as_u64().map(|p| p as usize))
            .collect(),
        total,
    ))
}

pub fn last_diff(crate_dir: &Path) -> Option<(usize, usize)> {
    let text = std::fs::read_to_string(crate_dir.join(HISTORY)).ok()?;
    let last: Value = serde_json::from_str(text.lines().last()?).ok()?;
    Some((
        usize::try_from(last["passed"].as_u64()?).ok()?,
        usize::try_from(last["total"].as_u64()?).ok()?,
    ))
}

fn trend(passed: Vec<usize>, total: usize) -> Trend {
    let n = passed.len();
    let stalled = n > STALL_AFTER
        && passed[n - 1] < total
        && passed[n - STALL_AFTER..]
            .iter()
            .all(|p| *p <= passed[..n - STALL_AFTER].iter().copied().max().unwrap_or(0));
    Trend {
        passed,
        total,
        stalled,
    }
}

/// The entries of `known-diffs.jsonl`: `{"fn", "args", "actual", "reason"}` per line, `actual`
/// being the port's result as `diff --json` prints it. An entry without a reason is refused.
pub fn known_diffs(crate_dir: &Path) -> Result<Vec<Value>> {
    let path = crate_dir.join(KNOWN_DIFFS);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let mut entries = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let entry: Value = serde_json::from_str(line)
            .with_context(|| format!("{}:{}: not JSON", path.display(), i + 1))?;
        for key in ["fn", "args", "actual"] {
            anyhow::ensure!(
                entry.get(key).is_some(),
                "{}:{}: an entry needs `{key}`",
                path.display(),
                i + 1
            );
        }
        anyhow::ensure!(
            entry["reason"]
                .as_str()
                .is_some_and(|r| !r.trim().is_empty()),
            "{}:{}: an entry needs a `reason`: why the port is meant to differ here",
            path.display(),
            i + 1
        );
        entries.push(entry);
    }
    Ok(entries)
}

/// Counts the mismatches a known-diffs entry explains as passed (in `accepted`), and records the
/// entries no mismatch matched (in `stale_known`). A mismatch is explained only when the port
/// gives exactly the entry's `actual`: a different wrong result still fails.
pub fn apply_known(mut outcome: DiffOutcome, entries: &[Value]) -> DiffOutcome {
    if entries.is_empty() {
        return outcome;
    }
    let mut used = vec![false; entries.len()];
    outcome.mismatches.retain(|m| {
        let hit = entries.iter().position(|e| {
            e["fn"] == m["fn"] && e["args"] == m["args"] && e["actual"] == m["actual"]
        });
        match hit {
            Some(i) => {
                used[i] = true;
                false
            }
            None => true,
        }
    });
    let accepted = used.iter().filter(|u| **u).count();
    outcome.accepted = accepted;
    outcome.failed -= accepted.min(outcome.failed);
    outcome.passed += accepted;
    outcome.stale_known = entries
        .iter()
        .zip(&used)
        .filter(|(_, u)| !**u)
        .map(|(e, _)| json!({ "fn": e["fn"], "args": e["args"], "reason": e["reason"] }))
        .collect();
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome() -> DiffOutcome {
        DiffOutcome {
            total: 3,
            passed: 1,
            failed: 2,
            mismatches: vec![
                json!({"fn": "f", "args": [1], "expected": {"ok": 1}, "actual": {"ok": 2}}),
                json!({"fn": "f", "args": [2], "expected": {"ok": 1}, "actual": {"ok": 3}}),
            ],
            ..DiffOutcome::default()
        }
    }

    #[test]
    fn known_diffs_pass_only_with_the_listed_result() {
        let entries = [
            json!({"fn": "f", "args": [1], "actual": {"ok": 2}, "reason": "fixed a bug"}),
            json!({"fn": "f", "args": [2], "actual": {"ok": 4}, "reason": "other result"}),
            json!({"fn": "g", "args": [], "actual": {"ok": 0}, "reason": "gone"}),
        ];
        let out = apply_known(outcome(), &entries);
        assert_eq!((out.passed, out.failed, out.accepted), (2, 1, 1));
        assert_eq!(out.mismatches.len(), 1);
        assert_eq!(out.mismatches[0]["args"], json!([2]));
        assert_eq!(out.stale_known.len(), 2);
    }

    #[test]
    fn a_port_stalls_after_changes_that_gain_nothing() {
        assert!(!trend(vec![10, 12, 15], 20).stalled);
        assert!(!trend(vec![10, 12, 12, 12], 20).stalled);
        assert!(trend(vec![10, 12, 12, 11, 12], 20).stalled);
        assert!(!trend(vec![10, 12, 12, 11, 13], 20).stalled);
        assert!(!trend(vec![20, 20, 20, 20, 20], 20).stalled);
    }

    #[test]
    fn rerunning_diff_on_an_unchanged_port_is_not_an_attempt() {
        let dir = std::env::temp_dir().join(format!("ra-trend-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let mut last = None;
        for (port, passed) in [("a", 1), ("a", 2), ("b", 2), ("c", 2), ("d", 2), ("e", 2)] {
            std::fs::write(dir.join("src/port.rs"), port).unwrap();
            last = track(&dir, passed, 5);
        }
        std::fs::remove_dir_all(&dir).unwrap();
        let last = last.unwrap();
        assert_eq!(last.passed, vec![2, 2, 2, 2, 2]);
        assert!(last.stalled);
    }

    #[test]
    fn entries_need_a_reason() {
        let dir = std::env::temp_dir().join(format!("ra-known-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(KNOWN_DIFFS),
            "// comment\n{\"fn\": \"f\", \"args\": [1], \"actual\": {\"ok\": 2}}\n",
        )
        .unwrap();
        let err = known_diffs(&dir).unwrap_err();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(format!("{err:#}").contains("reason"), "{err:#}");
    }
}
