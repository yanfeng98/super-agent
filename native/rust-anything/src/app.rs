//! App-level record and replay for CLIs, in any language: the program is a black box.
//!
//! `record` runs the original on cases (the app's own examples, an inputs file, semantic traps)
//! twice, marks what differs between the two runs as noise, and writes each case as trycmd files
//! in `rust/app-cases/`. `replay` runs any build against those files and reports what differs.

use crate::walk::Canonical;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use toml_edit::{DocumentMut, Item, Table, value};

pub const CASES_DIR: &str = "rust/app-cases";
pub const CONFIG_FILE: &str = "rust-anything.json";
pub const REPORT_FILE: &str = "replay-report.md";
pub const FALLBACK_ENV: &str = "RUST_ANYTHING_FALLBACK_LOG";
pub const BUILTIN_NORMALIZERS: [&str; 3] = ["timestamps", "uuids", "key-order"];
pub(crate) const SCHEMA: &str = "rust-anything/app-cases@1";
pub(crate) const MAX_TRAPS: usize = 300;
pub(crate) const FRESH_DIR: &str = "files";
const REMOVED_EXT: &str = "removed";
const MAX_INPUT_FILE: u64 = 1 << 20;
const TIMESTAMP: &str = r"\b(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun) (?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) \d{2} \d{4} \d{2}:\d{2}:\d{2} GMT[+-]\d{4}(?: \([^)\n]*\))?|\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}(?::\d{2}(?:[.,]\d+)?)?(?:Z|[+-]\d{2}:?\d{2})?|\b\d{2}:\d{2}:\d{2}(?:[.,]\d+)?\b";
const UUID: &str =
    r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b";

pub(crate) const INT_TRAPS: [&str; 8] = [
    "-7",
    "0",
    "-1",
    "2147483648",
    "9007199254740993",
    "9223372036854775807",
    "-9223372036854775808",
    "18446744073709551616",
];
pub(crate) const FLOAT_TRAPS: [&str; 7] = [
    "-0.0",
    "0.1",
    "0.30000000000000004",
    "1e21",
    "1e-7",
    "5e-324",
    "1.7976931348623157e308",
];
pub(crate) const STRING_TRAPS: [&str; 10] = [
    "héllo wörld",
    "👍🏽 thumbs",
    "日本語",
    "",
    "e\u{301}",
    "İstanbul",
    "straße",
    "ılık ırmak",
    "👨\u{200D}👩\u{200D}👧 family",
    "a\u{00A0}b",
];
pub(crate) const STDIN_TRAPS: [&str; 4] = [
    "naïve café 👍🏽\nsecond line\n",
    "",
    "日本語 テキスト\r\nline two\r\n",
    "  spaced   out  \n\n",
];
pub(crate) const JSON_TRAPS: [&str; 2] = [
    r#"{"zebra":1,"apple":2,"10":3,"2":4,"é":5,"Apple":6}"#,
    r#"{"n":9007199254740993,"f":1e21,"t":0.1,"neg":-7}"#,
];
const NOT_APP_NAMES: [&str; 29] = [
    "node", "nodejs", "python", "python3", "py", "ruby", "java", "go", "deno", "bun", "tsx",
    "ts-node", "sh", "bash", "zsh", "cargo", "npx", "npm", "pnpm", "yarn", "uv", "poetry",
    "bundle", "run", "exec", "env", "sudo", "php", "dotnet",
];

/// `rust/app-cases/rust-anything.json`: how to run the original, and the normalizers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub schema: String,
    pub name: String,
    pub run: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub normalize: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub normalizers: BTreeMap<String, String>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Set when the app is a server: cases are requests, in `http/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<crate::http::HttpConfig>,
    /// Numbers in the output match within this relative tolerance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub float_tolerance: Option<f64>,
    /// Written files holding raw floats, by path: compared value by value (`f32le`, `f64le`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub float_files: BTreeMap<String, String>,
    /// Outputs whose line order the original's own runs disagreed on, by case: their lines must
    /// match in any order.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub unordered: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<crate::effects::EffectsConfig>,
    /// CLIs: repository paths copied (from the snapshot in `files/`) into the working directory
    /// before every case; a path ending in `/` gives its contents.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fresh: Vec<String>,
}

pub(crate) fn default_timeout() -> u64 {
    30
}

impl Config {
    pub fn load(repo: &Path) -> Result<Option<Config>> {
        let path = cases_dir(repo).join(CONFIG_FILE);
        if !path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        Ok(Some(
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?,
        ))
    }
}

pub fn cases_dir(repo: &Path) -> PathBuf {
    repo.join(CASES_DIR)
}

/// One line of an `--inputs` file.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub stdin: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    #[serde(default)]
    pub remove: Vec<String>,
}

#[derive(Debug, Clone)]
struct Case {
    args: Vec<String>,
    stdin: Option<Vec<u8>>,
    env: BTreeMap<String, String>,
    env_remove: Vec<String>,
    inherit_env: bool,
    files: BTreeMap<PathBuf, Vec<u8>>,
    remove: Vec<PathBuf>,
}

impl Case {
    fn new(args: Vec<String>) -> Self {
        Case {
            args,
            stdin: None,
            env: BTreeMap::new(),
            env_remove: Vec::new(),
            inherit_env: true,
            files: BTreeMap::new(),
            remove: Vec::new(),
        }
    }

    fn key(&self) -> String {
        serde_json::json!([self.args, self.stdin, self.env, self.files, self.remove]).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
    Code(i32),
    Killed,
    Timeout,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Code(c) => write!(f, "exit code {c}"),
            Status::Killed => write!(f, "killed by a signal"),
            Status::Timeout => write!(f, "timed out"),
        }
    }
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Status,
    files: BTreeMap<PathBuf, Vec<u8>>,
    removed: Vec<PathBuf>,
    fallback: Option<bool>,
    effects: Option<Vec<u8>>,
}

impl Run {
    fn written(&self, case: &Case) -> BTreeMap<PathBuf, Vec<u8>> {
        self.files
            .iter()
            .filter(|(path, bytes)| case.files.get(*path) != Some(*bytes))
            .map(|(p, b)| (p.clone(), b.clone()))
            .collect()
    }
}

pub(crate) struct Normalizers {
    patterns: Vec<Regex>,
    key_order: bool,
    float_tolerance: Option<f64>,
    float_files: BTreeMap<PathBuf, usize>,
    unordered: BTreeMap<String, Vec<String>>,
}

impl Normalizers {
    fn new(config: &Config) -> Result<Self> {
        let mut sources: Vec<(&str, &str)> = Vec::new();
        let mut key_order = false;
        for name in &config.normalize {
            match name.as_str() {
                "timestamps" => sources.push((name, TIMESTAMP)),
                "uuids" => sources.push((name, UUID)),
                "key-order" => key_order = true,
                other => bail!(
                    "unknown normalizer `{other}`: use {} or define it as `name=regex`",
                    BUILTIN_NORMALIZERS.join(", ")
                ),
            }
        }
        sources.extend(
            config
                .normalizers
                .iter()
                .map(|(n, p)| (n.as_str(), p.as_str())),
        );
        let patterns = sources
            .iter()
            .map(|(name, pattern)| {
                Regex::new(pattern).with_context(|| format!("normalizer `{name}`: {pattern}"))
            })
            .collect::<Result<Vec<_>>>()?;
        let float_tolerance = config
            .float_tolerance
            .map(crate::tolerance::check)
            .transpose()?;
        let float_files = config
            .float_files
            .iter()
            .map(|(path, format)| Ok((PathBuf::from(path), float_width(format)?)))
            .collect::<Result<_>>()?;
        Ok(Normalizers {
            patterns,
            key_order,
            float_tolerance,
            float_files,
            unordered: config.unordered.clone(),
        })
    }

    /// Whether the original's runs disagreed on the line order of this case's `field`.
    pub(crate) fn unordered(&self, case: &str, field: &str) -> bool {
        self.unordered
            .get(case)
            .is_some_and(|fields| fields.iter().any(|f| f == field))
    }

    pub(crate) fn apply(&self, text: &str) -> String {
        let mut out = text.to_string();
        for p in &self.patterns {
            out = p.replace_all(&out, "[..]").into_owned();
        }
        out
    }

    fn canonical(&self, text: &str) -> String {
        if self.key_order {
            sort_json_text(text)
        } else {
            text.to_string()
        }
    }
}

fn sort_value(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let sorted: BTreeMap<String, Value> =
                map.into_iter().map(|(k, v)| (k, sort_value(v))).collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sort_value).collect()),
        other => other,
    }
}

fn sort_json_text(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with(['{', '['])
        && let Ok(v) = serde_json::from_str::<Value>(trimmed)
    {
        return serde_json::to_string_pretty(&sort_value(v)).unwrap_or_default() + "\n";
    }
    text.split('\n')
        .map(|line| {
            let t = line.trim();
            if t.starts_with(['{', '['])
                && let Ok(v) = serde_json::from_str::<Value>(t)
            {
                return sort_value(v).to_string();
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn temp_dir(tag: &str) -> Result<PathBuf> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "rust-anything-app-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    Ok(dir)
}

pub(crate) fn resolve_word(repo: &Path, word: &str) -> String {
    if word.starts_with('-') {
        return word.to_string();
    }
    let path = repo.join(word);
    let looks_like_path = word.contains('/') || word.contains('\\') || path.is_file();
    if looks_like_path && path.exists() {
        return path
            .canonical()
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
    }
    word.to_string()
}

#[cfg(any(windows, test))]
fn is_python_program(program: &str) -> bool {
    Path::new(program)
        .file_stem()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            ["python", "python3", "py"]
                .iter()
                .any(|candidate| name.eq_ignore_ascii_case(candidate))
        })
}

fn run_case(
    command: &[String],
    repo: &Path,
    case: &Case,
    timeout: Duration,
    effects: Option<&crate::effects::EffectsConfig>,
    base: Option<&Path>,
) -> Result<Run> {
    let sandbox = temp_dir("run")?;
    let result = run_in(command, repo, case, timeout, &sandbox, effects, base);
    let _ = std::fs::remove_dir_all(&sandbox);
    result
}

fn run_in(
    command: &[String],
    repo: &Path,
    case: &Case,
    timeout: Duration,
    sandbox: &Path,
    effects: Option<&crate::effects::EffectsConfig>,
    base: Option<&Path>,
) -> Result<Run> {
    let work = sandbox.join("cwd");
    std::fs::create_dir_all(&work)?;
    if let Some(base) = base {
        crate::http::copy_tree(base, &work)?;
    }
    for path in &case.remove {
        let target = work.join(path);
        if target.is_dir() {
            std::fs::remove_dir_all(&target)?;
        } else if target.exists() {
            std::fs::remove_file(&target)?;
        }
    }
    for (path, bytes) in &case.files {
        let target = work.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, bytes)?;
    }
    let before: BTreeMap<PathBuf, FileStamp> = walk_files(&work)
        .into_iter()
        .filter_map(|path| {
            let stamp = FileStamp::of(&path)?;
            Some((path.strip_prefix(&work).ok()?.to_path_buf(), stamp))
        })
        .collect();
    let log = sandbox.join("fallback.log");
    let (program, rest) = command.split_first().context("empty command")?;
    let mut cmd = Command::new(resolve_word(repo, program));
    cmd.args(rest.iter().map(|w| resolve_word(repo, w)))
        .args(&case.args)
        .current_dir(&work)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !case.inherit_env {
        cmd.env_clear();
    }
    #[cfg(windows)]
    if is_python_program(program)
        && (!case.inherit_env || std::env::var_os("PYTHONIOENCODING").is_none())
    {
        // Python otherwise inherits the legacy Windows console code page even though its stdio
        // is piped. UTF-8 keeps generated Unicode traps recordable; a case can still override it.
        cmd.env("PYTHONIOENCODING", "utf-8");
    }
    for key in &case.env_remove {
        cmd.env_remove(key);
    }
    let sinks = effects
        .map(|config| -> Result<_> {
            let sinks = crate::effects::Sinks::start(config)?;
            sinks.reset(repo)?;
            Ok(sinks)
        })
        .transpose()?;
    if let Some(sinks) = &sinks {
        cmd.envs(sinks.env());
    }
    cmd.envs(&case.env).env(FALLBACK_ENV, &log);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        cmd.process_group(0);
    }
    let mut child = cmd
        .spawn()
        .with_context(|| format!("running `{}`", command.join(" ")))?;
    let stdin = child.stdin.take();
    let input = case.stdin.clone().unwrap_or_default();
    let writer = std::thread::spawn(move || {
        if let Some(mut pipe) = stdin {
            let _ = pipe.write_all(&input);
        }
    });
    let mut out_pipe = child.stdout.take().context("no stdout pipe")?;
    let mut err_pipe = child.stderr.take().context("no stderr pipe")?;
    let out_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = out_pipe.read_to_end(&mut buf);
        buf
    });
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = err_pipe.read_to_end(&mut buf);
        buf
    });
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait()? {
            #[cfg(unix)]
            let pipes_done =
                writer.is_finished() && out_reader.is_finished() && err_reader.is_finished();
            #[cfg(not(unix))]
            let pipes_done = true;
            if pipes_done {
                break s.code().map_or(Status::Killed, Status::Code);
            }
        }
        if start.elapsed() > timeout {
            #[cfg(unix)]
            {
                let _ = Command::new("/bin/kill")
                    .args(["-KILL", "--", &format!("-{}", child.id())])
                    .stderr(Stdio::null())
                    .status();
            }
            let _ = child.kill();
            let _ = child.wait();
            break Status::Timeout;
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let _ = writer.join();
    let stdout = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();

    let mut cwd_forms = vec![work.to_string_lossy().into_owned()];
    if let Ok(real) = work.canonical() {
        cwd_forms.insert(0, real.to_string_lossy().into_owned());
    }
    let redact = |bytes: Vec<u8>| -> Vec<u8> {
        match String::from_utf8(bytes) {
            Ok(mut text) => {
                for form in &cwd_forms {
                    text = text.replace(form.as_str(), "[CWD]");
                }
                text.into_bytes()
            }
            Err(e) => e.into_bytes(),
        }
    };
    let mut files = BTreeMap::new();
    let mut present = BTreeSet::new();
    for path in walk_files(&work) {
        let rel = path.strip_prefix(&work).unwrap_or(&path).to_path_buf();
        present.insert(rel.clone());
        let stamp = before.get(&rel);
        if stamp.is_some() && stamp == FileStamp::of(&path).as_ref() {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap_or_default();
        if stamp.is_some()
            && !case.files.contains_key(&rel)
            && let Some(base) = base
            && std::fs::read(base.join(&rel)).is_ok_and(|b| b == bytes)
        {
            continue;
        }
        files.insert(rel, redact(bytes));
    }
    let removed = before
        .keys()
        .filter(|rel| !present.contains(*rel))
        .cloned()
        .collect();
    let fallback = std::fs::metadata(&log).ok().map(|m| m.len() > 0);
    let effects = sinks.map(|s| redact(s.take(repo)));
    Ok(Run {
        stdout: redact(stdout),
        stderr: redact(stderr),
        status,
        files,
        removed,
        fallback,
        effects,
    })
}

#[derive(PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: Option<std::time::SystemTime>,
}

impl FileStamp {
    fn of(path: &Path) -> Option<Self> {
        let meta = std::fs::metadata(path).ok()?;
        Some(FileStamp {
            len: meta.len(),
            modified: meta.modified().ok(),
        })
    }
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                out.push(entry.path());
            }
        }
    }
    out.sort();
    out
}

fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .clamp(1, 8);
    let next = AtomicUsize::new(0);
    let results: Vec<Mutex<Option<R>>> = items.iter().map(|_| Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    let Some(item) = items.get(i) else { break };
                    let r = f(item);
                    if let Ok(mut slot) = results[i].lock() {
                        *slot = Some(r);
                    }
                }
            });
        }
    });
    results
        .into_iter()
        .filter_map(|m| m.into_inner().ok().flatten())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tok {
    pub(crate) text: String,
    pub(crate) op: bool,
}

pub(crate) fn shell_tokens(line: &str) -> Option<Vec<Tok>> {
    let mut toks = Vec::new();
    let mut cur = String::new();
    let mut in_tok = false;
    let mut chars = line.chars().peekable();
    let flush = |toks: &mut Vec<Tok>, cur: &mut String, in_tok: &mut bool| {
        if *in_tok {
            toks.push(Tok {
                text: std::mem::take(cur),
                op: false,
            });
            *in_tok = false;
        }
    };
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => flush(&mut toks, &mut cur, &mut in_tok),
            '\'' => {
                in_tok = true;
                loop {
                    match chars.next()? {
                        '\'' => break,
                        x => cur.push(x),
                    }
                }
            }
            '"' => {
                in_tok = true;
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => match chars.next()? {
                            x @ ('"' | '\\' | '$' | '`') => cur.push(x),
                            x => {
                                cur.push('\\');
                                cur.push(x);
                            }
                        },
                        '`' => return None,
                        '$' if chars.peek() == Some(&'(') => return None,
                        x => cur.push(x),
                    }
                }
            }
            '\\' => {
                in_tok = true;
                let mut following = chars.clone();
                let next = following.next();
                let after_next = following.next();
                let escaped_substitution = next == Some('$') && after_next == Some('(');
                let unc_prefix = cur.is_empty()
                    && next == Some('\\')
                    && after_next
                        .is_some_and(|c| !c.is_whitespace() && !"'\"\\|<>&;$`".contains(c));
                if cfg!(windows)
                    && (unc_prefix
                        || (!escaped_substitution
                            && !chars
                                .peek()
                                .is_some_and(|c| c.is_whitespace() || "'\"\\|<>&;`".contains(*c))))
                {
                    cur.push('\\');
                } else if let Some(x) = chars.next() {
                    cur.push(x);
                }
            }
            '|' | '<' | '>' | ';' | '&' => {
                flush(&mut toks, &mut cur, &mut in_tok);
                let mut op = c.to_string();
                while let Some(&n) = chars.peek() {
                    if !"|<>&".contains(n) {
                        break;
                    }
                    op.push(n);
                    chars.next();
                }
                toks.push(Tok { text: op, op: true });
            }
            '#' if !in_tok => break,
            '`' => return None,
            '$' if chars.peek() == Some(&'(') => return None,
            _ => {
                in_tok = true;
                cur.push(c);
            }
        }
    }
    flush(&mut toks, &mut cur, &mut in_tok);
    Some(toks)
}

