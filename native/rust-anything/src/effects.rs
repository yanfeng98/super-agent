use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub const EXT: &str = "effects";
const IO_TIMEOUT: Duration = Duration::from_secs(5);
const SETTLE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EffectsConfig {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub http: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub smtp: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db_reset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db_dump: Option<String>,
}

const ENV_KINDS: [&str; 4] = ["http-url", "smtp-host", "smtp-port", "smtp-url"];

impl EffectsConfig {
    pub fn is_empty(&self) -> bool {
        !self.http && !self.smtp && self.db_dump.is_none()
    }

    pub fn serial(&self) -> bool {
        self.db_reset.is_some() || self.db_dump.is_some()
    }

    pub fn from_args(
        kinds: &[String],
        env: &[String],
        db_reset: Option<&str>,
        db_dump: Option<&str>,
    ) -> Result<Option<Self>> {
        let mut config = EffectsConfig::default();
        for kind in kinds.iter().flat_map(|k| k.split(',')).map(str::trim) {
            match kind {
                "http" => config.http = true,
                "smtp" => config.smtp = true,
                "" => {}
                other => bail!("unknown effect `{other}`: use http or smtp"),
            }
        }
        for pair in env {
            let (var, kind) = pair
                .split_once('=')
                .with_context(|| format!("--effect-env `{pair}`: give VAR=KIND"))?;
            if !ENV_KINDS.contains(&kind) {
                bail!(
                    "--effect-env `{pair}`: KIND is one of {}",
                    ENV_KINDS.join(", ")
                );
            }
            if kind.starts_with("http") {
                config.http = true;
            } else {
                config.smtp = true;
            }
            config.env.insert(var.to_string(), kind.to_string());
        }
        config.db_reset = db_reset.map(str::to_string);
        config.db_dump = db_dump.map(str::to_string);
        Ok((!config.is_empty() || config.db_reset.is_some()).then_some(config))
    }

    pub fn merge(saved: Option<Self>, given: Option<Self>) -> Option<Self> {
        match (saved, given) {
            (_, Some(g)) => Some(g),
            (s, None) => s,
        }
    }
}

struct Sink {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

pub struct Sinks {
    config: EffectsConfig,
    events: Arc<Mutex<Vec<String>>>,
    active: Arc<AtomicUsize>,
    http: Option<Sink>,
    smtp: Option<Sink>,
}

impl Sinks {
    pub fn start(config: &EffectsConfig) -> Result<Sinks> {
        let events = Arc::new(Mutex::new(Vec::new()));
        let active = Arc::new(AtomicUsize::new(0));
        let http = config
            .http
            .then(|| listen(&events, &active, serve_http))
            .transpose()?;
        let smtp = config
            .smtp
            .then(|| listen(&events, &active, serve_smtp))
            .transpose()?;
        Ok(Sinks {
            config: config.clone(),
            events,
            active,
            http,
            smtp,
        })
    }

    pub fn env(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        if let Some(h) = &self.http {
            let url = format!("http://127.0.0.1:{}", h.port);
            for var in ["RUST_ANYTHING_HTTP_SINK", "HTTP_PROXY", "http_proxy"] {
                out.push((var.to_string(), url.clone()));
            }
        }
        if let Some(s) = &self.smtp {
            out.push(("SMTP_HOST".into(), "127.0.0.1".into()));
            out.push(("SMTP_PORT".into(), s.port.to_string()));
            out.push((
                "RUST_ANYTHING_SMTP_SINK".into(),
                format!("smtp://127.0.0.1:{}", s.port),
            ));
        }
        for (var, kind) in &self.config.env {
            let value = match (kind.as_str(), &self.http, &self.smtp) {
                ("http-url", Some(h), _) => format!("http://127.0.0.1:{}", h.port),
                ("smtp-host", _, Some(_)) => "127.0.0.1".to_string(),
                ("smtp-port", _, Some(s)) => s.port.to_string(),
                ("smtp-url", _, Some(s)) => format!("smtp://127.0.0.1:{}", s.port),
                _ => continue,
            };
            out.push((var.clone(), value));
        }
        out
    }

