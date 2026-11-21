//! `profile`: run a real workload under the language's own profiler and say what to port.
//!
//! Porting a leaf function rarely pays: each call into Rust costs more than the leaf's work.
//! What pays is moving a hot loop with its callers, keeping data on the Rust side, and using all
//! cores. This step finds those: hot functions in the repository, by inclusive time, with the
//! callers that spend it, call counts where the profiler has them, and a recommendation each.

use crate::walk::Canonical;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;

use crate::lang::Lang;

pub const SCHEMA: &str = "rust-anything/profile@1";

#[derive(Debug, Serialize)]
pub struct Caller {
    pub name: String,
    pub file: String,
    pub line: u32,
    /// Time of the callee spent under this caller.
    pub ms: f64,
    /// Calls from this caller, when the profiler counts calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calls: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct Hot {
    pub name: String,
    pub file: String,
    pub line: u32,
    /// The analyzed unit holding the function, and whether the unit exports it.
    pub unit: Option<String>,
    pub exported: bool,
    pub self_ms: f64,
    pub total_ms: f64,
    pub total_pct: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calls: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ns_per_call: Option<f64>,
    pub callers: Vec<Caller>,
    pub advice: String,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub language: Lang,
    pub command: Vec<String>,
    pub total_ms: f64,
    /// Share of the run spent in the repository's own code (the rest: runtime, libraries).
    pub repo_pct: f64,
    pub hot: Vec<Hot>,
    /// Functions the repository already compiles to machine code (numba, Cython, JAX).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub compiled: Vec<Compiled>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Compiled {
    pub name: String,
    pub file: String,
    pub line: u32,
    pub by: &'static str,
}

