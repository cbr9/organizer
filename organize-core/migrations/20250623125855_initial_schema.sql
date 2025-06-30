CREATE TABLE IF NOT EXISTS sessions (
    id              INTEGER PRIMARY KEY,
    start_time      INTEGER NOT NULL,
    end_time        INTEGER,
    status          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS transactions (
    id              INTEGER PRIMARY KEY,
    session_id      INTEGER NOT NULL,
    type               TEXT NOT NULL,
    action             TEXT NOT NULL,
    receipt            TEXT NOT NULL, 
    timestamp       INTEGER NOT NULL,
    undo_status TEXT NOT NULL DEFAULT "PENDING",
    FOREIGN KEY(session_id) REFERENCES sessions(id)
);

