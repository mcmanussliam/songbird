use chrono::{NaiveDate, NaiveDateTime, Weekday};

/// Mirror of songbird-ical's DateOrDateTime, defined independently so this crate
/// has no internal workspace dependencies.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DateOrDateTime {
    Date(NaiveDate),
    DateTime {
        local: NaiveDateTime,
        /// IANA timezone string (e.g. "America/New_York"). None means floating/UTC.
        tzid: Option<String>,
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

    pub fn naive_datetime(&self) -> Option<NaiveDateTime> {
        match self {
            DateOrDateTime::Date(_) => None,
            DateOrDateTime::DateTime { local, .. } => Some(*local),
        }
    }

    pub fn tzid(&self) -> Option<&str> {
        match self {
            DateOrDateTime::Date(_) => None,
            DateOrDateTime::DateTime { tzid, .. } => tzid.as_deref(),
        }
    }

    /// True when both values represent the same calendar date (for EXDATE / RECURRENCE-ID matching).
    pub fn same_date(&self, other: &DateOrDateTime) -> bool {
        self.naive_date() == other.naive_date()
    }
}

/// A single occurrence returned from expand_occurrences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    /// Effective start of this occurrence (may differ from the base DTSTART when overridden).
    pub start: DateOrDateTime,
    /// True when this occurrence was replaced by an override VEVENT.
    pub is_override: bool,
}

/// Describes a RECURRENCE-ID override VEVENT: the original occurrence date and the override's start.
#[derive(Debug, Clone)]
pub struct EventOverride {
    /// Matches the RECURRENCE-ID value — identifies which base occurrence this replaces.
    pub recurrence_id: DateOrDateTime,
    /// The override VEVENT's DTSTART — the new start for this occurrence.
    pub dtstart: DateOrDateTime,
}

/// Half-open date/time range [start, end).
#[derive(Debug, Clone)]
pub struct DateRange {
    pub start: DateOrDateTime,
    pub end: DateOrDateTime,
}

impl DateRange {
    pub fn contains(&self, dt: &DateOrDateTime) -> bool {
        let d = dt.naive_date();
        d >= self.start.naive_date() && d < self.end.naive_date()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frequency {
    Secondly,
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// A BYDAY entry like `-1SA` or `MO`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekdayOrdinal {
    /// None = every occurrence of this weekday in the period.
    /// Some(n) = the n-th (positive) or n-th from end (negative).
    pub ordinal: Option<i32>,
    pub weekday: Weekday,
}

/// Fully parsed RRULE per RFC 5545 §3.3.10.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceRule {
    pub freq: Frequency,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<DateOrDateTime>,
    pub bysecond: Vec<i32>,
    pub byminute: Vec<i32>,
    pub byhour: Vec<i32>,
    pub byday: Vec<WeekdayOrdinal>,
    pub bymonthday: Vec<i32>,
    pub byyearday: Vec<i32>,
    pub byweekno: Vec<i32>,
    pub bymonth: Vec<u8>,
    pub bysetpos: Vec<i32>,
    pub wkst: Weekday,
    /// Original raw RRULE string, preserved for serialization.
    pub raw: String,
}
