//! SQLite-backed local store. See system-design.md §5.4.
//!
//! Invariants (do not relax without an ADR):
//! - Soft deletes only: `deleted_at` is set, never a hard DELETE (AGENTS.md rule 4).
//! - dtstart/dtend stored as UTC epoch milliseconds. The display timezone is reconstructed
//!   from the `timezone` column at read time, never inferred from the device locale.
//! - WAL mode enabled on open for safe concurrent reads from a background sync isolate.

mod migrate;
pub mod model;
mod repo;

use rusqlite::Connection;
use thiserror::Error;

pub use model::*;
pub use repo::Repo;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration error: {0}")]
    Migration(#[from] rusqlite_migration::Error),
    #[error("data corruption: {0}")]
    Corruption(String),
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &str) -> Result<Self, StorageError> {
        let mut conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        migrate::apply(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let mut conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        migrate::apply(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn repo(&self) -> Repo<'_> {
        Repo::new(&self.conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn open_and_migrate() {
        Store::open_in_memory().unwrap();
    }

    #[test]
    fn insert_and_retrieve_calendar() {
        let store = Store::open_in_memory().unwrap();
        let repo = store.repo();
        let id = Uuid::new_v4();
        repo.insert_calendar(id, None, "Personal", &CalendarSource::Local, "{}", None, 0).unwrap();
        let cal = repo.get_calendar(id).unwrap().unwrap();
        assert_eq!(cal.display_name, "Personal");
        assert_eq!(cal.source, CalendarSource::Local);
    }

    #[test]
    fn insert_and_retrieve_event() {
        let store = Store::open_in_memory().unwrap();
        let repo = store.repo();
        let cal_id = Uuid::new_v4();
        repo.insert_calendar(cal_id, None, "Work", &CalendarSource::Local, "{}", None, 0).unwrap();

        let ev = NewEvent {
            id: "test-uid-1".into(),
            calendar_id: cal_id,
            summary: "Team meeting".into(),
            description: None,
            location: None,
            dtstart_ms: 1_700_000_000_000,
            dtstart_is_date_only: false,
            dtend_ms: 1_700_003_600_000,
            dtend_is_date_only: false,
            timezone: Some("Europe/London".into()),
            rrule: None,
            rdate: None,
            exdate: None,
            recurrence_id_ms: None,
            sequence: 0,
            status: EventStatus::Confirmed,
            last_modified_ms: 1_700_000_000_000,
            etag: None,
        };
        repo.insert_event(&ev).unwrap();

        let fetched = repo.get_event("test-uid-1").unwrap().unwrap();
        assert_eq!(fetched.summary, "Team meeting");
        assert_eq!(fetched.timezone.as_deref(), Some("Europe/London"));
        assert!(fetched.deleted_at_ms.is_none());
    }

    #[test]
    fn soft_delete_sets_deleted_at() {
        let store = Store::open_in_memory().unwrap();
        let repo = store.repo();
        let cal_id = Uuid::new_v4();
        repo.insert_calendar(cal_id, None, "X", &CalendarSource::Local, "{}", None, 0).unwrap();
        repo.insert_event(&NewEvent {
            id: "del-uid".into(),
            calendar_id: cal_id,
            summary: "Gone".into(),
            description: None,
            location: None,
            dtstart_ms: 0,
            dtstart_is_date_only: false,
            dtend_ms: 0,
            dtend_is_date_only: false,
            timezone: None,
            rrule: None,
            rdate: None,
            exdate: None,
            recurrence_id_ms: None,
            sequence: 0,
            status: EventStatus::Confirmed,
            last_modified_ms: 0,
            etag: None,
        }).unwrap();

        let deleted = repo.soft_delete_event("del-uid", 9999).unwrap();
        assert!(deleted);

        let ev = repo.get_event("del-uid").unwrap().unwrap();
        assert_eq!(ev.deleted_at_ms, Some(9999));
    }
}
