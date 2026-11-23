//! Pinned clock and random numbers (`record --pin-clock`), so units that read them can be
//! recorded and diffed.
//!
//! The setting lives in `pinned.json` next to the recording. While it is there, `record`, `diff`,
//! `fuzz` and `gate` start the original and the port with `RUST_ANYTHING_NOW_MS`,
//! `RUST_ANYTHING_SEED` and `TZ=UTC`. The JS and Python drivers replace the clock (`Date.now()`,
//! `new Date()`; `time.time`, `datetime.now`) with that instant and the random numbers
//! (`Math.random`; the `random` module) with mulberry32 from that seed. Before each case they
//! restart the generator and set `RUST_ANYTHING_CASE`, so a case's numbers do not depend on the
//! cases before it. The port reproduces both with the helper in [`HELPER`].

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::bridge::{Kind, Manifest};
use crate::lang::Lang;

pub const FILE: &str = "pinned.json";
pub const NOW_MS: &str = "RUST_ANYTHING_NOW_MS";
pub const SEED: &str = "RUST_ANYTHING_SEED";
pub const CASE: &str = "RUST_ANYTHING_CASE";
/// Where `record --pin-clock` writes the port's helper (`mod pinned;` in `port.rs`).
pub const HELPER: &str = "src/port/pinned.rs";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pin {
    /// Milliseconds since the Unix epoch that every clock read returns.
    pub now_ms: i64,
    /// The mulberry32 seed every case starts from.
    pub seed: u32,
}

impl Default for Pin {
    fn default() -> Self {
        Self {
            now_ms: 1_700_000_000_000,
            seed: 1,
        }
    }
}

/// Whether pinning reaches this crate's original and port: the Node and Python drivers, with a
/// native port that can read environment variables.
pub fn supported(m: &Manifest) -> bool {
    match m.language {
        Lang::Python => true,
        Lang::Ts => m.kind == Kind::Napi,
        _ => false,
    }
}

/// The crate's pinned values, if it records with a pinned clock.
pub fn load(crate_dir: &Path) -> Result<Option<Pin>> {
    let path = crate_dir.join(FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    serde_json::from_str(&text)
        .map(Some)
        .with_context(|| format!("reading {} (`now_ms`, `seed`)", path.display()))
}

/// Turns pinning on (keeping values already chosen) or off. Turning it on writes the port's
/// helper when the crate has none yet, and returns its path then.
pub fn set(crate_dir: &Path, on: bool) -> Result<Option<PathBuf>> {
    let path = crate_dir.join(FILE);
    if !on {
        if path.exists() {
            std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
        }
        return Ok(None);
    }
    let manifest = Manifest::load(crate_dir)?;
    if !supported(&manifest) {
        bail!(
            "`--pin-clock` covers TS/JS units with the napi bridge and Python units; this crate is \
             {:?} with the {:?} bridge",
            manifest.language,
            manifest.kind
        );
    }
    let pin = load(crate_dir)?.unwrap_or_default();
    std::fs::write(&path, serde_json::to_string_pretty(&pin)? + "\n")
        .with_context(|| format!("writing {}", path.display()))?;
    let helper = crate_dir.join(HELPER);
    if helper.exists() {
        return Ok(None);
    }
    std::fs::create_dir_all(helper.parent().unwrap_or(crate_dir))?;
    std::fs::write(&helper, HELPER_RS).with_context(|| format!("writing {}", helper.display()))?;
    Ok(Some(helper))
}

/// Sets the pinned environment on a driver command, or clears it when the crate is not pinned,
/// so a variable left in the caller's shell cannot change a recording.
pub fn apply(cmd: &mut Command, crate_dir: &Path) -> Result<()> {
    match load(crate_dir)? {
        Some(pin) => {
            cmd.env(NOW_MS, pin.now_ms.to_string())
                .env(SEED, pin.seed.to_string())
                .env("TZ", "UTC")
                .env_remove(CASE);
        }
        None => {
            cmd.env_remove(NOW_MS).env_remove(SEED).env_remove(CASE);
        }
    }
    Ok(())
}

/// Installed in the Node record and diff scripts after the codec: a fixed `Date` and a seeded
/// `Math.random` when `RUST_ANYTHING_NOW_MS` is set, and `pinCase(index)` to call before each case.
pub const PIN_JS: &str = r#"
const pinCase = (() => {
  const now = process.env.RUST_ANYTHING_NOW_MS
  if (now === undefined) return () => {}
  const nowMs = Number(now)
  const seed = Number(process.env.RUST_ANYTHING_SEED ?? 0) >>> 0
  let state = seed
  const RealDate = Date
  class PinnedDate extends RealDate {
    constructor(...args) { super(...(args.length === 0 ? [nowMs] : args)) }
    static now() { return nowMs }
    static [Symbol.hasInstance](v) { return v instanceof RealDate }
  }
  globalThis.Date = new Proxy(PinnedDate, { apply: () => new RealDate(nowMs).toString() })
  Math.random = () => {
    state = (state + 0x6D2B79F5) >>> 0
    let t = state
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
  return (index) => {
    state = seed
    process.env.RUST_ANYTHING_CASE = String(index)
  }
})()
"#;

/// The port's side: `src/port/pinned.rs`, declared with `mod pinned;` in `port.rs`.
pub const HELPER_RS: &str = r#"//! The clock and random numbers the original saw while rust-anything recorded it.
//!
//! Written by `rust-anything record --pin-clock`. Declare it with `mod pinned;` in `port.rs`,
//! then call `pinned::now_ms()` where the original reads the clock and `pinned::random()` where
//! it draws a random number. While `record`, `diff`, `fuzz` and `gate` run, the clock is fixed at
//! `RUST_ANYTHING_NOW_MS` (time zone UTC) and the numbers come from mulberry32 seeded with
//! `RUST_ANYTHING_SEED`, restarted at every case (the drivers change `RUST_ANYTHING_CASE` between
//! cases). Without those variables (in production) the real clock and a clock seed are used.
//!
//! The generator, as the drivers run it, on wrapping `u32` arithmetic:
//!
//! ```text
//! state = state + 0x6D2B79F5
//! t = (state ^ (state >> 15)) * (state | 1)
//! t = t ^ (t + (t ^ (t >> 7)) * (t | 61))
//! next_u32 = t ^ (t >> 14)
//! ```
//!
//! JS `Math.random()` and Python `random.random()` are `next_u32() / 2^32`. Python's
//! `getrandbits(k)` is `next_u32() >> (32 - k)` up to 32 bits and fills wider values 32 bits at a
//! time from the low end; `randint`, `choice`, `shuffle` and the other `random` functions are
//! CPython's own on top of those two, drawing indexes with `randbelow`. `random.seed()` does
//! nothing while pinned. `Date.now()`, `new Date()` and `Date()` read `now_ms()`; `time.time()`,
//! `datetime.now()`, `datetime.utcnow()` and `date.today()` read `now_secs()`.

#![allow(dead_code)]

use std::cell::Cell;
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static STATE: Cell<(Option<u64>, u32, bool)> = const { Cell::new((None, 0, false)) };
}

fn var<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok()?.parse().ok()
}

