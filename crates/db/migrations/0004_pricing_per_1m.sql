ALTER TABLE pricing_rule
  ADD COLUMN input_per_1m REAL NOT NULL DEFAULT 0.0;

ALTER TABLE pricing_rule
  ADD COLUMN cached_input_per_1m REAL NOT NULL DEFAULT 0.0;

ALTER TABLE pricing_rule
  ADD COLUMN output_per_1m REAL NOT NULL DEFAULT 0.0;

UPDATE pricing_rule
SET input_per_1m = input_per_1k * 1000.0,
    cached_input_per_1m = cached_input_per_1k * 1000.0,
    output_per_1m = output_per_1k * 1000.0;
