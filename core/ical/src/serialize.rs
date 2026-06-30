use crate::types::*;

/// Fold a content line at 75 octets per RFC 5545 §3.1, using CRLF + SPACE continuation.
fn fold_line(line: &str) -> String {
    let bytes = line.as_bytes();
    if bytes.len() <= 75 {
        return format!("{line}\r\n");
    }

    let mut out = String::new();
    let mut pos = 0;
    let mut first = true;

    while pos < bytes.len() {
        let limit = if first { 75 } else { 74 };
        let end = (pos + limit).min(bytes.len());

        // Don't split mid-UTF-8 sequence (continuation bytes start with 10xxxxxx)
        let mut safe_end = end;
        while safe_end > pos && bytes[safe_end - 1] & 0xC0 == 0x80 {
            safe_end -= 1;
        }
        if safe_end == pos {
            safe_end = end; // fallback: shouldn't happen with valid UTF-8
        }

        if !first {
            out.push(' ');
        }
        out.push_str(std::str::from_utf8(&bytes[pos..safe_end]).unwrap_or(""));
        out.push_str("\r\n");

        pos = safe_end;
        first = false;
    }

    out
}

/// RFC 5545 text-property escape: `,` `;` `\` `\n` must be backslash-escaped.
pub(crate) fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ',' => out.push_str("\\,"),
            ';' => out.push_str("\\;"),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out
}

/// Returns `(params_suffix, value)` for a date/time property.
/// e.g. (";VALUE=DATE", "20260124") or (";TZID=Europe/Berlin", "20260527T114500")
pub(crate) fn format_date_or_datetime(dt: &DateOrDateTime) -> (String, String) {
    match dt {
        DateOrDateTime::Date(d) => (";VALUE=DATE".into(), d.format("%Y%m%d").to_string()),
        DateOrDateTime::DateTime { local, tzid, is_utc } => {
            let val = local.format("%Y%m%dT%H%M%S").to_string();
            if *is_utc {
                (String::new(), format!("{val}Z"))
            } else if let Some(tz) = tzid {
                (format!(";TZID={tz}"), val)
            } else {
                (String::new(), val)
            }
        }
    }
}

pub fn serialize_vevent(event: &VEvent) -> String {
    let mut out = String::new();

    out.push_str("BEGIN:VEVENT\r\n");
    out.push_str(&fold_line(&format!("UID:{}", event.uid)));
    out.push_str(&fold_line(&format!("SUMMARY:{}", escape_text(&event.summary))));

    if let Some(desc) = &event.description {
        out.push_str(&fold_line(&format!("DESCRIPTION:{}", escape_text(desc))));
    }
    if let Some(loc) = &event.location {
        out.push_str(&fold_line(&format!("LOCATION:{}", escape_text(loc))));
    }

    let (p, v) = format_date_or_datetime(&event.dtstart);
    out.push_str(&fold_line(&format!("DTSTART{p}:{v}")));

    if let Some(dtend) = &event.dtend {
        let (p, v) = format_date_or_datetime(dtend);
        out.push_str(&fold_line(&format!("DTEND{p}:{v}")));
    }

    if let Some(rrule) = &event.rrule {
        out.push_str(&fold_line(&format!("RRULE:{rrule}")));
    }

    for rd in &event.rdate {
        let (p, v) = format_date_or_datetime(rd);
        out.push_str(&fold_line(&format!("RDATE{p}:{v}")));
    }

    // EXDATE: the VALUE type must match DTSTART (conformance fixture exdate_value_type_match).
    // Since parse_date_or_datetime preserves Date vs DateTime, the round-trip is correct
    // provided the parsed types are faithful.
    for ex in &event.exdate {
        let (p, v) = format_date_or_datetime(ex);
        out.push_str(&fold_line(&format!("EXDATE{p}:{v}")));
    }

    if let Some(rid) = &event.recurrence_id {
        let (p, v) = format_date_or_datetime(rid);
        out.push_str(&fold_line(&format!("RECURRENCE-ID{p}:{v}")));
    }

    if event.sequence > 0 {
        out.push_str(&fold_line(&format!("SEQUENCE:{}", event.sequence)));
    }

    if let Some(st) = &event.status {
        let s = match st {
            EventStatus::Confirmed => "CONFIRMED",
            EventStatus::Tentative => "TENTATIVE",
            EventStatus::Cancelled => "CANCELLED",
        };
        out.push_str(&fold_line(&format!("STATUS:{s}")));
    }

    if let Some(lm) = event.last_modified {
        out.push_str(&fold_line(&format!(
            "LAST-MODIFIED:{}Z",
            lm.format("%Y%m%dT%H%M%S")
        )));
    }

    for xp in &event.x_props {
        let params_str: String = xp
            .params
            .iter()
            .map(|(k, v)| format!(";{k}={v}"))
            .collect();
        out.push_str(&fold_line(&format!("{}{params_str}:{}", xp.name, xp.value)));
    }

    out.push_str("END:VEVENT\r\n");
    out
}

pub fn serialize_icalendar(cal: &VCalendar) -> String {
    let mut out = String::new();

    out.push_str("BEGIN:VCALENDAR\r\n");
    out.push_str(&fold_line(&format!("VERSION:{}", cal.version)));
    out.push_str(&fold_line(&format!("PRODID:{}", cal.prodid)));

    for tz in &cal.timezones {
        out.push_str("BEGIN:VTIMEZONE\r\n");
        out.push_str(&fold_line(&format!("TZID:{}", tz.tzid)));
        for line in &tz.raw_lines {
            out.push_str(&fold_line(line));
        }
        out.push_str("END:VTIMEZONE\r\n");
    }

    for event in &cal.events {
        out.push_str(&serialize_vevent(event));
    }

    out.push_str("END:VCALENDAR\r\n");
    out
}
