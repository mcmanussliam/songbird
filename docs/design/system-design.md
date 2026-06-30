# Open Source Shared Calendar — System Design Document (v1.0)

> **Naming note:** this project is named **Songbird**. Crate and repo-path names below were
> updated from earlier drafts (`calendar-*` → `songbird-*`, repo root `shared-calendar/` →
> `songbird/`) to match the scaffolded repository exactly.

**Status:** Ready for implementation
**Audience:** Engineering team / implementing agent
**Companion doc:** `market-analysis.md` (market analysis, competitive teardown, rationale). This document assumes that context and focuses entirely on **what to build and how**.

---

## 0. How to use this document

This is a handoff spec. Section order roughly follows build order:

1. §1 — what the product is, full feature inventory with phase/priority.
2. §2 — the one framework decision that was open before (Flutter, decided here, with rationale).
3. §3–4 — system architecture and repo layout, so you know where code goes before you write any.
4. §5 — the Rust core: the part everything else depends on. Build and test this first.
5. §6 — the Flutter app.
6. §7 — the sync service (backend).
7. §8–10 — the three subsystems that are easy to get wrong if under-specified: encryption/keys, CalDAV interop, and the sync/conflict protocol. Read these fully before writing the corresponding code, not just skimming.
8. §11 — testing strategy, including the literal conformance test corpus to implement against.
9. §12 — deployment and self-hosting.
10. §13 — plugin system (Phase 3, but the extension points must exist from Phase 1).
11. §14 — milestone plan mapped to this doc's sections.
12. §15 — explicitly open decisions the implementing team still needs to make, so nothing is silently assumed.

---

## 1. Product summary and full feature inventory

**One-line description:** A local-first, end-to-end-encrypted, group-first shared calendar app for iOS and Android, fully interoperable with standard CalDAV servers, with an optional managed sync service for push notifications, group invites, and presence.

### 1.1 Feature inventory

Legend: **P0** = MVP / Phase 1, blocks launch. **P1** = Phase 2, post-launch fast-follow. **P2** = Phase 3, plugin-era or later.

| # | Feature | Phase | Owner component |
|---|---|---|---|
| F1 | Local calendar CRUD (create/view/edit/delete events) | P0 | Core + App |
| F2 | Month / week / day / agenda views | P0 | App |
| F3 | Recurring events, full RFC 5545 (RRULE, EXDATE, RDATE, overrides) | P0 | Core (recurrence engine) |
| F4 | Offline-first: full read/write with no network | P0 | Core (storage) |
| F5 | CalDAV two-way sync to any third-party server (Nextcloud, Radicale, Fastmail, iCloud) | P0 | Core (CalDAV client) |
| F6 | ICS one-way feed subscription (school/sports/holiday calendars) | P0 | Core + Sync Service |
| F7 | Local reminders/notifications (device-scheduled) | P0 | App |
| F8 | Group calendars via native sync service (multi-user, multi-device) | P0 | Sync Service + Core |
| F9 | Per-member event colors | P0 | Core + App |
| F10 | End-to-end encryption of calendar content by default (native sync service path) | P0 | Core (crypto) + Sync Service |
| F11 | Account creation / device enrollment for native sync service | P0 | Sync Service + App |
| F12 | Push reminders via the native sync service (UnifiedPush/APNs/FCM) | P0 | Sync Service |
| F13 | Self-hosting: full sync-service feature parity when self-hosted | P0 | Sync Service |
| F14 | Group invites via link / QR code | P1 | Sync Service + App |
| F15 | Granular sharing: full read/write, read-only, free/busy-only | P1 | Sync Service + Core |
| F16 | Outbound CalDAV interop: local adapter exposing a group calendar to Thunderbird/Apple Calendar/etc. | P1 | Core (CalDAV adapter) |
| F17 | Read-only signed ICS subscription links for sharing out ("send grandma a link") | P1 | Sync Service |
| F18 | Calendar import from TimeTree / Google Calendar export | P1 | App + Core |
| F19 | Presence / read receipts ("who's seen this event") | P1 | Sync Service + App |
| F20 | Plugin system: stable extension API (`onEventCreated`, `provideAgendaCard`, `provideSidebarPanel`, etc.) | P2 (API designed in P0, see §13) | Core + App |
| F21 | Per-event comment/note threads (as a plugin, not core) | P2 | Plugin |
| F22 | Polls / availability-finding (as a plugin) | P2 | Plugin |
| F23 | Task lists (as a plugin) | P2 | Plugin |
| F24 | Desktop / web client | P2 | New component, deferred (§14) |

### 1.2 Explicitly out of scope (all phases, unless revisited)

Grocery lists, chore trackers, photo/video social archives as core features (only viable as third-party plugins per F20–23), advertising of any kind, any feature that requires the sync service to hold a standing decryption key.

---

## 2. Framework decision: Flutter

Previously left open between Flutter and React Native. **Decision: Flutter**, for this project specifically:

- It compiles to its own rendering engine (Skia/Impeller) instead of bridging to native widget trees, which avoids a category of platform-inconsistency bugs that RN's native-bridge model is more prone to — important for a small team trying to avoid the "feels like a port" criticism leveled at Etar.
- `flutter_rust_bridge` is a mature, widely-used, actively maintained path for exactly this project's shape: a Rust core with a single UI layer calling into it. It generates type-safe Dart bindings from Rust signatures, supports async Rust functions returning Dart `Future`/`Stream`, and handles the FFI boilerplate that would otherwise have to be hand-rolled twice (once per platform) under a more manual approach.
- Dart's isolate model maps cleanly onto "long-lived background sync engine talking to a Rust core" without blocking the UI thread, which is the dominant runtime pattern this app needs.
- Single language (Dart) for 100% of the UI layer, vs. RN's typical mix of JS/TS plus native modules for anything platform-specific — fewer languages in the repo for a volunteer-leaning OSS project to onboard contributors into.

**This decision is final for this document.** All UI-layer guidance below assumes Flutter. If the contributor base that materializes is overwhelmingly JS/TS rather than Dart, this is the one decision worth revisiting — but build against Flutter unless and until that happens.

---

## 3. System architecture overview

