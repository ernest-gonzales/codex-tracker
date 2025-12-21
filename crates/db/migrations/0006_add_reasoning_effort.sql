ALTER TABLE usage_event
  ADD COLUMN reasoning_effort TEXT;

CREATE INDEX IF NOT EXISTS idx_usage_event_home_model_effort
  ON usage_event (codex_home_id, model, reasoning_effort, ts);