/// Splits a command line into words, shell-style quoting, no shell operators.
pub fn shell_split(line: &str) -> Result<Vec<String>> {
    let toks = shell_tokens(line).with_context(|| format!("cannot split `{line}`"))?;
    if let Some(op) = toks.iter().find(|t| t.op) {
        bail!(
            "`{line}` uses the shell operator `{}`: put the pipeline in a script and give its path",
            op.text
        );
    }
    Ok(toks.into_iter().map(|t| t.text).collect())
}

pub(crate) fn shell_quote(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_alphanumeric() || "-_./=:,+@%".contains(c));
    if plain {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

fn lines(text: &str) -> Vec<&str> {
    let text = text.strip_suffix('\n').unwrap_or(text);
    if text.is_empty() {
        return Vec::new();
    }
    text.split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Equal(usize, usize),
    Delete(usize),
    Insert(usize),
}

fn diff_ops<A, B>(a: &[A], b: &[B], eq: impl Fn(&A, &B) -> bool) -> Vec<Op> {
    let mut head = 0;
    while head < a.len() && head < b.len() && eq(&a[head], &b[head]) {
        head += 1;
    }
    let mut tail = 0;
    while tail < a.len() - head
        && tail < b.len() - head
        && eq(&a[a.len() - 1 - tail], &b[b.len() - 1 - tail])
    {
        tail += 1;
    }
    let (am, bm) = (&a[head..a.len() - tail], &b[head..b.len() - tail]);
    let mut ops: Vec<Op> = (0..head).map(|i| Op::Equal(i, i)).collect();
    let (n, m) = (am.len(), bm.len());
    if n * m <= 4_000_000 {
        let mut lcs = vec![vec![0u32; m + 1]; n + 1];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                lcs[i][j] = if eq(&am[i], &bm[j]) {
                    lcs[i + 1][j + 1] + 1
                } else {
                    lcs[i + 1][j].max(lcs[i][j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n || j < m {
            if i < n && j < m && eq(&am[i], &bm[j]) {
                ops.push(Op::Equal(head + i, head + j));
                i += 1;
                j += 1;
            } else if i < n && (j == m || lcs[i + 1][j] >= lcs[i][j + 1]) {
                ops.push(Op::Delete(head + i));
                i += 1;
            } else {
                ops.push(Op::Insert(head + j));
                j += 1;
            }
        }
    } else {
        ops.extend((0..n).map(|i| Op::Delete(head + i)));
        ops.extend((0..m).map(|j| Op::Insert(head + j)));
    }
    ops.extend((0..tail).map(|k| Op::Equal(a.len() - tail + k, b.len() - tail + k)));
    ops
}

pub(crate) fn line_matches(expected: &str, actual: &str) -> bool {
    if !expected.contains("[..]") {
        return expected == actual;
    }
    let parts: Vec<&str> = expected.split("[..]").collect();
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    let Some(rest) = actual.strip_prefix(first) else {
        return false;
    };
    let Some(mut rest) = rest.strip_suffix(last) else {
        return false;
    };
    for mid in &parts[1..parts.len() - 1] {
        match rest.find(mid) {
            Some(i) => rest = &rest[i + mid.len()..],
            None => return false,
        }
    }
    true
}

/// Whether a line can match a line it is not equal to: `[..]` stands for any text, and with a
/// tolerance a number can meet a nearby number. Any other line matches its own text and nothing
/// else, so comparing the sorted sides already settles those.
fn can_match_beyond_its_text(line: &str, tol: Option<f64>) -> bool {
    line.contains("[..]") || (tol.is_some() && line.bytes().any(|b| b.is_ascii_digit()))
}

/// Expected lines against actual lines in any order: every expected line paired with a distinct
/// actual line. Sorting both sides and pairing by position cannot do this once a line matches
/// more than its own text, because no order agrees with a match that is not transitive -- `10.0`
/// has to meet `9.999999999999999` while sorting before `2.0`. So identical lines are paired off
/// first, and the lines that are left get a search that gives a line up when another line needs
/// it.
fn lines_match_in_any_order(expected: &[&str], actual: &[&str], tol: Option<f64>) -> bool {
    if expected.len() != actual.len() {
        return false;
    }
    let (mut sorted_e, mut sorted_a) = (expected.to_vec(), actual.to_vec());
    sorted_e.sort_unstable();
    sorted_a.sort_unstable();
    if sorted_e == sorted_a {
        return true;
    }
    if !expected.iter().any(|l| can_match_beyond_its_text(l, tol)) {
        return false;
    }
    let mut spare: HashMap<&str, usize> = HashMap::new();
    for line in actual {
        *spare.entry(*line).or_default() += 1;
    }
    let mut rest_expected = Vec::new();
    for line in expected {
        match spare.get_mut(line) {
            Some(count) if *count > 0 => *count -= 1,
            _ => rest_expected.push(*line),
        }
    }
    let rest_actual: Vec<&str> = spare
        .into_iter()
        .flat_map(|(line, count)| std::iter::repeat_n(line, count))
        .collect();
    let mut holder: Vec<Option<usize>> = vec![None; rest_actual.len()];
    for want in 0..rest_expected.len() {
        let mut tried = vec![false; rest_actual.len()];
        if !take_a_line(
            want,
            &rest_expected,
            &rest_actual,
            tol,
            &mut tried,
            &mut holder,
        ) {
            return false;
        }
    }
    true
}

/// One augmenting step: find a line for `want`, moving the expected lines that already hold one
/// along to another line they also match.
fn take_a_line(
    want: usize,
    expected: &[&str],
    actual: &[&str],
    tol: Option<f64>,
    tried: &mut [bool],
    holder: &mut [Option<usize>],
) -> bool {
    for i in 0..actual.len() {
        if tried[i] || !line_matches_within(expected[want], actual[i], tol) {
            continue;
        }
        tried[i] = true;
        if holder[i].is_none_or(|held| take_a_line(held, expected, actual, tol, tried, holder)) {
            holder[i] = Some(want);
            return true;
        }
    }
    false
}

/// A line within the float tolerance, for lines without wildcards.
fn line_matches_within(expected: &str, actual: &str, tol: Option<f64>) -> bool {
    line_matches(expected, actual)
        || tol.is_some_and(|t| {
            !expected.contains("[..]") && crate::tolerance::line_close(expected, actual, t)
        })
}

/// trycmd's matching: `[..]` is any text within a line, a `...` line is any number of lines.
fn text_matches(expected: &str, actual: &str, tol: Option<f64>) -> bool {
    let (e, a) = (lines(expected), lines(actual));
    let m = a.len();
    let mut next = vec![false; m + 1];
    next[m] = true;
    for line in e.iter().rev() {
        let mut cur = vec![false; m + 1];
        for j in (0..=m).rev() {
            cur[j] = if *line == "..." {
                next[j] || (j < m && cur[j + 1])
            } else {
                j < m && line_matches_within(line, a[j], tol) && next[j + 1]
            };
        }
        next = cur;
    }
    next[0]
}

fn render_diff(expected: &str, actual: &str, tol: Option<f64>) -> String {
    let (e, a) = (lines(expected), lines(actual));
    let ops = diff_ops(&e, &a, |x, y| line_matches_within(x, y, tol));
    let changed: Vec<bool> = ops.iter().map(|op| !matches!(op, Op::Equal(..))).collect();
    let near = |k: usize| {
        let lo = k.saturating_sub(3);
        let hi = (k + 4).min(ops.len());
        changed[lo..hi].iter().any(|c| *c)
    };
    let mut out = String::new();
    let mut skipped = false;
    for (k, op) in ops.iter().enumerate() {
        match op {
            Op::Equal(_, j) if near(k) => {
                skipped = false;
                out += &format!("  {}\n", a[*j]);
            }
            Op::Equal(..) => {
                if !skipped {
                    out += "  ⋮\n";
                    skipped = true;
                }
            }
            Op::Delete(i) => {
                skipped = false;
                out += &format!("- {}\n", e[*i]);
            }
            Op::Insert(j) => {
                skipped = false;
                out += &format!("+ {}\n", a[*j]);
            }
        }
    }
    let trail = |s: &str| s.ends_with('\n') || s.is_empty();
    if out.is_empty() || trail(expected) != trail(actual) {
        out += &format!(
            "(trailing newline: expected {}, got {})\n",
            trail(expected),
            trail(actual)
        );
    }
    out
}

fn tokens(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in line.char_indices() {
        if c.is_alphanumeric() {
            start.get_or_insert(i);
        } else {
            if let Some(s) = start.take() {
                out.push(&line[s..i]);
            }
            out.push(&line[i..i + c.len_utf8()]);
        }
    }
    if let Some(s) = start {
        out.push(&line[s..]);
    }
    out
}

pub(crate) fn merge_line(a: &str, b: &str) -> String {
    let (ta, tb) = (tokens(a), tokens(b));
    if ta.len() != tb.len() {
        return "[..]".to_string();
    }
    let mut out = String::new();
    for (x, y) in ta.iter().zip(&tb) {
        if x == y {
            out += x;
        } else if !out.ends_with("[..]") {
            out += "[..]";
        }
    }
    out
}

/// Two runs of the original: what they agree on stays, what differs becomes a wildcard.
fn merge_noise(a: &str, b: &str) -> String {
    if a == b {
        return a.to_string();
    }
    let (la, lb): (Vec<&str>, Vec<&str>) = (a.split('\n').collect(), b.split('\n').collect());
    let ops = diff_ops(&la, &lb, |x, y| x == y);
    let mut out: Vec<String> = Vec::new();
    let (mut dels, mut ins): (Vec<&str>, Vec<&str>) = (Vec::new(), Vec::new());
    let flush = |out: &mut Vec<String>, dels: &mut Vec<&str>, ins: &mut Vec<&str>| {
        if dels.is_empty() && ins.is_empty() {
            return;
        }
        if dels.len() == ins.len() {
            for (x, y) in dels.iter().zip(ins.iter()) {
                out.push(merge_line(x, y));
            }
        } else if out.last().map(String::as_str) != Some("...") {
            out.push("...".to_string());
        }
        dels.clear();
        ins.clear();
    };
    for op in ops {
        match op {
            Op::Equal(i, _) => {
                flush(&mut out, &mut dels, &mut ins);
                out.push(la[i].to_string());
            }
            Op::Delete(i) => dels.push(la[i]),
            Op::Insert(j) => ins.push(lb[j]),
        }
    }
    flush(&mut out, &mut dels, &mut ins);
    out.join("\n")
}

fn stem(word: &str) -> String {
    Path::new(word)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Names the app is invoked by, most specific first.
fn app_names(repo: &Path, run: &[String], explicit: Option<&str>) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut add = |name: &str| {
        let name = name.rsplit('/').next().unwrap_or(name).to_lowercase();
        if !name.is_empty() && !NOT_APP_NAMES.contains(&name.as_str()) && !names.contains(&name) {
            names.push(name);
        }
    };
    if let Some(name) = explicit {
        add(name);
    }
    for word in run {
        let path = repo.join(word);
        if word.starts_with('-') || path.is_dir() || Path::new(word).is_dir() {
            continue;
        }
        add(&stem(word));
    }
    if let Ok(text) = std::fs::read_to_string(repo.join("package.json"))
        && let Ok(pkg) = serde_json::from_str::<Value>(&text)
    {
        match &pkg["bin"] {
            Value::String(_) => add(pkg["name"].as_str().unwrap_or_default()),
            Value::Object(map) => map.keys().for_each(|k| add(k)),
            _ => {}
        }
    }
    if let Ok(text) = std::fs::read_to_string(repo.join("pyproject.toml"))
        && let Ok(doc) = text.parse::<DocumentMut>()
    {
        for table in [
            doc.get("project").and_then(|p| p.get("scripts")),
            doc.get("tool")
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.get("scripts")),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(t) = table.as_table_like() {
                t.iter().for_each(|(k, _)| add(k));
            }
        }
    }
    for (dir, dirs_only) in [("cmd", true), ("bin", false), ("exe", false)] {
        let Ok(entries) = std::fs::read_dir(repo.join(dir)) else {
            continue;
        };
        let mut found: Vec<String> = entries
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir() == dirs_only))
            .map(|e| stem(&e.file_name().to_string_lossy()))
            .collect();
        found.sort();
        found.iter().for_each(|n| add(n));
    }
    names
}

fn safe_relative(arg: &str) -> bool {
    let p = Path::new(arg);
    !arg.is_empty()
        && !arg.starts_with('-')
        && p.components().all(|c| matches!(c, Component::Normal(_)))
}

fn read_input_file(candidates: &[PathBuf]) -> Option<Vec<u8>> {
    candidates
        .iter()
        .find(|p| p.is_file() && std::fs::metadata(p).is_ok_and(|m| m.len() <= MAX_INPUT_FILE))
        .and_then(|p| std::fs::read(p).ok())
}

fn parse_example(line: &str, names: &[String], repo: &Path, doc_dir: &Path) -> Option<Case> {
    let toks = shell_tokens(line)?;
    let ops: Vec<usize> = (0..toks.len()).filter(|&i| toks[i].op).collect();
    let words = |range: &[Tok]| -> Vec<String> { range.iter().map(|t| t.text.clone()).collect() };
    let lookup = |arg: &str| -> Option<Vec<u8>> {
        if !safe_relative(arg) {
            return None;
        }
        read_input_file(&[repo.join(arg), doc_dir.join(arg)])
    };
    let (main, stdin): (Vec<String>, Option<Vec<u8>>) = match ops.as_slice() {
        [] => (words(&toks), None),
        [i] if toks[*i].text == "|" => {
            let feed = words(&toks[..*i]);
            let stdin = match feed.first().map(String::as_str) {
                Some("echo") => {
                    let newline = feed.get(1).map(String::as_str) != Some("-n");
                    let text: Vec<&str> = feed[1..]
                        .iter()
                        .map(String::as_str)
                        .filter(|w| !matches!(*w, "-n" | "-e"))
                        .collect();
                    let mut text = text.join(" ");
                    if newline {
                        text.push('\n');
                    }
                    text.into_bytes()
                }
                Some("printf") if feed.len() == 2 => feed[1]
                    .replace("\\n", "\n")
                    .replace("\\t", "\t")
                    .into_bytes(),
                Some("cat") if feed.len() == 2 => lookup(&feed[1])?,
                _ => return None,
            };
            (words(&toks[*i + 1..]), Some(stdin))
        }
        [i] if toks[*i].text == "<" && *i + 2 == toks.len() => {
            (words(&toks[..*i]), Some(lookup(&toks[*i + 1].text)?))
        }
        _ => return None,
    };
    let mut env = BTreeMap::new();
    let mut rest = main.as_slice();
    while let Some((first, tail)) = rest.split_first() {
        let Some((k, v)) = first.split_once('=') else {
            break;
        };
        if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            break;
        }
        env.insert(k.to_string(), v.to_string());
        rest = tail;
    }
    let (prog, args) = rest.split_first()?;
    if !names.contains(&stem(prog)) {
        return None;
    }
    let mut case = Case::new(args.to_vec());
    case.env = env;
    case.stdin = stdin;
    for arg in args {
        if let Some(bytes) = lookup(arg) {
            case.files.insert(PathBuf::from(arg), bytes);
        }
    }
    Some(case)
}

