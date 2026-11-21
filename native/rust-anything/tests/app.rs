//! `app record` and `app replay` on the same small CLI written in each of the six languages.
//!
//! Each test copies its fixture to a temp folder, records the original, replays the original
//! (everything matches), then replays a build whose division-by-zero message changed (only those
//! cases fail, with a diff in the report). A test is skipped when its runtime is missing.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use rust_anything::app::{self, RecordOptions, ReplayOptions};
use rust_anything::front_door::{self, FrontDoorOptions, WireOptions};
use rust_anything::lift::{self, LiftOptions};

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn fixture(lang: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let repo = std::env::temp_dir().join(format!(
        "rust-anything-app-test-{lang}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&repo);
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/cli")
            .join(lang),
        &repo,
    );
    repo
}

fn sh(repo: &Path, program: &str, args: &[&str]) {
    let out = Command::new(program)
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{program} {args:?}: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn add_bug(repo: &Path, source: &str, copy_to: Option<&str>) {
    let text = std::fs::read_to_string(repo.join(source)).unwrap();
    assert!(text.contains("division by zero"));
    std::fs::write(
        repo.join(copy_to.unwrap_or(source)),
        text.replace("division by zero", "divide by zero"),
    )
    .unwrap();
}

fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_string).collect()
}

fn record_then_replay(repo: &Path, run: &str, buggy: &str) {
    let outcome = app::record(&RecordOptions {
        repo,
        run: &words(run),
        inputs: None,
        normalize: &[],
        name: None,
        traps: true,
        timeout_secs: None,
        http: false,
        server_args: &[],
        fresh: &[],
        pace_ms: None,
        port: None,
        float_tolerance: None,
        float_files: &[],
        effects: None,
    })
    .unwrap();
    assert_eq!(outcome.examples, 7, "{outcome:?}");
    assert!(outcome.traps > 30, "{outcome:?}");
    assert!(outcome.unstable.is_empty(), "{outcome:?}");
    assert!(outcome.timed_out.is_empty(), "{outcome:?}");
    assert_eq!(outcome.noisy, ["example-006-now"], "{outcome:?}");

    let dir = app::cases_dir(repo);
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    assert_eq!(
        read("example-006-now.stdout").lines().collect::<Vec<_>>(),
        ["now: [..]"]
    );
    let div = read("example-003-div.toml");
    assert!(div.contains("args = [\"div\", \"7\", \"2\"]"), "{div}");
    assert!(div.contains("status.code = 0"), "{div}");
    assert_eq!(
        read("example-003-div.stdout").lines().collect::<Vec<_>>(),
        ["7 / 2 = 3 remainder 1"]
    );
    assert_eq!(read("example-004-words.stdin"), "one two three\n");
    assert_eq!(
        read("example-004-words.stdout").lines().collect::<Vec<_>>(),
        ["3"]
    );
    assert!(dir.join("example-005-copy.in/input.txt").is_file());
    assert!(
        read("example-005-copy.out/out.txt").starts_with("GR"),
        "the copy wrote its output file"
    );
    assert!(read("example-007-nope.toml").contains("status.code = 1"));
    assert!(read("example-007-nope.stderr").starts_with("usage: tcli"));

    let original = app::replay(&ReplayOptions {
        case: None,
        repo,
        build: None,
        timeout_secs: None,
        port: None,
    })
    .unwrap();
    assert_eq!(original.total, outcome.cases);
    assert_eq!(
        original.passed,
        original.total,
        "{}",
        std::fs::read_to_string(&original.report).unwrap()
    );
    assert!(original.fallback.is_none());

    let bug = app::replay(&ReplayOptions {
        case: None,
        repo,
        build: Some(buggy),
        timeout_secs: None,
        port: None,
    })
    .unwrap();
    assert!(!bug.failures.is_empty());
    assert_eq!(bug.passed + bug.failures.len(), bug.total);
    for f in &bug.failures {
        assert_eq!(f.args[0], "div", "{f:?}");
        assert!(f.problems.iter().all(|p| p.field == "stderr"), "{f:?}");
    }
    let report = std::fs::read_to_string(&bug.report).unwrap();
    assert!(report.contains("- error: division by zero"), "{report}");
    assert!(report.contains("+ error: divide by zero"), "{report}");

    let one = app::replay(&ReplayOptions {
        case: Some(&bug.failures[0].case),
        repo,
        build: Some(buggy),
        timeout_secs: None,
        port: None,
    })
    .unwrap();
    assert_eq!((one.total, one.failures.len()), (1, 1));
    let missing = app::replay(&ReplayOptions {
        case: Some("no-such-case"),
        repo,
        build: None,
        timeout_secs: None,
        port: None,
    });
    assert!(missing.unwrap_err().to_string().contains("no case named"));

    front_door_then_wire(repo);
    std::fs::remove_dir_all(repo).unwrap();
}

