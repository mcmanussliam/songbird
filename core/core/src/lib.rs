
//! Single, narrow, versioned API surface exposed to the Flutter app via flutter_rust_bridge.
//!
//! Dart code reaches this crate only, never the crates it composes directly.
//! To regenerate Dart bindings: `cd app && flutter_rust_bridge_codegen generate`

mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */

pub mod api;
pub use api::*;
