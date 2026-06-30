use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

use crate::{StorageError, model::*};

fn row_to_calendar(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<Calendar, StorageError>> {
    let id_str: String = row.get(0)?;
    let group_id_str: Option<String> = row.get(1)?;
    let source_str: String = row.get(3)?;
    let enc_key_str: Option<String> = row.get(5)?;
    let created_at_ms: i64 = row.get(6)?;
    Ok(Ok(Calendar {
        id: id_str
            .parse()
            .map_err(|_| StorageError::Corruption(format!("bad calendar id: {id_str}")))
            .unwrap_or_else(|_| Uuid::nil()),
        group_id: group_id_str
            .map(|s| {
                s.parse()
                    .map_err(|_| StorageError::Corruption(format!("bad group id: {s}")))
                    .unwrap_or_else(|_| Uuid::nil())
            }),
        display_name: row.get(2)?,
        source: CalendarSource::from_str(&source_str)
            .unwrap_or(CalendarSource::Local),
        source_config: row.get(4)?,
        encryption_key_id: enc_key_str.map(|s| {
            s.parse()
                .map_err(|_| StorageError::Corruption(format!("bad key id: {s}")))
                .unwrap_or_else(|_| Uuid::nil())
        }),
        created_at: chrono::DateTime::from_timestamp_millis(created_at_ms).unwrap_or_default(),
    }))
}

pub struct Repo<'a> {
    pub(crate) conn: &'a Connection,
}

