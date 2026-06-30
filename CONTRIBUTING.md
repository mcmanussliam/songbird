# Contributing to Songbird

## Before you start

1. Read [`AGENTS.md`](AGENTS.md) and [`docs/design/system-design.md`](docs/design/system-design.md).
   The architecture, data model, encryption design, and milestone plan are already decided —
   contributions should fit the existing design, or propose a change via an RFC (see below)
   rather than diverging silently.
2. Check `docs/design/system-design.md` §14 for the current milestone. Work outside the active
   milestone is welcome as a proposal, but won't be merged ahead of sequence without discussion.

## RFC process

Anything touching the core data model (`core/songbird-core`'s public API) or the sync protocol
(`core/songbird-sync`, or the sync-service API) needs a short written RFC before implementation —
a single markdown file in `docs/adr/` proposing the change, opened as a PR for discussion. This is
a lightweight process, not a bureaucratic one: a few paragraphs plus the tradeoffs is enough.

## Conventions

- Conventional commits (`feat:`, `fix:`, `docs:`, `refactor:`, etc.).
- Rust: `cargo fmt` + `cargo clippy --all-targets -- -D warnings` clean before opening a PR.
- Dart: `dart format` + `flutter analyze` clean before opening a PR.
- Any PR touching `songbird-recurrence` or `songbird-ical` must keep the conformance suite
  (`core/tests/conformance/`) at 100% passing, and should add a new fixture if it fixes a new
  recurrence/iCalendar bug class.

## Good first issues

Tagged `good-first-issue` once the issue tracker is live. Until then, the conformance fixture
list in `core/tests/conformance/README.md` is a good source of small, well-scoped, high-value
tasks (each fixture needs an implementation that makes it pass).

## Code of conduct

TODO: adopt a standard code of conduct (e.g. Contributor Covenant) before the first public release.
