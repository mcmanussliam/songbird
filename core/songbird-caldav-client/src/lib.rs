//! CalDAV client (RFC 4791) — inbound sync against third-party servers.
//!
//! Primary surface: [`CalDavClient`]. Construct one from a [`CalDavConfig`], then:
//! 1. Call [`CalDavClient::discover`] to find the principal and calendar home-set.
//! 2. Call [`CalDavClient::list_calendars`] to enumerate calendars.
//! 3. Call [`CalDavClient::sync_calendar`] to pull changes; store [`SyncOutcome::new_cursor`]
//!    and pass it back on the next call for incremental sync.
//! 4. Call [`CalDavClient::put_resource`] / [`CalDavClient::delete_resource`] to push local changes.

mod client;
mod types;
mod xml;

pub use client::CalDavClient;
pub use types::{
    Auth, CalDavConfig, CalDavError, CalendarInfo, FetchedResource, ResourceInfo, ResourceStatus,
    SyncOutcome,
};
