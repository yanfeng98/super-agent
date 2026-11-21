//! The catalog of TypeScript/JavaScript behaviors that do not carry over to Rust as-is.

use serde::Serialize;

/// A construct whose JS semantics differ from the obvious Rust translation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Risk {
    StringOffsets,
    RegexSyntax,
    DynamicRegex,
    DynamicCode,
    DynamicTyping,
    OrderedCollections,
    Exceptions,
    Async,
    Generators,
    ClassInheritance,
    Reflection,
    Callbacks,
    NumberSemantics,
    Json,
    /// Unbounded or differently sized integers (Python `int`, C/Go/Java widths and overflow).
    IntegerSemantics,
    /// Floor division and modulo that differ from Rust's (`//`, `%` on negatives, C `/` by zero).
    DivisionSemantics,
    /// Iteration over hash sets or maps whose order a port cannot reproduce.
    UnorderedIteration,
    /// Raw pointers, pointer arithmetic, manual memory.
    Pointers,
    /// Threads, goroutines, locks, shared mutable state.
    Concurrency,
    /// Null references that any object value may hold (Java, Go nil, C NULL).
    NullReferences,
    /// Changing classes, modules or methods at runtime (monkeypatching, `method_missing`).
    Monkeypatching,
    /// Code already compiled to machine code (numba, Cython, JAX): Rust gains little speed.
    AlreadyCompiled,
    /// Parameters typed as numpy arrays, which `record` cannot generate.
    ArrayParameters,
    /// C that depends on the target: inline assembly, `volatile` hardware access, bitfields,
    /// packed layouts, compiler extensions, integers cast to addresses.
    PlatformSpecific,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
}

impl Severity {
    pub fn weight(self) -> f64 {
        match self {
            Self::Low => 1.0,
            Self::Medium => 3.0,
            Self::High => 6.0,
        }
    }
}

impl Risk {
    pub fn severity(self) -> Severity {
        match self {
            Self::StringOffsets | Self::RegexSyntax | Self::DynamicRegex | Self::DynamicCode => {
                Severity::High
            }
            Self::DynamicTyping
            | Self::OrderedCollections
            | Self::Async
            | Self::Generators
            | Self::ClassInheritance
            | Self::Reflection
            | Self::Callbacks => Severity::Medium,
            Self::Exceptions | Self::NumberSemantics | Self::Json => Severity::Low,
            Self::Pointers
            | Self::Monkeypatching
            | Self::UnorderedIteration
            | Self::PlatformSpecific => Severity::High,
            Self::IntegerSemantics
            | Self::DivisionSemantics
            | Self::Concurrency
            | Self::NullReferences
            | Self::AlreadyCompiled => Severity::Medium,
            Self::ArrayParameters => Severity::High,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::StringOffsets => "String offsets and slicing",
            Self::RegexSyntax => "Regex features the `regex` crate lacks",
            Self::DynamicRegex => "Regexes built at runtime",
            Self::DynamicCode => "Dynamic code (eval, new Function, computed import)",
            Self::DynamicTyping => "Dynamic typing",
            Self::OrderedCollections => "Insertion-ordered maps and object keys",
            Self::Exceptions => "Exceptions",
            Self::Async => "Async and promises",
            Self::Generators => "Generators",
            Self::ClassInheritance => "Class inheritance",
            Self::Reflection => "Reflection and prototypes",
            Self::Callbacks => "Callback-shaped APIs",
            Self::NumberSemantics => "Number semantics",
            Self::Json => "JSON round-trips",
            Self::IntegerSemantics => "Integer width and overflow",
            Self::DivisionSemantics => "Division and modulo",
            Self::UnorderedIteration => "Hash iteration order",
            Self::Pointers => "Pointers and manual memory",
            Self::Concurrency => "Concurrency",
            Self::NullReferences => "Null references",
            Self::Monkeypatching => "Runtime changes to code",
            Self::AlreadyCompiled => "Already compiled (numba, Cython, JAX)",
            Self::ArrayParameters => "numpy array parameters",
            Self::PlatformSpecific => "Platform-specific C (asm, volatile, bitfields, packing)",
        }
    }

