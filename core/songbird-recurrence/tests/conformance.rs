use chrono::NaiveDate;
use songbird_recurrence::{
    expand_occurrences, parse_rrule,
    types::{DateOrDateTime, DateRange, EventOverride},
};

fn date(y: i32, m: u32, d: u32) -> DateOrDateTime {
    DateOrDateTime::Date(NaiveDate::from_ymd_opt(y, m, d).unwrap())
}

fn wide_range() -> DateRange {
    DateRange { start: date(2020, 1, 1), end: date(2030, 1, 1) }
}

#[test]
fn last_saturday_of_month() {
    let dtstart = DateOrDateTime::DateTime {
        local: NaiveDate::from_ymd_opt(2026, 1, 3).unwrap().and_hms_opt(10, 0, 0).unwrap(),
        tzid: Some("America/New_York".into()),
        is_utc: false,
    };
    let rule = parse_rrule("FREQ=MONTHLY;BYDAY=-1SA;COUNT=6").unwrap();
    let occs = expand_occurrences(&rule, &dtstart, &[], &[], &[], &wide_range());

    assert_eq!(occs.len(), 6);

    // Jan and May 2026 each have 5 Saturdays; the result must be the last (5th), not the 4th.
    let expected = [
        (2026, 1, 31),
        (2026, 2, 28),
        (2026, 3, 28),
        (2026, 4, 25),
        (2026, 5, 30),
        (2026, 6, 27),
    ];
    for (i, ((y, m, d), occ)) in expected.iter().zip(&occs).enumerate() {
        assert_eq!(
            occ.start.naive_date(),
            NaiveDate::from_ymd_opt(*y, *m, *d).unwrap(),
            "occurrence {i}: expected {y}-{m:02}-{d:02}, got {:?}",
            occ.start.naive_date()
        );
    }
}

#[test]
fn minutely_recurrence_expands_fully() {
    let dtstart = DateOrDateTime::DateTime {
        local: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        tzid: Some("UTC".into()),
        is_utc: false,
    };
    let rule = parse_rrule("FREQ=MINUTELY;INTERVAL=5;COUNT=100").unwrap();
    let occs = expand_occurrences(&rule, &dtstart, &[], &[], &[], &wide_range());

    assert_eq!(occs.len(), 100);
    assert_eq!(
        occs[0].start.naive_datetime().unwrap(),
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap()
    );
    assert_eq!(
        occs[99].start.naive_datetime().unwrap(),
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(8, 15, 0).unwrap()
    );
    for pair in occs.windows(2) {
        let a = pair[0].start.naive_datetime().unwrap();
        let b = pair[1].start.naive_datetime().unwrap();
        assert_eq!((b - a).num_minutes(), 5);
    }
}

#[test]
fn edited_occurrence_replaces_not_duplicates() {
    let dtstart = date(2026, 1, 24);
    let rule = parse_rrule("FREQ=MONTHLY;COUNT=6;BYMONTHDAY=24").unwrap();
    let overrides = [EventOverride {
        recurrence_id: date(2026, 2, 24),
        dtstart: date(2026, 2, 25),
    }];

    let occs = expand_occurrences(&rule, &dtstart, &[], &[], &overrides, &wide_range());

    assert_eq!(occs.len(), 6, "no duplicates, no drops");

    let dates: Vec<NaiveDate> = occs.iter().map(|o| o.start.naive_date()).collect();
    assert!(dates.contains(&NaiveDate::from_ymd_opt(2026, 1, 24).unwrap()));
    assert!(!dates.contains(&NaiveDate::from_ymd_opt(2026, 2, 24).unwrap()), "base Feb 24 must be gone");
    assert!(dates.contains(&NaiveDate::from_ymd_opt(2026, 2, 25).unwrap()), "override Feb 25 must be present");
    assert!(dates.contains(&NaiveDate::from_ymd_opt(2026, 3, 24).unwrap()));
    assert!(dates.contains(&NaiveDate::from_ymd_opt(2026, 6, 24).unwrap()));
}

#[test]
fn exdate_removes_correct_occurrence() {
    let dtstart = date(2026, 1, 6);
    let rule = parse_rrule("FREQ=WEEKLY;INTERVAL=1;COUNT=9;BYDAY=TU").unwrap();
    let exdate = [date(2026, 2, 3)];

    let occs = expand_occurrences(&rule, &dtstart, &[], &exdate, &[], &wide_range());

    assert_eq!(occs.len(), 8);
    let dates: Vec<NaiveDate> = occs.iter().map(|o| o.start.naive_date()).collect();
    assert!(!dates.contains(&NaiveDate::from_ymd_opt(2026, 2, 3).unwrap()));
    assert!(dates.contains(&NaiveDate::from_ymd_opt(2026, 1, 6).unwrap()));
    assert!(dates.contains(&NaiveDate::from_ymd_opt(2026, 3, 3).unwrap()));
}

#[test]
fn weekday_repeat_rule_excludes_weekends() {
    let dtstart = DateOrDateTime::DateTime {
        local: NaiveDate::from_ymd_opt(2026, 1, 5).unwrap().and_hms_opt(9, 15, 0).unwrap(),
        tzid: Some("America/Chicago".into()),
        is_utc: false,
    };
    let rule = parse_rrule("FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR;COUNT=15").unwrap();
    let occs = expand_occurrences(&rule, &dtstart, &[], &[], &[], &wide_range());

    assert_eq!(occs.len(), 15);

    use chrono::Datelike;
    for occ in &occs {
        let wd = occ.start.naive_date().weekday();
        assert!(
            matches!(
                wd,
                chrono::Weekday::Mon
                    | chrono::Weekday::Tue
                    | chrono::Weekday::Wed
                    | chrono::Weekday::Thu
                    | chrono::Weekday::Fri
            ),
            "occurrence on {:?} is not a weekday",
            occ.start.naive_date()
        );
    }

    // 5 weekdays/week × 3 weeks = 15th occurrence lands on Friday Jan 23
    assert_eq!(
        occs[14].start.naive_date(),
        NaiveDate::from_ymd_opt(2026, 1, 23).unwrap()
    );
}

#[test]
fn zero_duration_event_has_one_occurrence() {
    let dtstart = DateOrDateTime::DateTime {
        local: NaiveDate::from_ymd_opt(2026, 5, 27).unwrap().and_hms_opt(14, 0, 0).unwrap(),
        tzid: None,
        is_utc: true,
    };
    assert!(parse_rrule("").is_err());
    assert!(dtstart.naive_datetime().is_some());
}