const WORDCOUNT_PORT: &str = "pub fn count_words(text: &[u8]) -> usize {
    text.split(|b| b.is_ascii_whitespace() || *b == 0x0b)
        .filter(|w| !w.is_empty())
        .count()
}
";

const WORDCOUNT_GLUE: &str = "use std::io::Read;
use std::process::ExitCode;

use clap::ArgMatches;

pub fn run(_command: &str, _matches: &ArgMatches) -> ExitCode {
    let mut input = Vec::new();
    if std::io::stdin().read_to_end(&mut input).is_err() {
        return ExitCode::from(1);
    }
    println!(\"{}\", wordcount_rs::port::count_words(&input));
    ExitCode::SUCCESS
}
";

const WORDCOUNT_DECLINING_GLUE: &str = "use std::io::Read;
use std::process::ExitCode;

use clap::ArgMatches;

pub fn run(_command: &str, _matches: &ArgMatches) -> ExitCode {
    let mut input = Vec::new();
    if std::io::stdin().read_to_end(&mut input).is_err() {
        return ExitCode::from(1);
    }
    if input.starts_with(b\"#original\") {
        let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
        return crate::original::run_with_stdin(&argv[1..], &input);
    }
    println!(\"{}\", wordcount_rs::port::count_words(&input));
    ExitCode::SUCCESS
}
";

/// The front door answers everything through the original; then a unit takes over `words`.
fn front_door_then_wire(repo: &Path) {
    let door = front_door::front_door(&FrontDoorOptions {
        repo,
        library: false,
        check: true,
    })
    .unwrap();
    assert_eq!(
        door.commands,
        ["copy", "div", "len", "now", "upper", "words"]
    );
    let replay = door.replay.unwrap();
    assert_eq!(replay.passed, replay.total, "{:?}", replay.failures);
    let fallback = replay.fallback.as_ref().unwrap();
    assert_eq!(fallback.cases, replay.total);
    let app_dir = repo.join(front_door::APP_DIR);
    if common::have("cargo") {
        sh(&app_dir, "cargo", &["fmt", "--check"]);
    }

    let unit = repo.join("rust/wordcount-rs");
    std::fs::create_dir_all(unit.join("src")).unwrap();
    std::fs::write(
        unit.join("Cargo.toml"),
        "[package]\nname = \"wordcount-rs\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n[lib]\ncrate-type = [\"cdylib\", \"rlib\"]\n",
    )
    .unwrap();
    std::fs::write(unit.join("src/lib.rs"), "pub mod port;\n").unwrap();
    std::fs::write(unit.join("src/port.rs"), WORDCOUNT_PORT).unwrap();
    let wire = |commands: &[String]| {
        front_door::wire(&WireOptions {
            repo,
            unit: "wordcount-rs",
            commands,
            routes: &[],
            skip_gate: true,
            check: true,
        })
        .unwrap()
    };

    let stub = wire(&["words".to_string()]);
    assert!(stub.glue_pending);
    assert_eq!(stub.switch, "RUST_ANYTHING_UNIT_WORDCOUNT");
    let failing = stub.replay.unwrap();
    assert!(!failing.failures.is_empty());
    assert!(
        failing.failures.iter().all(|f| f.args[0] == "words"),
        "{:?}",
        failing.failures
    );

    std::fs::write(&stub.glue, WORDCOUNT_DECLINING_GLUE).unwrap();
    let wired = wire(&[]);
    assert!(!wired.glue_pending);
    assert_eq!(wired.commands, ["words"]);
    let replay = wired.replay.unwrap();
    assert_eq!(
        replay.passed,
        replay.total,
        "{}",
        std::fs::read_to_string(&replay.report).unwrap()
    );
    if common::have("cargo") {
        sh(&app_dir, "cargo", &["fmt", "--check"]);
        sh(
            &app_dir,
            "cargo",
            &["clippy", "--all-targets", "--", "-D", "warnings"],
        );
    }
    let fallback = replay.fallback.as_ref().unwrap();
    assert!(fallback.cases < fallback.total, "{fallback:?}");
    assert!(fallback.cases >= fallback.total - 5, "{fallback:?}");

    let exe = &replay.build[0];
    let log = repo.join("fallback.log");
    let run = |switch: Option<&str>, input: &[u8]| {
        let _ = std::fs::remove_file(&log);
        let mut cmd = Command::new(exe);
        cmd.arg("words")
            .env(rust_anything::app::FALLBACK_ENV, &log)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped());
        if let Some(value) = switch {
            cmd.env("RUST_ANYTHING_UNIT_WORDCOUNT", value);
        }
        let mut child = cmd.spawn().unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success());
        assert_eq!(
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .collect::<Vec<_>>(),
            ["3"]
        );
        std::fs::read_to_string(&log).unwrap()
    };
    assert_eq!(run(None, b"one two  three\n"), "", "Rust answered");
    assert_eq!(
        run(Some("original"), b"one two  three\n"),
        "words\n",
        "the kill switch fell back"
    );
    assert_eq!(
        run(None, b"#original two three\n"),
        "words\n",
        "the glue declined and passed stdin on"
    );
}

