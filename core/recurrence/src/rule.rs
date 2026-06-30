use chrono::{NaiveDate, NaiveDateTime, Weekday};

use crate::{RecurrenceError, types::*};

pub fn parse_rrule(raw: &str) -> Result<RecurrenceRule, RecurrenceError> {
    let mut freq = None;
    let mut interval = 1u32;
    let mut count: Option<u32> = None;
    let mut until: Option<DateOrDateTime> = None;
    let mut bysecond = Vec::new();
    let mut byminute = Vec::new();
    let mut byhour = Vec::new();
    let mut byday = Vec::new();
    let mut bymonthday = Vec::new();
    let mut byyearday = Vec::new();
    let mut byweekno = Vec::new();
    let mut bymonth = Vec::new();
    let mut bysetpos = Vec::new();
    let mut wkst = Weekday::Mon;

    for part in raw.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (key, value) = part.split_once('=').ok_or_else(|| {
            RecurrenceError::InvalidRrule(format!("missing '=' in part: {part}"))
        })?;

        match key.trim().to_uppercase().as_str() {
            "FREQ" => freq = Some(parse_freq(value)?),
            "INTERVAL" => {
                interval = value.trim().parse().map_err(|_| {
                    RecurrenceError::InvalidRrule(format!("bad INTERVAL: {value}"))
                })?;
            }
            "COUNT" => {
                count = Some(value.trim().parse().map_err(|_| {
                    RecurrenceError::InvalidRrule(format!("bad COUNT: {value}"))
                })?);
            }
            "UNTIL" => {
                until = Some(parse_until(value.trim())?);
            }
            "BYSECOND" => bysecond = parse_int_list(value)?,
            "BYMINUTE" => byminute = parse_int_list(value)?,
            "BYHOUR" => byhour = parse_int_list(value)?,
            "BYDAY" => byday = parse_byday_list(value)?,
            "BYMONTHDAY" => bymonthday = parse_int_list(value)?,
            "BYYEARDAY" => byyearday = parse_int_list(value)?,
            "BYWEEKNO" => byweekno = parse_int_list(value)?,
            "BYMONTH" => {
                bymonth = parse_int_list(value)?
                    .into_iter()
                    .map(|x| x as u8)
                    .collect();
            }
            "BYSETPOS" => bysetpos = parse_int_list(value)?,
            "WKST" => wkst = parse_weekday(value.trim())?,
            _ => {} // tolerate unknown properties per RFC
        }
    }

    let freq = freq.ok_or_else(|| RecurrenceError::InvalidRrule("missing FREQ".into()))?;

    Ok(RecurrenceRule {
        freq,
        interval,
        count,
        until,
        bysecond,
        byminute,
        byhour,
        byday,
        bymonthday,
        byyearday,
        byweekno,
        bymonth,
        bysetpos,
        wkst,
        raw: raw.to_string(),
    })
}

fn parse_freq(s: &str) -> Result<Frequency, RecurrenceError> {
    match s.trim().to_uppercase().as_str() {
        "SECONDLY" => Ok(Frequency::Secondly),
        "MINUTELY" => Ok(Frequency::Minutely),
        "HOURLY" => Ok(Frequency::Hourly),
        "DAILY" => Ok(Frequency::Daily),
        "WEEKLY" => Ok(Frequency::Weekly),
        "MONTHLY" => Ok(Frequency::Monthly),
        "YEARLY" => Ok(Frequency::Yearly),
        other => Err(RecurrenceError::InvalidRrule(format!("unknown FREQ: {other}"))),
    }
}

pub(crate) fn parse_weekday(s: &str) -> Result<Weekday, RecurrenceError> {
    match s.to_uppercase().as_str() {
        "MO" => Ok(Weekday::Mon),
        "TU" => Ok(Weekday::Tue),
        "WE" => Ok(Weekday::Wed),
        "TH" => Ok(Weekday::Thu),
        "FR" => Ok(Weekday::Fri),
        "SA" => Ok(Weekday::Sat),
        "SU" => Ok(Weekday::Sun),
        other => Err(RecurrenceError::InvalidRrule(format!("unknown weekday: {other}"))),
    }
}

fn parse_int_list(s: &str) -> Result<Vec<i32>, RecurrenceError> {
    s.split(',')
        .map(|v| {
            v.trim().parse::<i32>().map_err(|_| {
                RecurrenceError::InvalidRrule(format!("bad integer in list: {v}"))
            })
        })
        .collect()
}

fn parse_byday_list(s: &str) -> Result<Vec<WeekdayOrdinal>, RecurrenceError> {
    s.split(',').map(|v| parse_byday_entry(v.trim())).collect()
}

/// Parse a BYDAY entry: optional signed integer prefix + weekday code.
/// Examples: "MO", "-1SA", "+2FR", "1MO".
fn parse_byday_entry(s: &str) -> Result<WeekdayOrdinal, RecurrenceError> {
    // Weekday code is always the last 2 characters.
    if s.len() < 2 {
        return Err(RecurrenceError::InvalidRrule(format!("bad BYDAY entry: {s}")));
    }
    let (prefix, day_code) = s.split_at(s.len() - 2);
    let weekday = parse_weekday(day_code)?;
    let ordinal = if prefix.is_empty() {
        None
    } else {
        Some(
            prefix.parse::<i32>().map_err(|_| {
                RecurrenceError::InvalidRrule(format!("bad ordinal in BYDAY entry: {s}"))
            })?,
        )
    };
    Ok(WeekdayOrdinal { ordinal, weekday })
}

fn parse_until(s: &str) -> Result<DateOrDateTime, RecurrenceError> {
    if s.contains('T') {
        let is_utc = s.ends_with('Z');
        let dt_str = s.trim_end_matches('Z');
        let local = NaiveDateTime::parse_from_str(dt_str, "%Y%m%dT%H%M%S").map_err(|e| {
            RecurrenceError::InvalidRrule(format!("bad UNTIL datetime '{s}': {e}"))
        })?;
        Ok(DateOrDateTime::DateTime { local, tzid: None, is_utc })
    } else {
        let d = NaiveDate::parse_from_str(s, "%Y%m%d").map_err(|e| {
            RecurrenceError::InvalidRrule(format!("bad UNTIL date '{s}': {e}"))
        })?;
        Ok(DateOrDateTime::Date(d))
    }
}
