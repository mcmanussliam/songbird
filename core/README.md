# core/

Rust workspace containing all of Songbird's business logic. This code is shared across every
client (Flutter app, future web/desktop clients) and the optional sync server.

## Crates

```
core/
├── core/            Top-level FFI crate — the ONLY one exposed to Dart via flutter_rust_bridge
├── storage/         SQLite-backed local store (soft-delete only — tombstones are load-bearing)
├── recurrence/      RFC 5545 recurrence expansion — pure, no internal deps
├── ical/            iCalendar parse/serialize — pure, no internal deps
├── caldav-client/   CalDAV client (inbound) — syncs with third-party CalDAV servers
├── caldav-adapter/  CalDAV adapter (outbound) — exposes local data as a CalDAV server (Phase 2)
├── sync/            Sync engine: merge, change tracking, conflict resolution
├── crypto/          Encryption, key management, envelope crypto
└── tests/
    └── conformance/ Real-world iCalendar/recurrence bug corpus (must stay 100% green)
```

## Dependency rules

`recurrence` and `ical` take **no internal workspace dependencies** — this keeps them
independently testable and portable. All other crates may depend on them but not vice versa.
Only `core` (the FFI crate) is allowed to depend on everything.

## Building and testing

```bash
cargo build
cargo test

# Run only the conformance corpus (required before merging changes to recurrence or ical)
cargo test -p songbird-recurrence -p songbird-ical conformance

# CalDAV integration tests (requires Radicale — see tests/caldav-servers/)
cargo test -p songbird-caldav-client -- --include-ignored
```
