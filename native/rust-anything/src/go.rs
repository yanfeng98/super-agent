//! `go`: the hot-path loop in one command. Profile the workload, bridge the top portable
//! function (the outermost one, batched where `profile` says so), record it, hand the port to
//! an agent, then build, diff and gate it, and switch it on with `enable` only when all pass.
//! Run it again after porting by hand: every finished step is kept.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::bridge::Manifest;
use crate::lang::Lang;
use crate::profile::{Hot, Report};
use crate::walk::Canonical;

pub const TASK: &str = "TASK.md";

pub struct Options<'a> {
    pub root: &'a Path,
    pub command: &'a [String],
    pub lang: Option<Lang>,
    /// `claude`, `codex`, or a command that reads the prompt on stdin; `None` stops at the task.
    pub agent: Option<&'a str>,
    /// Port this function instead of the profile's pick.
    pub function: Option<&'a str>,
    /// Generated cases per function.
    pub count: usize,
    /// Agent turns: the first port, then one per failed diff or gate.
    pub rounds: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Pick {
    pub function: String,
    pub file: String,
    pub line: u32,
    pub total_pct: f64,
    /// `function:param` passed to `bridge --batch`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch: Option<String>,
    /// The caller that spends the most time in it: `name` and `file:line`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller: Option<(String, String)>,
    pub why: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// The task is written; port `src/port.rs` and run `go` again.
    WaitingForPort,
    /// Recorded cases still differ after the agent's turns.
    Mismatch,
    /// Every case matches but a gate check fails.
    GateFailed,
    /// Everything passed and the unit is switched on.
    On,
    /// Everything passed; this language is switched with the shadow module instead.
    Passed,
    /// Everything matches, but `bench` times the port slower than the original as its
    /// callers call it, so it stays off.
    Slower,
}

#[derive(Debug, Serialize)]
pub struct Outcome {
    pub stage: Stage,
    pub pick: Pick,
    pub crate_dir: PathBuf,
    pub task: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cases: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched: Option<usize>,
    /// `bench`'s geometric-mean speedup, call included, once every check passed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speedup: Option<f64>,
    pub failed_checks: Vec<String>,
    pub steps: Vec<String>,
    pub next: String,
}

/// The function to port: the first hot one (most time, so the outermost loop) that is exported
/// by a portable unit and takes arguments to record.
pub fn pick(report: &Report, function: Option<&str>) -> Result<Pick> {
    let eligible = |h: &&Hot| h.unit.is_some() && h.portable && h.exported && !h.params.is_empty();
    let hot = match function {
        Some(name) => report
            .hot
            .iter()
            .find(|h| h.name == name)
            .with_context(|| {
                format!("`{name}` is not among the hot functions of this run (see `rust-anything profile`)")
            })?,
        None => report.hot.iter().find(eligible).with_context(|| {
            format!(
                "no hot function to port: none of the {} hot functions is exported by a portable \
                 unit and takes arguments. `rust-anything profile` shows why.",
                report.hot.len()
            )
        })?,
    };
    if !eligible(&hot) {
        bail!("`{}` cannot be bridged: {}", hot.name, hot.advice);
    }
    let why = match &hot.batch {
        Some(spec) => format!(
            "{:.0}% of the run; called in a loop by `{}`, so it is bridged with `--batch {spec}`",
            hot.total_pct,
            hot.callers
                .first()
                .map_or("its caller", |c| c.name.as_str())
        ),
        None => format!(
            "{:.0}% of the run, the outermost portable function",
            hot.total_pct
        ),
    };
    Ok(Pick {
        function: hot.name.clone(),
        file: hot.file.clone(),
        line: hot.line,
        total_pct: hot.total_pct,
        batch: hot.batch.clone(),
        caller: hot
            .callers
            .first()
            .map(|c| (c.name.clone(), format!("{}:{}", c.file, c.line))),
        why,
    })
}

pub fn crate_name(function: &str) -> String {
    format!(
        "{}-rs",
        crate::bridge::snake_case(function).replace('_', "-")
    )
}

