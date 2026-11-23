//! rust-anything: plan, bridge, verify and ship incremental migrations to Rust.
//!
//! The steps (`analyze`, `bridge`, `harness`, `shadow`, `gate`) are language-neutral in shape;
//! each source language is a plugin module. TypeScript/JavaScript (`ts`) is the first.

#![recursion_limit = "256"]

pub(crate) const MIN_RUST: &str = env!("CARGO_PKG_RUST_VERSION");

pub mod analyze;
pub mod android;
pub mod android_device;
pub mod app;
pub mod bridge;
pub mod cache;
pub mod card;
pub mod codegen;
pub mod codegen_batch;
pub mod codegen_batch_native;
pub mod codegen_c;
pub mod codegen_csharp;
pub mod codegen_ffi;
pub mod codegen_go;
pub mod codegen_java;
pub mod codegen_kotlin;
pub mod codegen_php;
pub mod codegen_py;
pub mod codegen_ruby;
pub mod codegen_tswire;
pub mod crates;
pub mod doctor;
pub mod effects;
pub mod enable;
pub mod error_paths;
pub mod from_tests;
pub mod front_door;
pub mod front_door_http;
pub mod fuzz;
pub mod gate;
pub mod go;
pub mod harness;
pub mod harness_c;
pub mod harness_csharp;
pub mod harness_go;
pub mod harness_java;
pub mod harness_kotlin;
pub mod harness_php;
pub mod harness_py;
pub mod harness_ruby;
pub mod http;
#[cfg(test)]
mod js_number;
pub mod lang;
pub mod library;
pub mod lift;
pub mod mcp;
pub mod mcp_prompts;
pub mod package;
pub mod pin;
pub mod port_map;
pub mod process;
pub mod profile;
pub mod recording;
pub mod report;
pub mod risks;
pub mod setup;
pub mod shadow;
pub mod tolerance;
pub mod ts;
pub mod walk;
pub mod whole_app;
pub mod wire;
