# Recurrence / iCalendar conformance corpus

See system-design.md §11.1. These fixtures are seeded directly from real, documented bugs in
competing open-source calendar clients (Fossify Calendar, Etar) — not hypothetical edge cases.
**This suite must stay at 100% passing** (AGENTS.md rule 3); a regression here is not minor.

Each fixture should be exercised as: parse → expand occurrences over the date range noted below →
assert against the expected result → serialize → re-parse → assert the round trip is equivalent
or better than the original. Wire these up as `#[test]` functions in `songbird-recurrence` and
`songbird-ical` once M1 implementation starts — this README is the assertion spec, the `.ics`
files are the inputs.

| Fixture | Bug it regresses | Expected behavior |
|---|---|---|
| `last_weekday_of_month.ics` | `BYDAY=-1SA` computed as a fixed "4th Saturday" instead of the true last Saturday | Expand Jan–Jun of the test year; assert each occurrence lands on the actual last Saturday of its month, including the month(s) with a 5th Saturday |
| `minutely_recurrence.ics` | `FREQ=MINUTELY;INTERVAL=5;COUNT=100` silently collapsed to a single non-recurring event | Expand and assert exactly 100 occurrences, 5 minutes apart, starting at DTSTART |
| `edited_single_occurrence.ics` | An edited single occurrence (EXDATE + override VEVENT with matching RECURRENCE-ID) vanishes from the expanded series instead of replacing the base occurrence | Expand across the series; assert the overridden date shows the override's modified fields exactly once, not duplicated, not dropped |
| `exdate_value_type_match.ics` | Serialized EXDATE used a `DATE` value against a `DATE-TIME`-typed DTSTART, producing an file that fails re-import | Parse → serialize → re-parse; assert the serialized EXDATE's VALUE type matches DTSTART's VALUE type, and the round trip succeeds |
| `explicit_tzid_import.ics` | An event with an explicit TZID silently reinterpreted in the device's local timezone instead of the file's | Parse with the local/device timezone set to something other than the file's TZID; assert the parsed event retains the file's TZID and wall-clock time |
| `zero_duration_event.ics` | Events where DTEND == DTSTART (legal per RFC 5545) fail to sync / display | Parse; assert the event is retained with DTEND == DTSTART, not dropped or defaulted to a nonzero duration |
| `weekday_repeat_rule.ics` | `BYDAY=MO,TU,WE,TH,FR` ("every weekday") incorrectly expanded to every single day including weekends | Expand across 3 weeks; assert occurrences fall only on Mon–Fri |

Add a new fixture here, with a row in this table, any time a new recurrence/iCalendar bug is
found in the wild or in our own implementation — per CONTRIBUTING.md.
