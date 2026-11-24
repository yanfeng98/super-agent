//! What a scan finds: functions, how much of the run they cost, and whether they can move to Rust.

use serde::{Deserialize, Serialize};

/// Functions under this share of the run aren't worth porting.
pub const WORTH_IT: f64 = 0.02;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Param {
    pub name: String,
    pub annotation: Option<String>,
    /// Default value as source text, e.g. "2" or "None".
    pub default: Option<String>,
    /// Keyword-only (after `*` in Python).
    pub kwonly: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Hotspot {
    pub id: String,
    pub file: String,
    pub line: u32,
    pub end_line: u32,
    pub name: String,
    pub qualname: String,
    pub module: Option<String>,
    pub kind: String,
    pub ncalls: u64,
    pub tottime: f64,
    pub cumtime: f64,
    /// Share of the run spent in this function, callees included.
    pub share: f64,
    /// 0 = can't be ported, 1 = ideal candidate.
    pub portability: f64,
    pub score: f64,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub params: Vec<Param>,
    pub returns: Option<String>,
    pub source: String,
    /// Functions from the same file this one calls.
    pub calls: Vec<String>,
    /// A portable caller that takes about as long: porting it covers this one too,
    /// and saves a Python/JS -> Rust crossing per call.
    pub covered_by: Option<String>,
    /// Ruby: the module (or Object) whose method this is, as a Ruby expression.
    /// Python: the class of a @staticmethod.
    pub owner: Option<String>,
    /// Python: dataclasses and NamedTuples from the same file that the signature uses, with their fields.
    pub records: std::collections::BTreeMap<String, Vec<Param>>,
}

impl Hotspot {
    pub fn portable(&self) -> bool {
        self.blockers.is_empty()
    }

    pub fn worth_it(&self) -> bool {
        self.portable() && self.covered_by.is_none() && self.share >= WORTH_IT
    }

    /// Why it isn't worth porting, if it isn't.
    pub fn why_not(&self) -> Option<String> {
        if let Some(b) = self.blockers.first() {
            Some(b.clone())
        } else if let Some(by) = &self.covered_by {
            Some(format!("ported inside {by}"))
        } else if self.share < WORTH_IT {
            Some(format!(
                "too small to matter ({:.1}% of the run, {:.0}% needed)",
                self.share * 100.0,
                WORTH_IT * 100.0
            ))
        } else {
            None
        }
    }
}

/// Blockers about what a function does, not where it lives or what its signature looks like.
/// A helper with one of these can't be ported inside its caller either: the Rust copy would have to do
/// the same thing, and verify only compares results, so a port that skipped it would still pass.
/// The backends word them differently, so this lists each wording.
const PASSED_ON: [&str; 8] = [
    "does I/O",
    "not deterministic",
    "writes global state",
    "writes static fields",
    "writes package-level variables",
    "writes instance, class or global variables",
    "uses global variables",
    "keeps static variables",
];

/// Whether `called`, as written at a call site in `file`, is the name `name`.
/// PHP function and method names are case-insensitive and may be written `\name`.
fn same_name(file: &str, called: &str, name: &str) -> bool {
    if file.ends_with(".php") { called.trim_start_matches('\\').eq_ignore_ascii_case(name) } else { called == name }
}

/// Python and Node only record calls to module-level functions, so there a call never means a method.
const MODULE_LEVEL_CALLS: [&str; 9] = [".py", ".js", ".mjs", ".cjs", ".jsx", ".ts", ".mts", ".cts", ".tsx"];

/// Indices of the functions a call to `called` from `caller` may mean: same file, same name, never the
/// caller itself (an overload shares its name, not its line). When some of them are also the closer
/// match, only those: in Python and Node a module-level function, elsewhere one with the caller's kind
/// and owner, so a method of the caller's own class over one of another class. Overloads all stay in.
fn helper_indices(list: &[Hotspot], caller: &Hotspot, called: &str) -> Vec<usize> {
    let all: Vec<usize> = (0..list.len())
        .filter(|&j| {
            let g = &list[j];
            g.file == caller.file
                && same_name(&caller.file, called, &g.name)
                && !(g.line == caller.line && g.name == caller.name)
        })
        .collect();
    let module_level = MODULE_LEVEL_CALLS.iter().any(|e| caller.file.ends_with(e));
    let near: Vec<usize> = all
        .iter()
        .copied()
        .filter(|&j| {
            let g = &list[j];
            if module_level {
                g.kind == "function" && g.owner.is_none()
            } else {
                g.kind == caller.kind && g.owner == caller.owner
            }
        })
        .collect();
    if near.is_empty() { all } else { near }
}

/// The functions a call to `called` from `caller` may mean (see [`helper_indices`]).
/// `rank` and `port` both find helpers through this.
pub fn helpers_named<'a>(list: &'a [Hotspot], caller: &Hotspot, called: &str) -> Vec<&'a Hotspot> {
    helper_indices(list, caller, called).into_iter().map(|j| &list[j]).collect()
}

