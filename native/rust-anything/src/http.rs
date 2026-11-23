//! HTTP servers as a black box: start the original (or any build) on a port, send the recorded
//! requests in order, and store or compare what it answers.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use toml_edit::{DocumentMut, Item, Table, value};

use crate::app::{
    self, Config, Failure, Fallback, Normalizers, Problem, RecordOptions, RecordOutcome,
    ReplayOutcome,
};

pub const HTTP_DIR: &str = "http";
const READY: Duration = Duration::from_secs(60);
const SKIP_HEADERS: [&str; 12] = [
    "date",
    "host",
    "server",
    "connection",
    "keep-alive",
    "transfer-encoding",
    "content-length",
    "x-powered-by",
    "etag",
    "last-modified",
    "age",
    "via",
];
const LOCAL_HOSTS: [&str; 4] = ["localhost", "127.0.0.1", "0.0.0.0", "[::1]"];

/// `http` in `rust/app-cases/rust-anything.json`: the app is a server.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HttpConfig {
    /// A port the original always listens on. When unset, every start gets a free port through
    /// the `PORT` environment variable and `{port}` in the command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Arguments every start gets after its command: the original's and every build's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Repository paths copied (from the snapshot in `http/files/`) into a fresh working
    /// directory before every start, so state the server writes never leaks between runs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fresh: Vec<String>,
    /// Milliseconds to wait after each response before the next request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pace_ms: Option<u64>,
}

/// The `http` config of a recording: the saved one, changed by what this record was given.
pub(crate) fn merge_config(saved: Option<HttpConfig>, opts: &RecordOptions) -> Option<HttpConfig> {
    let asked =
        opts.http || opts.port.is_some() || !opts.server_args.is_empty() || opts.pace_ms.is_some();
    let mut config = match (saved, asked) {
        (Some(c), _) => c,
        (None, true) => HttpConfig::default(),
        (None, false) => return None,
    };
    if opts.port.is_some() {
        config.port = opts.port;
    }
    if !opts.server_args.is_empty() {
        config.args = opts.server_args.to_vec();
    }
    if !opts.fresh.is_empty() {
        config.fresh = opts.fresh.to_vec();
    }
    if let Some(pace) = opts.pace_ms {
        config.pace_ms = (pace > 0).then_some(pace);
    }
    Some(config)
}

pub(crate) const FILES_DIR: &str = "files";

pub(crate) fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)?.flatten() {
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(from, to)
            .with_context(|| format!("copying {} to {}", from.display(), to.display()))?;
    }
    Ok(())
}

/// Copies the `fresh` paths from the repository into `http/files/`, replacing the last snapshot.
fn snapshot(repo: &Path, fresh: &[String]) -> Result<Option<PathBuf>> {
    let dir = app::cases_dir(repo).join(HTTP_DIR).join(FILES_DIR);
    snapshot_to(repo, fresh, &dir)
}

/// Copies the `fresh` paths from the repository into `dir`, replacing the last snapshot.
pub(crate) fn snapshot_to(repo: &Path, fresh: &[String], dir: &Path) -> Result<Option<PathBuf>> {
    if fresh.is_empty() {
        return Ok(None);
    }
    let dir = dir.to_path_buf();
    let _ = std::fs::remove_dir_all(&dir);
    for path in fresh {
        let rel = Path::new(path);
        ensure!(
            rel.is_relative()
                && rel
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_))),
            "--fresh `{path}`: give a path inside the repository, relative to it"
        );
        let from = repo.join(rel);
        ensure!(from.exists(), "--fresh `{path}`: no such file or folder");
        let to = if path.ends_with('/') && from.is_dir() {
            dir.clone()
        } else {
            dir.join(rel)
        };
        copy_tree(&from, &to)?;
    }
    Ok(Some(dir))
}

/// The snapshot replay starts every build from, when the recording has `fresh` paths.
fn saved_snapshot(repo: &Path, config: &Config) -> Result<Option<PathBuf>> {
    if config.http.as_ref().is_none_or(|h| h.fresh.is_empty()) {
        return Ok(None);
    }
    let dir = app::cases_dir(repo).join(HTTP_DIR).join(FILES_DIR);
    ensure!(
        dir.is_dir(),
        "no {}: record again to snapshot the fresh files",
        dir.display()
    );
    Ok(Some(dir))
}

fn get() -> String {
    "GET".to_string()
}

/// One request, as recorded and as written in an `--inputs` line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Req {
    #[serde(default = "get")]
    pub method: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HttpInput {
    #[serde(default)]
    name: Option<String>,
    #[serde(flatten)]
    req: Req,
}

