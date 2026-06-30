//! Sync engine. State is reconciled record-by-record, not via an event-sourced log.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("sync transport error: {0}")]
    Transport(String),
    #[error("merge conflict requiring user attention: {0}")]
    ConflictNeedsAttention(String),
}

/// TODO(M4): four-case merge algorithm:
/// 1. no local record → insert
/// 2. remote sequence > local sequence, no concurrent local edit → remote wins
/// 3. genuine concurrent edit → field-level merge, scalar conflicts surfaced to the user
/// 4. remote tombstone → soft-delete, surface conflict if local had unsynced edits
pub fn merge_record() -> Result<(), SyncError> {
    unimplemented!("merge_record not yet implemented")
}
