# rust_bridge/

flutter_rust_bridge generated + hand-written glue code lives here once M3 starts.

Bridge target: `core/songbird-core` ONLY — see system-design.md §5.11 and AGENTS.md rule 1.
Do not generate bindings against any other crate in `core/`.

Setup (M3):
1. `cargo install flutter_rust_bridge_codegen`
2. Configure `flutter_rust_bridge.yaml` pointing at `../../core/songbird-core/src/api.rs`
3. Generated Dart output goes to `app/lib/bridge_generated/` (gitignored — regenerated, not
   hand-edited, see ../.gitignore)
