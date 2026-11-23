CREATE TABLE instance_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    global_frozen INTEGER NOT NULL DEFAULT 0 CHECK (global_frozen IN (0, 1)),
    freeze_operation_id TEXT
);
INSERT INTO instance_state (id, global_frozen) VALUES (1, 0);