    pub fn reset(&self, cwd: &Path) -> Result<()> {
        if let Some(cmd) = &self.config.db_reset {
            let out = shell(cmd, cwd, &self.env())?;
            if !out.status.success() {
                bail!(
                    "--db-reset `{cmd}` failed ({}): {}",
                    out.status,
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
        }
        Ok(())
    }

    pub fn take(&self, cwd: &Path) -> Vec<u8> {
        let deadline = Instant::now() + SETTLE;
        while self.active.load(Ordering::SeqCst) > 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut text: String = self
            .events
            .lock()
            .map(|mut e| e.drain(..).collect())
            .unwrap_or_default();
        if let Some(cmd) = &self.config.db_dump {
            text.push_str("db\n");
            match shell(cmd, cwd, &self.env()) {
                Ok(out) => {
                    text.push_str(&String::from_utf8_lossy(&out.stdout));
                    if !out.status.success() {
                        text.push_str(&format!("(db dump failed: {})\n", out.status));
                    }
                }
                Err(e) => text.push_str(&format!("(db dump failed: {e:#})\n")),
            }
        }
        text.into_bytes()
    }
}

impl Drop for Sinks {
    fn drop(&mut self) {
        for sink in [self.http.as_mut(), self.smtp.as_mut()]
            .into_iter()
            .flatten()
        {
            sink.stop.store(true, Ordering::SeqCst);
            if let Some(t) = sink.thread.take() {
                let _ = t.join();
            }
        }
    }
}

fn shell(cmd: &str, cwd: &Path, env: &[(String, String)]) -> Result<crate::process::Output> {
    #[cfg(windows)]
    let mut c = {
        let mut c = Command::new("cmd");
        c.args(["/C", cmd]);
        c
    };
    #[cfg(not(windows))]
    let mut c = {
        let mut c = Command::new("sh");
        c.args(["-c", cmd]);
        c
    };
    c.current_dir(cwd)
        .envs(env.iter().map(|(k, v)| (k, v)))
        .stdin(std::process::Stdio::null());
    crate::process::output(&mut c, crate::process::QUICK)
}

type Handler = fn(TcpStream, &Mutex<Vec<String>>);

fn listen(
    events: &Arc<Mutex<Vec<String>>>,
    active: &Arc<AtomicUsize>,
    handler: Handler,
) -> Result<Sink> {
    let listener = TcpListener::bind("127.0.0.1:0").context("starting an effect sink")?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    let stop = Arc::new(AtomicBool::new(false));
    let (events, active, flag) = (events.clone(), active.clone(), stop.clone());
    let thread = std::thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
                    let (events, active) = (events.clone(), active.clone());
                    active.fetch_add(1, Ordering::SeqCst);
                    std::thread::spawn(move || {
                        handler(stream, &events);
                        active.fetch_sub(1, Ordering::SeqCst);
                    });
                }
                Err(_) => std::thread::sleep(Duration::from_millis(5)),
            }
        }
    });
    Ok(Sink {
        port,
        stop,
        thread: Some(thread),
    })
}

fn push(events: &Mutex<Vec<String>>, event: String) {
    if let Ok(mut e) = events.lock() {
        e.push(event);
    }
}

fn body_text(body: &[u8]) -> String {
    match std::str::from_utf8(body) {
        Ok(t) => t.to_string(),
        Err(_) => format!("({} bytes)", body.len()),
    }
}

fn serve_http(mut stream: TcpStream, events: &Mutex<Vec<String>>) {
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
        return;
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or_default().to_string();
    let mut headers: Vec<(String, String)> = Vec::new();
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h).unwrap_or(0) == 0 {
            break;
        }
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }
    let header = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    };
    if method.eq_ignore_ascii_case("CONNECT") {
        push(events, format!("https CONNECT {target}\n"));
        let _ = stream.write_all(
            b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        return;
    }
    let mut body = Vec::new();
    if header("transfer-encoding").is_some_and(|t| t.eq_ignore_ascii_case("chunked")) {
        loop {
            let mut size = String::new();
            if reader.read_line(&mut size).unwrap_or(0) == 0 {
                break;
            }
            let n = usize::from_str_radix(size.trim().split(';').next().unwrap_or("0"), 16)
                .unwrap_or(0);
            if n == 0 {
                break;
            }
            let mut chunk = vec![0; n + 2];
            if reader.read_exact(&mut chunk).is_err() {
                break;
            }
            chunk.truncate(n);
            body.extend(chunk);
        }
    } else if let Some(n) = header("content-length").and_then(|v| v.parse::<usize>().ok()) {
        body.resize(n, 0);
        if reader.read_exact(&mut body).is_err() {
            body.clear();
        }
    }
    let url = if target.starts_with("http://") || target.starts_with("https://") {
        target
    } else {
        format!("http://{}{target}", header("host").unwrap_or_default())
    };
    let url = match stream.local_addr() {
        Ok(local) => url
            .replace(&format!("127.0.0.1:{}", local.port()), "sink")
            .replace(&format!("localhost:{}", local.port()), "sink"),
        Err(_) => url,
    };
    let mut event = format!("http {method} {url}\n");
    if let Some(ct) = header("content-type") {
        event.push_str(&format!("content-type: {ct}\n"));
    }
    if !body.is_empty() {
        event.push('\n');
        event.push_str(&body_text(&body));
        if !event.ends_with('\n') {
            event.push('\n');
        }
    }
    push(events, event);
    let _ = stream.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
    );
}

