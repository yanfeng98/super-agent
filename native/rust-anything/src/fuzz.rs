//! Live differential testing: fresh inputs from a new seed, run through the original and the
//! port, mismatches reported and optionally kept in the recording.
//!
//! Every language's record and diff drivers read and write `cases.jsonl`, so a round records
//! its fresh cases there, diffs them, and puts the recording back. Whatever happens, the file
//! the round started from is restored before anything is kept.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use serde::Serialize;
use serde_json::Value;

use crate::bridge::Manifest;
use crate::harness::{self, CASES, DiffOptions, RecordOptions, WORK_DIR};

/// Which fresh cases `--keep` appends to the recording.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Keep {
    /// Only the cases where the port differs from the original.
    Mismatches,
    /// Every fresh case.
    All,
}

pub struct FuzzOptions<'a> {
    pub crate_dir: &'a Path,
    /// Fresh cases per function and class in each round.
    pub cases: usize,
    /// The first round's seed; later rounds count up from it. Random when `None`.
    pub seed: Option<u64>,
    /// Keep starting rounds until this much time has passed; one round when `None`.
    pub time: Option<Duration>,
    pub keep: Option<Keep>,
    pub resolve_from: Option<&'a Path>,
    pub node_args: &'a [String],
}

#[derive(Debug, Serialize)]
pub struct FuzzOutcome {
    /// Replays this run with `--seed`.
    pub seed: u64,
    pub rounds: u64,
    /// Fresh cases run on both sides (inputs the original timed out on are left out).
    pub cases: usize,
    pub passed: usize,
    pub failed: usize,
    /// Exports the port does not provide.
    pub missing: Vec<String>,
    /// The first mismatches: `{ fn, args, expected, actual }`.
    pub mismatches: Vec<Value>,
    /// Cases appended to `cases.jsonl` (`--keep`).
    pub kept: usize,
    pub recording: PathBuf,
}

const SHOWN: usize = 20;

/// Puts `cases.jsonl` and the timed-out list back as they were, also on an error.
struct Restore(Vec<(PathBuf, Option<Vec<u8>>)>);

impl Restore {
    fn new(paths: &[PathBuf]) -> Self {
        Self(
            paths
                .iter()
                .map(|p| (p.clone(), std::fs::read(p).ok()))
                .collect(),
        )
    }
}

impl Drop for Restore {
    fn drop(&mut self) {
        for (path, saved) in &self.0 {
            let _ = match saved {
                Some(bytes) => std::fs::write(path, bytes),
                None => std::fs::remove_file(path),
            };
        }
    }
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn random_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    splitmix((nanos as u64) ^ u64::from(std::process::id())) % 1_000_000_000
}

/// Numbers compared by value and object keys sorted, so the same value from two runtimes (`1`
/// and `1.0`, or the keys of one object in a different order) is one key.
fn norm(v: &Value) -> Value {
    match v {
        Value::Number(n) => n
            .as_f64()
            .and_then(serde_json::Number::from_f64)
            .map_or_else(|| v.clone(), Value::Number),
        Value::Array(a) => Value::Array(a.iter().map(norm).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, x)| (k.clone(), norm(x)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        other => other.clone(),
    }
}

fn key(name: &Value, args: &Value) -> String {
    serde_json::to_string(&(norm(name), norm(args))).unwrap_or_default()
}

/// The keys a case can be reported under by the language drivers' mismatches: the function
/// and its arguments (with keyword arguments appended, as Ruby shows them), or the class with
/// its constructor arguments and calls.
fn case_keys(c: &Value) -> Vec<String> {
    if let Some(class) = c.get("class") {
        return vec![key(
            class,
            &Value::Array(vec![c["ctor"].clone(), c["calls"].clone()]),
        )];
    }
    let mut keys = vec![key(&c["fn"], &c["args"])];
    if let (Some(args), Some(kwargs)) = (
        c["args"].as_array(),
        c.get("kwargs")
            .filter(|k| k.as_object().is_some_and(|o| !o.is_empty())),
    ) {
        let mut shown = args.clone();
        shown.push(kwargs.clone());
        keys.push(key(&c["fn"], &Value::Array(shown)));
    }
    keys
}

fn lines(text: &str) -> impl Iterator<Item = (&str, Value)> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| harness::parse_deep(l).ok().map(|v| (l, v)))
}

