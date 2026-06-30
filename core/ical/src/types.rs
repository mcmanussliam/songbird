use chrono::{NaiveDate, NaiveDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateOrDateTime {
    Date(NaiveDate),
    DateTime {
        local: NaiveDateTime,
        /// IANA timezone name (e.g. "Europe/Berlin"). `None` with `is_utc = false` = floating time.
        tzid: Option<String>,
        /// `true` when the value had a trailing `Z` (UTC); `false` + no tzid = floating.
        is_utc: bool,
    },
}

impl DateOrDateTime {
    pub fn naive_date(&self) -> NaiveDate {
        match self {
            DateOrDateTime::Date(d) => *d,
            DateOrDateTime::DateTime { local, .. } => local.date(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EventStatus {
    #[default]
    Confirmed,
    Tentative,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XProp {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VEvent {
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub dtstart: DateOrDateTime,
    pub dtend: Option<DateOrDateTime>,
    pub rrule: Option<String>,
    pub rdate: Vec<DateOrDateTime>,
    pub exdate: Vec<DateOrDateTime>,
    pub recurrence_id: Option<DateOrDateTime>,
    pub sequence: u32,
    pub status: Option<EventStatus>,
    pub last_modified: Option<NaiveDateTime>,
    pub x_props: Vec<XProp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VTimezone {
    pub tzid: String,
    /// Raw content lines preserved verbatim for round-trip.
    pub raw_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VCalendar {
    pub prodid: String,
    pub version: String,
    pub timezones: Vec<VTimezone>,
    pub events: Vec<VEvent>,
}
