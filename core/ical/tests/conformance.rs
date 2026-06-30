use songbird_ical::{
    parse_icalendar, serialize_icalendar, DateOrDateTime, EventStatus, IcalError,
};

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/../tests/conformance/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"))
}

fn roundtrip(ics: &str) -> Result<(), IcalError> {
    let cal = parse_icalendar(ics)?;
    let serialized = serialize_icalendar(&cal);
    let cal2 = parse_icalendar(&serialized)?;
    assert_eq!(cal.events.len(), cal2.events.len());
    for (a, b) in cal.events.iter().zip(&cal2.events) {
        assert_eq!(a.uid, b.uid);
        assert_eq!(a.dtstart, b.dtstart, "dtstart changed for {}", a.uid);
        assert_eq!(a.dtend, b.dtend, "dtend changed for {}", a.uid);
        assert_eq!(a.rrule, b.rrule, "rrule changed for {}", a.uid);
        assert_eq!(a.exdate, b.exdate, "exdate changed for {}", a.uid);
        assert_eq!(a.recurrence_id, b.recurrence_id, "recurrence_id changed for {}", a.uid);
    }
    Ok(())
}

#[test]
fn exdate_value_type_matches_dtstart() {
    let ics = fixture("exdate_value_type_match.ics");
    let cal = parse_icalendar(&ics).unwrap();
    let event = &cal.events[0];

    assert!(
        matches!(event.dtstart, DateOrDateTime::Date(_)),
        "expected DATE dtstart, got {:?}",
        event.dtstart
    );
    assert_eq!(event.exdate.len(), 1);
    assert!(
        matches!(event.exdate[0], DateOrDateTime::Date(_)),
        "EXDATE must be DATE-type to match DTSTART, got {:?}",
        event.exdate[0]
    );

    let serialized = serialize_icalendar(&cal);
    assert!(
        serialized.contains("EXDATE;VALUE=DATE:"),
        "serialized output must use EXDATE;VALUE=DATE:, got:\n{serialized}"
    );

    roundtrip(&ics).unwrap();
}

#[test]
fn explicit_tzid_preserved_on_import() {
    let ics = fixture("explicit_tzid_import.ics");
    let cal = parse_icalendar(&ics).unwrap();
    let event = &cal.events[0];

    match &event.dtstart {
        DateOrDateTime::DateTime { local, tzid, is_utc } => {
            assert_eq!(tzid.as_deref(), Some("Europe/Berlin"));
            assert!(!is_utc);
            assert_eq!(local.to_string(), "2026-05-27 11:45:00");
        }
        other => panic!("expected DateTime, got {other:?}"),
    }

    match &event.dtend {
        Some(DateOrDateTime::DateTime { local, tzid, .. }) => {
            assert_eq!(tzid.as_deref(), Some("Europe/Berlin"));
            assert_eq!(local.to_string(), "2026-05-27 12:30:00");
        }
        other => panic!("expected DateTime dtend, got {other:?}"),
    }

    assert!(!cal.timezones.is_empty(), "VTIMEZONE must be parsed");
    assert_eq!(cal.timezones[0].tzid, "Europe/Berlin");

    roundtrip(&ics).unwrap();
}

#[test]
fn zero_duration_event_is_preserved() {
    let ics = fixture("zero_duration_event.ics");
    let cal = parse_icalendar(&ics).unwrap();
    let event = &cal.events[0];

    assert_eq!(
        event.dtstart,
        event.dtend.clone().unwrap(),
        "DTEND must equal DTSTART"
    );
    assert_eq!(event.status, Some(EventStatus::Confirmed));

    roundtrip(&ics).unwrap();
}

#[test]
fn edited_single_occurrence_parses_both_vevents() {
    let ics = fixture("edited_single_occurrence.ics");
    let cal = parse_icalendar(&ics).unwrap();

    assert_eq!(cal.events.len(), 2, "must parse both base and override VEVENT");

    let base = cal.events.iter().find(|e| e.recurrence_id.is_none()).unwrap();
    assert!(base.rrule.is_some());
    assert_eq!(base.sequence, 1);

    let ov = cal.events.iter().find(|e| e.recurrence_id.is_some()).unwrap();
    assert_eq!(
        ov.recurrence_id.as_ref().unwrap().naive_date(),
        chrono::NaiveDate::from_ymd_opt(2026, 2, 24).unwrap(),
    );
    assert_eq!(
        ov.dtstart.naive_date(),
        chrono::NaiveDate::from_ymd_opt(2026, 2, 25).unwrap(),
    );

    roundtrip(&ics).unwrap();
}
