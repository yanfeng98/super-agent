mod common;

use std::process::Command;

#[test]
fn record_leaves_the_callers_working_directory_untouched() {
    if common::python().is_none() {
        return;
    }
    let root = std::env::temp_dir().join(format!("rust-anything-cwd-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let project = root.join("notes");
    let elsewhere = root.join("elsewhere");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(&elsewhere).unwrap();
    std::fs::write(
        project.join("pyproject.toml"),
        "[project]\nname = \"notes\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        project.join("notes.py"),
        "import os\n\ndef save(path: str, text: str) -> int:\n    os.makedirs(path, exist_ok=True)\n    with open(os.path.join(path, \"note.txt\"), \"w\") as f:\n        return f.write(text)\n",
    )
    .unwrap();

    let cli = env!("CARGO_BIN_EXE_rust-anything");
    let bridged = Command::new(cli)
        .arg("bridge")
        .arg(".")
        .arg("notes.py")
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(bridged.status.success(), "{bridged:?}");
    let recorded = Command::new(cli)
        .arg("record")
        .arg("../notes/rust/notes-rs")
        .current_dir(&elsewhere)
        .output()
        .unwrap();
    assert!(recorded.status.success(), "{recorded:?}");

    let left: Vec<_> = std::fs::read_dir(&elsewhere)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    let cases = std::fs::read_to_string(project.join("rust/notes-rs/cases.jsonl")).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert!(
        left.is_empty(),
        "record wrote into the caller's folder: {left:?}"
    );
    assert!(cases.lines().count() > 10, "{cases}");
}