    pub fn guidance(self) -> &'static str {
        match self {
            Self::StringOffsets => {
                "JS indexes strings in UTF-16 code units, Rust in UTF-8 bytes. Offsets that cross \
                 the boundary must be converted, or non-ASCII text gets wrong ranges silently."
            }
            Self::RegexSyntax => {
                "Lookaround, backreferences and the `y` flag are not in the `regex` crate. Use \
                 `fancy-regex` for those patterns, or rewrite them, and test them on non-ASCII \
                 text: `\\w`, `\\b` and case-insensitive matching differ between engines even \
                 where a pattern compiles. JS `/i` without `u` never matches a non-ASCII letter to \
                 an ASCII one, Rust `(?i)` does (`ſ` matches `s`, the Kelvin sign `K` matches `k`): \
                 write `(?i-u)` or compare with `eq_ignore_ascii_case`."
            }
            Self::DynamicRegex => {
                "Patterns come from data or config in JS syntax. Rust regex syntax differs, so \
                 user-visible behavior can change. Pin it with tests on real patterns."
            }
            Self::DynamicCode => {
                "eval, new Function and computed imports have no Rust equivalent. Keep this part \
                 in JS or replace it with an explicit registry."
            }
            Self::DynamicTyping => {
                "Values whose shape is checked at runtime need an explicit Rust enum or \
                 `serde_json::Value`."
            }
            Self::OrderedCollections => {
                "JS Map, Set and object keys keep insertion order. `HashMap` does not: use \
                 `indexmap` wherever order reaches output. With serde_json's `preserve_order`, \
                 `Map::remove` swaps the last key into the hole: use `shift_remove` for JS `delete`. \
                 Sort with `js::sort` when a comparator can return NaN: the order then depends \
                 on V8's algorithm."
            }
            Self::Exceptions => {
                "Thrown errors become `Result`. Errors crossing the bridge must map back to the \
                 same JS error shape."
            }
            Self::Async => {
                "Promises become futures. Bridge with napi-rs `async fn` or `wasm-bindgen-futures`, \
                 or keep the async shell in JS and port the sync core."
            }
            Self::Generators => {
                "Lazy generators become iterators, or return a collected Vec across the bridge."
            }
            Self::ClassInheritance => {
                "Class hierarchies become traits plus composition; `instanceof` checks become enums."
            }
            Self::Reflection => {
                "Proxy, Reflect, Symbol and prototype tricks need a redesign, not a translation."
            }
            Self::Callbacks => {
                "Function-typed parameters and methods cross the bridge on every call. Keep \
                 boundaries coarse: pass data in, get data out."
            }
            Self::NumberSemantics => {
                "JS numbers are f64; bitwise ops truncate to i32; parseInt is lenient. Pick Rust \
                 integer types deliberately. Printing differs: `${n}` gives `1e+21`, `1e-7` and `0` \
                 for -0 where Rust's `{}` does not, so use `js::number_to_string`. `Math.round` \
                 rounds halves up (-2.5 gives -2), Rust's `round` away from zero: use \
                 `js::math_round`. `toFixed` rounds exact halves up ((2.5).toFixed(0) is \"3\"), \
                 Rust's `{:.0}` to even."
            }
            Self::Json => {
                "JSON.stringify key order and number formatting must match `serde_json` output if \
                 the result is user-visible."
            }
            Self::IntegerSemantics => {
                "Python ints never overflow; C, Go and Java ints wrap or are undefined on overflow. \
                 Pick i64, i128 or a bignum crate deliberately, and use wrapping_* where the source wraps."
            }
            Self::DivisionSemantics => {
                "Python `//` and `%` floor toward negative infinity; Rust `/` and `%` truncate: use \
                 `div_euclid`/`rem_euclid` or explicit floor math. C division by zero is undefined."
            }
            Self::UnorderedIteration => {
                "Set and map iteration order comes from the source language's hash function. A port \
                 cannot reproduce it: sort before output, or make the order part of the contract."
            }
            Self::Pointers => {
                "Pointer arithmetic and manual memory become slices, indices and owned types. Code that \
                 exposes pointers across the boundary needs `unsafe`, isolated and explained."
            }
            Self::Concurrency => {
                "Threads, goroutines and locks need a design in Rust (channels, Send/Sync), not a \
                 line-by-line translation; keep concurrency in the caller where possible."
            }
            Self::NullReferences => {
                "Any reference may be null in the source: decide per parameter whether it is `Option` \
                 and what null means, instead of mapping every value to `Option`."
            }
            Self::Monkeypatching => {
                "Code that rewrites classes or modules at runtime has no Rust equivalent: keep it in \
                 the source language and port the stable parts."
            }
            Self::AlreadyCompiled => {
                "numba, Cython and JAX already compile this code to machine code (numba through \
                 LLVM, like Rust): a Rust port is unlikely to run much faster. Port it for a pure \
                 Rust app, not for speed, and profile first. Its floats may not match Rust's bit for \
                 bit (fastmath, operation order): record with `--float-tolerance`."
            }
            Self::ArrayParameters => {
                "`record` passes JSON values, never numpy arrays, so a function that takes \
                 `np.ndarray` gets no valid cases on the unit path. Record the app instead \
                 (`app record`, `app replay`), or port the function together with a caller that \
                 takes and returns plain lists."
            }
            Self::PlatformSpecific => {
                "This C depends on its target: register access, memory layout, the compiler's \
                 type sizes or its assembly. A recording on this machine says nothing about the \
                 target, and battle-tested driver or firmware code gains little from a rewrite: \
                 keep it in C and port the portable logic around it."
            }
        }
    }
}