#[derive(Debug, Clone)]
struct Resp {
    status: u16,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

pub fn free_port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

fn local(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn fill(stream: &mut impl Read, buf: &mut Vec<u8>, need: usize) -> Result<()> {
    let mut chunk = [0u8; 8192];
    while buf.len() < need {
        let n = stream.read(&mut chunk)?;
        ensure!(n > 0, "the connection closed in the middle of the response");
        buf.extend_from_slice(&chunk[..n]);
    }
    Ok(())
}

fn chunk_line_end(stream: &mut impl Read, buf: &mut Vec<u8>) -> Result<usize> {
    loop {
        if let Some(end) = find(buf, b"\r\n") {
            return Ok(end);
        }
        let need = buf.len() + 1;
        fill(stream, buf, need)?;
    }
}

fn read_chunked(stream: &mut impl Read, mut buf: Vec<u8>) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let line_end = chunk_line_end(stream, &mut buf)?;
        let size_text = String::from_utf8_lossy(&buf[..line_end]).into_owned();
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or("").trim(), 16)
            .with_context(|| format!("bad chunk size `{size_text}`"))?;
        buf.drain(..line_end + 2);
        if size == 0 {
            loop {
                let line_end = chunk_line_end(stream, &mut buf)?;
                if line_end == 0 {
                    return Ok(body);
                }
                buf.drain(..line_end + 2);
            }
        }
        let end = size.checked_add(2).context("chunk size is too large")?;
        fill(stream, &mut buf, end)?;
        ensure!(&buf[size..end] == b"\r\n", "bad chunk terminator");
        body.extend_from_slice(&buf[..size]);
        buf.drain(..end);
    }
}

fn read_response(stream: &mut impl Read, head_only: bool) -> Result<Resp> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let (status, head, mut rest) = loop {
        let header_end = loop {
            if let Some(i) = find(&buf, b"\r\n\r\n") {
                break i;
            }
            let n = stream.read(&mut chunk)?;
            ensure!(n > 0, "the connection closed before a full response");
            buf.extend_from_slice(&chunk[..n]);
        };
        let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
        let status_line = head.split("\r\n").next().unwrap_or_default();
        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .with_context(|| format!("not an HTTP response: `{status_line}`"))?;
        let rest = buf.split_off(header_end + 4);
        if status == 101 || !(100..200).contains(&status) {
            break (status, head, rest);
        }
        buf = rest;
    };
    let lines = head.split("\r\n").skip(1);
    let mut headers: BTreeMap<String, String> = BTreeMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let v = v.trim().to_string();
            headers
                .entry(k.trim().to_ascii_lowercase())
                .and_modify(|e| {
                    e.push_str(", ");
                    e.push_str(&v);
                })
                .or_insert(v);
        }
    }
    let chunked = headers
        .get("transfer-encoding")
        .is_some_and(|t| t.to_ascii_lowercase().contains("chunked"));
    let body = if head_only || status == 204 || status == 304 || (100..200).contains(&status) {
        Vec::new()
    } else if chunked {
        read_chunked(stream, rest)?
    } else if let Some(lengths) = headers.get("content-length") {
        let parse = |value: &str| -> Result<usize> {
            let value = value.trim();
            ensure!(
                !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()),
                "invalid Content-Length"
            );
            value.parse().context("invalid Content-Length")
        };
        let mut lengths = lengths.split(',');
        let len = parse(lengths.next().unwrap_or_default())?;
        for other in lengths {
            ensure!(parse(other)? == len, "conflicting Content-Length headers");
        }
        fill(stream, &mut rest, len)?;
        rest.truncate(len);
        rest
    } else {
        stream
            .read_to_end(&mut rest)
            .context("reading close-delimited HTTP body")?;
        rest
    };
    Ok(Resp {
        status,
        headers,
        body,
    })
}