/// The other exported functions of the file, kept in the original so the crate ports one.
fn others(root: &Path, lang: Lang, file: &str, function: &str) -> Result<Vec<String>> {
    let text =
        std::fs::read_to_string(root.join(file)).with_context(|| format!("reading {file}"))?;
    let (fns, classes) = if lang == Lang::Ts {
        (
            crate::ts::exports::exported_functions(file, &text),
            crate::ts::exports::exported_classes(file, &text),
        )
    } else {
        crate::lang::exports(lang, file, &text)
    };
    Ok(fns
        .into_iter()
        .map(|f| f.name)
        .chain(classes.into_iter().map(|c| c.name))
        .filter(|n| n != function)
        .map(|n| format!("{n}: not hot in the profiled run; `go` ports one function"))
        .collect())
}

pub fn task(crate_dir: &Path, pick: &Pick, command: &[String]) -> String {
    let batch = pick.batch.as_ref().map_or_else(String::new, |spec| {
        let function = spec.split(':').next().unwrap_or(&pick.function);
        format!(
            "\nThe crate also has a batch class for `{spec}`, which runs `{function}` over many \
             items kept in Rust. A `{}Index` struct in `src/port.rs` can prepare the items once \
             (see the Hot paths section of rust-anything's docs/how-it-works.md); `rust-anything go` runs `rust-anything \
             refresh` before building, so the batch class uses it.\n",
            crate::codegen_batch::pascal(function)
        )
    });
    format!(
        "# Port `{function}` to Rust\n\n\
         `{function}` (`{file}:{line}`) took {pct:.0}% of `{command}`. Port it, and only it, in \
         `src/port.rs` of this crate ({dir}). The bindings, the recorded cases and \
         `rust-anything.json` are generated: leave them as they are.\n{batch}\n\
         ## Rules\n\n\
         - Match the original exactly on every recorded case in `cases.jsonl`: values, errors and \
           their messages, edge cases (empty input, Unicode, large numbers). Read the original \
           first; port its behavior, not a cleaner idea of it.\n\
         - No `unwrap`, `expect` or `panic!` outside tests; no `unsafe` without a `// SAFETY:` comment.\n\
         - Keep helpers private; only the unit's functions are `pub`.\n\n\
         ## Loop\n\n\
         ```sh\n\
         rust-anything build .\n\
         rust-anything diff .     # every recorded case must match\n\
         rust-anything gate .     # fmt, clippy, tests, unsafe, API, diff\n\
         ```\n\n\
         Done when `diff` matches every case and `gate` passes. Do not switch the port on; \
         `rust-anything go` does that once both pass.\n",
        function = pick.function,
        file = pick.file,
        line = pick.line,
        pct = pick.total_pct,
        command = command.join(" "),
        dir = crate_dir.display(),
    )
}

/// The agent's command line: `claude` and `codex` run headless with edits allowed in the crate;
/// anything else runs as given, reading the prompt on stdin.
pub fn agent_command(agent: &str) -> Result<Command> {
    let words: Vec<&str> = agent.split_whitespace().collect();
    let (program, args) = words.split_first().context("--agent is empty")?;
    let mut cmd = Command::new(program);
    match *program {
        "claude" if args.is_empty() => {
            cmd.args([
                "-p",
                "--permission-mode",
                "acceptEdits",
                "--allowedTools",
                "Bash(cargo:*)",
                "Bash(rust-anything:*)",
            ]);
        }
        "codex" if args.is_empty() => {
            cmd.args(["exec", "--full-auto", "-"]);
        }
        _ => {
            cmd.args(args);
        }
    }
    Ok(cmd)
}

fn say(steps: &mut Vec<String>, line: String) {
    eprintln!("go: {line}");
    steps.push(line);
}

