//! Renders a [`Report`] as a Markdown migration plan for people.

use std::fmt::Write;

use crate::analyze::{Report, Status, UnitReport};

const DETAILED_CANDIDATES: usize = 5;

pub fn markdown(report: &Report, title: &str) -> String {
    let mut out = String::new();
    // Writing to a String cannot fail.
    let _ = render(&mut out, report, title);
    if let Some(w) = &report.whole_app {
        let _ = whole_app(&mut out, w);
    }
    out
}

fn whole_app(out: &mut String, w: &crate::whole_app::WholeApp) -> std::fmt::Result {
    writeln!(out, "\n## Whole app\n")?;
    writeln!(out, "Shape: **{}**.\n", w.shape)?;
    writeln!(out, "| Entry point | Kind | Found from |")?;
    writeln!(out, "| --- | --- | --- |")?;
    for e in &w.entry_points {
        let name = e.name.as_ref().map_or(String::new(), |n| format!("{n}: "));
        writeln!(out, "| {name}`{}` | {} | {} |", e.path, e.kind, e.evidence)?;
    }
    writeln!(out, "\n### Every dependency\n")?;
    if w.dependencies.is_empty() {
        writeln!(out, "None.")?;
    } else {
        writeln!(
            out,
            "| Dependency | Declared | Used by | Rust | Fit | Note |"
        )?;
        writeln!(out, "| --- | --- | --- | --- | --- | --- |")?;
        for d in &w.dependencies {
            let others: Vec<String> = d
                .names
                .iter()
                .filter(|n| **n != d.name)
                .map(|n| format!("`{n}`"))
                .collect();
            writeln!(
                out,
                "| `{}`{} | {} | {} | {} | {} | {} |",
                d.name,
                if others.is_empty() {
                    String::new()
                } else {
                    format!(" (also {})", others.join(", "))
                },
                if d.declared { "yes" } else { "no" },
                d.used_by,
                d.crates.unwrap_or("?"),
                d.fit,
                d.note
            )?;
        }
    }
    writeln!(out, "\n### Pieces, in port order\n")?;
    for p in &w.pieces {
        writeln!(
            out,
            "- wave {}: {}{} ({} lines)",
            p.wave,
            p.units
                .iter()
                .map(|u| format!("`{u}`"))
                .collect::<Vec<_>>()
                .join(", "),
            if p.tangled {
                ", tangled: one piece"
            } else {
                ""
            },
            p.loc
        )?;
    }
    writeln!(out, "\n### Decisions for the user\n")?;
    for d in &w.decisions {
        writeln!(
            out,
            "**{}** ({}): {}\n\n> {}\n",
            d.id, d.topic, d.question, d.recommendation
        )?;
    }
    Ok(())
}

