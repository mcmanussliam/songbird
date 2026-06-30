# ADR-0001: End-to-end encryption via local CalDAV adapter, not server-side decryption

**Status:** Accepted
**Date:** 2026-06-30

## Context

Songbird needs both (a) end-to-end encryption of calendar content on the native sync service
path, and (b) interoperability with standard CalDAV clients (Thunderbird, Apple Calendar, etc.).
These two goals are in tension: CalDAV (RFC 4791) is a protocol that assumes the server can read
and write event data, while true E2EE means the server architecturally cannot.

Two existing projects sit at the opposite poles of this tradeoff, and both informed this
decision (see market-analysis.md §2.2–§2.3 for the full research):

- **Proton Calendar** chose E2EE with no CalDAV interop story. As of mid-2026 it still has no
  CalDAV/CardDAV endpoint at all — only one-way ICS export and read-only subscription links.
  Their own community forum cites this as one of the most-requested, longest-unresolved
  complaints in the product.
- **EteSync/Etebase** proved the alternative works in production: rather than asking the server
  to decrypt, a small local process (`etesync-dav`) holds the decryption key on the user's own
  device, decrypts on the fly, and re-exposes the data as a real, standard CalDAV server on
  `localhost` that any compliant client connects to.

## Decision

Songbird follows the EteSync precedent, not Proton's. The sync service never holds a standing
decryption key, ever (see system-design.md §8.1, and AGENTS.md rule 2 — this is treated as an
invariant, not a default). CalDAV interop is provided by `songbird-caldav-adapter`
(system-design.md §5.10, §9.2): a local server, holding the calendar's content key on-device,
decrypting on the fly, serving plaintext CalDAV only to clients explicitly pointed at it.

The one place plaintext legitimately reaches the sync service is the opt-in, capability-scoped,
read-only ICS share link (§9.3) — and only because the sharing user's own device chose to push
that specific snapshot up for that specific purpose, not because the service holds a standing key.

## Consequences

- Self-hosters get identical encryption guarantees to the managed tier, by construction — this is
  a load-bearing claim for the monetization model in market-analysis.md §8.
- Outbound CalDAV interop (a real desktop client pointed at a Songbird group calendar) requires
  running a local adapter process, which is more setup friction than a server that just speaks
  CalDAV directly. This is deferred to Phase 2 (M6) rather than blocking launch.
- Reversing this later (moving to server-side decryption) would be a breaking change to the trust
  model and a significant trust cost with the userbase it's specifically built to earn — this
  decision should be treated as effectively permanent.
