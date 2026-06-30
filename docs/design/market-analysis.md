# Open Source Shared Calendar — Market Analysis & Technical Plan (v2)

## 1. Why this is worth building

TimeTree's core idea — one shared calendar a group co-owns — is genuinely good UX and still has no clean open-source equivalent. Everything that currently claims that territory falls into one of four buckets, and research into each one's actual GitHub issue trackers and support forums turns up a consistent pattern: **every existing project picked one axis (privacy, openness, UX, or sync robustness) and sacrificed the others**, rather than a single project being merely "behind."

- **Proprietary "shared calendar" apps** (TimeTree, Howbout, Cupla, FamilyWall, Homsy): good UX, closed source, increasingly ad- or subscription-driven, your data lives on their servers under their terms, and they bolt on unrelated features (social feeds, grocery lists, chat) to justify subscription pricing.
- **Privacy-first proprietary calendars** (Proton Calendar, Tuta): real end-to-end encryption, but the encryption model is architecturally incompatible with CalDAV — Proton's servers genuinely cannot read your events, while CalDAV is a protocol that assumes the server can read and edit them. The result, confirmed directly in Proton's own community forum as of mid-2026, is no CalDAV/CardDAV endpoint at all: only one-way ICS export and read-only subscription links, with users explicitly saying the lack of two-way sync is "the only thing preventing a mass exodus" from Google Calendar. This is a structural lesson, not a fixable bug — it's what happens when E2EE is bolted onto a CalDAV-shaped product after the fact instead of designed alongside an interop layer from day one.
- **Generic calendars** (Google Calendar, Outlook, iCloud): free and reliable, but built around *personal* scheduling with sharing bolted on — no per-member identity, no lightweight group-first model, and you're trusting a surveillance-advertising company with your family's schedule.
- **Existing open source calendar clients and sync layers** (Etar, Fossify Calendar, Simple Calendar, Nextcloud Calendar, Merkuro, EteSync, DecSync CC, Radicale, DAVx⁵): no ads, real respect for user data — but each one is missing a different load-bearing piece, detailed in §2 below.

That gap, plus genuine TimeTree user frustration about no offline-first design and no iCal subscription support, is the wedge. The product isn't "another calendar app" — it's "the shared-calendar UX TimeTree popularized, built on open standards, with your data under your control, and without the specific reliability failures that have dogged every open-source attempt at this for a decade."

## 2. Competitive teardown — what specifically breaks in each existing project

This is the part worth taking seriously: it's not enough to say "they're thin clients." The actual GitHub issues and forum threads tell you exactly which architectural decisions to avoid repeating.

### 2.1 Recurrence/RRULE handling — the single most common failure across every open-source client

Pulled directly from open issue trackers, these are representative, not exhaustive:

- **"Last weekday of month" rules computed wrong.** A `BYDAY=-1SA` (last Saturday of the month) recurrence synced via DAVx⁵ displays correctly in Etar and Thunderbird but renders as "every fourth Saturday" in Fossify Calendar — which is simply wrong one month in three, since some months have a fifth Saturday.
- **Minutely/sub-daily recurrence silently dropped.** An `RRULE:FREQ=MINUTELY;COUNT=100;INTERVAL=5` synced into Etar shows as a single non-recurring event — the recurrence is parsed away entirely rather than expanded or rejected loudly.
- **Edited single occurrences vanish from the series.** Fossify Calendar fails to display specific occurrences of a monthly series after they've been individually edited (an `EXDATE` + override-`VEVENT` pattern), even though the same Radicale-backed calendar displays correctly in Etar and Morgen — meaning the bug is in how one specific client reconciles base-event-plus-overrides, not in the data.
- **Invalid ICS on export, not just import.** Fossify's own `EXDATE` export used a bare `DATE` value against a `DATE-TIME`-typed `DTSTART`, producing a file that breaks re-import into Radicale — a self-inflicted round-trip failure.
- **Timezone dropped on `.ics` import.** Opening a `.ics` with an explicit `TZID` causes Fossify to silently reinterpret the time in the phone's local timezone instead of the one specified in the file, while Etar parses the same file correctly — confirmed by the Fossify maintainers themselves as an Etar-vs-Fossify parser difference, not a Radicale or DAVx⁵ issue.
- **Zero-duration events disappear depending on which app created them.** Events where `DTEND` equals `DTSTART` (legal per RFC 5545, common for all-day reminders exported from some tools) fail to sync to the device in both Fossify and Etar when created by DAVx⁵, but sync fine when created natively — a cross-client interoperability gap nobody has actually fixed, with both app teams pointing at the other.