fn render(out: &mut String, r: &Report, title: &str) -> std::fmt::Result {
    let s = &r.summary;
    writeln!(
        out,
        "# Rust migration plan: {title} ({})\n",
        r.language.name()
    )?;
    writeln!(
        out,
        "{}, {} ({} lines), {}, {}, grouped into {}.\n",
        count(s.packages, "package"),
        count(s.source_files, "source file"),
        s.loc,
        count(s.test_files, "test file"),
        count(s.ui_files, "UI file"),
        count(s.units, "unit")
    )?;
    for warning in &r.warnings {
        writeln!(out, "> **Warning:** {warning}\n")?;
    }
    writeln!(
        out,
        "| Ready | Caution | Blocked | Keep in source language |"
    )?;
    writeln!(out, "| ---: | ---: | ---: | ---: |")?;
    writeln!(
        out,
        "| {} | {} | {} | {} |\n",
        s.ready, s.caution, s.blocked, s.keep
    )?;
    if s.parse_errors > 0 {
        writeln!(
            out,
            "> {} had parse errors; their units are marked caution.\n",
            count(s.parse_errors, "file")
        )?;
    }

    writeln!(out, "## Start here\n")?;
    if r.start_here.is_empty() {
        if s.blocked > 0 {
            writeln!(
                out,
                "No unit is a safe first port yet. See the blocked units and dependencies below.\n"
            )?;
        } else if s.units == 0 {
            writeln!(out, "No unit to port was found.\n")?;
        } else if s.ready + s.caution == 0 {
            writeln!(
                out,
                "No unit to port first: every unit stays in the source language.\n"
            )?;
        } else {
            writeln!(
                out,
                "No unit exports anything a bridge can carry yet: the units below expose constants, types or nothing a port could call.\n"
            )?;
        }
    } else {
        writeln!(
            out,
            "Best first ports: safe to port (readiness) and worth porting (loops and string work, tests, size).\n"
        )?;
        writeln!(
            out,
            "| # | Unit | Lines | Priority | Readiness | Tests | Bridge | Main risks |"
        )?;
        writeln!(
            out,
            "| ---: | --- | ---: | ---: | ---: | ---: | --- | --- |"
        )?;
        for (i, id) in r.start_here.iter().enumerate() {
            let Some(u) = unit(r, id) else { continue };
            writeln!(
                out,
                "| {} | `{}` | {} | {} | {} | {} | {} | {} |",
                i + 1,
                u.id,
                u.loc,
                u.priority,
                u.readiness,
                u.direct_tests,
                u.bridge.label(),
                top_risks(u, 3)
            )?;
        }
        writeln!(out)?;
        for id in r.start_here.iter().take(DETAILED_CANDIDATES) {
            if let Some(u) = unit(r, id) {
                unit_detail(out, u)?;
            }
        }
    }

    writeln!(out, "## Order by package\n")?;
    writeln!(
        out,
        "Packages come after the workspace packages they use. Inside a package, port by wave: a unit's imports are in earlier waves.\n"
    )?;
    for p in &r.packages {
        let deps = if p.workspace_deps.is_empty() {
            String::new()
        } else {
            format!(", uses {}", p.workspace_deps.join(", "))
        };
        writeln!(
            out,
            "### {}. {} (`{}`, {}, {}{deps})\n",
            p.order,
            p.name,
            if p.dir.is_empty() { "." } else { &p.dir },
            p.runtime,
            count(p.test_files, "test file")
        )?;
        writeln!(
            out,
            "| Wave | Unit | Status | Lines | Readiness | Bridge | Main risks |"
        )?;
        writeln!(out, "| ---: | --- | --- | ---: | ---: | --- | --- |")?;
        let mut units: Vec<&UnitReport> = r.units.iter().filter(|u| u.package == p.name).collect();
        units.sort_by(|a, b| a.wave.cmp(&b.wave).then(b.readiness.cmp(&a.readiness)));
        for u in units {
            writeln!(
                out,
                "| {} | `{}` | {} | {} | {} | {} | {} |",
                u.wave,
                u.id,
                status_label(u.status),
                u.loc,
                u.readiness,
                u.bridge.label(),
                top_risks(u, 2)
            )?;
        }
        writeln!(out)?;
    }

    if !r.dependencies.is_empty() {
        writeln!(out, "## {} dependencies\n", r.language.registry())?;
        writeln!(out, "| Package | Units | Rust | Fit | Note |")?;
        writeln!(out, "| --- | ---: | --- | --- | --- |")?;
        for d in &r.dependencies {
            writeln!(
                out,
                "| `{}` | {} | {} | {} | {} |",
                d.name,
                d.used_by,
                d.crates.unwrap_or("?"),
                d.fit,
                d.note
            )?;
        }
        writeln!(out)?;
    }

    let blocked: Vec<&UnitReport> = r
        .units
        .iter()
        .filter(|u| u.status == Status::Blocked)
        .collect();
    if !blocked.is_empty() {
        writeln!(out, "## Blocked\n")?;
        for u in blocked {
            writeln!(
                out,
                "- `{}`: {}",
                u.id,
                u.notes.first().map_or("", String::as_str)
            )?;
        }
        writeln!(out)?;
    }

    writeln!(out, "## Risk guide\n")?;
    let mut seen = std::collections::BTreeSet::new();
    for u in &r.units {
        seen.extend(u.risks.iter().map(|r| r.risk));
    }
    for risk in seen {
        writeln!(
            out,
            "- **{}** ({:?}): {}",
            risk.title(),
            risk.severity(),
            risk.guidance()
        )?;
    }
    Ok(())
}

fn count(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

fn unit_detail(out: &mut String, u: &UnitReport) -> std::fmt::Result {
    writeln!(out, "### `{}`\n", u.id)?;
    writeln!(
        out,
        "{} lines in {}, wave {}, priority {}, readiness {}, bridge **{}**.\n",
        u.loc,
        count(u.files.len(), "file"),
        u.wave,
        u.priority,
        u.readiness,
        u.bridge.label()
    )?;
    for note in &u.notes {
        writeln!(out, "- {note}")?;
    }
    for risk in &u.risks {
        writeln!(
            out,
            "- {} x{}: {}",
            risk.risk.title(),
            risk.count,
            risk.samples
                .iter()
                .map(|s| format!("`{s}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )?;
    }
    writeln!(out)
}

fn unit<'r>(r: &'r Report, id: &str) -> Option<&'r UnitReport> {
    r.units.iter().find(|u| u.id == id)
}

fn top_risks(u: &UnitReport, n: usize) -> String {
    if u.risks.is_empty() {
        return "-".into();
    }
    u.risks
        .iter()
        .take(n)
        .map(|r| format!("{} ({})", r.risk.title(), r.count))
        .collect::<Vec<_>>()
        .join("; ")
}

fn status_label(status: Status) -> &'static str {
    match status {
        Status::Ready => "ready",
        Status::Caution => "caution",
        Status::Blocked => "blocked",
        Status::Keep => "keep",
    }
}