pub fn fuzz(opts: &FuzzOptions) -> Result<FuzzOutcome> {
    let crate_dir = std::path::absolute(opts.crate_dir)?;
    let manifest = Manifest::load(&crate_dir)?;
    if opts.cases == 0 {
        bail!("`--cases` must be at least 1");
    }
    let seed = opts.seed.unwrap_or_else(random_seed);
    let recording = crate_dir.join(CASES);
    let original = std::fs::read_to_string(&recording).unwrap_or_default();
    let mut seen: HashSet<String> = lines(&original)
        .filter_map(|(_, c)| case_keys(&c).into_iter().next())
        .collect();
    let record_opts = RecordOptions {
        crate_dir: &crate_dir,
        count: opts.cases,
        inputs: None,
        module: None,
        resolve_from: opts.resolve_from,
        node_args: opts.node_args,
    };
    let diff_opts = DiffOptions {
        crate_dir: &crate_dir,
        module: None,
        ignore_key_order: false,
        node_args: opts.node_args,
        float_tolerance: None,
    };
    let mut outcome = FuzzOutcome {
        seed,
        rounds: 0,
        cases: 0,
        passed: 0,
        failed: 0,
        missing: Vec::new(),
        mismatches: Vec::new(),
        kept: 0,
        recording: recording.clone(),
    };
    let mut keep = String::new();
    let started = Instant::now();
    {
        let _restore = Restore::new(&[
            recording.clone(),
            crate_dir.join(WORK_DIR).join(harness::TIMED_OUT),
        ]);
        loop {
            let salt = splitmix(seed.wrapping_add(outcome.rounds)) | 0x100;
            outcome.rounds += 1;
            let (generated, _, _) = harness::generate_cases_salted(&record_opts, &manifest, salt)?;
            let fresh: Vec<Value> = generated
                .into_iter()
                .filter(|c| {
                    case_keys(c)
                        .into_iter()
                        .next()
                        .is_some_and(|k| seen.insert(k))
                })
                .collect();
            if fresh.is_empty() {
                break;
            }
            harness::record_with(&record_opts, &manifest, &fresh)?;
            let ran = std::fs::read_to_string(&recording).unwrap_or_default();
            let d = harness::settle(&diff_opts, harness::diff_exact(&diff_opts)?)?;
            outcome.cases += d.total;
            outcome.passed += d.passed;
            outcome.failed += d.failed;
            for m in &d.missing {
                if !outcome.missing.contains(m) {
                    outcome.missing.push(m.clone());
                }
            }
            let failing: HashSet<String> = d
                .mismatches
                .iter()
                .map(|m| key(&m["fn"], &m["args"]))
                .collect();
            for (line, c) in lines(&ran) {
                let wanted = match opts.keep {
                    Some(Keep::All) => true,
                    Some(Keep::Mismatches) => case_keys(&c).iter().any(|k| failing.contains(k)),
                    None => false,
                };
                if wanted {
                    keep.push_str(line);
                    keep.push('\n');
                    outcome.kept += 1;
                }
            }
            let room = SHOWN.saturating_sub(outcome.mismatches.len());
            outcome
                .mismatches
                .extend(d.mismatches.into_iter().take(room));
            if opts.time.is_none_or(|budget| started.elapsed() >= budget) {
                break;
            }
        }
    }
    let _ = std::fs::remove_dir_all(crate_dir.join(WORK_DIR).join("coverage"));
    if !keep.is_empty() {
        let mut text = original;
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&keep);
        std::fs::write(&recording, text)?;
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mismatches_find_their_cases_across_driver_shapes() {
        let ruby = json!({ "fn": "f", "args": [1, "a"], "kwargs": { "sep": "-" }, "ok": 2 });
        let shown = json!({ "fn": "f", "args": [1.0, "a", { "sep": "-" }] });
        assert!(case_keys(&ruby).contains(&key(&shown["fn"], &shown["args"])));
        let class = json!({ "class": "C", "ctor": [1], "calls": [], "results": [] });
        let reported = json!({ "fn": "C", "args": [[1], []] });
        assert_eq!(
            case_keys(&class),
            vec![key(&reported["fn"], &reported["args"])]
        );
    }

    #[test]
    fn a_case_key_ignores_the_order_of_object_keys() {
        // The original and the port are different runtimes: the same value comes back with its
        // keys in a different order, and a case must still match its own report.
        let left = json!({ "fn": "f", "args": [{ "a": 1, "b": [2, { "c": 3, "d": 4 }] }] });
        let right = json!({ "fn": "f", "args": [{ "b": [2, { "d": 4, "c": 3 }], "a": 1 }] });
        assert_eq!(
            key(&left["fn"], &left["args"]),
            key(&right["fn"], &right["args"])
        );
    }

    #[test]
    fn restore_puts_files_back_or_removes_new_ones() {
        let dir =
            std::env::temp_dir().join(format!("rust-anything-restore-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let kept = dir.join("a");
        let fresh = dir.join("b");
        std::fs::write(&kept, "old").unwrap();
        {
            let _r = Restore::new(&[kept.clone(), fresh.clone()]);
            std::fs::write(&kept, "new").unwrap();
            std::fs::write(&fresh, "new").unwrap();
        }
        assert_eq!(std::fs::read_to_string(&kept).unwrap(), "old");
        assert!(!fresh.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
