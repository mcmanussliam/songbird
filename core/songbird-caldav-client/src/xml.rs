use quick_xml::{Reader, events::Event};

#[derive(Debug, Default)]
pub(crate) struct MultistatusEntry {
    pub href: String,
    pub top_level_status: Option<u16>,
    pub etag: Option<String>,
    pub display_name: Option<String>,
    pub sync_token: Option<String>,
    pub ctag: Option<String>,
    pub current_user_principal_href: Option<String>,
    pub calendar_home_set_href: Option<String>,
    pub is_calendar: bool,
    pub is_collection: bool,
}

pub(crate) struct MultistatusResult {
    pub entries: Vec<MultistatusEntry>,
    /// Top-level D:sync-token in a sync-collection REPORT response (RFC 6578).
    pub sync_token: Option<String>,
}

#[derive(Default)]
struct PropstatTemp {
    status: Option<u16>,
    etag: Option<String>,
    display_name: Option<String>,
    sync_token: Option<String>,
    ctag: Option<String>,
    current_user_principal_href: Option<String>,
    calendar_home_set_href: Option<String>,
    is_calendar: bool,
    is_collection: bool,
}

#[derive(Default)]
struct ResponseTemp {
    href: String,
    top_level_status: Option<u16>,
    propstats: Vec<PropstatTemp>,
}

fn parse_http_status(s: &str) -> Option<u16> {
    // "HTTP/1.1 200 OK" → 200
    s.split_whitespace().nth(1)?.parse().ok()
}

fn finalize_entry(r: ResponseTemp) -> MultistatusEntry {
    let mut e = MultistatusEntry {
        href: r.href,
        top_level_status: r.top_level_status,
        ..Default::default()
    };

    for ps in r.propstats {
        if ps.status != Some(200) {
            continue;
        }
        if ps.etag.is_some() { e.etag = ps.etag; }
        if ps.display_name.is_some() { e.display_name = ps.display_name; }
        if ps.sync_token.is_some() { e.sync_token = ps.sync_token; }
        if ps.ctag.is_some() { e.ctag = ps.ctag; }
        if ps.current_user_principal_href.is_some() {
            e.current_user_principal_href = ps.current_user_principal_href;
        }
        if ps.calendar_home_set_href.is_some() {
            e.calendar_home_set_href = ps.calendar_home_set_href;
        }
        if ps.is_calendar { e.is_calendar = true; }
        if ps.is_collection { e.is_collection = true; }
    }

    e
}

