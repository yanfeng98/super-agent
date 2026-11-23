//! MCP server over stdio: every step as a tool, for any agent.
//!
//! Newline-delimited JSON-RPC 2.0, as the MCP stdio transport specifies. The protocol surface
//! is small (initialize, tools/list, tools/call, prompts/list, prompts/get, ping, progress and
//! cancellation), so it is implemented directly on serde_json instead of pulling in an SDK.
//!
//! Each tools/call runs on its own thread, so ping and notifications/cancelled are answered
//! while a long call runs; one writer thread owns stdout and writes whole lines.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::{analyze, app, bridge, front_door, gate, harness, lift, shadow};

const PROTOCOL_VERSION: &str = "2025-06-18";

pub fn serve() -> Result<()> {
    serve_io(std::io::stdin().lock(), std::io::stdout())
}

/// In-flight tools/call requests by id, each with its cancelled flag.
type Running = Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>;

fn serve_io(input: impl BufRead, output: impl Write + Send + 'static) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel::<Value>();
    let writer = std::thread::spawn(move || -> std::io::Result<()> {
        let mut output = output;
        for message in rx {
            if message.is_null() {
                break;
            }
            let mut line = message.to_string();
            line.push('\n');
            output.write_all(line.as_bytes())?;
            output.flush()?;
        }
        Ok(())
    });
    let running = Running::default();
    let mut workers: Vec<(Arc<AtomicBool>, JoinHandle<()>)> = Vec::new();
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request = match serde_json::from_str::<Value>(&line) {
            Ok(request) => request,
            Err(e) => {
                let _ = tx.send(error(Value::Null, -32700, &format!("parse error: {e}")));
                continue;
            }
        };
        match request["method"].as_str() {
            Some("tools/call") if request.get("id").is_some() => {
                workers.retain(|(_, worker)| !worker.is_finished());
                workers.push(spawn_call(request, tx.clone(), running.clone()));
            }
            Some("notifications/cancelled") => {
                let key = request["params"]["requestId"].to_string();
                if let Some(flag) = lock(&running).get(&key) {
                    flag.store(true, Ordering::SeqCst);
                }
            }
            _ => {
                if let Some(response) = handle(&request) {
                    let _ = tx.send(response);
                }
            }
        }
    }
    for (cancelled, worker) in workers {
        if !cancelled.load(Ordering::SeqCst) {
            let _ = worker.join();
        }
    }
    let _ = tx.send(Value::Null);
    writer
        .join()
        .map_err(|_| anyhow::anyhow!("the MCP writer thread panicked"))??;
    Ok(())
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Runs one tools/call on its own thread. A cancelled request gets no response and no further
/// progress; the check and the send happen under the `running` lock, so a cancellation lands
/// either before the response (nothing is sent) or after it (the request is already gone).
fn spawn_call(
    request: Value,
    tx: Sender<Value>,
    running: Running,
) -> (Arc<AtomicBool>, JoinHandle<()>) {
    let key = request["id"].to_string();
    let cancelled = Arc::new(AtomicBool::new(false));
    lock(&running).insert(key.clone(), cancelled.clone());
    let flag = cancelled.clone();
    let worker = std::thread::spawn(move || {
        let token = progress_token(&request);
        let name = request["params"]["name"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        if let Some(token) = &token {
            let _ = tx.send(progress(token, 0, &format!("running {name}")));
        }
        let response = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle(&request)))
            .unwrap_or_else(|_| {
                Some(error(
                    request["id"].clone(),
                    -32603,
                    &format!("tool `{name}` panicked"),
                ))
            });
        let mut running = lock(&running);
        if running.get(&key).is_some_and(|f| Arc::ptr_eq(f, &flag)) {
            running.remove(&key);
        }
        if flag.load(Ordering::SeqCst) {
            return;
        }
        if let Some(token) = &token {
            let failed = response
                .as_ref()
                .is_none_or(|r| r["result"]["isError"] != false);
            let done = if failed { "failed" } else { "finished" };
            let _ = tx.send(progress(token, 1, &format!("{name} {done}")));
        }
        if let Some(response) = response {
            let _ = tx.send(response);
        }
    });
    (cancelled, worker)
}

/// `params._meta.progressToken` when it is a string or an integer, as the spec requires.
fn progress_token(request: &Value) -> Option<Value> {
    let token = &request["params"]["_meta"]["progressToken"];
    (token.is_string() || token.is_i64() || token.is_u64()).then(|| token.clone())
}

fn progress(token: &Value, done: u64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "notifications/progress",
        "params": { "progressToken": token, "progress": done, "total": 1, "message": message },
    })
}

/// The response to one JSON-RPC message; `None` for notifications.
pub fn handle(request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request["method"].as_str().unwrap_or_default();
    let id = id?;
    let result = match method {
        "initialize" => json!({
            "protocolVersion": request["params"]["protocolVersion"].as_str().unwrap_or(PROTOCOL_VERSION),
            "capabilities": { "tools": {}, "prompts": {} },
            "serverInfo": { "name": "rust-anything", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Incremental TS/JS to Rust migration. Loop per unit: analyze -> bridge -> \
                record -> (port src/port.rs) -> build -> diff -> shadow -> gate. Never mark a port done \
                before gate passes.",
        }),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools() }),
        "tools/call" => {
            let name = request["params"]["name"].as_str().unwrap_or_default();
            let args = &request["params"]["arguments"];
            match call(name, args) {
                Ok(value) => json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&value).unwrap_or_default() }],
                    "structuredContent": value,
                    "isError": false,
                }),
                Err(e) => {
                    let text = format!("{e:#}");
                    json!({
                        "content": [{ "type": "text", "text": text }],
                        "structuredContent": failure(&text, args),
                        "isError": true,
                    })
                }
            }
        }
        "prompts/list" => crate::mcp_prompts::list(),
        "prompts/get" => {
            let params = &request["params"];
            match crate::mcp_prompts::get(
                params["name"].as_str().unwrap_or_default(),
                &params["arguments"],
            ) {
                Ok(prompt) => prompt,
                Err(message) => return Some(error(id, -32602, &message)),
            }
        }
        _ => return Some(error(id, -32601, &format!("unknown method `{method}`"))),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// One kind of tool failure: a stable code, whether the same call may succeed when retried,
/// and the next step, where `{path}` is the call's crate, repo or root.
struct FailureKind {
    code: &'static str,
    retryable: bool,
    matches: fn(&str) -> bool,
    next: Option<&'static str>,
}

/// Checked in order; the first match wins, `failed` otherwise.
const FAILURES: &[FailureKind] = &[
    FailureKind {
        code: "timed_out",
        retryable: true,
        matches: |t| t.contains("still running after"),
        next: Some(
            "retry; if it times out again, restart the MCP server with RUST_ANYTHING_COMMAND_TIMEOUT set higher",
        ),
    },
    FailureKind {
        code: "no_recording",
        retryable: false,
        matches: |t| {
            (t.contains("no recorded cases") || t.contains("no cases in"))
                && t.contains("app record` first")
        },
        next: Some("rust-anything app record {path} -- <command that runs the original>"),
    },
    FailureKind {
        code: "no_recording",
        retryable: false,
        matches: |t| t.contains("no recorded cases"),
        next: Some("rust-anything record {path}"),
    },
    FailureKind {
        code: "stale_recording",
        retryable: false,
        matches: |t| t.contains("changed since it was recorded"),
        next: Some("rust-anything record {path}"),
    },
    FailureKind {
        code: "stale_build",
        retryable: false,
        matches: |t| t.contains("is older than the crate's sources"),
        next: Some("rust-anything build {path}"),
    },
    FailureKind {
        code: "build_failed",
        retryable: false,
        matches: |t| t.contains("cargo build") && t.contains("failed"),
        next: Some("fix the compiler errors in the message, then rust-anything build {path}"),
    },
    FailureKind {
        code: "invalid_arguments",
        retryable: false,
        matches: |t| t.starts_with("missing `") && !t.contains('\n') && !t.contains(": "),
        next: None,
    },
    FailureKind {
        code: "unknown_tool",
        retryable: false,
        matches: |t| t.starts_with("unknown tool `"),
        next: Some("call tools/list for the tool names"),
    },
];