pub fn go(opts: &Options) -> Result<Outcome> {
    let root = opts
        .root
        .canonical()
        .with_context(|| format!("resolving {}", opts.root.display()))?;
    let lang = opts.lang.unwrap_or_else(|| Lang::detect(&root));
    let mut steps = Vec::new();

    let report = crate::profile::profile(&root, opts.command, Some(lang))?;
    let pick = pick(&report, opts.function)?;
    say(
        &mut steps,
        format!(
            "profiled `{}` ({:.0} ms): porting `{}` ({}:{}), {}",
            opts.command.join(" "),
            report.total_ms,
            pick.function,
            pick.file,
            pick.line,
            pick.why
        ),
    );

    let name = crate_name(&pick.function);
    let crate_dir = root.join("rust").join(&name);
    if crate_dir.join("rust-anything.json").is_file() {
        let m = Manifest::load(&crate_dir)?;
        if !m.exports.iter().any(|f| f.name == pick.function) {
            bail!(
                "{} exists for other functions ({}); move it aside or pass --function",
                crate_dir.display(),
                m.exports
                    .iter()
                    .map(|f| f.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        say(
            &mut steps,
            format!("kept the crate at {}", crate_dir.display()),
        );
    } else {
        let outcome = crate::bridge::bridge(&crate::bridge::Options {
            root: &root,
            unit: &pick.file,
            kind: None,
            out: Some(crate_dir.clone()),
            name: Some(name.clone()),
            force: false,
            keep: others(&root, lang, &pick.file, &pick.function)?,
            lang: Some(lang),
            batch: pick.batch.iter().cloned().collect(),
            as_json: Vec::new(),
            allocator: crate::bridge::Allocator::default(),
        })?;
        say(
            &mut steps,
            format!(
                "bridged {} ({:?}) at {}",
                outcome.exports.join(", "),
                outcome.kind,
                outcome.crate_dir.display()
            ),
        );
    }
    let manifest = Manifest::load(&crate_dir)?;

    let cases = crate_dir.join(crate::harness::CASES);
    let stale = crate::recording::stale(&crate_dir, &manifest);
    if !cases.is_file() || stale.is_some() {
        let captured = match manifest.language {
            Lang::Ts | Lang::Python => {
                let c = crate::from_tests::capture(&crate_dir, opts.command)?;
                let n: usize = c.per_function.values().sum();
                say(
                    &mut steps,
                    format!("captured {n} real call(s) from the workload"),
                );
                (n > 0).then_some(c.inputs)
            }
            _ => None,
        };
        let recorded = crate::harness::record(&crate::harness::RecordOptions {
            crate_dir: &crate_dir,
            count: opts.count,
            inputs: captured.as_deref(),
            module: None,
            resolve_from: None,
            node_args: &[],
        })?;
        say(
            &mut steps,
            format!(
                "recorded {} case(s) ({} throw in the original)",
                recorded.cases, recorded.errors
            ),
        );
    } else {
        say(&mut steps, "kept the recorded cases".into());
    }

    let task_path = crate_dir.join(TASK);
    let task = task(&crate_dir, &pick, opts.command);
    std::fs::write(&task_path, &task)
        .with_context(|| format!("writing {}", task_path.display()))?;
    let outcome = |stage, steps, cases, matched, failed_checks, next: String| Outcome {
        stage,
        pick: pick.clone(),
        crate_dir: crate_dir.clone(),
        task: task_path.clone(),
        cases,
        matched,
        speedup: None,
        failed_checks,
        steps,
        next,
    };

    let mut prompt = task.clone();
    let rounds = if opts.agent.is_some() {
        opts.rounds.max(1)
    } else {
        0
    };
    let mut round = 0;
    let mut need_port = !crate::gate::ported_check(&crate_dir).passed;
    let (cases, matched) = loop {
        if need_port {
            let Some(agent) = opts.agent.filter(|_| round < rounds) else {
                say(&mut steps, format!("wrote {}", task_path.display()));
                return Ok(outcome(
                    Stage::WaitingForPort,
                    steps,
                    None,
                    None,
                    Vec::new(),
                    format!(
                        "Port `{}` in {} as {} says (or pass --agent claude), then run the same \
                         `rust-anything go` again.",
                        pick.function,
                        crate_dir.join("src/port.rs").display(),
                        task_path.display()
                    ),
                ));
            };
            round += 1;
            say(
                &mut steps,
                format!("handing the port to `{agent}` (turn {round} of {rounds})"),
            );
            let mut cmd = agent_command(agent)?;
            cmd.current_dir(&crate_dir).stdout(std::io::stderr());
            let status = crate::process::status_with_input(
                &mut cmd,
                prompt.as_bytes(),
                crate::process::TESTS,
            )?;
            if !status.success() {
                bail!("`{agent}` failed ({status})");
            }
            if !crate::gate::ported_check(&crate_dir).passed {
                say(&mut steps, "the agent left stubs in src/port.rs".into());
                prompt = format!(
                    "{task}\n\nsrc/port.rs still has `todo!`, `unimplemented!` or functions that only panic. Finish them.\n"
                );
                continue;
            }
        }

        let (cases, matched, problems) = check(&crate_dir, &mut steps)?;
        if problems.is_empty() {
            break (cases, matched);
        }
        if round >= rounds {
            let stage = if matched < cases {
                Stage::Mismatch
            } else {
                Stage::GateFailed
            };
            return Ok(outcome(
                stage,
                steps,
                Some(cases),
                Some(matched),
                problems,
                format!(
                    "Fix the port ({}), then run the same `rust-anything go` again. The unit stays off.",
                    task_path.display()
                ),
            ));
        }
        prompt = format!(
            "{task}\n\nYour last port does not pass yet. Fix it:\n\n{}\n",
            problems.join("\n\n")
        );
        need_port = true;
    };

    let rows = crate::harness::bench(&crate::harness::BenchOptions {
        crate_dir: &crate_dir,
        min_ms: 200,
        resolve_from: None,
        node_args: &[],
    })?;
    let speedup = crate::harness::verdict(&rows, manifest.language).speedup;
    say(
        &mut steps,
        format!(
            "bench: {} as its callers call it, call included",
            times(speedup)
        ),
    );
    if speedup < 1.0 {
        let next = slower_advice(&pick, speedup);
        let mut out = outcome(
            Stage::Slower,
            steps,
            Some(cases),
            Some(matched),
            Vec::new(),
            next,
        );
        out.speedup = Some(speedup);
        return Ok(out);
    }

    let mut out = if crate::enable::is_supported(&manifest) {
        let on = crate::enable::enable(&crate_dir)?;
        say(
            &mut steps,
            format!(
                "switched on: {} {} in Rust now ({}); `rust-anything disable {}` turns it off",
                on.functions.join(", "),
                if on.functions.len() == 1 {
                    "runs"
                } else {
                    "run"
                },
                on.files
                    .iter()
                    .map(|f| f.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                crate_dir.display()
            ),
        );
        outcome(
            Stage::On,
            steps,
            Some(cases),
            Some(matched),
            Vec::new(),
            format!(
                "Measure it: `rust-anything report {}` writes REPORT.md and card.svg.",
                crate_dir.display()
            ),
        )
    } else {
        outcome(
            Stage::Passed,
            steps,
            Some(cases),
            Some(matched),
            Vec::new(),
            format!(
                "Every case matches and gate passes. Switch it in with the shadow module \
                 (`rust-anything shadow {}`), then `rust-anything report` it.",
                crate_dir.display()
            ),
        )
    };
    out.speedup = Some(speedup);
    Ok(out)
}

fn times(x: f64) -> String {
    if x >= 1.0 {
        format!("{x:.1}x faster")
    } else {
        format!("{:.1}x slower", 1.0 / x.max(1e-9))
    }
}

/// What to do when every case matches but the port, called as its callers call it, loses.
pub fn slower_advice(pick: &Pick, speedup: f64) -> String {
    let caller = pick.caller.as_ref().map_or_else(
        || "its caller".to_string(),
        |(n, at)| format!("`{n}` ({at})"),
    );
    let lost = times(speedup);
    match &pick.batch {
        Some(spec) => {
            let function = spec.split(':').next().unwrap_or(&pick.function);
            format!(
                "Every case matches, but called once per item the port is {lost} than the \
                 original, so it stays off. The speed is in the batch: in {caller}, build \
                 `{}Batch` once over the items and call it per query (the crate's type \
                 declarations list its methods), or port {caller} with it. Then `rust-anything enable` and \
                 `rust-anything report`.",
                crate::codegen_batch::pascal(function)
            )
        }
        None => format!(
            "Every case matches, but the port is {lost} than the original as {caller} calls it: \
             each call into Rust costs more than its work. It stays off. Port {caller} with it \
             (`rust-anything go --function <name>`), or keep it in the original."
        ),
    }
}

/// Build, diff and gate: the cases, how many match, and what still fails.
fn check(crate_dir: &Path, steps: &mut Vec<String>) -> Result<(usize, usize, Vec<String>)> {
    if !Manifest::load(crate_dir)?.batches.is_empty()
        && let Err(e) = crate::bridge::refresh(crate_dir)
    {
        say(steps, "refresh failed".into());
        return Ok((0, 0, vec![format!("`rust-anything refresh` fails:\n{e:#}")]));
    }
    if let Err(e) = crate::harness::build(crate_dir) {
        say(steps, "the build failed".into());
        return Ok((0, 0, vec![format!("The build fails:\n{e:#}")]));
    }
    let diff = crate::harness::diff(&crate::harness::DiffOptions {
        crate_dir,
        module: None,
        ignore_key_order: false,
        node_args: &[],
        float_tolerance: None,
    })?;
    say(
        steps,
        format!("diff: {}/{} cases match", diff.passed, diff.total),
    );
    if diff.failed > 0 || !diff.missing.is_empty() || diff.total == 0 {
        let shown: Vec<String> = diff
            .mismatches
            .iter()
            .take(5)
            .map(|m| serde_json::to_string(m).unwrap_or_default())
            .collect();
        return Ok((
            diff.total,
            diff.passed,
            vec![format!(
                "{} of {} recorded cases differ{}. First mismatches:\n{}",
                diff.total - diff.passed,
                diff.total,
                if diff.missing.is_empty() {
                    String::new()
                } else {
                    format!("; missing from Rust: {}", diff.missing.join(", "))
                },
                shown.join("\n")
            )],
        ));
    }
    let gate = crate::gate::gate(crate_dir, &[])?;
    let failed: Vec<String> = gate
        .checks
        .iter()
        .filter(|c| !c.passed)
        .map(|c| format!("gate check `{}` fails: {}", c.name, c.detail))
        .collect();
    say(
        steps,
        if failed.is_empty() {
            "gate passed".into()
        } else {
            format!("gate: {} check(s) fail", failed.len())
        },
    );
    Ok((diff.total, diff.passed, failed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::Caller;

    fn hot(name: &str, pct: f64, unit: bool, params: &[&str], batch: Option<&str>) -> Hot {
        Hot {
            name: name.into(),
            file: "app.py".into(),
            line: 1,
            unit: unit.then(|| "app.py".into()),
            exported: true,
            self_ms: 1.0,
            total_ms: pct,
            total_pct: pct,
            calls: None,
            ns_per_call: None,
            callers: vec![Caller {
                name: "main".into(),
                file: "app.py".into(),
                line: 9,
                ms: pct,
                calls: None,
            }],
            advice: "Hot, outside the analyzed units.".into(),
            portable: unit,
            params: params.iter().map(|p| p.to_string()).collect(),
            batch: batch.map(str::to_string),
        }
    }

    fn report(hot: Vec<Hot>) -> Report {
        Report {
            schema: crate::profile::SCHEMA,
            language: Lang::Python,
            command: vec!["python3".into(), "app.py".into()],
            total_ms: 100.0,
            repo_pct: 100.0,
            hot,
            compiled: Vec::new(),
        }
    }

    #[test]
    fn the_outermost_portable_function_with_arguments_wins() {
        let r = report(vec![
            hot("main", 99.0, true, &[], None),
            hot("load", 95.0, false, &["path"], None),
            hot("dedupe", 90.0, true, &["names", "limit"], None),
            hot(
                "levenshtein",
                85.0,
                true,
                &["a", "b"],
                Some("levenshtein:b"),
            ),
        ]);
        let p = pick(&r, None).unwrap();
        assert_eq!((p.function.as_str(), p.batch), ("dedupe", None));
        assert!(p.why.contains("outermost"), "{}", p.why);
    }

    #[test]
    fn a_loop_the_profile_batches_is_bridged_with_its_batch() {
        let r = report(vec![
            hot("main", 99.0, true, &[], None),
            hot(
                "levenshtein",
                85.0,
                true,
                &["a", "b"],
                Some("levenshtein:b"),
            ),
        ]);
        let p = pick(&r, None).unwrap();
        assert_eq!(p.batch.as_deref(), Some("levenshtein:b"));
        assert!(p.why.contains("--batch levenshtein:b"), "{}", p.why);
    }

    #[test]
    fn a_named_function_must_be_portable() {
        let r = report(vec![hot("load", 95.0, false, &["path"], None)]);
        assert!(pick(&r, Some("load")).is_err());
        assert!(pick(&r, Some("missing")).is_err());
        assert!(pick(&r, None).is_err());
    }

    #[test]
    fn agents_run_headless_and_others_as_given() {
        let args = |c: &Command| -> Vec<String> {
            c.get_args()
                .map(|a| a.to_string_lossy().into_owned())
                .collect()
        };
        let claude = agent_command("claude").unwrap();
        assert_eq!(claude.get_program(), "claude");
        assert_eq!(
            args(&claude)[..3],
            ["-p", "--permission-mode", "acceptEdits"]
        );
        assert_eq!(
            args(&agent_command("codex").unwrap()),
            ["exec", "--full-auto", "-"]
        );
        let own = agent_command("my-agent --fast").unwrap();
        assert_eq!(
            (own.get_program(), args(&own)),
            ("my-agent".as_ref(), vec!["--fast".to_string()])
        );
        assert!(agent_command("  ").is_err());
    }

    #[test]
    fn a_slower_port_stays_off_and_says_where_the_speed_is() {
        let r = report(vec![hot(
            "score",
            85.0,
            true,
            &["q", "text"],
            Some("score:text"),
        )]);
        let batched = slower_advice(&pick(&r, None).unwrap(), 0.13);
        assert!(batched.contains("7.7x slower"), "{batched}");
        assert!(batched.contains("`ScoreBatch`"), "{batched}");
        assert!(batched.contains("`main` (app.py:9)"), "{batched}");
        let r = report(vec![hot("dedupe", 85.0, true, &["names"], None)]);
        let plain = slower_advice(&pick(&r, None).unwrap(), 0.5);
        assert!(
            plain.contains("2.0x slower") && plain.contains("--function"),
            "{plain}"
        );
        assert_eq!(times(31.42), "31.4x faster");
    }

    #[test]
    fn the_other_functions_and_classes_of_the_file_stay_in_the_original() {
        let root = std::env::temp_dir().join(format!("ra-go-others-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("lib.py"),
            "def hot(xs):\n    return xs\n\n\ndef cold(x):\n    return x\n\n\nclass Table:\n    def __init__(self, n):\n        self.n = n\n\n    def render(self):\n        return str(self.n)\n",
        )
        .unwrap();
        let kept = others(&root, Lang::Python, "lib.py", "hot").unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        let names: Vec<&str> = kept.iter().map(|k| k.split(':').next().unwrap()).collect();
        assert_eq!(names, ["cold", "Table"], "{kept:?}");
    }

    #[test]
    fn crate_names_follow_the_function() {
        assert_eq!(crate_name("fuzzyScore"), "fuzzy-score-rs");
        assert_eq!(crate_name("dedupe_names"), "dedupe-names-rs");
    }
}
