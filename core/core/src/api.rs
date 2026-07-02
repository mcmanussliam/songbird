//! Dart-facing API surface. All public types and functions here are what
//! flutter_rust_bridge generates Dart bindings from.
//!
//! Run `flutter_rust_bridge_codegen generate` in `app/` to regenerate
//! `app/lib/src/rust/frb_generated.dart` after changing signatures here.

use std::sync::{Arc, OnceLock};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;
use uuid::Uuid;

use songbird_caldav_client::{Auth, CalDavClient, CalDavConfig};
use songbird_ical::{parse_icalendar, IcalError};
use songbird_recurrence::{expand_occurrences, parse_rrule, DateOrDateTime, DateRange};
use songbird_storage::{
    model::{CalendarSource, EventStatus, NewEvent, SyncCursor, SyncTransport, UpdateEvent},
    Repo, Store, StorageError,
};

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("call init() before any other API function")]
    NotInitialized,
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("calendar not found: {id}")]
    CalendarNotFound { id: String },
    #[error("event not found: {id}")]
    EventNotFound { id: String },
    #[error("CalDAV error: {0}")]
    CalDav(String),
    #[error("iCalendar parse error: {0}")]
    Ical(String),
    #[error("config error: {0}")]
    Config(String),
}

impl From<IcalError> for CoreError {
    fn from(e: IcalError) -> Self {
        CoreError::Ical(e.to_string())
    }
}

impl From<songbird_caldav_client::CalDavError> for CoreError {
    fn from(e: songbird_caldav_client::CalDavError) -> Self {
        CoreError::CalDav(e.to_string())
    }
}

struct AppState {
    store: Store,
}

static APP: OnceLock<Arc<Mutex<AppState>>> = OnceLock::new();

fn app() -> Result<Arc<Mutex<AppState>>, CoreError> {
    APP.get().cloned().ok_or(CoreError::NotInitialized)
}

