CREATE TABLE IF NOT EXISTS usage_event (
  id TEXT PRIMARY KEY,
  ts TEXT NOT NULL,
  model TEXT NOT NULL,
  input_tokens INTEGER NOT NULL,
  cached_input_tokens INTEGER NOT NULL,
  output_tokens INTEGER NOT NULL,
  reasoning_output_tokens INTEGER NOT NULL,
  total_tokens INTEGER NOT NULL,
  context_used INTEGER NOT NULL,
  context_window INTEGER NOT NULL,
  cost_usd REAL,
  source TEXT NOT NULL,
  request_id TEXT,
  raw_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_usage_event_ts ON usage_event (ts);
CREATE INDEX IF NOT EXISTS idx_usage_event_model ON usage_event (model);

CREATE TABLE IF NOT EXISTS pricing_rule (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  model_pattern TEXT NOT NULL,
  input_per_1k REAL NOT NULL,
  cached_input_per_1k REAL NOT NULL,
  output_per_1k REAL NOT NULL,
  effective_from TEXT NOT NULL,
  effective_to TEXT
);

CREATE TABLE IF NOT EXISTS ingest_cursor (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  codex_home TEXT NOT NULL,
  file_path TEXT NOT NULL,
  inode INTEGER,
  mtime TEXT,
  byte_offset INTEGER NOT NULL,
  last_event_key TEXT,
  updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_ingest_cursor_source
  ON ingest_cursor (codex_home, file_path);

CREATE TABLE IF NOT EXISTS app_setting (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
