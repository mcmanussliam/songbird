//! Single, narrow, versioned API surface exposed to the Flutter app via flutter_rust_bridge.
//!
//! Dart code reaches this crate only — never the crates it composes directly.
//! To regenerate Dart bindings: `cd app && flutter_rust_bridge_codegen generate`

pub mod api;
pub use api::*;
