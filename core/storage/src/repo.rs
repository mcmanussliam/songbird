use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

use crate::{StorageError, model::*};

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
                |row| {
                    let id_str: String = row.get(0)?;
                    let group_id_str: Option<String> = row.get(1)?;
                    let source_str: String = row.get(3)?;
                    let enc_key_str: Option<String> = row.get(5)?;
                    let created_at_ms: i64 = row.get(6)?;
                    Ok((id_str, group_id_str, row.get::<_, String>(2)?, source_str, row.get::<_, String>(4)?, enc_key_str, created_at_ms))
                },
            )
            .optional()?
            .map(|(id_str, group_id_str, display_name, source_str, source_config, enc_key_str, created_at_ms)| {
                Ok(Calendar {
                    id: id_str.parse().map_err(|_| StorageError::Corruption(format!("bad calendar id: {id_str}")))?,
                    group_id: group_id_str
                        .map(|s| s.parse().map_err(|_| StorageError::Corruption(format!("bad group id: {s}"))))
                        .transpose()?,
                    display_name,
                    source: CalendarSource::from_str(&source_str)
                        .ok_or_else(|| StorageError::Corruption(format!("unknown source type: {source_str}")))?,
                    source_config,
                    encryption_key_id: enc_key_str
                        .map(|s| s.parse().map_err(|_| StorageError::Corruption(format!("bad key id: {s}"))))
                        .transpose()?,
                    created_at: chrono::DateTime::from_timestamp_millis(created_at_ms)
                        .unwrap_or_default(),
                })
            })
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