/// Every Python function in the repository a decorator compiles.
fn compiled_in(root: &Path, lang: Lang) -> Vec<Compiled> {
    if lang != Lang::Python {
        return Vec::new();
    }
    let Ok(ws) = crate::lang::python::discover(root) else {
        return Vec::new();
    };
    ws.files
        .iter()
        .filter(|f| f.kind == crate::ts::discover::FileKind::Source)
        .flat_map(|f| {
            let text = std::fs::read_to_string(root.join(&f.path)).unwrap_or_default();
            crate::risks::python_compiled(&text)
                .into_iter()
                .map(|(name, line, by)| Compiled {
                    name,
                    file: f.path.clone(),
                    line,
                    by,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The body of the Python function `name` defined at or just after `line`.
fn python_body(text: &str, name: &str, line: u32) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = (line as usize).saturating_sub(1);
    let def = (start..lines.len().min(start + 20)).find(|&i| {
        let t = lines[i].trim_start();
        t.strip_prefix("def ")
            .or_else(|| t.strip_prefix("async def "))
            .is_some_and(|d| d.starts_with(name) && d[name.len()..].trim_start().starts_with('('))
    })?;
    let indent = |l: &str| l.len() - l.trim_start().len();
    let base = indent(lines[def]);
    let body: Vec<&str> = lines[def + 1..]
        .iter()
        .take_while(|l| l.trim().is_empty() || indent(l) > base)
        .copied()
        .collect();
    Some(body.join("\n"))
}

fn calls(body: &str, name: &str) -> bool {
    body.match_indices(&format!("{name}(")).any(|(i, _)| {
        body[..i]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
    })
}

/// Advice for a hot function that is compiled already, or that calls compiled code.
fn compiled_advice(root: &Path, h: &Hot, compiled: &[Compiled]) -> Option<String> {
    if let Some(c) = compiled
        .iter()
        .find(|c| c.file == h.file && c.name == h.name)
    {
        return Some(format!(
            "Already compiled by {} (`{}:{}`): a Rust port is unlikely to run it much faster. \
             Port it for a pure Rust app, not for speed.",
            c.by, c.file, c.line
        ));
    }
    let text = std::fs::read_to_string(root.join(&h.file)).ok()?;
    let body = python_body(&text, &h.name, h.line)?;
    let called: Vec<&Compiled> = compiled.iter().filter(|c| calls(&body, &c.name)).collect();
    let first = called.first()?;
    let names: Vec<String> = called.iter().map(|c| format!("`{}`", c.name)).collect();
    Some(format!(
        "{} It calls {} compiled by {} (`{}:{}`): the profiler counts that compiled time here, so \
         porting this may gain little; port the Python around those calls, or measure without them.",
        h.advice,
        names.join(", "),
        first.by,
        first.file,
        first.line
    ))
}

/// One function's numbers before advice: key is (file, name, line).
#[derive(Default)]
struct Acc {
    self_ms: f64,
    total_ms: f64,
    calls: Option<u64>,
    callers: HashMap<(String, String, u32), (f64, Option<u64>)>,
}

type Key = (String, String, u32);

pub fn profile(root: &Path, command: &[String], lang: Option<Lang>) -> Result<Report> {
    if command.is_empty() {
        bail!(
            "give the workload to run after `--`, e.g. `rust-anything profile . -- node bench.mjs`"
        );
    }
    let root = root
        .canonical()
        .with_context(|| format!("resolving {}", root.display()))?;
    let lang = lang.unwrap_or_else(|| Lang::detect(&root));
    let work = std::env::temp_dir().join(format!("rust-anything-profile-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)?;
    let (functions, total_ms) = match lang {
        Lang::Ts => node(&root, command, &work)?,
        Lang::Python => python(&root, command, &work)?,
        Lang::Go => go(&root, command, &work)?,
        Lang::Java => java(&root, command, &work)?,
        Lang::Kotlin => bail!(
            "`profile` does not run Kotlin workloads yet; use JFR (`jfr print --stack-depth 256`) \
             and port the hot loops it shows"
        ),
        Lang::Ruby => ruby(&root, command, &work)?,
        Lang::C => bail!(
            "`profile` runs TS/JS, Python, Go, Java and Ruby workloads; for C, use `perf record -g` \
             and port the hot loops it shows"
        ),
        Lang::Php => bail!(
            "`profile` does not run PHP workloads yet; use SPX or Xdebug's profiler and port the \
             hot loops it shows"
        ),
        Lang::Csharp => bail!(
            "`profile` does not run C# workloads yet; use `dotnet-trace` or your IDE's profiler and \
             port the hot loops it shows"
        ),
    };
    let _ = std::fs::remove_dir_all(&work);
    Ok(report(&root, lang, command, functions, total_ms))
}

/// A frame's file: repository-relative, or absolute for user code elsewhere (a workload
/// script); `None` for the runtime and installed libraries.
fn relative(root: &Path, file: &str) -> Option<String> {
    let decoded;
    let path = if let Some(url_path) = file.strip_prefix("file://") {
        decoded = percent_encoding::percent_decode_str(url_path).decode_utf8_lossy();
        decoded.as_ref()
    } else {
        file
    };
    if !path.starts_with('/') && !path.contains(":\\") {
        let rel = path.trim_start_matches("./");
        return (!rel.starts_with(['<', '~']) && !rel.contains(':') && root.join(rel).is_file())
            .then(|| rel.replace('\\', "/")); // node:internal/..., <frozen ...>, ~
    }
    // Installed libraries and runtimes: npm, Python, Ruby gems and stdlib, Go modules and GOROOT.
    if path.split(['/', '\\']).any(|p| {
        matches!(
            p,
            "node_modules"
                | "site-packages"
                | "dist-packages"
                | ".venv"
                | "venv"
                | "gems"
                | "vendor"
        )
    }) || ["/lib/python3", "/lib/ruby/", "/pkg/mod/"]
        .iter()
        .any(|d| path.contains(d))
        || goroot().is_some_and(|g| path.starts_with(g))
    {
        return None;
    }
    let p = PathBuf::from(path);
    Some(match p.strip_prefix(root) {
        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
        Err(_) => path.to_string(),
    })
}

/// Go's own sources, which are not user code.
fn goroot() -> Option<&'static str> {
    static GOROOT: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    GOROOT
        .get_or_init(|| {
            let out = crate::process::output(
                Command::new("go").args(["env", "GOROOT"]),
                crate::process::QUICK,
            )
            .ok()?;
            let root = String::from_utf8(out.stdout).ok()?.trim().to_string();
            (!root.is_empty()).then(|| format!("{root}/"))
        })
        .as_deref()
}

/// Whether a frame file is inside the repository (relative).
fn in_repo(file: &str) -> bool {
    !file.starts_with('/')
}

// ---- node --cpu-prof ------------------------------------------------------------------------

fn node(root: &Path, command: &[String], work: &Path) -> Result<(HashMap<Key, Acc>, f64)> {
    let dir = work.join("node");
    std::fs::create_dir_all(&dir)?;
    let mut options = std::env::var("NODE_OPTIONS").unwrap_or_default();
    let dir_option = dir
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    options.push_str(&format!(" --cpu-prof --cpu-prof-dir=\"{dir_option}\""));
    let status = crate::process::status(
        Command::new(&command[0])
            .args(&command[1..])
            .current_dir(root)
            .env("NODE_OPTIONS", options.trim()),
        crate::process::TESTS,
    )
    .with_context(|| format!("running {}", command[0]))?;
    if !status.success() {
        bail!("the workload failed ({status})");
    }
    let mut functions: HashMap<Key, Acc> = HashMap::new();
    let mut total = 0.0;
    for entry in std::fs::read_dir(&dir)?.flatten() {
        let profile: Value = serde_json::from_str(&std::fs::read_to_string(entry.path())?)?;
        total += merge_cpuprofile(root, &profile, &mut functions);
    }
    if total == 0.0 {
        bail!("no CPU profile was written: is the workload a node process?");
    }
    Ok((functions, total))
}

/// A `.cpuprofile` node: its frame when it is user code, its own time, its children.
struct Frame {
    key: Option<Key>,
    self_ms: f64,
    children: Vec<u64>,
}

/// Adds one `.cpuprofile` to `functions`; returns its duration in ms.
fn merge_cpuprofile(root: &Path, profile: &Value, functions: &mut HashMap<Key, Acc>) -> f64 {
    let Some(nodes) = profile["nodes"].as_array() else {
        return 0.0;
    };
    let samples = profile["samples"].as_array().map_or(0, Vec::len).max(1);
    let duration_ms = (profile["endTime"].as_f64().unwrap_or(0.0)
        - profile["startTime"].as_f64().unwrap_or(0.0))
        / 1000.0;
    let per_sample = duration_ms / samples as f64;
    let mut by_id: HashMap<u64, Frame> = HashMap::new();
    let mut is_child = HashSet::new();
    for n in nodes {
        let id = n["id"].as_u64().unwrap_or(0);
        let frame = &n["callFrame"];
        let name = frame["functionName"].as_str().unwrap_or("").to_string();
        let url = frame["url"].as_str().unwrap_or("");
        let line = u32::try_from(frame["lineNumber"].as_i64().unwrap_or(0) + 1).unwrap_or(0);
        let key = relative(root, url).map(|file| {
            let shown = if name.is_empty() {
                format!("(anonymous):{line}")
            } else {
                name.clone()
            };
            (file, shown, line)
        });
        let children: Vec<u64> = n["children"]
            .as_array()
            .map_or_else(Vec::new, |c| c.iter().filter_map(Value::as_u64).collect());
        is_child.extend(children.iter().copied());
        let self_ms = n["hitCount"].as_f64().unwrap_or(0.0) * per_sample;
        by_id.insert(
            id,
            Frame {
                key,
                self_ms,
                children,
            },
        );
    }
    // Inclusive time per node, depth first.
    fn total(id: u64, by_id: &HashMap<u64, Frame>, memo: &mut HashMap<u64, f64>) -> f64 {
        if let Some(t) = memo.get(&id) {
            return *t;
        }
        let Some(frame) = by_id.get(&id) else {
            return 0.0;
        };
        let t = frame.self_ms
            + frame
                .children
                .iter()
                .map(|c| total(*c, by_id, memo))
                .sum::<f64>();
        memo.insert(id, t);
        t
    }
    let mut memo = HashMap::new();
    // Walk from the roots, tracking the repo frames on the stack: recursion counts once, and a
    // callee's caller is the nearest *named* repo frame above it (callbacks are skipped).
    let roots: Vec<u64> = by_id
        .keys()
        .filter(|id| !is_child.contains(id))
        .copied()
        .collect();
    let mut stack: Vec<(u64, Vec<Key>)> = roots.into_iter().map(|r| (r, Vec::new())).collect();
    while let Some((id, active)) = stack.pop() {
        let Some(Frame {
            key,
            self_ms,
            children,
        }) = by_id.get(&id)
        else {
            continue;
        };
        let mut next = active.clone();
        if let Some(k) = key {
            let inclusive = total(id, &by_id, &mut memo);
            let acc = functions.entry(k.clone()).or_default();
            acc.self_ms += self_ms;
            if !active.contains(k) {
                acc.total_ms += inclusive;
                if let Some(caller) = active
                    .iter()
                    .rev()
                    .find(|c| !c.1.starts_with("(anonymous)"))
                    && caller != k
                {
                    acc.callers.entry(caller.clone()).or_default().0 += inclusive;
                }
            }
            // Anonymous callbacks stay on the stack too; the caller lookup above skips them.
            next.push(k.clone());
        }
        for c in children {
            stack.push((*c, next.clone()));
        }
    }
    duration_ms
}

// ---- Python cProfile ------------------------------------------------------------------------

const PSTATS_PY: &str = r#"
import json, pstats, sys
stats = pstats.Stats(sys.argv[1]).stats
out = []
for (file, line, name), (cc, nc, tt, ct, callers) in stats.items():
    out.append({
        "file": file, "line": line, "name": name, "calls": nc, "self": tt * 1000, "total": ct * 1000,
        "callers": [{"file": f, "line": l, "name": n, "calls": c[1], "total": c[3] * 1000} for (f, l, n), c in callers.items()],
    })
json.dump(out, open(sys.argv[2], "w"))
"#;

fn python(root: &Path, command: &[String], work: &Path) -> Result<(HashMap<Key, Acc>, f64)> {
    let program = Path::new(&command[0])
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if !program.starts_with("python") {
        bail!(
            "for Python, give the interpreter and script: `rust-anything profile . -- python3 bench.py`"
        );
    }
    let stats = work.join("profile.pstats");
    let start = std::time::Instant::now();
    let status = crate::process::status(
        Command::new(&command[0])
            .args(["-m", "cProfile", "-o"])
            .arg(&stats)
            .args(&command[1..])
            .current_dir(root),
        crate::process::TESTS,
    )
    .with_context(|| format!("running {}", command[0]))?;
    let total_ms = start.elapsed().as_secs_f64() * 1000.0;
    if !status.success() {
        bail!("the workload failed ({status})");
    }
    let json = work.join("profile.json");
    let script = work.join("pstats_json.py");
    std::fs::write(&script, PSTATS_PY)?;
    let status = crate::process::status(
        Command::new(&command[0])
            .arg(&script)
            .arg(&stats)
            .arg(&json),
        crate::process::TESTS,
    )?;
    if !status.success() {
        bail!("reading the cProfile output failed");
    }
    let entries: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(&json)?)?;
    Ok((ingest_entries(root, &entries), total_ms))
}

/// Function entries from a tracing profiler (cProfile, the Ruby tracer): file, line, name,
/// calls, self and total ms, and callers with their calls and ms.
fn ingest_entries(root: &Path, entries: &[Value]) -> HashMap<Key, Acc> {
    let mut functions: HashMap<Key, Acc> = HashMap::new();
    for e in entries {
        let Some(file) = relative(root, e["file"].as_str().unwrap_or("")) else {
            continue;
        };
        let key = (
            file,
            e["name"].as_str().unwrap_or("").to_string(),
            u32::try_from(e["line"].as_u64().unwrap_or(0)).unwrap_or(0),
        );
        let acc = functions.entry(key).or_default();
        acc.self_ms += e["self"].as_f64().unwrap_or(0.0);
        acc.total_ms += e["total"].as_f64().unwrap_or(0.0);
        acc.calls = Some(acc.calls.unwrap_or(0) + e["calls"].as_u64().unwrap_or(0));
        for c in e["callers"].as_array().into_iter().flatten() {
            let Some(cfile) = relative(root, c["file"].as_str().unwrap_or("")) else {
                continue;
            };
            let ckey = (
                cfile,
                c["name"].as_str().unwrap_or("").to_string(),
                u32::try_from(c["line"].as_u64().unwrap_or(0)).unwrap_or(0),
            );
            let slot = acc.callers.entry(ckey).or_default();
            slot.0 += c["total"].as_f64().unwrap_or(0.0);
            slot.1 = Some(slot.1.unwrap_or(0) + c["calls"].as_u64().unwrap_or(0));
        }
    }
    functions
}

// ---- stack samples (Go pprof, Java JFR) -----------------------------------------------------

/// Adds sampled stacks, each root first with `None` for frames outside user code, and the time
/// it stands for. A function's total counts once per sample (recursion); its caller is the
/// nearest named user frame above it (lambdas and closures are skipped).
fn merge_samples(samples: &[(Vec<Option<Key>>, f64)], functions: &mut HashMap<Key, Acc>) {
    for (stack, ms) in samples {
        let mut seen: Vec<&Key> = Vec::new();
        let mut named: Option<&Key> = None;
        for frame in stack {
            let Some(k) = frame else { continue };
            if !seen.contains(&k) {
                seen.push(k);
                let acc = functions.entry(k.clone()).or_default();
                acc.total_ms += ms;
                if let Some(caller) = named.filter(|c| *c != k) {
                    acc.callers.entry(caller.clone()).or_default().0 += ms;
                }
            }
            if !k.1.starts_with("(anonymous)") {
                named = Some(k);
            }
        }
        if let Some(Some(leaf)) = stack.last() {
            functions.entry(leaf.clone()).or_default().self_ms += ms;
        }
    }
}

/// A Go function's own name: `example.com/x/pkg.Slugify` is `Slugify`, `pkg.(*T).M` is `M`;
/// closures (`Outer.func1`) are anonymous.
fn go_name(full: &str) -> String {
    let tail = full.rsplit('/').next().unwrap_or(full);
    let rest = tail.split_once('.').map_or(tail, |(_, r)| r);
    let parts: Vec<&str> = rest.split('.').collect();
    match parts.as_slice() {
        [.., outer, f] if f.starts_with("func") && f[4..].chars().all(|c| c.is_ascii_digit()) => {
            format!("(anonymous) in {outer}")
        }
        [.., last] => last.to_string(),
        [] => rest.to_string(),
    }
}

/// `go tool pprof -raw` output: samples (leaf first, by location id) and locations (each one or
/// more frames, innermost first: `func file:line:col s=start`).
fn parse_pprof_raw(root: &Path, raw: &str) -> Vec<(Vec<Option<Key>>, f64)> {
    let mut locations: HashMap<u64, Vec<Option<Key>>> = HashMap::new();
    let mut samples: Vec<(Vec<u64>, f64)> = Vec::new();
    let mut section = "";
    let mut current: Option<u64> = None;
    // Which sample value is CPU time: `samples/count cpu/nanoseconds` puts it second.
    let mut value_index = 1;
    for line in raw.lines() {
        match line.trim_end() {
            "Samples:" => {
                section = "samples";
                continue;
            }
            "Locations" => {
                section = "locations";
                continue;
            }
            "Mappings" => {
                section = "";
                continue;
            }
            _ => {}
        }
        match section {
            "samples" if line.contains(':') => {
                let (values, ids) = line.split_once(':').unwrap_or_default();
                let values: Vec<f64> = values
                    .split_whitespace()
                    .filter_map(|v| v.parse().ok())
                    .collect();
                let ids: Vec<u64> = ids
                    .split_whitespace()
                    .filter_map(|v| v.parse().ok())
                    .collect();
                if let Some(ns) = values.get(value_index).or(values.last()) {
                    samples.push((ids, ns / 1e6));
                }
            }
            "samples" => {
                let kinds: Vec<&str> = line.split_whitespace().collect();
                if let Some(i) = kinds.iter().position(|k| k.ends_with("/nanoseconds")) {
                    value_index = i;
                }
            }
            "locations" => {
                let text = line.trim_start();
                let frame_text = match text.split_once(": ") {
                    Some((id, rest)) if id.chars().all(|c| c.is_ascii_digit()) => {
                        current = id.parse().ok();
                        // `0x4c2ded M=1 func file:line:col s=start`
                        rest.split_whitespace()
                            .skip(2)
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                    _ => text.to_string(),
                };
                let Some(id) = current else { continue };
                let mut parts = frame_text.split_whitespace();
                let (Some(func), Some(place)) = (parts.next(), parts.next()) else {
                    continue;
                };
                let start = parts
                    .find_map(|p| p.strip_prefix("s="))
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0);
                let file = place.rsplitn(3, ':').nth(2).unwrap_or(place);
                let key = relative(root, file).map(|f| (f, go_name(func), start));
                locations.entry(id).or_default().push(key);
            }
            _ => {}
        }
    }
    samples
        .into_iter()
        .map(|(ids, ms)| {
            // Leaf first, and inlined frames innermost first: reversed, the stack is root first.
            let mut stack: Vec<Option<Key>> = ids
                .iter()
                .flat_map(|id| locations.get(id).cloned().unwrap_or_default())
                .collect();
            stack.reverse();
            (stack, ms)
        })
        .collect()
}

/// Go: `go test` with `-cpuprofile` (Go has no profiler to attach from outside), read with
/// `go tool pprof -raw`.
fn go(root: &Path, command: &[String], work: &Path) -> Result<(HashMap<Key, Acc>, f64)> {
    let program = Path::new(&command[0])
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if program != "go" || command.get(1).map(String::as_str) != Some("test") {
        bail!(
            "for Go, profile a benchmark or test: `rust-anything profile . -- go test -run XXX -bench . ./pkg` \
             (Go profiles only from inside the process; `go test` adds -cpuprofile)"
        );
    }
    let out = work.join("cpu.pprof");
    let status = crate::process::status(
        Command::new(&command[0])
            .arg("test")
            .arg(format!("-cpuprofile={}", out.display()))
            .args(&command[2..])
            .current_dir(root),
        crate::process::TESTS,
    )
    .with_context(|| format!("running {}", command[0]))?;
    if !status.success() {
        bail!("the workload failed ({status})");
    }
    let raw = crate::process::output(
        Command::new("go")
            .args(["tool", "pprof", "-raw"])
            .arg(&out)
            .current_dir(root),
        crate::process::TESTS,
    )?;
    if !raw.status.success() {
        bail!(
            "go tool pprof failed: {}",
            String::from_utf8_lossy(&raw.stderr)
        );
    }
    let samples = parse_pprof_raw(root, &String::from_utf8_lossy(&raw.stdout));
    let total: f64 = samples.iter().map(|(_, ms)| ms).sum();
    let mut functions = HashMap::new();
    merge_samples(&samples, &mut functions);
    Ok((functions, total))
}

/// The line declaring method `name` in Java source: the nearest declaration at or above `near`.
fn java_declaration(text: &str, name: &str, near: u32) -> Option<u32> {
    let needle = format!("{name}(");
    let mut best = None;
    for (i, line) in text.lines().enumerate() {
        let n = u32::try_from(i + 1).unwrap_or(u32::MAX);
        if n > near.max(1) {
            break;
        }
        let Some(at) = line.find(&needle) else {
            continue;
        };
        let before = line[..at].trim_end();
        // A declaration names a type or modifier before the method: `static int pages(`.
        let declares = before
            .ends_with(|c: char| c.is_alphanumeric() || matches!(c, '>' | ']' | '_'))
            && !before.contains('=')
            && !before.contains("return")
            && !before.contains("new ")
            && !before.ends_with('.')
            && !line.trim_end().ends_with(';');
        if declares && at > 0 && line.as_bytes()[at - 1].is_ascii_whitespace() {
            best = Some(n);
        }
    }
    best
}

/// Java: Flight Recorder's execution samples, read with `jfr print --json`.
fn java(root: &Path, command: &[String], work: &Path) -> Result<(HashMap<Key, Acc>, f64)> {
    let recording = work.join("profile.jfr");
    let mut options = std::env::var("JAVA_TOOL_OPTIONS").unwrap_or_default();
    options.push_str(&format!(
        " -XX:StartFlightRecording=filename={},settings=profile -XX:FlightRecorderOptions=stackdepth=256",
        recording.display()
    ));
    let status = crate::process::status(
        Command::new(&command[0])
            .args(&command[1..])
            .current_dir(root)
            .env("JAVA_TOOL_OPTIONS", options.trim()),
        crate::process::TESTS,
    )
    .with_context(|| format!("running {}", command[0]))?;
    if !status.success() {
        bail!("the workload failed ({status})");
    }
    let out = crate::process::output(
        Command::new("jfr")
            .args([
                "print",
                "--json",
                "--stack-depth",
                "256",
                "--events",
                "jdk.ExecutionSample",
            ])
            .arg(&recording),
        crate::process::TESTS,
    )
    .context("running jfr (it comes with the JDK)")?;
    if !out.status.success() {
        bail!("jfr print failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    let json: Value = serde_json::from_slice(&out.stdout).context("jfr output")?;
    // Sources by class: `com/example/text/Strings` -> its file in the repository.
    let mut sources: HashMap<String, String> = HashMap::new();
    for entry in crate::walk::builder(root).build() {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "java")
            && let Ok(rel) = path.strip_prefix(root)
        {
            let rel = rel.to_string_lossy().replace('\\', "/");
            let text = std::fs::read_to_string(path).unwrap_or_default();
            let package = text
                .lines()
                .find_map(|l| {
                    l.trim()
                        .strip_prefix("package ")
                        .map(|p| p.trim_end_matches(';').trim().to_string())
                })
                .unwrap_or_default();
            let class = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let fq = if package.is_empty() {
                class
            } else {
                format!("{}/{class}", package.replace('.', "/"))
            };
            sources.insert(fq, rel);
        }
    }
    const SAMPLE_MS: f64 = 10.0; // the `profile` settings sample Java threads every 10 ms
    let mut raw: Vec<Vec<Option<(String, String, u32)>>> = Vec::new();
    for event in json["recording"]["events"].as_array().into_iter().flatten() {
        let frames = event["values"]["stackTrace"]["frames"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut stack: Vec<Option<(String, String, u32)>> = frames
            .iter()
            .map(|f| {
                let class = f["method"]["type"]["name"].as_str()?;
                let outer = class.split('$').next().unwrap_or(class);
                let file = sources.get(outer)?;
                let name = f["method"]["name"].as_str()?.to_string();
                let line = u32::try_from(f["lineNumber"].as_i64().unwrap_or(0)).unwrap_or(0);
                Some((file.clone(), name, line))
            })
            .collect();
        stack.reverse();
        raw.push(stack);
    }
    // Key each method by its declaration line, found from the lowest sampled line.
    let mut lowest: HashMap<(String, String), u32> = HashMap::new();
    for (file, name, line) in raw.iter().flatten().flatten() {
        let e = lowest.entry((file.clone(), name.clone())).or_insert(*line);
        *e = (*e).min(*line);
    }
    let mut texts: HashMap<String, String> = HashMap::new();
    let mut key = |file: &str, name: &str| -> Key {
        if name.starts_with("lambda$") {
            return (file.to_string(), format!("(anonymous) {name}"), 0);
        }
        let text = texts
            .entry(file.to_string())
            .or_insert_with(|| std::fs::read_to_string(root.join(file)).unwrap_or_default());
        let near = lowest
            .get(&(file.to_string(), name.to_string()))
            .copied()
            .unwrap_or(0);
        let line = java_declaration(text, name, near).unwrap_or(near);
        (file.to_string(), name.to_string(), line)
    };
    let samples: Vec<(Vec<Option<Key>>, f64)> = raw
        .iter()
        .map(|stack| {
            let keys = stack
                .iter()
                .map(|f| f.as_ref().map(|(file, name, _)| key(file, name)))
                .collect();
            (keys, SAMPLE_MS)
        })
        .collect();
    let total = samples.len() as f64 * SAMPLE_MS;
    if samples.is_empty() {
        bail!("no samples were recorded: run the workload longer (Java is sampled every 10 ms)");
    }
    let mut functions = HashMap::new();
    merge_samples(&samples, &mut functions);
    Ok((functions, total))
}

// ---- Ruby: a call tracer --------------------------------------------------------------------

/// Loaded with `RUBYOPT=-r`: times every Ruby method call on the main thread (inclusive, self,
/// callers) and writes cProfile-like entries at exit. Tracing slows the program; the shares stay
/// comparable.
const RUBY_TRACER: &str = r#"
require "json"

module RustAnythingProfile
  STATS = Hash.new { |h, k| h[k] = { calls: 0, self: 0.0, total: 0.0, callers: Hash.new { |c, ck| c[ck] = [0, 0.0] } } }
  STACK = []
  ACTIVE = Hash.new(0)
  MAIN = Thread.current
  TRACE = TracePoint.new(:call, :return) do |tp|
    next unless Thread.current == MAIN
    now = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    if tp.event == :call
      path = tp.path
      path = File.expand_path(path) unless path.start_with?("<")
      key = [path, tp.lineno, tp.method_id.to_s]
      ACTIVE[key] += 1
      STACK << [key, now, 0.0]
    elsif (frame = STACK.pop)
      key, start, child = frame
      elapsed = now - start
      ACTIVE[key] -= 1
      outer = ACTIVE[key].zero?
      s = STATS[key]
      s[:calls] += 1
      s[:self] += elapsed - child
      s[:total] += elapsed if outer
      if (parent = STACK.last)
        parent[2] += elapsed
        c = s[:callers][parent[0]]
        c[0] += 1
        c[1] += elapsed if outer
      end
    end
  end
  TRACE.enable

  at_exit do
    TRACE.disable
    out = STATS.map do |(file, line, name), s|
      { file: file, line: line, name: name, calls: s[:calls], self: s[:self] * 1000, total: s[:total] * 1000,
        callers: s[:callers].map { |(f, l, n), (calls, total)| { file: f, line: l, name: n, calls: calls, total: total * 1000 } } }
    end
    File.write(ENV.fetch("RUST_ANYTHING_PROFILE_OUT"), JSON.generate(out))
  end
end
"#;

/// Ruby: the tracer above, loaded into any Ruby command through `RUBYOPT`.
fn ruby(root: &Path, command: &[String], work: &Path) -> Result<(HashMap<Key, Acc>, f64)> {
    let tracer = work.join("rust_anything_profile.rb");
    std::fs::write(&tracer, RUBY_TRACER)?;
    let out = work.join("profile.json");
    let mut rubyopt = std::env::var("RUBYOPT").unwrap_or_default();
    rubyopt.push_str(&format!(" -r{}", tracer.display()));
    let start = std::time::Instant::now();
    let status = crate::process::status(
        Command::new(&command[0])
            .args(&command[1..])
            .current_dir(root)
            .env("RUBYOPT", rubyopt.trim())
            .env("RUST_ANYTHING_PROFILE_OUT", &out),
        crate::process::TESTS,
    )
    .with_context(|| format!("running {}", command[0]))?;
    let total_ms = start.elapsed().as_secs_f64() * 1000.0;
    if !status.success() {
        bail!("the workload failed ({status})");
    }
    let entries: Vec<Value> = serde_json::from_str(
        &std::fs::read_to_string(&out)
            .context("the Ruby tracer wrote nothing: is the workload a Ruby process?")?,
    )?;
    Ok((ingest_entries(root, &entries), total_ms))
}

// ---- the report -----------------------------------------------------------------------------

/// The argument a caller passes per loop item: an arrow or for-of variable among `f`'s call
/// arguments in the caller's source. `None` when the source does not show one.
fn batch_param(root: &Path, caller: &Key, callee: &str, params: &[String]) -> Option<String> {
    let text = std::fs::read_to_string(root.join(&caller.0)).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let start = (caller.2 as usize).saturating_sub(1);
    let body = lines.get(start..(start + 80).min(lines.len()))?.join("\n");
    let mut loop_vars: Vec<String> = Vec::new();
    for (i, _) in body.match_indices("=>") {
        let before = body[..i].trim_end().trim_end_matches(')');
        let name: String = before
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if !name.is_empty() {
            loop_vars.push(name);
        }
    }
    for word in ["const ", "let ", "var ", "for "] {
        for (i, _) in body.match_indices(word) {
            let rest = &body[i + word.len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if rest[name.len()..].trim_start().starts_with("of ")
                || rest[name.len()..].trim_start().starts_with("in ")
            {
                loop_vars.push(name);
            }
        }
    }
    let (more, index_vars) = loop_variables(&body);
    loop_vars.extend(more);
    let call = body.find(&format!("{callee}("))?;
    let args_text = &body[call + callee.len() + 1..];
    let mut depth = 0;
    let mut args = vec![String::new()];
    for c in args_text.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' if depth == 0 => break,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                args.push(String::new());
                continue;
            }
            _ => {}
        }
        args.last_mut()?.push(c);
    }
    // The item itself (`t`), or the item at the loop index (`texts[i]`).
    let per_item = |a: &str| {
        let a = a.trim();
        loop_vars.iter().any(|v| v == a)
            || a.strip_suffix(']')
                .and_then(|a| a.split_once('['))
                .is_some_and(|(list, i)| {
                    !list.is_empty() && index_vars.iter().any(|v| v == i.trim())
                })
    };
    let position = args.iter().position(|a| per_item(a))?;
    params.get(position).cloned()
}

/// Loop variables in Go, Java and Ruby source: `(items, indexes)`. Items: `for _, t := range`,
/// `for (String t : texts)`, `|t|`; indexes: `for i := range`, `for (int i = 0;`.
fn loop_variables(body: &str) -> (Vec<String>, Vec<String>) {
    let ident = |s: &str| -> String {
        s.chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    };
    let last_ident = |s: &str| -> String {
        let s = s.trim_end();
        let rev: String = s
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        rev.chars().rev().collect()
    };
    let (mut items, mut indexes) = (Vec::new(), Vec::new());
    for (i, _) in body.match_indices("for") {
        if body[..i]
            .chars()
            .last()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let rest = &body[i + 3..];
        let line = rest.split(['{', '\n']).next().unwrap_or(rest);
        if let Some((vars, _)) = line.split_once(":= range") {
            // Go: `for i, t := range xs` (t is the item), `for i := range xs` (an index).
            let vars: Vec<String> = vars.split(',').map(|v| v.trim().to_string()).collect();
            match vars.as_slice() {
                [index, item] => {
                    items.push(item.clone());
                    indexes.push(index.clone());
                }
                [index] => indexes.push(index.clone()),
                _ => {}
            }
        } else if let Some(inner) = line.trim_start().strip_prefix('(') {
            // Java: `for (String t : texts)` or `for (int i = 0; ...)`.
            if let Some((decl, _)) = inner.split_once(" : ").filter(|(d, _)| !d.contains(';')) {
                items.push(last_ident(decl));
            } else if let Some((decl, _)) = inner.split_once('=') {
                indexes.push(last_ident(decl));
            }
        }
    }
    // Ruby blocks: `{ |t| ... }`, `do |t, i|`.
    for (i, _) in body.match_indices('|') {
        let before = body[..i].trim_end();
        if before.ends_with('{') || before.ends_with("do") {
            let first = ident(body[i + 1..].trim_start());
            if !first.is_empty() {
                items.push(first);
            }
        }
    }
    items.retain(|v| !v.is_empty() && v != "_");
    indexes.retain(|v| !v.is_empty() && v != "_");
    (items, indexes)
}

fn report(
    root: &Path,
    lang: Lang,
    command: &[String],
    functions: HashMap<Key, Acc>,
    total_ms: f64,
) -> Report {
    let analysis = crate::analyze::analyze_lang(root, lang).ok();
    let unit_of = |file: &str| -> Option<(String, bool)> {
        let a = analysis.as_ref()?;
        let u = a.units.iter().find(|u| u.files.iter().any(|f| f == file))?;
        let portable = !matches!(
            u.status,
            crate::analyze::Status::Keep | crate::analyze::Status::Blocked
        );
        Some((u.id.clone(), portable))
    };
    let exports_of = |file: &str, name: &str| -> Option<crate::ts::exports::ExportedFn> {
        let text = std::fs::read_to_string(root.join(file)).ok()?;
        let fns = if lang == Lang::Ts {
            crate::ts::exports::exported_functions(file, &text)
        } else {
            crate::lang::exports(lang, file, &text).0
        };
        fns.into_iter().find(|f| f.name == name)
    };
    let repo_self: f64 = functions
        .iter()
        .filter(|(k, _)| in_repo(&k.0))
        .map(|(_, a)| a.self_ms)
        .sum();
    let mut hot: Vec<Hot> = functions
        .into_iter()
        .filter(|((file, _, _), a)| in_repo(file) && total_ms > 0.0 && a.total_ms / total_ms >= 0.01)
        .map(|((file, name, line), a)| {
            let unit = unit_of(&file);
            let export = exports_of(&file, &name);
            let mut callers: Vec<Caller> = a
                .callers
                .iter()
                .map(|((f, n, l), (ms, calls))| Caller { name: n.clone(), file: f.clone(), line: *l, ms: *ms, calls: *calls })
                .collect();
            callers.sort_by(|x, y| y.ms.total_cmp(&x.ms));
            callers.truncate(5);
            let ns_per_call = a.calls.filter(|c| *c > 0).map(|c| a.total_ms * 1e6 / c as f64);
            let total_pct = 100.0 * a.total_ms / total_ms;
            let top = callers.first();
            let small_per_call = ns_per_call.is_some_and(|ns| ns < 20_000.0) || (a.calls.is_none() && top.is_some_and(|c| c.ms >= 0.5 * a.total_ms));
            let advice = match (&unit, &export) {
                (Some((u, true)), Some(e)) if small_per_call && top.is_some() => {
                    let c = top.expect("checked");
                    let params: Vec<String> = e.params.iter().map(|p| p.name.clone()).collect();
                    let batch = batch_param(root, &(c.file.clone(), c.name.clone(), c.line), &name, &params);
                    let calls = a.calls.map_or_else(String::new, |n| format!(", {n} calls"));
                    match batch {
                        Some(param) => format!(
                            "Called in a loop by `{}` ({}:{}{calls}): a call per item would cost more than the work. \
                             Batch it, keeping the items in Rust and using all cores: \
                             `rust-anything bridge {} {u} --batch {name}:{param}`, then prepare the items once in a \
                             `{}Index` in port.rs. Better still, port `{}` with it.",
                            c.name, c.file, c.line, root.display(), crate::codegen_batch::pascal(&name), c.name
                        ),
                        None => format!(
                            "Mostly called by `{}` ({}:{}{calls}): port `{}` together with `{name}`, so one call into Rust does the whole loop.",
                            c.name, c.file, c.line, c.name
                        ),
                    }
                }
                (Some((u, true)), Some(_)) => format!(
                    "Hot and exported by `{u}`: port it (`rust-anything bridge {} {u}`), then check `rust-anything bench`.",
                    root.display()
                ),
                (Some((u, true)), None) => format!("Hot inside `{u}`: port the exported function that calls it, with it."),
                (Some((u, false)), _) => format!("Hot, but `{u}` must stay in the source language (see `rust-anything analyze`)."),
                (None, _) => "Hot, outside the analyzed units.".into(),
            };
            Hot {
                unit: unit.map(|(u, _)| u),
                exported: export.is_some(),
                name,
                file,
                line,
                self_ms: a.self_ms,
                total_ms: a.total_ms,
                total_pct,
                calls: a.calls,
                ns_per_call,
                callers,
                advice,
            }
        })
        .collect();
    hot.sort_by(|a, b| b.total_ms.total_cmp(&a.total_ms));
    hot.truncate(25);
    let compiled = compiled_in(root, lang);
    for h in &mut hot {
        if let Some(advice) = compiled_advice(root, h, &compiled) {
            h.advice = advice;
        }
    }
    Report {
        schema: SCHEMA,
        language: lang,
        command: command.to_vec(),
        total_ms,
        repo_pct: if total_ms > 0.0 {
            (100.0 * repo_self / total_ms).min(100.0)
        } else {
            0.0
        },
        compiled,
        hot,
    }
}

pub fn markdown(r: &Report) -> String {
    use std::fmt::Write as _;
    let mut out = format!(
        "# Profile: `{}`\n\n{:.0} ms in all; {:.0}% of it in the repository's own code.\n\n\
         | Function | File | Total | Self | Calls | Unit |\n| --- | --- | ---: | ---: | ---: | --- |\n",
        r.command.join(" "),
        r.total_ms,
        r.repo_pct
    );
    for h in &r.hot {
        let _ = writeln!(
            out,
            "| `{}` | `{}:{}` | {:.0}% ({:.0} ms) | {:.0} ms | {} | {} |",
            h.name,
            h.file,
            h.line,
            h.total_pct,
            h.total_ms,
            h.self_ms,
            h.calls.map_or_else(|| "-".into(), |c| c.to_string()),
            h.unit
                .as_deref()
                .map_or_else(|| "-".into(), |u| format!("`{u}`"))
        );
    }
    if !r.compiled.is_empty() {
        out.push_str(
            "\n## Already compiled\n\nThese run as machine code already; Rust is unlikely to make them \
             much faster:\n\n",
        );
        for c in &r.compiled {
            let _ = writeln!(out, "- `{}` (`{}:{}`): {}", c.name, c.file, c.line, c.by);
        }
    }
    out.push_str("\n## What to port\n\n");
    let mut seen = BTreeMap::new();
    for h in r.hot.iter().filter(|h| h.unit.is_some()) {
        if seen.insert(h.name.clone(), ()).is_some() {
            continue;
        }
        let _ = writeln!(
            out,
            "- **`{}`** ({:.0}%): {}",
            h.name, h.total_pct, h.advice
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn compiled_code_changes_the_advice() {
        let root = std::env::temp_dir().join(format!("rust-anything-numba-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sim")).unwrap();
        std::fs::write(root.join("pyproject.toml"), "[project]\nname = \"sim\"\n").unwrap();
        std::fs::write(
            root.join("sim/physics.py"),
            "from numba import njit\n\n@njit(cache=True)\ndef step(x, dt):\n    return x + dt\n\n\
             def run(xs):\n    out = []\n    for x in xs:\n        out.append(step(x, 0.1))\n    return out\n\n\
             def other(xs):\n    return [x * 2 for x in xs]\n",
        )
        .unwrap();
        let compiled = compiled_in(&root, Lang::Python);
        assert_eq!(compiled.len(), 1);
        assert_eq!(
            (compiled[0].name.as_str(), compiled[0].line, compiled[0].by),
            ("step", 4, "numba")
        );
        let hot = |name: &str, line: u32| Hot {
            name: name.into(),
            file: "sim/physics.py".into(),
            line,
            unit: None,
            exported: true,
            self_ms: 1.0,
            total_ms: 1.0,
            total_pct: 50.0,
            calls: None,
            ns_per_call: None,
            callers: Vec::new(),
            advice: "Port it.".into(),
        };
        let own = compiled_advice(&root, &hot("step", 3), &compiled).unwrap();
        assert!(
            own.starts_with("Already compiled by numba (`sim/physics.py:4`)"),
            "{own}"
        );
        let caller = compiled_advice(&root, &hot("run", 7), &compiled).unwrap();
        assert!(
            caller.starts_with("Port it. It calls `step` compiled by numba"),
            "{caller}"
        );
        assert!(compiled_advice(&root, &hot("other", 13), &compiled).is_none());
        assert!(compiled_in(&root, Lang::Ts).is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn frames_outside_user_code_are_dropped() {
        let root = Path::new("/repo");
        assert_eq!(relative(root, "node:internal/main"), None);
        assert_eq!(relative(root, "file:///repo/node_modules/x/index.js"), None);
        assert_eq!(relative(root, "/usr/lib/python3.13/json/__init__.py"), None);
        assert_eq!(
            relative(root, "file:///repo/src/fuzzy.ts").as_deref(),
            Some("src/fuzzy.ts")
        );
        // A workload script elsewhere is user code, shown with its full path.
        assert_eq!(
            relative(root, "/tmp/bench.ts").as_deref(),
            Some("/tmp/bench.ts")
        );
        assert!(in_repo("src/fuzzy.ts") && !in_repo("/tmp/bench.ts"));
    }

    #[test]
    fn a_relative_script_path_under_the_repository_is_kept() {
        let root = std::env::temp_dir().join(format!("ra-profile-rel-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("bench.py"), "").unwrap();
        assert_eq!(relative(&root, "bench.py").as_deref(), Some("bench.py"));
        assert_eq!(relative(&root, "./bench.py").as_deref(), Some("bench.py"));
        assert_eq!(relative(&root, "missing.py"), None);
        assert_eq!(relative(&root, "<frozen runpy>"), None);
        assert_eq!(relative(&root, "~"), None);
        std::fs::remove_dir_all(&root).unwrap();
    }

    fn node(id: u64, name: &str, url: &str, line: i64, hits: u64, children: &[u64]) -> Value {
        json!({ "id": id, "hitCount": hits, "children": children,
                "callFrame": { "functionName": name, "url": url, "lineNumber": line - 1 } })
    }

    #[test]
    fn node_writes_profile_to_a_temp_path_with_spaces() {
        if !Command::new("node")
            .arg("--version")
            .status()
            .is_ok_and(|s| s.success())
        {
            return;
        }
        let dir = crate::app::temp_dir("profile with spaces").unwrap();
        let root = dir.canonicalize().unwrap();
        let command = ["node".into(), "-e".into(), "console.log('workload')".into()];
        let result = super::node(&root, &command, &root);
        std::fs::remove_dir_all(dir).unwrap();
        assert!(result.unwrap().1 > 0.0);
    }

    #[test]
    fn cpuprofile_times_and_callers() {
        let f = "file:///repo/src/search.ts";
        // (root) -> matched -> (anonymous) -> fuzzyScore -> fuzzyScore (recursion); 1 ms a sample.
        let profile = json!({
            "startTime": 0, "endTime": 10_000, "samples": vec![0; 10],
            "nodes": [
                node(1, "(root)", "", 0, 0, &[2]),
                node(2, "matched", f, 9, 1, &[3]),
                node(3, "", f, 10, 1, &[4]),
                node(4, "fuzzyScore", f, 2, 5, &[5]),
                node(5, "fuzzyScore", f, 2, 3, &[]),
            ],
        });
        let mut functions = HashMap::new();
        let ms = merge_cpuprofile(Path::new("/repo"), &profile, &mut functions);
        assert_eq!(ms, 10.0);
        let key = |n: &str, l: u32| ("src/search.ts".to_string(), n.to_string(), l);
        let fuzzy = &functions[&key("fuzzyScore", 2)];
        assert_eq!(
            (fuzzy.self_ms, fuzzy.total_ms),
            (8.0, 8.0),
            "recursion counts once"
        );
        // The caller is the named function, not the arrow callback in between.
        assert_eq!(
            fuzzy.callers.keys().collect::<Vec<_>>(),
            [&key("matched", 9)]
        );
        assert_eq!(functions[&key("matched", 9)].total_ms, 10.0);
        assert!(functions.contains_key(&key("(anonymous):10", 10)));
    }

    #[test]
    fn cpuprofile_recognizes_encoded_paths_in_the_repository() {
        let root = Path::new("/repo/work area");
        let profile = json!({
            "startTime": 0,
            "endTime": 10_000,
            "samples": [1],
            "nodes": [node(1, "hot", "file:///repo/work%20area/step%23one%25.mjs", 0, 1, &[])],
        });
        let mut functions = HashMap::new();
        merge_cpuprofile(root, &profile, &mut functions);
        assert!(
            functions.contains_key(&("step#one%.mjs".into(), "hot".into(), 0)),
            "{:?}",
            functions.keys()
        );
        assert_eq!(
            relative(root, "/repo/work area/step%20.mjs").as_deref(),
            Some("step%20.mjs")
        );
    }

    #[test]
    fn finds_the_argument_that_varies_per_item() {
        let root =
            std::env::temp_dir().join(format!("rust-anything-profile-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("search.ts"),
            "export function matched(query: string, all: string[]) {\n  return all.map(key => ({ key, score: fuzzyScore(query, key) }))\n}\n",
        )
        .unwrap();
        std::fs::write(
            root.join("search.py"),
            "def best(q, keys):\n    for k in keys:\n        yield score(q, k)\n",
        )
        .unwrap();
        let params = ["query".to_string(), "text".to_string()];
        let caller = |file: &str, name: &str| (file.to_string(), name.to_string(), 1);
        assert_eq!(
            batch_param(
                &root,
                &caller("search.ts", "matched"),
                "fuzzyScore",
                &params
            )
            .as_deref(),
            Some("text")
        );
        assert_eq!(
            batch_param(&root, &caller("search.py", "best"), "score", &params).as_deref(),
            Some("text")
        );
        // `query` is the same for every item: no batch parameter to give.
        assert_eq!(
            batch_param(&root, &caller("search.ts", "matched"), "missing", &params),
            None
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn go_names_and_pprof() {
        assert_eq!(go_name("example.com/strkit/strkit.Slugify"), "Slugify");
        assert_eq!(go_name("testing.(*B).runN"), "runN");
        assert_eq!(
            go_name("example.com/x/pkg.Outer.func2"),
            "(anonymous) in Outer"
        );
        let raw = "Samples:\nsamples/count cpu/nanoseconds\n          2   20000000: 1 2 \n          1   10000000: 3 2 \n\
                   Locations\n     1: 0x1 M=1 golang.org/x/text.Map /home/u/go/pkg/mod/golang.org/x/text/map.go:568:0 s=528\n\
                   \x20            example.com/x/pkg.Slugify /repo/pkg/a.go:21:0 s=18\n\
                   \x20    2: 0x2 M=1 example.com/x/pkg.slugAll /repo/pkg/a_test.go:24:0 s=21\n\
                   \x20    3: 0x3 M=1 example.com/x/pkg.Slugify /repo/pkg/a.go:25:0 s=18\nMappings\n";
        let samples = parse_pprof_raw(Path::new("/repo"), raw);
        assert_eq!(samples.len(), 2);
        let key = |f: &str, n: &str, l: u32| Some((f.to_string(), n.to_string(), l));
        // Root first; the inlined Slugify sits between slugAll and strings.Map (not user code).
        assert_eq!(
            samples[0],
            (
                vec![
                    key("pkg/a_test.go", "slugAll", 21),
                    key("pkg/a.go", "Slugify", 18),
                    None
                ],
                20.0
            )
        );
        let mut functions = HashMap::new();
        merge_samples(&samples, &mut functions);
        let slug = &functions[&("pkg/a.go".to_string(), "Slugify".to_string(), 18)];
        assert_eq!((slug.total_ms, slug.self_ms), (30.0, 10.0));
        assert_eq!(slug.callers.values().map(|c| c.0).sum::<f64>(), 30.0);
    }

    #[test]
    fn java_declarations() {
        let src = "class Strings {\n    public static String slugify(String text, int max) {\n        \
                   String s = slugify(text, 0);\n        return s;\n    }\n}\n";
        assert_eq!(java_declaration(src, "slugify", 4), Some(2));
        assert_eq!(java_declaration(src, "slugify", 1), None);
    }

    #[test]
    fn loop_variables_in_go_java_ruby() {
        let (items, indexes) = loop_variables(
            "for _, t := range texts {\n\tSlugify(t, 30)\n}\nfor i := range xs {}\n\
             for (String w : words) {}\nfor (int j = 0; j < n; j++) {}\ntexts.map { |x| f(x) }\nxs.each do |y, k|\n",
        );
        assert_eq!(items, ["t", "w", "x", "y"]);
        assert_eq!(
            indexes,
            ["_", "i", "j"]
                .into_iter()
                .filter(|v| *v != "_")
                .collect::<Vec<_>>()
        );
    }
}
