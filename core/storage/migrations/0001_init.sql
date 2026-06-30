-- Initial schema. See system-design.md §5.4 for full rationale, especially:
--   - soft deletes (deleted_at) are mandatory, never a hard DELETE (AGENTS.md rule 4)
--   - dtstart/dtend stored UTC-normalized + is_date_only flag, never inferred from device locale

CREATE TABLE groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE memberships (
    user_id TEXT NOT NULL,
    group_id TEXT NOT NULL REFERENCES groups(id),
    role TEXT NOT NULL CHECK (role IN ('owner','editor','viewer','freebusy_only')),
    color TEXT NOT NULL,
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, group_id)
);

CREATE TABLE calendars (
    id TEXT PRIMARY KEY,
    group_id TEXT REFERENCES groups(id),
    display_name TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('local','caldav','native_sync','subscription')),
    source_config TEXT NOT NULL,
    encryption_key_id TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE events (
    id TEXT PRIMARY KEY,
    calendar_id TEXT NOT NULL REFERENCES calendars(id),
    summary TEXT NOT NULL,
    description TEXT,
    location TEXT,
    dtstart INTEGER NOT NULL,
    dtstart_is_date_only INTEGER NOT NULL DEFAULT 0,
    dtend INTEGER NOT NULL,
    dtend_is_date_only INTEGER NOT NULL DEFAULT 0,
    timezone TEXT,
    rrule TEXT,
    rdate TEXT,
    exdate TEXT,
    recurrence_id INTEGER,
    sequence INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'confirmed',
    last_modified INTEGER NOT NULL,
    etag TEXT,
    deleted_at INTEGER
);

CREATE INDEX idx_events_calendar_dtstart ON events(calendar_id, dtstart);
CREATE INDEX idx_events_recurrence_id ON events(recurrence_id);

CREATE TABLE sync_cursors (
    calendar_id TEXT NOT NULL,
    transport TEXT NOT NULL CHECK (transport IN ('caldav','native_sync')),
    cursor_token TEXT,
    last_synced_at INTEGER,
    PRIMARY KEY (calendar_id, transport)
);
