# server/

Optional managed/self-hostable sync backend. Provides push notifications, group invites, and
presence on top of the local-first core. The app works fully offline and with any third-party
CalDAV server without this service — it only adds convenience features.

Not yet implemented (M4+). See `docs/design/system-design.md` §7 for the full spec.

## Structure

```
server/
├── songbird-server/   Rust crate — the sync service binary
└── deploy/            Docker Compose self-hosting setup — see deploy/README.md
```

## Self-hosting

Once M4 lands, `server/deploy/` will contain a `docker-compose.yml` that brings up the full
stack. See `deploy/README.md` for requirements and configuration.
