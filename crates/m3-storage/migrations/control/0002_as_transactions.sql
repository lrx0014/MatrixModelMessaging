-- AS delivery is an acknowledgement ledger, never a social inbox.
CREATE TABLE as_transactions (
    transaction_id TEXT PRIMARY KEY NOT NULL,
    received_at INTEGER NOT NULL DEFAULT (unixepoch()),
    event_count INTEGER NOT NULL CHECK (event_count >= 0)
);
CREATE TABLE as_users (
    user_id TEXT PRIMARY KEY NOT NULL
);
