//! Runtime checks shared by the integration tests.
//!
//! A test whose runtime is missing is skipped with a note, unless `RUST_ANYTHING_REQUIRE` names
//! that runtime (comma-separated: `node,python3,go,ruby,java`): then the test fails, so CI notices
//! when a language suite stops running. `java` covers `javac` too.

#![allow(dead_code)]

use std::process::Command;

fn required(runtime: &str) -> bool {
    let runtime = if runtime == "javac" { "java" } else { runtime };
    std::env::var("RUST_ANYTHING_REQUIRE").is_ok_and(|list| {
        list.split(',')
            .any(|r| r.trim().eq_ignore_ascii_case(runtime))
    })
}

fn runs(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .is_ok_and(|o| o.status.success())
}

fn version_args(program: &str) -> &'static [&'static str] {
    match program {
        "go" => &["version"],
        "java" | "javac" | "kotlinc" => &["-version"],
        _ => &["--version"],
    }
}

fn missing(runtime: &str, why: &str) -> bool {
    assert!(
        !required(runtime),
        "{runtime} is required (RUST_ANYTHING_REQUIRE) but {why}"
    );
    eprintln!("skipped: {runtime} {why}");
    false
}

/// Whether `program` runs; skips (or, when required, fails) the test when it does not.
pub fn have(program: &str) -> bool {
    runs(program, version_args(program)) || missing(program, "was not found")
}

/// Whether `program args` succeeds, for runtimes that need more than to exist (a minimum version).
pub fn have_with(program: &str, args: &[&str], need: &str) -> bool {
    runs(program, args) || missing(program, need)
}

/// Ruby 3 or newer, which a test that builds a magnus/rb-sys extension needs: macOS's system
/// Ruby is 2.6, and rb-sys does not build on it.
pub fn have_ruby_3() -> bool {
    have_with(
        "ruby",
        &["-e", "exit(RUBY_VERSION.to_i >= 3 ? 0 : 1)"],
        "3 or newer was not found",
    )
}

/// `python3`, else `python` (Windows installs only the latter); required as `python3` or `python`.
pub fn python() -> Option<&'static str> {
    if let Some(found) = ["python3", "python"]
        .into_iter()
        .find(|p| runs(p, &["--version"]))
    {
        return Some(found);
    }
    if required("python") {
        missing("python", "was not found");
    }
    missing("python3", "was not found");
    None
}