/// The structuredContent of a failed tool call.
fn failure(text: &str, args: &Value) -> Value {
    let kind = FAILURES.iter().find(|k| (k.matches)(text));
    let path = ["crate", "repo", "root"]
        .iter()
        .find_map(|key| args[key].as_str())
        .unwrap_or("<crate>");
    json!({
        "code": kind.map_or("failed", |k| k.code),
        "message": text,
        "retryable": kind.is_some_and(|k| k.retryable),
        "next": kind.and_then(|k| k.next).map(|n| n.replace("{path}", path)),
    })
}

/// How a tool behaves, for the MCP tool annotations.
struct ToolMeta {
    name: &'static str,
    title: &'static str,
    read_only: bool,
    destructive: bool,
    idempotent: bool,
    open_world: bool,
}

const fn meta(
    name: &'static str,
    title: &'static str,
    [read_only, destructive, idempotent, open_world]: [bool; 4],
) -> ToolMeta {
    ToolMeta {
        name,
        title,
        read_only,
        destructive,
        idempotent,
        open_world,
    }
}

/// `[readOnly, destructive, idempotent, openWorld]` per tool. Tools that run the original's
/// code, test commands or cargo are open-world; only app_lift rewrites files the user wrote.
const TOOL_META: &[ToolMeta] = &[
    meta("analyze", "Analyze repository", [true, false, true, false]),
    meta(
        "bridge",
        "Scaffold unit crate",
        [false, false, false, false],
    ),
    meta(
        "refresh",
        "Regenerate bindings",
        [false, false, true, false],
    ),
    meta(
        "record",
        "Record original behavior",
        [false, false, false, true],
    ),
    meta("build", "Build crate", [false, false, true, true]),
    meta(
        "diff",
        "Diff port against recording",
        [false, false, true, true],
    ),
    meta("profile", "Profile workload", [false, false, true, true]),
    meta(
        "go",
        "Port the hot path end to end",
        [false, false, false, true],
    ),
    meta("enable", "Switch a unit on", [false, false, true, false]),
    meta("disable", "Switch a unit off", [false, false, true, false]),
    meta("report", "Write a report card", [false, false, true, true]),
    meta("bench", "Benchmark port", [false, false, true, true]),
    meta(
        "fuzz",
        "Fuzz port against original",
        [false, false, false, true],
    ),
    meta(
        "shadow",
        "Generate shadow module",
        [false, false, false, true],
    ),
    meta(
        "gate",
        "Check definition of done",
        [false, false, true, true],
    ),
    meta("map", "Map the port", [true, false, true, false]),
    meta("package", "Lay out packages", [false, false, true, false]),
    meta(
        "app_record",
        "Record whole app",
        [false, false, false, true],
    ),
    meta("app_replay", "Replay app cases", [false, false, true, true]),
    meta(
        "app_front_door",
        "Scaffold Rust front door",
        [false, false, false, true],
    ),
    meta("app_wire", "Wire unit into app", [false, false, true, true]),
    meta(
        "app_lift",
        "Lift to pure Rust app",
        [false, true, false, true],
    ),
];

fn tools() -> Value {
    let mut tools = tool_schemas();
    for tool in tools.as_array_mut().into_iter().flatten() {
        let name = tool["name"].as_str().unwrap_or_default();
        if let Some(m) = TOOL_META.iter().find(|m| m.name == name) {
            tool["title"] = json!(m.title);
            tool["annotations"] = json!({
                "title": m.title,
                "readOnlyHint": m.read_only,
                "destructiveHint": m.destructive,
                "idempotentHint": m.idempotent,
                "openWorldHint": m.open_world,
            });
        }
    }
    tools
}