fn send(port: u16, req: &Req, timeout: Duration) -> Result<Resp> {
    let mut stream = TcpStream::connect_timeout(&local(port), timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let body = req.body.as_deref().unwrap_or("").as_bytes();
    let mut head = format!(
        "{} {} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n",
        req.method, req.path
    );
    for (k, v) in &req.headers {
        if !matches!(
            k.to_ascii_lowercase().as_str(),
            "host" | "connection" | "content-length"
        ) {
            head += &format!("{k}: {v}\r\n");
        }
    }
    if req.body.is_some() || matches!(req.method.as_str(), "POST" | "PUT" | "PATCH") {
        head += &format!("Content-Length: {}\r\n", body.len());
    }
    head += "\r\n";
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    read_response(&mut stream, req.method == "HEAD")
}

/// A server process; stopped (SIGTERM, then SIGKILL) when dropped.
struct Server {
    child: Child,
    port: u16,
    dir: PathBuf,
    cwd: PathBuf,
    log: PathBuf,
    stopped: bool,
    sinks: Option<crate::effects::Sinks>,
}

impl Server {
    fn start(command: &[String], launch: &Launch, repo: &Path, port: u16) -> Result<Server> {
        let dir = app::temp_dir("server")?;
        let words: Vec<String> = command
            .iter()
            .map(|w| app::resolve_word(repo, &w.replace("{port}", &port.to_string())))
            .collect();
        let (program, rest) = words.split_first().context("empty command")?;
        let log = dir.join("fallback.log");
        let cwd = match launch.files {
            Some(files) => {
                let cwd = dir.join("cwd");
                copy_tree(files, &cwd)?;
                cwd
            }
            None => repo.to_path_buf(),
        };
        let sinks = launch
            .effects
            .map(|config| -> Result<_> {
                let sinks = crate::effects::Sinks::start(config)?;
                sinks.reset(&cwd)?;
                Ok(sinks)
            })
            .transpose()?;
        let mut child = Command::new(program)
            .args(rest)
            .args(
                launch
                    .args
                    .iter()
                    .map(|a| a.replace("{port}", &port.to_string())),
            )
            .current_dir(&cwd)
            .envs(sinks.iter().flat_map(crate::effects::Sinks::env))
            .env("PORT", port.to_string())
            .env(app::FALLBACK_ENV, &log)
            .stdin(Stdio::null())
            .stdout(std::fs::File::create(dir.join("stdout"))?)
            .stderr(std::fs::File::create(dir.join("stderr"))?)
            .spawn()
            .with_context(|| format!("starting `{}`", command.join(" ")))?;
        let deadline = Instant::now() + READY;
        loop {
            if TcpStream::connect_timeout(&local(port), Duration::from_millis(200)).is_ok() {
                break;
            }
            let failure = if let Some(status) = child.try_wait()? {
                Some(format!("exited ({status}) before listening"))
            } else if Instant::now() > deadline {
                Some(format!("did not listen within {}s", READY.as_secs()))
            } else {
                None
            };
            if let Some(failure) = failure {
                let _ = child.kill();
                let _ = child.wait();
                let stderr = std::fs::read_to_string(dir.join("stderr")).unwrap_or_default();
                let tail: Vec<&str> = stderr.lines().rev().take(15).collect();
                let tail: Vec<&str> = tail.into_iter().rev().collect();
                let _ = std::fs::remove_dir_all(&dir);
                bail!(
                    "`{}` {failure} on port {port}{}",
                    command.join(" "),
                    if tail.is_empty() {
                        String::new()
                    } else {
                        format!(":\n{}", tail.join("\n"))
                    }
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(Server {
            child,
            port,
            dir,
            cwd,
            log,
            stopped: false,
            sinks,
        })
    }

    fn fallbacks(&self) -> Option<usize> {
        std::fs::read_to_string(&self.log)
            .ok()
            .map(|t| t.lines().count())
    }

    fn stop(&mut self) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .args(["-TERM", &self.child.id().to_string()])
                .stderr(Stdio::null())
                .status();
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if let Ok(Some(_)) = self.child.try_wait() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

type Answer = (
    std::result::Result<Resp, String>,
    Option<bool>,
    Option<Vec<u8>>,
);

/// What every start gets besides its command: the server's arguments and its fresh files.
struct Launch<'a> {
    args: &'a [String],
    files: Option<&'a Path>,
    pace: Option<Duration>,
    effects: Option<&'a crate::effects::EffectsConfig>,
}

/// Starts the server once and sends every request in order.
fn run_all(
    command: &[String],
    launch: &Launch,
    repo: &Path,
    port: u16,
    reqs: &[&Req],
    timeout: Duration,
) -> Result<Vec<Answer>> {
    let mut server = Server::start(command, launch, repo, port)?;
    let mut out = Vec::new();
    for req in reqs {
        let before = server.fallbacks().unwrap_or(0);
        let resp = send(server.port, req, timeout).map_err(|e| format!("{e:#}"));
        if let Some(pace) = launch.pace {
            std::thread::sleep(pace);
        }
        let fell_back = server.fallbacks().map(|after| after > before);
        let effects = server.sinks.as_ref().map(|s| s.take(&server.cwd));
        out.push((resp, fell_back, effects));
    }
    server.stop();
    Ok(out)
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = text.get(i + 1..i + 3)
            && let Ok(b) = u8::from_str_radix(hex, 16)
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn percent_encode(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(char::from(b));
        } else {
            out += &format!("%{b:02X}");
        }
    }
    out
}

fn parse_curl(line: &str, repo: &Path, doc_dir: &Path) -> Option<Req> {
    let toks = app::shell_tokens(line)?;
    if toks.iter().any(|t| t.op) {
        return None;
    }
    let words: Vec<String> = toks.into_iter().map(|t| t.text).collect();
    let (first, args) = words.split_first()?;
    if first != "curl" {
        return None;
    }
    let mut method: Option<String> = None;
    let mut headers = BTreeMap::new();
    let mut data: Vec<String> = Vec::new();
    let mut url: Option<String> = None;
    let mut i = 0;
    let read_file = |name: &str| -> Option<String> {
        let name = name.strip_prefix('@')?;
        [repo.join(name), doc_dir.join(name)]
            .iter()
            .find(|p| p.is_file())
            .and_then(|p| std::fs::read_to_string(p).ok())
    };
    while i < args.len() {
        let arg = &args[i];
        let mut next = || -> Option<String> {
            i += 1;
            args.get(i).cloned()
        };
        match arg.as_str() {
            "-X" | "--request" => method = Some(next()?),
            "-H" | "--header" => {
                let h = next()?;
                let (k, v) = h.split_once(':')?;
                headers.insert(k.trim().to_string(), v.trim().to_string());
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii" => {
                let d = next()?;
                data.push(if d.starts_with('@') && arg != "--data-raw" {
                    let mut contents = read_file(&d)?;
                    if arg != "--data-binary" {
                        contents.retain(|c| !matches!(c, '\r' | '\n' | '\0'));
                    }
                    contents
                } else {
                    d
                });
            }
            "--json" => {
                let d = next()?;
                data.push(if d.starts_with('@') {
                    read_file(&d)?
                } else {
                    d
                });
                headers
                    .entry("Content-Type".to_string())
                    .or_insert_with(|| "application/json".to_string());
                headers
                    .entry("Accept".to_string())
                    .or_insert_with(|| "application/json".to_string());
            }
            "-A" | "--user-agent" => {
                headers.insert("User-Agent".to_string(), next()?);
            }
            "-b" | "--cookie" => {
                headers.insert("Cookie".to_string(), next()?);
            }
            "-e" | "--referer" => {
                headers.insert("Referer".to_string(), next()?);
            }
            "-o" | "--output" | "-w" | "--write-out" | "-m" | "--max-time"
            | "--connect-timeout" => {
                next()?;
            }
            "--silent"
            | "--show-error"
            | "--include"
            | "--verbose"
            | "--location"
            | "--fail"
            | "--insecure"
            | "--no-progress-meter" => {}
            other if other.starts_with("-X") && other.len() > 2 => {
                method = Some(other[2..].to_string());
            }
            other if other.starts_with("-H") && other.len() > 2 => {
                let (k, v) = other[2..].split_once(':')?;
                headers.insert(k.trim().to_string(), v.trim().to_string());
            }
            other
                if other.starts_with('-')
                    && other.len() > 1
                    && !other.starts_with("--")
                    && other[1..].chars().all(|c| "sSivLfk".contains(c)) => {}
            other if other.starts_with('-') => return None,
            other => url = Some(other.to_string()),
        }
        i += 1;
    }
    let url = url?;
    let (scheme, rest) = url.split_once("://").unwrap_or(("http", &url));
    if !matches!(scheme, "http" | "https") {
        return None;
    }
    let rest = rest.split('#').next()?;
    let (host, target) = rest.split_at(rest.find(['/', '?']).unwrap_or(rest.len()));
    let path = if target.starts_with('/') {
        target.to_string()
    } else {
        format!("/{target}")
    };
    let host = if host.starts_with('[') {
        host.split_inclusive(']').next().unwrap_or(host)
    } else {
        host.split(':').next().unwrap_or(host)
    };
    if !LOCAL_HOSTS.contains(&host) {
        return None;
    }
    let body = (!data.is_empty()).then(|| data.join("&"));
    if body.is_some()
        && !headers
            .keys()
            .any(|k| k.eq_ignore_ascii_case("content-type"))
    {
        headers.insert(
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );
    }
    let method = method.unwrap_or_else(|| if body.is_some() { "POST" } else { "GET" }.to_string());
    Some(Req {
        method,
        path,
        headers,
        body,
    })
}

fn read_inputs(path: &Path) -> Result<Vec<(Option<String>, Req)>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let input: HttpInput = serde_json::from_str(line).with_context(|| {
            format!(
                "{}:{}: expected {{\"method\", \"path\", \"headers\": {{...}}, \"body\"}}",
                path.display(),
                n + 1
            )
        })?;
        ensure!(
            input.req.path.starts_with('/'),
            "{}:{}: `path` must start with /",
            path.display(),
            n + 1
        );
        out.push((input.name, input.req));
    }
    Ok(out)
}

fn trap_values(raw: &str) -> Vec<&'static str> {
    if raw.parse::<i128>().is_ok() {
        app::INT_TRAPS.to_vec()
    } else if raw.parse::<f64>().is_ok() && raw.contains(['.', 'e', 'E']) {
        app::FLOAT_TRAPS.to_vec()
    } else {
        app::STRING_TRAPS.to_vec()
    }
}

