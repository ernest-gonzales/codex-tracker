CREATE TABLE IF NOT EXISTS usage_limit_snapshot (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  codex_home_id INTEGER NOT NULL,
  ts TEXT NOT NULL,
  limit_type TEXT NOT NULL,
  percent_left REAL NOT NULL,
  reset_at TEXT NOT NULL,
  source TEXT NOT NULL,
  raw_line TEXT,
  FOREIGN KEY (codex_home_id) REFERENCES codex_home(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_limit_snapshot_home_type_ts
  ON usage_limit_snapshot (codex_home_id, limit_type, ts);

CREATE INDEX IF NOT EXISTS idx_limit_snapshot_home_type_reset
  ON usage_limit_snapshot (codex_home_id, limit_type, reset_at);
