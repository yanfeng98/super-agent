//! The libraries each source language imports and the Rust crates that replace them, from the
//! data table `crates.toml`: adding a mapping is a few lines there, not code.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::ts::deps::{CrateMatch, Fit};

const TABLE: &str = include_str!("crates.toml");

struct Entry {
    names: Vec<String>,
    prefixes: Vec<String>,
    crates: String,
    fit: Fit,
    note: String,
}

fn table() -> &'static BTreeMap<String, Vec<Entry>> {
    static PARSED: OnceLock<BTreeMap<String, Vec<Entry>>> = OnceLock::new();
    PARSED.get_or_init(|| parse(TABLE).unwrap_or_default())
}

fn parse(text: &str) -> Result<BTreeMap<String, Vec<Entry>>, String> {
    let doc: toml_edit::DocumentMut = text.parse().map_err(|e| format!("crates.toml: {e}"))?;
    let strings = |t: &toml_edit::Table, key: &str| -> Vec<String> {
        t.get(key)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut out = BTreeMap::new();
    for (lang, item) in doc.iter() {
        let tables = item
            .as_array_of_tables()
            .ok_or_else(|| format!("crates.toml: `{lang}` must be a list of [[{lang}]] entries"))?;
        let mut entries = Vec::new();
        for t in tables.iter() {
            let text = |key: &str| t.get(key).and_then(|v| v.as_str()).unwrap_or_default();
            let fit = match text("fit") {
                "good" => Fit::Good,
                "partial" => Fit::Partial,
                "none" => Fit::None,
                other => return Err(format!("crates.toml: [[{lang}]] fit `{other}`")),
            };
            entries.push(Entry {
                names: strings(t, "names"),
                prefixes: strings(t, "prefixes"),
                crates: text("crates").to_string(),
                fit,
                note: text("note").to_string(),
            });
        }
        out.insert(lang.to_string(), entries);
    }
    Ok(out)
}

/// The first entry for `lang` whose names hold `name` or whose prefixes start it.
pub fn find(lang: &str, name: &str) -> Option<CrateMatch> {
    table()
        .get(lang)?
        .iter()
        .enumerate()
        .find(|(_, e)| {
            e.names.iter().any(|n| n == name)
                || e.prefixes.iter().any(|p| name.starts_with(p.as_str()))
        })
        .map(|(row, e)| CrateMatch {
            crates: &e.crates,
            fit: e.fit,
            note: &e.note,
            row,
        })
}

/// The longest configured prefix of `lang` that starts `name`, if any. A prefix ending in `.`
/// covers a whole namespace (`androidx.`), not one module, so it is never returned.
pub fn longest_prefix(lang: &str, name: &str) -> Option<&'static str> {
    table()
        .get(lang)?
        .iter()
        .flat_map(|e| e.prefixes.iter().map(String::as_str))
        .filter(|p| !p.ends_with('.') && name.starts_with(p))
        .max_by_key(|p| p.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_parses_and_every_entry_names_something() {
        let parsed = parse(TABLE).unwrap();
        assert_eq!(parsed.len(), 9);
        for (lang, entries) in &parsed {
            for e in entries {
                assert!(
                    !e.names.is_empty() || !e.prefixes.is_empty(),
                    "{lang}: {}",
                    e.crates
                );
                assert!(
                    !e.crates.is_empty() || e.fit == Fit::None,
                    "{lang}: {:?}",
                    e.names
                );
            }
        }
    }

    #[test]
    fn the_first_match_wins() {
        assert_eq!(find("ts", "json5").map(|m| m.crates), Some("json5"));
        assert_eq!(
            find("java", "org.slf4j.Logger").map(|m| m.crates),
            Some("tracing")
        );
        assert!(find("ts", "left-pad-but-new").is_none());
        assert!(find("nope", "json5").is_none());
    }

    #[test]
    fn ffmpeg_bindings_map_in_every_language() {
        for (lang, name) in [
            ("ts", "fluent-ffmpeg"),
            ("python", "av"),
            ("c", "<libavcodec/avcodec.h>"),
            ("go", "github.com/asticode/go-astiav"),
            ("java", "org.bytedeco.ffmpeg.global.avcodec"),
        ] {
            let m = find(lang, name).unwrap_or_else(|| panic!("{lang}: {name}"));
            assert_eq!(
                (m.crates, m.fit),
                ("ffmpeg-next", Fit::Partial),
                "{lang}: {name}"
            );
        }
        assert!(find("c", "<libavcodecx.h>").is_none());
    }

    #[test]
    fn ruby_require_names_map_to_the_gem_that_serves_them() {
        // `require "active_support/core_ext/..."` names `active_support`; the gem is
        // `activesupport`, and the Rails components are separate gems again.
        for (required, crates, fit) in [
            ("active_support", "chrono, heck, inflector", Fit::Partial),
            ("active_record", "sqlx, diesel", Fit::Partial),
            ("action_controller", "axum", Fit::None),
            ("action_view", "axum", Fit::None),
            ("action_mailer", "axum", Fit::None),
            ("active_job", "axum", Fit::None),
            ("active_storage", "axum", Fit::None),
            ("railties", "axum", Fit::None),
        ] {
            let m = find("ruby", required).unwrap_or_else(|| panic!("ruby: {required}"));
            assert_eq!((m.crates, m.fit), (crates, fit), "ruby: {required}");
        }
        // The gem names themselves keep answering the same way.
        assert_eq!(
            find("ruby", "activesupport").map(|m| m.crates),
            Some("chrono, heck, inflector")
        );
        assert_eq!(
            find("ruby", "sequel").map(|m| m.crates),
            Some("sqlx, diesel")
        );
        assert!(find("ruby", "railsx").is_none());
    }

    #[test]
    fn csharp_libraries_a_project_references_map_or_are_kept() {
        for (name, crates, fit) in [
            ("StackExchange.Redis", "redis", Fit::Good),
            ("MongoDB.Driver", "mongodb", Fit::Good),
            ("RabbitMQ.Client", "lapin", Fit::Good),
            ("Grpc.Net.Client", "tonic", Fit::Partial),
            ("Google.Protobuf", "prost", Fit::Good),
            ("Microsoft.Data.SqlClient", "tiberius", Fit::Partial),
            ("FluentValidation", "validator", Fit::Good),
        ] {
            let m = find("csharp", name).unwrap_or_else(|| panic!("csharp: {name}"));
            assert_eq!((m.crates, m.fit), (crates, fit), "csharp: {name}");
        }
        // Neighbours that were already mapped keep answering the same way.
        assert_eq!(
            find("csharp", "Newtonsoft.Json").map(|m| m.crates),
            Some("serde_json")
        );
        assert_eq!(
            find("csharp", "Microsoft.Extensions.Logging").map(|m| m.crates),
            Some("tracing")
        );
        // `AutoMapper` and the DI container were already answered; a second row for either would
        // never be reached, since `find` takes the first match.
        assert!(find("csharp", "AutoMapper").is_some_and(|m| m.fit == Fit::Good));
        assert!(
            find("csharp", "Microsoft.Extensions.DependencyInjection")
                .is_some_and(|m| m.fit == Fit::None)
        );
    }

    #[test]
    fn a_go_subpackage_import_maps_to_the_module_row() {
        // A Go import names a package, and one module holds many: the module path is a prefix.
        for (name, crates, fit) in [
            (
                "golang.org/x/text/cases",
                "unicode-normalization, icu",
                Fit::Partial,
            ),
            (
                "golang.org/x/text/language",
                "unicode-normalization, icu",
                Fit::Partial,
            ),
            ("github.com/spf13/cobra/doc", "clap", Fit::Good),
            (
                "github.com/jackc/pgx/v5/pgxpool",
                "sqlx, tokio-postgres",
                Fit::Good,
            ),
            ("github.com/pkg/errors", "anyhow, thiserror", Fit::Good),
        ] {
            let m = find("go", name).unwrap_or_else(|| panic!("go: {name}"));
            assert_eq!((m.crates, m.fit), (crates, fit), "go: {name}");
        }
        // The module roots keep answering the same way.
        assert_eq!(
            find("go", "golang.org/x/text").map(|m| m.crates),
            Some("unicode-normalization, icu")
        );
        assert_eq!(
            find("go", "github.com/google/uuid").map(|m| m.crates),
            Some("uuid")
        );
        assert!(find("go", "github.com/not-a-module/x").is_none());
    }

    #[test]
    fn go_and_java_libraries_a_module_imports_map_or_are_kept() {
        for (lang, name, crates, fit) in [
            (
                "go",
                "gorm.io/gorm/clause",
                "sqlx, diesel, sea-orm",
                Fit::Partial,
            ),
            ("go", "github.com/redis/go-redis/v9", "redis", Fit::Good),
            (
                "go",
                "github.com/golang-jwt/jwt/v5",
                "jsonwebtoken",
                Fit::Good,
            ),
            (
                "go",
                "github.com/go-playground/validator/v10",
                "validator",
                Fit::Good,
            ),
            ("go", "github.com/go-chi/chi/v5", "axum", Fit::Partial),
            ("java", "io.grpc.ManagedChannel", "tonic", Fit::Good),
            ("java", "com.google.protobuf.Message", "prost", Fit::Good),
            (
                "java",
                "org.apache.kafka.clients.producer.KafkaProducer",
                "rdkafka",
                Fit::Partial,
            ),
            ("java", "redis.clients.jedis.Jedis", "redis", Fit::Good),
        ] {
            let m = find(lang, name).unwrap_or_else(|| panic!("{lang}: {name}"));
            assert_eq!((m.crates, m.fit), (crates, fit), "{lang}: {name}");
        }
        // The tables that were already right keep answering the same way.
        assert_eq!(
            find("go", "github.com/spf13/cobra").map(|m| m.crates),
            Some("clap")
        );
        assert_eq!(
            find("java", "com.squareup.okhttp3:okhttp").map(|m| m.crates),
            Some("reqwest")
        );
    }

    #[test]
    fn ts_libraries_a_node_service_imports_map_or_are_kept() {
        for (name, crates, fit) in [
            ("joi", "garde, validator", Fit::Partial),
            ("graphql", "async-graphql", Fit::Partial),
            ("mongoose", "mongodb", Fit::Partial),
            ("ioredis", "redis", Fit::Good),
            ("rxjs", "futures, tokio-stream", Fit::Partial),
            ("socket.io", "socketioxide", Fit::Partial),
            ("playwright", "chromiumoxide", Fit::Partial),
            ("sharp", "image", Fit::Good),
            ("bcrypt", "argon2, bcrypt", Fit::Good),
            ("jsonwebtoken", "jsonwebtoken", Fit::Good),
            ("execa", "std::process, tokio::process", Fit::Good),
            ("pg", "tokio-postgres, sqlx", Fit::Good),
            ("mysql2", "mysql_async, sqlx", Fit::Partial),
            ("better-sqlite3", "rusqlite", Fit::Good),
            ("papaparse", "csv", Fit::Good),
            ("cheerio", "scraper", Fit::Partial),
            ("marked", "pulldown-cmark, comrak", Fit::Good),
            ("helmet", "tower-http", Fit::Good),
            ("multer", "multer", Fit::Partial),
            ("inquirer", "dialoguer", Fit::Good),
            ("@prisma/client", "sqlx, diesel, sea-orm", Fit::Partial),
        ] {
            let m = find("ts", name).unwrap_or_else(|| panic!("ts: {name}"));
            assert_eq!((m.crates, m.fit), (crates, fit), "ts: {name}");
        }
        // Neighbours that already answered keep answering.
        assert_eq!(find("ts", "zod").map(|m| m.crates), Some("serde, garde"));
        assert_eq!(find("ts", "express").map(|m| m.crates), Some("axum"));
        assert_eq!(
            find("ts", "ws").map(|m| m.crates),
            Some("tokio-tungstenite")
        );
        assert!(find("ts", "left-pad-but-new").is_none());
    }

    #[test]
    fn c_headers_map_by_library_not_by_the_one_header_a_file_includes() {
        for (header, crates, fit) in [
            ("<openssl/sha.h>", "rustls, ring", Fit::Partial),
            ("<openssl/hmac.h>", "rustls, ring", Fit::Partial),
            ("<json-c/json.h>", "serde_json", Fit::Good),
            ("<libxml/parser.h>", "quick-xml", Fit::Partial),
            ("<png.h>", "png, jpeg-decoder", Fit::Good),
            ("<libpq-fe.h>", "tokio-postgres, sqlx", Fit::Good),
            ("<hiredis/hiredis.h>", "redis", Fit::Good),
            ("<uuid/uuid.h>", "uuid", Fit::Good),
        ] {
            let m = find("c", header).unwrap_or_else(|| panic!("c: {header}"));
            assert_eq!((m.crates, m.fit), (crates, fit), "c: {header}");
        }
        // The headers that were already mapped keep answering the same way.
        assert_eq!(find("c", "<zlib.h>").map(|m| m.crates), Some("flate2"));
        assert_eq!(
            find("c", "<openssl/ssl.h>").map(|m| m.crates),
            Some("rustls, ring")
        );
        assert!(find("c", "<not-a-header.h>").is_none());
    }

    #[test]
    fn kotlin_libraries_a_gradle_module_imports_map_or_are_kept() {
        for (name, crates, fit) in [
            ("kotlinx.datetime.Clock", "jiff", Fit::Good),
            ("com.squareup.moshi.JsonAdapter", "serde_json", Fit::Partial),
            ("retrofit2.Retrofit", "reqwest", Fit::Partial),
            ("androidx.room.RoomDatabase", "rusqlite", Fit::Partial),
            ("org.jetbrains.exposed.sql.Database", "sqlx", Fit::Partial),
            (
                "com.google.dagger.hilt.android.HiltAndroidApp",
                "-",
                Fit::None,
            ),
            ("androidx.compose.runtime.Composable", "-", Fit::None),
            ("android.telephony.TelephonyManager", "-", Fit::None),
        ] {
            let m = find("kotlin", name).unwrap_or_else(|| panic!("kotlin: {name}"));
            assert_eq!((m.crates, m.fit), (crates, fit), "kotlin: {name}");
        }
        // The broad `androidx.` entry is a fallback: the specific ones above it win.
        assert_eq!(
            find("kotlin", "androidx.room.Room").map(|m| m.crates),
            Some("rusqlite")
        );
        // A Kotlin module that predates the entry still maps the way it did.
        assert_eq!(
            find("kotlin", "kotlinx.coroutines.flow.Flow").map(|m| m.crates),
            Some("tokio, rayon")
        );
        assert_eq!(
            longest_prefix("kotlin", "androidx.room.Room"),
            Some("androidx.room")
        );
        assert_eq!(
            longest_prefix("kotlin", "androidx.core.content.ContextCompat"),
            None
        );
    }

    #[test]
    fn python_service_libraries_map_or_are_kept() {
        for (name, crates, fit) in [
            ("redis", "redis", Fit::Good),
            ("websockets", "tokio-tungstenite", Fit::Good),
            ("websocket", "tokio-tungstenite", Fit::Good),
            ("pyjwt", "jsonwebtoken", Fit::Good),
            ("jwt", "jsonwebtoken", Fit::Good),
            ("jose", "jsonwebtoken", Fit::Good),
            ("authlib", "-", Fit::None),
            ("bcrypt", "argon2, bcrypt", Fit::Good),
            ("grpcio", "tonic", Fit::Good),
            ("grpc", "tonic", Fit::Good),
            ("protobuf", "prost", Fit::Good),
            ("google.protobuf", "prost", Fit::Good),
            ("tenacity", "backon", Fit::Good),
            ("structlog", "tracing", Fit::Good),
            ("aiofiles", "tokio::fs", Fit::Good),
            ("motor", "mongodb", Fit::Good),
            ("boto3", "aws-sdk-* (per service)", Fit::Partial),
            ("celery", "apalis", Fit::Partial),
            ("paramiko", "russh", Fit::Partial),
            ("scipy", "ndarray, statrs", Fit::Partial),
            ("scikit-learn", "linfa", Fit::Partial),
            ("matplotlib", "plotters", Fit::Partial),
            ("uvicorn", "axum", Fit::None),
        ] {
            let m = find("python", name).unwrap_or_else(|| panic!("python: {name}"));
            assert_eq!((m.crates, m.fit), (crates, fit), "python: {name}");
        }
        // Rows that already answered keep answering.
        assert_eq!(
            find("python", "requests").map(|m| m.crates),
            Some("reqwest")
        );
        assert_eq!(find("python", "pillow").map(|m| m.crates), Some("image"));
        assert!(find("python", "left-pad").is_none());
    }
}
