use chrono::{NaiveDate, NaiveDateTime};

use crate::{IcalError, types::*};

/// Unfold RFC 5545 §3.1 line folding: CRLF (or bare LF) followed by a
/// space or tab is a continuation — strip the line break and leading whitespace.
pub(crate) fn unfold(ics: &str) -> String {
    let mut out = String::with_capacity(ics.len());
    let mut chars = ics.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
                if matches!(chars.peek(), Some(' ') | Some('\t')) {
                    chars.next();
                } else {
                    out.push('\n');
                }
            } else {
                out.push(ch);
            }
        } else if ch == '\n' {
            if matches!(chars.peek(), Some(' ') | Some('\t')) {
                chars.next();
            } else {
                out.push('\n');
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Parse one unfolded content line into (name, params[(k,v)], value).
/// Returns None for blank lines.
pub(crate) fn parse_content_line(line: &str) -> Option<(&str, Vec<(String, String)>, String)> {
    let line = line.trim_end_matches('\r');
    if line.is_empty() {
        return None;
    }

    let colon = find_colon(line)?;
    let name_params = &line[..colon];
    let value = line[colon + 1..].to_string();

    let mut segments = name_params.splitn(100, ';');
    let name = segments.next()?.trim();

    let mut params = Vec::new();
    for seg in segments {
        if let Some((k, v)) = seg.split_once('=') {
            params.push((
                k.trim().to_uppercase(),
                v.trim().trim_matches('"').to_string(),
            ));
        }
    }

    Some((name, params, value))
}

fn find_colon(s: &str) -> Option<usize> {
    let mut in_quote = false;

    for (i, ch) in s.char_indices() {
        match ch {
            '"' => in_quote = !in_quote,
            ':' if !in_quote => return Some(i),
            _ => {}
        }
    }

    None
}

/// Parse a date/time value string given its property parameters.
pub(crate) fn parse_date_or_datetime(
    value: &str,
    params: &[(String, String)],
) -> Result<DateOrDateTime, IcalError> {
    let value_type = params.iter().find(|(k, _)| k == "VALUE").map(|(_, v)| v.as_str());
    let tzid = params.iter().find(|(k, _)| k == "TZID").map(|(_, v)| v.clone());

    let is_date = value_type == Some("DATE") || (!value.contains('T') && value.len() == 8);

    if is_date {
        let d = NaiveDate::parse_from_str(value, "%Y%m%d").map_err(|e| {
            IcalError::Malformed(format!("bad DATE value '{value}': {e}"))
        })?;
        return Ok(DateOrDateTime::Date(d));
    }

    let is_utc = value.ends_with('Z');
    let dt_str = value.trim_end_matches('Z');
    let local = NaiveDateTime::parse_from_str(dt_str, "%Y%m%dT%H%M%S").map_err(|e| {
        IcalError::Malformed(format!("bad DATE-TIME value '{value}': {e}"))
    })?;

    Ok(DateOrDateTime::DateTime { local, tzid, is_utc })
}

/// RFC 5545 text-property unescape: `\,` `\;` `\n` `\N` `\\` → `,` `;` `\n` `\n` `\`.
fn unescape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(',') => out.push(','),
                Some(';') => out.push(';'),
                Some(other) => { out.push('\\'); out.push(other); }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }

    out
}

pub fn parse_icalendar(ics: &str) -> Result<VCalendar, IcalError> {
    let unfolded = unfold(ics);
    let lines: Vec<&str> = unfolded.lines().collect();

    let mut cal = VCalendar {
        prodid: String::new(),
        version: String::new(),
        timezones: Vec::new(),
        events: Vec::new(),
    };

    let mut i = 0;

    // Skip to BEGIN:VCALENDAR
    while i < lines.len() && lines[i].trim() != "BEGIN:VCALENDAR" {
        i += 1;
    }
    i += 1;

    while i < lines.len() {
        let line = lines[i].trim();

        if line == "END:VCALENDAR" {
            break;
        }

        if line == "BEGIN:VEVENT" {
            let (event, next_i) = parse_vevent_lines(&lines, i + 1)?;
            cal.events.push(event);
            i = next_i;
            continue;
        }

        if line == "BEGIN:VTIMEZONE" {
            let (tz, next_i) = parse_vtimezone_lines(&lines, i + 1);
            cal.timezones.push(tz);
            i = next_i;
            continue;
        }

        if let Some((name, _params, value)) = parse_content_line(line) {
            match name.to_uppercase().as_str() {
                "PRODID" => cal.prodid = value,
                "VERSION" => cal.version = value,
                _ => {}
            }
        }

        i += 1;
    }

    Ok(cal)
}