```
┌──────────────────────────────────────────────────────────────────┐
│                         FLUTTER APP (iOS / Android)                │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  Presentation layer (widgets, screens, navigation)           │  │
│  ├────────────────────────────────────────────────────────────┤  │
│  │  State layer (Riverpod providers, view models)                │  │
│  ├────────────────────────────────────────────────────────────┤  │
│  │  flutter_rust_bridge generated bindings                       │  │
│  └───────────────────────────┬────────────────────────────────┘  │
└──────────────────────────────┼────────────────────────────────────┘
                                │  FFI (Dart ⇄ Rust)
┌──────────────────────────────▼────────────────────────────────────┐
│                          RUST CORE (single crate workspace)         │
│  ┌───────────────┐ ┌────────────────┐ ┌─────────────────────┐    │
│  │ Storage layer  │ │ Recurrence      │ │ Encryption module     │    │
│  │ (SQLite)       │ │ engine (RFC     │ │ (per-calendar keys,   │    │
│  │                │ │ 5545)           │ │ envelope encryption)  │    │
│  └───────┬───────┘ └────────┬───────┘ └──────────┬───────────┘    │
│          │                  │                     │                  │
│  ┌───────▼──────────────────▼─────────────────────▼───────────┐    │
│  │              Sync engine (CRDT merge, change tracking)        │    │
│  └───────┬─────────────────────────────────────┬───────────────┘    │
│          │                                     │                     │
│  ┌───────▼───────────────┐         ┌───────────▼─────────────────┐ │
│  │ CalDAV client          │         │ CalDAV adapter (local server) │ │
│  │ (outbound to 3rd-party │         │ (outbound to local clients,   │ │
│  │  CalDAV servers)       │         │  Phase 2, see §9.2)           │ │
│  └───────┬───────────────┘         └────────────────────────────┘ │
└──────────┼──────────────────────────────────────────────────────────┘
           │                                          ▲
           │ HTTPS / CalDAV (RFC 4791)                │ HTTPS / native sync protocol
           ▼                                          │
┌─────────────────────────┐          ┌────────────────┴──────────────────┐
│  Third-party CalDAV       │          │           SYNC SERVICE              │
│  server (user-supplied):  │          │  ┌───────────┐ ┌────────────────┐  │
│  Nextcloud, Radicale,      │          │  │ API        │ │ Auth /          │  │
│  Fastmail, iCloud, etc.    │          │  │ gateway    │ │ device enroll   │  │
└─────────────────────────┘          │  └─────┬─────┘ └────────────────┘  │
                                        │  ┌─────▼─────┐ ┌────────────────┐  │
                                        │  │ Encrypted  │ │ Push relay      │  │
                                        │  │ event store │ │ (UnifiedPush/   │  │
                                        │  │ (ciphertext)│ │  APNs/FCM)      │  │
                                        │  └────────────┘ └────────────────┘  │
                                        │  ┌─────────────────────────────┐  │
                                        │  │ CalDAV gateway (Phase 2)      │  │
                                        │  │ (serves read-only ICS links)  │  │
                                        │  └─────────────────────────────┘  │
                                        └────────────────────────────────────┘
```

### 3.1 Component list (for ownership / module-boundary purposes)

| Component | Language | Runs on | Repo location |
|---|---|---|---|
| `core` | Rust | Compiled into the app (mobile) | `core/` |
| `app` | Dart/Flutter | iOS, Android | `app/` |
| `sync-service` | Rust or Go (decide per §15) | Server (managed or self-hosted) | `sync-service/` |
| `bridge` (FFI glue, generated + hand-written wrappers) | Rust + Dart | Build-time codegen, part of `app`'s build | `app/rust_bridge/` |
| Conformance test corpus | Data files + Rust test harness | CI | `core/tests/conformance/` |

---

## 4. Repository layout

```
songbird/
├── core/                          # Rust workspace — the single source of truth for business logic
│   ├── Cargo.toml                 # workspace manifest
│   ├── songbird-storage/          # SQLite-backed local store
│   ├── songbird-recurrence/       # RFC 5545 recurrence expansion engine
│   ├── songbird-ical/             # iCalendar (RFC 5545/5546) parse/serialize
│   ├── songbird-sync/             # sync engine: CRDT merge, change tracking, conflict resolution
│   ├── songbird-crypto/           # encryption, key management, envelope crypto
│   ├── songbird-caldav-client/    # CalDAV client (inbound: talk to 3rd-party servers)
│   ├── songbird-caldav-adapter/   # CalDAV adapter (outbound: local server, Phase 2)
│   ├── songbird-core/             # top-level crate composing the above, exposes the FFI-facing API
│   └── tests/
│       └── conformance/           # seed corpus, see §11.1 — real .ics files from known bugs
├── app/
│   ├── lib/
│   │   ├── main.dart
│   │   ├── presentation/          # screens, widgets, navigation
│   │   ├── state/                 # Riverpod providers, view models
│   │   ├── platform/              # push registration, widgets, share sheet, deep links
│   │   └── plugin_api/            # Dart side of the plugin extension points (§13)
│   ├── rust_bridge/                # flutter_rust_bridge generated + hand-written glue
│   ├── ios/
│   ├── android/
│   └── pubspec.yaml
├── sync-service/
│   ├── api-gateway/
│   ├── event-store/
│   ├── push-relay/
│   ├── caldav-gateway/             # Phase 2
│   ├── auth/
│   └── deploy/                     # Docker Compose / Helm chart for self-hosters
├── docs/
│   ├── adr/                        # architecture decision records, one file per decision
│   └── api/                        # generated API docs for core + sync-service
└── CONTRIBUTING.md
```

Module boundaries are enforced, not just organized: Cargo workspace crate boundaries mean `songbird-core` (the FFI-facing crate) cannot accidentally expose internals of `songbird-storage` to Dart without an explicit re-export, and a CI lint step fails the build if Dart `presentation/` code imports anything other than `state/` providers (no UI code talking to the bridge directly).

---

## 5. The Rust core

This is the part to build and test first (§14, Milestone 1), before any UI exists. Everything else is replaceable; this is not.

### 5.1 Responsibilities