fn real_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
}

/// Milliseconds since the Unix epoch, like JS `Date.now()`.
pub fn now_ms() -> f64 {
    var::<i64>("RUST_ANYTHING_NOW_MS").map_or_else(|| real_ms().floor(), |ms| ms as f64)
}

/// Seconds since the Unix epoch, like Python `time.time()`.
pub fn now_secs() -> f64 {
    var::<i64>("RUST_ANYTHING_NOW_MS").map_or_else(|| real_ms() / 1000.0, |ms| ms as f64 / 1000.0)
}

/// The next 32 random bits.
pub fn next_u32() -> u32 {
    let seed = var::<u32>("RUST_ANYTHING_SEED");
    let case = var::<u64>("RUST_ANYTHING_CASE");
    STATE.with(|cell| {
        let (last_case, mut state, started) = cell.get();
        if !started || (seed.is_some() && case != last_case) {
            state = seed.unwrap_or_else(|| real_ms().to_bits() as u32);
        }
        state = state.wrapping_add(0x6D2B_79F5);
        let mut t = (state ^ (state >> 15)).wrapping_mul(state | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        cell.set((case, state, true));
        t ^ (t >> 14)
    })
}

/// A float in `[0, 1)`, like JS `Math.random()` and Python `random.random()`.
pub fn random() -> f64 {
    f64::from(next_u32()) / 4_294_967_296.0
}

/// Python's `random.getrandbits(k)`, for `k` up to 64.
pub fn getrandbits(k: u32) -> u64 {
    assert!(k <= 64, "getrandbits takes at most 64 bits here");
    match k {
        0 => 0,
        1..=32 => u64::from(next_u32() >> (32 - k)),
        _ => {
            let low = u64::from(next_u32());
            low | (u64::from(next_u32() >> (64 - k)) << 32)
        }
    }
}

/// CPython's `Random._randbelow(n)`: an integer in `[0, n)`, as `randint`, `randrange`,
/// `choice` and `shuffle` draw them.
pub fn randbelow(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let k = 64 - n.leading_zeros();
    loop {
        let r = getrandbits(k);
        if r < n {
            return r;
        }
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn mulberry32(seed: u32, n: usize) -> Vec<u32> {
        let mut state = seed;
        (0..n)
            .map(|_| {
                state = state.wrapping_add(0x6D2B_79F5);
                let mut t = (state ^ (state >> 15)).wrapping_mul(state | 1);
                t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
                t ^ (t >> 14)
            })
            .collect()
    }

    #[test]
    fn the_js_generator_is_mulberry32_and_restarts_per_case() {
        if std::process::Command::new("node")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let script = format!(
            "{PIN_JS}\npinCase(0)\nconst a = [Math.random(), Math.random(), Math.random()]\n\
             pinCase(1)\nconst b = Math.random()\n\
             console.log(JSON.stringify({{ a: a.map(x => x * 4294967296), b: b * 4294967296, \
             now: Date.now(), date: new Date().getTime(), iso: new Date(0).toISOString(), \
             s: typeof Date(), inst: new Date() instanceof Date, \
             case: process.env.RUST_ANYTHING_CASE }}))"
        );
        let out = std::process::Command::new("node")
            .args(["--input-type=module", "-e", &script])
            .env(NOW_MS, "1700000000000")
            .env(SEED, "7")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let want = mulberry32(7, 3);
        let got: Vec<u64> = v["a"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap() as u64)
            .collect();
        assert_eq!(got, want.iter().map(|&x| u64::from(x)).collect::<Vec<_>>());
        assert_eq!(v["b"].as_f64().unwrap() as u32, want[0]);
        assert_eq!(v["now"], 1_700_000_000_000_i64);
        assert_eq!(v["date"], 1_700_000_000_000_i64);
        assert_eq!(v["iso"], "1970-01-01T00:00:00.000Z");
        assert_eq!(v["s"], "string");
        assert_eq!(v["inst"], true);
        assert_eq!(v["case"], "1");
    }
}
