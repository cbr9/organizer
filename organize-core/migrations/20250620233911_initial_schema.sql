-- This is the complete and final SQL migration for the journal database.

-- The `sessions` table tracks each individual execution of `organize run`.
CREATE TABLE IF NOT EXISTS sessions (
    id              INTEGER PRIMARY KEY,
    start_time      INTEGER NOT NULL,
    end_time        INTEGER,
    config          TEXT NOT NULL,
    status          TEXT NOT NULL
);

-- The `transactions` table records every undoable action performed during a session.
CREATE TABLE IF NOT EXISTS transactions (
    id              INTEGER PRIMARY KEY,
    session_id      INTEGER NOT NULL,
    type               TEXT NOT NULL,
    action             TEXT NOT NULL,
    receipt            TEXT NOT NULL, 
    timestamp       INTEGER NOT NULL,
    FOREIGN KEY(session_id) REFERENCES sessions(id)
);

