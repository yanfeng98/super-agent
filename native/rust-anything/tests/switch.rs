//! `enable` and `disable` on real source files: the switched-on block must reach every caller,
//! honor the kill switch, and come out byte for byte. Stand-in modules play the built Rust, so
//! nothing is compiled; the shared build folder is checked with two tiny crates of one name.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use rust_anything::{bridge, cache, enable};

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let path = entry.path();
        if entry.file_name() == "node_modules" || entry.file_name() == "rust" {
            continue;
        }
        if path.is_dir() {
            copy_tree(&path, &to.join(entry.file_name()));
        } else {
            std::fs::copy(&path, to.join(entry.file_name())).unwrap();
        }
    }
}

fn scratch(name: &str, fixture: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rust-anything-switch-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(fixture),
        &dir,
    );
    dir.canonicalize().unwrap()
}

fn bridged(root: &Path, unit: &str, name: &str, keep: &[&str]) -> PathBuf {
    let out = root.join("rust").join(name);
    bridge::bridge(&bridge::Options {
        root,
        unit,
        kind: None,
        out: Some(out.clone()),
        name: Some(name.into()),
        force: false,
        keep: keep.iter().map(|k| k.to_string()).collect(),
        lang: None,
        batch: Vec::new(),
        as_json: Vec::new(),
        allocator: Default::default(),
    })
    .unwrap();
    out
}

fn run(cmd: &mut Command) -> String {
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn python_callers_get_rust_until_disabled() {
    let Some(python) = common::python() else {
        return;
    };
    let root = scratch("python", "go-fuzzy");
    let crate_dir = bridged(&root, "fuzzy.py", "dedupe-rs", &["levenshtein"]);
    let package = crate_dir.join("python/dedupe_rs");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(
        package.join("__init__.py"),
        "def dedupe(names, limit):\n    return [[\"from rust\"]]\n",
    )
    .unwrap();
    let before = std::fs::read(root.join("fuzzy.py")).unwrap();
    let call = |switch: Option<&str>| {
        let mut cmd = Command::new(python);
        cmd.args(["-c", "import fuzzy; print(fuzzy.dedupe(['ab', 'ac'], 1))"])
            .current_dir(&root)
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .env_remove("RUST_ANYTHING_MODE");
        if let Some(v) = switch {
            cmd.env("RUST_ANYTHING_UNIT_DEDUPE", v);
        }
        run(&mut cmd)
    };
    assert_eq!(call(None), "[['ab', 'ac']]");

    let on = enable::enable(&crate_dir).unwrap();
    assert_eq!(on.functions, ["dedupe"]);
    assert_eq!(on.switch, "RUST_ANYTHING_UNIT_DEDUPE");
    assert!(enable::is_enabled(&crate_dir).unwrap());
    let text = std::fs::read_to_string(root.join("fuzzy.py")).unwrap();
    assert!(
        !text.contains(root.to_str().unwrap()),
        "no absolute path:\n{text}"
    );
    assert_eq!(call(None), "[['from rust']]");
    assert_eq!(call(Some("original")), "[['ab', 'ac']]");
    enable::enable(&crate_dir).unwrap();
    assert_eq!(
        std::fs::read_to_string(root.join("fuzzy.py")).unwrap(),
        text,
        "enabling twice adds one block"
    );

    let off = enable::disable(&crate_dir).unwrap();
    assert_eq!(off.files, [root.join("fuzzy.py")]);
    assert_eq!(std::fs::read(root.join("fuzzy.py")).unwrap(), before);
    assert!(!enable::is_enabled(&crate_dir).unwrap());
    assert_eq!(call(None), "[['ab', 'ac']]");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn typescript_callers_get_rust_until_disabled() {
    if !common::have("node") {
        return;
    }
    let root = scratch("ts", "mini");
    let crate_dir = bridged(&root, "packages/a/src/util.ts", "util-rs", &[]);
    std::fs::write(
        crate_dir.join("index.cjs"),
        "exports.count = () => 42\nexports.words = () => ['rust']\nexports.rangeOf = () => null\n",
    )
    .unwrap();
    let file = root.join("packages/a/src/util.ts");
    let before = std::fs::read(&file).unwrap();
    let call = |switch: Option<&str>| {
        let mut cmd = Command::new("node");
        cmd.args([
            "--no-warnings",
            "--input-type=module",
            "-e",
            "const u = await import('./packages/a/src/util.ts'); console.log(u.count('a b c'), u.words('$x'))",
        ])
        .current_dir(&root)
        .env_remove("RUST_ANYTHING_MODE");
        if let Some(v) = switch {
            cmd.env("RUST_ANYTHING_UNIT_UTIL", v);
        }
        run(&mut cmd)
    };
    assert_eq!(call(None), "2 [ 'x' ]");

    let on = enable::enable(&crate_dir).unwrap();
    assert_eq!(on.functions, ["rangeOf", "words", "count"]);
    assert_eq!(call(None), "42 [ 'rust' ]");
    assert_eq!(call(Some("original")), "2 [ 'x' ]");

    enable::disable(&crate_dir).unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), before);
    assert_eq!(call(None), "2 [ 'x' ]");
    std::fs::remove_dir_all(root).unwrap();
}

/// A crate's library file is named after its package alone, so two crates with one name must
/// not share it: the second builds in its own `target/`, and each keeps its own library.
#[test]
fn two_crates_with_one_name_keep_their_own_library() {
    let root = std::env::temp_dir().join(format!("rust-anything-shared-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let shared = root.join("shared");
    let krate = |dir: &str, value: u32| -> PathBuf {
        let dir = root.join(dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"same-rs\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[workspace]\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/lib.rs"),
            format!("#[no_mangle]\npub extern \"C\" fn value() -> u32 {{ {value} }}\n"),
        )
        .unwrap();
        dir
    };
    let build = |dir: &Path| -> Vec<u8> {
        let target = cache::target_in(&shared, dir).unwrap();
        let status = Command::new("cargo")
            .args(["build", "--release", "--quiet", "--manifest-path"])
            .arg(dir.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", &target.dir)
            .status()
            .unwrap();
        assert!(status.success());
        let own = cache::publish(&target, dir, "same_rs", &target.dir.join("release")).unwrap();
        let lib = ["libsame_rs.so", "libsame_rs.dylib", "same_rs.dll"]
            .iter()
            .map(|n| own.join(n))
            .find(|p| p.exists())
            .unwrap();
        std::fs::read(lib).unwrap()
    };
    let (a, b) = (krate("a", 1), krate("b", 2));
    let first = build(&a);
    assert!(cache::target_in(&shared, &a).unwrap().shared);
    assert!(!cache::target_in(&shared, &b).unwrap().shared);
    let second = build(&b);
    assert_ne!(first, second, "each crate links its own code");
    assert_eq!(
        build(&a),
        first,
        "a rebuild after the other crate keeps a's library"
    );
    assert!(b.join("target/release").is_dir());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn go_takes_the_root_as_profile_does() {
    let bin = env!("CARGO_BIN_EXE_rust-anything");
    let parse = |args: &[&str]| {
        let out = Command::new(bin)
            .args(args)
            .current_dir(std::env::temp_dir())
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stderr).into_owned()
    };
    for args in [
        &["go", ".", "--", "true"][..],
        &["go", "--root", ".", "--", "true"][..],
    ] {
        let err = parse(args);
        assert!(!err.contains("unexpected argument"), "{args:?}: {err}");
    }
    let both = parse(&["go", "a", "--root", "b", "--", "true"]);
    assert!(both.contains("cannot be used with"), "{both}");
}
