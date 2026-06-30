//! CalDAV client (RFC 4791) — inbound sync against third-party servers.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CalDavError {
    #[error("discovery failed: {0}")]
    DiscoveryFailed(String),
    #[error("request failed: {0}")]
    RequestFailed(String),
}

/// TODO(M2): PROPFIND-based discovery against a user-supplied base URL, well-known URI fallback.
pub async fn discover_calendars(_base_url: &str) -> Result<Vec<String>, CalDavError> {
    Err(CalDavError::DiscoveryFailed("discover_calendars not yet implemented (M2)".into()))
}

/// TODO(M2): sync-collection REPORT (RFC 6578) preferred, CTag/ETag fallback for servers
/// without sync-collection support.
pub async fn sync_collection(_calendar_url: &str, _cursor: Option<&str>) -> Result<(), CalDavError> {
    Err(CalDavError::RequestFailed("sync_collection not yet implemented (M2)".into()))
}
