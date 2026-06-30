//! SQLite-backed local store. See system-design.md §5.4.
//!
//! TODO(M1): wire up `rusqlite_migration` against `migrations/0001_init.sql`, open in WAL
//! mode, and expose a typed repository API (events, calendars, groups, sync_cursors) — not
//! raw SQL — to songbird-sync and songbird-core.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(String),
}

pub struct Store {
    _private: (),
}

impl Store {
    /// TODO(M1): open (or create) the SQLite file at `path`, run pending migrations, enable WAL.
    pub fn open(_path: &str) -> Result<Self, StorageError> {
        Err(StorageError::Database("Store::open not yet implemented (M1)".into()))
    }
}