/// Strings where regex engines disagree most: `\\w`, `\\d`, `\\s` and `\\b` on non-ASCII,
/// case-insensitive matching (Turkish i, Kelvin sign, long s, final sigma), joiners, line
/// separators and combining marks.
pub const REGEX_TRAPS: [&str; 12] = [
    "Iı İi",
    "K \u{212A} k",
    "ſ s S",
    "x\u{200D}y 👨\u{200D}👩\u{200D}👧",
    "a\u{00A0}b\u{2009}c\u{3000}d",
    "١٢٣ ٤٥",
    "𝟘𝟙 ²³",
    "snake_case foo-bar o'neil",
    "ǅungla Ǆ",
    "one\r\ntwo\u{2028}three\u{85}four",
    "cafe\u{301} café",
    "ΣΑΣ σας ς",
];

/// Whether source text uses regular expressions, so its string inputs get [`REGEX_TRAPS`].
pub fn uses_regex(lang: crate::lang::Lang, source: &str) -> bool {
    use crate::lang::Lang;
    static JS_LITERAL: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let has = |needles: &[&str]| needles.iter().any(|n| source.contains(n));
    match lang {
        Lang::Ts => {
            has(&["RegExp("])
                || JS_LITERAL
                    .get_or_init(|| {
                        regex::Regex::new(
                            r"(?:^|[=(,:!&|?{;]|return)\s*/[^/*\s\n][^/\n]*/[dgimsuyv]*",
                        )
                        .expect("valid regex")
                    })
                    .is_match(source)
        }
        Lang::Python => has(&[
            "import re\n",
            "import re ",
            "from re import",
            "import regex",
            "re.compile(",
        ]),
        Lang::Ruby => has(&[
            "=~", "Regexp", "%r{", ".match(", ".scan(/", ".gsub(/", ".sub(/", ".split(/",
            ".match?(",
        ]),
        Lang::Go => has(&["\"regexp\""]),
        Lang::Java => has(&[
            "java.util.regex",
            "Pattern.compile",
            ".replaceAll(",
            ".matches(",
            ".split(",
        ]),
        Lang::Kotlin => has(&["kotlin.text.Regex", "Regex(", ".toRegex("]),
        Lang::C => has(&["<regex.h>", "pcre", "regcomp("]),
        Lang::Php => has(&["preg_", "mb_ereg"]),
        Lang::Csharp => has(&["Regex", "RegularExpressions"]),
    }
}

/// What compiles a Python function to machine code, from one of its decorators
/// (`@njit`, `@numba.jit(cache=True)`, `@cython.cfunc`, `@jax.jit`).
pub fn compiled_by(decorator: &str) -> Option<&'static str> {
    let d = decorator.trim().trim_start_matches('@');
    let name = d.split('(').next().unwrap_or(d).trim();
    if name.starts_with("cython.") {
        return Some("Cython");
    }
    let (module, last) = name.rsplit_once('.').map_or(("", name), |(m, l)| (m, l));
    if module == "jax" || module.starts_with("jax.") {
        return (last == "jit").then_some("JAX");
    }
    let numba_module = matches!(module, "" | "numba" | "nb" | "cuda" | "numba.cuda");
    let numba_name = matches!(
        last,
        "jit" | "njit" | "vectorize" | "guvectorize" | "cfunc" | "stencil" | "generated_jit"
    );
    (numba_module && numba_name).then_some("numba")
}

/// The Python functions a decorator compiles: name, line of the `def`, compiler.
pub fn python_compiled(source: &str) -> Vec<(String, u32, &'static str)> {
    let mut out = Vec::new();
    let (mut pending, mut depth) = (None, 0i32);
    for (i, line) in source.lines().enumerate() {
        let t = line.trim_start();
        if depth > 0 {
            depth += paren_balance(t);
            continue;
        }
        if let Some(rest) = t.strip_prefix('@') {
            pending = pending.or(compiled_by(rest));
            depth = paren_balance(t).max(0);
        } else if let Some(def) = t
            .strip_prefix("def ")
            .or_else(|| t.strip_prefix("async def "))
        {
            if let Some(by) = pending.take() {
                let name = def.split(['(', ':']).next().unwrap_or("").trim();
                out.push((name.to_string(), (i + 1) as u32, by));
            }
        } else if !t.is_empty() && !t.starts_with('#') {
            pending = None;
        }
    }
    out
}

