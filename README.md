# Songbird

A local-first, end-to-end-encrypted, group-first shared calendar — fully interoperable with
standard CalDAV servers, with an optional managed sync service for push notifications, group
invites, and presence.

Songbird is the open-source answer to "TimeTree, but it respects you": one shared calendar a
group co-owns, built on open standards, with your data under your control.

## Status

M3 in progress. Core Rust library implemented; Flutter UI scaffolded with stub bridge.
See [`docs/design/system-design.md`](docs/design/system-design.md) §14 for the full milestone
plan.

## Repository layout

```
songbird/
├── core/            Rust workspace — all business logic shared by every client
├── app/             Flutter app (iOS + Android)
├── server/          Optional managed/self-hostable sync backend (M4+)
├── tests/           Integration test infrastructure (CalDAV server Docker Compose)
└── docs/
    ├── design/      Full system design + market analysis — read these first
    ├── adr/         Architecture Decision Records
    └── api/         Generated API docs (cargo doc output)
```

## For contributors and coding agents

Start with [`AGENTS.md`](AGENTS.md) — it points to the full design doc and lists the rules that
matter (module boundaries, the encryption invariant, the recurrence conformance suite).

## License

AGPL-3.0-or-later. See [`LICENSE`](LICENSE). The core app is always free, fully functional, and
self-hostable with full feature parity to any managed/paid tier — see
`docs/design/system-design.md` for the monetization model and why this is a structural
guarantee, not just a policy.