fn find_examples(repo: &Path, names: &[String]) -> Vec<Case> {
    if names.is_empty() {
        return Vec::new();
    }
    scan_examples(repo, |line, doc_dir| {
        parse_example(line, names, repo, doc_dir)
    })
}

/// Shell lines in the repository's Markdown code blocks and `.sh` files, each given to `parse`
/// with the folder of the file it came from.
pub(crate) fn scan_examples<T>(repo: &Path, parse: impl Fn(&str, &Path) -> Option<T>) -> Vec<T> {
    let skip = ["node_modules", "target", "vendor", ".git"];
    let walker = crate::walk::filtered(repo, move |e| {
        let name = e.file_name().to_string_lossy();
        !skip.contains(&name.as_ref())
    })
    .max_depth(Some(4))
    .sort_by_file_name(|a, b| a.cmp(b))
    .build();
    let rust_dir = repo.join("rust");
    let mut out = Vec::new();
    for entry in walker.flatten() {
        let path = entry.path();
        if path.starts_with(&rust_dir) || !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let markdown = matches!(ext.as_str(), "md" | "markdown");
        if !markdown && ext != "sh" {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let doc_dir = path.parent().unwrap_or(repo);
        let mut fence: Option<String> = None;
        for raw in text.lines() {
            let line = raw.trim();
            if markdown {
                if line.starts_with("```") || line.starts_with("~~~") {
                    fence = match fence {
                        Some(_) => None,
                        None => Some(
                            line.trim_start_matches(['`', '~'])
                                .split_whitespace()
                                .next()
                                .unwrap_or_default()
                                .to_lowercase(),
                        ),
                    };
                    continue;
                }
                let Some(lang) = &fence else { continue };
                let candidate = match line.strip_prefix("$ ") {
                    Some(rest) => rest,
                    None if matches!(lang.as_str(), "sh" | "bash" | "shell" | "zsh") => line,
                    None => continue,
                };
                if let Some(case) = parse(candidate, doc_dir) {
                    out.push(case);
                }
            } else {
                let candidate = line.strip_prefix("$ ").unwrap_or(line);
                if let Some(case) = parse(candidate, doc_dir) {
                    out.push(case);
                }
            }
        }
    }
    out
}

fn read_inputs(path: &Path) -> Result<Vec<(Option<String>, Case)>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let input: Input = serde_json::from_str(line)
            .with_context(|| format!("{}:{}: expected {{\"args\": [...], \"stdin\": ..., \"env\": {{...}}, \"files\": {{path: text}}}}", path.display(), n + 1))?;
        let mut case = Case::new(input.args);
        case.stdin = input.stdin.map(String::into_bytes);
        case.env = input.env;
        for (file, content) in input.files {
            ensure!(
                safe_relative(&file),
                "{}:{}: file `{file}` must be a relative path inside the sandbox",
                path.display(),
                n + 1
            );
            case.files.insert(PathBuf::from(file), content.into_bytes());
        }
        for file in input.remove {
            ensure!(
                safe_relative(&file),
                "{}:{}: removed `{file}` must be a relative path inside the sandbox",
                path.display(),
                n + 1
            );
            case.remove.push(PathBuf::from(file));
        }
        out.push((input.name, case));
    }
    Ok(out)
}