#[derive(Debug, Clone)]
pub struct CalendarView {
    pub id: String,
    pub display_name: String,
    /// "local", "caldav", "subscription"
    pub source: String,
    pub last_synced_ms: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct OccurrenceView {
    pub event_id: String,
    pub calendar_id: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub dtstart_ms: i64,
    pub dtend_ms: i64,
    pub is_all_day: bool,
    pub timezone: Option<String>,
    pub has_recurrence: bool,
    /// Non-null for detached override instances (have a RECURRENCE-ID).
    pub recurrence_id_ms: Option<i64>,
    pub status: String,
}

/// Fields for creating a new event. Dates are UTC epoch milliseconds.
#[derive(Debug, Clone)]
pub struct EventDraft {
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub dtstart_ms: i64,
    pub dtend_ms: i64,
    pub is_all_day: bool,
    pub timezone: Option<String>,
    pub rrule: Option<String>,
}

/// Wraps a nullable field update so `Option<NullableStringUpdate>` can distinguish
/// "leave unchanged" (`None`) from "set" (`Some`, whose inner `value` may itself be `None`
/// to clear the field)
///
/// `flutter_rust_bridge` cannot generate bindings for `Option<Option<T>>`.
#[derive(Debug, Clone)]
pub struct NullableStringUpdate {
    pub value: Option<String>,
}

/// Partial update, only `Some` fields are written; `None` leaves the existing value unchanged.
/// For nullable fields, a present update whose
/// `value` is `None` means "clear it."
#[derive(Debug, Clone)]
pub struct EventPatch {
    pub summary: Option<String>,
    pub description: Option<NullableStringUpdate>,
    pub location: Option<NullableStringUpdate>,
    pub dtstart_ms: Option<i64>,
    pub dtend_ms: Option<i64>,
    pub is_all_day: Option<bool>,
    pub timezone: Option<NullableStringUpdate>,
    pub rrule: Option<NullableStringUpdate>,
}

#[derive(Debug, Clone)]
pub enum DeleteScope {
    /// Delete only this occurrence (or the whole event if not recurring).
    ThisOnly,
    /// Delete this occurrence and all future ones by truncating the RRULE UNTIL.
    ThisAndFuture,
    /// Delete the entire event series.
    All,
}

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub fetched: u32,
    pub deleted: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DateRangeMs {
    pub start_ms: i64,
    pub end_ms: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CalDavSourceConfig {
    base_url: String,
    username: String,
    password: String,
    calendar_url: String,
}

/// Open (or create) the local SQLite database. Safe to call multiple times — only
/// the first call opens the database; subsequent calls are no-ops.
pub async fn init(db_path: String) -> Result<(), CoreError> {
    if APP.get().is_some() {
        return Ok(());
    }
    let store = Store::open(&db_path)?;
    let state = Arc::new(Mutex::new(AppState { store }));
    // Ignore error if another thread raced us to set it.
    let _ = APP.set(state);
    Ok(())
}

pub async fn list_calendars() -> Result<Vec<CalendarView>, CoreError> {
    let arc = app()?;
    let guard = arc.lock().await;
    let calendars = guard.store.repo().list_calendars()?;
    Ok(calendars
        .into_iter()
        .map(|c| CalendarView {
            id: c.id.to_string(),
            display_name: c.display_name,
            source: c.source.as_str().to_string(),
            last_synced_ms: None,
        })
        .collect())
}

pub async fn create_local_calendar(display_name: String) -> Result<String, CoreError> {
    let arc = app()?;
    let guard = arc.lock().await;
    let id = Uuid::new_v4();
    let now_ms = Utc::now().timestamp_millis();
    guard.store.repo().insert_calendar(
        id,
        None,
        &display_name,
        &CalendarSource::Local,
        "{}",
        None,
        now_ms,
    )?;
    Ok(id.to_string())
}

/// Discover a CalDAV server, create one local calendar record per remote calendar found.
/// Returns the local IDs of created calendar records.
pub async fn add_caldav_account(
    base_url: String,
    username: String,
    password: String,
) -> Result<Vec<String>, CoreError> {
    let config = CalDavConfig {
        base_url: base_url.clone(),
        auth: Auth::Basic {
            username: username.clone(),
            password: password.clone(),
        },
    };
    let client = CalDavClient::new(config)?;
    let (_principal, home_set) = client.discover().await?;
    let remote_calendars = client.list_calendars(&home_set).await?;

    let arc = app()?;
    let guard = arc.lock().await;
    let now_ms = Utc::now().timestamp_millis();
    let mut ids = Vec::new();

    for remote_cal in remote_calendars {
        let local_id = Uuid::new_v4();
        let src_config = serde_json::to_string(&CalDavSourceConfig {
            base_url: base_url.clone(),
            username: username.clone(),
            password: password.clone(),
            calendar_url: remote_cal.href.clone(),
        })
        .map_err(|e| CoreError::Config(e.to_string()))?;

        guard.store.repo().insert_calendar(
            local_id,
            None,
            remote_cal.display_name.as_deref().unwrap_or("Unnamed Calendar"),
            &CalendarSource::CalDav,
            &src_config,
            None,
            now_ms,
        )?;
        ids.push(local_id.to_string());
    }

    Ok(ids)
}

pub async fn create_event(calendar_id: String, draft: EventDraft) -> Result<String, CoreError> {
    let cal_uuid: Uuid = calendar_id
        .parse()
        .map_err(|_| CoreError::CalendarNotFound { id: calendar_id.clone() })?;

    let arc = app()?;
    let guard = arc.lock().await;
    let now_ms = Utc::now().timestamp_millis();

    guard
        .store
        .repo()
        .get_calendar(cal_uuid)?
        .ok_or_else(|| CoreError::CalendarNotFound { id: calendar_id.clone() })?;

    let uid = format!("{}-songbird", Uuid::new_v4());
    guard.store.repo().insert_event(&NewEvent {
        id: uid.clone(),
        calendar_id: cal_uuid,
        summary: draft.summary,
        description: draft.description,
        location: draft.location,
        dtstart_ms: draft.dtstart_ms,
        dtstart_is_date_only: draft.is_all_day,
        dtend_ms: draft.dtend_ms,
        dtend_is_date_only: draft.is_all_day,
        timezone: draft.timezone,
        rrule: draft.rrule,
        rdate: None,
        exdate: None,
        recurrence_id_ms: None,
        sequence: 0,
        status: EventStatus::Confirmed,
        last_modified_ms: now_ms,
        etag: None,
    })?;

    Ok(uid)
}

pub async fn update_event(event_id: String, patch: EventPatch) -> Result<(), CoreError> {
    let arc = app()?;
    let guard = arc.lock().await;
    let repo = guard.store.repo();

    let existing = repo
        .get_event(&event_id)?
        .ok_or_else(|| CoreError::EventNotFound { id: event_id.clone() })?;

    let now_ms = Utc::now().timestamp_millis();
    let is_all_day = patch.is_all_day.unwrap_or(existing.dtstart_is_date_only);

    repo.update_event(&UpdateEvent {
        id: event_id,
        summary: patch.summary.unwrap_or(existing.summary),
        description: patch.description.map(|u| u.value).unwrap_or(existing.description),
        location: patch.location.map(|u| u.value).unwrap_or(existing.location),
        dtstart_ms: patch.dtstart_ms.unwrap_or(existing.dtstart_ms),
        dtstart_is_date_only: is_all_day,
        dtend_ms: patch.dtend_ms.unwrap_or(existing.dtend_ms),
        dtend_is_date_only: is_all_day,
        timezone: patch.timezone.map(|u| u.value).unwrap_or(existing.timezone),
        rrule: patch.rrule.map(|u| u.value).unwrap_or(existing.rrule),
        rdate: None,
        exdate: None,
        sequence: existing.sequence + 1,
        status: existing.status,
        last_modified_ms: now_ms,
        etag: None,
    })?;

    Ok(())
}

pub async fn delete_event(
    event_id: String,
    scope: DeleteScope,
    _recurrence_id_ms: Option<i64>,
) -> Result<(), CoreError> {
    let arc = app()?;
    let guard = arc.lock().await;
    let now_ms = Utc::now().timestamp_millis();

    match scope {
        DeleteScope::All | DeleteScope::ThisOnly => {
            guard.store.repo().soft_delete_event(&event_id, now_ms)?;
        }
        DeleteScope::ThisAndFuture => {
            // TODO(M4): truncate RRULE with UNTIL instead of deleting the whole series.
            guard.store.repo().soft_delete_event(&event_id, now_ms)?;
        }
    }

    Ok(())
}

/// Return all occurrences (with recurrences expanded) across the given calendars
/// that overlap [range.start_ms, range.end_ms).
pub async fn occurrences_in_range(
    calendar_ids: Vec<String>,
    range: DateRangeMs,
) -> Result<Vec<OccurrenceView>, CoreError> {
    let arc = app()?;
    let guard = arc.lock().await;
    let repo = guard.store.repo();

    let mut results: Vec<OccurrenceView> = Vec::new();
    let expand_range = DateRange {
        start: ms_to_date_or_datetime(range.start_ms, false),
        end: ms_to_date_or_datetime(range.end_ms, false),
    };

    for cal_id_str in &calendar_ids {
        let cal_uuid: Uuid = match cal_id_str.parse() {
            Ok(id) => id,
            Err(_) => continue,
        };

        let events = repo.events_in_range(cal_uuid, range.start_ms, range.end_ms)?;

        for ev in events {
            let has_recurrence = ev.rrule.is_some() || ev.rdate.is_some();

            if !has_recurrence {
                results.push(OccurrenceView {
                    event_id: ev.id,
                    calendar_id: cal_id_str.clone(),
                    summary: ev.summary,
                    description: ev.description,
                    location: ev.location,
                    dtstart_ms: ev.dtstart_ms,
                    dtend_ms: ev.dtend_ms,
                    is_all_day: ev.dtstart_is_date_only,
                    timezone: ev.timezone,
                    has_recurrence: false,
                    recurrence_id_ms: ev.recurrence_id_ms,
                    status: ev.status.as_str().to_string(),
                });
                continue;
            }

            let Some(rrule_str) = &ev.rrule else { continue };
            let rule = match parse_rrule(rrule_str) {
                Ok(r) => r,
                Err(_) => continue,
            };

            let dtstart = ms_to_date_or_datetime(ev.dtstart_ms, ev.dtstart_is_date_only);
            let duration_ms = ev.dtend_ms - ev.dtstart_ms;

            for occ in expand_occurrences(&rule, &dtstart, &[], &[], &[], &expand_range) {
                let occ_start_ms = date_or_datetime_to_ms(&occ.start);
                results.push(OccurrenceView {
                    event_id: ev.id.clone(),
                    calendar_id: cal_id_str.clone(),
                    summary: ev.summary.clone(),
                    description: ev.description.clone(),
                    location: ev.location.clone(),
                    dtstart_ms: occ_start_ms,
                    dtend_ms: occ_start_ms + duration_ms,
                    is_all_day: ev.dtstart_is_date_only,
                    timezone: ev.timezone.clone(),
                    has_recurrence: true,
                    recurrence_id_ms: None,
                    status: ev.status.as_str().to_string(),
                });
            }
        }
    }

    Ok(results)
}

pub async fn sync_now(calendar_id: String) -> Result<SyncResult, CoreError> {
    let cal_uuid: Uuid = calendar_id
        .parse()
        .map_err(|_| CoreError::CalendarNotFound { id: calendar_id.clone() })?;

    // Fetch config and cursor, then drop the lock before network I/O.
    let (src_config, cursor_token) = {
        let arc = app()?;
        let guard = arc.lock().await;
        let repo = guard.store.repo();

        let cal = repo
            .get_calendar(cal_uuid)?
            .ok_or_else(|| CoreError::CalendarNotFound { id: calendar_id.clone() })?;

        if cal.source != CalendarSource::CalDav {
            return Ok(SyncResult { fetched: 0, deleted: 0, errors: vec![] });
        }

        let cfg: CalDavSourceConfig = serde_json::from_str(&cal.source_config)
            .map_err(|e| CoreError::Config(e.to_string()))?;
        let cursor = repo
            .get_sync_cursor(cal_uuid, &SyncTransport::CalDav)?
            .and_then(|c| c.cursor_token);

        (cfg, cursor)
    };

    let client = CalDavClient::new(CalDavConfig {
        base_url: src_config.base_url,
        auth: Auth::Basic {
            username: src_config.username,
            password: src_config.password,
        },
    })?;

    let outcome = client.sync_calendar(&src_config.calendar_url, cursor_token.as_deref()).await?;
    let now_ms = Utc::now().timestamp_millis();

    let arc = app()?;
    let guard = arc.lock().await;
    let repo = guard.store.repo();

    let mut fetched = 0u32;
    let mut deleted = 0u32;
    let mut errors: Vec<String> = Vec::new();
    let mut seen_uids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for resource in &outcome.fetched {
        match apply_fetched_resource(&repo, cal_uuid, resource) {
            Ok(uid) => {
                seen_uids.insert(uid);
                fetched += 1;
            }
            Err(e) => errors.push(e.to_string()),
        }
    }

    for deleted_res in &outcome.deleted {
        // ResourceInfo.href is the CalDAV resource path, which equals the event UID for most servers.
        // For servers where href != uid, we'd need to look up by href — acceptable at M3.
        let href_uid = deleted_res.href.trim_end_matches(".ics").rsplit('/').next().unwrap_or(&deleted_res.href);
        match repo.soft_delete_event(href_uid, now_ms) {
            Ok(_) => deleted += 1,
            Err(e) => errors.push(e.to_string()),
        }
    }

    if outcome.is_full_sync {
        for uid in repo.all_active_event_ids_for_calendar(cal_uuid)? {
            if !seen_uids.contains(&uid) && repo.soft_delete_event(&uid, now_ms)? {
                deleted += 1;
            }
        }
    }

    repo.upsert_sync_cursor(&SyncCursor {
        calendar_id: cal_uuid,
        transport: SyncTransport::CalDav,
        cursor_token: outcome.new_cursor,
        last_synced_at_ms: Some(now_ms),
    })?;

    Ok(SyncResult { fetched, deleted, errors })
}

fn apply_fetched_resource(
    repo: &Repo<'_>,
    calendar_id: Uuid,
    resource: &songbird_caldav_client::FetchedResource,
) -> Result<String, CoreError> {
    let cal = parse_icalendar(&resource.ical_data)?;
    let event = cal
        .events
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::Ical("no VEVENT in fetched resource".into()))?;

    let uid = event.uid.clone();
    let dtstart_ms = ical_date_to_ms(&event.dtstart);
    let dtend_ms = event.dtend.as_ref().map(ical_date_to_ms).unwrap_or(dtstart_ms);
    let now_ms = Utc::now().timestamp_millis();

    let tz = match &event.dtstart {
        songbird_ical::DateOrDateTime::DateTime { tzid, .. } => tzid.clone(),
        _ => None,
    };
    let is_date_start = matches!(event.dtstart, songbird_ical::DateOrDateTime::Date(_));
    let is_date_end = event
        .dtend
        .as_ref()
        .is_some_and(|d| matches!(d, songbird_ical::DateOrDateTime::Date(_)));
    let status = match event.status {
        Some(songbird_ical::EventStatus::Tentative) => EventStatus::Tentative,
        Some(songbird_ical::EventStatus::Cancelled) => EventStatus::Cancelled,
        _ => EventStatus::Confirmed,
    };

    repo.upsert_event(&NewEvent {
        id: event.uid,
        calendar_id,
        summary: event.summary,
        description: event.description,
        location: event.location,
        dtstart_ms,
        dtstart_is_date_only: is_date_start,
        dtend_ms,
        dtend_is_date_only: is_date_end,
        timezone: tz,
        rrule: event.rrule,
        rdate: None,
        exdate: None,
        recurrence_id_ms: event.recurrence_id.as_ref().map(ical_date_to_ms),
        sequence: event.sequence,
        status,
        last_modified_ms: event
            .last_modified
            .map(|dt| dt.and_utc().timestamp_millis())
            .unwrap_or(now_ms),
        etag: resource.info.etag.clone(),
    })?;

    Ok(uid)
}

fn ical_date_to_ms(dt: &songbird_ical::DateOrDateTime) -> i64 {
    match dt {
        songbird_ical::DateOrDateTime::Date(d) => d
            .and_hms_opt(0, 0, 0)
            .map(|ndt| ndt.and_utc().timestamp_millis())
            .unwrap_or(0),
        songbird_ical::DateOrDateTime::DateTime { local, .. } => local.and_utc().timestamp_millis(),
    }
}

fn ms_to_date_or_datetime(ms: i64, is_date_only: bool) -> DateOrDateTime {
    let ndt = chrono::DateTime::from_timestamp_millis(ms)
        .unwrap_or_default()
        .naive_utc();
    if is_date_only {
        DateOrDateTime::Date(ndt.date())
    } else {
        DateOrDateTime::DateTime { local: ndt, tzid: None, is_utc: true }
    }
}

fn date_or_datetime_to_ms(dt: &DateOrDateTime) -> i64 {
    match dt {
        DateOrDateTime::Date(d) => d
            .and_hms_opt(0, 0, 0)
            .map(|ndt| ndt.and_utc().timestamp_millis())
            .unwrap_or(0),
        DateOrDateTime::DateTime { local, .. } => local.and_utc().timestamp_millis(),
    }
}
