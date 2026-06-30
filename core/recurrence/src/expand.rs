use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};

use crate::types::*;

/// Expand a recurrence rule into concrete occurrences within `range`.
///
/// Algorithm follows RFC 5545 §3.8.5: generate per-frequency periods, expand BY*
/// rules within each period, apply BYSETPOS, remove EXDATEs, apply overrides,
/// then filter to the requested range while honouring COUNT/UNTIL.
pub fn expand_occurrences(
    rule: &RecurrenceRule,
    dtstart: &DateOrDateTime,
    rdate: &[DateOrDateTime],
    exdate: &[DateOrDateTime],
    overrides: &[EventOverride],
    range: &DateRange,
) -> Vec<Occurrence> {
    let mut results: Vec<DateOrDateTime> = Vec::new();

    match dtstart {
        DateOrDateTime::Date(start_date) => {
            generate_date_occurrences(rule, *start_date, &mut results);
        }
        DateOrDateTime::DateTime { local: start_dt, tzid, is_utc } => {
            generate_datetime_occurrences(rule, *start_dt, tzid.clone(), *is_utc, &mut results);
        }
    }

    // Add RDATE values (they bypass RRULE, but still get EXDATE applied)
    for rd in rdate {
        results.push(rd.clone());
    }

    results.sort();
    results.dedup();

    // Remove EXDATEs (date-only comparison for DATE type; full NaiveDate for DATE-TIME)
    results.retain(|occ| !exdate.iter().any(|ex| ex.same_date(occ)));

    // Apply overrides: remove the base occurrence matching RECURRENCE-ID, note its position
    // then later emit the override's start at the same slot.
    let mut override_map: Vec<(DateOrDateTime, DateOrDateTime)> = overrides
        .iter()
        .map(|ov| (ov.recurrence_id.clone(), ov.dtstart.clone()))
        .collect();

    let mut final_occurrences: Vec<Occurrence> = results
        .into_iter()
        .filter_map(|occ| {
            if let Some(pos) = override_map.iter().position(|(rid, _)| rid.same_date(&occ)) {
                let (_, override_start) = override_map.remove(pos);
                Some(Occurrence { start: override_start, is_override: true })
            } else {
                Some(Occurrence { start: occ, is_override: false })
            }
        })
        .collect();

    // Filter to the requested date range
    final_occurrences.retain(|occ| range.contains(&occ.start));

    final_occurrences
}

// ─── Date-only generation ────────────────────────────────────────────────────

fn generate_date_occurrences(
    rule: &RecurrenceRule,
    start: NaiveDate,
    out: &mut Vec<DateOrDateTime>,
) {
    let max = rule.count.unwrap_or(u32::MAX);
    let mut emitted = 0u32;

    // Safety cap for open-ended rules (should not be hit in conformance tests)
    const SAFETY_CAP: u32 = 50_000;

    match rule.freq {
        Frequency::Minutely | Frequency::Secondly | Frequency::Hourly => {
            // Date-only events have no time component; sub-day frequencies collapse to one per day
            if should_emit_date(rule, start) {
                out.push(DateOrDateTime::Date(start));
            }
        }

        Frequency::Daily => {
            let mut current = start;
            while emitted < max && emitted < SAFETY_CAP {
                if should_emit_date(rule, current) {
                    out.push(DateOrDateTime::Date(current));
                    emitted += 1;
                    if emitted >= max { break; }
                    if past_until_date(rule, current) { break; }
                }
                current = current + Duration::days(rule.interval as i64);
                if past_until_date(rule, current) { break; }
            }
        }

        Frequency::Weekly => {
            let period_start = week_start(start, rule.wkst);
            weekly_date(rule, period_start, start, max, SAFETY_CAP, out);
        }

        Frequency::Monthly => {
            monthly_date(rule, start, max, SAFETY_CAP, out);
        }

        Frequency::Yearly => {
            yearly_date(rule, start, max, SAFETY_CAP, out);
        }
    }
}

