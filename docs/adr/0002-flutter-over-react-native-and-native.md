# ADR-0002: Flutter over React Native and over native SwiftUI/Jetpack Compose

**Status:** Accepted
**Date:** 2026-06-30

## Context

The mobile UI layer needed a framework decision. Three options were considered: fully native
(SwiftUI + Jetpack Compose, two codebases), React Native, and Flutter. See system-design.md §2
and §5.4 for the full writeup; summarized here.

## Decision

Flutter, with a single shared Dart UI codebase calling into the `songbird-core` Rust crate via
`flutter_rust_bridge`. Reasons:

- Flutter compiles to its own rendering engine rather than bridging to native widget trees,
  avoiding a category of platform-inconsistency bugs RN's native-bridge model is more prone to.
- `flutter_rust_bridge` is a mature, actively maintained path for exactly this project's shape
  (Rust core, single UI layer) — generates type-safe Dart bindings, supports async Rust functions
  and Streams mapping to Dart `Future`/`Stream`.
- Single language (Dart) for the entire UI layer, vs. RN's typical JS/TS-plus-native-modules mix
  — fewer languages for a volunteer-leaning OSS contributor base to onboard into.
- Fully native (two codebases) was rejected specifically because it doubles the surface area for
  recurrence/sync bugs to be fixed twice instead of once, which runs directly against this
  project's stated priority of maintainability over UI polish for its own sake.

## Consequences

- Real risk: "feels like a port" criticism (the same complaint leveled at Etar) if cross-platform
  UI is treated as an afterthought. Mitigated by explicit platform-adaptive design discipline, not
  by avoiding the framework — see system-design.md §5.4.
- No web/desktop client in v1 (F24, deferred per explicit direction — see §15.6). The
  bring-your-own-CalDAV path already covers "I want this in a normal calendar app on my
  computer" in the meantime.
- If the contributor base that materializes is overwhelmingly JS/TS rather than Dart, this is the
  one decision worth revisiting — flagged explicitly in system-design.md §2.
