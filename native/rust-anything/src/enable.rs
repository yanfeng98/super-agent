//! `enable` and `disable`: switch a ported unit in and out of the app with one command.
//!
//! `enable` puts one marked block in each original source file of the unit (TS/JS or Python).
//! The block loads the built Rust module and rebinds the ported functions, so every caller gets
//! Rust without an import changing. `disable` removes exactly the lines `enable` added. At run
//! time `RUST_ANYTHING_MODE=old` or the unit's `RUST_ANYTHING_UNIT_<NAME>=original` keeps the
//! original, and a module that fails to load falls back to it (`RUST_ANYTHING_MODE=new` makes
//! that an error instead).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::bridge::{Kind, Manifest};
use crate::lang::Lang;
use crate::walk::Canonical;

const START: &str = ">>> rust-anything:";
const END: &str = "<<< rust-anything:";
const EOF_NOTE: &str = " (newline added)";

#[derive(Debug, Serialize)]
pub struct Switched {
    pub crate_name: String,
    pub on: bool,
    /// Source files that changed.
    pub files: Vec<PathBuf>,
    /// Functions that run in Rust while the unit is on.
    pub functions: Vec<String>,
    /// Exports left to the original, with why.
    pub skipped: Vec<String>,
    /// The variable that sends this unit back to the original for one run.
    pub switch: String,
}

/// Whether `enable` left its block in any of the unit's files.
pub fn is_enabled(crate_dir: &Path) -> Result<bool> {
    let manifest = Manifest::load(crate_dir)?;
    Ok(files(&manifest).iter().any(|f| {
        std::fs::read_to_string(manifest.root.join(f))
            .is_ok_and(|text| find_block(&text, &manifest.crate_name).is_some())
    }))
}

/// Sets the unit's kill switch on a command that runs the original, so a recording, diff or
/// bench of an enabled unit measures the original, not the port.
pub fn original_only(cmd: &mut Command, manifest: &Manifest) {
    cmd.env(
        crate::front_door::kill_switch(&manifest.crate_name),
        "original",
    );
}