**The lesson:** every one of these is a symptom of ad hoc, organically-grown recurrence and import/export code with no conformance suite. None of them are exotic edge cases — "last weekday of the month," "edit one occurrence," and "respect the timezone in the file" are core, everyday calendar behaviors. This is the strongest evidence for why §4.5 below (a golden conformance suite, tested against real-world ICS samples pulled from these exact bug reports) is not optional polish — it is the single highest-ROI engineering investment in this entire project, because it is the one category of bug that has independently sunk the recurrence reliability of *every* competing open-source client.

### 2.2 Nextcloud Calendar — the closest thing to a "good enough" existing answer, and why it still isn't

Nextcloud Calendar is the most credible existing open-source answer for groups, and it's worth being precise about why it doesn't close the gap:

- **Push notifications for shared-calendar events are unreliable in practice**, independent of correct configuration. Multiple open issues report that sharees on a shared calendar don't receive push notifications for reminders even with "send reminder notifications to calendar sharees" enabled, server-side push enabled, and no errors in logs — this has recurred across several Nextcloud Calendar versions, not one isolated regression.
- **Notification delivery requires non-default server tuning.** Reliable on-time reminders require replacing the default background-job cron with a dedicated `occ dav:send-event-reminders` cron entry — the out-of-the-box behavior is "reminders sent whenever background jobs happen to run next," which is a poor fit for time-sensitive "the kids need picking up in 20 minutes" use cases.
- **Operational ceiling, not feature ceiling.** Default per-user calendar/subscription limits (hit in practice at 30 calendars) and database-level sharing bugs (notifications not propagating to groups) show this is fundamentally a groupware add-on bolted onto a general-purpose cloud server, not a product designed around "a calendar is a group of people" — exactly the TimeTree-vs-Google-Calendar distinction from §1.

This validates rather than undercuts the plan: Nextcloud Calendar is *good CalDAV*, which is why it's the right interop target, but it confirms that **CalDAV compliance alone does not deliver the TimeTree experience** even on the best-run open-source CalDAV server available today.

### 2.3 EteSync/Etebase — the closest existing precedent for "E2EE that still speaks CalDAV," and a validated architecture pattern

EteSync (now Etebase) is the one project that already solved the exact problem Proton couldn't: end-to-end encrypted sync that *still* interoperates with standard CalDAV/CardDAV clients. Its approach is directly useful as precedent:

- Etebase is "a journaled, end-to-end encrypted backend as a service" with official client libraries for Python, Java, Kotlin, C/C++, Rust, and JS/TypeScript, and a server the data owner can self-host.
- Critically, **it does not try to make the server itself speak CalDAV.** Instead it ships `etesync-dav`, a small *local* process that runs on the user's own machine, decrypts data with a key the server never sees, and re-exposes it as a real local CalDAV/CardDAV server that Thunderbird, Apple Calendar, or any standard client connects to on `localhost`.
- It maintains a full encrypted, tamper-evident *journal* of every change (not just current state), which is what lets it offer reliable conflict history and rollback — a stronger reliability story than most CalDAV servers, which only expose current state plus sync-tokens.
- A long-running, detailed community comparison against Radicale/SOGo (non-encrypted CalDAV servers) found EteSync to be the most reliable sync solution the reviewer had used across years of daily use, specifically calling out the absence of the duplicate-event and missing-data failures common elsewhere, and noting it as one of the only solutions that correctly preserves contact groups on iOS/macOS.

**What this validates for our plan:** the "encrypt client-side, expose decrypted ICS only at the edge to clients holding the key" design in the original §4.2 is not speculative — it is a proven pattern. The local-adapter trick is the cleanest version of it, more elegant than trying to push CalDAV semantics through a zero-knowledge cloud server, and is what §5.5 below builds on directly.