pub(crate) fn is_word(arg: &str) -> bool {
    let mut chars = arg.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn trap_cases(bases: &[Case]) -> Vec<Case> {
    let mut out = Vec::new();
    for base in bases {
        for (i, arg) in base.args.iter().enumerate() {
            if i == 0 && is_word(arg) {
                continue;
            }
            let number = arg.parse::<f64>().is_ok();
            if arg.starts_with('-') && !number {
                continue;
            }
            let values: Vec<&str> = if arg.parse::<i128>().is_ok() {
                INT_TRAPS.to_vec()
            } else if number && arg.contains(['.', 'e', 'E']) {
                FLOAT_TRAPS.to_vec()
            } else if base.files.contains_key(Path::new(arg)) {
                vec!["missing-input.txt"]
            } else {
                STRING_TRAPS.to_vec()
            };
            for v in values.into_iter().filter(|v| v != arg) {
                let mut case = base.clone();
                case.args[i] = v.to_string();
                out.push(case);
            }
        }
        if let Some(stdin) = &base.stdin {
            let json = std::str::from_utf8(stdin)
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(t).ok())
                .is_some_and(|v| v.is_object() || v.is_array());
            let values: &[&str] = if json { &JSON_TRAPS } else { &STDIN_TRAPS };
            for v in values {
                let mut case = base.clone();
                case.stdin = Some(v.as_bytes().to_vec());
                out.push(case);
            }
        }
    }
    out.truncate(MAX_TRAPS);
    out
}

pub(crate) fn slug(text: &str) -> String {
    slug_max(text, 24)
}

/// A case name the user gave in `--inputs`: longer than a generated one, so names that differ
/// only past the 24th character stay apart.
pub(crate) fn given_slug(text: &str) -> String {
    slug_max(text, 96)
}

fn slug_max(text: &str, max: usize) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.truncate(max);
    out.trim_end_matches('-').to_string()
}

pub struct RecordOptions<'a> {
    pub repo: &'a Path,
    /// The command that runs the original; taken from the saved config when empty.
    pub run: &'a [String],
    pub inputs: Option<&'a Path>,
    /// Built-in normalizer names, or `name=regex` for your own.
    pub normalize: &'a [String],
    pub name: Option<&'a str>,
    pub traps: bool,
    pub timeout_secs: Option<u64>,
    /// The app is a server: record requests instead of command lines.
    pub http: bool,
    /// The port a server always listens on (implies `http`); unset, it gets a free one through
    /// `PORT` and `{port}`.
    pub port: Option<u16>,
    /// Servers: arguments every start gets after its command, the original's and every build's
    /// (implies `http`); kept from the last recording when empty.
    pub server_args: &'a [String],
    /// Servers: repository files or folders copied into a fresh working directory before every
    /// start (implies `http`); kept from the last recording when empty.
    pub fresh: &'a [String],
    /// Servers: milliseconds to wait after each response, for work the server does after
    /// answering (a file watcher, a write-behind); kept from the last recording when `None`.
    pub pace_ms: Option<u64>,
    /// Numbers in the output match within this tolerance; kept from the last recording when
    /// `None`, removed with 0.
    pub float_tolerance: Option<f64>,
    /// Written files of raw floats, `path=f32le` or `path=f64le`; kept from the last recording
    /// when empty.
    pub float_files: &'a [String],
    pub effects: Option<crate::effects::EffectsConfig>,
}

#[derive(Debug, Serialize)]
pub struct RecordOutcome {
    pub dir: PathBuf,
    pub run: Vec<String>,
    pub cases: usize,
    pub examples: usize,
    pub inputs: usize,
    pub traps: usize,
    /// Cases where the two runs of the original differed.
    pub noisy: Vec<String>,
    /// What differed in each noisy case, and how: `values` became wildcards, `order` is compared
    /// in any order, `lines` became a `...` that matches anything.
    pub noise: Vec<Noise>,
    /// Cases whose exit code or written files differed between the two runs.
    pub unstable: Vec<String>,
    /// Cases not written: the original timed out or did not answer.
    pub timed_out: Vec<String>,
}

struct Recorded {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    effects: Option<Vec<u8>>,
    status: Status,
    written: BTreeMap<PathBuf, Vec<u8>>,
    removed: Vec<PathBuf>,
    binary: bool,
    noise: Vec<(String, NoiseKind)>,
    unstable: bool,
}

/// How two runs of the original disagreed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NoiseKind {
    /// Values within lines changed (times, ids): those parts are wildcards now.
    Values,
    /// The same lines came out in another order (hash maps, concurrency): replay compares them
    /// in any order, and a port must produce exactly those lines.
    Order,
    /// A different number of lines came out: that stretch is a `...` wildcard, which matches
    /// anything. Look at these first.
    Lines,
}

#[derive(Debug, Clone, Serialize)]
pub struct Noise {
    pub case: String,
    pub field: String,
    pub kind: NoiseKind,
}

fn same_lines_any_order(a: &str, b: &str) -> bool {
    let (mut la, mut lb) = (lines(a), lines(b));
    la.sort_unstable();
    lb.sort_unstable();
    la == lb
}

pub(crate) fn settle_text(
    norm: &Normalizers,
    a: &[u8],
    b: &[u8],
) -> (Vec<u8>, Option<NoiseKind>, bool) {
    match (std::str::from_utf8(a), std::str::from_utf8(b)) {
        (Ok(a), Ok(b)) => {
            let (a, b) = (norm.apply(a), norm.apply(b));
            if a == b || (norm.key_order && norm.canonical(&a) == norm.canonical(&b)) {
                (a.into_bytes(), None, false)
            } else if same_lines_any_order(&a, &b) {
                (a.into_bytes(), Some(NoiseKind::Order), false)
            } else {
                let merged = merge_noise(&a, &b);
                let kind = if merged.split('\n').any(|l| l == "...") {
                    NoiseKind::Lines
                } else {
                    NoiseKind::Values
                };
                (merged.into_bytes(), Some(kind), false)
            }
        }
        _ => (a.to_vec(), (a != b).then_some(NoiseKind::Values), true),
    }
}

fn settle(norm: &Normalizers, case: &Case, one: &Run, two: &Run) -> Recorded {
    let binary = [&one.stdout, &one.stderr, &two.stdout, &two.stderr]
        .iter()
        .any(|bytes| std::str::from_utf8(bytes).is_err());
    let settle_stream = |a: &[u8], b: &[u8]| {
        if binary {
            (a.to_vec(), (a != b).then_some(NoiseKind::Values))
        } else {
            let (bytes, noise, _) = settle_text(norm, a, b);
            (bytes, noise)
        }
    };
    let (stdout, out_noise) = settle_stream(&one.stdout, &two.stdout);
    let (stderr, err_noise) = settle_stream(&one.stderr, &two.stderr);
    let (effects, fx_noise) = match (&one.effects, &two.effects) {
        (Some(a), Some(b)) => {
            let (bytes, noise, _) = settle_text(norm, a, b);
            (Some(bytes), noise)
        }
        _ => (None, None),
    };
    let (w1, w2) = (one.written(case), two.written(case));
    let mut unstable = one.status != two.status
        || one.removed != two.removed
        || !w1.keys().eq(w2.keys())
        || (binary && (out_noise.is_some() || err_noise.is_some()));
    let mut noise: Vec<(String, NoiseKind)> = [
        ("stdout", out_noise),
        ("stderr", err_noise),
        (crate::effects::EXT, fx_noise),
    ]
    .into_iter()
    .filter_map(|(f, k)| Some((f.to_string(), k?)))
    .collect();
    let mut written = BTreeMap::new();
    for (path, a) in &w1 {
        let b = w2.get(path).unwrap_or(a);
        let (bytes, file_noise, file_bin) = settle_text(norm, a, b);
        unstable |= file_bin && file_noise.is_some();
        if let Some(kind) = file_noise {
            noise.push((format!("file {}", path.display()), kind));
        }
        written.insert(path.clone(), bytes);
    }
    Recorded {
        stdout,
        stderr,
        effects,
        status: one.status.clone(),
        written,
        removed: one.removed.clone(),
        binary,
        noise,
        unstable,
    }
}

fn dotted() -> Table {
    let mut t = Table::new();
    t.set_dotted(true);
    t
}

