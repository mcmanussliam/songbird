//! Integration tests against real CalDAV servers.
//!
//! All tests are `#[ignore]` by default. To run them:
//!
//! 1. Start the test servers:
//!    ```
//!    docker compose -f tests/caldav-servers/docker-compose.yml up -d
//!    ```
//! 2. Run:
//!    ```
//!    cargo test -p songbird-caldav-client -- --include-ignored
//!    ```
//!
//! Override defaults via env vars:
//!   CALDAV_TEST_URL  (default: http://localhost:5232)
//!   CALDAV_TEST_USER (default: testuser)
//!   CALDAV_TEST_PASS (default: testpass)

use songbird_caldav_client::{Auth, CalDavClient, CalDavConfig};
use uuid::Uuid;

fn config() -> CalDavConfig {
    CalDavConfig {
        base_url: std::env::var("CALDAV_TEST_URL")
            .unwrap_or_else(|_| "http://localhost:5232".to_string()),
        auth: Auth::Basic {
            username: std::env::var("CALDAV_TEST_USER")
                .unwrap_or_else(|_| "testuser".to_string()),
            password: std::env::var("CALDAV_TEST_PASS")
                .unwrap_or_else(|_| "testpass".to_string()),
        },
    }
}

fn server_available(base_url: &str) -> bool {
    use std::net::TcpStream;
    use std::time::Duration;
    // Parse host:port from the URL
    let without_scheme = base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host_port = without_scheme.split('/').next().unwrap_or("localhost:5232");
    let addr: std::net::SocketAddr = if host_port.contains(':') {
        host_port.parse().unwrap_or_else(|_| "127.0.0.1:5232".parse().unwrap())
    } else {
        format!("{}:5232", host_port)
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:5232".parse().unwrap())
    };
    TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok()
}

fn test_ical(uid: &str, summary: &str, dtstart: &str, dtend: &str) -> String {
    format!(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//songbird//integration-test//EN\r\n\
         BEGIN:VEVENT\r\nUID:{uid}\r\nDTSTART:{dtstart}\r\nDTEND:{dtend}\r\n\
         SUMMARY:{summary}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
    )
}

#[tokio::test]
#[ignore]
async fn radicale_discovery() {
    let cfg = config();
    if !server_available(&cfg.base_url) {
        eprintln!("radicale not available at {} — skipping", cfg.base_url);
        return;
    }
    let client = CalDavClient::new(cfg).unwrap();
    let (principal, home_set) = client.discover().await.expect("discover failed");
    assert!(!principal.is_empty(), "principal URL should not be empty");
    assert!(!home_set.is_empty(), "home-set URL should not be empty");
    eprintln!("principal: {principal}  home_set: {home_set}");
}