#[test]
fn ts_cli_records_and_replays() {
    if !common::have("node") {
        return;
    }
    let repo = fixture("ts");
    add_bug(&repo, "tcli.js", Some("tcli-bug.js"));
    record_then_replay(&repo, "node tcli.js", "node tcli-bug.js");
}

#[test]
fn python_cli_records_and_replays() {
    if !common::have("python3") {
        return;
    }
    let repo = fixture("python");
    add_bug(&repo, "tcli.py", Some("tcli_bug.py"));
    record_then_replay(&repo, "python3 tcli.py", "python3 tcli_bug.py");
}

#[test]
fn c_cli_records_and_replays() {
    if !common::have("cc") {
        return;
    }
    let repo = fixture("c");
    sh(&repo, "cc", &["-O1", "-o", "tcli", "tcli.c"]);
    add_bug(&repo, "tcli.c", Some("tcli-bug.c"));
    sh(&repo, "cc", &["-O1", "-o", "tcli-bug", "tcli-bug.c"]);
    record_then_replay(&repo, "./tcli", "./tcli-bug");
}

#[test]
fn go_cli_records_and_replays() {
    if !common::have("go") {
        return;
    }
    let repo = fixture("go");
    sh(&repo, "go", &["build", "-o", "tcli", "."]);
    add_bug(&repo, "main.go", None);
    sh(&repo, "go", &["build", "-o", "tcli-bug", "."]);
    record_then_replay(&repo, "./tcli", "./tcli-bug");
}

#[test]
fn ruby_cli_records_and_replays() {
    if !common::have("ruby") {
        return;
    }
    let repo = fixture("ruby");
    add_bug(&repo, "tcli.rb", Some("tcli_bug.rb"));
    record_then_replay(&repo, "ruby tcli.rb", "ruby tcli_bug.rb");
}

#[test]
fn php_cli_records_and_replays() {
    if !common::have("php") {
        return;
    }
    let repo = fixture("php");
    add_bug(&repo, "tcli.php", Some("tcli_bug.php"));
    record_then_replay(&repo, "php tcli.php", "php tcli_bug.php");
}

#[test]
fn java_cli_records_and_replays() {
    if !common::have("javac") || !common::have("java") {
        return;
    }
    let repo = fixture("java");
    sh(&repo, "javac", &["-d", "classes", "Tcli.java"]);
    add_bug(&repo, "Tcli.java", None);
    sh(&repo, "javac", &["-d", "classes-bug", "Tcli.java"]);
    record_then_replay(
        &repo,
        "java -XX:-UsePerfData -cp ./classes Tcli",
        "java -XX:-UsePerfData -cp ./classes-bug Tcli",
    );
}

fn dotnet_build(repo: &Path, out: &str) {
    sh(
        repo,
        "dotnet",
        &["build", "-c", "Release", "--nologo", "-v", "q", "-o", out],
    );
}