/// Parse a DAV:multistatus XML body into structured entries.
///
/// Handles both PROPFIND responses (properties in propstat/prop) and
/// sync-collection REPORT responses (RFC 6578, with top-level D:sync-token
/// and 404-status response entries for deleted resources).
pub(crate) fn parse_multistatus(xml: &str) -> quick_xml::Result<MultistatusResult> {
    let mut reader = Reader::from_str(xml);
    let mut result = MultistatusResult { entries: Vec::new(), sync_token: None };
    let mut path: Vec<String> = Vec::new();
    let mut text_buf = String::new();
    let mut cur_response: Option<ResponseTemp> = None;
    let mut cur_propstat: Option<PropstatTemp> = None;

    loop {
        match reader.read_event()? {
            Event::Start(e) => {
                let local = std::str::from_utf8(e.name().local_name().as_ref())
                    .unwrap_or("")
                    .to_string();
                text_buf.clear();

                match local.as_str() {
                    "response" if path.last().is_some_and(|p| p == "multistatus") => {
                        cur_response = Some(ResponseTemp::default());
                    }
                    "propstat" if cur_response.is_some() => {
                        cur_propstat = Some(PropstatTemp::default());
                    }
                    _ => {}
                }

                path.push(local);
            }

            // Self-closing elements — only care about resourcetype children.
            Event::Empty(e) => {
                let name = e.name();
                let local_bytes = name.local_name();
                let local = std::str::from_utf8(local_bytes.as_ref()).unwrap_or("");
                if let Some(ps) = cur_propstat.as_mut() {
                    match local {
                        "calendar" => ps.is_calendar = true,
                        "collection" => ps.is_collection = true,
                        _ => {}
                    }
                }
            }

            Event::Text(e) => {
                if let Ok(text) = e.unescape() {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        text_buf.push_str(trimmed);
                    }
                }
            }

            Event::End(_) => {
                let path_str = path.join("/");

                match path_str.as_str() {
                    "multistatus/sync-token" => {
                        if !text_buf.is_empty() {
                            result.sync_token = Some(text_buf.clone());
                        }
                    }
                    "multistatus/response/href" => {
                        if let Some(r) = cur_response.as_mut() {
                            r.href = text_buf.clone();
                        }
                    }
                    "multistatus/response/status" => {
                        if let Some(r) = cur_response.as_mut() {
                            r.top_level_status = parse_http_status(&text_buf);
                        }
                    }
                    "multistatus/response/propstat/status" => {
                        if let Some(ps) = cur_propstat.as_mut() {
                            ps.status = parse_http_status(&text_buf);
                        }
                    }
                    "multistatus/response/propstat/prop/getetag" => {
                        if let Some(ps) = cur_propstat.as_mut() {
                            ps.etag = Some(text_buf.trim_matches('"').to_string());
                        }
                    }
                    "multistatus/response/propstat/prop/displayname" => {
                        if let Some(ps) = cur_propstat.as_mut() {
                            ps.display_name = Some(text_buf.clone());
                        }
                    }
                    "multistatus/response/propstat/prop/sync-token" => {
                        if let Some(ps) = cur_propstat.as_mut() {
                            if !text_buf.is_empty() {
                                ps.sync_token = Some(text_buf.clone());
                            }
                        }
                    }
                    "multistatus/response/propstat/prop/getctag" => {
                        if let Some(ps) = cur_propstat.as_mut() {
                            ps.ctag = Some(text_buf.clone());
                        }
                    }
                    "multistatus/response/propstat/prop/current-user-principal/href" => {
                        if let Some(ps) = cur_propstat.as_mut() {
                            ps.current_user_principal_href = Some(text_buf.clone());
                        }
                    }
                    "multistatus/response/propstat/prop/calendar-home-set/href" => {
                        if let Some(ps) = cur_propstat.as_mut() {
                            ps.calendar_home_set_href = Some(text_buf.clone());
                        }
                    }
                    "multistatus/response/propstat" => {
                        if let (Some(r), Some(ps)) = (cur_response.as_mut(), cur_propstat.take()) {
                            r.propstats.push(ps);
                        }
                    }
                    "multistatus/response" => {
                        if let Some(r) = cur_response.take() {
                            result.entries.push(finalize_entry(r));
                        }
                    }
                    _ => {}
                }

                path.pop();
            }

            Event::Eof => break,
            _ => {}
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_principal_propfind() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/</D:href>
    <D:propstat>
      <D:prop>
        <D:current-user-principal><D:href>/principals/alice/</D:href></D:current-user-principal>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let ms = parse_multistatus(xml).unwrap();
        assert_eq!(ms.entries[0].current_user_principal_href.as_deref(), Some("/principals/alice/"));
    }

    #[test]
    fn parses_sync_collection_response() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/calendars/alice/default/event1.ics</D:href>
    <D:propstat>
      <D:prop><D:getetag>"etag1"</D:getetag></D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/calendars/alice/default/event2.ics</D:href>
    <D:status>HTTP/1.1 404 Not Found</D:status>
  </D:response>
  <D:sync-token>http://example.com/sync/42</D:sync-token>
</D:multistatus>"#;
        let ms = parse_multistatus(xml).unwrap();
        assert_eq!(ms.entries.len(), 2);
        assert_eq!(ms.entries[0].etag.as_deref(), Some("etag1"));
        assert_eq!(ms.entries[1].top_level_status, Some(404));
        assert_eq!(ms.sync_token.as_deref(), Some("http://example.com/sync/42"));
    }

    #[test]
    fn parses_calendar_listing() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav" xmlns:CS="http://calendarserver.org/ns/">
  <D:response>
    <D:href>/calendars/alice/personal/</D:href>
    <D:propstat>
      <D:prop>
        <D:displayname>Personal</D:displayname>
        <D:resourcetype><D:collection/><C:calendar/></D:resourcetype>
        <CS:getctag>abc123</CS:getctag>
        <D:sync-token>http://example.com/sync/1</D:sync-token>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let ms = parse_multistatus(xml).unwrap();
        let entry = &ms.entries[0];
        assert_eq!(entry.display_name.as_deref(), Some("Personal"));
        assert!(entry.is_calendar);
        assert!(entry.is_collection);
        assert_eq!(entry.ctag.as_deref(), Some("abc123"));
        assert_eq!(entry.sync_token.as_deref(), Some("http://example.com/sync/1"));
    }
}
