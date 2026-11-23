//! `analyze --whole-app`: what a whole-app port needs on top of the unit plan. The app's entry
//! points and shape, every external dependency (imported or declared) with its Rust crate, the
//! port order as pieces (units that import each other move together), and the decisions to put
//! to the user in the grilling round.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use toml_edit::DocumentMut;

use crate::analyze::{Report, Status, crate_for};
use crate::lang::Lang;
use crate::ts::deps::Fit;

#[derive(Debug, Serialize)]
pub struct WholeApp {
    /// `cli`, `server` or `library`: what the front door has to be.
    pub shape: &'static str,
    pub entry_points: Vec<EntryPoint>,
    /// Every external dependency, imported by a unit or only declared in a manifest.
    pub dependencies: Vec<Dependency>,
    /// The port order: one piece per unit, or one for units that import each other.
    pub pieces: Vec<Piece>,
    /// Questions for the user, numbered, each with a recommended answer.
    pub decisions: Vec<Decision>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntryPoint {
    /// `cli`, `server` or `library`.
    pub kind: &'static str,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// What gave it away: a manifest field, or the code at `path:line`.
    pub evidence: String,
}

#[derive(Debug, Serialize)]
pub struct Dependency {
    /// The manifest's spelling when the library is declared, else the one the code imports.
    pub name: String,
    /// Every spelling of the library, sorted: the names the code imports and the manifest
    /// declares that the crate table resolves to the same row (`yaml`, `pyyaml`).
    pub names: Vec<String>,
    /// Listed in a manifest (package.json, pyproject, go.mod, Gemfile, pom.xml, ...), under any
    /// of `names`.
    pub declared: bool,
    /// Units that import it.
    pub used_by: usize,
    pub crates: Option<&'static str>,
    /// `good`, `partial`, `none`, or `unknown` when no table knows it.
    pub fit: &'static str,
    pub note: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Piece {
    pub wave: usize,
    pub units: Vec<String>,
    pub loc: usize,
    /// The units import each other: port them together.
    pub tangled: bool,
}

#[derive(Debug, Serialize)]
pub struct Decision {
    pub id: String,
    pub topic: String,
    pub question: String,
    pub recommendation: String,
}

const SERVER: &str = r#"WebApplication\.Create(?:Slim)?Builder|\bHttpListener\(|\.UseKestrel\(|\$_SERVER\[\s*['"]REQUEST_(?:METHOD|URI)|\$_(?:GET|POST)\[|\.listen\(|createServer\(|ListenAndServe|HTTPServer\(|HttpServer\.create|uvicorn\.run|\bapp\.run\(|SpringApplication\.run|TCPServer\.new|WEBrick::HTTPServer|Sinatra::Base|FastAPI\(|Flask\(|\bexpress\(\)|\blisten\(\s*\w+\s*,\s*\w+\s*\)|\bserve\(\s*listener"#;

fn main_pattern(lang: Lang) -> Option<&'static str> {
    Some(match lang {
        Lang::Ts => r"^#!.*\bnode\b",
        Lang::Python => r#"if\s+__name__\s*==\s*['"]__main__['"]"#,
        Lang::Go => r"^func\s+main\s*\(\s*\)",
        Lang::C => r"\bint\s+main\s*\(",
        Lang::Ruby => r"^#!.*\bruby\b|\bARGV\b|__FILE__\s*==\s*\$0",
        Lang::Java => r"public\s+static\s+void\s+main\s*\(",
        Lang::Kotlin => r"\bfun\s+main\s*\(",
        Lang::Php => r#"^#!.*\bphp\b|\$argv\b|PHP_SAPI\s*===?\s*['"]cli['"]"#,
        Lang::Csharp => r"\bstatic\s+(?:async\s+)?(?:void|int|Task(?:<int>)?)\s+Main\s*\(",
    })
}

fn fit_label(fit: Option<Fit>) -> &'static str {
    match fit {
        Some(Fit::Good) => "good",
        Some(Fit::Partial) => "partial",
        Some(Fit::None) => "none",
        None => "unknown",
    }
}