fn tool_schemas() -> Value {
    let crate_dir = json!({ "type": "string", "description": "Folder of the generated crate (holds rust-anything.json)." });
    json!([
        {
            "name": "analyze",
            "description": "Map a repository (TypeScript/JavaScript, Python, C, Go, Ruby, Java or Kotlin): units, port order (waves), risks with file:line, bridge per unit, dependency-to-crate fit, and `start_here`: the best first ports.",
            "inputSchema": { "type": "object", "properties": {
                "root": { "type": "string", "description": "Repository root." },
                "lang": { "type": "string", "enum": ["ts", "python", "c", "go", "ruby", "java", "kotlin", "php", "csharp"], "description": "Source language; detected from the repository when omitted." },
                "whole_app": { "type": "boolean", "description": "Plan a whole-app port: entry points and shape (cli, server, library), every dependency imported or declared with its crate and fit, pieces in port order (units that import each other together), and numbered decisions with a recommended answer for the grilling round." },
            }, "required": ["root"] },
        },
        {
            "name": "bridge",
            "description": "Scaffold a Rust crate for one unit: napi-rs or wasm-bindgen bindings in src/lib.rs, stubs to port in src/port.rs, UTF-16 helpers when the unit uses string offsets. Never overwrites an existing folder.",
            "inputSchema": { "type": "object", "properties": {
                "root": { "type": "string" },
                "unit": { "type": "string", "description": "Unit id from analyze." },
                "kind": { "type": "string", "enum": ["napi", "wasm"], "description": "Defaults to the plan's bridge." },
                "out": { "type": "string", "description": "Crate folder; defaults to <root>/rust/<name>." },
                "name": { "type": "string", "description": "Crate name; defaults to `<unit>-rs`." },
                "force": { "type": "boolean", "description": "Bridge a unit the plan marks blocked or keep." },
                "keep": { "type": "array", "items": { "type": "string" }, "description": "Exports to keep in the source language: `name` or `name: reason`." },
                "batch": { "type": "array", "items": { "type": "string" }, "description": "Batch classes, `function:param`: the function runs over many `param` values kept in Rust, on all cores, in one call. A `<Function>Index` struct in port.rs replaces the default per-item loop." },
                "as_json": { "type": "array", "items": { "type": "string" }, "description": "Type parameters the app only fills with data (`T` in `paginate<T>(items: T[])`): bridged as JSON values instead of keeping the export in TS." },
                "allocator": { "type": "string", "enum": ["mimalloc", "rusty_alloc"], "description": "Global allocator for crates with batches. mimalloc (default) is C and leaves wasm alone; rusty_alloc is pure Rust (no C compiler) and also serves wasm." },
            }, "required": ["root", "unit"] },
        },
        {
            "name": "refresh",
            "description": "Regenerate a crate's bindings (lib.rs, index.cjs, js.rs) from the signatures in src/port.rs, after changing them, adding any new batch classes. Never touches port.rs.",
            "inputSchema": { "type": "object", "properties": {
                "crate": crate_dir,
                "batch": { "type": "array", "items": { "type": "string" }, "description": "Batch classes to add, `function:param`: the function runs over many `param` values kept in Rust, on all cores, in one call. A `<Function>Index` struct in port.rs replaces the default per-item loop." },
            }, "required": ["crate"] },
        },
        {
            "name": "record",
            "description": "Run the original functions on generated edge-case inputs (non-ASCII, emoji, -0, NaN, ...) plus optional extra inputs, and save the results as cases.jsonl: the behavior the port must match.",
            "inputSchema": { "type": "object", "properties": {
                "crate": crate_dir,
                "count": { "type": "integer", "description": "Generated cases per function (default 200)." },
                "inputs": { "type": "string", "description": "JSONL file of extra cases: {\"fn\": name, \"args\": [...]}." },
                "module": { "type": "string", "description": "JS only: import the original from this JS module instead of the TS source." },
                "resolve_from": { "type": "string", "description": "Resolve npm packages from this folder's node_modules." },
                "float_tolerance": { "type": "number", "description": "Numeric code: floats match when they differ by at most this times the larger magnitude (absolute below 1), e.g. 1e-9. Saved with the crate for diff and gate; 0 removes it." },
                "node_args": { "type": "array", "items": { "type": "string" }, "description": "Extra arguments for node, e.g. [\"--import=tsx\"]." },
                "test_command": { "type": "array", "items": { "type": "string" }, "description": "TS/JS or Python: also record every call the original's own test suite makes to the unit's functions, e.g. [\"npx\", \"jest\"] or [\"pytest\"]. Runs in the original's folder." },
                "profile": { "type": "string", "enum": ["host", "android"], "description": "Record on this machine (default) or on a connected Android device (JVM crates)." },
                "device": { "type": "string", "description": "Android device serial; picked automatically when only one is connected." },
                "pin_clock": { "type": "boolean", "description": "TS/JS (napi) and Python: true pins the clock and random numbers while recording and diffing (`Date.now()`, `new Date()`, `Math.random`; `time.time`, `datetime.now`, `random`) to the values in pinned.json, and writes src/port/pinned.rs for the port to read them (`mod pinned;` in port.rs). Kept for later runs; false stops pinning." },
            }, "required": ["crate"] },
        },
        {
            "name": "build",
            "description": "Build the crate: the napi-rs .node, wasm-bindgen output, Python extension or shared library. With `profile: \"android\"`, build the JNI library for Android ABIs into `jniLibs/` (needs the NDK and the Rust Android targets; returned as {ndk, api, libraries} instead of {artifact}).",
            "inputSchema": { "type": "object", "properties": {
                "crate": crate_dir,
                "profile": { "type": "string", "enum": ["host", "android"], "description": "Build for this machine (default) or for Android." },
                "abi": { "type": "array", "items": { "type": "string", "enum": ["arm64-v8a", "armeabi-v7a", "x86_64", "x86"] }, "description": "Android ABIs; defaults to arm64-v8a, armeabi-v7a and x86_64." },
                "api": { "type": "integer", "description": "Android API level (at least 26; default 26)." },
                "ndk": { "type": "string", "description": "Android NDK root; found from ANDROID_NDK_HOME or the SDK when omitted." },
                "jni_libs": { "type": "string", "description": "Where to install the Android libraries; defaults to <crate>/jniLibs." },
            }, "required": ["crate"] },
        },
        {
            "name": "diff",
            "description": "Replay cases.jsonl against the built Rust module and list mismatches. With `profile: \"android\"`, replay on the device against the .so built for its ABI.",
            "inputSchema": { "type": "object", "properties": {
                "crate": crate_dir,
                "profile": { "type": "string", "enum": ["host", "android"], "description": "Diff on this machine (default) or on a connected Android device (JVM crates)." },
                "device": { "type": "string", "description": "Android device serial; picked automatically when only one is connected." },
                "jni_libs": { "type": "string", "description": "Android libraries folder to diff; defaults to <crate>/jniLibs." },
                "ignore_key_order": { "type": "boolean" },
                "module": { "type": "string", "description": "Test this JS module instead of the crate's `index.cjs`." },
                "node_args": { "type": "array", "items": { "type": "string" }, "description": "Extra arguments for node, e.g. [\"--import=tsx\"]." },
                "float_tolerance": { "type": "number", "description": "Floats match within this tolerance for this run (the one saved by record otherwise)." },
            }, "required": ["crate"] },
        },
        {
            "name": "profile",
            "description": "Run a real workload (a command: node or python3 script, `go test -bench`, java, ruby) under the language's profiler and rank the repository's hot functions by inclusive time, with their callers, call counts and advice: what to port together, and which loops to batch (`bridge` with `batch`). Profile before porting: leaf functions called one at a time are slower in Rust.",
            "inputSchema": { "type": "object", "properties": {
                "root": { "type": "string" },
                "command": { "type": "array", "items": { "type": "string" }, "description": "The workload, run in root: [\"node\", \"bench.mjs\"]." },
                "lang": { "type": "string", "enum": ["ts", "python", "go", "java", "ruby"], "description": "Detected from the repository when omitted." },
            }, "required": ["root", "command"] },
        },
        {
            "name": "go",
            "description": "The hot-path loop in one call: profile the workload, pick the top portable function (the outermost loop; batched where profile says so), bridge it into rust/<function>-rs, record generated cases plus the calls the workload makes, write TASK.md, hand the port to `agent` (claude, codex, or a command reading the prompt on stdin), then build, diff and gate, and switch the unit on with `enable` only when every case matches, gate passes and `bench` shows it is not slower as its callers call it. Without `agent` it stops at `waiting_for_port`: port src/port.rs as TASK.md says and call go again; finished steps are kept. Returns the stage (waiting_for_port, mismatch, gate_failed, slower, on, passed), the speedup, the pick, the steps and what to do next.",
            "inputSchema": { "type": "object", "properties": {
                "root": { "type": "string", "description": "Repository root." },
                "command": { "type": "array", "items": { "type": "string" }, "description": "The workload, run in root: [\"python3\", \"app.py\"]." },
                "agent": { "type": "string", "description": "claude, codex, or a command that reads a prompt on stdin. Omit to port it yourself." },
                "function": { "type": "string", "description": "Port this hot function instead of the profile's pick." },
                "lang": { "type": "string", "enum": ["ts", "python", "go", "java", "ruby"], "description": "Detected from the repository when omitted." },
                "count": { "type": "integer", "description": "Generated cases per function (default 200)." },
                "rounds": { "type": "integer", "description": "Agent turns: the port, then one per failing diff or gate (default 3)." },
            }, "required": ["root", "command"] },
        },
        {
            "name": "enable",
            "description": "Switch a ported TS/JS or Python unit on: one marked block in each original source file rebinds the unit's functions to the built Rust module, for every caller, with no import changed. RUST_ANYTHING_MODE=old or RUST_ANYTHING_UNIT_<NAME>=original runs the original for one run. Refuses while recorded cases differ unless `force`.",
            "inputSchema": { "type": "object", "properties": { "crate": crate_dir, "force": { "type": "boolean" } }, "required": ["crate"] },
        },
        {
            "name": "disable",
            "description": "Switch a unit off: removes exactly the lines enable added, leaving the source files byte for byte as they were.",
            "inputSchema": { "type": "object", "properties": { "crate": crate_dir }, "required": ["crate"] },
        },
        {
            "name": "report",
            "description": "Write <crate>/report/REPORT.md and card.svg for a pull request or README: the unit, recorded cases matched (diff), speedup call included (bench), fallback rate, on or off, and the machine with its core count. Builds first; every number comes from this run.",
            "inputSchema": { "type": "object", "properties": {
                "crate": crate_dir,
                "out": { "type": "string", "description": "Output folder; defaults to <crate>/report." },
                "min_ms": { "type": "integer", "description": "Minimum measured time per function and side, in milliseconds (default 200)." },
                "node_args": { "type": "array", "items": { "type": "string" }, "description": "Extra arguments for node, e.g. [\"--import=tsx\"]." },
            }, "required": ["crate"] },
        },
        {
            "name": "bench",
            "description": "Time the recorded cases on the original and on the built Rust module, bridge included; speedup below 1 means the port is slower as its callers call it. Returns the rows and a verdict with advice: ship, keep the slower functions, or port a bigger unit.",
            "inputSchema": { "type": "object", "properties": {
                "crate": crate_dir,
                "resolve_from": { "type": "string" },
                "min_ms": { "type": "integer", "description": "Minimum measured time per function and side, in milliseconds (default 200)." },
                "node_args": { "type": "array", "items": { "type": "string" }, "description": "Extra arguments for node, e.g. [\"--import=tsx\"]." },
            }, "required": ["crate"] },
        },
        {
            "name": "fuzz",
            "description": "Live differential test: generate fresh inputs from a new seed (the record generator, without the cases already recorded), run them through the original and the built port, and report mismatches the recording missed. Returns the seed to replay, counts and the first mismatches; `keep` appends mismatching (or all) fresh cases to cases.jsonl so the next diff and gate check them. Works wherever record and diff do on this machine (TS/JS, Python, C, Go, Ruby, Java, Kotlin).",
            "inputSchema": { "type": "object", "properties": {
                "crate": crate_dir,
                "cases": { "type": "integer", "description": "Fresh cases per function (and scenarios per class) in each round (default 200)." },
                "seed": { "type": "integer", "description": "Seed of the first round; random when omitted (returned as `seed`). Later rounds count up from it." },
                "time": { "type": "number", "description": "Keep running new rounds until this many seconds have passed; one round when omitted." },
                "keep": { "type": "string", "enum": ["mismatches", "all"], "description": "Append the mismatching fresh cases, or all of them, to cases.jsonl." },
                "resolve_from": { "type": "string", "description": "Resolve npm packages the original imports from this folder's node_modules." },
                "node_args": { "type": "array", "items": { "type": "string" }, "description": "Extra arguments for node, e.g. [\"--import=tsx\"]." },
            }, "required": ["crate"] },
        },
        {
            "name": "shadow",
            "description": "Generate a drop-in shadow (a TS or Python module, Go package, Ruby file, Java classes or C file) that runs the original and the Rust port side by side and reports mismatches; RUST_ANYTHING_MODE=old|shadow|new switches at runtime.",
            "inputSchema": { "type": "object", "properties": { "crate": crate_dir, "out": { "type": "string" } }, "required": ["crate"] },
        },
        {
            "name": "map",
            "description": "Port map of a repository: each unit of the original with the crate that ports it, its functions in Rust and kept in the original, its cases and last diff, the whole app's glue (Rust only, or still handing cases to the original), the units not ported yet in the order analyze suggests, and every port whose original changed since it was recorded (`changed_upstream`: re-record it and diff). Read-only; the CLI's `map --write` saves it as rust/PORT_MAP.md.",
            "inputSchema": { "type": "object", "properties": {
                "repo": { "type": "string", "description": "Repository root." },
                "crates": { "type": "array", "items": { "type": "string" }, "description": "Folders with crates kept outside the repository." }
            }, "required": ["repo"] },
        },
        {
            "name": "package",
            "description": "Lay out installable packages for a gated port; writes files only, never builds other platforms or publishes. PyO3: a maturin project (abi3 wheel importable under the original module's name). CLI front door (`rust/app` of a Node CLI): a main npm package whose command runs `<name>-<platform>`'s front door binary, with the original pinned to its exact version as a dependency for every case the port hands back. napi: a main npm package whose loader picks `<name>-<platform>` from optionalDependencies and falls back to the original JS, one package per platform (this machine's binary copied in when built). wasm: one npm package with the built pkg/. PyO3 and napi also get a GitHub Actions workflow building macOS (arm64, x86_64), Linux (x86_64, aarch64) and Windows.",
            "inputSchema": { "type": "object", "properties": {
                "crate": crate_dir,
                "name": { "type": "string", "description": "Package name on npm or PyPI; defaults to the crate name." },
                "version": { "type": "string", "description": "Package version; defaults to the original package's." },
                "out": { "type": "string", "description": "Output folder; defaults to <crate>/package." },
                "fallback": { "type": "string", "description": "napi: module to load where no binary fits; defaults to the original file when it is plain JS that imports nothing." },
                "bin": { "type": "array", "items": { "type": "string" }, "description": "CLI front door: the commands the package installs; defaults to the package name without its scope." },
                "original": { "type": "string", "description": "CLI front door: the original npm package it falls back to, as <name>@<version>; defaults to the one in the repository's node_modules." },
                "dry_run": { "type": "boolean", "description": "List the files, write nothing." },
            }, "required": ["crate"] },
        },
        {
            "name": "gate",
            "description": "Definition of done: fmt, clippy -D warnings, cargo test, unsafe with SAFETY comments only, no unimplemented functions, build, API present, recorded behavior matches.",
            "inputSchema": { "type": "object", "properties": { "crate": crate_dir, "node_args": { "type": "array", "items": { "type": "string" }, "description": "Extra arguments for node, e.g. [\"--import=tsx\"]." } }, "required": ["crate"] },
        },
        {
            "name": "app_record",
            "description": "Record the whole CLI app's behavior: run the original on its own documented examples, an inputs file and semantic traps (non-ASCII and emoji, negative division, overflow, float extremes, map order), twice each. What differs between runs is reported by kind in `noise`: values (now [..] wildcards), order (lines compared in any order) and lines (a ... that matches anything): show every one to the user. Cases are trycmd files in <repo>/rust/app-cases/ (argv, stdin, env, files in; stdout, stderr, exit code, files out).",
            "inputSchema": { "type": "object", "properties": {
                "repo": { "type": "string", "description": "Repository root." },
                "run": { "type": "array", "items": { "type": "string" }, "description": "Command that runs the original, paths relative to repo: [\"python3\", \"cli.py\"]. Optional after the first record." },
                "inputs": { "type": "string", "description": "JSONL of extra cases: {\"name\", \"args\": [...], \"stdin\", \"env\": {...}, \"files\": {path: text}}." },
                "normalize": { "type": "array", "items": { "type": "string" }, "description": "Normalizers: timestamps, uuids, key-order, or name=regex." },
                "name": { "type": "string", "description": "Name the app is invoked by in its docs." },
                "timeout": { "type": "integer", "description": "Seconds before a run is killed (default 30)." },
                "traps": { "type": "boolean", "description": "Add semantic-trap cases (default true)." },
                "http": { "type": "boolean", "description": "The app is a server: `run` starts it (it takes its port from PORT or {port}); cases are requests from curl lines in its docs, `inputs` ({\"name\", \"method\", \"path\", \"headers\", \"body\"}) and traps." },
                "port": { "type": "integer", "description": "A port the server always listens on (implies http)." },
                "effects": { "type": "array", "items": { "type": "string" }, "description": "Record what leaves the process: \"http\" (a local sink set as HTTP_PROXY, http_proxy and RUST_ANYTHING_HTTP_SINK) and \"smtp\" (a local mail server, SMTP_HOST and SMTP_PORT). Each case stores what reached them, in order, and replay compares it." },
                "effect_env": { "type": "array", "items": { "type": "string" }, "description": "Point the app's own settings at a sink: VAR=http-url, VAR=smtp-host, VAR=smtp-port or VAR=smtp-url." },
                "db_reset": { "type": "string", "description": "Shell command that resets the database before each case (each start for servers); cases then run one at a time." },
                "db_dump": { "type": "string", "description": "Shell command that prints the database's state (e.g. `sqlite3 app.db .dump`), run after each case or request and recorded with the effects." },
                "args": { "type": "array", "items": { "type": "string" }, "description": "Servers: arguments every start gets after its command, the original's and every build's ([\"db.json\"]). Implies http; saved." },
                "fresh": { "type": "array", "items": { "type": "string" }, "description": "Repository files or folders the app reads or writes (a project tree, a database file), snapshotted and copied into a fresh working directory before every case (CLIs) or start (servers); a CLI's removed files are compared too. Saved." },
                "pace": { "type": "integer", "description": "Servers: milliseconds to wait after each response, for work the server does after answering (a file watcher). Implies http; saved and used by replay; 0 removes it." },
                "float_tolerance": { "type": "number", "description": "Numeric output: numbers match when they differ by at most this times the larger magnitude (absolute below 1), e.g. 1e-9. Saved with the cases; 0 removes it." },
                "float_file": { "type": "array", "items": { "type": "string" }, "description": "Written files of raw floats, compared value by value within float_tolerance: `PATH=f32le` or `PATH=f64le`. Saved with the cases." },
            }, "required": ["repo"] },
        },
        {
            "name": "app_replay",
            "description": "Replay <repo>/rust/app-cases/ against a build (the original, the front door, the final Rust binary): passed/total, fallback rate when the build reports it, and a report with a diff per failing case (rust/app-cases/replay-report.md).",
            "inputSchema": { "type": "object", "properties": {
                "repo": { "type": "string", "description": "Repository root." },
                "build": { "type": "string", "description": "Command line of the build to test; the original when omitted." },
                "port": { "type": "integer", "description": "Servers: the port the build always listens on; otherwise it gets a free one through PORT and {port}." },
                "timeout": { "type": "integer", "description": "Seconds before a run is killed (default 30)." },
                "case": { "type": "string", "description": "Replay only this case, by the name the report gives it (a library: only this unit): the fast loop for fixing one mismatch." },
            }, "required": ["repo"] },
        },
        {
            "name": "app_front_door",
            "description": "Scaffold <repo>/rust/app, the Rust binary that owns the CLI's entry point: a clap parser for the recorded commands and flags, every command running the original with the same argv, stdin and env (each fallback counted). For a server: an axum app that starts the original on a port of its own and proxies every route no unit answers. Builds it and replays the cases: 100% passed at 100% fallback. Needs app_record first; never overwrites rust/app.",
            "inputSchema": { "type": "object", "properties": {
                "repo": { "type": "string", "description": "Repository root." },
                "check": { "type": "boolean", "description": "Build and replay (default true)." },
                "library": { "type": "boolean", "description": "Treat the repository as a library even if it has a main: the front door is a Rust crate with the public API, re-exporting each wired unit's port. Found by default when there is no main and no server." },
            }, "required": ["repo"] },
        },
        {
            "name": "app_wire",
            "description": "After a unit passes gate, hand its commands to its port.rs: adds the unit crate to rust/app as a library, writes a glue stub (src/units/<unit>.rs, yours to write: parse args, call <crate>::port, print what the original prints), regenerates the routing with a kill switch RUST_ANYTHING_UNIT_<NAME>=original, then builds and replays. Run it again after writing the glue: 100% passed and a lower fallback rate.",
            "inputSchema": { "type": "object", "properties": {
                "repo": { "type": "string", "description": "Repository root." },
                "unit": { "type": "string", "description": "Unit crate folder, its name under rust/, or the unit id it was bridged from." },
                "commands": { "type": "array", "items": { "type": "string" }, "description": "Commands the unit answers; kept from the last wire when omitted." },
                "routes": { "type": "array", "items": { "type": "string" }, "description": "Servers: routes the unit answers, axum syntax: \"GET /users/{id}\", \"/path\" for any method, \"*\" for everything else; kept from the last wire when omitted." },
                "skip_gate": { "type": "boolean", "description": "Do not run gate on the unit first." },
                "check": { "type": "boolean", "description": "Build and replay (default true)." },
            }, "required": ["repo", "unit"] },
        },
        {
            "name": "app_lift",
            "description": "Make rust/app a pure Rust app. Refuses (and lists every blocker) unless replay passes with fallback rate 0 and every unit passes gate. Then, on a copy first: removes the fallback and kill switches, turns each unit into a plain library (no cdylib, bindings or generated wrappers), adds a trycmd test running rust/app-cases on cargo test, checks fmt/clippy/tests, DECISIONS.md targets and assets, and replays the pure binary. Never touches the original.",
            "inputSchema": { "type": "object", "properties": {
                "repo": { "type": "string", "description": "Repository root." },
            }, "required": ["repo"] },
        },
    ])
}

