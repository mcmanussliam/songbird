//! The entire Dart-facing surface lives here. See system-design.md §5.11.
//!
//! TODO(M3+): wire these up for real once songbird-storage/songbird-recurrence/songbird-ical
//! are implemented (M1) and flutter_rust_bridge codegen is set up in app/rust_bridge/ (M3).
//! Signatures below are illustrative placeholders matching the design doc, not final.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("not yet implemented: {0}")]
    NotImplemented(&'static str),
}

pub async fn init(_db_path: String) -> Result<(), CoreError> {
    Err(CoreError::NotImplemented("init"))
}

// TODO(M3): create_local_calendar, create_event, update_event, delete_event,
// occurrences_in_range, add_caldav_account, sync_now, watch_occurrences, watch_sync_status —
// see system-design.md §5.11 for full signatures.