fn paren_balance(line: &str) -> i32 {
    line.chars().fold(0, |n, c| match c {
        '(' => n + 1,
        ')' => n - 1,
        _ => n,
    })
}

/// Whether a JS regex matches case-insensitively where Rust's `(?i)` folds more: without the
/// `u` or `v` flag JS never folds `ſ` to `s` or the Kelvin sign to `k`.
pub fn regex_folds_differently(pattern: &str, flags: &str) -> bool {
    if !flags.contains('i') || flags.contains('u') || flags.contains('v') {
        return false;
    }
    let mut escaped = false;
    pattern.chars().any(|c| {
        let letter = !escaped && matches!(c, 's' | 'S' | 'k' | 'K');
        escaped = !escaped && c == '\\';
        letter
    })
}

/// Whether a JS regex uses features the `regex` crate does not support.
pub fn regex_needs_fancy(pattern: &str, flags: &str) -> bool {
    if flags.contains('y') {
        return true;
    }
    if ["(?=", "(?!", "(?<=", "(?<!", "\\k<"]
        .iter()
        .any(|needle| pattern.contains(needle))
    {
        return true;
    }
    let bytes = pattern.as_bytes();
    let mut in_class = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                if !in_class && bytes.get(i + 1).is_some_and(|b| (b'1'..=b'9').contains(b)) {
                    return true;
                }
                i += 1;
            }
            b'[' => in_class = true,
            b']' => in_class = false,
            _ => {}
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{
        compiled_by, python_compiled, regex_folds_differently, regex_needs_fancy, uses_regex,
    };

    #[test]
    fn detects_case_folds_rust_widens() {
        assert!(regex_folds_differently("^(ms|secs?)$", "i"));
        assert!(regex_folds_differently("K", "gi"));
        assert!(!regex_folds_differently("secs?", "iu"));
        assert!(!regex_folds_differently("secs?", "g"));
        assert!(!regex_folds_differently("\\s+\\d", "i"));
        assert!(!regex_folds_differently("abc", "i"));
    }
    use crate::lang::Lang;

    #[test]
    fn finds_regex_use() {
        assert!(uses_regex(Lang::Ts, "const re = /\\w+/gu;"));
        assert!(uses_regex(Lang::Ts, "return s.replace(/[^a-z]/g, '')"));
        assert!(uses_regex(Lang::Ts, "new RegExp(p, 'i')"));
        assert!(!uses_regex(Lang::Ts, "const half = a / b / 2;"));
        assert!(!uses_regex(Lang::Ts, "// a comment\nconst x = 1;"));
        assert!(uses_regex(
            Lang::Python,
            "import re\n\ndef f(s):\n    return re.sub('a', 'b', s)\n"
        ));
        assert!(!uses_regex(Lang::Python, "import requests\n"));
        assert!(uses_regex(Lang::Go, "import \"regexp\""));
        assert!(uses_regex(Lang::Ruby, "s =~ /a/"));
        assert!(uses_regex(Lang::Java, "Pattern.compile(\"a+\")"));
        assert!(uses_regex(Lang::C, "#include <regex.h>"));
    }

    #[test]
    fn finds_compiled_python() {
        assert_eq!(compiled_by("@njit"), Some("numba"));
        assert_eq!(
            compiled_by("@numba.jit(nopython=True, cache=True)"),
            Some("numba")
        );
        assert_eq!(compiled_by("@nb.vectorize(['f8(f8)'])"), Some("numba"));
        assert_eq!(compiled_by("@cuda.jit"), Some("numba"));
        assert_eq!(compiled_by("@cython.cfunc"), Some("Cython"));
        assert_eq!(compiled_by("@jax.jit"), Some("JAX"));
        assert_eq!(compiled_by("@functools.lru_cache"), None);
        assert_eq!(compiled_by("@app.jit_route"), None);
        let src = "import numba\n\n@numba.njit(\n    cache=True,\n)\ndef step(x, dt):\n    return x + dt\n\n@staticmethod\ndef plain():\n    pass\n\n@property\n@njit\ndef energy(s):\n    return s\n";
        assert_eq!(
            python_compiled(src),
            [
                ("step".to_string(), 6, "numba"),
                ("energy".to_string(), 15, "numba")
            ]
        );
    }

    #[test]
    fn detects_unsupported_regex_features() {
        assert!(regex_needs_fancy("(?<=a)b", ""));
        assert!(regex_needs_fancy("(a)\\1", ""));
        assert!(regex_needs_fancy("a", "y"));
        assert!(!regex_needs_fancy("[\\w\\d]+?", "g"));
        assert!(!regex_needs_fancy("(?<name>x)", "d"));
        assert!(!regex_needs_fancy("[\\1]", ""));
    }
}
