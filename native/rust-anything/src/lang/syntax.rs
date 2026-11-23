//! Small helpers over tree-sitter syntax trees, shared by the language plugins.

use tree_sitter::{Language, Node, Parser, Tree};

/// Parses `text`; `None` only when the grammar cannot be loaded.
pub fn parse(language: &Language, text: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(language).ok()?;
    parser.parse(text, None)
}

pub fn text<'t>(node: Node, source: &'t str) -> &'t str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

/// 1-based line of a node.
pub fn line(node: Node) -> u32 {
    u32::try_from(node.start_position().row + 1).unwrap_or(u32::MAX)
}

/// Every node in the tree, depth first.
pub fn walk<'t>(root: Node<'t>, mut visit: impl FnMut(Node<'t>)) {
    let mut cursor = root.walk();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        visit(node);
        let children: Vec<Node> = node.children(&mut cursor).collect();
        stack.extend(children.into_iter().rev());
    }
}

pub fn named_children<'t>(node: Node<'t>) -> Vec<Node<'t>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

/// Non-blank lines.
pub fn loc(text: &str) -> usize {
    text.lines().filter(|l| !l.trim().is_empty()).count()
}

/// `text` with its `<!-- -->` comments removed (a `pom.xml`, a project file).
pub fn without_xml_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A build file with its comments removed, so a commented-out `implementation("a:b:1")` is not
/// read as a dependency the project declares. `//` counts only at the start of a line: a trailing
/// one follows code the project does declare.
pub fn without_slash_comments(text: &str) -> String {
    let mut unblocked = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut quote = None;
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                unblocked.push(c);
                if c == '\\' {
                    if let Some(escaped) = chars.next() {
                        unblocked.push(escaped);
                    }
                } else if c == q || c == '\n' {
                    quote = None;
                }
            }
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                unblocked.push(c);
            }
            None if c == '/' && chars.peek() == Some(&'*') => {
                chars.next();
                let mut last = ' ';
                for c in chars.by_ref() {
                    if last == '*' && c == '/' {
                        break;
                    }
                    last = c;
                }
            }
            None => unblocked.push(c),
        }
    }
    unblocked
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}