fn write_case(dir: &Path, bin: &str, name: &str, case: &Case, rec: &Recorded) -> Result<()> {
    for ext in [
        "stdin",
        "stdout",
        "stderr",
        REMOVED_EXT,
        crate::effects::EXT,
    ] {
        let _ = std::fs::remove_file(dir.join(format!("{name}.{ext}")));
    }
    for ext in ["in", "out"] {
        let _ = std::fs::remove_dir_all(dir.join(format!("{name}.{ext}")));
    }
    let mut doc = DocumentMut::new();
    let mut bin_t = dotted();
    bin_t["name"] = value(bin);
    doc["bin"] = Item::Table(bin_t);
    let mut args = toml_edit::Array::new();
    for a in &case.args {
        args.push(a.as_str());
    }
    doc["args"] = value(args);
    if !case.env.is_empty() {
        let mut add = dotted();
        for (k, v) in &case.env {
            add[k.as_str()] = value(v.as_str());
        }
        let mut env = dotted();
        env.insert("add", Item::Table(add));
        doc["env"] = Item::Table(env);
    }
    match rec.status {
        Status::Code(code) => {
            let mut status = dotted();
            status["code"] = value(i64::from(code));
            doc["status"] = Item::Table(status);
        }
        _ => doc["status"] = value("interrupted"),
    }
    let mut fs = dotted();
    fs["sandbox"] = value(true);
    if !case.files.is_empty() {
        fs["base"] = value(format!("{name}.in"));
    }
    if !case.remove.is_empty() {
        let mut remove = toml_edit::Array::new();
        for path in &case.remove {
            remove.push(path.to_string_lossy().as_ref());
        }
        fs["remove"] = value(remove);
    }
    doc["fs"] = Item::Table(fs);
    if rec.binary {
        doc["binary"] = value(true);
    }
    std::fs::write(dir.join(format!("{name}.toml")), doc.to_string())?;
    if let Some(stdin) = &case.stdin {
        std::fs::write(dir.join(format!("{name}.stdin")), stdin)?;
    }
    std::fs::write(dir.join(format!("{name}.stdout")), &rec.stdout)?;
    std::fs::write(dir.join(format!("{name}.stderr")), &rec.stderr)?;
    if !rec.removed.is_empty() {
        let list: String = rec
            .removed
            .iter()
            .map(|p| format!("{}\n", p.display()))
            .collect();
        std::fs::write(dir.join(format!("{name}.{REMOVED_EXT}")), list)?;
    }
    if let Some(effects) = &rec.effects {
        std::fs::write(dir.join(format!("{name}.{}", crate::effects::EXT)), effects)?;
    }
    for (sub, files) in [("in", &case.files), ("out", &rec.written)] {
        for (path, bytes) in files {
            let target = dir.join(format!("{name}.{sub}")).join(path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, bytes)?;
        }
    }
    Ok(())
}

pub(crate) fn reserve_case_name(
    taken: &mut BTreeSet<String>,
    base: String,
    mut suffix: usize,
) -> String {
    let mut name = base.clone();
    while !taken.insert(name.clone()) {
        name = format!("{base}-{suffix}");
        suffix += 1;
    }
    name
}

pub fn record(opts: &RecordOptions) -> Result<RecordOutcome> {
    let repo = opts
        .repo
        .canonical()
        .with_context(|| format!("no folder {}", opts.repo.display()))?;
    let existing = Config::load(&repo)?;
    let run = if !opts.run.is_empty() {
        opts.run.to_vec()
    } else if let Some(c) = &existing {
        c.run.clone()
    } else {
        bail!(
            "give the command that runs the original after `--`, e.g. `rust-anything app record . -- python3 cli.py`"
        );
    };
    let mut normalize = existing
        .as_ref()
        .map(|c| c.normalize.clone())
        .unwrap_or_default();
    let mut normalizers = existing
        .as_ref()
        .map(|c| c.normalizers.clone())
        .unwrap_or_default();
    for n in opts.normalize {
        if let Some((name, pattern)) = n.split_once('=') {
            normalizers.insert(name.to_string(), pattern.to_string());
        } else if !normalize.contains(n) {
            normalize.push(n.clone());
        }
    }
    let explicit = opts
        .name
        .map(str::to_string)
        .or_else(|| existing.as_ref().map(|c| c.name.clone()));
    let names = app_names(&repo, &run, explicit.as_deref());
    let mut config = Config {
        schema: SCHEMA.to_string(),
        name: explicit
            .or_else(|| names.first().cloned())
            .unwrap_or_else(|| "app".to_string()),
        run,
        normalize,
        normalizers,
        timeout_secs: opts
            .timeout_secs
            .or(existing.as_ref().map(|c| c.timeout_secs))
            .unwrap_or_else(default_timeout),
        http: crate::http::merge_config(existing.as_ref().and_then(|c| c.http.clone()), opts),
        float_tolerance: match opts.float_tolerance {
            Some(t) => (crate::tolerance::check(t)? > 0.0).then_some(t),
            None => existing.as_ref().and_then(|c| c.float_tolerance),
        },
        float_files: if opts.float_files.is_empty() {
            existing
                .as_ref()
                .map(|c| c.float_files.clone())
                .unwrap_or_default()
        } else {
            opts.float_files
                .iter()
                .map(|spec| {
                    let (path, format) = spec.rsplit_once('=').with_context(|| {
                        format!("--float-file `{spec}`: use PATH=f32le or PATH=f64le")
                    })?;
                    float_width(format)?;
                    Ok((path.to_string(), format.to_string()))
                })
                .collect::<Result<_>>()?
        },
        unordered: BTreeMap::new(),
        effects: crate::effects::EffectsConfig::merge(
            existing.as_ref().and_then(|c| c.effects.clone()),
            opts.effects.clone(),
        ),
        fresh: Vec::new(),
    };
    if let Some(http) = config.http.as_mut() {
        if !opts.fresh.is_empty() {
            http.fresh = opts.fresh.to_vec();
        }
    } else if !opts.fresh.is_empty() {
        config.fresh = opts.fresh.to_vec();
    } else if let Some(c) = &existing {
        config.fresh = c.fresh.clone();
    }
    let norm = Normalizers::new(&config)?;
    if config.http.is_some() {
        let outcome = crate::http::record(&repo, &config, &norm, opts)?;
        config.unordered = unordered_of(&outcome.noise);
        std::fs::write(
            cases_dir(&repo).join(CONFIG_FILE),
            serde_json::to_string_pretty(&config)? + "\n",
        )?;
        return Ok(outcome);
    }

    let examples = find_examples(&repo, &names);
    let inputs = match opts.inputs {
        Some(path) => read_inputs(path)?,
        None => Vec::new(),
    };
    let mut planned: Vec<(String, Case)> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut taken = BTreeSet::new();
    let mut push = |prefix: &str, n: usize, given: Option<String>, case: Case| {
        if !seen.insert(case.key()) {
            return false;
        }
        let base_name = match given {
            Some(g) if !given_slug(&g).is_empty() => given_slug(&g),
            _ => match case.args.first().map(|a| slug(a)).filter(|s| !s.is_empty()) {
                Some(s) => format!("{prefix}-{n:03}-{s}"),
                None => format!("{prefix}-{n:03}"),
            },
        };
        let name = reserve_case_name(&mut taken, base_name, n);
        planned.push((name, case));
        true
    };
    let mut bases = Vec::new();
    let mut counts = [0usize; 3];
    for case in examples {
        if push("example", counts[0] + 1, None, case.clone()) {
            counts[0] += 1;
            bases.push(case);
        }
    }
    for (given, case) in inputs {
        if push("input", counts[1] + 1, given, case.clone()) {
            counts[1] += 1;
            bases.push(case);
        }
    }
    if bases.is_empty() {
        let case = Case::new(Vec::new());
        push("example", 1, Some("no-args".to_string()), case.clone());
        bases.push(case);
    }
    if opts.traps {
        for case in trap_cases(&bases) {
            if push("trap", counts[2] + 1, None, case) {
                counts[2] += 1;
            }
        }
    }

    let timeout = Duration::from_secs(config.timeout_secs);
    let effects = config.effects.as_ref();
    let base = crate::http::snapshot_to(&repo, &config.fresh, &cases_dir(&repo).join(FRESH_DIR))?;
    let base = base.as_deref();
    let run_twice = |(_, case): &(String, Case)| -> Result<(Run, Run)> {
        let one = run_case(&config.run, &repo, case, timeout, effects, base)?;
        let two = run_case(&config.run, &repo, case, timeout, effects, base)?;
        Ok((one, two))
    };
    let runs = if effects.is_some_and(crate::effects::EffectsConfig::serial) {
        planned.iter().map(run_twice).collect()
    } else {
        par_map(&planned, run_twice)
    };
    let dir = cases_dir(&repo);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let mut outcome = RecordOutcome {
        dir: dir.clone(),
        run: config.run.clone(),
        cases: 0,
        examples: counts[0],
        inputs: counts[1],
        traps: counts[2],
        noisy: Vec::new(),
        noise: Vec::new(),
        unstable: Vec::new(),
        timed_out: Vec::new(),
    };
    for ((name, case), runs) in planned.iter().zip(runs) {
        let (one, two) = runs?;
        if one.status == Status::Timeout || two.status == Status::Timeout {
            outcome.timed_out.push(name.clone());
            continue;
        }
        let rec = settle(&norm, case, &one, &two);
        write_case(&dir, &config.name, name, case, &rec)?;
        outcome.cases += 1;
        if !rec.noise.is_empty() {
            outcome.noisy.push(name.clone());
        }
        outcome
            .noise
            .extend(rec.noise.iter().map(|(field, kind)| Noise {
                case: name.clone(),
                field: field.clone(),
                kind: *kind,
            }));
        if rec.unstable {
            outcome.unstable.push(name.clone());
        }
    }
    config.unordered = unordered_of(&outcome.noise);
    std::fs::write(
        dir.join(CONFIG_FILE),
        serde_json::to_string_pretty(&config)? + "\n",
    )?;
    Ok(outcome)
}

fn unordered_of(noise: &[Noise]) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for n in noise.iter().filter(|n| n.kind == NoiseKind::Order) {
        out.entry(n.case.clone()).or_default().push(n.field.clone());
    }
    out
}

enum Expect {
    Code(i32),
    Failed,
    Killed,
    Skip,
}

struct Spec {
    name: String,
    case: Case,
    stdout: Option<Vec<u8>>,
    stderr: Option<Vec<u8>>,
    effects: Option<Vec<u8>>,
    status: Expect,
    out_dir: Option<PathBuf>,
    removed: Option<Vec<PathBuf>>,
    binary: bool,
}

fn read_files(base: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    walk_files(base)
        .into_iter()
        .map(|p| {
            let rel = p.strip_prefix(base).unwrap_or(&p).to_path_buf();
            let bytes = std::fs::read(&p).unwrap_or_default();
            (rel, bytes)
        })
        .collect()
}