fn traps(bases: &[Req]) -> Vec<Req> {
    let mut out = Vec::new();
    for base in bases {
        let (path, query) = match base.path.split_once('?') {
            Some((p, q)) => (p, Some(q)),
            None => (base.path.as_str(), None),
        };
        let segments: Vec<&str> = path.split('/').collect();
        for i in 2..segments.len() {
            let raw = percent_decode(segments[i]);
            for v in trap_values(&raw).into_iter().filter(|v| *v != raw) {
                let mut parts: Vec<String> = segments.iter().map(|s| s.to_string()).collect();
                parts[i] = percent_encode(v);
                let mut req = base.clone();
                req.path = parts.join("/") + &query.map(|q| format!("?{q}")).unwrap_or_default();
                out.push(req);
            }
        }
        if let Some(query) = query {
            let pairs: Vec<&str> = query.split('&').collect();
            for (i, pair) in pairs.iter().enumerate() {
                let Some((key, raw)) = pair.split_once('=') else {
                    continue;
                };
                let raw = percent_decode(raw);
                for v in trap_values(&raw).into_iter().filter(|v| *v != raw) {
                    let mut new: Vec<String> = pairs.iter().map(|p| p.to_string()).collect();
                    new[i] = format!("{key}={}", percent_encode(v));
                    let mut req = base.clone();
                    req.path = format!("{path}?{}", new.join("&"));
                    out.push(req);
                }
            }
        }
        if let Some(body) = &base.body {
            let json =
                serde_json::from_str::<Value>(body).is_ok_and(|v| v.is_object() || v.is_array());
            let values: &[&str] = if json {
                &app::JSON_TRAPS
            } else {
                &app::STDIN_TRAPS
            };
            for v in values {
                let mut req = base.clone();
                req.body = Some(v.to_string());
                out.push(req);
            }
        }
    }
    out.push(Req {
        method: "GET".to_string(),
        path: "/rust-anything-missing".to_string(),
        headers: BTreeMap::new(),
        body: None,
    });
    out.truncate(app::MAX_TRAPS);
    out
}

