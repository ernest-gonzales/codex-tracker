ALTER TABLE pricing_rule
  ADD COLUMN cached_input_per_1k REAL NOT NULL DEFAULT 0.0;