#[test]
fn csharp_cli_records_and_replays() {
    if !common::have("dotnet") {
        return;
    }
    let repo = fixture("csharp");
    dotnet_build(&repo, "out");
    add_bug(&repo, "Program.cs", None);
    dotnet_build(&repo, "out-bug");
    record_then_replay(&repo, "dotnet out/tcli.dll", "dotnet out-bug/tcli.dll");
}

#[test]
fn the_command_exits_non_zero_on_a_mismatch() {
    if !common::have("python3") {
        return;
    }
    let repo = fixture("python");
    add_bug(&repo, "tcli.py", Some("tcli_bug.py"));
    std::fs::write(
        repo.join("inputs.jsonl"),
        "{\"name\": \"stamped\", \"args\": [\"upper\", \"at 2026-11-16T10:00:00Z\"]}\n\
         {\"args\": [\"copy\", \"a.txt\", \"b.txt\"], \"files\": {\"a.txt\": \"x\"}}\n\
         {\"args\": [\"div\", \"1\", \"0\"]}\n",
    )
    .unwrap();
    let bin = env!("CARGO_BIN_EXE_rust-anything");
    let record = Command::new(bin)
        .args(["app", "record"])
        .arg(&repo)
        .args([
            "--no-traps",
            "--inputs",
            "inputs.jsonl",
            "--normalize",
            "timestamps",
            "--",
            "python3",
            "tcli.py",
        ])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(
        record.status.success(),
        "{}",
        String::from_utf8_lossy(&record.stderr)
    );
    let dir = app::cases_dir(&repo);
    assert_eq!(
        std::fs::read_to_string(dir.join("stamped.stdout"))
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        ["AT [..]"]
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("input-002-copy.out/b.txt")).unwrap(),
        "X"
    );
    let config = std::fs::read_to_string(dir.join(app::CONFIG_FILE)).unwrap();
    assert!(config.contains("\"timestamps\""), "{config}");

    let replay = |build: Option<&str>| {
        let mut cmd = Command::new(bin);
        cmd.args(["app", "replay"]).arg(&repo);
        if let Some(b) = build {
            cmd.args(["--build", b]);
        }
        cmd.output().unwrap()
    };
    let good = replay(None);
    assert!(good.status.success());
    assert!(String::from_utf8_lossy(&good.stdout).starts_with("10/10 cases passed"));
    let bad = replay(Some("python3 tcli_bug.py"));
    assert_eq!(bad.status.code(), Some(1));
    let text = String::from_utf8_lossy(&bad.stdout);
    assert!(text.starts_with("9/10 cases passed"), "{text}");
    assert!(text.contains("- input-003-div: stderr"), "{text}");
    std::fs::remove_dir_all(&repo).unwrap();
}

fn write_unit(repo: &Path, name: &str, port: &str) {
    let unit = repo.join("rust").join(name);
    std::fs::create_dir_all(unit.join("src")).unwrap();
    std::fs::write(
        unit.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n[lib]\ncrate-type = [\"cdylib\", \"rlib\"]\n"
        ),
    )
    .unwrap();
    std::fs::write(unit.join("src/lib.rs"), "pub mod port;\n").unwrap();
    std::fs::write(unit.join("src/port.rs"), port).unwrap();
}

/// How the original counts and uppercases: a faithful port matches its own language.
struct Semantics {
    count: &'static str,
    upper: &'static str,
    unit: &'static str,
}

const CHARS: Semantics = Semantics {
    count: "Count::Chars",
    upper: "Upper::Full",
    unit: "characters",
};

fn textops_port(s: &Semantics) -> String {
    format!(
        "pub enum Count {{
    Chars,
    Bytes,
}}

pub enum Upper {{
    Full,
    Simple,
    Ascii,
}}

pub const COUNT: Count = {};
pub const UPPER: Upper = {};
pub const UNIT: &str = {:?};

pub fn upper(text: &str) -> String {{
    match UPPER {{
        Upper::Full => text.to_uppercase(),
        Upper::Simple => text
            .chars()
            .map(|c| {{
                let mut up = c.to_uppercase();
                match (up.next(), up.next()) {{
                    (Some(u), None) => u,
                    _ => c,
                }}
            }})
            .collect(),
        Upper::Ascii => text.to_ascii_uppercase(),
    }}
}}

pub fn count(text: &str) -> usize {{
    match COUNT {{
        Count::Chars => text.chars().count(),
        Count::Bytes => text.len(),
    }}
}}