fn case_slug(req: &Req) -> String {
    let first = req
        .path
        .split(['/', '?'])
        .find(|s| !s.is_empty())
        .unwrap_or("root");
    app::slug(&format!("{} {first}", req.method))
}

fn settle_headers(
    norm: &Normalizers,
    a: &BTreeMap<String, String>,
    b: &BTreeMap<String, String>,
) -> (BTreeMap<String, String>, bool) {
    let mut out = BTreeMap::new();
    let mut noisy = false;
    for (k, va) in a {
        if SKIP_HEADERS.contains(&k.as_str()) {
            continue;
        }
        let Some(vb) = b.get(k) else {
            noisy = true;
            continue;
        };
        let (va, vb) = (norm.apply(va), norm.apply(vb));
        if va == vb {
            out.insert(k.clone(), va);
        } else {
            noisy = true;
            out.insert(k.clone(), app::merge_line(&va, &vb));
        }
    }
    noisy |= b
        .keys()
        .any(|k| !a.contains_key(k) && !SKIP_HEADERS.contains(&k.as_str()));
    (out, noisy)
}

fn table(entries: &BTreeMap<String, String>) -> Table {
    let mut t = Table::new();
    for (k, v) in entries {
        t[k.as_str()] = value(v.as_str());
    }
    t
}

fn write_case(
    dir: &Path,
    name: &str,
    req: &Req,
    status: u16,
    headers: &BTreeMap<String, String>,
    body: &[u8],
    binary: bool,
) -> Result<()> {
    let mut doc = DocumentMut::new();
    let mut request = Table::new();
    request["method"] = value(req.method.as_str());
    request["path"] = value(req.path.as_str());
    if let Some(body) = &req.body {
        request["body"] = value(body.as_str());
    }
    if !req.headers.is_empty() {
        request.insert("headers", Item::Table(table(&req.headers)));
    }
    doc.insert("request", Item::Table(request));
    let mut response = Table::new();
    response["status"] = value(i64::from(status));
    if binary {
        response["binary"] = value(true);
    }
    if !headers.is_empty() {
        response.insert("headers", Item::Table(table(headers)));
    }
    doc.insert("response", Item::Table(response));
    std::fs::write(dir.join(format!("{name}.toml")), doc.to_string())?;
    std::fs::write(dir.join(format!("{name}.body")), body)?;
    Ok(())
}

