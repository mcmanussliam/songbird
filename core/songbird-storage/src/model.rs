use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarSource {
    Local,
    CalDav,
    NativeSync,
    Subscription,
}

impl CalendarSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            CalendarSource::Local => "local",
            CalendarSource::CalDav => "caldav",
            CalendarSource::NativeSync => "native_sync",
            CalendarSource::Subscription => "subscription",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "local" => Some(CalendarSource::Local),
            "caldav" => Some(CalendarSource::CalDav),
            "native_sync" => Some(CalendarSource::NativeSync),
            "subscription" => Some(CalendarSource::Subscription),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Calendar {
    pub id: Uuid,
    pub group_id: Option<Uuid>,
    pub display_name: String,
    pub source: CalendarSource,
    pub source_config: String,
    pub encryption_key_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

impl EventStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStatus::Confirmed => "confirmed",
            EventStatus::Tentative => "tentative",
            EventStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "tentative" => EventStatus::Tentative,
            "cancelled" => EventStatus::Cancelled,
            _ => EventStatus::Confirmed,
        }
    }
}

/// A stored event row. dtstart/dtend are UTC epoch milliseconds.
/// The display timezone is reconstructed from `timezone` at read time.
#[derive(Debug, Clone)]
pub struct Event {
    pub id: String,
    pub calendar_id: Uuid,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub dtstart_ms: i64,
    pub dtstart_is_date_only: bool,
    pub dtend_ms: i64,
    pub dtend_is_date_only: bool,
    pub timezone: Option<String>,
    pub rrule: Option<String>,
    pub rdate: Option<String>,
    pub exdate: Option<String>,
    pub recurrence_id_ms: Option<i64>,
    pub sequence: u32,
    pub status: EventStatus,
    pub last_modified_ms: i64,
    pub etag: Option<String>,
    pub deleted_at_ms: Option<i64>,
}

pub struct NewEvent {
    pub id: String,
    pub calendar_id: Uuid,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub dtstart_ms: i64,
    pub dtstart_is_date_only: bool,
    pub dtend_ms: i64,
    pub dtend_is_date_only: bool,
    pub timezone: Option<String>,
    pub rrule: Option<String>,
    pub rdate: Option<String>,
    pub exdate: Option<String>,
    pub recurrence_id_ms: Option<i64>,
    pub sequence: u32,
    pub status: EventStatus,
    pub last_modified_ms: i64,
    pub etag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncTransport {
    CalDav,
    NativeSync,
}

impl SyncTransport {
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncTransport::CalDav => "caldav",
            SyncTransport::NativeSync => "native_sync",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SyncCursor {
    pub calendar_id: Uuid,
    pub transport: SyncTransport,
    pub cursor_token: Option<String>,
    pub last_synced_at_ms: Option<i64>,
}