pub fn enable(crate_dir: &Path) -> Result<Switched> {
    let crate_dir = crate_dir
        .canonical()
        .with_context(|| format!("resolving {}", crate_dir.display()))?;
    let manifest = Manifest::load(&crate_dir)?;
    supported(&manifest)?;
    let switch = crate::front_door::kill_switch(&manifest.crate_name);
    let mut by_file: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for f in &manifest.exports {
        by_file.entry(&f.file).or_default().push(&f.name);
    }
    let mut out = Switched {
        crate_name: manifest.crate_name.clone(),
        on: true,
        files: Vec::new(),
        functions: Vec::new(),
        skipped: manifest
            .js_only
            .iter()
            .map(|j| format!("{}: kept in the original", j.name))
            .collect(),
        switch: switch.clone(),
    };
    let mut edits = Vec::new();
    for (file, names) in by_file {
        let path = manifest.root.join(file);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        if find_block(&text, &manifest.crate_name).is_some() {
            out.functions.extend(names.iter().map(|n| n.to_string()));
            continue;
        }
        let (swap, skip): (Vec<&str>, Vec<&str>) = names
            .into_iter()
            .partition(|n| declared(&text, n, manifest.language));
        out.skipped.extend(skip.iter().map(|n| {
            format!("{n}: not a plain function declaration in {file}, so callers keep the original")
        }));
        if swap.is_empty() {
            continue;
        }
        let dir = path
            .parent()
            .context("source file has no folder")?
            .canonical()?;
        let block = match manifest.language {
            Lang::Python => python_block(&manifest, &crate_dir, &dir, &swap, &switch),
            _ => js_block(
                &manifest,
                &crate_dir,
                &dir,
                &swap,
                &switch,
                esm(file, &text),
            ),
        };
        let edited = insert(&text, &block, manifest.language);
        edits.push((path.clone(), edited));
        out.files.push(path);
        out.functions.extend(swap.iter().map(|n| n.to_string()));
    }
    if out.functions.is_empty() {
        bail!(
            "nothing to switch on in {}: {}",
            manifest.crate_name,
            out.skipped.join("; ")
        );
    }
    for (path, text) in edits {
        std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(out)
}

pub fn disable(crate_dir: &Path) -> Result<Switched> {
    let crate_dir = crate_dir
        .canonical()
        .with_context(|| format!("resolving {}", crate_dir.display()))?;
    let manifest = Manifest::load(&crate_dir)?;
    let mut out = Switched {
        crate_name: manifest.crate_name.clone(),
        on: false,
        files: Vec::new(),
        functions: Vec::new(),
        skipped: Vec::new(),
        switch: crate::front_door::kill_switch(&manifest.crate_name),
    };
    for file in files(&manifest) {
        let path = manifest.root.join(file);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Some(restored) = remove(&text, &manifest.crate_name) {
            std::fs::write(&path, restored)
                .with_context(|| format!("writing {}", path.display()))?;
            out.files.push(path);
        }
    }
    out.functions = manifest.exports.iter().map(|f| f.name.clone()).collect();
    Ok(out)
}

pub fn is_supported(manifest: &Manifest) -> bool {
    matches!(
        (manifest.language, manifest.kind),
        (Lang::Python, Kind::Pyo3) | (Lang::Ts, Kind::Napi | Kind::Wasm)
    )
}

fn supported(manifest: &Manifest) -> Result<()> {
    match manifest.language {
        _ if is_supported(manifest) => Ok(()),
        lang => bail!(
            "`enable` switches TS/JS and Python ports in place; for {} use the shadow module \
             (`rust-anything shadow`), imported where the app imported the original, and \
             RUST_ANYTHING_MODE=old|shadow|new",
            lang.name()
        ),
    }
}

fn files(manifest: &Manifest) -> Vec<&str> {
    let mut files: Vec<&str> = manifest.exports.iter().map(|f| f.file.as_str()).collect();
    files.sort_unstable();
    files.dedup();
    files
}

/// A function the block can rebind: a top-level `def` (Python) or a `function` declaration.
fn declared(text: &str, name: &str, lang: Lang) -> bool {
    let name = regex::escape(name);
    let pattern = if lang == Lang::Python {
        format!(r"(?m)^(async\s+)?def\s+{name}\s*\(")
    } else {
        format!(r"(?m)^\s*(export\s+)?(default\s+)?(async\s+)?function\s*\*?\s*{name}\s*[<(]")
    };
    regex::Regex::new(&pattern).is_ok_and(|re| re.is_match(text))
}

fn esm(file: &str, text: &str) -> bool {
    if file.ends_with(".cjs") || file.ends_with(".cts") {
        return false;
    }
    file.ends_with(".mjs")
        || file.ends_with(".mts")
        || text
            .lines()
            .any(|l| l.starts_with("import ") || l.starts_with("export "))
}

fn marker(comment: &str, crate_name: &str, crate_dir: &Path, dir: &Path) -> (String, String) {
    let at = crate::shadow::relative(dir, crate_dir);
    (
        format!(
            "{comment} {START} {crate_name} is on. `rust-anything disable {at}` removes this block."
        ),
        format!("{comment} {END} {crate_name}"),
    )
}

fn js_block(
    manifest: &Manifest,
    crate_dir: &Path,
    dir: &Path,
    names: &[&str],
    switch: &str,
    esm: bool,
) -> String {
    let (start, end) = marker("//", &manifest.crate_name, crate_dir, dir);
    let module = if manifest.kind == Kind::Wasm {
        let lib = manifest.lib_name();
        crate_dir.join("pkg").join(format!("{lib}.js"))
    } else {
        crate_dir.join("index.cjs")
    };
    let rel = crate::shadow::relative(dir, &module);
    let here = if esm {
        "import.meta.dirname"
    } else {
        "__dirname"
    };
    let var = format!(
        "__rustAnything_{}",
        crate::front_door::ident(&manifest.crate_name)
    );
    let assign: Vec<String> = names.iter().map(|n| format!("{n} = {var}.{n}")).collect();
    format!(
        "{start}\n\
         // @ts-ignore\n\
         const {var} = ((p) => p && p.env.RUST_ANYTHING_MODE !== 'old' && p.env.{switch} !== 'original' && (() => {{ try {{ const m = p.getBuiltinModule('node:module'); const at = p.getBuiltinModule('node:path').join({here}, {rel:?}); return m.createRequire(at)(at) }} catch (e) {{ if (p.env.RUST_ANYTHING_MODE === 'new') throw e }} }})())(globalThis.process)\n\
         // @ts-ignore\n\
         if ({var}) {{ {} }}\n\
         {end}\n",
        assign.join("; ")
    )
}

fn python_block(
    manifest: &Manifest,
    crate_dir: &Path,
    dir: &Path,
    names: &[&str],
    switch: &str,
) -> String {
    let (start, end) = marker("#", &manifest.crate_name, crate_dir, dir);
    let (package_dir, lib) = crate::harness_py::package(crate_dir, manifest);
    let rel = crate::shadow::relative(dir, &package_dir);
    let fun = format!(
        "_rust_anything_{}",
        crate::front_door::ident(&manifest.crate_name)
    );
    let names: Vec<String> = names.iter().map(|n| format!("{n:?}")).collect();
    format!(
        "{start}\n\
         def {fun}():\n\
         \x20   import importlib, os, sys\n\
         \x20   mode = os.environ.get(\"RUST_ANYTHING_MODE\")\n\
         \x20   if mode == \"old\" or os.environ.get(\"{switch}\") == \"original\":\n\
         \x20       return\n\
         \x20   at = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), {rel:?}))\n\
         \x20   sys.path.insert(0, at)\n\
         \x20   try:\n\
         \x20       native = importlib.import_module({lib:?})\n\
         \x20   except ImportError:\n\
         \x20       if mode == \"new\":\n\
         \x20           raise\n\
         \x20       return\n\
         \x20   finally:\n\
         \x20       sys.path.remove(at)\n\
         \x20   globals().update({{name: getattr(native, name) for name in [{}]}})\n\
         \n\
         \n\
         {fun}()\n\
         del {fun}\n\
         {end}\n",
        names.join(", ")
    )
}

/// Puts the block on its own lines: Python before a top-level `if __name__ == "__main__":`
/// (else at the end, after every `def`), JS after a shebang and `"use strict"`, where hoisted
/// function declarations can already be rebound.
fn insert(text: &str, block: &str, lang: Lang) -> String {
    let at = if lang == Lang::Python {
        line_starts(text)
            .find(|&i| {
                let line = &text[i..];
                line.starts_with("if __name__ == \"__main__\"")
                    || line.starts_with("if __name__ == '__main__'")
            })
            .unwrap_or(text.len())
    } else {
        line_starts(text)
            .find(|&i| {
                let line = text[i..].lines().next().unwrap_or_default().trim();
                !(line.starts_with("#!")
                    || line == "'use strict'"
                    || line == "\"use strict\""
                    || line == "'use strict';"
                    || line == "\"use strict\";")
            })
            .unwrap_or(text.len())
    };
    let mut out = String::with_capacity(text.len() + block.len() + 1);
    out.push_str(&text[..at]);
    if at == text.len() && !text.is_empty() && !text.ends_with('\n') {
        out.push('\n');
        let (first, rest) = block.split_once('\n').unwrap_or((block, ""));
        out.push_str(first);
        out.push_str(EOF_NOTE);
        out.push('\n');
        out.push_str(rest);
    } else {
        out.push_str(block);
    }
    out.push_str(&text[at..]);
    out
}

fn line_starts(text: &str) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .filter(move |&i| i < text.len())
}