fn read(root: &Path, file: &str) -> String {
    std::fs::read_to_string(root.join(file)).unwrap_or_default()
}

/// Entry points named by the manifests.
fn manifest_entries(root: &Path, lang: Lang) -> Vec<EntryPoint> {
    let mut out = Vec::new();
    if let Ok(pkg) = serde_json::from_str::<Value>(&read(root, "package.json")) {
        let name = pkg["name"].as_str().unwrap_or("app");
        match &pkg["bin"] {
            Value::String(path) => out.push(EntryPoint {
                kind: "cli",
                path: path.trim_start_matches("./").to_string(),
                name: Some(name.rsplit('/').next().unwrap_or(name).to_string()),
                evidence: "package.json `bin`".into(),
            }),
            Value::Object(bins) => {
                for (bin, path) in bins {
                    out.push(EntryPoint {
                        kind: "cli",
                        path: path
                            .as_str()
                            .unwrap_or_default()
                            .trim_start_matches("./")
                            .to_string(),
                        name: Some(bin.clone()),
                        evidence: "package.json `bin`".into(),
                    });
                }
            }
            _ => {}
        }
    }
    if let Ok(doc) = read(root, "pyproject.toml").parse::<DocumentMut>() {
        let tables = [
            doc.get("project").and_then(|p| p.get("scripts")),
            doc.get("tool")
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.get("scripts")),
        ];
        for table in tables.into_iter().flatten() {
            if let Some(t) = table.as_table_like() {
                for (name, target) in t.iter() {
                    out.push(EntryPoint {
                        kind: "cli",
                        path: target.as_str().unwrap_or_default().to_string(),
                        name: Some(name.to_string()),
                        evidence: "pyproject.toml scripts".into(),
                    });
                }
            }
        }
    }
    if lang == Lang::Go
        && let Ok(entries) = std::fs::read_dir(root.join("cmd"))
    {
        let mut dirs: Vec<String> = entries
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        dirs.sort();
        for dir in dirs {
            out.push(EntryPoint {
                kind: "cli",
                path: format!("cmd/{dir}"),
                name: Some(dir),
                evidence: "a `cmd/` folder".into(),
            });
        }
    }
    if lang == Lang::Php {
        if let Ok(doc) = serde_json::from_str::<Value>(&read(root, "composer.json")) {
            for bin in doc["bin"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                out.push(EntryPoint {
                    kind: "cli",
                    path: bin.trim_start_matches("./").to_string(),
                    name: Some(bin.rsplit('/').next().unwrap_or(bin).to_string()),
                    evidence: "composer.json `bin`".into(),
                });
            }
        }
        for front in ["public/index.php", "web/index.php", "index.php"] {
            if root.join(front).is_file() {
                out.push(EntryPoint {
                    kind: "server",
                    path: front.to_string(),
                    name: None,
                    evidence: format!("`{front}`, a front controller: serve it with `php -S`"),
                });
                break;
            }
        }
    }
    if lang == Lang::Csharp {
        for project in csproj_files(root) {
            let text = read(root, &project);
            let dir = project
                .rsplit_once('/')
                .map_or(".".to_string(), |(d, _)| d.to_string());
            let name = project
                .rsplit('/')
                .next()
                .unwrap_or(&project)
                .trim_end_matches(".csproj")
                .to_string();
            if text.contains("Microsoft.NET.Sdk.Web") {
                out.push(EntryPoint {
                    kind: "server",
                    path: dir,
                    name: Some(name),
                    evidence: format!("`{project}` uses the web SDK"),
                });
            } else if text.contains("<OutputType>Exe</OutputType>") {
                out.push(EntryPoint {
                    kind: "cli",
                    path: dir,
                    name: Some(name),
                    evidence: format!("`{project}` builds an executable"),
                });
            }
        }
    }
    if lang == Lang::Ruby {
        let gemspec = std::fs::read_dir(root)
            .into_iter()
            .flatten()
            .flatten()
            .find(|e| e.path().extension().is_some_and(|x| x == "gemspec"))
            .map(|e| std::fs::read_to_string(e.path()).unwrap_or_default())
            .unwrap_or_default();
        let list = Regex::new(r#"executables\s*=\s*(?:\[([^\]]*)\]|%w[\[(]([^\])]*)[\])])"#)
            .expect("valid regex");
        if let Some(c) = list.captures(&gemspec) {
            let names = c.get(1).or(c.get(2)).map_or("", |m| m.as_str());
            for name in names
                .split([',', ' ', '\n'])
                .map(|n| n.trim().trim_matches(['"', '\'']))
                .filter(|n| !n.is_empty())
            {
                out.push(EntryPoint {
                    kind: "cli",
                    path: format!("exe/{name}"),
                    name: Some(name.to_string()),
                    evidence: "gemspec `executables`".into(),
                });
            }
        }
    }
    out
}