/// A function that calls a helper with a [`PASSED_ON`] blocker gets that blocker too, through any
/// number of helpers, including ones blocked only by where they live (they're ported inside the caller
/// all the same). When a call may mean more than one function, one of them doing I/O is enough.
fn pass_on_helper_blockers(list: &mut [Hotspot]) {
    // what each function passes on to its callers, if anything
    let mut passed: Vec<Option<String>> =
        list.iter().map(|h| h.blockers.iter().find(|b| PASSED_ON.iter().any(|p| b.starts_with(p))).cloned()).collect();
    loop {
        let mut found = Vec::new();
        for (i, h) in list.iter().enumerate().filter(|(i, _)| passed[*i].is_none()) {
            let hit = h.calls.iter().find_map(|called| {
                let b = helper_indices(list, h, called).into_iter().find_map(|j| passed[j].as_ref())?;
                let verb = if b.starts_with("not ") { "is " } else { "" };
                Some(format!("calls {called}, which {verb}{b}"))
            });
            if let Some(blocker) = hit {
                found.push((i, blocker));
            }
        }
        if found.is_empty() {
            return;
        }
        for (i, blocker) in found {
            list[i].blockers.push(blocker.clone());
            passed[i] = Some(blocker);
        }
    }
}

/// Give each function an id and a score (time share x portability), best first.
pub fn rank(mut list: Vec<Hotspot>, wall: f64) -> Vec<Hotspot> {
    pass_on_helper_blockers(&mut list);
    for h in &mut list {
        h.id = match h.module.as_deref() {
            Some(m) if m != "__main__" => format!("{m}.{}", h.qualname),
            _ => format!("{}:{}", h.file, h.qualname),
        };
        h.share = if wall > 0.0 { (h.cumtime / wall).clamp(0.0, 1.0) } else { 0.0 };
        h.portability = if h.portable() { h.portability.clamp(0.0, 1.0) } else { 0.0 };
        h.score = h.share * h.portability;
    }
    // Prefer the outermost portable function: one call into Rust instead of one per inner call.
    let covers: Vec<Option<String>> = (0..list.len())
        .map(|i| {
            let h = &list[i];
            if !h.portable() {
                return None;
            }
            list.iter()
                .filter(|g| {
                    g.portable()
                        && g.id != h.id
                        && g.share >= 0.9 * h.share
                        && g.calls.iter().any(|c| helper_indices(&list, g, c).contains(&i))
                })
                .max_by(|a, b| a.share.total_cmp(&b.share))
                .map(|g| g.id.clone())
        })
        .collect();
    for (h, cover) in list.iter_mut().zip(covers) {
        if cover.is_some() {
            h.covered_by = cover;
            h.score = 0.0;
        }
    }
    list.sort_by(|a, b| b.score.total_cmp(&a.score).then(b.share.total_cmp(&a.share)));
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hs(name: &str, module: Option<&str>, cumtime: f64, portability: f64, blocked: bool) -> Hotspot {
        Hotspot {
            name: name.into(),
            qualname: name.into(),
            file: "app.py".into(),
            module: module.map(Into::into),
            cumtime,
            portability,
            blockers: if blocked { vec!["does I/O: print".into()] } else { vec![] },
            ..Default::default()
        }
    }

    #[test]
    fn outer_loop_covers_inner_function() {
        let mut outer = hs("find_duplicates", Some("fuzzy"), 9.9, 1.0, false);
        outer.calls = vec!["lev".into()];
        let ranked = rank(vec![hs("lev", Some("fuzzy"), 9.5, 1.0, false), outer], 10.0);
        assert_eq!(ranked[0].id, "fuzzy.find_duplicates");
        assert_eq!(ranked[1].covered_by.as_deref(), Some("fuzzy.find_duplicates"));
        assert!(!ranked[1].worth_it());
    }

    #[test]
    fn portable_beats_bigger_but_blocked() {
        let ranked = rank(
            vec![
                hs("main", Some("__main__"), 10.0, 1.0, true),
                hs("lev", Some("fuzzy"), 9.0, 1.0, false),
                hs("norm", Some("fuzzy"), 0.1, 1.0, false),
            ],
            10.0,
        );
        assert_eq!(ranked[0].id, "fuzzy.lev");
        assert!((ranked[0].share - 0.9).abs() < 1e-9);
        assert_eq!(ranked[1].id, "fuzzy.norm");
        assert_eq!(ranked[2].id, "app.py:main");
        assert_eq!(ranked[2].score, 0.0);
        assert!(!ranked[1].worth_it());
    }

    fn calling(mut h: Hotspot, calls: &[&str]) -> Hotspot {
        h.calls = calls.iter().map(|c| c.to_string()).collect();
        h
    }

    fn blocked_by(mut h: Hotspot, blocker: &str) -> Hotspot {
        h.blockers = vec![blocker.into()];
        h
    }

    fn by_id<'a>(ranked: &'a [Hotspot], id: &str) -> &'a Hotspot {
        ranked.iter().find(|h| h.id == id).unwrap()
    }

    #[test]
    fn caller_of_a_helper_that_does_io_is_blocked() {
        // build_all spends its time waiting on run_tool's subprocess: porting it would drop the subprocess
        let ranked = rank(
            vec![
                calling(hs("build_all", Some("work"), 9.9, 1.0, false), &["checksum", "run_tool"]),
                blocked_by(hs("run_tool", Some("work"), 9.8, 1.0, false), "does I/O: subprocess.run"),
                hs("checksum", Some("work"), 0.1, 1.0, false),
            ],
            10.0,
        );
        let outer = by_id(&ranked, "work.build_all");
        assert!(!outer.portable() && !outer.worth_it());
        assert_eq!(outer.blockers, ["calls run_tool, which does I/O: subprocess.run"]);
        assert_eq!(outer.score, 0.0);
        assert!(ranked.iter().all(|h| !h.worth_it()), "nothing here is worth porting");
    }

    #[test]
    fn helper_blockers_pass_up_a_chain_and_through_cycles() {
        let ranked = rank(
            vec![
                calling(hs("top", Some("m"), 10.0, 1.0, false), &["mid"]),
                calling(hs("mid", Some("m"), 9.0, 1.0, false), &["leaf", "top"]),
                blocked_by(hs("leaf", Some("m"), 8.0, 1.0, false), "not deterministic: random.random"),
            ],
            10.0,
        );
        assert_eq!(by_id(&ranked, "m.mid").blockers, ["calls leaf, which is not deterministic: random.random"]);
        assert_eq!(
            by_id(&ranked, "m.top").blockers,
            ["calls mid, which calls leaf, which is not deterministic: random.random"]
        );
    }

    #[test]
    fn where_a_helper_lives_does_not_block_its_caller() {
        // a helper is ported inside its caller, so being a method or living in the entry script doesn't matter,
        // and a same-named function in another file isn't the helper
        let ranked = rank(
            vec![
                calling(hs("outer", Some("m"), 10.0, 1.0, false), &["inner", "save"]),
                blocked_by(hs("inner", Some("m"), 9.0, 1.0, false), "nested function"),
                Hotspot {
                    file: "other.py".into(),
                    ..blocked_by(hs("save", Some("other"), 1.0, 1.0, false), "does I/O: open")
                },
            ],
            10.0,
        );
        assert!(by_id(&ranked, "m.outer").worth_it());
    }

    #[test]
    fn helper_blockers_pass_through_a_helper_blocked_only_by_where_it_lives() {
        let ranked = rank(
            vec![
                calling(hs("top", Some("m"), 10.0, 1.0, false), &["mid"]),
                calling(blocked_by(hs("mid", Some("m"), 9.0, 1.0, false), "nested function"), &["leaf"]),
                blocked_by(hs("leaf", Some("m"), 8.0, 1.0, false), "does I/O: open"),
            ],
            10.0,
        );
        assert_eq!(by_id(&ranked, "m.top").blockers, ["calls mid, which calls leaf, which does I/O: open"]);
    }

    #[test]
    fn a_blocked_caller_no_longer_covers_its_portable_helper() {
        let ranked = rank(
            vec![
                calling(hs("outer", Some("m"), 10.0, 1.0, false), &["crunch", "save"]),
                hs("crunch", Some("m"), 9.5, 1.0, false),
                blocked_by(hs("save", Some("m"), 0.4, 1.0, false), "does I/O: open"),
            ],
            10.0,
        );
        assert_eq!(ranked[0].id, "m.crunch");
        assert!(ranked[0].worth_it() && ranked[0].covered_by.is_none());
        assert!(!by_id(&ranked, "m.outer").portable());
    }

    #[test]
    fn a_cycle_without_blockers_stays_portable() {
        let ranked = rank(
            vec![
                calling(hs("a", Some("m"), 10.0, 1.0, false), &["b"]),
                calling(hs("b", Some("m"), 9.0, 1.0, false), &["a"]),
            ],
            10.0,
        );
        assert!(ranked.iter().all(|h| h.portable()));
    }

    #[test]
    fn a_method_of_the_same_name_is_not_the_helper() {
        // `outer` calls the top-level `run`, not `Runner.run`, whichever the profiler listed first
        let function = |h: Hotspot| Hotspot { kind: "function".into(), ..h };
        let method = |h: Hotspot| Hotspot { kind: "method".into(), qualname: "Runner.run".into(), ..h };
        let outer = || function(calling(hs("outer", Some("m"), 10.0, 1.0, false), &["run"]));
        let ranked = rank(
            vec![
                outer(),
                method(blocked_by(hs("run", Some("m"), 5.0, 1.0, false), "does I/O: print")),
                function(hs("run", Some("m"), 0.1, 1.0, false)),
            ],
            10.0,
        );
        assert!(by_id(&ranked, "m.outer").portable());
        let ranked = rank(
            vec![
                outer(),
                method(hs("run", Some("m"), 5.0, 1.0, false)),
                function(blocked_by(hs("run", Some("m"), 0.1, 1.0, false), "does I/O: subprocess.run")),
            ],
            10.0,
        );
        assert_eq!(by_id(&ranked, "m.outer").blockers, ["calls run, which does I/O: subprocess.run"]);
    }

    #[test]
    fn in_python_a_bare_call_from_a_staticmethod_means_the_module_level_function() {
        let static_in_text = |h: Hotspot| Hotspot { kind: "method".into(), owner: Some("Text".into()), ..h };
        let ranked = rank(
            vec![
                static_in_text(calling(hs("clean", Some("m"), 10.0, 1.0, false), &["norm"])),
                static_in_text(hs("norm", Some("m"), 5.0, 1.0, false)),
                Hotspot {
                    kind: "function".into(),
                    ..blocked_by(hs("norm", Some("m"), 0.1, 1.0, false), "does I/O: open")
                },
            ],
            10.0,
        );
        assert_eq!(by_id(&ranked, "m.clean").blockers, ["calls norm, which does I/O: open"]);
    }

    #[test]
    fn php_helpers_are_covered_by_their_caller_whatever_the_case() {
        let php = |h: Hotspot| Hotspot { file: "app.php".into(), ..h };
        let ranked = rank(
            vec![
                php(calling(hs("outer", Some("m"), 10.0, 1.0, false), &["\\CRUNCH"])),
                php(hs("crunch", Some("m"), 9.5, 1.0, false)),
            ],
            10.0,
        );
        assert_eq!(by_id(&ranked, "m.crunch").covered_by.as_deref(), Some("m.outer"));
    }

    #[test]
    fn each_backends_wording_passes_on() {
        for (file, blocker) in [
            ("App.java", "writes static fields"),
            ("Program.cs", "writes static fields"),
            ("main.go", "writes package-level variables"),
            ("app.rb", "writes instance, class or global variables"),
            ("app.php", "uses global variables"),
            ("main.c", "keeps static variables"),
            ("app.py", "writes global state"),
        ] {
            let at = |h: Hotspot| Hotspot { file: file.into(), ..h };
            let ranked = rank(
                vec![
                    at(calling(hs("outer", Some("m"), 10.0, 1.0, false), &["bump"])),
                    at(blocked_by(hs("bump", Some("m"), 9.0, 1.0, false), blocker)),
                ],
                10.0,
            );
            assert_eq!(by_id(&ranked, "m.outer").blockers, [format!("calls bump, which {blocker}")], "{file}");
        }
    }

    #[test]
    fn signature_and_ruby_super_blockers_do_not_pass_on() {
        for blocker in ["calls super", "returns nothing (void)", "takes vararg"] {
            let ranked = rank(
                vec![
                    calling(hs("outer", Some("m"), 10.0, 1.0, false), &["inner"]),
                    blocked_by(hs("inner", Some("m"), 9.0, 1.0, false), blocker),
                ],
                10.0,
            );
            assert!(by_id(&ranked, "m.outer").portable(), "{blocker}");
        }
    }

    #[test]
    fn php_helper_names_ignore_case_and_a_leading_backslash() {
        for called in ["runtool", "\\runTool"] {
            let php = |h: Hotspot| Hotspot { file: "app.php".into(), ..h };
            let ranked = rank(
                vec![
                    php(calling(hs("outer", Some("m"), 10.0, 1.0, false), &[called])),
                    php(blocked_by(hs("runTool", Some("m"), 9.0, 1.0, false), "does I/O: exec")),
                ],
                10.0,
            );
            assert!(!by_id(&ranked, "m.outer").portable(), "{called}");
        }
    }
}