pub fn div(a: i64, b: i64) -> Option<(i64, i64)> {{
    (b != 0).then(|| (a / b, a % b))
}}
",
        s.count, s.upper, s.unit
    )
}

const TEXTOPS_GLUE: &str = "use std::ffi::OsString;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::ArgMatches;
use textops_rs::port;

pub fn run(command: &str, matches: &ArgMatches) -> ExitCode {
    let args: Vec<String> = matches
        .get_many::<OsString>(\"args\")
        .into_iter()
        .flatten()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    match (command, args.as_slice()) {
        (\"upper\", words) => {
            println!(\"{}\", port::upper(&words.join(\" \")));
            ExitCode::SUCCESS
        }
        (\"len\", [text]) => {
            println!(\"{}\", port::count(text));
            ExitCode::SUCCESS
        }
        (\"div\", [a, b]) => match (a.parse::<i64>(), b.parse::<i64>()) {
            (Ok(a), Ok(b)) => match port::div(a, b) {
                Some((q, r)) => {
                    println!(\"{a} / {b} = {q} remainder {r}\");
                    ExitCode::SUCCESS
                }
                None => {
                    eprintln!(\"error: division by zero\");
                    ExitCode::from(2)
                }
            },
            _ => {
                eprintln!(\"error: not a number\");
                ExitCode::from(2)
            }
        },
        (\"copy\", [from, to]) => {
            let Ok(text) = std::fs::read_to_string(from) else {
                eprintln!(\"error: cannot read {from}\");
                return ExitCode::from(1);
            };
            if std::fs::write(to, port::upper(&text)).is_err() {
                eprintln!(\"error: cannot write {to}\");
                return ExitCode::from(1);
            }
            println!(\"copied {} {}\", port::count(&text), port::UNIT);
            ExitCode::SUCCESS
        }
        (\"now\", _) => {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            println!(\"now: {nanos}\");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(\"usage: tcli upper|len|div|words|copy|now\");
            ExitCode::from(1)
        }
    }
}
";

/// Record, front door, wire every command to Rust, lift, then replay the pure binary with the
/// original moved away: nothing may still need it.
fn lift_end_to_end(repo: &Path, run: &str, semantics: &Semantics, original: &str) {
    app::record(&RecordOptions {
        repo,
        run: &words(run),
        inputs: None,
        normalize: &[],
        name: None,
        traps: false,
        timeout_secs: None,
        http: false,
        server_args: &[],
        fresh: &[],
        pace_ms: None,
        port: None,
        float_tolerance: None,
        float_files: &[],
        effects: None,
    })
    .unwrap();
    let door = front_door::front_door(&FrontDoorOptions {
        repo,
        library: false,
        check: true,
    })
    .unwrap();
    let replay = door.replay.unwrap();
    assert_eq!(replay.passed, replay.total);
    assert_eq!(replay.fallback.unwrap().cases, replay.total);

    let blocked = lift::lift(&LiftOptions { repo }).unwrap();
    assert!(!blocked.lifted);
    assert!(
        blocked
            .blockers
            .iter()
            .any(|b| b.contains("no unit is wired")),
        "{:?}",
        blocked.blockers
    );

    write_unit(repo, "wordcount-rs", WORDCOUNT_PORT);
    write_unit(repo, "textops-rs", &textops_port(semantics));
    let wire = |unit: &str, commands: &[&str], check: bool| {
        front_door::wire(&WireOptions {
            repo,
            unit,
            commands: &commands.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
            routes: &[],
            skip_gate: true,
            check,
        })
        .unwrap()
    };
    let words_glue = wire("wordcount", &["words"], false).glue;
    std::fs::write(&words_glue, WORDCOUNT_DECLINING_GLUE).unwrap();
    let partial = wire("textops", &["upper", "len"], true);
    let fallback = partial.replay.unwrap().fallback.unwrap();
    assert!(fallback.cases > 0);
    let blocked = lift::lift(&LiftOptions { repo }).unwrap();
    assert!(!blocked.lifted);
    assert!(
        blocked
            .blockers
            .iter()
            .any(|b| b.contains("the glue in") && b.contains("textops")),
        "{:?}",
        blocked.blockers
    );
    assert!(
        blocked
            .blockers
            .iter()
            .any(|b| b.contains("wordcount") && b.contains("hands cases to the original")),
        "{:?}",
        blocked.blockers
    );
    std::fs::write(&words_glue, WORDCOUNT_GLUE).unwrap();
    std::fs::write(&partial.glue, TEXTOPS_GLUE).unwrap();
    let blocked = lift::lift(&LiftOptions { repo }).unwrap();
    assert!(
        blocked
            .blockers
            .iter()
            .any(|b| b.contains("still fall back") && b.contains("example-003-div")),
        "{:?}",
        blocked.blockers
    );

    let all = wire(
        "textops",
        &["upper", "len", "div", "copy", "now", "tcli"],
        true,
    );
    let replay = all.replay.unwrap();
    assert_eq!(replay.passed, replay.total, "{:?}", replay.failures);
    assert_eq!(replay.fallback.unwrap().cases, 0);
    for dir in ["rust/app", "rust/wordcount-rs", "rust/textops-rs"] {
        sh(&repo.join(dir), "cargo", &["fmt"]);
    }

    let lifted = lift::lift(&LiftOptions { repo }).unwrap();
    assert!(lifted.lifted, "{:?}", lifted.blockers);
    assert!(
        lifted.checklist.iter().all(|i| i.mark != lift::Mark::Fail),
        "{:?}",
        lifted.checklist
    );
    assert!(
        lifted
            .checklist
            .iter()
            .any(|i| i.name == "rust/app: cargo test" && i.mark == lift::Mark::Pass)
    );
    let replay = lifted.replay.as_ref().unwrap();
    assert_eq!(replay.passed, replay.total);

    let app_dir = repo.join(front_door::APP_DIR);
    assert!(!app_dir.join("src/original.rs").exists());
    let main = std::fs::read_to_string(app_dir.join("src/main.rs")).unwrap();
    assert!(!main.contains("original"), "{main}");
    let units = std::fs::read_to_string(app_dir.join("src/units/mod.rs")).unwrap();
    assert!(!units.contains("RUST_ANYTHING_UNIT"), "{units}");
    assert!(app_dir.join("tests/app_cases.rs").is_file());
    for unit in ["wordcount-rs", "textops-rs"] {
        let cargo =
            std::fs::read_to_string(repo.join("rust").join(unit).join("Cargo.toml")).unwrap();
        assert!(!cargo.contains("cdylib"), "{cargo}");
    }
    assert!(front_door::State::load(repo).unwrap().lifted);

    let hidden = repo.join(format!("{original}.hidden"));
    std::fs::rename(repo.join(original), &hidden).unwrap();
    let pure = app::replay(&ReplayOptions {
        case: None,
        repo,
        build: Some(&lifted.binary.as_ref().unwrap().to_string_lossy()),
        timeout_secs: None,
        port: None,
    })
    .unwrap();
    assert_eq!(
        pure.passed,
        pure.total,
        "{}",
        std::fs::read_to_string(&pure.report).unwrap()
    );
    assert!(
        pure.fallback.is_none(),
        "the pure binary has no fallback hook"
    );
    std::fs::rename(&hidden, repo.join(original)).unwrap();

    let again = lift::lift(&LiftOptions { repo });
    assert!(again.is_err());
    let rewire = front_door::wire(&WireOptions {
        repo,
        unit: "wordcount",
        commands: &[],
        routes: &[],
        skip_gate: true,
        check: false,
    });
    assert!(rewire.is_err());
    std::fs::remove_dir_all(repo).unwrap();
}

#[test]
fn ts_cli_lifts_to_pure_rust() {
    if !common::have("node") || !common::have("cargo") {
        return;
    }
    lift_end_to_end(&fixture("ts"), "node tcli.js", &CHARS, "tcli.js");
}

#[test]
fn python_cli_lifts_to_pure_rust() {
    if !common::have("python3") || !common::have("cargo") {
        return;
    }
    lift_end_to_end(&fixture("python"), "python3 tcli.py", &CHARS, "tcli.py");
}

#[test]
fn c_cli_lifts_to_pure_rust() {
    if !common::have("cc") || !common::have("cargo") {
        return;
    }
    let repo = fixture("c");
    sh(&repo, "cc", &["-O1", "-o", "tcli", "tcli.c"]);
    let bytes = Semantics {
        count: "Count::Bytes",
        upper: "Upper::Ascii",
        unit: "bytes",
    };
    lift_end_to_end(&repo, "./tcli", &bytes, "tcli");
}

#[test]
fn go_cli_lifts_to_pure_rust() {
    if !common::have("go") || !common::have("cargo") {
        return;
    }
    let repo = fixture("go");
    sh(&repo, "go", &["build", "-o", "tcli", "."]);
    let go = Semantics {
        count: "Count::Bytes",
        upper: "Upper::Simple",
        unit: "bytes",
    };
    lift_end_to_end(&repo, "./tcli", &go, "tcli");
}

#[test]
fn php_cli_lifts_to_pure_rust() {
    if !common::have("php") || !common::have("cargo") {
        return;
    }
    lift_end_to_end(&fixture("php"), "php tcli.php", &CHARS, "tcli.php");
}

#[test]
fn ruby_cli_lifts_to_pure_rust() {
    if !common::have("ruby") || !common::have("cargo") {
        return;
    }
    lift_end_to_end(&fixture("ruby"), "ruby tcli.rb", &CHARS, "tcli.rb");
}

#[test]
fn java_cli_lifts_to_pure_rust() {
    if !common::have("javac") || !common::have("java") || !common::have("cargo") {
        return;
    }
    let repo = fixture("java");
    sh(&repo, "javac", &["-d", "classes", "Tcli.java"]);
    lift_end_to_end(
        &repo,
        "java -XX:-UsePerfData -cp ./classes Tcli",
        &CHARS,
        "classes",
    );
}

#[test]
fn csharp_cli_lifts_to_pure_rust() {
    if !common::have("dotnet") || !common::have("cargo") {
        return;
    }
    let repo = fixture("csharp");
    dotnet_build(&repo, "out");
    let invariant = Semantics {
        count: "Count::Chars",
        upper: "Upper::Simple",
        unit: "characters",
    };
    lift_end_to_end(&repo, "dotnet out/tcli.dll", &invariant, "out");
}

#[test]
fn lift_strips_bridge_bindings_in_every_language() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let base = std::env::temp_dir().join(format!("rust-anything-strip-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let units = [
        ("mini", "packages/a/src/util.ts", "ts"),
        ("python", "src/textkit/", "python"),
        ("c", "src/text.c", "c"),
        ("go", "strkit/", "go"),
        ("ruby", "lib/textkit/", "ruby"),
        ("java", "src/main/java/com/example/text/", "java"),
    ];
    for (fixture, unit, lang) in units {
        let dir = base.join(lang);
        rust_anything::bridge::bridge(&rust_anything::bridge::Options {
            root: &fixtures.join(fixture),
            unit,
            kind: None,
            out: Some(dir.clone()),
            name: None,
            force: false,
            keep: Vec::new(),
            lang: None,
            batch: Vec::new(),
            as_json: Vec::new(),
            allocator: Default::default(),
        })
        .unwrap();
        if lang == "ts" {
            let port = std::fs::read_to_string(dir.join("src/port.rs")).unwrap();
            std::fs::write(
                dir.join("src/port.rs"),
                format!("{port}\n#[allow(unused_imports)]\nuse crate::utf16;\n"),
            )
            .unwrap();
        }
        let removed = lift::strip_unit(&dir).unwrap();
        assert!(!removed.is_empty(), "{lang}");
        let cargo = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        for binding in [
            "cdylib",
            "staticlib",
            "napi",
            "pyo3",
            "magnus",
            "jni",
            "build =",
        ] {
            assert!(!cargo.contains(binding), "{lang}: {cargo}");
        }
        let lib = std::fs::read_to_string(dir.join("src/lib.rs")).unwrap();
        let expected = if lang == "ts" {
            "pub mod port;\npub mod utf16;\n"
        } else {
            "pub mod port;\n"
        };
        assert_eq!(lib, expected, "{lang}");
        for generated in [
            "build.rs",
            "index.cjs",
            "python",
            "go",
            "ruby",
            "java",
            "include",
            "src/wire.rs",
            "src/js.rs",
            "src/pyconv.rs",
            "src/rbconv.rs",
        ] {
            assert!(!dir.join(generated).exists(), "{lang}: {generated}");
        }
        assert!(dir.join("src/port.rs").is_file());
        if common::have("cargo") {
            let out = Command::new("cargo")
                .args(["check", "--quiet", "--manifest-path"])
                .arg(dir.join("Cargo.toml"))
                .env("CARGO_TARGET_DIR", base.join("target"))
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{lang}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
    std::fs::remove_dir_all(&base).unwrap();
}
