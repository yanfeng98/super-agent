//! C ABI bridge: drop-in `extern "C"` replacements with the original names and signatures.
//!
//! The port is safe Rust over C's ABI types (`c_int`, `usize`, `f64`, ...), with each
//! `const char *` as `Option<&[u8]>` (`None` for `NULL`). `lib.rs` is the only place with
//! `unsafe`: reading C strings, and exporting unmangled symbols.

use crate::MIN_RUST;
use std::fmt::Write as _;

use crate::bridge::Manifest;
use crate::lang::c::scalar;
use crate::ts::exports::{ExportedFn, Param};

fn is_string(p: &Param) -> bool {
    p.ts_type == "string"
}

/// The ABI type of a C scalar, as Rust spells it.
fn abi(c_type: &str) -> String {
    scalar(c_type).map_or_else(|| "()".to_string(), |(_, _, abi, _)| abi.to_string())
}

fn ret_type(f: &ExportedFn) -> Option<String> {
    let r = f.native_return.as_deref().unwrap_or("void");
    (r != "void").then(|| abi(r))
}

pub fn port_rs(m: &Manifest) -> String {
    let mut out = format!(
        "//! The Rust port of `{}`: safe Rust; `lib.rs` exports each function under its C name.\n//!\n\
         //! Replace each `unimplemented!` with the port of the C function. C has no error channel:\n\
         //! return what the C code returns, and never panic (a panic aborts at the C boundary).\n\n\
         #![forbid(unsafe_code)]\n",
        m.unit
    );
    for f in &m.exports {
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| {
                let ty = if is_string(p) {
                    "Option<&[u8]>".to_string()
                } else {
                    abi(p.native_type.as_deref().unwrap_or("int"))
                };
                format!("{}: {ty}", p.name)
            })
            .collect();
        let ret = ret_type(f).map(|r| format!(" -> {r}")).unwrap_or_default();
        out.push('\n');
        let _ = writeln!(
            out,
            "/// Port of `{}` (`{}:{}`).",
            f.prototype.as_deref().unwrap_or(&f.name),
            f.file,
            f.line
        );
        for p in f.params.iter().filter(|p| is_string(p)) {
            let _ = writeln!(
                out,
                "///\n/// `{}`: the bytes of the C string, without its NUL; `None` for `NULL`.",
                p.name
            );
        }
        let _ = writeln!(out, "pub fn {}({}){ret} {{", f.name, params.join(", "));
        let names: Vec<String> = f.params.iter().map(|p| format!("&{}", p.name)).collect();
        match names.len() {
            0 => {}
            1 => {
                let _ = writeln!(out, "    let _ = {};", names[0]);
            }
            _ => {
                let _ = writeln!(out, "    let _ = ({});", names.join(", "));
            }
        }
        let _ = writeln!(
            out,
            "    unimplemented!(\"port `{}` from {}\")\n}}",
            f.name, f.file
        );
    }
    out
}

pub fn lib_rs(m: &Manifest) -> String {
    let any_string = m.exports.iter().any(|f| f.params.iter().any(is_string));
    let mut out = format!(
        "//! C ABI for `{}`: drop-in replacements with the original names and signatures, generated\n\
         //! by rust-anything from `rust-anything.json`. All of the crate's `unsafe` code lives in this file.\n\n",
        m.unit
    );
    out.push_str("pub mod port;\n");
    if any_string {
        out.push_str(
            r#"
use std::ffi::c_char;

mod __rust_anything_c_abi {
    use std::ffi::{CStr, c_char};

    /// A `const char *` argument as bytes; `None` for `NULL`.
    pub(super) fn c_str<'a>(ptr: *const c_char) -> Option<&'a [u8]> {
        if ptr.is_null() {
            return None;
        }

        // SAFETY: the C API takes NUL-terminated strings that stay valid for the call;
        // NULL is handled above.
        Some(unsafe { CStr::from_ptr(ptr) }.to_bytes())
    }
}
"#,
        );
    }
    for f in &m.exports {
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| {
                let ty = if is_string(p) {
                    "*const c_char".to_string()
                } else {
                    abi(p.native_type.as_deref().unwrap_or("int"))
                };
                format!("{}: {ty}", p.name)
            })
            .collect();
        let args: Vec<String> = f
            .params
            .iter()
            .map(|p| {
                if is_string(p) {
                    format!("crate::__rust_anything_c_abi::c_str({})", p.name)
                } else {
                    p.name.clone()
                }
            })
            .collect();
        let ret = ret_type(f).map(|r| format!(" -> {r}")).unwrap_or_default();
        let prototype = f.prototype.as_deref().unwrap_or(&f.name);
        let _ = writeln!(
            out,
            "\n/// `{prototype}` (`{}:{}`).\n\
             #[cfg(not(feature = \"shadow\"))]\n\
             // SAFETY: exported under the C name it replaces: link it instead of the original object.\n\
             #[unsafe(no_mangle)]\npub extern \"C\" fn {name}({params}){ret} {{\n    port::{name}({args})\n}}\n\n\
             /// `{name}` as `rust_anything_{name}`, for shadow builds beside the original.\n\
             // SAFETY: a name no C code defines.\n\
             #[unsafe(no_mangle)]\npub extern \"C\" fn rust_anything_{name}({params}){ret} {{\n    port::{name}({args})\n}}",
            f.file,
            f.line,
            name = f.name,
            params = params.join(", "),
            args = args.join(", "),
        );
    }
    out
}

