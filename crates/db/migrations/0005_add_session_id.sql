ALTER TABLE usage_event
  ADD COLUMN session_id TEXT;

UPDATE usage_event
SET session_id = source
WHERE session_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_usage_event_home_session_ts
  ON usage_event (codex_home_id, session_id, ts);