fn weekly_date(
    rule: &RecurrenceRule,
    period_start: NaiveDate,
    dtstart: NaiveDate,
    max: u32,
    cap: u32,
    out: &mut Vec<DateOrDateTime>,
) {
    let mut emitted = 0u32;
    let mut period = period_start;
    let weekdays = effective_weekdays(rule, dtstart.weekday());

    'outer: loop {
        let mut period_candidates: Vec<NaiveDate> = weekdays
            .iter()
            .map(|&wd| period + Duration::days(days_until_weekday(period.weekday(), wd) as i64))
            .filter(|&d| d >= dtstart)
            .collect();

        if !rule.bymonth.is_empty() {
            period_candidates.retain(|d| rule.bymonth.contains(&(d.month() as u8)));
        }
        period_candidates.sort();

        for d in period_candidates {
            if past_until_date(rule, d) { break 'outer; }
            out.push(DateOrDateTime::Date(d));
            emitted += 1;
            if emitted >= max || emitted >= cap { break 'outer; }
        }

        period = period + Duration::weeks(rule.interval as i64);
        // If period is too far past UNTIL, stop
        if let Some(DateOrDateTime::Date(u)) = &rule.until {
            if period > *u { break; }
        }
    }
}

fn monthly_date(
    rule: &RecurrenceRule,
    dtstart: NaiveDate,
    max: u32,
    cap: u32,
    out: &mut Vec<DateOrDateTime>,
) {
    let mut emitted = 0u32;
    let mut year = dtstart.year();
    let mut month = dtstart.month();

    'outer: loop {
        if !rule.bymonth.is_empty() && !rule.bymonth.contains(&(month as u8)) {
            advance_month(&mut year, &mut month, rule.interval);
            continue;
        }

        let mut candidates = monthly_candidates_date(rule, year, month, dtstart);
        apply_bysetpos_dates(&mut candidates, &rule.bysetpos);
        candidates.sort();

        for d in candidates {
            if d < dtstart { continue; }
            if past_until_date(rule, d) { break 'outer; }
            out.push(DateOrDateTime::Date(d));
            emitted += 1;
            if emitted >= max || emitted >= cap { break 'outer; }
        }

        advance_month(&mut year, &mut month, rule.interval);

        if let Some(DateOrDateTime::Date(u)) = &rule.until {
            if NaiveDate::from_ymd_opt(year, month, 1).map_or(false, |d| d > *u) {
                break;
            }
        }
    }
}

fn monthly_candidates_date(
    rule: &RecurrenceRule,
    year: i32,
    month: u32,
    dtstart: NaiveDate,
) -> Vec<NaiveDate> {
    if !rule.byday.is_empty() && !rule.bymonthday.is_empty() {
        // RFC 5545: when both BYDAY and BYMONTHDAY present with FREQ=MONTHLY,
        // BYDAY limits the set produced by BYMONTHDAY.
        let monthdays: Vec<NaiveDate> = rule
            .bymonthday
            .iter()
            .filter_map(|&d| resolve_monthday(year, month, d))
            .collect();
        monthdays
            .into_iter()
            .filter(|d| rule.byday.iter().any(|wd| wd.ordinal.is_none() && wd.weekday == d.weekday()))
            .collect()
    } else if !rule.byday.is_empty() {
        rule.byday
            .iter()
            .filter_map(|wd| expand_byday_monthly(year, month, wd))
            .flatten()
            .collect()
    } else if !rule.bymonthday.is_empty() {
        rule.bymonthday
            .iter()
            .filter_map(|&d| resolve_monthday(year, month, d))
            .collect()
    } else {
        // Default: same day-of-month as DTSTART, clamped to month end
        resolve_monthday(year, month, dtstart.day() as i32)
            .into_iter()
            .collect()
    }
}

fn yearly_date(
    rule: &RecurrenceRule,
    dtstart: NaiveDate,
    max: u32,
    cap: u32,
    out: &mut Vec<DateOrDateTime>,
) {
    let mut emitted = 0u32;
    let mut year = dtstart.year();

    'outer: loop {
        let months: Vec<u32> = if rule.bymonth.is_empty() {
            vec![dtstart.month()]
        } else {
            rule.bymonth.iter().map(|&m| m as u32).collect()
        };

        for month in &months {
            let candidates = monthly_candidates_date(rule, year, *month, dtstart);
            for d in candidates {
                if d < dtstart { continue; }
                if past_until_date(rule, d) { break 'outer; }
                out.push(DateOrDateTime::Date(d));
                emitted += 1;
                if emitted >= max || emitted >= cap { break 'outer; }
            }
        }

        year += rule.interval as i32;

        if let Some(DateOrDateTime::Date(u)) = &rule.until {
            if NaiveDate::from_ymd_opt(year, 1, 1).map_or(false, |d| d > *u) {
                break;
            }
        }
    }
}

// ─── DateTime generation ─────────────────────────────────────────────────────

fn generate_datetime_occurrences(
    rule: &RecurrenceRule,
    start: NaiveDateTime,
    tzid: Option<String>,
    is_utc: bool,
    out: &mut Vec<DateOrDateTime>,
) {
    let max = rule.count.unwrap_or(u32::MAX);
    const SAFETY_CAP: u32 = 50_000;

    let emit = |dt: NaiveDateTime, out: &mut Vec<DateOrDateTime>| {
        out.push(DateOrDateTime::DateTime {
            local: dt,
            tzid: tzid.clone(),
            is_utc,
        });
    };

    match rule.freq {
        Frequency::Secondly => {
            let mut current = start;
            let mut emitted = 0u32;
            while emitted < max && emitted < SAFETY_CAP {
                if past_until_dt(rule, current) { break; }
                emit(current, out);
                emitted += 1;
                current = current + Duration::seconds(rule.interval as i64);
            }
        }

        Frequency::Minutely => {
            let mut current = start;
            let mut emitted = 0u32;
            while emitted < max && emitted < SAFETY_CAP {
                if past_until_dt(rule, current) { break; }
                // Expand BYSECOND within each minute
                let seconds = effective_seconds(rule, current.second());
                for sec in &seconds {
                    let candidate = current.with_second(*sec as u32).unwrap_or(current);
                    if candidate < start { continue; }
                    if past_until_dt(rule, candidate) { return; }
                    emit(candidate, out);
                    emitted += 1;
                    if emitted >= max || emitted >= SAFETY_CAP { return; }
                }
                current = current + Duration::minutes(rule.interval as i64);
            }
        }

        Frequency::Hourly => {
            let mut current = start;
            let mut emitted = 0u32;
            while emitted < max && emitted < SAFETY_CAP {
                if past_until_dt(rule, current) { break; }
                let minutes = effective_minutes(rule, current.minute());
                for min in &minutes {
                    let candidate = current.with_minute(*min as u32).unwrap_or(current);
                    if candidate < start { continue; }
                    emit(candidate, out);
                    emitted += 1;
                    if emitted >= max || emitted >= SAFETY_CAP { return; }
                }
                current = current + Duration::hours(rule.interval as i64);
            }
        }

        Frequency::Daily => {
            let mut current = start;
            let mut emitted = 0u32;
            while emitted < max && emitted < SAFETY_CAP {
                if past_until_dt(rule, current) { break; }
                if !rule.bymonth.is_empty() && !rule.bymonth.contains(&(current.month() as u8)) {
                    current = current + Duration::days(rule.interval as i64);
                    continue;
                }
                emit(current, out);
                emitted += 1;
                current = current + Duration::days(rule.interval as i64);
            }
        }

        Frequency::Weekly => {
            let start_date = start.date();
            let start_time = start.time();
            let period_start = week_start(start_date, rule.wkst);
            let weekdays = effective_weekdays(rule, start_date.weekday());
            let mut period = period_start;
            let mut emitted = 0u32;

            'outer: loop {
                let mut period_dates: Vec<NaiveDate> = weekdays
                    .iter()
                    .map(|&wd| {
                        period + Duration::days(days_until_weekday(period.weekday(), wd) as i64)
                    })
                    .filter(|&d| d >= start_date)
                    .collect();

                if !rule.bymonth.is_empty() {
                    period_dates.retain(|d| rule.bymonth.contains(&(d.month() as u8)));
                }
                period_dates.sort();

                for d in period_dates {
                    let candidate = d.and_time(start_time);
                    if past_until_dt(rule, candidate) { break 'outer; }
                    emit(candidate, out);
                    emitted += 1;
                    if emitted >= max || emitted >= SAFETY_CAP { break 'outer; }
                }

                period = period + Duration::weeks(rule.interval as i64);
                if let Some(u) = until_naive_date(rule) {
                    if period > u { break; }
                }
            }
        }

        Frequency::Monthly => {
            let start_time = start.time();
            let mut year = start.year();
            let mut month = start.month();
            let mut emitted = 0u32;

            'outer: loop {
                if !rule.bymonth.is_empty() && !rule.bymonth.contains(&(month as u8)) {
                    advance_month(&mut year, &mut month, rule.interval);
                    continue;
                }

                let mut candidates =
                    monthly_candidates_date(rule, year, month, start.date());
                apply_bysetpos_dates(&mut candidates, &rule.bysetpos);
                candidates.sort();

                for d in candidates {
                    if d < start.date() { continue; }
                    let candidate = d.and_time(start_time);
                    if past_until_dt(rule, candidate) { break 'outer; }
                    emit(candidate, out);
                    emitted += 1;
                    if emitted >= max || emitted >= SAFETY_CAP { break 'outer; }
                }

                advance_month(&mut year, &mut month, rule.interval);
                if let Some(u) = until_naive_date(rule) {
                    if NaiveDate::from_ymd_opt(year, month, 1).map_or(false, |d| d > u) {
                        break;
                    }
                }
            }
        }

        Frequency::Yearly => {
            let start_time = start.time();
            let mut year = start.year();
            let mut emitted = 0u32;

            'outer: loop {
                let months: Vec<u32> = if rule.bymonth.is_empty() {
                    vec![start.month()]
                } else {
                    rule.bymonth.iter().map(|&m| m as u32).collect()
                };

                for month in &months {
                    let mut candidates = monthly_candidates_date(rule, year, *month, start.date());
                    candidates.sort();
                    for d in candidates {
                        if d < start.date() { continue; }
                        let candidate = d.and_time(start_time);
                        if past_until_dt(rule, candidate) { break 'outer; }
                        emit(candidate, out);
                        emitted += 1;
                        if emitted >= max || emitted >= SAFETY_CAP { break 'outer; }
                    }
                }

                year += rule.interval as i32;
                if let Some(u) = until_naive_date(rule) {
                    if NaiveDate::from_ymd_opt(year, 1, 1).map_or(false, |d| d > u) {
                        break;
                    }
                }
            }
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Expand BYDAY for a single month: returns all matching NaiveDates.
fn expand_byday_monthly(year: i32, month: u32, wd: &WeekdayOrdinal) -> Option<Vec<NaiveDate>> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let last = last_day_of_month(year, month)?;

    if let Some(ordinal) = wd.ordinal {
        // Specific ordinal: find the n-th (or n-th from end) occurrence of wd.weekday in month.
        let date = if ordinal > 0 {
            let days_to_first =
                days_until_weekday(first.weekday(), wd.weekday) as i64;
            let first_occ = first + Duration::days(days_to_first);
            if first_occ.month() != month { return Some(vec![]); }
            let nth = first_occ + Duration::weeks((ordinal - 1) as i64);
            if nth.month() != month { None } else { Some(nth) }
        } else {
            // Negative ordinal
            let days_back =
                days_back_to_weekday(last.weekday(), wd.weekday) as i64;
            let last_occ = last - Duration::days(days_back);
            if last_occ.month() != month { return Some(vec![]); }
            let nth = last_occ - Duration::weeks((-ordinal - 1) as i64);
            if nth.month() != month { None } else { Some(nth) }
        };
        Some(date.into_iter().collect())
    } else {
        // No ordinal: all occurrences of wd.weekday in the month.
        let days_to_first = days_until_weekday(first.weekday(), wd.weekday) as i64;
        let mut current = first + Duration::days(days_to_first);
        let mut dates = Vec::new();
        while current.month() == month && current <= last {
            dates.push(current);
            current = current + Duration::weeks(1);
        }
        Some(dates)
    }
}

/// Resolve a (possibly negative) BYMONTHDAY value to a concrete NaiveDate.
fn resolve_monthday(year: i32, month: u32, day: i32) -> Option<NaiveDate> {
    if day > 0 {
        NaiveDate::from_ymd_opt(year, month, day as u32)
    } else if day < 0 {
        let last = last_day_of_month(year, month)?;
        let target = last.day() as i32 + day + 1;
        if target < 1 { None } else { NaiveDate::from_ymd_opt(year, month, target as u32) }
    } else {
        None // 0 is invalid per RFC 5545
    }
}

fn last_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .map(|d| d - Duration::days(1))
}

/// Days forward from `from` to reach `to` (0 if same weekday).
fn days_until_weekday(from: Weekday, to: Weekday) -> u32 {
    (to.num_days_from_monday() + 7 - from.num_days_from_monday()) % 7
}

/// Days backward from `from` to reach `to` (0 if same weekday).
fn days_back_to_weekday(from: Weekday, to: Weekday) -> u32 {
    (from.num_days_from_monday() + 7 - to.num_days_from_monday()) % 7
}

/// Monday-aligned week start for a date, respecting WKST.
fn week_start(date: NaiveDate, wkst: Weekday) -> NaiveDate {
    let offset = days_back_to_weekday(date.weekday(), wkst) as i64;
    date - Duration::days(offset)
}

/// Effective weekday list: BYDAY if specified, otherwise dtstart's weekday.
fn effective_weekdays(rule: &RecurrenceRule, dtstart_weekday: Weekday) -> Vec<Weekday> {
    if rule.byday.is_empty() {
        vec![dtstart_weekday]
    } else {
        rule.byday.iter().map(|w| w.weekday).collect()
    }
}

fn effective_seconds(rule: &RecurrenceRule, start_sec: u32) -> Vec<u32> {
    if rule.bysecond.is_empty() {
        vec![start_sec]
    } else {
        rule.bysecond.iter().map(|&s| s as u32).collect()
    }
}

fn effective_minutes(rule: &RecurrenceRule, start_min: u32) -> Vec<u32> {
    if rule.byminute.is_empty() {
        vec![start_min]
    } else {
        rule.byminute.iter().map(|&m| m as u32).collect()
    }
}

fn apply_bysetpos_dates(candidates: &mut Vec<NaiveDate>, bysetpos: &[i32]) {
    if bysetpos.is_empty() {
        return;
    }
    candidates.sort();
    let len = candidates.len();
    let mut selected: Vec<NaiveDate> = bysetpos
        .iter()
        .filter_map(|&pos| {
            let idx = if pos > 0 {
                (pos - 1) as usize
            } else {
                len.checked_sub((-pos) as usize)?
            };
            candidates.get(idx).copied()
        })
        .collect();
    selected.sort();
    *candidates = selected;
}

fn should_emit_date(rule: &RecurrenceRule, d: NaiveDate) -> bool {
    if !rule.bymonth.is_empty() && !rule.bymonth.contains(&(d.month() as u8)) {
        return false;
    }
    true
}

fn past_until_date(rule: &RecurrenceRule, d: NaiveDate) -> bool {
    match &rule.until {
        Some(DateOrDateTime::Date(u)) => d > *u,
        Some(DateOrDateTime::DateTime { local, .. }) => d > local.date(),
        None => false,
    }
}

fn past_until_dt(rule: &RecurrenceRule, dt: NaiveDateTime) -> bool {
    match &rule.until {
        Some(DateOrDateTime::Date(u)) => dt.date() > *u,
        Some(DateOrDateTime::DateTime { local, .. }) => dt > *local,
        None => false,
    }
}

fn until_naive_date(rule: &RecurrenceRule) -> Option<NaiveDate> {
    match &rule.until {
        Some(DateOrDateTime::Date(d)) => Some(*d),
        Some(DateOrDateTime::DateTime { local, .. }) => Some(local.date()),
        None => None,
    }
}

fn advance_month(year: &mut i32, month: &mut u32, interval: u32) {
    let total = *month as i32 - 1 + interval as i32;
    *year += total / 12;
    *month = (total % 12 + 1) as u32;
}

// Silence unused-import warning for NaiveTime (used via trait in .time())
const _: fn() -> NaiveTime = || NaiveTime::from_hms_opt(0, 0, 0).unwrap();