/// Entry points found in the code: a `main`, or a server starting.
fn code_entries(root: &Path, report: &Report) -> Vec<EntryPoint> {
    let server = Regex::new(SERVER).expect("valid regex");
    let main = main_pattern(report.language).map(|p| {
        regex::RegexBuilder::new(p)
            .multi_line(true)
            .build()
            .expect("valid regex")
    });
    let mut files: Vec<&String> = report.units.iter().flat_map(|u| &u.files).collect();
    files.sort();
    files.dedup();
    let mut out = Vec::new();
    for file in files {
        let text = read(root, file);
        let line_of = |at: usize| text[..at].matches('\n').count() + 1;
        if report.language == Lang::Go && !text.contains("package main") {
            continue;
        }
        if let Some(m) = server.find(&text) {
            out.push(EntryPoint {
                kind: "server",
                path: file.clone(),
                name: None,
                evidence: format!("`{}` at {file}:{}", m.as_str().trim(), line_of(m.start())),
            });
        } else if let Some(m) = main.as_ref().and_then(|re| re.find(&text)) {
            out.push(EntryPoint {
                kind: "cli",
                path: file.clone(),
                name: None,
                evidence: format!("`{}` at {file}:{}", m.as_str().trim(), line_of(m.start())),
            });
        }
    }
    out
}

/// Project files under `root`, relative to it, outside build output.
fn csproj_files(root: &Path) -> Vec<String> {
    crate::walk::filtered(root, |e| {
        !matches!(
            e.file_name().to_string_lossy().as_ref(),
            "bin" | "obj" | "node_modules"
        )
    })
    .build()
    .flatten()
    .filter(|e| e.path().extension().is_some_and(|x| x == "csproj"))
    .filter_map(|e| {
        e.path()
            .strip_prefix(root)
            .ok()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
    })
    .collect()
}

/// Dependencies the manifests declare (runtime ones, not dev or test), as each language's
/// `discover` reads them. A package's dependency on another package of the same workspace is
/// not an external one.
pub fn declared(root: &Path, lang: Lang) -> BTreeSet<String> {
    let ws = match lang {
        Lang::Ts => crate::ts::discover::discover(root),
        _ => crate::lang::discover(lang, root),
    };
    let Ok(ws) = ws else {
        return BTreeSet::new();
    };
    let packages = &ws.packages;
    packages
        .iter()
        .flat_map(|p| {
            p.deps.iter().filter(move |d| {
                !d.is_empty() && !packages.iter().any(|q| q.dir != p.dir && &q.name == *d)
            })
        })
        .cloned()
        .collect()
}