/// Parse a bare VEVENT block (between BEGIN:VEVENT and END:VEVENT).
pub fn parse_vevent(ics: &str) -> Result<VEvent, IcalError> {
    let unfolded = unfold(ics);
    let lines: Vec<&str> = unfolded.lines().collect();

    let start = lines
        .iter()
        .position(|l| l.trim() == "BEGIN:VEVENT")
        .map(|i| i + 1)
        .unwrap_or(0);

    let (event, _) = parse_vevent_lines(&lines, start)?;
    Ok(event)
}

fn parse_vevent_lines(lines: &[&str], start: usize) -> Result<(VEvent, usize), IcalError> {
    let mut uid = String::new();
    let mut summary = String::new();
    let mut description: Option<String> = None;
    let mut location: Option<String> = None;
    let mut dtstart: Option<DateOrDateTime> = None;
    let mut dtend: Option<DateOrDateTime> = None;
    let mut rrule: Option<String> = None;
    let mut rdate: Vec<DateOrDateTime> = Vec::new();
    let mut exdate: Vec<DateOrDateTime> = Vec::new();
    let mut recurrence_id: Option<DateOrDateTime> = None;
    let mut sequence = 0u32;
    let mut status: Option<EventStatus> = None;
    let mut last_modified: Option<NaiveDateTime> = None;
    let mut x_props: Vec<XProp> = Vec::new();

    let mut i = start;

    while i < lines.len() {
        let line = lines[i].trim();

        if line == "END:VEVENT" {
            i += 1;
            break;
        }

        if let Some((name, params, value)) = parse_content_line(line) {
            match name.to_uppercase().as_str() {
                "UID" => uid = value,
                "SUMMARY" => summary = unescape_text(&value),
                "DESCRIPTION" => description = Some(unescape_text(&value)),
                "LOCATION" => location = Some(unescape_text(&value)),
                "DTSTART" => dtstart = Some(parse_date_or_datetime(&value, &params)?),
                "DTEND" => dtend = Some(parse_date_or_datetime(&value, &params)?),
                "RRULE" => rrule = Some(value),
                "RDATE" => {
                    for v in value.split(',') {
                        rdate.push(parse_date_or_datetime(v.trim(), &params)?);
                    }
                }
                "EXDATE" => {
                    for v in value.split(',') {
                        exdate.push(parse_date_or_datetime(v.trim(), &params)?);
                    }
                }
                "RECURRENCE-ID" => {
                    recurrence_id = Some(parse_date_or_datetime(&value, &params)?)
                }
                "SEQUENCE" => sequence = value.trim().parse().unwrap_or(0),
                "STATUS" => {
                    status = Some(match value.trim() {
                        "TENTATIVE" => EventStatus::Tentative,
                        "CANCELLED" => EventStatus::Cancelled,
                        _ => EventStatus::Confirmed,
                    })
                }
                "LAST-MODIFIED" => {
                    last_modified = NaiveDateTime::parse_from_str(
                        value.trim().trim_end_matches('Z'),
                        "%Y%m%dT%H%M%S",
                    )
                    .ok();
                }
                n if n.starts_with("X-") => {
                    x_props.push(XProp {
                        name: name.to_uppercase(),
                        params,
                        value,
                    });
                }
                _ => {}
            }
        }

        i += 1;
    }

    let dtstart =
        dtstart.ok_or_else(|| IcalError::Malformed("VEVENT missing DTSTART".into()))?;
    if uid.is_empty() {
        return Err(IcalError::Malformed("VEVENT missing UID".into()));
    }

    Ok((
        VEvent {
            uid,
            summary,
            description,
            location,
            dtstart,
            dtend,
            rrule,
            rdate,
            exdate,
            recurrence_id,
            sequence,
            status,
            last_modified,
            x_props,
        },
        i,
    ))
}

fn parse_vtimezone_lines(lines: &[&str], start: usize) -> (VTimezone, usize) {
    let mut tzid = String::new();
    let mut raw_lines: Vec<String> = Vec::new();
    let mut i = start;
    let mut depth = 1usize; // we're inside BEGIN:VTIMEZONE

    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("BEGIN:") {
            depth += 1;
            raw_lines.push(line.to_string());
        } else if line.starts_with("END:") {
            depth -= 1;
            if depth == 0 {
                i += 1;
                break;
            }
            raw_lines.push(line.to_string());
        } else {
            if let Some((name, _params, value)) = parse_content_line(line) {
                if name.eq_ignore_ascii_case("TZID") {
                    tzid = value.clone();
                }
            }
            raw_lines.push(line.to_string());
        }

        i += 1;
    }

    (VTimezone { tzid, raw_lines }, i)
}