impl<'a> Repo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn insert_calendar(
        &self,
        id: Uuid,
        group_id: Option<Uuid>,
        display_name: &str,
        source: &CalendarSource,
        source_config: &str,
        encryption_key_id: Option<Uuid>,
        created_at_ms: i64,
    ) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO calendars (id, group_id, display_name, source_type, source_config, encryption_key_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id.to_string(),
                group_id.map(|g| g.to_string()),
                display_name,
                source.as_str(),
                source_config,
                encryption_key_id.map(|k| k.to_string()),
                created_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn get_calendar(&self, id: Uuid) -> Result<Option<Calendar>, StorageError> {
        self.conn
            .query_row(
                "SELECT id, group_id, display_name, source_type, source_config, encryption_key_id, created_at
                 FROM calendars WHERE id = ?1",
                params![id.to_string()],
                row_to_calendar,
            )
            .optional()?
            .map(|r| r)
            .transpose()
    }

    pub fn insert_event(&self, ev: &NewEvent) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO events
             (id, calendar_id, summary, description, location,
              dtstart, dtstart_is_date_only, dtend, dtend_is_date_only, timezone,
              rrule, rdate, exdate, recurrence_id, sequence, status, last_modified, etag)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
            params![
                ev.id,
                ev.calendar_id.to_string(),
                ev.summary,
                ev.description,
                ev.location,
                ev.dtstart_ms,
                ev.dtstart_is_date_only as i32,
                ev.dtend_ms,
                ev.dtend_is_date_only as i32,
                ev.timezone,
                ev.rrule,
                ev.rdate,
                ev.exdate,
                ev.recurrence_id_ms,
                ev.sequence,
                ev.status.as_str(),
                ev.last_modified_ms,
                ev.etag,
            ],
        )?;
        Ok(())
    }

    pub fn get_event(&self, id: &str) -> Result<Option<Event>, StorageError> {
        self.conn
            .query_row(
                "SELECT id, calendar_id, summary, description, location,
                        dtstart, dtstart_is_date_only, dtend, dtend_is_date_only, timezone,
                        rrule, rdate, exdate, recurrence_id, sequence, status, last_modified, etag, deleted_at
                 FROM events WHERE id = ?1",
                params![id],
                row_to_event,
            )
            .optional()
            .map_err(StorageError::from)?
            .map(|r| r.map_err(StorageError::from))
            .transpose()
    }

    pub fn soft_delete_event(&self, id: &str, deleted_at_ms: i64) -> Result<bool, StorageError> {
        let n = self.conn.execute(
            "UPDATE events SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![deleted_at_ms, id],
        )?;
        Ok(n > 0)
    }

    pub fn events_in_range(
        &self,
        calendar_id: Uuid,
        start_ms: i64,
        end_ms: i64,
    ) -> Result<Vec<Event>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, calendar_id, summary, description, location,
                    dtstart, dtstart_is_date_only, dtend, dtend_is_date_only, timezone,
                    rrule, rdate, exdate, recurrence_id, sequence, status, last_modified, etag, deleted_at
             FROM events
             WHERE calendar_id = ?1
               AND dtstart < ?3
               AND dtend >= ?2
               AND deleted_at IS NULL
             ORDER BY dtstart",
        )?;
        let rows = stmt.query_map(params![calendar_id.to_string(), start_ms, end_ms], row_to_event)?;
        rows.map(|r| r.map_err(StorageError::from).and_then(|inner| inner.map_err(StorageError::from)))
            .collect()
    }

    pub fn upsert_sync_cursor(&self, cursor: &SyncCursor) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO sync_cursors (calendar_id, transport, cursor_token, last_synced_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(calendar_id, transport) DO UPDATE SET
               cursor_token = excluded.cursor_token,
               last_synced_at = excluded.last_synced_at",
            params![
                cursor.calendar_id.to_string(),
                cursor.transport.as_str(),
                cursor.cursor_token,
                cursor.last_synced_at_ms,
            ],
        )?;
        Ok(())
    }

    pub fn get_sync_cursor(
        &self,
        calendar_id: Uuid,
        transport: &SyncTransport,
    ) -> Result<Option<SyncCursor>, StorageError> {
        self.conn
            .query_row(
                "SELECT calendar_id, transport, cursor_token, last_synced_at
                 FROM sync_cursors WHERE calendar_id = ?1 AND transport = ?2",
                params![calendar_id.to_string(), transport.as_str()],
                |row| {
                    let cal_id_str: String = row.get(0)?;
                    let transport_str: String = row.get(1)?;
                    Ok((cal_id_str, transport_str, row.get::<_, Option<String>>(2)?, row.get::<_, Option<i64>>(3)?))
                },
            )
            .optional()?
            .map(|(cal_id_str, transport_str, cursor_token, last_synced_at_ms)| {
                Ok(SyncCursor {
                    calendar_id: cal_id_str.parse().map_err(|_| {
                        StorageError::Corruption(format!("bad calendar_id in sync_cursors: {cal_id_str}"))
                    })?,
                    transport: match transport_str.as_str() {
                        "caldav" => SyncTransport::CalDav,
                        _ => SyncTransport::NativeSync,
                    },
                    cursor_token,
                    last_synced_at_ms,
                })
            })
            .transpose()
    }

    pub fn list_calendars(&self) -> Result<Vec<Calendar>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, group_id, display_name, source_type, source_config, encryption_key_id, created_at
             FROM calendars ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], row_to_calendar)?;
        rows.map(|r| r.map_err(StorageError::from).and_then(|inner| inner))
            .collect()
    }

    pub fn update_event(&self, ev: &UpdateEvent) -> Result<bool, StorageError> {
        let n = self.conn.execute(
            "UPDATE events SET
               summary = ?2, description = ?3, location = ?4,
               dtstart = ?5, dtstart_is_date_only = ?6, dtend = ?7, dtend_is_date_only = ?8,
               timezone = ?9, rrule = ?10, rdate = ?11, exdate = ?12,
               sequence = ?13, status = ?14, last_modified = ?15, etag = ?16
             WHERE id = ?1 AND deleted_at IS NULL",
            params![
                ev.id,
                ev.summary,
                ev.description,
                ev.location,
                ev.dtstart_ms,
                ev.dtstart_is_date_only as i32,
                ev.dtend_ms,
                ev.dtend_is_date_only as i32,
                ev.timezone,
                ev.rrule,
                ev.rdate,
                ev.exdate,
                ev.sequence,
                ev.status.as_str(),
                ev.last_modified_ms,
                ev.etag,
            ],
        )?;
        Ok(n > 0)
    }

    /// INSERT OR REPLACE — used by CalDAV sync to upsert incoming events.
    pub fn upsert_event(&self, ev: &NewEvent) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO events
             (id, calendar_id, summary, description, location,
              dtstart, dtstart_is_date_only, dtend, dtend_is_date_only, timezone,
              rrule, rdate, exdate, recurrence_id, sequence, status, last_modified, etag)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)
             ON CONFLICT(id) DO UPDATE SET
               summary = excluded.summary,
               description = excluded.description,
               location = excluded.location,
               dtstart = excluded.dtstart,
               dtstart_is_date_only = excluded.dtstart_is_date_only,
               dtend = excluded.dtend,
               dtend_is_date_only = excluded.dtend_is_date_only,
               timezone = excluded.timezone,
               rrule = excluded.rrule,
               rdate = excluded.rdate,
               exdate = excluded.exdate,
               recurrence_id = excluded.recurrence_id,
               sequence = excluded.sequence,
               status = excluded.status,
               last_modified = excluded.last_modified,
               etag = excluded.etag,
               deleted_at = NULL",
            params![
                ev.id,
                ev.calendar_id.to_string(),
                ev.summary,
                ev.description,
                ev.location,
                ev.dtstart_ms,
                ev.dtstart_is_date_only as i32,
                ev.dtend_ms,
                ev.dtend_is_date_only as i32,
                ev.timezone,
                ev.rrule,
                ev.rdate,
                ev.exdate,
                ev.recurrence_id_ms,
                ev.sequence,
                ev.status.as_str(),
                ev.last_modified_ms,
                ev.etag,
            ],
        )?;
        Ok(())
    }

    /// All non-deleted events for a calendar, used during full CalDAV syncs to detect deletions.
    pub fn all_active_event_ids_for_calendar(&self, calendar_id: Uuid) -> Result<Vec<String>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM events WHERE calendar_id = ?1 AND deleted_at IS NULL",
        )?;
        let rows = stmt.query_map(params![calendar_id.to_string()], |row| row.get(0))?;
        rows.map(|r| r.map_err(StorageError::from)).collect()
    }
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<Event, StorageError>> {
    let calendar_id_str: String = row.get(1)?;
    let status_str: String = row.get(15)?;
    Ok(Ok(Event {
        id: row.get(0)?,
        calendar_id: calendar_id_str
            .parse()
            .map_err(|_| StorageError::Corruption(format!("bad calendar_id: {calendar_id_str}")))
            .unwrap_or_else(|_| Uuid::nil()),
        summary: row.get(2)?,
        description: row.get(3)?,
        location: row.get(4)?,
        dtstart_ms: row.get(5)?,
        dtstart_is_date_only: row.get::<_, i32>(6)? != 0,
        dtend_ms: row.get(7)?,
        dtend_is_date_only: row.get::<_, i32>(8)? != 0,
        timezone: row.get(9)?,
        rrule: row.get(10)?,
        rdate: row.get(11)?,
        exdate: row.get(12)?,
        recurrence_id_ms: row.get(13)?,
        sequence: row.get::<_, u32>(14)?,
        status: EventStatus::from_str(&status_str),
        last_modified_ms: row.get(16)?,
        etag: row.get(17)?,
        deleted_at_ms: row.get(18)?,
    }))
}