- Own the canonical local data model and local SQLite store.
- Expand recurrence rules correctly (this is the project's single highest-leverage piece of engineering, per the competitive teardown in the companion doc).
- Parse and serialize iCalendar data losslessly enough to round-trip through third-party CalDAV servers without corruption.
- Encrypt/decrypt calendar content client-side.
- Speak CalDAV as a client (inbound sync to third-party servers).
- Speak CalDAV as a server, locally, for outbound interop (Phase 2, §9.2).
- Merge concurrent changes deterministically (CRDT-style merge for the small set of genuinely concurrent fields, last-writer-wins elsewhere).
- Expose a single, stable, versioned API surface to the Dart layer — this is the API contract the Flutter team builds against, and it should not leak SQLite, CalDAV, or crypto implementation details.

### 5.2 Crate structure and dependency direction

Dependency direction is one-way, enforced by Cargo workspace visibility:

```
songbird-core
   ├── depends on: songbird-storage, songbird-recurrence, songbird-ical,
   │               songbird-sync, songbird-crypto, songbird-caldav-client,
   │               songbird-caldav-adapter
songbird-sync
   └── depends on: songbird-storage, songbird-ical, songbird-crypto
songbird-caldav-client
   └── depends on: songbird-ical
songbird-caldav-adapter
   └── depends on: songbird-ical, songbird-crypto
songbird-recurrence
   └── depends on: (nothing internal — pure RFC 5545 logic, fully unit-testable in isolation)
songbird-ical
   └── depends on: (nothing internal)
songbird-storage
   └── depends on: (nothing internal — SQLite + schema only)
songbird-crypto
   └── depends on: (nothing internal)
```

No crate below `songbird-core` may depend on `songbird-core` — this prevents circular coupling and keeps `songbird-recurrence` and `songbird-ical` independently testable and, eventually, independently publishable/reusable by other projects (a real secondary win: a correct, well-tested standalone RFC 5545 recurrence crate is something the wider Rust ecosystem currently lacks a great answer for).

### 5.3 Data model

Core entities, expressed as the conceptual model (concrete SQLite DDL in §5.4):

```
User
  id: UUID
  display_name: String
  device_keys: [DeviceKey]          # see §8

Group ("a calendar is a group of people")
  id: UUID
  name: String
  members: [Membership]
  calendars: [Calendar]

Membership
  user_id: UUID
  group_id: UUID
  role: Owner | Editor | Viewer | FreeBusyOnly   # maps to F15 granular sharing
  color: ColorValue                              # per-member color, F9
  joined_at: Timestamp

Calendar
  id: UUID
  group_id: UUID | null             # null = personal/local-only calendar
  display_name: String
  source: Local | CalDAV(server_config) | NativeSync(group_id) | Subscription(ics_url)
  encryption_key_id: UUID | null    # null only for plain CalDAV-sourced calendars

Event
  id: UUID                          # maps to iCalendar UID
  calendar_id: UUID
  summary: String
  description: String | null
  location: String | null
  dtstart: DateTimeOrDate
  dtend: DateTimeOrDate
  timezone: TzId | null
  rrule: RecurrenceRule | null       # see songbird-recurrence
  rdate: [DateTimeOrDate]
  exdate: [DateTimeOrDate]
  recurrence_id: DateTimeOrDate | null   # set on override instances
  sequence: u32                      # RFC 5545 SEQUENCE, for conflict ordering
  status: Confirmed | Tentative | Cancelled
  organizer: UserRef | null
  attendees: [AttendeeRef]
  reminders: [Reminder]
  last_modified: Timestamp
  etag: String | null                # CalDAV ETag, for 3rd-party-sourced events

Reminder
  trigger: RelativeDuration | AbsoluteTime
  action: Display | Push

SyncCursor                            # per (calendar, transport) sync-token / change-tracking state
  calendar_id: UUID
  transport: CalDAV | NativeSync
  cursor_token: String
  last_synced_at: Timestamp
```

`Event` deliberately mirrors RFC 5545's `VEVENT` fields closely rather than inventing a parallel internal model — this is what keeps `songbird-ical` a thin, mostly-mechanical translation layer instead of a lossy one, which is exactly the class of bug cataloged in the companion doc's §2.1 (Fossify's invalid `EXDATE` export, dropped timezones on import).

### 5.4 Storage layer (`songbird-storage`)

SQLite via `rusqlite` (or `sqlx` with the SQLite driver — pick one, see §15), single file per app install, WAL mode enabled for concurrent read/write safety with the sync engine running on a background isolate.

Representative schema (abbreviated; full schema lives in `core/songbird-storage/migrations/`):

```sql
CREATE TABLE groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE memberships (
    user_id TEXT NOT NULL,
    group_id TEXT NOT NULL REFERENCES groups(id),
    role TEXT NOT NULL CHECK (role IN ('owner','editor','viewer','freebusy_only')),
    color TEXT NOT NULL,
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, group_id)
);

CREATE TABLE calendars (
    id TEXT PRIMARY KEY,
    group_id TEXT REFERENCES groups(id),
    display_name TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('local','caldav','native_sync','subscription')),
    source_config TEXT NOT NULL,        -- JSON blob, shape depends on source_type
    encryption_key_id TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE events (
    id TEXT PRIMARY KEY,                -- iCalendar UID
    calendar_id TEXT NOT NULL REFERENCES calendars(id),
    summary TEXT NOT NULL,
    description TEXT,
    location TEXT,
    dtstart INTEGER NOT NULL,           -- stored as epoch millis, UTC-normalized
    dtstart_is_date_only INTEGER NOT NULL DEFAULT 0,
    dtend INTEGER NOT NULL,
    dtend_is_date_only INTEGER NOT NULL DEFAULT 0,
    timezone TEXT,                      -- IANA TZID, null for UTC/floating
    rrule TEXT,                         -- raw RRULE string, expansion happens in songbird-recurrence
    rdate TEXT,                         -- JSON array of timestamps
    exdate TEXT,                        -- JSON array of timestamps
    recurrence_id INTEGER,              -- non-null only on override instances
    sequence INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'confirmed',
    last_modified INTEGER NOT NULL,
    etag TEXT,
    deleted_at INTEGER                  -- soft delete, needed for sync tombstones
);

CREATE INDEX idx_events_calendar_dtstart ON events(calendar_id, dtstart);
CREATE INDEX idx_events_recurrence_id ON events(recurrence_id);

CREATE TABLE sync_cursors (
    calendar_id TEXT NOT NULL,
    transport TEXT NOT NULL CHECK (transport IN ('caldav','native_sync')),
    cursor_token TEXT,
    last_synced_at INTEGER,
    PRIMARY KEY (calendar_id, transport)
);
```

Notes:

- **Soft deletes (`deleted_at`) are mandatory**, not optional — a hard delete loses the tombstone needed to propagate the deletion to other devices/servers during the next sync pass. This is the most common class of "sync deleted an event on one device, it reappeared from another" bug.
- **`dtstart`/`dtend` are stored UTC-normalized with an explicit `is_date_only` flag** rather than as raw strings, specifically to avoid the timezone-on-import bug class cataloged in the companion doc (Fossify reinterpreting an explicit `TZID` in the device's local zone). The *display* timezone is reconstructed from `timezone` at read time, never inferred from the device's current locale.
- Migrations are managed with a simple forward-only migration runner (e.g. `rusqlite_migration`), versioned, checked into `core/songbird-storage/migrations/`.

### 5.5 Recurrence engine (`songbird-recurrence`)

Pure, dependency-free, exhaustively tested. Responsibilities:

- Parse `RRULE` strings per RFC 5545 §3.3.10 into a structured `RecurrenceRule`.
- Expand a `RecurrenceRule` + `DTSTART` + `RDATE`/`EXDATE` into a concrete, bounded list of occurrences for a given date range (the UI never asks for "all occurrences," only "occurrences between date A and B," to keep infinite/open-ended rules tractable).
- Correctly apply `BYDAY`, `BYMONTHDAY`, `BYMONTH`, `BYSETPOS`, `BYYEARDAY`, `BYWEEKNO`, including negative-ordinal forms (`BYDAY=-1SA` = "last Saturday") — this is the exact bug class that broke Fossify (computed as "every fourth Saturday" instead).
- Correctly resolve override instances: when an event has a `recurrence_id`-tagged override `VEVENT`, that specific occurrence is replaced by the override, not duplicated alongside it — this is the exact bug class that made edited occurrences vanish in Fossify.
- Timezone-aware expansion: recurrence expansion happens in the event's stored `timezone`, with DST transitions resolved correctly (an event recurring weekly across a DST boundary keeps its local wall-clock time, not its UTC offset).

API surface (Rust, illustrative):

```rust
pub struct RecurrenceRule { /* parsed RRULE fields */ }

pub fn parse_rrule(raw: &str) -> Result<RecurrenceRule, IcalError>;

pub fn expand_occurrences(
    rule: &RecurrenceRule,
    dtstart: DateTimeOrDate,
    rdate: &[DateTimeOrDate],
    exdate: &[DateTimeOrDate],
    overrides: &[EventOverride],
    range: DateRange,
) -> Vec<Occurrence>;
```

### 5.6 iCalendar parser/serializer (`songbird-ical`)

Wraps (or, if no sufficiently correct crate exists at implementation time, implements directly) RFC 5545/5546 parsing and serialization. Design constraints:

- **Round-trip fidelity**: parsing a `.ics` file and re-serializing it without modification should produce a file that re-imports cleanly into the source server — this is the property Fossify's `EXDATE` export bug violated (`EXDATE:20250429` against a `DATE-TIME`-typed `DTSTART`, which RFC 5545 requires to match value types).
- **Preserve unknown/vendor `X-` properties** rather than dropping them on round-trip, where feasible — this is what makes interop with Thunderbird/Mozilla-derived clients (which add `X-MOZ-*` properties) non-destructive.
- Exposes both a streaming parser (for large `.ics` imports) and a single-`VEVENT` parse/serialize path (for the common per-event sync case).

### 5.7 Sync engine (`songbird-sync`)

Owns reconciliation between the local store and any remote transport (CalDAV or native sync service). See §10 for the full conflict-resolution algorithm; this section covers structure only.

- **Change tracking**: every local mutation increments the event's `sequence` and updates `last_modified`. A per-(calendar, transport) `sync_cursors` row tracks the last-seen remote state (CalDAV sync-token, or native-sync cursor) so incremental sync never needs a full re-fetch.
- **Merge**: per-field CRDT-style merge for `summary`, `description`, `location`, `dtstart`/`dtend` (last-writer-wins by `last_modified`/`sequence`, since true concurrent-edit merging of "the event moved to two different times" has no sensible automatic resolution — surface the conflict to the user in that specific case, see §10.3); structural merge for `attendees`/`reminders` (additive, union-based) where concurrent edits don't actually conflict.
- **No append-only journal.** State is reconciled record-by-record (current-state model, not an event-sourced log) — this keeps storage and sync-engine complexity down; see the companion doc's §3 for why the EteSync-style full journal was considered and deliberately not adopted.
- Runs sync passes triggered by: app foreground, periodic background fetch (where the OS permits), explicit pull-to-refresh, and push-triggered wake (native sync service path only).

### 5.8 Encryption module (`songbird-crypto`)

See §8 for the full key-management design. This crate provides the primitives:

- Per-calendar symmetric content key (AES-256-GCM or XChaCha20-Poly1305 — pick one, see §15) used to encrypt event payloads before they leave the device on the native-sync path.
- Asymmetric device key pairs (X25519 for key agreement, Ed25519 for signing) for envelope-encrypting the per-calendar key to each authorized device during sharing/invite flows.
- Deliberately has **no knowledge of CalDAV, storage, or sync** — it's a pure crypto primitives crate, independently auditable.

### 5.9 CalDAV client (`songbird-caldav-client`) — inbound

Implements RFC 4791 (CalDAV) client behavior against any compliant server:

- Discovery (`PROPFIND` against the user-supplied base URL, well-known URI fallback).
- Calendar collection listing.
- Incremental sync via `sync-collection` REPORT (preferred) with `CTag`/`ETag`-based fallback for servers that don't support it.
- `PUT`/`DELETE` for outbound changes, `GET`/`REPORT` for inbound.
- Auth: Basic, Digest, OAuth2 (for providers that require it) — pluggable auth strategy.

This crate is what gets validated against the §11.1 conformance corpus and against real servers (Nextcloud, Radicale, Fastmail) before any UI work begins (Milestone 1, §14).

### 5.10 CalDAV adapter (`songbird-caldav-adapter`) — outbound, Phase 2

See §9.2 for full design. Summary: a local CalDAV server (same shape as `etesync-dav`) that holds the calendar's decryption key, decrypts on the fly, and serves plaintext CalDAV to clients explicitly pointed at it.

### 5.11 FFI bridge surface

The `songbird-core` crate is the **only** crate `flutter_rust_bridge` generates bindings from. It exposes a narrow, stable, versioned API — Dart code never reaches past this into the other crates. Representative surface (illustrative signatures, not final):

```rust
// songbird-core/src/api.rs — the entire Dart-facing surface lives here

pub async fn init(db_path: String) -> Result<(), CoreError>;

pub async fn create_local_calendar(display_name: String) -> Result<CalendarId, CoreError>;
pub async fn create_event(calendar_id: CalendarId, draft: EventDraft) -> Result<EventId, CoreError>;
pub async fn update_event(event_id: EventId, patch: EventPatch) -> Result<(), CoreError>;
pub async fn delete_event(event_id: EventId, scope: DeleteScope /* this-only | this-and-future | all */) -> Result<(), CoreError>;

pub async fn occurrences_in_range(calendar_ids: Vec<CalendarId>, range: DateRange) -> Result<Vec<OccurrenceView>, CoreError>;

pub async fn add_caldav_account(config: CalDavConfig) -> Result<AccountId, CoreError>;
pub async fn sync_now(calendar_id: CalendarId) -> Result<SyncResult, CoreError>;

// Streams, for reactive UI — flutter_rust_bridge supports Rust Stream -> Dart Stream
pub fn watch_occurrences(calendar_ids: Vec<CalendarId>, range: DateRange) -> impl Stream<Item = Vec<OccurrenceView>>;
pub fn watch_sync_status() -> impl Stream<Item = SyncStatus>;
```

Streaming APIs (`watch_*`) are the primary mechanism the Flutter state layer uses to stay reactive without polling — Riverpod providers subscribe directly to these Rust-backed streams.

---

## 6. The Flutter app

### 6.1 Tech stack

- **Flutter** (stable channel), Dart null-safety throughout.
- **State management: Riverpod** (not Provider/Bloc/GetX) — chosen for compile-time-safe dependency injection, first-class support for async/stream providers (a direct fit for the `watch_*` Rust streams above), and no `BuildContext` coupling, which keeps state logic testable independent of widget trees.
- **Navigation: `go_router`** — declarative routing, deep-link support (needed for invite links, F14, and `webcal://`/`.ics` file-open intents).
- **`flutter_rust_bridge`** for the FFI layer (codegen as a build step, see `app/rust_bridge/`).
- **Local notifications:** `flutter_local_notifications` for on-device reminder scheduling (F7); push delivery (F12) is a separate path through the OS push frameworks, triggering a sync pass on receipt rather than carrying the reminder payload itself (payloads are encrypted, see §8).
- **Calendar UI widgets:** build custom (month/week/day/agenda grid) rather than depending on a third-party calendar widget package — third-party calendar widgets are a common source of the "doesn't feel native" complaints leveled at competitors, and the interaction model here (multi-calendar overlay, per-member colors, drag-to-reschedule) is specific enough to warrant owning it.

### 6.2 App architecture (layered)

```
presentation/   — Widgets only. No business logic, no direct bridge calls.
state/          — Riverpod providers. Owns all bridge calls and stream subscriptions.
                  Exposes view-model-shaped data to presentation/.
platform/       — Platform channel code: push token registration, home-screen widgets,
                  share sheet integration, deep link handling. Talks to state/, never
                  directly to presentation/.
plugin_api/     — Dart-side plugin extension points (§13). Empty/stub in Phase 1,
                  but the interfaces are defined here from the start so Phase 3
                  plugins don't require a presentation-layer refactor.
```

CI enforces (via `import_lint` or a custom Dart analyzer plugin) that `presentation/` cannot import the generated bridge package directly — all bridge access goes through `state/` providers. This is the Dart-side mirror of the Cargo workspace boundary enforcement in §5.2.

### 6.3 Screen inventory

| Screen | Feature(s) | Notes |
|---|---|---|
| Calendar (month/week/day/agenda) | F1, F2, F3, F9 | Default landing screen; view toggle persists per user |
| Event detail / edit | F1, F3, F7 | Recurrence editor must surface "this event / this and future / all events" scope explicitly on edit and delete (this is precisely where Fossify's edited-occurrence bug became user-visible) |
| Calendar/account list | F5, F6, F8, F13 | Lists local, CalDAV, native-sync, and subscription calendars distinctly, with per-calendar sync status |
| Add calendar flow | F5, F6, F8 | Three explicit entry points: "add a CalDAV server," "subscribe to a feed URL," "create/join a group" — never a single ambiguous "add calendar" button |
| Group detail / members | F8, F9, F14, F15 | Member list with role badges (Owner/Editor/Viewer/Free-busy), invite link/QR generation |
| Onboarding / account setup | F11 | Local-only usable with zero account creation; native sync service signup is opt-in, presented after first local calendar is already created, never gating first use |
| Settings | F10, F13 | Encryption status indicator, self-host server URL override, export/backup |
| Import | F18 | TimeTree/Google export file picker → preview → confirm |

### 6.4 Offline-first UI patterns

- Every write (create/edit/delete) commits to the local Rust core synchronously from the UI's perspective (sub-frame latency) and is queued for sync in the background — the UI never blocks on network.
- A persistent, unobtrusive sync-status indicator (not a blocking spinner) reflects `watch_sync_status()`: synced / syncing / offline / conflict-needs-attention.
- Conflicts that need a user decision (§10.3) surface as a dismissible-but-not-silently-discardable in-app notification, never a popup that can be lost by accidental tap-away.

### 6.5 Platform integration

- **Push registration:** on native-sync account creation, register a UnifiedPush/APNs/FCM token with the sync service (§7.6); receipt of a push triggers a background sync pass via the Rust core, not a foreground full app launch.
- **Home-screen widgets:** native iOS WidgetKit / Android App Widget, both reading from the same local SQLite store the main app uses (read-only, no bridge round-trip needed if the widget extension links the same Rust core as a static library — confirm feasibility per-platform during implementation; fallback is a periodically-refreshed cache written by the main app).
- **Deep links:** `https://app.example/invite/{token}` and `webcal://`/`.ics` file associations both route through `go_router`.

---

## 7. Sync service (backend)

### 7.1 Responsibilities

Everything CalDAV cannot express: push delivery, group invites, presence, granular per-member permissions, and (Phase 2) outbound read-only ICS links. **It never holds a standing decryption key** — see §8.

### 7.2 Tech stack

- **Language: Rust** (reuses `songbird-ical` and `songbird-crypto` crates directly from the `core` workspace — no reimplementation of recurrence or crypto logic server-side) or **Go**, if the team's server-side contributor pool favors it. Recommendation: **Rust**, specifically to share the `songbird-ical`/`songbird-crypto` crates rather than maintaining parallel implementations server- and client-side (a direct mitigation against the exact "two implementations drift apart" failure mode that caused several of the §11.1 conformance bugs in competing projects). Final call belongs to whoever staffs this component first — see §15.
- **Web framework:** `axum` (if Rust) — async, well-supported, integrates cleanly with `tokio`.
- **Database:** PostgreSQL for the event/membership store; Redis (or equivalent) for ephemeral presence state and push-token caching.
- **Containerized**, with a Docker Compose file and a Helm chart in `sync-service/deploy/` as the self-hosting on-ramp (F13).

### 7.3 Components

| Component | Responsibility |
|---|---|
| API gateway | AuthN/Z, request routing, rate limiting |
| Auth / device enrollment | Account creation, device key registration (§8.2), session tokens |
| Encrypted event store | Stores ciphertext event payloads + minimum routing metadata (calendar ID, group ID, rough timestamp for ordering — never plaintext content) |
| Push relay | Maintains push tokens per device, sends silent/data pushes (no plaintext payload) on every relevant event change, abstracts over UnifiedPush/APNs/FCM |
| CalDAV gateway (Phase 2) | Serves signed, capability-scoped read-only ICS feed URLs (F17); does **not** serve full read/write CalDAV — that's the local adapter's job (§9.2), specifically so this component never needs a decryption key |
| Group/invite service | Generates and validates invite links/QR tokens (F14), manages membership roles (F15) |

### 7.4 Server-side data model

```sql
CREATE TABLE accounts (
    id UUID PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE devices (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES accounts(id),
    public_key BYTEA NOT NULL,         -- X25519 public key, see §8.2
    push_token TEXT,
    push_platform TEXT CHECK (push_platform IN ('apns','fcm','unifiedpush')),
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE groups (
    id UUID PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE memberships (
    account_id UUID NOT NULL REFERENCES accounts(id),
    group_id UUID NOT NULL REFERENCES groups(id),
    role TEXT NOT NULL,
    encrypted_calendar_key BYTEA NOT NULL,   -- per-member envelope-encrypted copy of the group's
                                              -- calendar content key, see §8.3 — server stores
                                              -- this ciphertext but never has the device private
                                              -- key needed to open it
    PRIMARY KEY (account_id, group_id)
);

CREATE TABLE event_records (
    id UUID PRIMARY KEY,                -- mirrors the Rust core's Event.id (iCalendar UID)
    group_id UUID NOT NULL REFERENCES groups(id),
    ciphertext BYTEA NOT NULL,           -- AEAD-encrypted serialized Event, client-side
    temporal_bucket TEXT NOT NULL,       -- ISO year-week (e.g. '2026-W27'), plaintext, see §8.4/§15.4
    sequence INTEGER NOT NULL,           -- mirrors core's sequence field, for ordering only
    updated_at TIMESTAMPTZ NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0   -- tombstone, mirrors core's soft-delete
);

CREATE INDEX idx_event_records_bucket ON event_records(group_id, temporal_bucket);

CREATE TABLE invites (
    token TEXT PRIMARY KEY,
    group_id UUID NOT NULL REFERENCES groups(id),
    role TEXT NOT NULL,
    expires_at TIMESTAMPTZ,
    max_uses INTEGER,
    uses INTEGER NOT NULL DEFAULT 0
);
```

The server genuinely cannot read `event_records.ciphertext` or `memberships.encrypted_calendar_key` — both are opaque blobs to it. This is enforced by construction (no server-side decryption key ever exists), not by policy.

### 7.5 API design (sketch)

REST over HTTPS, JSON envelopes for metadata, binary ciphertext as base64 or a separate binary body depending on size. Representative endpoints (full OpenAPI spec to be generated in `docs/api/` during implementation):

```
POST   /v1/accounts                       create account
POST   /v1/accounts/{id}/devices          register device + public key + push token
POST   /v1/groups                         create group
POST   /v1/groups/{id}/invites            create invite (role, expiry, max_uses)
POST   /v1/invites/{token}/accept         join group via invite, exchange device key
GET    /v1/groups/{id}/events?since=<cursor>   incremental pull, returns ciphertext records
PUT    /v1/groups/{id}/events/{event_id}  push a ciphertext record (upsert)
DELETE /v1/groups/{id}/events/{event_id}  tombstone an event
GET    /v1/groups/{id}/members            list members + roles (not key material)
POST   /v1/groups/{id}/share-links        (Phase 2) create a read-only signed ICS link
```

### 7.6 Push notification flow

1. Client A writes/modifies an event in a native-sync calendar.
2. Core's sync engine pushes the encrypted record to `PUT /v1/groups/{id}/events/{event_id}`.
3. Sync service stores the ciphertext, looks up all *other* devices in the group, and sends a push via UnifiedPush/APNs/FCM **carrying the encrypted event delta itself** (small, single-event ciphertext, not a full-resync signal) — per §15's finalized decision, this is what enables rich, attributed notification text without the push provider or sync service ever seeing plaintext.
4. On iOS, a Notification Service Extension intercepts the push before display, decrypts the delta locally using the device's already-held calendar key (§8.3), updates the local store, and constructs the human-visible notification text from the decrypted result. On Android, an equivalent high-priority FCM data-message handler runs the same decrypt-then-construct sequence before the OS-level notification is shown. Either way, the visible notification ("Alex moved Soccer practice to 5pm") is assembled entirely on-device.
5. A full incremental sync pass (`GET /v1/groups/{id}/events?since=<cursor>`) still runs on next app foreground/background-fetch as a correctness backstop, in case a push was dropped — the push path is a latency optimization for the common case, not the only path data can arrive through.

This deliberately avoids Nextcloud Calendar's documented failure mode (push notifications silently not arriving despite correct configuration, per the companion doc's §2.2) by keeping the push payload itself trivial and idempotent — a missed or duplicate push only costs a slightly-delayed sync pass, never a missed reminder, since local reminder scheduling (F7) is independent of push delivery and already has the event data once any sync pass succeeds.

### 7.7 Group invite flow (F14)

1. An existing member with sufficient role requests an invite: `POST /v1/groups/{id}/invites` with desired role, expiry, max uses.
2. Server returns a token; client renders it as a link (`https://app.example/invite/{token}`) and/or QR code.
3. New member opens the link (deep link into the app, or a web fallback page prompting app install), calls `POST /v1/invites/{token}/accept`, which registers their device and returns the group's metadata.
4. **Key exchange happens device-to-device-mediated-by-server, not server-generated:** the inviting member's device encrypts the group's calendar content key to the new member's device public key (X25519 ECDH) and uploads that envelope via the membership record (§7.4's `encrypted_calendar_key`) — the server relays this blob but cannot open it. See §8.3 for the full key-exchange sequence.

---

## 8. Encryption and key management (detailed)

This is the subsystem most likely to be gotten wrong if under-specified, so it's spelled out fully here rather than left to be inferred from the architecture diagram.

### 8.1 Threat model and goals

- The sync service operator (including a malicious or compelled operator) must not be able to read event content, even with full database access.
- A self-hoster running their own sync service gets identical guarantees to the managed service — encryption is not a managed-tier-only feature.
- Losing a single device's key must not lose access to the calendar for the rest of the group (recoverability via the remaining devices/members, not a single point of failure).
- The bring-your-own-CalDAV path (F5) is explicitly **not** required to be E2EE — that's the user's own server, under their own trust model, and CalDAV's wire format doesn't support E2EE without breaking interop. E2EE is specifically a native-sync-service-path guarantee (F10's scope).

### 8.2 Device identity

- On first launch (or first native-sync-service signup), the app generates an X25519 key pair (encryption/key-agreement) and an Ed25519 key pair (signing), stored in the platform secure enclave/keystore (iOS Keychain / Android Keystore) — never written to the SQLite store in plaintext, never leave the device.
- The device's **public** keys are registered with the sync service (`POST /v1/accounts/{id}/devices`) and are the only key material the server ever sees.
- Multi-device: a second device for the same account goes through the same device-registration flow and must be authorized by an *existing* device (scan a QR code shown on the new device, or approve a push prompt on the existing one) before it receives any calendar key — this prevents a compromised account credential alone from silently pulling plaintext.

### 8.3 Per-calendar content key and group key exchange

- Each native-sync-backed `Calendar` has one symmetric content key (AES-256-GCM, generated client-side when the calendar/group is created).
- To add a member or device to the group, an existing authorized device with the key performs an X25519 ECDH key agreement with the new device's public key, derives a wrapping key (HKDF), and encrypts the calendar content key with it (envelope encryption). This ciphertext is what's stored server-side in `memberships.encrypted_calendar_key` (§7.4) — opaque to the server.
- The new device, on accept, fetches its `encrypted_calendar_key` envelope, performs the matching ECDH + HKDF + decrypt locally, and now holds the plaintext content key — at no point does the server hold or need it.
- **Key rotation** (on member removal, F15's role downgrade, or a "remove this device" action): a remaining authorized device generates a new content key, re-encrypts going forward, and re-wraps it to every still-authorized device's public key — removed members/devices simply never receive the new envelope. Historical events already on a removed device remain readable to that device (rotation is not retroactive re-encryption of history, which is both expensive and not actually necessary for the threat model — the goal is preventing *future* access, not erasing what was already legitimately synced).

### 8.4 What gets encrypted vs. what's metadata

| Data | Encrypted? | Rationale |
|---|---|---|
| Event summary, description, location, attendees, reminders | Yes (AEAD, per-calendar key) | Core content |
| `dtstart`/`dtend`/recurrence rule (exact values) | Yes | Encrypted alongside content; see the coarse-bucket mechanism below for how range queries still work without decrypting this |
| Coarse temporal bucket (ISO year-week, e.g. `2026-W27`) | No (plaintext metadata, finalized per §15.4) | Enables efficient incremental range-bounded sync without forcing a full-history download; leaks only "this group has some event in this week," not exact timing |
| Group ID, event ID, `sequence`, `updated_at`, tombstone flag | No (plaintext metadata) | Needed for sync/ordering/routing; leaks "a group with N members has activity," not content |
| Push token, device public key | No | Inherently can't be encrypted and still be useful to the server |
| Member display names | Encrypted in transit, decrypted on-device before display (finalized per §15.5) | Push payloads carry ciphertext deltas; a Notification Service Extension (iOS) / equivalent on-device handler (Android) decrypts before the notification is shown, so rich attributed text reaches the user without the push provider or server ever holding plaintext names |

---

## 9. CalDAV interop layer (detailed)

### 9.1 Inbound: CalDAV client (F5) — Phase 1

Standard client behavior against RFC 4791. No special encryption handling needed — calendars synced via this path are, by definition, not E2EE (the third-party server already has plaintext access by design; see §8.1). Build and validate this against the §11.1 conformance corpus and against real Nextcloud/Radicale/Fastmail instances before any UI work begins.

### 9.2 Outbound: local CalDAV adapter (F16) — Phase 2

For a user who wants to view/edit their native-sync group calendar from Thunderbird, Apple Calendar, or another standard CalDAV client:

- The app (or, on desktop where there's no persistent "app" process, a small companion binary built from the same `songbird-caldav-adapter` crate) runs a CalDAV server bound to `localhost` (mobile: in-process; desktop: a literal local HTTP server, mirroring the `etesync-dav` precedent).
- This adapter holds the calendar's content key (already present on the device per §8.3) and decrypts on the fly, serving standard plaintext CalDAV responses to any client pointed at `http://localhost:<port>/`.
- Authentication to the local adapter is a separate, locally-generated credential (not the user's cloud account password) — shown once in-app for the user to enter into Thunderbird/etc.
- **The sync service itself is never involved in this path beyond its normal encrypted-blob-relay role** — the decrypt happens entirely on the device that already has the key. This is the direct implementation of the architecture decision documented in the companion doc's ADR-0001 (see §12.4).

### 9.3 Outbound: read-only signed ICS links (F17) — Phase 2

For the "grandma just wants to see it" case, where running a full CalDAV adapter on her device isn't realistic:

- The sharing user's own device (which holds the key) periodically generates a plaintext `.ics` snapshot of the calendar/sub-range they're choosing to share, and uploads it to the sync service at a capability-scoped, unguessable, signed URL.
- The sync service serves this URL as a static (or periodically-refreshed) ICS feed — this is the one place the sync service legitimately handles plaintext, and it's strictly opt-in, per-share, and limited to exactly what the sharing user chose to expose (never the full encrypted store).
- This mirrors Proton's and Homsy's "subscribe to my calendar" link pattern, with the explicit constraint that the server only ever receives what was deliberately pushed up for this purpose, never a standing decryption capability.

---

## 10. Sync protocol and conflict resolution (detailed)

### 10.1 Sync cursor model

Each `(calendar_id, transport)` pair tracks an opaque cursor (`sync_cursors` table, §5.4):

- **CalDAV transport:** the cursor is the server's `sync-token` (RFC 6578) where supported, falling back to a `CTag` + per-resource `ETag` comparison for servers without `sync-collection` support.
- **Native sync transport:** the cursor is a server-issued opaque pagination token returned from `GET /v1/groups/{id}/events?since=<cursor>` (§7.5).

A sync pass is always: fetch changes since cursor → merge locally (§10.2) → push local changes since last push → advance cursor only after both directions succeed, so a failed pass is safely retryable from the last known-good cursor.

### 10.2 Merge algorithm

For each remote record received during a sync pass, compared against the local record with the same `id`:

1. **No local record exists** → insert directly (remote create, or first sync).
2. **Local record exists, remote `sequence` > local `sequence`** → remote wins outright, overwrite local (simple forward progress, the common case — only one side changed since last sync).
3. **Local record exists, local has unsynced local changes (`last_modified` after `sync_cursors.last_synced_at`) AND remote also changed since then** → genuine concurrent edit. Apply field-level merge:
   - Non-conflicting fields (different fields changed on each side, e.g. local changed `location`, remote changed `description`) → merge both, no user involvement.
   - Same field changed on both sides, **and** the field is in the additive/structural set (`attendees`, `reminders`) → union merge.
   - Same field changed on both sides, **and** the field is scalar (`summary`, `dtstart`/`dtend`, `rrule`) → this is a true conflict. Resolve by `last_modified` timestamp (most-recent-edit-wins) **and** surface it to the user as a non-blocking "this event was changed on two devices, here's what we kept" notice with an undo affordance pointing at the losing version (retained briefly in a local conflict-log table, not silently discarded) — this is the §6.4 "conflict needs attention" UI state.
4. **Remote tombstone (`deleted = true`)** → soft-delete locally regardless of local edits, but if the local record had unsynced edits at the moment of deletion, surface the same conflict notice rather than silently vanishing an event the user was actively editing.

### 10.3 What "surfacing a conflict" means concretely

Not a modal, not a blocking dialog (per §6.4). A dismissible item in a small "Recent changes" list, each entry reading like "Soccer practice time was changed on two devices — kept Tuesday 4pm (newest edit). [Restore Monday 3pm instead]" with the restore action available for a bounded retention window (e.g. 30 days) before the losing version is purged from the local conflict log.

---

## 11. Testing strategy

### 11.1 Recurrence/iCalendar conformance corpus — build this before any UI work

`core/tests/conformance/` holds real `.ics` fixtures, each one a regression test, seeded directly from documented real-world bugs (do not treat these as hypothetical — they are transcribed from actual GitHub issues against Fossify and Etar):

| Fixture | Bug it regresses |
|---|---|
| `last_weekday_of_month.ics` | `BYDAY=-1SA` ("last Saturday") must not be computed as a fixed fourth-occurrence rule |
| `minutely_recurrence.ics` | `FREQ=MINUTELY;INTERVAL=5;COUNT=100` must expand, not silently collapse to a single non-recurring event |
| `edited_single_occurrence.ics` | An `EXDATE` + override `VEVENT` (matching `RECURRENCE-ID`) must replace, not duplicate, that occurrence in the expanded series |
| `exdate_value_type_match.ics` (export test) | Serialized `EXDATE` must match the `VALUE` type of `DTSTART` (`DATE` vs `DATE-TIME`) — round-trip through the parser and re-import must succeed |
| `explicit_tzid_import.ics` | An event with an explicit `TZID` must retain that timezone on import, never silently reinterpreted in the device's local zone |
| `zero_duration_event.ics` | `DTEND == DTSTART` is legal per RFC 5545 and must sync correctly, not vanish |
| `weekday_repeat_rule.ics` | A "repeat every weekday" `BYDAY=MO,TU,WE,TH,FR` rule must not expand to every single day |

Each fixture is run through: parse → expand occurrences over a representative date range → assert against a hand-verified expected occurrence list → serialize → re-parse → assert byte-for-byte-equivalent-or-better round trip. This suite runs in CI on every PR touching `songbird-recurrence` or `songbird-ical`, full stop, no exceptions.

### 11.2 Other testing layers

- **Unit tests:** every crate in `core/`, especially `songbird-recurrence` and `songbird-crypto`, at high coverage given how unforgiving both domains are of subtle bugs.
- **Integration tests (CalDAV):** automated test runs against real or containerized instances of Nextcloud, Radicale, and (where feasible via a test account) Fastmail — not mocked CalDAV responses, since the conformance bugs cataloged in §11.1 were specifically *interop* bugs between real server implementations and real clients.
- **Sync protocol integration tests:** multi-device simulation harness (spin up N instances of the core against a test sync-service instance, inject concurrent edits, assert convergence per §10.2's rules).
- **Crypto tests:** known-answer tests for the AEAD/ECDH/HKDF primitives, plus a dedicated test asserting the sync service's stored `event_records.ciphertext` and `memberships.encrypted_calendar_key` are never byte-identical to any plaintext fixture (a cheap, valuable regression guard against an accidental plaintext-leak bug).
- **Flutter widget/golden tests:** standard Flutter testing for the presentation layer; not a priority ahead of the core's correctness, but required before each screen in §6.3 ships.
- **End-to-end (Phase 2+):** scripted multi-device scenarios (two simulators/emulators, one self-hosted sync-service instance, assert an event created on device A appears correctly on device B within the expected push-to-sync latency).

---

## 12. Deployment and self-hosting

### 12.1 Managed sync service

Standard containerized deployment (the team's choice of cloud provider), horizontally scalable `api-gateway`/`push-relay` instances behind a load balancer, PostgreSQL with standard backup/replication, Redis for ephemeral state.

### 12.2 Self-hosting (F13)

`sync-service/deploy/docker-compose.yml` brings up the full stack (api-gateway, event-store/Postgres, push-relay, Redis) with a single command. Requirements documented in `sync-service/deploy/README.md`:

- A domain/TLS termination (Caddy or Traefik reverse proxy included in the compose file by default).
- UnifiedPush distributor configuration for self-hosters who don't want to depend on FCM/APNs (with FCM/APNs as the default-easy-path fallback for those who don't mind).
- Explicit documentation that self-hosting yields **100% feature parity** with the managed tier, including E2EE (the encryption guarantees in §8 hold regardless of who operates the server, by construction) — this is the load-bearing claim from the companion doc's §8 monetization section and must be true in practice, not just in marketing copy.

### 12.3 Mobile app distribution

Standard App Store / Play Store distribution for the managed-service-aware build; F-Droid-compatible build target for the self-hoster audience (no Google Play Services dependency in that build variant — push falls back to UnifiedPush or polling-only in that variant).

### 12.4 Architecture Decision Records

Per the companion doc's §6.6, ADRs are checked into `docs/adr/` from commit one. The first two should be written before implementation starts, since they're referenced by name elsewhere in this document:

- `ADR-0001`: E2EE via local CalDAV adapter (EteSync-precedent pattern) instead of server-side decryption (Proton-style) or no encryption (Nextcloud-style).
- `ADR-0002`: Flutter over React Native and over native SwiftUI/Jetpack Compose, per §2.

---

## 13. Plugin system design

The extension API is **designed in Phase 1** even though plugins themselves ship in Phase 3 (per the companion doc's §6.2: retrofitting a plugin boundary after the fact is what kills extensibility long-term).

### 13.1 Extension points (Dart side, `app/lib/plugin_api/`)

```dart
abstract class CalendarPlugin {
  String get id;
  String get displayName;

  /// Called whenever a new event is created, before the create is finalized.
  /// Plugins may attach metadata but not block creation in Phase 1.
  Future<void> onEventCreated(EventView event) async {}

  /// Renders an optional card in the agenda view, per event.
  Widget? provideAgendaCard(BuildContext context, EventView event) => null;

  /// Renders an optional panel in the event detail sidebar.
  Widget? provideSidebarPanel(BuildContext context, EventView event) => null;
}
```

### 13.2 Extension points (Rust side, `songbird-core`)

Mirrors the Dart side for plugins that need core-level hooks (e.g. a polls plugin that needs its own synced data type): a narrow `PluginDataStore` trait scoped to a plugin-namespaced key-value table, so plugin data syncs through the same native-sync transport without polluting the core `Event` schema.

### 13.3 Phase 3 candidate plugins (from the feature inventory, §1.1)

Per-event comment/note threads (F21), polls/availability-finding (F22), task lists (F23) — each built **as** a plugin against this API, not merged into core, per the companion doc's explicit decision to keep these out of core scope.

---

## 14. Milestone plan (maps to this document's sections)

| Milestone | Scope | Exit criteria |
|---|---|---|
| **M1 — Core foundation** | §5.2–5.6: `songbird-storage`, `songbird-recurrence`, `songbird-ical` | §11.1 conformance corpus passes at 100%; no UI exists yet |
| **M2 — CalDAV client** | §5.9, §9.1 | Two-way sync validated against live Nextcloud, Radicale, and Fastmail instances |
| **M3 — Bare client app** | §6, minus native-sync-dependent screens | Flutter app does local calendars + CalDAV sync only; F1–F7 functional; this is the "barebones client" milestone from the companion doc's 90-day plan |
| **M4 — Encryption + sync service** | §7, §8 | Account creation, device enrollment, group creation, encrypted event sync between 2+ devices working end-to-end |
| **M5 — Push + groups UX** | §6.5, §7.6, §7.7, F8–F13 | Full P0 feature set complete; this is launch-ready |
| **M6 — Invites, sharing, outbound CalDAV** | §9.2, §9.3, F14–F19 | Phase 2 complete |
| **M7 — Plugin platform** | §13, F20–F23 | Phase 3; first-party comment/notes plugin ships as the reference implementation of the plugin API |

ADRs (§12.4) and CONTRIBUTING.md are written before M1 starts, not after M5, per the companion doc's governance guidance.

---

## 15. Open decisions for the implementing team

All decisions below are now finalized except #6, which stays deferred per explicit direction.

1. **SQLite driver: `rusqlite`, decided.** Simpler and lighter than `sqlx` for an embedded, FFI-exposed store with no need for compile-time-checked SQL. The async API surface in §5.11 wraps blocking `rusqlite` calls in `tokio::task::spawn_blocking` rather than adopting `sqlx`'s async-native driver — this keeps the storage crate dependency-light, which matters more here than the marginal ergonomics `sqlx` would add.
2. **AEAD cipher: XChaCha20-Poly1305, decided.** Its 192-bit nonce space makes random nonce generation safe by construction (no nonce-reuse bookkeeping required, unlike AES-256-GCM's 96-bit nonce), which removes an entire class of implementation mistakes from `songbird-crypto` — the simpler-to-get-right option, not just a reasonable one.
3. **Sync service language: Rust, decided.** Shares `songbird-ical` and `songbird-crypto` directly with the core workspace instead of maintaining a second implementation server-side — directly closes off the "two implementations drift apart" failure mode that caused several of the §11.1 conformance bugs in competing projects, and it's the lower-total-effort choice once the core crates already exist.
4. **Encrypting `dtstart`/`dtend`/recurrence rules: encrypt them, with a clean path back to good UX.** Full timing data is encrypted inside the AEAD payload, as the default recommendation said — but to avoid forcing every client to download a group's entire event history just to render the current month (a real UX cost), each `event_records` row also carries one extra **unencrypted, coarse-grained temporal bucket** (ISO year-week, e.g. `2026-W27` — wide enough that it doesn't reveal a specific event's exact time, narrow enough to make incremental range queries cheap). The sync service filters `GET /v1/groups/{id}/events?since=<cursor>&bucket_range=<...>` by this bucket; the client still decrypts and does exact-time filtering locally per §8.4's original guidance. This is the "figure out a way to make it work cleanly" resolution: real plaintext leakage is limited to "this group has *some* event in this week," not exact times, while the app still gets efficient incremental sync instead of always pulling full history.
5. **Member display names in push payloads: ship rich, attributed notifications without leaking names to the push provider.** Push payloads carry the actual encrypted event-change ciphertext (small deltas, not full resync signals), and the client decrypts it **on-device, before the human-visible notification is constructed** — a Notification Service Extension on iOS, and an equivalent high-priority FCM data-message handler on Android that runs before the OS surfaces anything to the user. The push provider and the sync service only ever see ciphertext; the visible notification ("Alex moved Soccer practice to 5pm") is assembled entirely from a local decrypt. This is the same pattern Signal uses for exactly this tradeoff, and it gets the best-UX outcome the direction called for without any plaintext ever leaving the device. §7.6 is updated accordingly: pushes are no longer purely "something changed" signals, they carry ciphertext directly.
6. **Desktop/web client framework** (F24) — **left deferred, per explicit direction.** Revisit Flutter-desktop vs. Tauri-over-Rust-core vs. plain web-CalDAV-client once mobile is shipped and the contributor base is known.
7. **Home-screen widget data access pattern** (§6.5) — whether widget extensions can statically link the Rust core directly or need a cache-file handoff from the main app; platform-API-dependent, confirm during M5 implementation.