pub fn whole_app(root: &Path, report: &Report) -> WholeApp {
    let lang = report.language;
    let mut entry_points = manifest_entries(root, lang);
    for found in code_entries(root, report) {
        let covered = entry_points.iter().any(|e| {
            e.kind == found.kind
                && (e.path == found.path || found.path.starts_with(&format!("{}/", e.path)))
        });
        if !covered {
            entry_points.push(found);
        }
    }
    let shape = if entry_points.iter().any(|e| e.kind == "server") {
        "server"
    } else if entry_points.iter().any(|e| e.kind == "cli") {
        "cli"
    } else {
        "library"
    };
    if shape == "library" {
        for package in &report.packages {
            entry_points.push(EntryPoint {
                kind: "library",
                path: if package.dir.is_empty() {
                    ".".to_string()
                } else {
                    package.dir.clone()
                },
                name: Some(package.name.clone()),
                evidence: "no main and no server: the package's public API".into(),
            });
        }
    }

    let declared = declared(root, lang);
    let imported: BTreeSet<&String> = report.units.iter().flat_map(|u| &u.external_deps).collect();
    let key = |name: &str| match crate_for(lang, name) {
        Some(m) => (Some(m.row), String::new()),
        None => (None, name.to_string()),
    };
    let mut joined: BTreeMap<(Option<usize>, String), Dependency> = BTreeMap::new();
    for name in imported.into_iter().chain(&declared) {
        let found = crate_for(lang, name);
        let dep = joined.entry(key(name)).or_insert_with(|| Dependency {
            name: String::new(),
            names: Vec::new(),
            declared: false,
            used_by: 0,
            crates: found.map(|m| m.crates),
            fit: fit_label(found.map(|m| m.fit)),
            note: found.map_or("", |m| m.note),
        });
        if !dep.names.contains(name) {
            dep.names.push(name.clone());
        }
        dep.declared |= declared.contains(name);
    }
    for unit in &report.units {
        let keys: BTreeSet<_> = unit.external_deps.iter().map(|n| key(n)).collect();
        for k in keys {
            if let Some(dep) = joined.get_mut(&k) {
                dep.used_by += 1;
            }
        }
    }
    let mut dependencies: Vec<Dependency> = joined
        .into_values()
        .map(|mut dep| {
            dep.names.sort();
            dep.name = dep
                .names
                .iter()
                .find(|n| declared.contains(*n))
                .unwrap_or(&dep.names[0])
                .clone();
            dep
        })
        .collect();
    let rank = |fit: &str| match fit {
        "none" => 0,
        "unknown" => 1,
        "partial" => 2,
        _ => 3,
    };
    dependencies.sort_by(|a, b| {
        rank(a.fit)
            .cmp(&rank(b.fit))
            .then(b.used_by.cmp(&a.used_by))
            .then(a.name.cmp(&b.name))
    });

    let movable: Vec<&crate::analyze::UnitReport> = report
        .units
        .iter()
        .filter(|u| u.status != Status::Keep)
        .collect();
    let index: BTreeMap<&str, usize> = movable
        .iter()
        .enumerate()
        .map(|(i, u)| (u.id.as_str(), i))
        .collect();
    let edges: Vec<Vec<usize>> = movable
        .iter()
        .map(|u| {
            u.internal_deps
                .iter()
                .filter_map(|d| index.get(d.as_str()).copied())
                .collect()
        })
        .collect();
    let (_, members) = crate::analyze::waves(&edges);
    let mut seen = BTreeSet::new();
    let mut pieces = Vec::new();
    for (i, unit) in movable.iter().enumerate() {
        let mut group = members[i].clone();
        if group.is_empty() {
            group.push(i);
        }
        group.sort_unstable();
        if !seen.insert(group.clone()) {
            continue;
        }
        pieces.push(Piece {
            wave: unit.wave,
            units: group.iter().map(|&j| movable[j].id.clone()).collect(),
            loc: group.iter().map(|&j| movable[j].loc).sum(),
            tangled: group.len() > 1,
        });
    }
    pieces.sort_by(|a, b| a.wave.cmp(&b.wave).then(a.units.cmp(&b.units)));

    let mut decisions = Vec::new();
    let mut ask = |topic: String, question: String, recommendation: String| {
        decisions.push(Decision {
            id: format!("D{}", decisions.len() + 1),
            topic,
            question,
            recommendation,
        });
    };
    let entries: Vec<String> = entry_points
        .iter()
        .map(|e| match &e.name {
            Some(n) => format!("{n} ({})", e.path),
            None => e.path.clone(),
        })
        .collect();
    ask(
        "shape".into(),
        format!(
            "The app is a {shape}, entered through {}. Port every entry point, in that shape?",
            entries.join(", ")
        ),
        match shape {
            "server" => {
                "Yes: an axum front door (`app front-door`) takes the server over first".into()
            }
            "cli" => {
                "Yes: a clap front door (`app front-door`) takes every command over first".into()
            }
            _ => "Yes: the Rust crate's public API becomes the library's front door".into(),
        },
    );
    let electron = lang == Lang::Ts
        && serde_json::from_str::<Value>(&read(root, "package.json")).is_ok_and(|pkg| {
            ["dependencies", "devDependencies"]
                .iter()
                .any(|f| pkg[f].get("electron").is_some())
        });
    if electron {
        ask(
            "desktop shell".into(),
            "This is an Electron app. Its main process is Node code that can move to Rust; its \
             windows are web pages. Port the main-process logic and keep the web UI?"
                .into(),
            "Yes: port the main process piece by piece and keep the renderer as it is. Whether \
             the UI later moves to Tauri (the same web UI on a Rust backend) is a separate \
             choice, made once the logic is Rust"
                .into(),
        );
    }
    for unit in report.units.iter().filter(|u| u.status == Status::Keep) {
        ask(
            format!("keep {}", unit.id),
            format!(
                "{} stays in the source language ({}). What replaces it in the Rust app, or does it stay outside?",
                unit.id,
                unit.notes
                    .first()
                    .map_or("no Rust equivalent", |n| n.trim_end_matches('.'))
            ),
            "Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust".into(),
        );
    }
    for unit in report.units.iter().filter(|u| u.status == Status::Blocked) {
        ask(
            format!("blocked {}", unit.id),
            format!(
                "{} is blocked: {}. Rework that part in Rust, or keep the unit outside?",
                unit.id,
                unit.notes
                    .first()
                    .map_or("see its risks", |n| n.trim_end_matches('.'))
            ),
            "Research the blocker first; then port it with the piece that uses it".into(),
        );
    }
    for dep in &dependencies {
        let users = match dep.used_by {
            0 => "declared, no unit imports it".to_string(),
            1 => "used by 1 unit".to_string(),
            n => format!("used by {n} units"),
        };
        let note = dep.note.trim_end_matches('.');
        match dep.fit {
            "none" => ask(
                format!("dependency {}", dep.name),
                format!(
                    "No crate does what {} does ({users}{}). Reimplement what the app uses, or drop the feature?",
                    dep.name,
                    if note.is_empty() {
                        String::new()
                    } else {
                        format!("; {note}")
                    }
                ),
                "Reimplement only the part the app uses, proved by its recorded cases".into(),
            ),
            "unknown" => ask(
                format!("dependency {}", dep.name),
                format!("Which crate replaces {} ({users})?", dep.name),
                format!(
                    "Research it against primary sources into rust/research/{}.md before porting what uses it",
                    dep.name.replace(['/', ':'], "-")
                ),
            ),
            "partial" => ask(
                format!("dependency {}", dep.name),
                format!(
                    "{} maps to {} only in part ({users}{}). Use it?",
                    dep.name,
                    dep.crates.unwrap_or("?"),
                    if note.is_empty() {
                        String::new()
                    } else {
                        format!("; {note}")
                    }
                ),
                format!(
                    "Use {}, and record cases for the behavior it does not cover",
                    dep.crates.unwrap_or("it")
                ),
            ),
            _ => {}
        }
    }
    for piece in pieces.iter().filter(|p| p.tangled) {
        ask(
            "tangled units".into(),
            format!(
                "{} import each other ({} lines). Port them as one piece?",
                piece.units.join(", "),
                piece.loc
            ),
            format!(
                "Yes: one piece in wave {}, no glue between its parts",
                piece.wave
            ),
        );
    }

    WholeApp {
        shape,
        entry_points,
        dependencies,
        pieces,
        decisions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(files: &[(&str, &str)]) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "rust-anything-whole-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        for (path, text) in files {
            let p = dir.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        }
        dir
    }

    #[test]
    fn manifests_declare_dependencies_in_every_language() {
        type Case<'a> = (Lang, &'a [(&'a str, &'a str)], &'a [&'a str]);
        let cases: [Case; 6] = [
            (
                Lang::Ts,
                &[(
                    "package.json",
                    r#"{"dependencies":{"express":"^4"},"devDependencies":{"vitest":"1"}}"#,
                )],
                &["express"],
            ),
            (
                Lang::Python,
                &[
                    (
                        "pyproject.toml",
                        "[project]\nname = \"x\"\ndependencies = [\"Requests>=2\", \"PyYAML\"]\n",
                    ),
                    ("requirements.txt", "# pinned\nclick==8.1\n-r other.txt\n"),
                ],
                &["click", "pyyaml", "requests"],
            ),
            (
                Lang::Go,
                &[(
                    "go.mod",
                    "module x\n\nrequire (\n\tgithub.com/spf13/cobra v1.8.0\n\tgolang.org/x/sys v0.1.0 // indirect\n)\nrequire github.com/google/uuid v1.6.0\n",
                )],
                &["github.com/google/uuid", "github.com/spf13/cobra"],
            ),
            (
                Lang::Ruby,
                &[
                    (
                        "Gemfile",
                        "source 'x'\ngem \"sinatra\"\ngroup :test do\n  gem 'rspec'\nend\n",
                    ),
                    ("x.gemspec", "s.add_dependency 'thor', '~> 1'\n"),
                ],
                &["sinatra", "thor"],
            ),
            (
                Lang::Java,
                &[(
                    "pom.xml",
                    "<dependencies><dependency><groupId>com.google.code.gson</groupId><artifactId>gson</artifactId></dependency><dependency><groupId>org.junit</groupId><artifactId>junit</artifactId><scope>test</scope></dependency></dependencies>",
                )],
                &["com.google.code.gson:gson"],
            ),
            (
                Lang::Kotlin,
                &[(
                    "build.gradle.kts",
                    "dependencies {\n  implementation(\"com.google.code.gson:gson:2.11.0\")\n  testImplementation(\"junit:junit:4.13.2\")\n}\n",
                )],
                &["com.google.code.gson:gson"],
            ),
        ];
        for (lang, files, expected) in cases {
            let dir = repo(files);
            let found: Vec<String> = declared(&dir, lang).into_iter().collect();
            assert_eq!(found, expected, "{lang:?}");
            std::fs::remove_dir_all(&dir).unwrap();
        }
    }

    #[test]
    fn manifests_name_entry_points() {
        let dir = repo(&[
            (
                "package.json",
                r#"{"name":"@me/tool","bin":{"tool":"./bin/tool.js"}}"#,
            ),
            (
                "pyproject.toml",
                "[project.scripts]\nmytool = \"mytool.cli:main\"\n",
            ),
        ]);
        let found: Vec<(String, String)> = manifest_entries(&dir, Lang::Ts)
            .into_iter()
            .map(|e| (e.name.unwrap_or_default(), e.path))
            .collect();
        assert_eq!(
            found,
            [
                ("tool".to_string(), "bin/tool.js".to_string()),
                ("mytool".to_string(), "mytool.cli:main".to_string())
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_glob_in_a_gradle_string_does_not_open_a_comment() {
        let dir = repo(&[(
            "build.gradle.kts",
            "android {\n    packaging { resources { excludes += \"META-INF/*\" } }\n}\n\
             dependencies {\n    implementation(\"com.google.code.gson:gson:2.11.0\")\n}\n\
             /* implementation(\"com.example:legacy:1.0\") */\n",
        )]);
        let found: Vec<String> = declared(&dir, Lang::Kotlin).into_iter().collect();
        assert_eq!(found, ["com.google.code.gson:gson"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