fn load_spec(dir: &Path, path: &Path) -> Result<Spec> {
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let doc: DocumentMut = text
        .parse()
        .with_context(|| format!("parsing {}", path.display()))?;
    let args = match doc.get("args") {
        Some(item) if item.as_array().is_some() => item
            .as_array()
            .into_iter()
            .flatten()
            .map(|v| {
                v.as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| v.to_string())
            })
            .collect(),
        Some(item) => shell_split(item.as_str().unwrap_or_default())?,
        None => Vec::new(),
    };
    let mut case = Case::new(args);
    let side = |key: &str| -> Result<Option<Vec<u8>>> {
        if let Some(s) = doc.get(key).and_then(Item::as_str) {
            return Ok(Some(s.as_bytes().to_vec()));
        }
        let file = dir.join(format!("{name}.{key}"));
        Ok(if file.exists() {
            Some(std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?)
        } else {
            None
        })
    };
    case.stdin = side("stdin")?;
    if let Some(env) = doc.get("env") {
        case.inherit_env = env.get("inherit").and_then(Item::as_bool).unwrap_or(true);
        if let Some(add) = env.get("add").and_then(Item::as_table_like) {
            for (k, v) in add.iter() {
                case.env
                    .insert(k.to_string(), v.as_str().unwrap_or_default().to_string());
            }
        }
        if let Some(remove) = env.get("remove").and_then(Item::as_array) {
            case.env_remove = remove
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
        }
    }
    let base = doc
        .get("fs")
        .and_then(|f| f.get("base"))
        .and_then(Item::as_str)
        .map(|b| dir.join(b))
        .unwrap_or_else(|| dir.join(format!("{name}.in")));
    if base.is_dir() {
        case.files = read_files(&base);
    }
    if let Some(remove) = doc
        .get("fs")
        .and_then(|f| f.get("remove"))
        .and_then(Item::as_array)
    {
        case.remove = remove
            .iter()
            .filter_map(|v| v.as_str().map(PathBuf::from))
            .collect();
    }
    let status = match doc.get("status") {
        None => Expect::Code(0),
        Some(item) => match (item.as_str(), item.get("code").and_then(Item::as_integer)) {
            (Some("success"), _) => Expect::Code(0),
            (Some("failed"), _) => Expect::Failed,
            (Some("interrupted"), _) => Expect::Killed,
            (Some("skipped"), _) => Expect::Skip,
            (_, Some(code)) => Expect::Code(i32::try_from(code).unwrap_or(i32::MAX)),
            _ => bail!("{}: unknown `status`", path.display()),
        },
    };
    let out_dir = dir.join(format!("{name}.out"));
    Ok(Spec {
        stdout: side("stdout")?,
        stderr: side("stderr")?,
        effects: side(crate::effects::EXT)?,
        status,
        out_dir: out_dir.is_dir().then_some(out_dir),
        removed: side(REMOVED_EXT)?.map(|bytes| {
            String::from_utf8_lossy(&bytes)
                .lines()
                .filter(|l| !l.is_empty())
                .map(PathBuf::from)
                .collect()
        }),
        binary: doc.get("binary").and_then(Item::as_bool).unwrap_or(false),
        name,
        case,
    })
}

fn float_width(format: &str) -> Result<usize> {
    match format {
        "f32le" => Ok(4),
        "f64le" => Ok(8),
        other => bail!("float file format `{other}`: use f32le or f64le"),
    }
}

fn floats(bytes: &[u8], width: usize) -> Vec<f64> {
    bytes
        .chunks_exact(width)
        .map(|c| match c.try_into() {
            Ok(b) if width == 4 => f64::from(f32::from_le_bytes(b)),
            _ => c.try_into().map_or(f64::NAN, f64::from_le_bytes),
        })
        .collect()
}

fn compare_floats(
    expected: &[u8],
    actual: &[u8],
    width: usize,
    tol: Option<f64>,
) -> Option<String> {
    if !expected.len().is_multiple_of(width) || expected.len() != actual.len() {
        return Some(format!(
            "{} values expected, {} bytes got ({} bytes per value)\n",
            expected.len() / width,
            actual.len(),
            width
        ));
    }
    let (e, a) = (floats(expected, width), floats(actual, width));
    let tol = tol.unwrap_or(0.0);
    let same =
        |x: f64, y: f64| x == y || (x.is_nan() && y.is_nan()) || crate::tolerance::near(x, y, tol);
    let off: Vec<usize> = (0..e.len()).filter(|&i| !same(e[i], a[i])).collect();
    let first = *off.first()?;
    let (mut abs, mut rel) = (0.0f64, 0.0f64);
    for &i in &off {
        let d = (e[i] - a[i]).abs();
        abs = abs.max(d);
        rel = rel.max(d / e[i].abs().max(a[i].abs()).max(f64::MIN_POSITIVE));
    }
    Some(format!(
        "{} of {} values differ beyond the tolerance ({tol}); first at index {first}: {} expected, {} got; max abs diff {abs:e}, max rel diff {rel:e}\n",
        off.len(),
        e.len(),
        e[first],
        a[first]
    ))
}

pub(crate) fn compare(
    norm: &Normalizers,
    expected: &[u8],
    actual: &[u8],
    binary: bool,
    unordered: bool,
) -> Option<String> {
    if !binary && let (Ok(e), Ok(a)) = (std::str::from_utf8(expected), std::str::from_utf8(actual))
    {
        let e = norm.canonical(&norm.apply(e));
        let a = norm.canonical(&norm.apply(a));
        let tol = norm.float_tolerance;
        if unordered {
            let (mut el, mut al) = (lines(&e), lines(&a));
            let same = lines_match_in_any_order(&el, &al, tol);
            el.sort_unstable();
            al.sort_unstable();
            return (!same).then(|| {
                format!(
                    "(lines compared in any order: the original's own runs disagreed on it)\n{}",
                    render_diff(&el.join("\n"), &al.join("\n"), tol)
                )
            });
        }
        return (!text_matches(&e, &a, tol)).then(|| render_diff(&e, &a, tol));
    }
    (expected != actual).then(|| {
        format!(
            "binary content differs ({} bytes expected, {} bytes got)\n",
            expected.len(),
            actual.len()
        )
    })
}

pub struct ReplayOptions<'a> {
    pub repo: &'a Path,
    /// Command line of the build to test; the original's command when `None`.
    pub build: Option<&'a str>,
    pub timeout_secs: Option<u64>,
    /// Servers: the port the build listens on, when it is fixed.
    pub port: Option<u16>,
    /// Replay only this case (a library: only this unit).
    pub case: Option<&'a str>,
}

pub(crate) fn only_case<T>(
    items: Vec<T>,
    case: Option<&str>,
    name: impl Fn(&T) -> &str,
) -> Result<Vec<T>> {
    let Some(case) = case else {
        return Ok(items);
    };
    let names: Vec<String> = items.iter().map(|i| name(i).to_string()).collect();
    let kept: Vec<T> = items.into_iter().filter(|i| name(i) == case).collect();
    ensure!(
        !kept.is_empty(),
        "no case named {case}; the cases are: {}",
        names.join(", ")
    );
    Ok(kept)
}

#[derive(Debug, Serialize)]
pub struct Problem {
    pub field: String,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct Failure {
    pub case: String,
    pub args: Vec<String>,
    pub problems: Vec<Problem>,
}

/// Cases the build handed to the original: the front door appends a line to the file named by
/// `RUST_ANYTHING_FALLBACK_LOG` for every fallback (and may create it empty to report none).
#[derive(Debug, Serialize)]
pub struct Fallback {
    pub cases: usize,
    pub total: usize,
    pub rate: f64,
    /// The cases that fell back.
    pub names: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ReplayOutcome {
    pub build: Vec<String>,
    pub passed: usize,
    pub total: usize,
    pub skipped: usize,
    pub failures: Vec<Failure>,
    pub fallback: Option<Fallback>,
    pub report: PathBuf,
}

fn check(
    norm: &Normalizers,
    command: &[String],
    repo: &Path,
    spec: &Spec,
    timeout: Duration,
    effects: Option<&crate::effects::EffectsConfig>,
    base: Option<&Path>,
) -> Result<(Vec<Problem>, Option<bool>)> {
    let run = run_case(command, repo, &spec.case, timeout, effects, base)?;
    let mut problems = Vec::new();
    let status_ok = match (&spec.status, &run.status) {
        (Expect::Code(e), Status::Code(a)) => e == a,
        (Expect::Failed, Status::Code(a)) => *a != 0,
        (Expect::Killed, Status::Killed) | (Expect::Skip, _) => true,
        _ => false,
    };
    if !status_ok {
        let expected = match &spec.status {
            Expect::Code(c) => format!("exit code {c}"),
            Expect::Failed => "a non-zero exit code".to_string(),
            Expect::Killed => "killed by a signal".to_string(),
            Expect::Skip => String::new(),
        };
        problems.push(Problem {
            field: "status".to_string(),
            detail: format!("expected {expected}, got {}\n", run.status),
        });
    }
    let no_effects = Vec::new();
    for (field, expected, actual) in [
        ("stdout", &spec.stdout, &run.stdout),
        ("stderr", &spec.stderr, &run.stderr),
        (
            crate::effects::EXT,
            &spec.effects,
            run.effects.as_ref().unwrap_or(&no_effects),
        ),
    ] {
        if let Some(expected) = expected
            && let Some(detail) = compare(
                norm,
                expected,
                actual,
                spec.binary,
                norm.unordered(&spec.name, field),
            )
        {
            problems.push(Problem {
                field: field.to_string(),
                detail,
            });
        }
    }
    if let Some(expected) = &spec.removed
        && *expected != run.removed
    {
        let list = |paths: &[PathBuf]| -> String {
            paths
                .iter()
                .map(|p| format!("  {}\n", p.display()))
                .collect()
        };
        problems.push(Problem {
            field: "removed files".to_string(),
            detail: format!(
                "expected removed:\n{}got removed:\n{}",
                list(expected),
                list(&run.removed)
            ),
        });
    }
    if let Some(out_dir) = &spec.out_dir {
        for (path, expected) in read_files(out_dir) {
            let field = format!("file {}", path.display());
            match run.files.get(&path) {
                None => problems.push(Problem {
                    field,
                    detail: "not written\n".to_string(),
                }),
                Some(actual) => {
                    let unordered = norm.unordered(&spec.name, &field);
                    let detail = match norm.float_files.get(&path) {
                        Some(&width) => {
                            compare_floats(&expected, actual, width, norm.float_tolerance)
                        }
                        None => compare(norm, &expected, actual, false, unordered),
                    };
                    if let Some(detail) = detail {
                        problems.push(Problem { field, detail });
                    }
                }
            }
        }
    }
    Ok((problems, run.fallback))
}

pub(crate) fn report_markdown(outcome: &ReplayOutcome) -> String {
    stable_paths(&report_body(outcome))
}

/// The report with the per-run sandbox paths replaced, so two replays of the same build write the
/// same file and CI can commit it and fail on `git diff --exit-code`.
fn stable_paths(text: &str) -> String {
    static SANDBOX: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let tmp = std::env::temp_dir();
    let tmp = tmp.to_string_lossy();
    let tmp = tmp.trim_end_matches(['/', '\\']);
    let text = if tmp.len() > 1 {
        text.replace(tmp, "$TMPDIR")
    } else {
        text.to_string()
    };
    SANDBOX
        .get_or_init(|| regex::Regex::new(r"rust-anything-app-(\w+)-\d+-\d+").expect("valid regex"))
        .replace_all(&text, "rust-anything-app-$1")
        .into_owned()
}

fn report_body(outcome: &ReplayOutcome) -> String {
    let mut out = String::from("# App replay\n\n");
    out += &format!(
        "Build: `{}`\n\n",
        outcome
            .build
            .iter()
            .map(|w| shell_quote(w))
            .collect::<Vec<_>>()
            .join(" ")
    );
    out += &format!("Passed: {}/{}\n\n", outcome.passed, outcome.total);
    if outcome.skipped > 0 {
        out += &format!("Skipped: {}\n\n", outcome.skipped);
    }
    out += &match &outcome.fallback {
        Some(f) => format!(
            "Fallback rate: {:.1}% ({}/{} cases answered by the original)\n\n",
            f.rate * 100.0,
            f.cases,
            f.total
        ),
        None => "Fallback rate: not reported by this build\n\n".to_string(),
    };
    if outcome.failures.is_empty() {
        out += "Every case matches.\n";
        return out;
    }
    out += "## Failing cases\n";
    for f in &outcome.failures {
        out += &format!(
            "\n### {}\n\nArguments: `{}`\n",
            f.case,
            f.args
                .iter()
                .map(|w| shell_quote(w))
                .collect::<Vec<_>>()
                .join(" ")
        );
        for p in &f.problems {
            out += &format!(
                "\n{}:\n\n```diff\n{}```\n",
                p.field,
                p.detail.replace("```", "`\u{200b}``")
            );
        }
    }
    out
}

fn load_specs(dir: &Path) -> Result<Vec<Spec>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| {
            format!(
                "no cases in {}: run `rust-anything app record` first",
                dir.display()
            )
        })?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "toml"))
        .collect();
    paths.sort();
    let specs = paths
        .iter()
        .map(|p| load_spec(dir, p))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        !specs.is_empty(),
        "no cases in {}: run `rust-anything app record` first",
        dir.display()
    );
    Ok(specs)
}

