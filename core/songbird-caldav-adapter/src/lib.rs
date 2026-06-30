//! Local CalDAV adapter. See system-design.md §5.10 / §9.2 — this is the EteSync-precedent
//! pattern (ADR-0001): holds the calendar's content key locally, decrypts on the fly, and
//! serves standard plaintext CalDAV on localhost to clients explicitly pointed at it. The sync
//! service is never involved in the decrypt — see AGENTS.md rule 2.
//!
//! Deferred to M6 (Phase 2) per system-design.md §14. This crate is scaffolded now so the
//! workspace dependency graph is correct from the start.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("adapter server error: {0}")]
    ServerError(String),
}

/// TODO(M6): bind a CalDAV server to localhost, authenticated with a locally-generated
/// credential (not the cloud account password), per system-design.md §9.2.
pub async fn serve_local_caldav(_port: u16) -> Result<(), AdapterError> {
    Err(AdapterError::ServerError("serve_local_caldav not yet implemented (M6)".into()))
}
