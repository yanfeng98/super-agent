//! `report`: a unit's result as Markdown and an SVG card, for a pull request or a README.
//!
//! Every number comes from a run made for the report: `diff` for the recorded cases that
//! match, `bench` for the speedup, and the cases of functions the Rust module does not provide
//! for the fallback rate. The machine and its core count go next to the timing.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::bridge::Manifest;
use crate::harness::{self, BenchRow};
use crate::walk::Canonical;

pub const SCHEMA: &str = "rust-anything/report@1";

#[derive(Debug, Serialize)]
pub struct Card {
    pub schema: &'static str,
    pub crate_name: String,
    pub unit: String,
    pub language: String,
    pub functions: Vec<String>,
    pub cases: usize,
    pub matched: usize,
    /// Geometric mean of `original_ns / rust_ns` over the timed functions, call included.
    pub speedup: f64,
    pub bench: Vec<BenchRow>,
    /// Recorded cases whose function the Rust module does not provide, so the original answers.
    pub fallback_cases: usize,
    /// `Some(true)` while `enable` has switched the unit on; `None` where `enable` does not apply.
    pub enabled: Option<bool>,
    pub machine: String,
    pub markdown: PathBuf,
    pub svg: PathBuf,
}

impl Card {
    pub fn fallback_rate(&self) -> f64 {
        if self.cases == 0 {
            0.0
        } else {
            self.fallback_cases as f64 / self.cases as f64
        }
    }
}

pub struct Options<'a> {
    pub crate_dir: &'a Path,
    pub out: Option<&'a Path>,
    pub min_ms: u64,
    pub node_args: &'a [String],
}

pub fn report(opts: &Options) -> Result<Card> {
    let crate_dir = opts
        .crate_dir
        .canonical()
        .with_context(|| format!("resolving {}", opts.crate_dir.display()))?;
    let manifest = Manifest::load(&crate_dir)?;
    harness::build(&crate_dir)?;
    let diff = harness::diff(&harness::DiffOptions {
        crate_dir: &crate_dir,
        module: None,
        ignore_key_order: false,
        node_args: opts.node_args,
        float_tolerance: None,
    })?;
    let bench = harness::bench(&harness::BenchOptions {
        crate_dir: &crate_dir,
        min_ms: opts.min_ms,
        resolve_from: None,
        node_args: opts.node_args,
    })?;
    let verdict = harness::verdict(&bench, manifest.language);
    let per_function = cases_per_function(&crate_dir);
    let fallback_cases = diff
        .missing
        .iter()
        .map(|f| per_function.get(f).copied().unwrap_or(0))
        .sum();
    let out = opts
        .out
        .map_or_else(|| crate_dir.join("report"), Path::to_path_buf);
    std::fs::create_dir_all(&out).with_context(|| format!("creating {}", out.display()))?;
    let mut card = Card {
        schema: SCHEMA,
        crate_name: manifest.crate_name.clone(),
        unit: manifest.unit.clone(),
        language: manifest.language.name().to_string(),
        functions: manifest.exports.iter().map(|f| f.name.clone()).collect(),
        cases: diff.total,
        matched: diff.passed,
        speedup: verdict.speedup,
        bench,
        fallback_cases,
        enabled: crate::enable::is_enabled(&crate_dir).ok(),
        machine: machine(),
        markdown: out.join("REPORT.md"),
        svg: out.join("card.svg"),
    };
    card.enabled = card
        .enabled
        .filter(|_| crate::enable::is_supported(&manifest));
    std::fs::write(&card.svg, svg(&card))?;
    std::fs::write(&card.markdown, markdown(&card))?;
    Ok(card)
}

fn cases_per_function(crate_dir: &Path) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    let text = std::fs::read_to_string(crate_dir.join(harness::CASES)).unwrap_or_default();
    for line in text.lines() {
        if let Ok(case) = serde_json::from_str::<serde_json::Value>(line)
            && let Some(name) = case["fn"].as_str()
        {
            *out.entry(name.to_string()).or_default() += 1;
        }
    }
    out
}

