# core/tests/conformance/

Seed corpus of real-world iCalendar and recurrence bugs — each `.ics` file encodes a case where
a competing calendar app (Nextcloud, Etar, Fossify, etc.) produced the wrong result. Tests in
`songbird-recurrence` and `songbird-ical` load these fixtures and assert correct behaviour.

This suite must stay at **100% pass rate** before any commit that touches `recurrence/` or
`ical/`. A regression here means we've reintroduced a bug that was already found in the wild.

See `docs/design/system-design.md` §11.1 for the full rationale.
