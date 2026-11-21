//! Kotlin's generated artifacts are Java artifacts.
//!
//! A Kotlin top-level function compiles to a static method on `<File>Kt`, and `@JvmStatic`
//! in an `object` compiles to a static method on that class. Java source can call both, so
//! the Java JNI codegen, batch codegen, reflection driver and shadow generation are reused
//! unchanged. Keeping one generator also keeps generated code under the same clippy gate.

pub use crate::codegen_java::{
    DRIVER_JAVA, ERROR_JAVA, JSON_JAVA, WIRE_JAVA, native_class, shadows, targets, typed_rs,
    wire_rs, wrappers,
};
