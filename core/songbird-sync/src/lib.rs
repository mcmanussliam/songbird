//! Sync engine. See system-design.md §5.7 (structure) and §10 (the full merge/conflict algorithm
//! — read §10.2 in full before implementing `merge_record`, the rules are specific).
//!
//! No append-only journal (finalized decision, see system-design.md §5.7 and the companion
//! market-analysis.md §3): state is reconciled record-by-record, not via an event-sourced log.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("sync transport error: {0}")]
    Transport(String),
    #[error("merge conflict requiring user attention: {0}")]
    ConflictNeedsAttention(String),
}

/// TODO(M4+): implement per system-design.md §10.2's four-case merge algorithm:
/// 1. no local record -> insert
/// 2. remote sequence > local sequence, no concurrent local edit -> remote wins
/// 3. genuine concurrent edit -> field-level merge, scalar conflicts surfaced to the user
/// 4. remote tombstone -> soft-delete, surface conflict if local had unsynced edits
pub fn merge_record() -> Result<(), SyncError> {
    unimplemented!("merge_record not yet implemented")
}