/// The CPU's name and its core count, as `bench` numbers must be published with.
pub fn machine() -> String {
    let cores = std::thread::available_parallelism().map_or(0, std::num::NonZero::get);
    let mut cmd = std::process::Command::new("sysctl");
    cmd.args(["-n", "machdep.cpu.brand_string"]);
    let name = crate::process::output(&mut cmd, crate::process::QUICK)
        .ok()
        .filter(crate::process::Output::success)
        .map(|o| o.stdout_text().trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::fs::read_to_string("/proc/cpuinfo").ok().and_then(|t| {
                t.lines()
                    .find(|l| l.starts_with("model name"))
                    .and_then(|l| l.split_once(':'))
                    .map(|(_, v)| v.trim().to_string())
            })
        })
        .unwrap_or_else(|| std::env::consts::ARCH.to_string());
    let os = match std::env::consts::OS {
        "macos" => "macOS",
        "linux" => "Linux",
        "windows" => "Windows",
        other => other,
    };
    format!("{name}, {cores} cores, {os}")
}

fn times(x: f64) -> String {
    if x >= 10.0 {
        format!("{x:.0}x")
    } else {
        format!("{x:.1}x")
    }
}

fn ns(v: f64) -> String {
    if v >= 1e9 {
        format!("{:.2} s", v / 1e9)
    } else if v >= 1e6 {
        format!("{:.1} ms", v / 1e6)
    } else if v >= 1e3 {
        format!("{:.1} µs", v / 1e3)
    } else {
        format!("{v:.0} ns")
    }
}

fn state(card: &Card) -> &'static str {
    match card.enabled {
        Some(true) => "on",
        Some(false) => "off",
        None => "not switched by `enable`",
    }
}

