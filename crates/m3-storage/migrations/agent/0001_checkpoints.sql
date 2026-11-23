CREATE TABLE checkpoints (
    stream TEXT PRIMARY KEY NOT NULL,
    position TEXT NOT NULL,
    committed_at INTEGER NOT NULL
);
