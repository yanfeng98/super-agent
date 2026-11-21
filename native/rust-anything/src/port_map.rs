use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;

use crate::bridge::{MANIFEST, Manifest};
use crate::front_door::APP_DIR;
use crate::harness::CASES;
use crate::recording;

pub const MAP_FILE: &str = "rust/PORT_MAP.md";

#[derive(Debug, Serialize)]
pub struct PortMap {
    pub root: PathBuf,
    pub ports: Vec<Port>,
    pub not_ported: Vec<Unported>,
    pub glue: Vec<Glue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analyze_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Port {
    pub unit: String,
    #[serde(rename = "crate")]
    pub crate_dir: String,
    pub language: String,
    pub kind: String,
    pub in_rust: Vec<String>,
    pub kept_in_original: Vec<String>,
    pub cases: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_diff: Option<(usize, usize)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed_upstream: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Unported {
    pub unit: String,
    pub loc: usize,
    pub readiness: u32,
    pub exports: usize,
}

#[derive(Debug, Serialize)]
pub struct Glue {
    pub file: String,
    pub falls_back: bool,
}

impl PortMap {
    pub fn changed(&self) -> usize {
        self.ports
            .iter()
            .filter(|p| p.changed_upstream.is_some())
            .count()
    }
}

pub fn map(root: &Path, extra: &[PathBuf]) -> Result<PortMap> {
    let root = std::fs::canonicalize(root)?;
    let mut crate_dirs = manifests(&root);
    for dir in extra {
        crate_dirs.extend(manifests(&std::fs::canonicalize(dir)?));
    }
    crate_dirs.sort();
    crate_dirs.dedup();

    let mut ports = Vec::new();
    for dir in crate_dirs {
        let Ok(m) = Manifest::load(&dir) else {
            continue;
        };
        if std::fs::canonicalize(&m.root).ok().as_deref() != Some(root.as_path()) {
            continue;
        }
        ports.push(port(&root, &dir, &m));
    }
    ports.sort_by(|a, b| a.unit.cmp(&b.unit).then(a.crate_dir.cmp(&b.crate_dir)));

    let (not_ported, analyze_error) = match crate::analyze::analyze(&root) {
        Ok(report) => {
            let order = |id: &str| {
                report
                    .start_here
                    .iter()
                    .position(|s| s == id)
                    .unwrap_or(usize::MAX)
            };
            let mut units: Vec<_> = report
                .units
                .iter()
                .filter(|u| !ports.iter().any(|p| covers(&p.unit, &u.id, &u.files)))
                .collect();
            units.sort_by(|a, b| {
                order(&a.id)
                    .cmp(&order(&b.id))
                    .then(b.priority.cmp(&a.priority))
                    .then(a.id.cmp(&b.id))
            });
            let units = units
                .into_iter()
                .map(|u| Unported {
                    unit: u.id.clone(),
                    loc: u.loc,
                    readiness: u.readiness,
                    exports: u.exports,
                })
                .collect();
            (units, None)
        }
        Err(e) => (Vec::new(), Some(format!("{e:#}"))),
    };

    Ok(PortMap {
        glue: glue(&root),
        root,
        ports,
        not_ported,
        analyze_error,
    })
}

fn covers(unit: &str, id: &str, files: &[String]) -> bool {
    unit == id || files.iter().any(|f| f == unit)
}

fn port(root: &Path, dir: &Path, m: &Manifest) -> Port {
    let in_rust = m
        .exports
        .iter()
        .map(|f| f.name.clone())
        .chain(m.classes.iter().map(|c| c.name.clone()))
        .collect();
    let kept_in_original = m.js_only.iter().map(|j| j.name.clone()).collect();
    let cases = std::fs::read_to_string(dir.join(CASES))
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0);
    Port {
        unit: m.unit.clone(),
        crate_dir: dir
            .strip_prefix(root)
            .unwrap_or(dir)
            .to_string_lossy()
            .replace('\\', "/"),
        language: format!("{:?}", m.language).to_lowercase(),
        kind: format!("{:?}", m.kind).to_lowercase(),
        in_rust,
        kept_in_original,
        cases,
        last_diff: recording::last_diff(dir),
        changed_upstream: recording::stale(dir, m),
    }
}
fn manifests(dir: &Path) -> Vec<PathBuf> {
    ignore::WalkBuilder::new(dir)
        .hidden(false)
        .filter_entry(|e| {
            !matches!(
                e.file_name().to_str(),
                Some("node_modules" | "target" | ".git" | "vendor" | ".venv" | "venv")
            )
        })
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.file_name() == MANIFEST && e.file_type().is_some_and(|t| t.is_file()))
        .filter_map(|e| e.path().parent().map(Path::to_path_buf))
        .collect()
}

