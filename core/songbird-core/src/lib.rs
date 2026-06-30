//! songbird-core: the single, narrow, stable, versioned API surface exposed to the Flutter app.
//!
//! Dart code reaches this crate only — never the crates it composes directly. `api` is where
//! every Dart-facing function lives. Nothing outside `api` should be reachable from
//! flutter_rust_bridge codegen once that's wired up in M3.

pub mod api;
