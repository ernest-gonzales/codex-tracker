CREATE TABLE IF NOT EXISTS message_event (
  id TEXT PRIMARY KEY,
  ts TEXT NOT NULL,
  role TEXT NOT NULL,
  source TEXT NOT NULL,
  session_id TEXT NOT NULL,
  raw_json TEXT,
  codex_home_id INTEGER,
  FOREIGN KEY (codex_home_id) REFERENCES codex_home(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_message_event_home_ts
  ON message_event (codex_home_id, ts);

CREATE INDEX IF NOT EXISTS idx_message_event_home_session_ts
  ON message_event (codex_home_id, session_id, ts);