pub(crate) fn record(
    repo: &Path,
    config: &Config,
    norm: &Normalizers,
    opts: &RecordOptions,
) -> Result<RecordOutcome> {
    let examples = app::scan_examples(repo, |line, doc_dir| parse_curl(line, repo, doc_dir));
    let inputs = match opts.inputs {
        Some(path) => read_inputs(path)?,
        None => Vec::new(),
    };
    let mut planned: Vec<(String, Req)> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut taken = BTreeSet::new();
    let mut counts = [0usize; 3];
    let mut bases = Vec::new();
    let mut push = |kind: usize, given: Option<String>, req: Req| -> bool {
        let key = serde_json::to_string(&req).unwrap_or_default();
        if !seen.insert(key) {
            return false;
        }
        counts[kind] += 1;
        let prefix = ["example", "input", "trap"][kind];
        let name = match given.map(|g| app::given_slug(&g)).filter(|g| !g.is_empty()) {
            Some(g) => g,
            None => format!("{prefix}-{:03}-{}", counts[kind], case_slug(&req)),
        };
        let name = app::reserve_case_name(&mut taken, name, counts[kind]);
        planned.push((name, req));
        true
    };
    for req in examples {
        if push(0, None, req.clone()) {
            bases.push(req);
        }
    }
    for (given, req) in inputs {
        if push(1, given, req.clone()) {
            bases.push(req);
        }
    }
    if bases.is_empty() {
        let root = Req {
            method: "GET".to_string(),
            path: "/".to_string(),
            headers: BTreeMap::new(),
            body: None,
        };
        push(0, None, root.clone());
        bases.push(root);
    }
    if opts.traps {
        for req in traps(&bases) {
            push(2, None, req);
        }
    }
    planned.sort_by(|a, b| a.0.cmp(&b.0));

    let timeout = Duration::from_secs(config.timeout_secs);
    let fixed = config.http.as_ref().and_then(|h| h.port);
    let reqs: Vec<&Req> = planned.iter().map(|(_, r)| r).collect();
    let http = config.http.clone().unwrap_or_default();
    let files = snapshot(repo, &http.fresh)?;
    let launch = Launch {
        args: &http.args,
        files: files.as_deref(),
        pace: http.pace_ms.map(Duration::from_millis),
        effects: config.effects.as_ref(),
    };
    let one = run_all(
        &config.run,
        &launch,
        repo,
        fixed.map_or_else(free_port, Ok)?,
        &reqs,
        timeout,
    )?;
    let two = run_all(
        &config.run,
        &launch,
        repo,
        fixed.map_or_else(free_port, Ok)?,
        &reqs,
        timeout,
    )?;

    let dir = app::cases_dir(repo).join(HTTP_DIR);
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
    for (((name, req), (a, _, fx_a)), (b, _, fx_b)) in planned.iter().zip(one).zip(two) {
        let (Ok(a), Ok(b)) = (a, b) else {
            outcome.timed_out.push(name.clone());
            continue;
        };
        let (body, body_noise, binary) = app::settle_text(norm, &a.body, &b.body);
        let (headers, header_noisy) = settle_headers(norm, &a.headers, &b.headers);
        write_case(&dir, name, req, a.status, &headers, &body, binary)?;
        let effects_file = dir.join(format!("{name}.{}", crate::effects::EXT));
        let _ = std::fs::remove_file(&effects_file);
        let mut fx_noise = None;
        if let (Some(fa), Some(fb)) = (&fx_a, &fx_b) {
            let (fx, noise, _) = app::settle_text(norm, fa, fb);
            std::fs::write(&effects_file, fx)?;
            fx_noise = noise;
        }
        outcome.cases += 1;
        if body_noise.is_some() || header_noisy || fx_noise.is_some() {
            outcome.noisy.push(name.clone());
        }
        for (field, kind) in [
            ("body", body_noise),
            ("headers", header_noisy.then_some(app::NoiseKind::Values)),
            (crate::effects::EXT, fx_noise),
        ] {
            if let Some(kind) = kind {
                outcome.noise.push(app::Noise {
                    case: name.clone(),
                    field: field.to_string(),
                    kind,
                });
            }
        }
        if a.status != b.status || (binary && body_noise.is_some()) {
            outcome.unstable.push(name.clone());
        }
    }
    Ok(outcome)
}

struct Spec {
    name: String,
    req: Req,
    status: u16,
    headers: BTreeMap<String, String>,
    body: Option<Vec<u8>>,
    binary: bool,
    effects: Option<Vec<u8>>,
}

