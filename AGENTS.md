# AGENTS.md

Instructions for any AI coding agent (Claude Code, Codex, Cursor, etc.) working in this repository.

## What this project is

**Songbird** — a local-first, end-to-end-encrypted, group-first shared calendar app for iOS and
Android (Flutter), fully interoperable with standard CalDAV servers, with an optional managed
sync service. Full context lives in `docs/design/`:

- `docs/design/market-analysis.md` — why this exists, competitive teardown of TimeTree/Nextcloud/
  EteSync/Proton/DecSync/Etar/Fossify with specific cited bugs.
- `docs/design/system-design.md` — the authoritative implementation spec: architecture, data
  models, API design, encryption design, sync protocol, milestone plan. **Read this before
  writing any code.** Section numbers referenced below (e.g. "§5.4") refer to this file.

If anything in this file conflicts with `system-design.md`, the design doc wins — this file is a
thin pointer plus a few operating rules, not a second source of truth.

## Current milestone

Check `docs/design/system-design.md` §14 for the milestone table. **Update this section** when a
milestone completes, so the next agent session knows where things stand:

> **Status:** M3 in progress. Rust core API (`songbird-core`) implemented: init, list/create
> calendars, add CalDAV account, create/update/delete events, occurrences_in_range (with
> recurrence expansion), sync_now (CalDAV sync). Flutter app scaffolded with Bridge abstraction
> layer (BridgeStub for dev, BridgeFrb once frb codegen runs), Riverpod state providers,
> CalendarScreen (month grid + day agenda), EventDetailScreen, EventEditScreen,
> CalendarListScreen, AddCalDavScreen, and local notifications (F7) in platform/.
> **To activate the real Rust bridge:** install flutter_rust_bridge_codegen, uncomment
> `flutter_rust_bridge` in pubspec.yaml, run `flutter_rust_bridge_codegen generate` in app/,
> then wire BridgeFrb into bridgeProvider. Next up: M4 — native sync service.

## Repository layout

```
songbird/
├── core/                    Rust workspace (package names keep the songbird- prefix)
│   ├── storage/             SQLite-backed local store
│   ├── recurrence/          RFC 5545 recurrence expansion (pure, no internal deps)
│   ├── ical/                iCalendar parse/serialize (pure, no internal deps)
│   ├── sync/                sync engine: merge, change tracking, conflict resolution
│   ├── crypto/              encryption, key management, envelope crypto
│   ├── caldav-client/       CalDAV client — inbound, talks to 3rd-party servers
│   ├── caldav-adapter/      CalDAV adapter — outbound local server (Phase 2)
│   ├── core/                top-level FFI crate, the ONLY one exposed to Dart
│   └── tests/conformance/   seed corpus — real recurrence/iCalendar bugs as regression tests
├── app/                     Flutter app, see §6
│   ├── lib/presentation/    widgets/screens only, no bridge calls
│   ├── lib/state/           Riverpod providers, owns all bridge calls
│   ├── lib/platform/        push registration, widgets, share sheet, deep links
│   ├── lib/plugin_api/      plugin extension points (§13), stubbed in Phase 1
│   └── rust_bridge/         flutter_rust_bridge generated + hand-written glue
├── server/                  Backend, see §7 — songbird-server crate
│   └── deploy/              Docker Compose self-hosting on-ramp
├── tests/caldav-servers/    Docker Compose for CalDAV integration test servers
├── docs/
│   ├── design/              authoritative design docs (read-only references)
│   ├── adr/                 Architecture Decision Records
│   └── api/                 Generated API docs
```

## Build & test

```
# Rust core
cd core && cargo build && cargo test

# Conformance corpus specifically (must be 100% before merging anything touching
# songbird-recurrence or songbird-ical — see system-design.md §11.1)
cd core && cargo test -p songbird-recurrence -p songbird-ical conformance

# CalDAV integration tests (requires Radicale running — see tests/caldav-servers/)
cd core && cargo test -p songbird-caldav-client -- --include-ignored

# Flutter app
cd app && flutter pub get && flutter test

# Server
cd server && cargo build && cargo test
```

## Rules an agent should not violate

1. **Module boundaries are not suggestions.** Dependency direction in the Cargo workspace
   (system-design.md §5.2) and the Dart `presentation/` → `state/` → bridge layering (§6.2) are
   enforced by CI lints. Don't add an import that crosses a boundary even if it "would just
   work" — fix it the architecturally correct way or flag the boundary as wrong in an ADR, don't
   route around it silently. In particular: `songbird-recurrence` and `songbird-ical` take no
   internal-crate dependencies, ever — that's what keeps them independently testable.
2. **Never add a server-side decryption key, anywhere, for any reason** — including "just for
   debugging" or "just for this one admin feature." The entire encryption design (§8) depends on
   this never happening. If a feature seems to need it, that's a sign the feature needs a
   different design, not an exception.
3. **Any change to `songbird-recurrence` or `songbird-ical` must keep the §11.1 conformance suite
   at 100%.** These tests encode real historical bugs from competing projects — a regression here
   is not a minor issue.
4. **Soft-delete only** in `songbird-storage` (`deleted_at`, never a hard `DELETE`) — see §5.4's
   rationale. Tombstones are load-bearing for sync correctness.
5. **New ADRs go in `docs/adr/`**, one file per decision, for anything not already decided in the
   design doc. Don't silently make an architectural call and only mention it in a commit message.

## Conventions

- Conventional commits.
- Rust: `rustfmt` + `clippy` clean before commit.
- Dart: `dart format` + `flutter analyze` clean before commit.
- No feature work outside the milestone currently in progress (see "Current milestone" above)
  without flagging it explicitly — this project is sequenced deliberately (§14), not built
  breadth-first.
