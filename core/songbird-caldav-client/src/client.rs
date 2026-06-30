use reqwest::{Client, Method, StatusCode};

use crate::types::{Auth, CalDavConfig, CalDavError, CalendarInfo, FetchedResource, ResourceInfo, ResourceStatus, SyncOutcome};
use crate::xml::{MultistatusEntry, parse_multistatus};

pub struct CalDavClient {
    http: Client,
    base_url: String,
    auth: Auth,
}

impl CalDavClient {
    pub fn new(config: CalDavConfig) -> Result<Self, CalDavError> {
        let http = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;
        Ok(Self { http, base_url: config.base_url, auth: config.auth })
    }

    fn apply_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth {
            Auth::Basic { username, password } => builder.basic_auth(username, Some(password)),
            Auth::Bearer(token) => builder.bearer_auth(token),
        }
    }

    fn check_status(&self, status: StatusCode, url: &str) -> Result<(), CalDavError> {
        let code = status.as_u16();
        match code {
            401 | 403 => Err(CalDavError::AuthError { status: code, url: url.to_string() }),
            412 => Err(CalDavError::EtagConflict),
            // 207 Multi-Status is success for PROPFIND/REPORT
            207 => Ok(()),
            c if StatusCode::from_u16(c).is_ok_and(|s| s.is_success()) => Ok(()),
            _ => Err(CalDavError::Protocol(format!("HTTP {} from {}", code, url))),
        }
    }

    fn resolve_href(&self, href: &str) -> Result<String, CalDavError> {
        if href.starts_with("http://") || href.starts_with("https://") {
            return Ok(href.to_string());
        }
        let base = reqwest::Url::parse(&self.base_url)
            .map_err(|e| CalDavError::Protocol(format!("invalid base URL: {}", e)))?;
        base.join(href)
            .map(|u| u.to_string())
            .map_err(|e| CalDavError::Protocol(format!("cannot resolve {}: {}", href, e)))
    }

    fn well_known_url(&self) -> Result<String, CalDavError> {
        let mut base = reqwest::Url::parse(&self.base_url)
            .map_err(|e| CalDavError::Protocol(format!("invalid base URL: {}", e)))?;
        base.set_path("/.well-known/caldav");
        base.set_query(None);
        Ok(base.to_string())
    }

    async fn propfind(&self, url: &str, depth: &str, body: &str) -> Result<String, CalDavError> {
        let method = Method::from_bytes(b"PROPFIND").expect("static");
        let resp = self.apply_auth(
            self.http.request(method, url)
                .header("Depth", depth)
                .header("Content-Type", "application/xml; charset=utf-8")
                .body(body.to_string()),
        ).send().await?;
        let status = resp.status();
        self.check_status(status, url)?;
        Ok(resp.text().await?)
    }

    async fn report(&self, url: &str, body: &str) -> Result<String, CalDavError> {
        let method = Method::from_bytes(b"REPORT").expect("static");
        let resp = self.apply_auth(
            self.http.request(method, url)
                .header("Content-Type", "application/xml; charset=utf-8")
                .body(body.to_string()),
        ).send().await?;
        let status = resp.status();
        self.check_status(status, url)?;
        Ok(resp.text().await?)
    }

    /// Discover the principal URL and calendar home-set URL for this account.
    ///
    /// Tries PROPFIND at `base_url` first, then `/.well-known/caldav` as RFC 6764 fallback.
    pub async fn discover(&self) -> Result<(String, String), CalDavError> {
        let principal_body = concat!(
            r#"<?xml version="1.0" encoding="UTF-8"?>"#, "\n",
            r#"<D:propfind xmlns:D="DAV:"><D:prop><D:current-user-principal/></D:prop></D:propfind>"#
        );

        let principal_href = match self.propfind(&self.base_url, "0", principal_body).await {
            Ok(xml) => {
                let ms = parse_multistatus(&xml)?;
                ms.entries.into_iter().find_map(|e| e.current_user_principal_href)
            }
            Err(_) => None,
        };

        let principal_url = if let Some(href) = principal_href {
            self.resolve_href(&href)?
        } else {
            let wk = self.well_known_url()?;
            let xml = self.propfind(&wk, "0", principal_body).await.map_err(|_| {
                CalDavError::DiscoveryFailed(
                    "could not reach base URL or /.well-known/caldav".into(),
                )
            })?;
            let ms = parse_multistatus(&xml)?;
            ms.entries
                .into_iter()
                .find_map(|e| e.current_user_principal_href)
                .and_then(|href| self.resolve_href(&href).ok())
                .ok_or_else(|| {
                    CalDavError::DiscoveryFailed(
                        "no current-user-principal in PROPFIND response".into(),
                    )
                })?
        };

        let home_body = concat!(
            r#"<?xml version="1.0" encoding="UTF-8"?>"#, "\n",
            r#"<D:propfind xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">"#,
            r#"<D:prop><C:calendar-home-set/></D:prop></D:propfind>"#
        );

        let xml = self.propfind(&principal_url, "0", home_body).await?;
        let ms = parse_multistatus(&xml)?;
        let home_set = ms.entries
            .into_iter()
            .find_map(|e| e.calendar_home_set_href)
            .and_then(|href| self.resolve_href(&href).ok())
            .ok_or_else(|| {
                CalDavError::DiscoveryFailed("no calendar-home-set in principal PROPFIND".into())
            })?;

        Ok((principal_url, home_set))
    }

    /// List all calendar collections under `home_set_url`.
    pub async fn list_calendars(&self, home_set_url: &str) -> Result<Vec<CalendarInfo>, CalDavError> {
        let body = concat!(
            r#"<?xml version="1.0" encoding="UTF-8"?>"#, "\n",
            r#"<D:propfind xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav" xmlns:CS="http://calendarserver.org/ns/">"#,
            r#"<D:prop><D:displayname/><D:resourcetype/><D:sync-token/><CS:getctag/></D:prop>"#,
            r#"</D:propfind>"#
        );

        let xml = self.propfind(home_set_url, "1", body).await?;
        let ms = parse_multistatus(&xml)?;

        Ok(ms.entries
            .into_iter()
            .filter(|e| e.is_calendar)
            .map(|e| CalendarInfo {
                href: self.resolve_href(&e.href).unwrap_or(e.href),
                display_name: e.display_name,
                ctag: e.ctag,
                sync_token: e.sync_token,
            })
            .collect())
    }

    /// Fetch the iCalendar content of a single resource by path or URL.
    pub async fn fetch_resource(&self, href: &str) -> Result<String, CalDavError> {
        let url = self.resolve_href(href)?;
        let resp = self.apply_auth(self.http.get(&url)).send().await?;
        self.check_status(resp.status(), &url)?;
        Ok(resp.text().await?)
    }

    /// PUT a resource. Pass `etag` to update an existing resource (uses `If-Match`);
    /// omit to create a new one (uses `If-None-Match: *`). Returns the server's new ETag.
    pub async fn put_resource(
        &self,
        href: &str,
        ical: &str,
        etag: Option<&str>,
    ) -> Result<String, CalDavError> {
        let url = self.resolve_href(href)?;
        let mut builder = self.apply_auth(
            self.http.put(&url)
                .header("Content-Type", "text/calendar; charset=utf-8")
                .body(ical.to_string()),
        );
        builder = match etag {
            Some(t) => builder.header("If-Match", t),
            None => builder.header("If-None-Match", "*"),
        };
        let resp = builder.send().await?;
        let status = resp.status();
        self.check_status(status, &url)?;
        let new_etag = resp
            .headers()
            .get("ETag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        Ok(new_etag)
    }

    /// DELETE a resource. Pass `etag` to guard against concurrent modifications.
    pub async fn delete_resource(&self, href: &str, etag: Option<&str>) -> Result<(), CalDavError> {
        let url = self.resolve_href(href)?;
        let mut builder = self.apply_auth(self.http.delete(&url));
        if let Some(t) = etag {
            builder = builder.header("If-Match", t);
        }
        let resp = builder.send().await?;
        self.check_status(resp.status(), &url)?;
        Ok(())
    }

    /// Incremental sync via RFC 6578 sync-collection REPORT.
    ///
    /// Pass an empty `sync_token` for an initial full sync.
    async fn sync_via_sync_collection(
        &self,
        calendar_url: &str,
        sync_token: &str,
    ) -> Result<SyncOutcome, CalDavError> {
        let token_elem = if sync_token.is_empty() {
            "<D:sync-token/>".to_string()
        } else {
            format!("<D:sync-token>{}</D:sync-token>", sync_token)
        };

        let body = format!(
            concat!(
                r#"<?xml version="1.0" encoding="UTF-8"?>"#, "\n",
                r#"<D:sync-collection xmlns:D="DAV:">{}<D:sync-level>1</D:sync-level>"#,
                r#"<D:prop><D:getetag/></D:prop></D:sync-collection>"#
            ),
            token_elem
        );

        let xml = self.report(calendar_url, &body).await?;
        let ms = parse_multistatus(&xml)?;

        let mut outcome = SyncOutcome {
            new_cursor: ms.sync_token.as_deref().map(|t| format!("st:{}", t)),
            ..Default::default()
        };

        for entry in ms.entries {
            let href = self.resolve_href(&entry.href)?;
            if entry.top_level_status == Some(404) {
                outcome.deleted.push(ResourceInfo {
                    href,
                    etag: entry.etag,
                    status: ResourceStatus::Deleted,
                });
            } else {
                let ical_data = self.fetch_resource(&href).await?;
                outcome.fetched.push(FetchedResource {
                    info: ResourceInfo { href, etag: entry.etag, status: ResourceStatus::Present },
                    ical_data,
                });
            }
        }

        Ok(outcome)
    }

    /// CTag/ETag fallback sync for servers without sync-collection support.
    ///
    /// Returns all current resources when the CTag changed. Because CTag does not
    /// identify which resources were deleted, `outcome.is_full_sync` is set to `true`
    /// so the caller can infer deletions from the complete resource list.
    async fn sync_via_ctag(
        &self,
        calendar_url: &str,
        previous_ctag: Option<&str>,
    ) -> Result<SyncOutcome, CalDavError> {
        let body = concat!(
            r#"<?xml version="1.0" encoding="UTF-8"?>"#, "\n",
            r#"<D:propfind xmlns:D="DAV:" xmlns:CS="http://calendarserver.org/ns/">"#,
            r#"<D:prop><CS:getctag/><D:getetag/><D:resourcetype/></D:prop></D:propfind>"#
        );

        let xml = self.propfind(calendar_url, "1", body).await?;
        let ms = parse_multistatus(&xml)?;

        let (mut new_ctag, mut resources): (Option<String>, Vec<MultistatusEntry>) =
            (None, Vec::new());

        for entry in ms.entries {
            if entry.is_calendar {
                new_ctag = entry.ctag;
            } else if !entry.is_collection {
                resources.push(entry);
            }
        }

        if let (Some(prev), Some(curr)) = (previous_ctag, new_ctag.as_deref()) {
            if prev == curr {
                return Ok(SyncOutcome {
                    new_cursor: Some(format!("ct:{}", curr)),
                    ..Default::default()
                });
            }
        }

        let mut outcome = SyncOutcome {
            new_cursor: new_ctag.as_deref().map(|c| format!("ct:{}", c)),
            is_full_sync: true,
            ..Default::default()
        };

        for entry in resources {
            let href = self.resolve_href(&entry.href)?;
            let ical_data = self.fetch_resource(&href).await?;
            outcome.fetched.push(FetchedResource {
                info: ResourceInfo { href, etag: entry.etag, status: ResourceStatus::Present },
                ical_data,
            });
        }

        Ok(outcome)
    }

    /// Sync a calendar collection, returning changed/deleted resources with their content.
    ///
    /// Tries RFC 6578 sync-collection REPORT first, falls back to CTag/ETag polling.
    /// Pass `cursor` from the previous `SyncOutcome::new_cursor`; omit for an initial sync.
    pub async fn sync_calendar(
        &self,
        calendar_url: &str,
        cursor: Option<&str>,
    ) -> Result<SyncOutcome, CalDavError> {
        match cursor {
            Some(c) if c.starts_with("st:") => {
                let token = &c["st:".len()..];
                if let Ok(o) = self.sync_via_sync_collection(calendar_url, token).await {
                    return Ok(o);
                }
                // sync-token may have expired; fall through to full CTag sync
            }
            Some(c) if c.starts_with("ct:") => {
                let ctag = &c["ct:".len()..];
                return self.sync_via_ctag(calendar_url, Some(ctag)).await;
            }
            _ => {}
        }

        // First sync or unrecognized cursor: prefer sync-collection, fall back to CTag.
        match self.sync_via_sync_collection(calendar_url, "").await {
            Ok(o) => Ok(o),
            Err(_) => self.sync_via_ctag(calendar_url, None).await,
        }
    }
}