/// `include/<lib>.h`: the prototypes, and the `rust_anything_` aliases.
pub fn header(m: &Manifest) -> String {
    let guard = format!("{}_H", m.lib_name().to_uppercase());
    let mut out = format!(
        "/* C API of the Rust port of `{}`, generated by rust-anything. */\n#ifndef {guard}\n#define {guard}\n\n\
         #include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n#include <sys/types.h>\n\n",
        m.unit
    );
    for f in &m.exports {
        let _ = writeln!(out, "{};", f.prototype.as_deref().unwrap_or(&f.name));
    }
    out.push_str("\n/* The same functions under `rust_anything_` names, for shadow runs. */\n");
    for f in &m.exports {
        let prototype = f.prototype.as_deref().unwrap_or(&f.name);
        let _ = writeln!(
            out,
            "{};",
            prototype.replacen(
                &format!("{}(", f.name),
                &format!("rust_anything_{}(", f.name),
                1
            )
        );
    }
    let _ = writeln!(out, "\n#endif /* {guard} */");
    out
}

/// A C shadow file: each function calls the original (compiled renamed) and the Rust port.
pub fn shadow_c(m: &Manifest) -> String {
    let mut out = String::from(
        "/* Generated by rust-anything: runs the C original and the Rust port side by side.\n\
         *\n * Build: compile the original sources with `-D<name>=<name>__original` for each function\n\
         * below (see `shadow/flags.txt`), then link this file, those objects and the Rust library.\n\
         * Build the Rust library with `cargo build --release --features shadow`, so it exports only the\n\
         * `rust_anything_` names. Callers get the original's results; differences are counted and reported\n\
         * on stderr.\n */\n\
         #include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n#include <stdio.h>\n#include <string.h>\n#include <sys/types.h>\n\n\
         long rust_anything_shadow_calls = 0;\nlong rust_anything_shadow_mismatches = 0;\n\n",
    );
    for f in &m.exports {
        let prototype = f.prototype.as_deref().unwrap_or(&f.name);
        let original = prototype.replacen(
            &format!("{}(", f.name),
            &format!("{}__original(", f.name),
            1,
        );
        let rust = prototype.replacen(
            &format!("{}(", f.name),
            &format!("rust_anything_{}(", f.name),
            1,
        );
        let args = f
            .params
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(out, "{original};\n{rust};\n");
        if f.native_return.as_deref() == Some("void") {
            let _ = writeln!(
                out,
                "{prototype} {{\n    {name}__original({args});\n    rust_anything_{name}({args});\n}}\n",
                name = f.name
            );
        } else {
            let ret = f.native_return.as_deref().unwrap_or("int");
            let _ = writeln!(
                out,
                "{prototype} {{\n    {ret} expected = {name}__original({args});\n    {ret} actual = rust_anything_{name}({args});\n\
                 \x20   rust_anything_shadow_calls++;\n    if (memcmp(&expected, &actual, sizeof expected) != 0) {{\n\
                 \x20       rust_anything_shadow_mismatches++;\n        if (rust_anything_shadow_mismatches <= 10)\n\
                 \x20           fprintf(stderr, \"[rust-anything] mismatch in {name}\\n\");\n    }}\n    return expected;\n}}\n",
                name = f.name
            );
        }
    }
    out
}

/// `-D` flags that rename the originals for a shadow build.
pub fn shadow_flags(m: &Manifest) -> String {
    m.exports
        .iter()
        .map(|f| format!("-D{0}={0}__original", f.name))
        .collect::<Vec<_>>()
        .join(" ")
        + "\n"
}

pub fn cargo_toml(m: &Manifest) -> String {
    format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2024\"\nrust-version = \"{MIN_RUST}\"\npublish = false\n\
         description = \"Rust port of `{}`, generated by rust-anything\"\n\n\
         [lib]\ncrate-type = [\"cdylib\", \"staticlib\", \"rlib\"]\n\n\
         [features]\n# Export only the `rust_anything_` names, to link beside the original C code.\nshadow = []\n\n[dependencies]\n\n\
         [profile.release]\nlto = true\n\n[lints.clippy]\nall = {{ level = \"warn\", priority = -1 }}\n\n[lints.rust]\nunused_lifetimes = \"warn\"\ntrivial_numeric_casts = \"warn\"\n",
        m.crate_name, m.unit
    )
}