/// The recorded cases' arguments, each with whether the original exited 0.
pub(crate) fn recorded_args(repo: &Path) -> Result<Vec<(Vec<String>, bool)>> {
    Ok(load_specs(&cases_dir(repo))?
        .into_iter()
        .map(|s| (s.case.args, matches!(s.status, Expect::Code(0))))
        .collect())
}

pub fn replay(opts: &ReplayOptions) -> Result<ReplayOutcome> {
    let repo = opts
        .repo
        .canonical()
        .with_context(|| format!("no folder {}", opts.repo.display()))?;
    let dir = cases_dir(&repo);
    let config = Config::load(&repo)?;
    if config.is_none()
        && let Ok(state) = crate::front_door::State::load(&repo)
        && state.kind == crate::front_door::AppKind::Library
    {
        ensure!(
            opts.build.is_none(),
            "{} is a library: its cases replay through each unit's bindings, so --build does not apply",
            state.name
        );
        return crate::library::replay(&repo, &state, opts.case);
    }
    let command = match (opts.build, &config) {
        (Some(build), _) => shell_split(build)?,
        (None, Some(c)) => c.run.clone(),
        (None, None) => bail!(
            "no {} in {}: run `rust-anything app record` first, or pass --build",
            CONFIG_FILE,
            dir.display()
        ),
    };
    ensure!(!command.is_empty(), "the build command is empty");
    let config = config.unwrap_or_else(|| Config {
        schema: SCHEMA.to_string(),
        name: "app".to_string(),
        run: Vec::new(),
        normalize: Vec::new(),
        normalizers: BTreeMap::new(),
        timeout_secs: default_timeout(),
        http: None,
        float_tolerance: None,
        float_files: BTreeMap::new(),
        unordered: BTreeMap::new(),
        effects: None,
        fresh: Vec::new(),
    });
    let norm = Normalizers::new(&config)?;
    let timeout = Duration::from_secs(opts.timeout_secs.unwrap_or(config.timeout_secs));
    if config.http.is_some() {
        return crate::http::replay(
            &repo, &config, &norm, &command, opts.port, opts.case, timeout,
        );
    }
    let specs = only_case(load_specs(&dir)?, opts.case, |s| &s.name)?;
    let effects = config.effects.as_ref();
    let base = if config.fresh.is_empty() {
        None
    } else {
        let base = dir.join(FRESH_DIR);
        ensure!(
            base.is_dir(),
            "no {}: record again to snapshot the fresh files",
            base.display()
        );
        Some(base)
    };
    let check_one = |spec: &Spec| {
        if matches!(spec.status, Expect::Skip) {
            return Ok((Vec::new(), None));
        }
        check(
            &norm,
            &command,
            &repo,
            spec,
            timeout,
            effects,
            base.as_deref(),
        )
    };
    let results: Vec<Result<_>> = if effects.is_some_and(crate::effects::EffectsConfig::serial) {
        specs.iter().map(check_one).collect()
    } else {
        par_map(&specs, check_one)
    };
    let mut outcome = ReplayOutcome {
        build: command.clone(),
        passed: 0,
        total: 0,
        skipped: 0,
        failures: Vec::new(),
        fallback: None,
        report: dir.join(REPORT_FILE),
    };
    let (mut reported, mut fell_back) = (false, Vec::new());
    for (spec, result) in specs.iter().zip(results) {
        if matches!(spec.status, Expect::Skip) {
            outcome.skipped += 1;
            continue;
        }
        let (problems, fallback) = result?;
        outcome.total += 1;
        reported |= fallback.is_some();
        if fallback == Some(true) {
            fell_back.push(spec.name.clone());
        }
        if problems.is_empty() {
            outcome.passed += 1;
        } else {
            outcome.failures.push(Failure {
                case: spec.name.clone(),
                args: spec.case.args.clone(),
                problems,
            });
        }
    }
    if reported {
        outcome.fallback = Some(Fallback {
            cases: fell_back.len(),
            total: outcome.total,
            rate: fell_back.len() as f64 / outcome.total.max(1) as f64,
            names: fell_back,
        });
    }
    std::fs::write(&outcome.report, report_markdown(&outcome))
        .with_context(|| format!("writing {}", outcome.report.display()))?;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_names_keep_their_tail() {
        let a = given_slug("tests/format/typescript/conformance/types/never/never.ts");
        let b = given_slug("tests/format/typescript/conformance/types/tuple/tuple.ts");
        assert_ne!(a, b);
        assert_eq!(
            a,
            "tests-format-typescript-conformance-types-never-never-ts"
        );
        assert_eq!(slug(&a).len(), 23);
    }

    #[test]
    fn reports_do_not_carry_sandbox_paths() {
        let tmp = std::env::temp_dir();
        let line = format!(
            "-cwd {}\n",
            tmp.join("rust-anything-app-run-4242-7")
                .join("cwd")
                .display()
        );
        let stable = stable_paths(&line);
        assert!(!stable.contains("4242"), "{stable}");
        assert!(stable.contains("$TMPDIR"), "{stable}");
        assert_eq!(
            stable,
            stable_paths(&line.replace("4242-7", "99-0")),
            "{stable}"
        );
    }

    fn text_matches_exact(e: &str, a: &str) -> bool {
        text_matches(e, a, None)
    }

    fn norm_with_tolerance(tol: Option<f64>) -> Normalizers {
        Normalizers::new(&Config {
            schema: SCHEMA.to_string(),
            name: "app".to_string(),
            run: Vec::new(),
            normalize: Vec::new(),
            normalizers: BTreeMap::new(),
            timeout_secs: default_timeout(),
            http: None,
            float_tolerance: tol,
            float_files: BTreeMap::new(),
            unordered: BTreeMap::new(),
            effects: None,
            fresh: Vec::new(),
        })
        .unwrap()
    }

    #[test]
    fn unordered_lines_pair_up_by_value_within_the_tolerance() {
        let norm = norm_with_tolerance(Some(1e-9));
        // `2.0` meets `2.0` and `10.0` meets `9.999999999999999`, but `10.0` sorts before `2.0`
        // as text, so pairing the sorted lines by position compared `10.0` against `2.0`.
        let expected = b"n 2.0\nn 10.0\n";
        let actual = b"n 2.0\nn 9.999999999999999\n";
        assert!(compare(&norm, expected, actual, false, false).is_none());
        assert!(compare(&norm, expected, actual, false, true).is_none());
        // A line that is genuinely different is still reported.
        let changed = b"n 2.0\nn 11.0\n";
        assert!(compare(&norm, expected, changed, false, true).is_some());
        // The order the lines arrive in does not matter.
        let swapped = b"n 9.999999999999999\nn 2.0\n";
        assert!(compare(&norm, expected, swapped, false, true).is_none());
    }

    #[test]
    fn unordered_lines_pair_up_around_a_wildcard() {
        let norm = norm_with_tolerance(Some(1e-9));
        // `[..]` takes the line the numeric pairing does not want, so a tolerance and a wildcard
        // in the same output do not fight over the ordering.
        let expected = b"n 10.0\nn [..]\n";
        assert!(compare(&norm, expected, b"n 10.0\nn 2.0\n", false, true).is_none());
        assert!(compare(&norm, expected, b"n 2.0\nn 10.0\n", false, true).is_none());
        assert!(
            compare(
                &norm,
                expected,
                b"n 2.0\nn 9.999999999999999\n",
                false,
                true
            )
            .is_none()
        );
        // Nothing to take the 10.0: both actual lines are far from it.
        assert!(compare(&norm, expected, b"n 2.0\nn 3.0\n", false, true).is_some());
    }

    #[test]
    fn unordered_lines_keep_a_plain_permutation_matching() {
        let norm = norm_with_tolerance(Some(1e-9));
        // The same lines in another order match whatever the numbers look like, and a value
        // written two ways stays a difference rather than becoming one by being re-sorted.
        assert!(compare(&norm, b"n 2\nn 2.0\n", b"n 2.0\nn 2\n", false, true).is_none());
        assert!(compare(&norm, b"n 2\nn 2\n", b"n 2.0\nn 2.0\n", false, true).is_some());
    }

    #[test]
    fn unordered_lines_without_a_tolerance_still_compare_as_text() {
        let norm = norm_with_tolerance(None);
        let expected = b"b\na\n";
        assert!(compare(&norm, expected, b"a\nb\n", false, true).is_none());
        assert!(compare(&norm, expected, b"a\nc\n", false, true).is_some());
        assert!(compare(&norm, expected, b"a\n", false, true).is_some());
        // Without a tolerance a number is text like any other, so this stays a mismatch.
        let numeric = b"n 10.0\nn 2.0\n";
        assert!(compare(&norm, numeric, b"n 2.0\nn 9.999999999999999\n", false, true).is_some());
    }

    #[cfg(unix)]
    #[test]
    fn replay_deadline_covers_descendants_that_keep_pipes_open() {
        for script in ["sleep 6 & wait", "sleep 6 & exit 0"] {
            let command = ["/bin/sh".into(), "-c".into(), script.into()];
            let start = Instant::now();
            let run = run_case(
                &command,
                Path::new("."),
                &Case::new(Vec::new()),
                Duration::from_millis(200),
                None,
                None,
            )
            .unwrap();
            assert!(start.elapsed() < Duration::from_secs(3), "{script}");
            assert!(run.status == Status::Timeout, "{script}: {}", run.status);
        }
    }

    #[test]
    fn matching_follows_trycmd() {
        assert!(text_matches_exact("a\nb\n", "a\nb\n"));
        assert!(text_matches_exact("now: [..]\n", "now: 1234\n"));
        assert!(text_matches_exact("x [..] y [..] z\n", "x 1 y 2 y 3 z\n"));
        assert!(!text_matches_exact("x [..] z\n", "x 1 y\n"));
        assert!(text_matches_exact(
            "head\n...\ntail\n",
            "head\n1\n2\ntail\n"
        ));
        assert!(text_matches_exact("head\n...\ntail\n", "head\ntail\n"));
        assert!(!text_matches_exact("head\ntail\n", "head\nmid\ntail\n"));
        assert!(text_matches_exact("a\r\nb", "a\nb\n"));
    }

    #[test]
    fn numbers_match_within_the_tolerance() {
        let e = "step 1\nenergy 0.3\nat [..]\n";
        let a = "step 1\nenergy 0.30000000000000004\nat 12:00\n";
        assert!(!text_matches(e, a, None));
        assert!(text_matches(e, a, Some(1e-9)));
        assert!(!text_matches(
            e,
            "step 2\nenergy 0.3\nat 12:00\n",
            Some(1e-9)
        ));
        assert!(!render_diff(e, a, Some(1e-9)).contains("energy"));
    }

    #[cfg(unix)]
    #[test]
    fn binary_capture_replays_its_text_companion_byte_for_byte() {
        let repo = temp_dir("binary-stream-pair").unwrap();
        let run = [
            "/bin/sh".to_string(),
            "-c".to_string(),
            r"printf '\377'; printf 'at 2026-11-16T10:00:00Z\n' >&2".to_string(),
        ];
        let recorded = record(&RecordOptions {
            repo: &repo,
            run: &run,
            inputs: None,
            normalize: &["timestamps".into()],
            name: None,
            traps: false,
            timeout_secs: None,
            http: false,
            server_args: &[],
            fresh: &[],
            pace_ms: None,
            port: None,
            float_tolerance: None,
            float_files: &[],
            effects: None,
        })
        .unwrap();
        assert_eq!(recorded.cases, 1);
        let replayed = replay(&ReplayOptions {
            repo: &repo,
            build: None,
            timeout_secs: None,
            port: None,
            case: None,
        })
        .unwrap();
        assert_eq!(
            (replayed.passed, replayed.total),
            (1, 1),
            "{:?}",
            replayed.failures
        );
        std::fs::remove_dir_all(repo).unwrap();
    }

    #[test]
    fn noise_is_classified() {
        let norm = Normalizers::new(&Config {
            schema: SCHEMA.to_string(),
            name: "app".to_string(),
            run: Vec::new(),
            normalize: Vec::new(),
            normalizers: BTreeMap::new(),
            timeout_secs: 30,
            http: None,
            float_tolerance: None,
            float_files: BTreeMap::new(),
            unordered: BTreeMap::new(),
            effects: None,
            fresh: Vec::new(),
        })
        .unwrap();
        let kind = |a: &str, b: &str| settle_text(&norm, a.as_bytes(), b.as_bytes()).1;
        assert_eq!(kind("a\nb\n", "a\nb\n"), None);
        assert_eq!(kind("x=1\ny=2\n", "y=2\nx=1\n"), Some(NoiseKind::Order));
        assert_eq!(kind("at 10:01\n", "at 10:02\n"), Some(NoiseKind::Values));
        assert_eq!(kind("a\nz\n", "a\nb\nc\nz\n"), Some(NoiseKind::Lines));
        let (kept, _, _) = settle_text(&norm, b"x=1\ny=2\n", b"y=2\nx=1\n");
        assert_eq!(kept, b"x=1\ny=2\n", "order noise keeps the lines exactly");
    }

    #[test]
    fn unordered_output_matches_in_any_order_but_not_other_lines() {
        let norm = Normalizers::new(&Config {
            schema: SCHEMA.to_string(),
            name: "app".to_string(),
            run: Vec::new(),
            normalize: Vec::new(),
            normalizers: BTreeMap::new(),
            timeout_secs: 30,
            http: None,
            float_tolerance: None,
            float_files: BTreeMap::new(),
            unordered: BTreeMap::from([("c".to_string(), vec!["stdout".to_string()])]),
            effects: None,
            fresh: Vec::new(),
        })
        .unwrap();
        assert!(norm.unordered("c", "stdout"));
        assert!(!norm.unordered("c", "stderr"));
        let e = b"x=1\ny=2\n";
        assert!(compare(&norm, e, b"y=2\nx=1\n", false, true).is_none());
        assert!(compare(&norm, e, b"y=2\nx=2\n", false, true).is_some());
        assert!(compare(&norm, e, b"x=1\n", false, true).is_some());
        assert!(compare(&norm, e, b"y=2\nx=1\n", false, false).is_some());
    }

    #[test]
    fn timestamps_cover_javascript_date_strings() {
        let re = Regex::new(TIMESTAMP).unwrap();
        let line = "Generated on Sat Nov 21 2026 01:30:12 GMT+0300 (Türkiye Standard Time) - ok";
        assert_eq!(re.replace_all(line, "[..]"), "Generated on [..] - ok");
        assert_eq!(
            re.replace_all("Mon Jan 05 2026 09:00:00 GMT-0800", "[..]"),
            "[..]"
        );
        assert_eq!(re.replace_all("at 2026-11-16T10:00:00Z", "[..]"), "at [..]");
        assert_eq!(re.replace_all("Saturday 12", "[..]"), "Saturday 12");
    }

    #[test]
    fn noise_becomes_wildcards() {
        assert_eq!(
            merge_noise(
                "start\nnow: 2026-11-16 10:00:01\nend\n",
                "start\nnow: 2026-11-16 10:00:02\nend\n"
            ),
            "start\nnow: 2026-11-16 10:00:[..]\nend\n"
        );
        assert_eq!(merge_noise("a\nb\nz\n", "a\nc\nd\nz\n"), "a\n...\nz\n");
        assert_eq!(merge_line("id=ab12 ok", "id=cd34 ok"), "id=[..] ok");
        let merged = merge_noise("x 1\n", "x 2\n");
        assert!(text_matches_exact(&merged, "x 3\n"));
    }

    #[test]
    fn normalizers_by_name() {
        let mut config = Config {
            schema: SCHEMA.to_string(),
            name: "app".to_string(),
            run: Vec::new(),
            normalize: vec!["timestamps".into(), "uuids".into(), "key-order".into()],
            normalizers: BTreeMap::from([("req".to_string(), r"req-\d+".to_string())]),
            timeout_secs: 30,
            http: None,
            float_tolerance: None,
            float_files: BTreeMap::new(),
            unordered: BTreeMap::new(),
            effects: None,
            fresh: Vec::new(),
        };
        let norm = Normalizers::new(&config).unwrap();
        assert_eq!(
            norm.apply(
                "at 2026-11-16T10:00:00.123Z id 123e4567-e89b-12d3-a456-426614174000 req-77"
            ),
            "at [..] id [..] [..]"
        );
        assert_eq!(
            norm.canonical("{\"b\":1,\"a\":{\"d\":2,\"c\":3}}\nplain"),
            "{\"a\":{\"c\":3,\"d\":2},\"b\":1}\nplain"
        );
        config.normalize.push("nope".into());
        assert!(Normalizers::new(&config).is_err());
    }

    #[test]
    fn examples_become_cases() {
        let repo = temp_dir("examples").unwrap();
        std::fs::write(repo.join("in.txt"), "hello\n").unwrap();
        std::fs::write(
            repo.join("README.md"),
            "# tool\n\n```console\n$ tool count in.txt\n3\n$ echo \"a b\" | tool words\n$ git status\n```\n\n```sh\nLANG=C tool upper 'x y'\ntool sort < in.txt\ntool a | grep b\n```\n",
        )
        .unwrap();
        let names = vec!["tool".to_string()];
        let cases = find_examples(&repo, &names);
        let args: Vec<Vec<String>> = cases.iter().map(|c| c.args.clone()).collect();
        assert_eq!(
            args,
            [
                vec!["count", "in.txt"],
                vec!["words"],
                vec!["upper", "x y"],
                vec!["sort"],
            ]
        );
        assert_eq!(cases[0].files[Path::new("in.txt")], b"hello\n");
        assert_eq!(cases[1].stdin.as_deref(), Some(&b"a b\n"[..]));
        assert_eq!(cases[2].env["LANG"], "C");
        assert_eq!(cases[3].stdin.as_deref(), Some(&b"hello\n"[..]));
        std::fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn traps_keep_the_shape_of_the_case() {
        let mut base = Case::new(vec!["div".into(), "7".into(), "2".into()]);
        base.stdin = Some(b"{\"a\":1}".to_vec());
        let traps = trap_cases(&[base]);
        assert!(traps.iter().all(|c| c.args[0] == "div"));
        assert!(traps.iter().any(|c| c.args == ["div", "-7", "2"]));
        assert!(traps.iter().any(|c| c.args == ["div", "7", "0"]));
        assert!(
            traps
                .iter()
                .any(|c| c.stdin.as_deref() == Some(JSON_TRAPS[0].as_bytes()))
        );
        let text = trap_cases(&[Case::new(vec!["upper".into(), "hi".into()])]);
        assert!(text.iter().any(|c| c.args[1] == "👍🏽 thumbs"));
    }

    #[cfg(unix)]
    #[test]
    fn recording_keeps_all_cases_when_a_generated_name_is_already_used() {
        let repo = temp_dir("case-name-collisions").unwrap();
        let inputs = repo.join("cases.jsonl");
        std::fs::write(
            &inputs,
            concat!(
                "{\"name\":\"x\",\"args\":[\"one\"]}\n",
                "{\"name\":\"x-3\",\"args\":[\"two\"]}\n",
                "{\"name\":\"x\",\"args\":[\"three\"]}\n"
            ),
        )
        .unwrap();
        let recorded = record(&RecordOptions {
            repo: &repo,
            run: &["/bin/echo".into()],
            inputs: Some(&inputs),
            normalize: &[],
            name: None,
            traps: false,
            timeout_secs: None,
            http: false,
            server_args: &[],
            fresh: &[],
            pace_ms: None,
            port: None,
            float_tolerance: None,
            float_files: &[],
            effects: None,
        })
        .unwrap();
        let read = |name: &str| std::fs::read_to_string(recorded.dir.join(name)).unwrap();
        assert_eq!(recorded.cases, 3);
        assert_eq!(read("x.stdout"), "one\n");
        assert_eq!(read("x-3.stdout"), "two\n");
        assert_eq!(read("x-4.stdout"), "three\n");
        std::fs::remove_dir_all(repo).unwrap();
    }

    #[test]
    fn names_come_from_the_run_command_and_manifests() {
        let repo = temp_dir("names").unwrap();
        std::fs::create_dir_all(repo.join("out")).unwrap();
        std::fs::write(
            repo.join("package.json"),
            r#"{"name":"x","bin":{"mytool":"b.js"}}"#,
        )
        .unwrap();
        let run: Vec<String> = ["java", "-cp", "out", "Tcli"].map(String::from).to_vec();
        assert_eq!(app_names(&repo, &run, None), ["tcli", "mytool"]);
        std::fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn python_program_names_are_case_insensitive() {
        assert!(is_python_program("PYTHON"));
        assert!(is_python_program("/tools/Python3.exe"));
        assert!(is_python_program("Py.cmd"));
        assert!(!is_python_program("pythonw.exe"));
    }

    #[test]
    fn shell_words() {
        assert_eq!(
            shell_split(r#"cargo run -q -- "a b" 'c d' e\ f"#).unwrap(),
            ["cargo", "run", "-q", "--", "a b", "c d", "e f"]
        );
        assert!(shell_split("a | b").is_err());
        assert!(shell_tokens("echo $(date)").is_none());
        assert!(shell_tokens("echo `date`").is_none());
        assert_eq!(
            shell_split(r"echo \$(date) \`date\`").unwrap(),
            ["echo", "$(date)", "`date`"]
        );
        #[cfg(windows)]
        assert_eq!(
            shell_split(r"C:\tools\rust-anything\app.exe --flag").unwrap(),
            [r"C:\tools\rust-anything\app.exe", "--flag"]
        );
        #[cfg(windows)]
        assert_eq!(
            shell_split(r"run C:\$cache\tool.exe").unwrap(),
            ["run", r"C:\$cache\tool.exe"]
        );
        #[cfg(windows)]
        assert_eq!(
            shell_split(r"\\server\share\tool.exe --flag").unwrap(),
            [r"\\server\share\tool.exe", "--flag"]
        );
        assert_eq!(shell_split(r"echo one\\two").unwrap(), ["echo", r"one\two"]);
    }
}
