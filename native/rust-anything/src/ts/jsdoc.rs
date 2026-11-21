//! Types for untyped JavaScript: JSDoc `@param`/`@returns` tags and sibling `.d.ts` declarations,
//! written in the same TS dialect the TypeScript path reads.

use std::path::Path;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::exports::ExportedFn;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamTag {
    pub name: String,
    pub ts_type: Option<String>,
    pub optional: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JsDoc {
    pub params: Vec<ParamTag>,
    pub returns: Option<String>,
}

impl JsDoc {
    pub fn param(&self, name: &str) -> Option<&ParamTag> {
        self.params.iter().find(|p| p.name == name)
    }
}

pub fn leading(source: &str, offset: usize) -> Option<JsDoc> {
    let before = source[..offset].trim_end();
    let body_end = before.strip_suffix("*/")?.len();
    let start = before[..body_end].rfind("/**")?;
    let body = &before[start + 3..body_end];
    if body.contains("*/") {
        return None;
    }
    Some(parse(body))
}

pub fn parse(body: &str) -> JsDoc {
    let mut chunks: Vec<String> = Vec::new();
    for line in body.lines() {
        let line = line.trim_start();
        let line = line.strip_prefix('*').unwrap_or(line).trim();
        match (line.strip_prefix('@'), chunks.last_mut()) {
            (Some(tag), _) => chunks.push(tag.to_string()),
            (None, Some(chunk)) => {
                chunk.push('\n');
                chunk.push_str(line);
            }
            (None, None) => {}
        }
    }
    let mut doc = JsDoc::default();
    let mut nested: Vec<(String, String, Option<String>, bool)> = Vec::new();
    for chunk in &chunks {
        let chunk = chunk.as_str();
        let (tag, rest) = chunk.split_once(char::is_whitespace).unwrap_or((chunk, ""));
        let rest = rest.trim_start();
        let (raw_type, rest) = match braced(rest) {
            Some((t, after)) => (Some(t), after.trim_start()),
            None => (None, rest),
        };
        match tag {
            "param" | "arg" | "argument" => {
                let Some((name, bracketed)) = param_name(rest) else {
                    continue;
                };
                let (ts_type, optional_type) = match raw_type {
                    Some(t) => match to_ts(t) {
                        Some((ts, opt)) => (Some(ts), opt),
                        None => (None, false),
                    },
                    None => (None, false),
                };
                let optional = bracketed || optional_type;
                if let Some((parent, field)) = name.split_once('.') {
                    nested.push((parent.to_string(), field.to_string(), ts_type, optional));
                } else {
                    doc.params.push(ParamTag {
                        name,
                        ts_type,
                        optional,
                    });
                }
            }
            "returns" | "return" => {
                doc.returns = raw_type.and_then(to_ts).map(|(t, _)| t);
            }
            _ => {}
        }
    }
    for p in &mut doc.params {
        let fields: Vec<_> = nested
            .iter()
            .filter(|(parent, ..)| *parent == p.name)
            .collect();
        if fields.is_empty() || !matches!(p.ts_type.as_deref(), Some("object" | "Object")) {
            continue;
        }
        if fields
            .iter()
            .any(|(_, f, t, _)| f.contains('.') || t.is_none())
        {
            p.ts_type = None;
            continue;
        }
        let members: Vec<String> = fields
            .iter()
            .map(|(_, f, t, opt)| {
                format!(
                    "{f}{}: {}",
                    if *opt { "?" } else { "" },
                    t.as_deref().unwrap_or("unknown")
                )
            })
            .collect();
        p.ts_type = Some(format!("{{ {} }}", members.join("; ")));
    }
    doc
}

fn braced(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix('{')?;
    let mut depth = 1;
    for (i, c) in rest.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&rest[..i], &rest[i + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

fn param_name(text: &str) -> Option<(String, bool)> {
    if let Some(inner) = text.strip_prefix('[') {
        let end = inner.find(']')?;
        let name = inner[..end].split('=').next()?.trim();
        return valid_name(name).then(|| (name.to_string(), true));
    }
    let name = text.split(|c: char| c.is_whitespace()).next()?;
    let name = name.trim_end_matches(['-', ',']);
    valid_name(name).then(|| (name.to_string(), false))
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '$' | '.'))
}

pub fn to_ts(jsdoc: &str) -> Option<(String, bool)> {
    let mut t = jsdoc.trim();
    let optional = t.ends_with('=');
    t = t.trim_end_matches('=').trim();
    let rest = t.starts_with("...");
    t = t.trim_start_matches("...").trim();
    if t.contains("function(") || t.contains("=>") {
        return None;
    }
    let mut ts = match t {
        "" | "*" | "?" => "unknown".to_string(),
        _ => {
            let (nullable, inner) = match (t.strip_prefix('?'), t.strip_prefix('!')) {
                (Some(inner), _) => (true, inner),
                (_, Some(inner)) => (false, inner),
                _ => (false, t),
            };
            let inner = normalize(inner.trim());
            if nullable {
                format!("({inner}) | null")
            } else {
                inner
            }
        }
    };
    if rest {
        ts = format!("({ts})[]");
    }
    is_type(&ts).then_some((ts, optional))
}

fn normalize(t: &str) -> String {
    let t = t.replace("Object.<", "Record<").replace(".<", "<");
    let mut out = String::with_capacity(t.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        out.push_str(match word.as_str() {
            "String" => "string",
            "Number" => "number",
            "Boolean" => "boolean",
            "Object" => "object",
            w => w,
        });
        word.clear();
    };
    for c in t.chars() {
        if c.is_alphanumeric() || c == '_' || c == '$' {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

fn is_type(ts: &str) -> bool {
    let source = format!("type __T = {ts};");
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, &source, SourceType::ts()).parse();
    ret.diagnostics.is_empty() && ret.program.body.len() == 1
}

pub fn declaration_file(root: &Path, file: &str) -> Option<String> {
    let (stem, ext) = file.rsplit_once('.')?;
    let own = match ext {
        "mjs" => "d.mts",
        "cjs" => "d.cts",
        "js" | "jsx" => "d.ts",
        _ => return None,
    };
    [own, "d.ts", "d.mts", "d.cts"]
        .iter()
        .map(|d| format!("{stem}.{d}"))
        .find(|candidate| root.join(candidate).is_file())
}

pub fn apply_declarations(root: &Path, file: &str, fns: &mut [ExportedFn]) {
    let Some(decl_file) = declaration_file(root, file) else {
        return;
    };
    let Ok(text) = std::fs::read_to_string(root.join(&decl_file)) else {
        return;
    };
    let declared = super::exports::declared_functions(&decl_file, &text);
    for f in fns {
        let mut matching = declared.iter().filter(|d| d.name == f.name);
        let (Some(d), None) = (matching.next(), matching.next()) else {
            continue;
        };
        for (p, dp) in f.params.iter_mut().zip(&d.params) {
            if dp.rest != p.rest {
                break;
            }
            if dp.ts_type != "unknown" {
                p.ts_type = dp.ts_type.clone();
            }
            p.optional |= dp.optional;
        }
        if d.returns != "unknown" {
            f.returns = d.returns.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_jsdoc_types() {
        let ts = |t: &str| to_ts(t).map(|(t, _)| t);
        assert_eq!(ts("string"), Some("string".into()));
        assert_eq!(ts("Array.<String>"), Some("Array<string>".into()));
        assert_eq!(ts("?number"), Some("(number) | null".into()));
        assert_eq!(ts("string|undefined"), Some("string|undefined".into()));
        assert_eq!(
            ts("Object.<string, number>"),
            Some("Record<string, number>".into())
        );
        assert_eq!(
            ts("{a: string, b: number}"),
            Some("{a: string, b: number}".into())
        );
        assert_eq!(ts("...string"), Some("(string)[]".into()));
        assert_eq!(ts("*"), Some("unknown".into()));
        assert_eq!(to_ts("number="), Some(("number".into(), true)));
        assert_eq!(ts("function(string): number"), None);
    }

    #[test]
    fn reads_param_and_return_tags() {
        let doc = parse(
            "\n * Splits a line.\n * @param {string} source - the text\n * @param {string} [delimiter=\",\"]\n\
             * @param {Object} opts\n * @param {boolean} opts.trim\n * @param {number} [opts.limit]\n\
             * @returns {string[]}\n ",
        );
        assert_eq!(
            doc.param("source").unwrap().ts_type.as_deref(),
            Some("string")
        );
        assert!(!doc.param("source").unwrap().optional);
        assert!(doc.param("delimiter").unwrap().optional);
        assert_eq!(
            doc.param("opts").unwrap().ts_type.as_deref(),
            Some("{ trim: boolean; limit?: number }")
        );
        assert_eq!(doc.returns.as_deref(), Some("string[]"));
    }
}
