//! songbird-core: the single, narrow, stable, versioned API surface exposed to the Flutter app.
//!
//! Dart code reaches this crate ONLY — never the crates it composes (songbird-storage,
//! songbird-recurrence, etc. directly). See system-design.md §5.2 for the enforced dependency
//! direction and §5.11 for the full intended FFI surface.
//!
//! `api` is where every Dart-facing function lives, mirroring system-design.md §5.11's
//! illustrative signatures. Nothing outside `api` should be reachable from flutter_rust_bridge
//! codegen once that's wired up in M3.

pub mod api;