**What EteSync does *not* solve, and why our native sync service is still needed:** EteSync is fundamentally a personal-data-sync tool extended to support sharing, not a group-first product. It has no group invite links/QR codes, no presence, and its UI is explicitly a DAVx⁵-style "accounts and sync settings" interface rather than a calendar app — confirmed by its own F-Droid listing modeling its UI directly on DAVx⁵. It's the right reference architecture for the *sync layer*; it is not a TimeTree-UX competitor, which is exactly the gap this plan targets.

### 2.4 DecSync CC — proof there's appetite for "no server at all," and why that model doesn't fit a group product

DecSync takes server-optional sync to its logical extreme: it syncs contacts/calendars/tasks by writing changes into a shared directory and letting any file-sync tool (Syncthing recommended specifically, though Nextcloud or Dropbox also work) replicate that directory across devices, with no central server or account at all.

- It's a real, working, conflict-aware sync model (it avoids the naive "two people edit a CSV-like file and Syncthing creates a `.sync-conflict` copy" failure mode that plain file-sync would have).
- But independent reviews are candid about the cost: Linux setup is noticeably more involved than Android's F-Droid flow, devices left offline can flip calendars to read-only even after the local sync component restarts, and the entire model depends on the user correctly running and trusting a peer-to-peer file-sync daemon — a reasonable tradeoff for a privacy-maximalist individual, a poor onboarding experience for "my partner and the babysitter need to see this calendar by tonight."
- It has no concept of push notifications, invite links, or any server-mediated feature at all by design — which is precisely why it's a fit for syncing *your own* devices, not for the group-coordination use case this plan targets. A family member who isn't running Syncthing simply cannot be in the group.

