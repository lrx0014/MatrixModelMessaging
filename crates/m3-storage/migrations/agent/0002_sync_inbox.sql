-- Raw Client-Server timeline events are committed before SDK processing.
CREATE TABLE matrix_events (
    event_id TEXT PRIMARY KEY NOT NULL,
    room_id TEXT NOT NULL,
    raw_json TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('received', 'pending_decryption', 'ready', 'ignored')),
    body TEXT,
    sender TEXT NOT NULL,
    origin_server_ts INTEGER NOT NULL,
    diagnostic TEXT,
    CHECK ((state = 'ready') = (body IS NOT NULL))
);
CREATE TABLE sync_batches (
    next_batch TEXT PRIMARY KEY NOT NULL,
    since_token TEXT,
    captured_at INTEGER NOT NULL DEFAULT (unixepoch())
);