#[tokio::test]
#[ignore]
async fn radicale_calendar_roundtrip() {
    let cfg = config();
    if !server_available(&cfg.base_url) {
        eprintln!("radicale not available at {} — skipping", cfg.base_url);
        return;
    }
    let client = CalDavClient::new(cfg.clone()).unwrap();

    // Discover home-set
    let (_principal, home_set) = client.discover().await.expect("discover failed");

    // Create (or re-use) a test calendar
    let cal_href = format!("{}/integration-test/", home_set.trim_end_matches('/'));
    client
        .create_calendar(&cal_href, "Integration Test")
        .await
        .expect("create_calendar failed");

    // Verify it appears in the listing
    let calendars = client.list_calendars(&home_set).await.expect("list_calendars failed");
    assert!(
        calendars.iter().any(|c| c.href.contains("integration-test")),
        "created calendar not found in listing: {calendars:#?}"
    );

    // PUT an event
    let uid = format!("{}@songbird-test", Uuid::new_v4());
    let event_href = format!("{}/{}.ics", cal_href.trim_end_matches('/'), Uuid::new_v4());
    let ical = test_ical(&uid, "Initial Event", "20260101T100000Z", "20260101T110000Z");
    let etag = client.put_resource(&event_href, &ical, None).await.expect("PUT failed");

    // Initial sync: event should appear
    let outcome = client
        .sync_calendar(&cal_href, None)
        .await
        .expect("initial sync failed");
    let found = outcome.fetched.iter().any(|r| r.info.href.contains(
        event_href.trim_start_matches("http://").split('/').last().unwrap_or("")
    ));
    assert!(found || outcome.is_full_sync, "event not returned by initial sync");

    // Update the event
    let updated_ical =
        test_ical(&uid, "Updated Event", "20260101T100000Z", "20260101T110000Z");
    let etag_for_update = if etag.is_empty() { None } else { Some(etag.as_str()) };
    let new_etag = client
        .put_resource(&event_href, &updated_ical, etag_for_update)
        .await
        .expect("update PUT failed");

    // Incremental sync: only the updated event should come back
    let cursor = outcome.new_cursor.as_deref();
    let inc_outcome = client
        .sync_calendar(&cal_href, cursor)
        .await
        .expect("incremental sync failed");
    let updated_found = inc_outcome
        .fetched
        .iter()
        .any(|r| r.ical_data.contains("Updated Event"));
    assert!(
        updated_found || inc_outcome.is_full_sync,
        "updated event not found in incremental sync"
    );

    // Delete the event
    let delete_etag = if new_etag.is_empty() { None } else { Some(new_etag.as_str()) };
    client
        .delete_resource(&event_href, delete_etag)
        .await
        .expect("DELETE failed");

    // Final sync: event should appear as deleted (sync-collection) or absent (ctag)
    let del_cursor = inc_outcome.new_cursor.as_deref();
    let del_outcome = client
        .sync_calendar(&cal_href, del_cursor)
        .await
        .expect("post-delete sync failed");
    let deleted_href_suffix = event_href
        .trim_start_matches("http://")
        .split('/')
        .last()
        .unwrap_or("");
    let is_deleted = del_outcome
        .deleted
        .iter()
        .any(|r| r.href.contains(deleted_href_suffix));
    let still_present = del_outcome
        .fetched
        .iter()
        .any(|r| r.info.href.contains(deleted_href_suffix));
    assert!(
        is_deleted || (!still_present && del_outcome.is_full_sync),
        "deleted event still present in final sync"
    );

    eprintln!("radicale_calendar_roundtrip passed");
}

#[tokio::test]
#[ignore]
async fn radicale_etag_conflict() {
    let cfg = config();
    if !server_available(&cfg.base_url) {
        eprintln!("radicale not available at {} — skipping", cfg.base_url);
        return;
    }
    let client = CalDavClient::new(cfg.clone()).unwrap();
    let (_principal, home_set) = client.discover().await.expect("discover failed");
    let cal_href = format!("{}/integration-test/", home_set.trim_end_matches('/'));
    client
        .create_calendar(&cal_href, "Integration Test")
        .await
        .ok();

    let uid = format!("{}@songbird-test", Uuid::new_v4());
    let event_href = format!("{}/{}.ics", cal_href.trim_end_matches('/'), Uuid::new_v4());
    let ical = test_ical(&uid, "Conflict Test", "20260202T120000Z", "20260202T130000Z");

    // Create the resource
    let etag = client.put_resource(&event_href, &ical, None).await.expect("initial PUT failed");

    // Update with a stale ETag — server must reject with 412
    let stale_etag = "\"stale-etag-that-does-not-match\"";
    let result = client.put_resource(&event_href, &ical, Some(stale_etag)).await;
    assert!(
        matches!(result, Err(songbird_caldav_client::CalDavError::EtagConflict)),
        "expected EtagConflict, got: {result:?}"
    );

    // Cleanup
    let cleanup_etag = if etag.is_empty() { None } else { Some(etag.as_str()) };
    client.delete_resource(&event_href, cleanup_etag).await.ok();
    eprintln!("radicale_etag_conflict passed");
}