**What this validates:** there is real, repeated demand (DecSync, EteSync, the original document's cited Lemmy thread) for "I don't want to trust anyone's server with this," which is why the bring-your-own-CalDAV-server path in §4.1 needs to remain a true first-class citizen and not a checkbox feature — but it also confirms that a serverless/P2P-only model cannot deliver push notifications or frictionless group invites, which is why the native sync service in §4.1's second path is still the only way to actually replicate TimeTree's UX rather than CalDAV's.

### 2.5 Merkuro (KDE) and the broader "good client, no group features" pattern

Merkuro is the most actively maintained newer entrant (built on KDE's Kalendar lineage, supports local calendars plus Nextcloud/Google/Outlook/CalDAV). It reinforces rather than contradicts the gap: it's a genuinely well-built *client* over existing protocols and existing third-party servers, with the same structural ceiling as Etar/Fossify — there is no group-first data model, and group sharing is whatever the underlying CalDAV server (typically Nextcloud) happens to support, with all of §2.2's limitations inherited unchanged.

### 2.6 Summary table

| Project | Privacy/data ownership | Group-first UX (invites, presence) | Sync reliability (recurrence, conflict, push) | Self-hosting |
|---|---|---|---|---|
| TimeTree / Howbout / FamilyWall | Poor — proprietary servers, ad/sub-driven | Strong, by design | Generally solid (closed, well-resourced) | None |
| Proton Calendar | Excellent (true E2EE) | Weak — no sharing depth | N/A for interop — no CalDAV at all | None |
| Nextcloud Calendar | Good (self-hostable, not E2EE) | Weak — flaky shared-calendar push, no link-based invites | Good CalDAV compliance; notification delivery bugs | Yes, mature |
| Etar / Fossify / Simple Calendar | Good (local-first, thin client) | None — personal calendar clients | Poor — recurring-event and import/export bugs, see §2.1 | Depends on paired server |
| EteSync / Etebase | Excellent (true E2EE) | None — sync layer, not a calendar UX | Very good, by reputation and design | Yes |
| DecSync CC | Excellent (no server at all) | None, and structurally can't support invites/push | Good for personal multi-device; not group-capable | N/A — serverless |
| **This project (target)** | **Excellent (E2EE by default + BYO-CalDAV)** | **Strong (TimeTree-equivalent)** | **Target: conformance-tested, no known-class regressions** | **Yes, full parity with hosted tier** |

No row currently has more than two of the four columns at "good or better." That's the product opportunity stated as plainly as the research supports.

## 3. What to take and what to avoid from the field

| Take | Leave |
|---|---|
| TimeTree's group-first mental model (a calendar *is* a group of people) | TimeTree's closed data silo and proprietary sync |
| Homsy/FamilyWall's per-member color coding and iCal feed subscriptions | Bundling unrelated household features (groceries, chores) into the core app, or turning the whole app into a social feed (Howbout's direction) |
| CalDAV/iCalendar as the wire format, so any standards-compliant client can talk to it | Reinventing sync protocols, or requiring a proprietary client to use a proprietary server |
| EteSync's local-decrypt-then-expose-CalDAV adapter pattern for E2EE interop | Proton's path of shipping E2EE with no interop story and letting it become the top user complaint for years |
| DecSync's proof that BYO-sync-transport is viable and wanted | DecSync's all-or-nothing serverless model as the *only* option — it structurally cannot do push or invites |
| Offline-first local database with background sync | Requiring connectivity for core functionality, as TimeTree does |
| Self-hosting as a first-class option (à la Nextcloud) | Self-hosting as the *only* option — most users will never run a server |
| Conformance-test-driven recurrence engine from day one | The organic, ad hoc RRULE/EXDATE code visible across Etar/Fossify's bug trackers (§2.1) |

The honest design tension is: **CalDAV is the right open protocol, but CalDAV alone can't deliver TimeTree's UX** (presence, push notifications, granular per-person sharing) because those aren't part of the spec, and **E2EE alone can't deliver CalDAV interop** unless you adopt an EteSync-style local-adapter pattern instead of Proton's server-side-only approach. The plan below resolves both by treating CalDAV as a *first-class interop layer*, not the only sync mechanism — full compatibility for anyone who wants to point Thunderbird or Apple Calendar at their data, plus a richer native sync protocol for the app's own features, with encryption designed around interop from the start rather than retrofitted.

## 4. Product scope (v1)

**In scope:** shared group calendars, per-member colors, month/week/day/agenda views, recurring events with full RFC 5545 support, per-event notes, reminders, offline-first usage, CalDAV import/export and two-way sync, ICS feed subscriptions (school/sports/holidays), end-to-end encryption of calendar content by default, self-hosting option.

**Explicitly out of scope for v1** (resist TimeTree's and Homsy's feature creep): grocery lists, chore trackers, photo/video social archives, polls/availability-finding. These are good *plugin* candidates later (see §8), not core-app bloat.

## 5. Architecture

### 5.1 High-level shape

```
┌─────────────────────────────┐     ┌─────────────────────────────┐
│   Client (iOS / Android /    │     │   Sync Service (optional,    │
│   Desktop) — shared core +   │◄───►│   self-hostable or managed)  │
│   native UI shell            │     │   - encrypted event store     │
│                               │     │   - CalDAV gateway             │
│   Local DB (source of truth) │     │   - push notification relay   │
└─────────────────────────────┘     └─────────────────────────────┘
              │
              ▼
   Direct CalDAV/CardDAV to any
   third-party server (Nextcloud,
   Fastmail, iCloud, Google via
   bridge) — no account required
```

Two independent sync paths, both supported from day one:

1. **Bring-your-own CalDAV server.** The app is a fully capable CalDAV client. Point it at Nextcloud, Radicale, Fastmail, etc. No account with us required, ever. This wins the privacy-conscious/self-hoster crowd outright, costs nothing to maintain beyond protocol compliance, and is the exact audience that currently has to choose between Etar-class clients (thin, buggy recurrence) and EteSync (no group UX) — we beat both by being a full client with a tested recurrence engine.
2. **Native sync service** (our own, optional, paid tier candidate — see §9) for the features CalDAV can't express: read receipts/presence, push delivery, per-member granular permissions, group invites by link/QR. This is what actually replicates the "TimeTree feel." Following the EteSync precedent rather than Proton's, it speaks CalDAV *outward* for interop (so a Nextcloud or Apple Calendar user can subscribe read-only to a group calendar) via a local, key-holding adapter — never by asking the server itself to decrypt anything — while using a richer internal schema for its own clients.

### 5.2 Local-first data model

Every client holds the full local database; the network is an opportunistic sync target, never a hard dependency for reading or writing. This single decision fixes TimeTree's most-cited weakness and is also what makes the app feel fast.

- **Storage:** an embedded, multi-platform SQL engine (SQLite via a typed wrapper) as the canonical local store on every platform.
- **Sync algorithm:** event-sourced changes with per-record vector clocks / Lamport timestamps and CRDT-style merge for the small set of genuinely concurrent fields (event title, time, notes); last-writer-wins is fine for everything else given calendar edits are rarely concurrent. This avoids building a bespoke OT system while still resolving the realistic conflict cases (two people editing the same event while offline). State is reconciled record-by-record rather than via a full append-only journal, keeping the sync engine and its storage footprint simple and easy to reason about.
- **Encryption:** event payloads are encrypted client-side with per-calendar keys before they ever leave the device; the sync service stores ciphertext plus the minimum metadata needed to route push notifications and resolve sharing. CalDAV interop is provided the way EteSync proved out, not the way Proton avoided: a local adapter process (or in-process equivalent inside the app for mobile, where running a literal localhost server is awkward) holds the calendar key, decrypts on the fly, and serves standard CalDAV to any client that authenticates — the cloud sync service itself never holds a decryption key and never needs to.

### 5.3 Recommended stack

Given "maintainable, extensible, clean" as the explicit priority, optimize for *one shared business-logic codebase* and a *single cross-platform UI codebase* rather than maintaining two native UI implementations:

- **Shared core (data model, sync engine, CalDAV/iCalendar parsing, recurrence rules, encryption):** Rust, compiled to a single binary core consumed from the UI layer via generated bindings — `flutter_rust_bridge` if the UI is Flutter (Dart FFI under the hood), or a native module exposing the Rust core via JSI/TurboModules if the UI is React Native. Rust is the right pick here specifically for maintainability: stricter compiler guarantees around the gnarliest part of this domain (timezone math, RRULE recurrence expansion, conflict-free merge logic), and excellent existing crates for iCalendar parsing. Given the recurrence-engine failures cataloged in §2.1 were largely caused by treating RRULE/EXDATE expansion and timezone resolution as scattered date math rather than a hardened module, Rust's type system is a genuine mitigation here, not just a style preference.
- **iOS + Android UI:** a single cross-platform codebase — **Flutter** or **React Native**, picked once and used consistently rather than maintaining parallel SwiftUI/Jetpack Compose implementations. Flutter is the stronger default for this project specifically: it compiles to its own rendering engine rather than bridging to native widgets, which avoids a class of platform-inconsistency bugs RN is more prone to, and `flutter_rust_bridge` is a mature, widely-used path for exactly this "Rust core, Dart UI" shape. React Native is a reasonable alternative if the contributor base skews more JS/TypeScript than Dart — worth deciding based on who actually shows up to contribute, not in the abstract.
- **No web or desktop client in v1.** The bring-your-own-CalDAV path (§5.1) already covers "I want to see this in a normal calendar app on my computer" via Thunderbird, Apple Calendar, or any CalDAV client pointed at the user's own server — so a dedicated web/desktop client is deferred rather than blocking the mobile launch. See §7 for when this is revisited.
- **Sync service:** small, boring, horizontally-scalable backend (Go or Rust) — a CRDT/event store, a CalDAV gateway translating internal events to/from RFC 4791, and a push relay (APNs/FCM/UnifiedPush). UnifiedPush specifically, so self-hosters aren't forced through Google/Apple's push infra, and so reminder delivery doesn't inherit Nextcloud Calendar's documented pattern of cron-dependent, unreliable shared-calendar push (§2.2).
- **Recurrence/iCalendar engine:** implement against RFC 5545/5546 directly with a conformance test suite — this is the single most bug-prone area in every competing open-source calendar, and §2.1's catalog of specific Fossify/Etar bugs (BYDAY=-1 miscalculation, dropped MINUTELY rules, lost edited occurrences, invalid EXDATE export, timezone-on-import dropped, zero-duration event sync loss) should be turned directly into the seed corpus for this suite — each one becomes a regression test before a single line of UI is written.

### 5.4 Mitigating the cross-platform "feels like a port" risk

Etar gets criticized for an interface that's described as hideous, missing handy features competitors have — a real risk for any cross-platform UI if treated as an afterthought. The mitigation isn't "go native," it's discipline: build against each platform's spacing, motion, and navigation conventions explicitly rather than accepting framework defaults (Flutter's Cupertino widget set plus careful platform-adaptive layout gets meaningfully closer to native feel than a default Material-everywhere build), budget real design time rather than treating UI as a thin wrapper over the core, and treat platform-specific polish (haptics, native share sheets, widget/lock-screen integrations) as first-class work items rather than gaps to fill in "later." The payoff for accepting this risk is real: one UI codebase instead of two means recurrence and sync bugs fixed once instead of twice, and a much smaller team can credibly maintain feature parity across iOS and Android — which, given this is a volunteer-leaning OSS project, matters more than the marginal UI polish a fully native build would buy.

### 5.5 The CalDAV interop layer, in more detail

Because §2.2 and §2.3 both turn on getting this exactly right, it's worth specifying it concretely rather than leaving it as a diagram box:

- **Inbound (third-party server → us):** standard CalDAV/CardDAV client implementation against RFC 4791/6352, talking to Nextcloud, Radicale, Fastmail, iCloud, or any compliant server. No special handling needed; this is a solved problem if the recurrence engine underneath it is solid (§5.3).
- **Outbound (us → third-party client, native sync service path):** a per-user, per-device local CalDAV server (same shape as `etesync-dav`) that the app spins up on-device or as a thin local helper on desktop, authenticated separately from the cloud account, holding the decryption key locally, and serving plaintext ICS over `localhost` only to clients explicitly pointed at it (Thunderbird, Apple Calendar, DAVx⁵ on a *different* device talking through a relay endpoint that never itself holds the key). This is strictly more conservative than what Nextcloud offers (where the server can read everything) and strictly more interoperable than what Proton offers (nothing) — it's the actual resolution of the tension named in §3.
- **Read-only group subscription links:** a signed, capability-scoped ICS feed URL (à la Proton's and Homsy's "subscribe to my calendar" links) for the common case of "grandma just wants to see the calendar in her existing Apple Calendar app and never needs write access" — generated server-side from already-decrypted data the *sharing user's own device* pushed up specifically for this purpose, so the cloud service still never needs a standing decryption key, only a per-share opt-in blob.

## 6. Maintainability, extensibility, and "really clean" — concrete mechanisms

These properties don't come from intentions, they come from structural decisions made early:

1. **Hexagonal/ports-and-adapters architecture in the core.** The sync engine, recurrence engine, and encryption layer expose narrow interfaces (`CalendarStore`, `SyncTransport`, `Crypto`) with zero knowledge of UI or platform. Every platform adapter (CalDAV transport, native sync transport, SQLite store) is swappable and independently testable. This is what lets the project add e.g. a Matrix-based sync transport or a different storage engine later without touching business logic.
2. **A real plugin boundary, decided up front.** Define a stable internal event/extension API (e.g. `onEventCreated`, `provideAgendaCard`, `provideSidebarPanel`) before building "extra" features like polls or grocery lists, and build those features *as* the first plugins, not as special-cased core code. This is the single highest-leverage decision for "extensible" — retrofitting a plugin system after the fact is what kills most monolithic OSS apps' second decade.
3. **Conformance-test-driven core.** RFC 5545 recurrence expansion, timezone handling, and CalDAV interop each get a golden test suite — seeded directly from the real-world failure cases in §2.1, not invented from scratch — run in CI on every PR. This single category of bug is what differentiates this project from Etar/Fossify's longstanding, still-open recurrence complaints.
4. **Strict module boundaries enforced by tooling, not convention** — lint rules / build-graph checks (e.g. Gradle module visibility, Cargo workspace crate boundaries) that fail CI if UI code imports sync internals directly, so the architecture can't silently erode.
5. **Trunk-based development with mandatory CI:** typed core language (Rust/Kotlin) + exhaustive unit tests on the core, snapshot/UI tests on each native client, automated release pipelines (Fastlane for iOS, Gradle Play Publisher for Android), and conventional commits + changelog generation so history stays legible.
6. **Documentation as a first-class artifact:** architecture decision records (ADRs) checked into the repo from commit one — including an explicit ADR explaining the EteSync-style local-adapter decision over Proton's server-side-only model, since that's the single decision most likely to be questioned and revisited by future contributors — a CONTRIBUTING.md with a "good first issue" pipeline, and generated API docs for the core's public interface.
7. **Governance set up early:** a lightweight RFC process for anything touching the core data model or sync protocol (so "extensible" doesn't decay into "everyone bolts features onto the UI layer"), and a maintainers' team distinct from the founding org once the contributor base exists, similar to how Etar runs translation and contribution through Weblate/GitHub rather than a closed pipeline.

## 7. Feature roadmap

- **Phase 1 (MVP):** local-first calendar, CalDAV two-way sync to any third-party server (validated against the §2.1 conformance corpus before any UI ships), group calendars via the native sync service, per-member colors, recurring events, ICS subscription feeds, push reminders via UnifiedPush/APNs/FCM, iOS + Android.
- **Phase 2:** link-based group invites, granular sharing (free/busy-only sharing à la Howbout's privacy model), the local CalDAV-adapter outbound interop path from §5.5, calendar import from TimeTree/Google export.
- **Phase 3:** desktop/web client (revisit framework choice at this point — Compose Multiplatform or a Tauri shell over the same Rust core are both reasonable if a richer desktop experience beyond bring-your-own-CalDAV is warranted by then); plugin ecosystem opens up — third-party or first-party plugins for per-event comments/notes threads, polls/availability-finding, task lists, widgets — built entirely on the Phase-1 extension API rather than core changes.

## 8. Fair monetization

The instinct to avoid is TimeTree's mistake (ads + paywalling features users already had) and the instinct to avoid on the other side is relicensing-out-from-under-the-community, which is what torched community trust for MongoDB, Elastic, and CockroachDB when they relicensed and the developer community reacted badly. The model that fits an end-user app (not a developer tool) best is a **hosted-service-first, open-core-light** combination:

- **Core app: always free, fully functional, AGPL or similar copyleft license**, including unlimited self-hosting against your own CalDAV server. The app is never crippled, never ad-supported, never has a "free tier" that nags. This is non-negotiable for trust — communities support monetization when they believe it improves long-term sustainability and the core stays open, and lose trust the moment previously-free features get taken away.
- **Paid tier = the managed sync service**, not paid app features. Running reliable push delivery, multi-device CRDT sync at scale, and backups is real recurring infrastructure cost — charging for *that*, the way Bitwarden, Standard Notes, EteSync, and Proton do, is legible and fair: you're paying for hosting and ops, not for software that's artificially locked.
- **Self-hosting always remains a complete substitute for the paid tier** — anyone capable of running Postgres + the sync service binary themselves gets 100% of the paid tier's functionality for free. This is what keeps the paid tier *ethical* rather than coercive: people pay for convenience, not because the alternative was disabled.
- **Optional, transparent funding alongside the SaaS tier:** GitHub Sponsors / Open Collective for one-off contributors, and a clearly published "where the money goes" page.
- **No ads, no data sale, ever — write it into governance, not just a blog post.** Put a clause in the project's foundational documents (or a foundation/non-profit umbrella if it grows) committing to this, the way Signal's foundation structure forecloses an acquisition-driven monetization pivot.

This gets you recurring revenue proportional to actual operating cost (hosting), keeps the open-source promise fully intact (nothing is ever taken away or held back from self-hosters), and avoids the open-core trap of deciding which features are "enterprise" in a consumer app where that distinction doesn't really exist.

## 9. Suggested first 90 days

1. Stand up the Rust core repo with the CalDAV client/server libraries and an RFC 5545 conformance suite seeded from §2.1's real-world bug corpus (last-weekday-of-month rules, minutely recurrence, edited single occurrences, EXDATE round-tripping, timezone-on-import, zero-duration events) — get this rock-solid before any UI exists. This corpus alone, run against the eventual core, should be a credible public marketing point ("we test against every recurrence bug that's plagued Etar and Fossify for years").
2. Build a CLI or minimal test harness against the core to validate sync correctness with real CalDAV servers (Nextcloud, Radicale, Fastmail) before touching mobile UI.
3. Prototype the EteSync-style local CalDAV adapter early (§5.5) — it's the riskiest, least-precedented-outside-EteSync piece of the architecture, and de-risking it before committing to the mobile UI shells avoids a late architectural surprise.
4. Ship a barebones Flutter/React Native client that only does local + CalDAV sync — no native sync service yet. This alone is already a better product than most current open-source calendar clients and gets early adopters and contributors in the door.
5. Write the ADRs and CONTRIBUTING guide *before* the first public release, not after.
6. Only then build the native sync service and the paid hosted tier, once the core protocol and data model are stable enough that you're not redesigning the schema underneath paying users.