fn glue(root: &Path) -> Vec<Glue> {
    let units = root.join(APP_DIR).join("src/units");
    let Ok(entries) = std::fs::read_dir(&units) else {
        return Vec::new();
    };
    let mut glue: Vec<Glue> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs") && e.file_name() != "mod.rs")
        .map(|e| Glue {
            file: format!("{APP_DIR}/src/units/{}", e.file_name().to_string_lossy()),
            falls_back: std::fs::read_to_string(e.path()).is_ok_and(|t| t.contains("original::")),
        })
        .collect();
    glue.sort_by(|a, b| a.file.cmp(&b.file));
    glue
}

pub fn markdown(map: &PortMap) -> String {
    let mut out = String::new();
    let _ = render(&mut out, map);
    out
}

fn render(out: &mut String, map: &PortMap) -> std::fmt::Result {
    let name = map.root.file_name().map_or_else(
        || map.root.display().to_string(),
        |n| n.to_string_lossy().into(),
    );
    writeln!(out, "# Port map: {name}\n")?;
    let total = map.ports.len() + map.not_ported.len();
    write!(
        out,
        "{} of {} units have a Rust port",
        map.ports.len(),
        if map.analyze_error.is_some() {
            "the".to_string()
        } else {
            total.to_string()
        }
    )?;
    match map.changed() {
        _ if map.ports.is_empty() => writeln!(out, ".\n")?,
        0 => writeln!(out, "; no original changed since it was recorded.\n")?,
        n => writeln!(
            out,
            "; **{n} changed upstream** since they were recorded: re-record them and run `diff`.\n"
        )?,
    }

    writeln!(out, "## Ports\n")?;
    if map.ports.is_empty() {
        writeln!(out, "None yet: `rust-anything bridge` starts one.\n")?;
    } else {
        writeln!(
            out,
            "| Unit | Crate | In Rust | Kept in the original | Cases | Last diff | Original |"
        )?;
        writeln!(out, "| --- | --- | --- | --- | ---: | ---: | --- |")?;
        for p in &map.ports {
            writeln!(
                out,
                "| `{}` | `{}` | {} | {} | {} | {} | {} |",
                p.unit,
                p.crate_dir,
                list(&p.in_rust),
                list(&p.kept_in_original),
                p.cases,
                p.last_diff
                    .map_or("not run".into(), |(m, t)| format!("{m}/{t}")),
                if p.changed_upstream.is_some() {
                    format!("**changed**: `rust-anything record {}`", p.crate_dir)
                } else {
                    "unchanged".into()
                }
            )?;
        }
        writeln!(out)?;
    }

    if !map.glue.is_empty() {
        writeln!(out, "## Whole app\n")?;
        writeln!(out, "| Glue | Runs |")?;
        writeln!(out, "| --- | --- |")?;
        for g in &map.glue {
            writeln!(
                out,
                "| `{}` | {} |",
                g.file,
                if g.falls_back {
                    "Rust, handing some cases to the original"
                } else {
                    "Rust only"
                }
            )?;
        }
        writeln!(out)?;
    }

    writeln!(out, "## Not ported yet\n")?;
    if let Some(e) = &map.analyze_error {
        writeln!(out, "`analyze` could not list the units: {e}\n")?;
    } else if map.not_ported.is_empty() {
        writeln!(out, "Every unit `analyze` found has a port.\n")?;
    } else {
        writeln!(out, "In the order `analyze` suggests.\n")?;
        writeln!(out, "| Unit | Lines | Exports | Readiness |")?;
        writeln!(out, "| --- | ---: | ---: | ---: |")?;
        for u in &map.not_ported {
            writeln!(
                out,
                "| `{}` | {} | {} | {} |",
                u.unit, u.loc, u.exports, u.readiness
            )?;
        }
        writeln!(out)?;
    }
    writeln!(
        out,
        "Regenerate with `rust-anything map --write`. Commit this file: when the original changes \
         upstream, the next map marks every port whose recording it outdated."
    )
}

fn list(items: &[String]) -> String {
    if items.is_empty() {
        "-".into()
    } else {
        items
            .iter()
            .map(|i| format!("`{i}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