pub fn markdown(card: &Card) -> String {
    let mut out = format!(
        "## `{}` in Rust\n\n![{} report card](card.svg)\n\n\
         | | |\n| --- | --- |\n\
         | Unit | `{}` ({}) |\n\
         | Recorded cases matched | {} / {} |\n\
         | Speedup, call included | {} (geometric mean) |\n\
         | Fallback to the original | {:.1}% ({} of {} cases) |\n\
         | Switched on | {} |\n\n",
        card.crate_name,
        card.crate_name,
        card.unit,
        card.language,
        card.matched,
        card.cases,
        times(card.speedup),
        100.0 * card.fallback_rate(),
        card.fallback_cases,
        card.cases,
        state(card),
    );
    if !card.bench.is_empty() {
        out.push_str(
            "| Function | Cases | Original per call | Rust per call | Speedup |\n| --- | ---: | ---: | ---: | ---: |\n",
        );
        for r in &card.bench {
            let _ = writeln!(
                out,
                "| `{}` | {} | {} | {} | {} |",
                r.name,
                r.cases,
                ns(r.original_ns),
                ns(r.rust_ns),
                times(r.speedup)
            );
        }
        out.push('\n');
    }
    let _ = writeln!(
        out,
        "Measured with `rust-anything report` on {}: `diff` replays every recorded case, `bench` \
         times them on both sides as callers call them. Reproduce: `rust-anything diff` and \
         `rust-anything bench` in the crate folder.",
        card.machine
    );
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Light card in the v3 style: white, an outlined panel for the original, a tinted orange one
/// for Rust, the headline result below.
pub fn svg(card: &Card) -> String {
    const W: u32 = 1200;
    const H: u32 = 600;
    let font = "-apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif";
    let mono = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";
    let (orig, rust) = card
        .bench
        .iter()
        .fold((0.0, 0.0), |(o, r), b| (o + b.original_ns, r + b.rust_ns));
    let max = orig.max(rust).max(1.0);
    let bar = |v: f64| (5.0_f64).max(470.0 * v / max);
    let all = card.matched == card.cases && card.cases > 0;
    let headline = if all {
        format!(
            "Same result as the original on all {} recorded cases",
            card.cases
        )
    } else {
        format!(
            "{} of {} recorded cases match the original",
            card.matched, card.cases
        )
    };
    let per_call = if card.bench.len() == 1 {
        "per call"
    } else {
        "per call, all functions"
    };
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" role="img" aria-label="{label}">
<rect width="{W}" height="{H}" fill="#ffffff"/>
<g font-family="{font}">
<svg x="64" y="52" width="60" height="60" viewBox="0 0 200 200">{crab}</svg>
<text x="140" y="94" font-size="34" font-weight="700" fill="#1a1a1a">rust-anything</text>
<text x="380" y="94" font-size="{title_size}" font-weight="700" fill="#1a1a1a">{name}, ported to <tspan fill="#e8561a">Rust</tspan></text>
<text x="64" y="140" font-size="18" fill="#666666">{unit} ({lang}) · {functions}</text>
<rect x="64" y="172" width="520" height="230" rx="20" fill="#ffffff" stroke="#e5e5e5" stroke-width="3"/>
<text x="96" y="218" font-size="26" font-weight="700" fill="#1a1a1a">Original</text>
<text x="552" y="218" font-size="17" fill="#666666" text-anchor="end">{per_call}</text>
<text x="96" y="318" font-family="{mono}" font-size="64" font-weight="700" fill="#1a1a1a">{orig_t}</text>
<rect x="96" y="360" width="{orig_w:.0}" height="14" rx="7" fill="#9e9e9e"/>
<rect x="616" y="172" width="520" height="230" rx="20" fill="#fdf4ef" stroke="#f2b89a" stroke-width="3"/>
<text x="648" y="218" font-size="26" font-weight="700" fill="#1a1a1a">Rust port</text>
<text x="1104" y="218" font-size="17" fill="#666666" text-anchor="end">switched {state}</text>
<text x="648" y="318" font-family="{mono}" font-size="64" font-weight="700" fill="#e8561a">{rust_t}</text>
<rect x="648" y="360" width="{rust_w:.0}" height="14" rx="7" fill="#e8561a"/>
<text x="600" y="460" font-size="28" fill="#1a1a1a" text-anchor="middle">{headline}: <tspan font-weight="700" fill="#e8561a">{speed}</tspan></text>
<text x="600" y="500" font-size="18" fill="#777777" text-anchor="middle">Fallback to the original: {fallback:.1}% of cases · call included · {machine}</text>
<text x="600" y="556" font-size="17" fill="#888888" text-anchor="middle">github.com/erkamyaman/rust-anything</text>
</g>
</svg>
"##,
        label = esc(&format!(
            "{}: {} of {} recorded cases match, {} faster",
            card.crate_name,
            card.matched,
            card.cases,
            times(card.speedup)
        )),
        name = esc(&card.crate_name),
        title_size = if card.crate_name.chars().count() > 22 {
            24
        } else {
            34
        },
        unit = esc(&card.unit),
        lang = esc(&card.language),
        functions = esc(&card.functions.join(", ")),
        orig_t = esc(&ns(orig)),
        rust_t = esc(&ns(rust)),
        orig_w = bar(orig),
        rust_w = bar(rust),
        state = if card.enabled == Some(true) {
            "on"
        } else {
            "off"
        },
        speed = if card.speedup >= 1.0 {
            format!("{} faster", times(card.speedup))
        } else {
            format!("{} slower", times(1.0 / card.speedup.max(1e-9)))
        },
        crab = include_str!("card_crab.svg").trim(),
        fallback = 100.0 * card.fallback_rate(),
        machine = esc(&card.machine),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> Card {
        Card {
            schema: SCHEMA,
            crate_name: "fuzzy-rs".into(),
            unit: "app.py".into(),
            language: "Python".into(),
            functions: vec!["dedupe".into()],
            cases: 412,
            matched: 412,
            speedup: 31.4,
            bench: vec![BenchRow {
                name: "dedupe".into(),
                cases: 412,
                original_ns: 9_120_000.0,
                rust_ns: 290_000.0,
                speedup: 31.4,
            }],
            fallback_cases: 0,
            enabled: Some(true),
            machine: "Apple M4, 10 cores, macOS".into(),
            markdown: "REPORT.md".into(),
            svg: "card.svg".into(),
        }
    }

    #[test]
    fn the_card_shows_what_was_measured() {
        let svg = svg(&card());
        assert!(svg.starts_with("<svg"), "{svg}");
        for want in [
            "fuzzy-rs",
            "9.1 ms",
            "290.0 µs",
            "all 412 recorded cases",
            "31x faster",
            "switched on",
            "0.0% of cases",
            "Apple M4, 10 cores",
        ] {
            assert!(svg.contains(want), "{want} missing:\n{svg}");
        }
        let md = markdown(&card());
        assert!(
            md.contains("| Recorded cases matched | 412 / 412 |"),
            "{md}"
        );
        assert!(
            md.contains("| `dedupe` | 412 | 9.1 ms | 290.0 µs | 31x |"),
            "{md}"
        );
    }

    #[test]
    fn text_from_the_project_is_escaped() {
        let mut c = card();
        c.unit = "src/<a&b>.ts".into();
        c.matched = 400;
        let svg = svg(&c);
        assert!(svg.contains("src/&lt;a&amp;b&gt;.ts"));
        assert!(svg.contains("400 of 412 recorded cases match"));
    }
}
