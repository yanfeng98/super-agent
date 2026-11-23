//! `app record --fresh` for a CLI: every case starts from a snapshot of a project folder, written
//! files are what differs from it, removed files are compared, and a case can remove a file of
//! the snapshot before it runs. A front door built in a `rust/` that is a symlink still runs the
//! original from the repository.

use std::path::{Path, PathBuf};
use std::process::Command;

use rust_anything::app::{self, RecordOptions, ReplayOptions};
use rust_anything::front_door::{self, FrontDoorOptions};

const TOOL: &str = r#"#!/bin/sh
cat config.txt || exit 3
rm -rf stale
echo built > out.txt
cp keep.txt keep.tmp && mv keep.tmp keep.txt
"#;

fn repo() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let repo = std::env::temp_dir().join(format!(
        "rust-anything-app-fresh-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&repo);
    let project = repo.join("fixtures/project");
    std::fs::create_dir_all(project.join("stale")).unwrap();
    std::fs::write(project.join("config.txt"), "name = demo\n").unwrap();
    std::fs::write(project.join("keep.txt"), "same\n").unwrap();
    std::fs::write(project.join("stale/old.txt"), "old\n").unwrap();
    std::fs::write(repo.join("tool.sh"), TOOL).unwrap();
    std::fs::write(repo.join("buggy.sh"), TOOL.replace("rm -rf stale\n", "")).unwrap();
    std::fs::write(
        repo.join("inputs.jsonl"),
        concat!(
            r#"{"name": "build", "args": []}"#,
            "\n",
            r#"{"name": "no-config", "args": ["x"], "remove": ["config.txt"], "files": {"extra.txt": "x\n"}}"#,
            "\n",
        ),
    )
    .unwrap();
    repo
}

fn record(repo: &Path) -> app::RecordOutcome {
    let inputs = repo.join("inputs.jsonl");
    app::record(&RecordOptions {
        repo,
        run: &["sh".to_string(), "tool.sh".to_string()],
        inputs: Some(&inputs),
        normalize: &[],
        name: Some("tool"),
        traps: false,
        timeout_secs: None,
        http: false,
        server_args: &[],
        fresh: &["fixtures/project/".to_string()],
        pace_ms: None,
        port: None,
        float_tolerance: None,
        float_files: &[],
        effects: None,
    })
    .unwrap()
}

fn replay(repo: &Path, build: &str) -> app::ReplayOutcome {
    app::replay(&ReplayOptions {
        repo,
        build: Some(build),
        case: None,
        timeout_secs: None,
        port: None,
    })
    .unwrap()
}

#[test]
fn cli_cases_start_from_the_fresh_snapshot() {
    let repo = repo();
    let outcome = record(&repo);
    assert_eq!(outcome.cases, 2, "{outcome:?}");
    assert!(outcome.unstable.is_empty(), "{outcome:?}");

    let dir = app::cases_dir(&repo);
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    assert_eq!(read("files/config.txt"), "name = demo\n");
    assert_eq!(read("build.stdout"), "name = demo\n");
    assert_eq!(read("build.out/out.txt"), "built\n");
    assert!(
        !dir.join("build.out/keep.txt").exists(),
        "a file rewritten with the same bytes is not a written file"
    );
    assert!(!dir.join("build.out/config.txt").exists());
    assert_eq!(read("build.removed"), "stale/old.txt\n");
    let config = read("rust-anything.json");
    assert!(config.contains("\"fixtures/project/\""), "{config}");
    assert!(!config.contains("\"http\""), "{config}");

    let no_config = read("no-config.toml");
    assert!(
        no_config.contains("remove = [\"config.txt\"]"),
        "{no_config}"
    );
    assert!(no_config.contains("status.code = 3"), "{no_config}");
    assert_eq!(read("no-config.in/extra.txt"), "x\n");

    let good = replay(&repo, "sh tool.sh");
    assert_eq!((good.passed, good.total), (2, 2), "{good:?}");

    let bad = replay(&repo, "sh buggy.sh");
    assert_eq!((bad.passed, bad.total), (1, 2), "{bad:?}");
    let report = std::fs::read_to_string(&bad.report).unwrap();
    assert!(report.contains("removed files"), "{report}");
    assert!(report.contains("stale/old.txt"), "{report}");

    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn replay_needs_the_snapshot() {
    let repo = repo();
    record(&repo);
    std::fs::remove_dir_all(app::cases_dir(&repo).join("files")).unwrap();
    let err = app::replay(&ReplayOptions {
        repo: &repo,
        build: Some("sh tool.sh"),
        case: None,
        timeout_secs: None,
        port: None,
    })
    .unwrap_err();
    assert!(err.to_string().contains("record again"), "{err}");
    let _ = std::fs::remove_dir_all(&repo);
}

#[cfg(unix)]
#[test]
fn a_front_door_in_a_symlinked_rust_dir_finds_the_original() {
    if Command::new("cargo").arg("--version").output().is_err() {
        return;
    }
    let repo = repo();
    let elsewhere = repo.with_extension("elsewhere");
    let _ = std::fs::remove_dir_all(&elsewhere);
    std::fs::create_dir_all(&elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, repo.join("rust")).unwrap();
    record(&repo);
    let door = front_door::front_door(&FrontDoorOptions {
        repo: &repo,
        library: false,
        check: true,
    })
    .unwrap();
    let replay = door.replay.unwrap();
    assert_eq!(replay.passed, replay.total, "{:?}", replay.failures);
    assert_eq!(replay.fallback.unwrap().cases, replay.total);
    let _ = std::fs::remove_dir_all(&repo);
    let _ = std::fs::remove_dir_all(&elsewhere);
}
