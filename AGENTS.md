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

> **Status:** Not started. Next up: M1 — Core foundation (`songbird-storage`,
> `songbird-recurrence`, `songbird-ical`, §11.1 conformance corpus). No UI work until M1's exit
> criteria are met.

## Repository layout

```
songbird/
├── core/                          Rust workspace — see system-design.md §5
│   ├── songbird-storage/          SQLite-backed local store
│   ├── songbird-recurrence/       RFC 5545 recurrence expansion (pure, no internal deps)
│   ├── songbird-ical/             iCalendar parse/serialize (pure, no internal deps)
│   ├── songbird-sync/             sync engine: merge, change tracking, conflict resolution
│   ├── songbird-crypto/           encryption, key management, envelope crypto
│   ├── songbird-caldav-client/    CalDAV client — inbound, talks to 3rd-party servers
│   ├── songbird-caldav-adapter/   CalDAV adapter — outbound local server (Phase 2)
│   ├── songbird-core/             top-level crate, the ONLY crate exposed to Dart via FFI
│   └── tests/conformance/         seed corpus — real recurrence/iCalendar bugs as regression tests
├── app/                           Flutter app, see §6
│   ├── lib/presentation/          widgets/screens only, no bridge calls
│   ├── lib/state/                 Riverpod providers, owns all bridge calls
│   ├── lib/platform/              push registration, widgets, share sheet, deep links
│   ├── lib/plugin_api/            plugin extension points (§13), stubbed in Phase 1
│   └── rust_bridge/               flutter_rust_bridge generated + hand-written glue
├── sync-service/                  Backend, see §7 — songbird-server crate
│   └── deploy/                    Docker Compose self-hosting on-ramp
├── docs/
│   ├── design/                    This project's two design docs (read-only references)
│   ├── adr/                       Architecture Decision Records — add one per non-obvious decision
│   └── api/                       Generated API docs
```

## Build & test

```
# Rust core
cd core && cargo build && cargo test

# Conformance corpus specifically (must be 100% before merging anything touching
# songbird-recurrence or songbird-ical — see system-design.md §11.1)
cd core && cargo test -p songbird-recurrence -p songbird-ical conformance

# Flutter app
cd app && flutter pub get && flutter test

# Sync service
cd sync-service && cargo build && cargo test
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