fn serve_smtp(stream: TcpStream, events: &Mutex<Vec<String>>) {
    let Ok(mut out) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    let mut say = |line: &str| out.write_all(format!("{line}\r\n").as_bytes()).is_ok();
    if !say("220 rust-anything ESMTP") {
        return;
    }
    let (mut from, mut to) = (String::new(), Vec::<String>::new());
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let cmd = line.trim_end();
        let upper = cmd.to_ascii_uppercase();
        let ok = if upper.starts_with("EHLO") {
            say("250-rust-anything") && say("250 AUTH PLAIN LOGIN")
        } else if upper.starts_with("HELO") {
            say("250 rust-anything")
        } else if upper.starts_with("MAIL FROM:") {
            from = cmd[10..].trim().to_string();
            say("250 OK")
        } else if upper.starts_with("RCPT TO:") {
            to.push(cmd[8..].trim().to_string());
            say("250 OK")
        } else if upper == "DATA" {
            if !say("354 End data with <CR><LF>.<CR><LF>") {
                return;
            }
            let mut data = String::new();
            loop {
                let mut l = String::new();
                if reader.read_line(&mut l).unwrap_or(0) == 0 {
                    return;
                }
                let t = l.trim_end_matches(['\r', '\n']);
                if t == "." {
                    break;
                }
                data.push_str(
                    t.strip_prefix('.')
                        .filter(|_| t.starts_with(".."))
                        .unwrap_or(t),
                );
                data.push('\n');
            }
            push(
                events,
                format!("smtp from {from} to {}\n{data}", to.join(", ")),
            );
            from.clear();
            to.clear();
            say("250 OK queued")
        } else if upper.starts_with("AUTH") {
            say("235 Authentication succeeded")
        } else if upper == "RSET" {
            from.clear();
            to.clear();
            say("250 OK")
        } else if upper == "QUIT" {
            let _ = say("221 Bye");
            return;
        } else if upper.starts_with("STARTTLS") {
            say("454 TLS not available")
        } else {
            say("250 OK")
        };
        if !ok {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn send(port: u16, text: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        s.write_all(text.as_bytes()).unwrap();
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
        out
    }

    #[test]
    fn http_requests_and_mail_become_effects() {
        let config = EffectsConfig::from_args(
            &["http".into(), "smtp".into()],
            &["HOOK=http-url".into()],
            None,
            None,
        )
        .unwrap()
        .unwrap();
        let sinks = Sinks::start(&config).unwrap();
        let env: BTreeMap<String, String> = sinks.env().into_iter().collect();
        let http: u16 = env["HOOK"].rsplit(':').next().unwrap().parse().unwrap();
        let smtp: u16 = env["SMTP_PORT"].parse().unwrap();
        let reply = send(
            http,
            "POST http://hooks.example/new HTTP/1.1\r\nHost: hooks.example\r\nContent-Type: application/json\r\nContent-Length: 7\r\n\r\n{\"a\":1}",
        );
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        let mut mail = TcpStream::connect(("127.0.0.1", smtp)).unwrap();
        mail.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        mail.write_all(
            b"EHLO me\r\nMAIL FROM:<a@x.test>\r\nRCPT TO:<b@y.test>\r\nDATA\r\nSubject: hi\r\n\r\n..dot\r\n.\r\nQUIT\r\n",
        )
        .unwrap();
        let mut out = String::new();
        let _ = mail.read_to_string(&mut out);
        assert!(out.contains("221"), "{out}");
        let text = String::from_utf8(sinks.take(Path::new("."))).unwrap();
        assert_eq!(
            text,
            "http POST http://hooks.example/new\ncontent-type: application/json\n\n{\"a\":1}\n\
             smtp from <a@x.test> to <b@y.test>\nSubject: hi\n\n.dot\n"
        );
        assert!(sinks.take(Path::new(".")).is_empty());
    }

    #[test]
    fn effect_arguments_are_checked() {
        assert!(
            EffectsConfig::from_args(&[], &[], None, None)
                .unwrap()
                .is_none()
        );
        assert!(EffectsConfig::from_args(&["ftp".into()], &[], None, None).is_err());
        assert!(EffectsConfig::from_args(&[], &["X".into()], None, None).is_err());
        let c = EffectsConfig::from_args(&[], &["URL=smtp-url".into()], None, Some("cat db"))
            .unwrap()
            .unwrap();
        assert!(c.smtp && !c.http && c.serial());
    }
}
