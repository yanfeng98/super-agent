//! Small helpers: paths, processes, number formatting.

use anyhow::{Context, Result};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

pub fn absolute(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Path from directory from to to, both absolute.
pub fn relative(from: &Path, to: &Path) -> PathBuf {
    let a: Vec<Component> = from.components().collect();
    let b: Vec<Component> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut out = PathBuf::new();
    for _ in common..a.len() {
        out.push("..");
    }
    for c in &b[common..] {
        out.push(c);
    }
    out
}

/// "python3" from "python3", "/usr/bin/python3" or "C:\\Python\\python.exe", on any OS.
pub fn program_stem(program: &str) -> String {
    let name = program.rsplit(['/', '\\']).next().unwrap_or(program).to_ascii_lowercase();
    name.strip_suffix(".exe").unwrap_or(&name).to_string()
}

pub fn forward_slashes(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// A name safe for Rust crates and Python modules.
pub fn slug(id: &str) -> String {
    id.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect()
}

pub fn output(cmd: &mut Command, what: &str) -> Result<Output> {
    cmd.output().with_context(|| format!("couldn't start {what} ({})", cmd.get_program().to_string_lossy()))
}

/// Like [`output`], but stops the command once `limit` has passed: asks it to close first (WM_CLOSE on Windows,
/// SIGTERM elsewhere), so it can shut down cleanly and write what it has, then ends it for good after `grace`.
/// Also says whether it had to be stopped.
pub fn output_for(cmd: &mut Command, what: &str, limit: Option<Duration>, grace: Duration) -> Result<(Output, bool)> {
    let Some(limit) = limit else { return Ok((output(cmd, what)?, false)) };
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("couldn't start {what} ({})", cmd.get_program().to_string_lossy()))?;
    fn drain<R: Read + Send + 'static>(r: Option<R>) -> std::thread::JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut r) = r {
                let _ = r.read_to_end(&mut buf);
            }
            buf
        })
    }
    let (stdout, stderr) = (drain(child.stdout.take()), drain(child.stderr.take()));
    let started = Instant::now();
    let mut asked: Option<Instant> = None;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        match asked {
            None if started.elapsed() >= limit => {
                asked = Some(if ask_to_close(child.id()) { Instant::now() } else { Instant::now() - grace });
            }
            Some(at) if at.elapsed() >= grace => {
                let _ = child.kill();
                break child.wait()?;
            }
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let out = Output { status, stdout: stdout.join().unwrap_or_default(), stderr: stderr.join().unwrap_or_default() };
    Ok((out, asked.is_some()))
}

/// Ask a process to close the way a user would. False if it can't be asked (a console app on Windows).
fn ask_to_close(pid: u32) -> bool {
    let (program, args) = if cfg!(windows) { ("taskkill", ["/PID"]) } else { ("kill", ["-TERM"]) };
    Command::new(program)
        .args(args)
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub fn combined(out: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

pub fn tail(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.trim_end().lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

pub fn secs(s: f64) -> String {
    if s >= 1.0 {
        format!("{s:.2}s")
    } else if s >= 1e-3 {
        format!("{:.0}ms", s * 1e3)
    } else {
        format!("{:.0}µs", s * 1e6)
    }
}

pub fn nanos(ns: f64) -> String {
    if ns < 1e3 {
        format!("{ns:.0}ns")
    } else if ns < 1e6 {
        format!("{:.1}µs", ns / 1e3)
    } else {
        format!("{:.1}ms", ns / 1e6)
    }
}

pub fn times(x: f64) -> String {
    if x >= 10.0 { format!("{x:.0}×") } else { format!("{x:.1}×") }
}

pub fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths() {
        let r = relative(Path::new("/a/b/c"), Path::new("/a/.fast/lib"));
        assert_eq!(forward_slashes(&r), "../../.fast/lib");
        assert_eq!(forward_slashes(&relative(Path::new("/a"), Path::new("/a/x"))), "x");
    }

    #[test]
    fn formatting() {
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(thousands(12), "12");
        assert_eq!(times(87.4), "87×");
        assert_eq!(times(3.21), "3.2×");
        assert_eq!(slug("text.utils.Lev-1"), "text_utils_lev_1");
        assert_eq!(nanos(350.0), "350ns");
        assert_eq!(secs(0.25), "250ms");
    }
}