/// The byte range of the unit's block, from its first line to the end of its last.
fn find_block(text: &str, crate_name: &str) -> Option<(usize, usize, bool)> {
    let start_tag = format!("{START} {crate_name} is on.");
    let end_tag = format!("{END} {crate_name}");
    let mut start = None;
    for i in std::iter::once(0).chain(text.match_indices('\n').map(|(i, _)| i + 1)) {
        let line = text[i..].split('\n').next().unwrap_or_default();
        match start {
            None if line.contains(&start_tag) => start = Some((i, line.ends_with(EOF_NOTE))),
            Some((s, added)) if line.trim_end().ends_with(&end_tag) => {
                let end = (i + line.len() + 1).min(text.len());
                return Some((s, end, added));
            }
            _ => {}
        }
    }
    None
}

fn remove(text: &str, crate_name: &str) -> Option<String> {
    let (start, end, added) = find_block(text, crate_name)?;
    let start = if added && end == text.len() && start > 0 {
        start - 1
    } else {
        start
    };
    Some(format!("{}{}", &text[..start], &text[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: &str = "# >>> rust-anything: fuzzy-rs is on. `rust-anything disable x` removes this block.\nx = 1\n# <<< rust-anything: fuzzy-rs\n";

    #[test]
    fn python_goes_before_the_main_guard_and_comes_out_byte_for_byte() {
        let text = "def f(a):\n    return a\n\nif __name__ == \"__main__\":\n    print(f(1))\n";
        let on = insert(text, BLOCK, Lang::Python);
        assert!(
            on.find("x = 1").unwrap() < on.find("if __name__").unwrap(),
            "{on}"
        );
        assert_eq!(remove(&on, "fuzzy-rs").unwrap(), text);
        assert!(remove(text, "fuzzy-rs").is_none());
    }

    #[test]
    fn a_file_without_a_final_newline_gets_it_back_the_same() {
        let text = "def f(a):\n    return a";
        let on = insert(text, BLOCK, Lang::Python);
        assert!(on.contains(EOF_NOTE), "{on}");
        assert_eq!(remove(&on, "fuzzy-rs").unwrap(), text);
    }

    #[test]
    fn js_goes_after_the_shebang_and_use_strict() {
        let text = "#!/usr/bin/env node\n'use strict'\nfunction f(a) { return a }\nmodule.exports = { f }\n";
        let block = BLOCK.replace('#', "//");
        let on = insert(text, &block, Lang::Ts);
        assert!(
            on.starts_with("#!/usr/bin/env node\n'use strict'\n// >>>"),
            "{on}"
        );
        assert_eq!(remove(&on, "fuzzy-rs").unwrap(), text);
    }

    #[test]
    fn only_declarations_can_be_rebound() {
        let js = "export function score(a: string): number { return 1 }\nexport const other = () => 2\nexport async function* gen<T>(x: T) {}\n";
        assert!(declared(js, "score", Lang::Ts));
        assert!(!declared(js, "other", Lang::Ts));
        assert!(declared(js, "gen", Lang::Ts));
        let py = "def dedupe(names):\n    pass\n\nclass A:\n    def inner(self):\n        pass\n";
        assert!(declared(py, "dedupe", Lang::Python));
        assert!(!declared(py, "inner", Lang::Python));
    }

    #[test]
    fn module_kind_follows_the_syntax() {
        assert!(esm("a.ts", "import x from 'y'\n"));
        assert!(esm("a.mjs", ""));
        assert!(!esm(
            "a.js",
            "const x = require('y')\nmodule.exports = {}\n"
        ));
        assert!(!esm("a.cjs", "export function f() {}\n"));
    }
}