/// `bridge`'s `allocator`: the default when omitted (or null), an error for any other value
/// than the two names, so a mistyped argument never bridges with an allocator nobody chose.
fn allocator_arg(args: &Value) -> Result<bridge::Allocator> {
    Ok(match args.get("allocator") {
        None | Some(Value::Null) => bridge::Allocator::default(),
        Some(Value::String(s)) if s == "mimalloc" => bridge::Allocator::Mimalloc,
        Some(Value::String(s)) if s == "rusty_alloc" => bridge::Allocator::RustyAlloc,
        Some(other) => anyhow::bail!("`allocator` is \"mimalloc\" or \"rusty_alloc\", not {other}"),
    })
}

fn call(name: &str, args: &Value) -> Result<Value> {
    let path = |key: &str| -> Result<PathBuf> {
        args[key]
            .as_str()
            .map(PathBuf::from)
            .with_context(|| format!("missing `{key}`"))
    };
    let opt_path = |key: &str| args[key].as_str().map(PathBuf::from);
    let strings = |key: &str| -> Vec<String> {
        args[key]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    Ok(match name {
        "analyze" => {
            let root = path("root")?;
            let lang = match args.get("lang") {
                Some(v) if !v.is_null() => serde_json::from_value(v.clone()).context("`lang`")?,
                _ => crate::lang::Lang::detect(&root),
            };
            let mut report = analyze::analyze_lang(&root, lang)?;
            if args["whole_app"].as_bool().unwrap_or(false) {
                report.whole_app = Some(crate::whole_app::whole_app(&root, &report));
            }
            serde_json::to_value(report)?
        }
        "bridge" => {
            let kind = match args["kind"].as_str() {
                Some("napi") => Some(bridge::Kind::Napi),
                Some("wasm") => Some(bridge::Kind::Wasm),
                _ => None,
            };
            let root = path("root")?;
            let allocator = allocator_arg(args)?;
            serde_json::to_value(bridge::bridge(&bridge::Options {
                root: &root,
                unit: args["unit"].as_str().context("missing `unit`")?,
                kind,
                out: opt_path("out"),
                name: args["name"].as_str().map(str::to_string),
                force: args["force"].as_bool().unwrap_or(false),
                keep: strings("keep"),
                lang: None,
                batch: strings("batch"),
                as_json: strings("as_json"),
                allocator,
            })?)?
        }
        "record" => {
            let crate_dir = path("crate")?;
            let profile = args["profile"].as_str().unwrap_or("host");
            if profile == "host" && !args["device"].is_null() {
                anyhow::bail!("`device` needs profile `android`");
            }
            if !matches!(profile, "host" | "android") {
                anyhow::bail!("unknown record profile `{profile}`: use host or android");
            }
            let mut inputs = opt_path("inputs");
            let test_command = strings("test_command");
            let captured = if test_command.is_empty() {
                None
            } else {
                let c = crate::from_tests::capture(&crate_dir, &test_command)?;
                inputs = Some(crate::from_tests::with_own_inputs(
                    &crate_dir,
                    inputs.as_deref(),
                    &c.inputs,
                )?);
                Some(serde_json::json!({
                    "per_function": c.per_function,
                    "not_captured": c.skipped,
                    "tests_passed": c.tests_passed,
                }))
            };
            let module = opt_path("module");
            let resolve_from = opt_path("resolve_from");
            if let Some(t) = args["float_tolerance"].as_f64() {
                let mut manifest = bridge::Manifest::load(&crate_dir)?;
                manifest.float_tolerance = (crate::tolerance::check(t)? > 0.0).then_some(t);
                manifest.save(&crate_dir)?;
            }
            let helper = match args["pin_clock"].as_bool() {
                Some(on) => crate::pin::set(&crate_dir, on)?,
                None => None,
            };
            let opts = harness::RecordOptions {
                crate_dir: &crate_dir,
                count: args["count"]
                    .as_u64()
                    .map_or(200, |n| usize::try_from(n).unwrap_or(200)),
                inputs: inputs.as_deref(),
                module: module.as_deref(),
                resolve_from: resolve_from.as_deref(),
                node_args: &strings("node_args"),
            };
            let device = args["device"].as_str();
            let mut out = serde_json::to_value(match profile {
                "host" => harness::record(&opts)?,
                "android" => harness::record_android(&opts, device)?,
                other => anyhow::bail!("unknown record profile `{other}`: use host or android"),
            })?;
            if let Some(c) = captured {
                out["from_tests"] = c;
            }
            if let Some(helper) = helper {
                out["pinned_helper"] = json!(helper);
            }
            out
        }
        "refresh" => {
            let batch = strings("batch");
            let crate_dir = path("crate")?;
            serde_json::to_value(if batch.is_empty() {
                bridge::refresh(&crate_dir)?
            } else {
                bridge::add_batches(&crate_dir, &batch)?
            })?
        }
        "profile" => {
            let command = strings("command");
            anyhow::ensure!(!command.is_empty(), "missing `command`");
            let lang = match args.get("lang") {
                Some(v) if !v.is_null() => {
                    Some(serde_json::from_value(v.clone()).context("`lang`")?)
                }
                _ => None,
            };
            serde_json::to_value(crate::profile::profile(&path("root")?, &command, lang)?)?
        }
        "build" => {
            let crate_dir = path("crate")?;
            match args["profile"].as_str().unwrap_or("host") {
                "android" => {
                    if !args["abi"].is_null() && !args["abi"].is_array() {
                        anyhow::bail!("`abi` must be an array of Android ABI names");
                    }
                    let api = if args["api"].is_null() {
                        crate::android::DEFAULT_API
                    } else {
                        let v = args["api"]
                            .as_u64()
                            .context("`api` must be a non-negative integer")?;
                        u32::try_from(v).context("`api` is too large")?
                    };
                    serde_json::to_value(harness::build_android(
                        &crate_dir,
                        &crate::android::Options {
                            abis: strings("abi"),
                            api,
                            ndk: opt_path("ndk"),
                            out: opt_path("jni_libs"),
                        },
                    )?)?
                }
                "host" => {
                    if !args["abi"].is_null()
                        || !args["api"].is_null()
                        || !args["ndk"].is_null()
                        || !args["jni_libs"].is_null()
                    {
                        anyhow::bail!("`abi`, `api`, `ndk` and `jni_libs` need profile `android`");
                    }
                    json!({ "artifact": harness::build(&crate_dir)? })
                }
                other => anyhow::bail!("unknown build profile `{other}`: use host or android"),
            }
        }
        "diff" => {
            let crate_dir = path("crate")?;
            let module = opt_path("module");
            let opts = harness::DiffOptions {
                crate_dir: &crate_dir,
                module: module.as_deref(),
                ignore_key_order: args["ignore_key_order"].as_bool().unwrap_or(false),
                node_args: &strings("node_args"),
                float_tolerance: args["float_tolerance"].as_f64(),
            };
            serde_json::to_value(match args["profile"].as_str().unwrap_or("host") {
                "host" => {
                    if !args["device"].is_null() || !args["jni_libs"].is_null() {
                        anyhow::bail!("`device` and `jni_libs` need profile `android`");
                    }
                    harness::diff(&opts)?
                }
                "android" => harness::diff_android(
                    &opts,
                    args["device"].as_str(),
                    args["jni_libs"].as_str().map(Path::new),
                )?,
                other => anyhow::bail!("unknown diff profile `{other}`: use host or android"),
            })?
        }
        "bench" => {
            let crate_dir = path("crate")?;
            let resolve_from = opt_path("resolve_from");
            let rows = harness::bench(&harness::BenchOptions {
                crate_dir: &crate_dir,
                min_ms: args["min_ms"].as_u64().unwrap_or(200),
                resolve_from: resolve_from.as_deref(),
                node_args: &strings("node_args"),
            })?;
            let lang = bridge::Manifest::load(&crate_dir)
                .map(|m| m.language)
                .unwrap_or_default();
            let verdict = harness::verdict(&rows, lang);
            serde_json::json!({ "rows": rows, "verdict": verdict })
        }
        "fuzz" => {
            let crate_dir = path("crate")?;
            let resolve_from = opt_path("resolve_from");
            let keep = match args["keep"].as_str() {
                None => None,
                Some("mismatches") => Some(crate::fuzz::Keep::Mismatches),
                Some("all") => Some(crate::fuzz::Keep::All),
                Some(other) => anyhow::bail!("unknown `keep` `{other}`: use mismatches or all"),
            };
            let time = match args["time"].as_f64() {
                Some(t) if t.is_finite() && t >= 0.0 => Some(std::time::Duration::from_secs_f64(t)),
                Some(_) => anyhow::bail!("`time` takes seconds"),
                None => None,
            };
            serde_json::to_value(crate::fuzz::fuzz(&crate::fuzz::FuzzOptions {
                crate_dir: &crate_dir,
                cases: args["cases"]
                    .as_u64()
                    .map_or(200, |n| usize::try_from(n).unwrap_or(200)),
                seed: args["seed"].as_u64(),
                time,
                keep,
                resolve_from: resolve_from.as_deref(),
                node_args: &strings("node_args"),
            })?)?
        }
        "shadow" => {
            let out = opt_path("out");
            serde_json::to_value(shadow::shadow(&path("crate")?, out.as_deref())?)?
        }
        "map" => {
            let crates: Vec<std::path::PathBuf> =
                strings("crates").into_iter().map(Into::into).collect();
            serde_json::to_value(crate::port_map::map(&path("repo")?, &crates)?)?
        }
        "package" => {
            let crate_dir = path("crate")?;
            serde_json::to_value(crate::package::package(&crate::package::Options {
                crate_dir: &crate_dir,
                name: args["name"].as_str().map(str::to_string),
                version: args["version"].as_str().map(str::to_string),
                out: opt_path("out"),
                dry_run: args["dry_run"].as_bool().unwrap_or(false),
                fallback: args["fallback"].as_str().map(str::to_string),
                bin: strings("bin"),
                original: args["original"].as_str().map(str::to_string),
            })?)?
        }
        "go" => {
            let command = strings("command");
            anyhow::ensure!(!command.is_empty(), "missing `command`");
            let lang = match args.get("lang") {
                Some(v) if !v.is_null() => {
                    Some(serde_json::from_value(v.clone()).context("`lang`")?)
                }
                _ => None,
            };
            let root = path("root")?;
            serde_json::to_value(crate::go::go(&crate::go::Options {
                root: &root,
                command: &command,
                lang,
                agent: args["agent"].as_str(),
                function: args["function"].as_str(),
                count: args["count"]
                    .as_u64()
                    .map_or(200, |n| usize::try_from(n).unwrap_or(200)),
                rounds: args["rounds"]
                    .as_u64()
                    .map_or(3, |n| usize::try_from(n).unwrap_or(3)),
            })?)?
        }
        "enable" => {
            let crate_dir = std::path::absolute(path("crate")?)?;
            if !args["force"].as_bool().unwrap_or(false) {
                harness::build(&crate_dir)?;
                let d = harness::diff(&harness::DiffOptions {
                    crate_dir: &crate_dir,
                    module: None,
                    ignore_key_order: false,
                    node_args: &[],
                    float_tolerance: None,
                })?;
                anyhow::ensure!(
                    d.failed == 0 && d.missing.is_empty(),
                    "{}/{} recorded cases match; switch it on once they all do, or pass force",
                    d.passed,
                    d.total
                );
            }
            serde_json::to_value(crate::enable::enable(&crate_dir)?)?
        }
        "disable" => serde_json::to_value(crate::enable::disable(&std::path::absolute(path(
            "crate",
        )?)?)?)?,
        "report" => {
            let crate_dir = std::path::absolute(path("crate")?)?;
            let out = opt_path("out").map(std::path::absolute).transpose()?;
            serde_json::to_value(crate::card::report(&crate::card::Options {
                crate_dir: &crate_dir,
                out: out.as_deref(),
                min_ms: args["min_ms"].as_u64().unwrap_or(200),
                node_args: &strings("node_args"),
            })?)?
        }
        "gate" => serde_json::to_value(gate::gate(
            Path::new(&path("crate")?),
            &strings("node_args"),
        )?)?,
        "app_record" => {
            let repo = path("repo")?;
            let inputs = opt_path("inputs");
            let effects = crate::effects::EffectsConfig::from_args(
                &strings("effects"),
                &strings("effect_env"),
                args["db_reset"].as_str(),
                args["db_dump"].as_str(),
            )?;
            serde_json::to_value(app::record(&app::RecordOptions {
                repo: &repo,
                run: &strings("run"),
                inputs: inputs.as_deref(),
                normalize: &strings("normalize"),
                name: args["name"].as_str(),
                traps: args["traps"].as_bool().unwrap_or(true),
                timeout_secs: args["timeout"].as_u64(),
                http: args["http"].as_bool().unwrap_or(false),
                port: args["port"].as_u64().and_then(|p| u16::try_from(p).ok()),
                server_args: &strings("args"),
                fresh: &strings("fresh"),
                pace_ms: args["pace"].as_u64(),
                float_tolerance: args["float_tolerance"].as_f64(),
                float_files: &strings("float_file"),
                effects,
            })?)?
        }
        "app_replay" => {
            let repo = path("repo")?;
            serde_json::to_value(app::replay(&app::ReplayOptions {
                repo: &repo,
                build: args["build"].as_str(),
                timeout_secs: args["timeout"].as_u64(),
                port: args["port"].as_u64().and_then(|p| u16::try_from(p).ok()),
                case: args["case"].as_str(),
            })?)?
        }
        "app_front_door" => {
            let repo = path("repo")?;
            serde_json::to_value(front_door::front_door(&front_door::FrontDoorOptions {
                repo: &repo,
                library: args["library"].as_bool().unwrap_or(false),
                check: args["check"].as_bool().unwrap_or(true),
            })?)?
        }
        "app_wire" => {
            let repo = path("repo")?;
            serde_json::to_value(front_door::wire(&front_door::WireOptions {
                repo: &repo,
                unit: args["unit"].as_str().context("missing `unit`")?,
                commands: &strings("commands"),
                routes: &strings("routes"),
                skip_gate: args["skip_gate"].as_bool().unwrap_or(false),
                check: args["check"].as_bool().unwrap_or(true),
            })?)?
        }
        "app_lift" => serde_json::to_value(lift::lift(&lift::LiftOptions {
            repo: &path("repo")?,
        })?)?,
        #[cfg(test)]
        "test_sleep" => {
            std::thread::sleep(std::time::Duration::from_millis(
                args["ms"].as_u64().unwrap_or(0),
            ));
            json!({ "slept": true })
        }
        _ => anyhow::bail!("unknown tool `{name}`"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocator_defaults_only_when_omitted() {
        let pick = |v: Value| allocator_arg(&v);
        assert_eq!(pick(json!({})).unwrap(), bridge::Allocator::Mimalloc);
        assert_eq!(
            pick(json!({ "allocator": null })).unwrap(),
            bridge::Allocator::Mimalloc
        );
        assert_eq!(
            pick(json!({ "allocator": "mimalloc" })).unwrap(),
            bridge::Allocator::Mimalloc
        );
        assert_eq!(
            pick(json!({ "allocator": "rusty_alloc" })).unwrap(),
            bridge::Allocator::RustyAlloc
        );
        for bad in [
            json!(false),
            json!(42),
            json!("jemalloc"),
            json!(["rusty_alloc"]),
        ] {
            let err = pick(json!({ "allocator": bad.clone() })).unwrap_err();
            assert!(
                format!("{err}").contains("\"mimalloc\" or \"rusty_alloc\""),
                "{bad}: {err}"
            );
        }
    }

    #[test]
    fn bridge_refuses_a_bad_allocator_before_writing_anything() {
        let out =
            std::env::temp_dir().join(format!("rust-anything-mcp-alloc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini");
        let err = call(
            "bridge",
            &json!({ "root": root, "unit": "packages/a/src/util.ts", "out": out, "allocator": 42 }),
        )
        .unwrap_err();
        assert!(format!("{err}").contains("`allocator`"), "{err}");
        assert!(!out.exists(), "a refused bridge must not create the crate");
    }

    #[test]
    fn speaks_mcp() {
        let init = handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-06-18" } })).unwrap();
        assert_eq!(init["result"]["serverInfo"]["name"], "rust-anything");
        assert!(
            handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).is_none()
        );
        let list = handle(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" })).unwrap();
        let names: Vec<&str> = list["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "analyze",
                "bridge",
                "refresh",
                "record",
                "build",
                "diff",
                "profile",
                "go",
                "enable",
                "disable",
                "report",
                "bench",
                "fuzz",
                "shadow",
                "map",
                "package",
                "gate",
                "app_record",
                "app_replay",
                "app_front_door",
                "app_wire",
                "app_lift"
            ]
        );
        let bad = handle(&json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "gate", "arguments": {} } })).unwrap();
        assert_eq!(bad["result"]["isError"], true);
        assert_eq!(
            bad["result"]["structuredContent"]["code"],
            "invalid_arguments"
        );
        let unknown = handle(&json!({ "jsonrpc": "2.0", "id": 4, "method": "nope" })).unwrap();
        assert_eq!(unknown["error"]["code"], -32601);
    }

    #[test]
    fn tools_carry_annotations() {
        let list = handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" })).unwrap();
        let tools = list["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), TOOL_META.len());
        for tool in tools {
            assert!(tool["title"].is_string(), "{}", tool["name"]);
            for hint in [
                "readOnlyHint",
                "destructiveHint",
                "idempotentHint",
                "openWorldHint",
            ] {
                assert!(
                    tool["annotations"][hint].is_boolean(),
                    "{} {hint}",
                    tool["name"]
                );
            }
        }
        let by_name =
            |n: &str| tools.iter().find(|t| t["name"] == n).unwrap()["annotations"].clone();
        assert_eq!(by_name("analyze")["readOnlyHint"], true);
        assert_eq!(by_name("analyze")["openWorldHint"], false);
        for n in [
            "record",
            "diff",
            "gate",
            "shadow",
            "bench",
            "fuzz",
            "profile",
            "app_record",
            "app_replay",
        ] {
            assert_eq!(by_name(n)["readOnlyHint"], false, "{n}");
            assert_eq!(by_name(n)["openWorldHint"], true, "{n}");
        }
        assert_eq!(by_name("app_lift")["destructiveHint"], true);
        assert_eq!(by_name("bridge")["destructiveHint"], false);
    }

    #[test]
    fn failures_are_structured() {
        let dir = std::env::temp_dir().join(format!("ra-mcp-missing-{}", std::process::id()));
        let response = handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "gate", "arguments": { "crate": dir } } }))
        .unwrap();
        let result = &response["result"];
        assert_eq!(result["isError"], true);
        let s = &result["structuredContent"];
        assert_eq!(s["message"], result["content"][0]["text"]);
        assert!(s["code"].is_string() && s["retryable"].is_boolean());

        let args = json!({ "crate": "rust/slug" });
        let cases = [
            (
                "no recorded cases to compare; run `rust-anything record` first",
                "no_recording",
                false,
                Some("rust-anything record rust/slug"),
            ),
            (
                "no cases in rust/app-cases: run `rust-anything app record` first",
                "no_recording",
                false,
                Some("rust-anything app record rust/slug -- <command that runs the original>"),
            ),
            (
                "slug changed since it was recorded: re-run `rust-anything record rust/slug`",
                "stale_recording",
                false,
                Some("rust-anything record rust/slug"),
            ),
            (
                "cargo build failed (exit status: 101):\nerror[E0308]",
                "build_failed",
                false,
                Some("fix the compiler errors in the message, then rust-anything build rust/slug"),
            ),
            (
                "`node x.mjs` was still running after 600s and was stopped",
                "timed_out",
                true,
                None,
            ),
            ("missing `crate`", "invalid_arguments", false, None),
            ("something else broke", "failed", false, None),
        ];
        for (text, code, retryable, next) in cases {
            let f = failure(text, &args);
            assert_eq!(f["code"], code, "{text}");
            assert_eq!(f["retryable"], retryable, "{text}");
            if let Some(next) = next {
                assert_eq!(f["next"], next, "{text}");
            }
        }
        assert!(failure("something else broke", &args)["next"].is_null());
    }

    #[test]
    fn serves_prompts() {
        let init =
            handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }))
                .unwrap();
        assert!(init["result"]["capabilities"]["prompts"].is_object());
        let list = handle(&json!({ "jsonrpc": "2.0", "id": 2, "method": "prompts/list" })).unwrap();
        let names: Vec<&str> = list["result"]["prompts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["port_unit", "port_whole_app"]);
        let get = |id: i64, name: &str, arguments: Value| {
            handle(
                &json!({ "jsonrpc": "2.0", "id": id, "method": "prompts/get",
                "params": { "name": name, "arguments": arguments } }),
            )
            .unwrap()
        };
        let unit = get(
            3,
            "port_unit",
            json!({ "path": "/repo", "unit": "src/slug.ts" }),
        );
        let text = unit["result"]["messages"][0]["content"]["text"]
            .as_str()
            .unwrap();
        assert!(text.contains("`src/slug.ts`") && text.contains("/repo"));
        assert!(text.contains("# Migration to Rust with rust-anything"));
        assert!(!text.starts_with("---"));
        let whole = get(4, "port_whole_app", json!({ "path": "/repo" }));
        let text = whole["result"]["messages"][0]["content"]["text"]
            .as_str()
            .unwrap();
        assert!(text.contains("/repo") && text.contains("# Whole app to a pure Rust app"));
        assert_eq!(get(5, "port_unit", json!({}))["error"]["code"], -32602);
        assert_eq!(
            get(6, "nope", json!({ "path": "/repo" }))["error"]["code"],
            -32602
        );
    }

    #[derive(Clone, Default)]
    struct Shared(Arc<Mutex<Vec<u8>>>);

    impl Write for Shared {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            lock(&self.0).extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn answers_ping_during_a_slow_call_and_honors_cancel() {
        let input = [
            json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": { "name": "test_sleep", "arguments": { "ms": 400 }, "_meta": { "progressToken": "p1" } } }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": { "name": "test_sleep", "arguments": { "ms": 5000 }, "_meta": { "progressToken": null } } }),
            json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": { "requestId": 2 } }),
            json!({ "jsonrpc": "2.0", "id": 3, "method": "ping" }),
        ]
        .iter()
        .map(|m| format!("{m}\n"))
        .collect::<String>();
        let out = Shared::default();
        let start = std::time::Instant::now();
        serve_io(std::io::Cursor::new(input), out.clone()).unwrap();
        assert!(start.elapsed() < std::time::Duration::from_secs(3));
        let text = String::from_utf8(lock(&out.0).clone()).unwrap();
        let messages: Vec<Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        let position = |id: i64| messages.iter().position(|m| m["id"] == id);
        assert!(position(3).unwrap() < position(1).unwrap(), "{text}");
        assert_eq!(
            messages[position(1).unwrap()]["result"]["structuredContent"]["slept"],
            true
        );
        assert_eq!(position(2), None, "{text}");
        let progress: Vec<&Value> = messages
            .iter()
            .filter(|m| m["method"] == "notifications/progress")
            .collect();
        assert_eq!(progress.len(), 2, "{text}");
        assert!(
            progress
                .iter()
                .all(|p| p["params"]["progressToken"] == "p1")
        );
        assert_eq!(progress[0]["params"]["progress"], 0);
        assert_eq!(progress[1]["params"]["progress"], 1);
        assert_eq!(progress[1]["params"]["total"], 1);
        assert!(progress[1]["params"]["message"].is_string());
    }
}
