//! Local CalDAV adapter (outbound, Phase 2). Holds the calendar's content key locally,
//! decrypts on the fly, and serves standard plaintext CalDAV on localhost to clients
//! explicitly pointed at it. The sync service is never involved in the decrypt.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("adapter server error: {0}")]
    ServerError(String),
}

/// TODO(M6): bind a CalDAV server to localhost authenticated with a locally-generated
/// credential (not the cloud account password).
pub async fn serve_local_caldav(_port: u16) -> Result<(), AdapterError> {
    Err(AdapterError::ServerError("serve_local_caldav not yet implemented (M6)".into()))
}
