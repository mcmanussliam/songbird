use thiserror::Error;

#[derive(Debug, Clone)]
pub enum Auth {
    Basic { username: String, password: String },
    Bearer(String),
}

#[derive(Debug, Clone)]
pub struct CalDavConfig {
    pub base_url: String,
    pub auth: Auth,
}

#[derive(Debug, Clone)]
pub struct CalendarInfo {
    pub href: String,
    pub display_name: Option<String>,
    pub ctag: Option<String>,
    pub sync_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceStatus {
    Present,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct ResourceInfo {
    pub href: String,
    pub etag: Option<String>,
    pub status: ResourceStatus,
}

#[derive(Debug)]
pub struct FetchedResource {
    pub info: ResourceInfo,
    pub ical_data: String,
}

#[derive(Debug, Default)]
pub struct SyncOutcome {
    pub fetched: Vec<FetchedResource>,
    pub deleted: Vec<ResourceInfo>,
    /// Opaque cursor to pass back on the next sync call.
    /// Prefixed: "st:<sync-token>" for RFC 6578 sync-collection,
    ///            "ct:<ctag>"       for CTag/ETag fallback.
    pub new_cursor: Option<String>,
    /// True when the CTag fallback performed a full collection fetch — the caller
    /// must treat any local resource absent from `fetched` as deleted.
    pub is_full_sync: bool,
}

#[derive(Debug, Error)]
pub enum CalDavError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("XML parse error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("discovery failed: {0}")]
    DiscoveryFailed(String),
    #[error("authentication failed: HTTP {status} at {url}")]
    AuthError { status: u16, url: String },
    #[error("ETag conflict: resource changed on server since last fetch")]
    EtagConflict,
}