fn strings(item: Option<&Item>) -> BTreeMap<String, String> {
    item.and_then(Item::as_table_like)
        .map(|t| {
            t.iter()
                .map(|(k, v)| (k.to_string(), v.as_str().unwrap_or_default().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn load_spec(dir: &Path, path: &Path) -> Result<Spec> {
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let doc: DocumentMut = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?
        .parse()
        .with_context(|| format!("parsing {}", path.display()))?;
    let request = doc.get("request").context("no [request]")?;
    let response = doc.get("response").context("no [response]")?;
    let body_file = dir.join(format!("{name}.body"));
    let effects_file = dir.join(format!("{name}.{}", crate::effects::EXT));
    Ok(Spec {
        effects: if effects_file.exists() {
            Some(std::fs::read(&effects_file)?)
        } else {
            None
        },
        req: Req {
            method: request
                .get("method")
                .and_then(Item::as_str)
                .unwrap_or("GET")
                .to_string(),
            path: request
                .get("path")
                .and_then(Item::as_str)
                .with_context(|| format!("{}: no request.path", path.display()))?
                .to_string(),
            headers: strings(request.get("headers")),
            body: request
                .get("body")
                .and_then(Item::as_str)
                .map(str::to_string),
        },
        status: response
            .get("status")
            .and_then(Item::as_integer)
            .and_then(|s| u16::try_from(s).ok())
            .with_context(|| format!("{}: no response.status", path.display()))?,
        headers: strings(response.get("headers")),
        body: if body_file.exists() {
            Some(std::fs::read(&body_file)?)
        } else {
            None
        },
        binary: response
            .get("binary")
            .and_then(Item::as_bool)
            .unwrap_or(false),
        name,
    })
}

/// The recorded requests, as `METHOD /path` without the query, in case order.
pub(crate) fn recorded_routes(repo: &Path) -> Result<Vec<String>> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for spec in load_specs(&app::cases_dir(repo).join(HTTP_DIR))? {
        let path = spec.req.path.split('?').next().unwrap_or("/").to_string();
        let route = format!("{} {path}", spec.req.method);
        if seen.insert(route.clone()) {
            out.push(route);
        }
    }
    Ok(out)
}

fn load_specs(dir: &Path) -> Result<Vec<Spec>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| {
            format!(
                "no HTTP cases in {}: run `rust-anything app record` first",
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
    ensure!(!specs.is_empty(), "no HTTP cases in {}", dir.display());
    Ok(specs)
}

pub(crate) fn replay(
    repo: &Path,
    config: &Config,
    norm: &Normalizers,
    command: &[String],
    port: Option<u16>,
    case: Option<&str>,
    timeout: Duration,
) -> Result<ReplayOutcome> {
    let dir = app::cases_dir(repo);
    let specs = app::only_case(load_specs(&dir.join(HTTP_DIR))?, case, |s| &s.name)?;
    let port = match port.or(config.http.as_ref().and_then(|h| h.port)) {
        Some(p) => p,
        None => free_port()?,
    };
    let reqs: Vec<&Req> = specs.iter().map(|s| &s.req).collect();
    let files = saved_snapshot(repo, config)?;
    let http = config.http.clone().unwrap_or_default();
    let launch = Launch {
        args: &http.args,
        files: files.as_deref(),
        pace: http.pace_ms.map(Duration::from_millis),
        effects: config.effects.as_ref(),
    };
    let answers = run_all(command, &launch, repo, port, &reqs, timeout)?;
    let mut outcome = ReplayOutcome {
        build: command.to_vec(),
        passed: 0,
        total: 0,
        skipped: 0,
        failures: Vec::new(),
        fallback: None,
        report: dir.join(app::REPORT_FILE),
    };
    let (mut reported, mut fell_back) = (false, Vec::new());
    for (spec, (answer, fallback, effects)) in specs.iter().zip(answers) {
        outcome.total += 1;
        reported |= fallback.is_some();
        if fallback == Some(true) {
            fell_back.push(spec.name.clone());
        }
        let mut problems = Vec::new();
        match answer {
            Err(e) => problems.push(Problem {
                field: "response".to_string(),
                detail: format!("no response: {e}\n"),
            }),
            Ok(resp) => {
                if resp.status != spec.status {
                    problems.push(Problem {
                        field: "status".to_string(),
                        detail: format!("expected {}, got {}\n", spec.status, resp.status),
                    });
                }
                for (k, expected) in &spec.headers {
                    let field = format!("header {k}");
                    let key = k.to_ascii_lowercase();
                    match resp.headers.get(&key) {
                        None => problems.push(Problem {
                            field,
                            detail: format!("- {expected}\n(missing)\n"),
                        }),
                        Some(actual) => {
                            let (e, a) = (norm.apply(expected), norm.apply(actual));
                            if !app::line_matches(&e, &a) {
                                problems.push(Problem {
                                    field,
                                    detail: format!("- {e}\n+ {a}\n"),
                                });
                            }
                        }
                    }
                }
                if let Some(expected) = &spec.body
                    && let Some(detail) = app::compare(
                        norm,
                        expected,
                        &resp.body,
                        spec.binary,
                        norm.unordered(&spec.name, "body"),
                    )
                {
                    problems.push(Problem {
                        field: "body".to_string(),
                        detail,
                    });
                }
            }
        }
        if let Some(expected) = &spec.effects
            && let Some(detail) = app::compare(
                norm,
                expected,
                effects.as_deref().unwrap_or_default(),
                false,
                norm.unordered(&spec.name, crate::effects::EXT),
            )
        {
            problems.push(Problem {
                field: crate::effects::EXT.to_string(),
                detail,
            });
        }
        if problems.is_empty() {
            outcome.passed += 1;
        } else {
            outcome.failures.push(Failure {
                case: spec.name.clone(),
                args: vec![spec.req.method.clone(), spec.req.path.clone()],
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
    std::fs::write(&outcome.report, app::report_markdown(&outcome))
        .with_context(|| format!("writing {}", outcome.report.display()))?;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curl_lines_become_requests() {
        let here = Path::new(".");
        let r = parse_curl(
            "curl -sS -X POST -H 'Content-Type: text/plain' -d 'hello world' http://localhost:8080/upper?x=1",
            here,
            here,
        )
        .unwrap();
        assert_eq!(r.method, "POST");
        assert_eq!(r.path, "/upper?x=1");
        assert_eq!(r.body.as_deref(), Some("hello world"));
        assert_eq!(r.headers["Content-Type"], "text/plain");
        let g = parse_curl("curl 127.0.0.1:3000", here, here).unwrap();
        assert_eq!((g.method.as_str(), g.path.as_str()), ("GET", "/"));
        let form = parse_curl("curl -d a=1 http://[::1]:80/f", here, here).unwrap();
        assert_eq!(form.method, "POST");
        assert_eq!(
            form.headers["Content-Type"],
            "application/x-www-form-urlencoded"
        );
        assert!(parse_curl("curl https://example.com/x", here, here).is_none());
        let query = parse_curl("curl 'http://localhost?x=y#section'", here, here).unwrap();
        assert_eq!(query.path, "/?x=y");
        let fragment = parse_curl("curl 'http://[::1]:8080/path#section'", here, here).unwrap();
        assert_eq!(fragment.path, "/path");
        assert!(parse_curl("curl ftp://localhost/file", here, here).is_none());
        assert!(parse_curl("curl --unknown x http://localhost/", here, here).is_none());
        assert!(parse_curl("curl http://localhost/ | jq .", here, here).is_none());
    }

    #[test]
    fn curl_data_and_binary_file_inputs_match_curl_file_semantics() {
        let root = app::temp_dir("curl-data").unwrap();
        std::fs::write(root.join("body.txt"), b"one\r\ntwo\0\nthree").unwrap();
        let body = |flag| {
            parse_curl(
                &format!("curl {flag} @body.txt http://localhost/upper"),
                &root,
                &root,
            )
            .unwrap()
            .body
            .unwrap()
        };
        let data = body("--data");
        let binary = body("--data-binary");
        std::fs::remove_dir_all(root).unwrap();
        assert_eq!(data, "onetwothree");
        assert_eq!(binary, "one\r\ntwo\0\nthree");
    }

    #[test]
    fn informational_responses_do_not_replace_the_final_response() {
        let raw = b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 103 Early Hints\r\nLink: </app.js>\r\n\r\nHTTP/1.1 201 Created\r\nContent-Length: 2\r\n\r\nok";
        let response = read_response(&mut &raw[..], false).unwrap();
        assert_eq!(response.status, 201);
        assert_eq!(response.body, b"ok");
        assert!(!response.headers.contains_key("link"));

        let switching = b"HTTP/1.1 101 Switching Protocols\r\n\r\n";
        assert_eq!(
            read_response(&mut &switching[..], false).unwrap().status,
            101
        );
    }

    #[test]
    fn responses_parse_with_length_chunks_or_close() {
        let fixed =
            b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\nhelloEXTRA";
        let r = read_response(&mut &fixed[..], false).unwrap();
        assert_eq!((r.status, r.body.as_slice()), (200, &b"hello"[..]));
        assert_eq!(r.headers["content-type"], "text/plain");
        let chunked = b"HTTP/1.1 404 Not Found\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nnot \r\n5;x=y\r\nfound\r\n0\r\n\r\n";
        let r = read_response(&mut &chunked[..], false).unwrap();
        assert_eq!((r.status, r.body.as_slice()), (404, &b"not found"[..]));
        let close = b"HTTP/1.0 500 Oops\r\n\r\nbye";
        assert_eq!(read_response(&mut &close[..], false).unwrap().body, b"bye");
    }

    #[test]
    fn malformed_http_chunks_are_errors() {
        let mut bad_ending = &b"1\r\naXX0\r\n\r\n"[..];
        assert!(read_chunked(&mut bad_ending, Vec::new()).is_err());
        let max = format!("{:x}\r\n", usize::MAX);
        assert!(read_chunked(&mut max.as_bytes(), Vec::new()).is_err());
        let mut truncated = &b"1\r\na\r\n0\r\n"[..];
        assert!(read_chunked(&mut truncated, Vec::new()).is_err());
        let mut trailers = &b"1\r\na\r\n0\r\nX-Trailer: present\r\n\r\n"[..];
        assert_eq!(read_chunked(&mut trailers, Vec::new()).unwrap(), b"a");
    }

    #[test]
    fn fixed_body_rejects_malformed_lengths_and_accepts_equal_duplicates() {
        let response = |lengths| {
            let msg = format!("HTTP/1.1 200 OK\r\n{lengths}\r\n\r\nabc trailing");
            read_response(&mut msg.as_bytes(), false)
        };
        assert!(response("Content-Length: invalid").is_err());
        assert!(response("Content-Length: 3\r\nContent-Length: 4").is_err());
        let matching = response("Content-Length: 3\r\nContent-Length: 03").unwrap();
        assert_eq!(matching.body, b"abc");
    }

    #[test]
    fn interrupted_close_delimited_body_is_an_error() {
        struct BrokenBody;

        impl Read for BrokenBody {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::TimedOut.into())
            }
        }

        let response = &b"HTTP/1.0 200 OK\r\n\r\npartial"[..];
        assert!(read_response(&mut response.chain(BrokenBody), false).is_err());
    }

    #[test]
    fn traps_change_path_segments_queries_and_bodies() {
        let base = Req {
            method: "POST".into(),
            path: "/div/7/2?name=bob".into(),
            headers: BTreeMap::new(),
            body: Some("{\"a\":1}".into()),
        };
        let t = traps(&[base]);
        assert!(t.iter().any(|r| r.path == "/div/-7/2?name=bob"));
        assert!(t.iter().any(|r| r.path == "/div/7/0?name=bob"));
        assert!(
            t.iter()
                .any(|r| r.path == "/div/7/2?name=%F0%9F%91%8D%F0%9F%8F%BD%20thumbs")
        );
        assert!(
            t.iter()
                .all(|r| r.path.starts_with("/div/") || r.path == "/rust-anything-missing")
        );
        assert!(
            t.iter()
                .any(|r| r.body.as_deref() == Some(app::JSON_TRAPS[0]))
        );
        assert_eq!(percent_decode("a%20b%zz"), "a b%zz");
    }
}
