export type UsageSummary = {
  total_tokens: number;
  input_tokens: number;
  cached_input_tokens: number;
  output_tokens: number;
  reasoning_output_tokens: number;
  total_cost_usd: number | null;
  input_cost_usd: number | null;
  cached_input_cost_usd: number | null;
  output_cost_usd: number | null;
};

export type TimeSeriesPoint = {
  bucket_start: string;
  value: number;
};

export type ModelCostBreakdown = {
  model: string;
  input_tokens: number;
  cached_input_tokens: number;
  output_tokens: number;
  reasoning_output_tokens: number;
  total_tokens: number;
  input_cost_usd: number | null;
  cached_input_cost_usd: number | null;
  output_cost_usd: number | null;
  total_cost_usd: number | null;
};

export type ModelEffortCostBreakdown = {
  model: string;
  reasoning_effort: string | null;
  input_tokens: number;
  cached_input_tokens: number;
  output_tokens: number;
  reasoning_output_tokens: number;
  total_tokens: number;
  input_cost_usd: number | null;
  cached_input_cost_usd: number | null;
  output_cost_usd: number | null;
  total_cost_usd: number | null;
};

export type UsageEvent = {
  id: string;
  ts: string;
  model: string;
  usage: {
    input_tokens: number;
    cached_input_tokens: number;
    output_tokens: number;
    reasoning_output_tokens: number;
    total_tokens: number;
  };
  context: {
    context_used: number;
    context_window: number;
  };
  cost_usd: number | null;
  reasoning_effort: string | null;
  source: string;
  session_id: string;
};

export type ActiveSession = {
  session_id: string;
  model: string;
  last_seen: string;
  session_start: string;
  context_used: number;
  context_window: number;
};

export type ContextPressureStats = {
  avg_context_used: number | null;
  avg_context_window: number | null;
  avg_pressure_pct: number | null;
  sample_count: number;
};

export type UsageLimitSnapshot = {
  limit_type: string;
  percent_left: number;
  reset_at: string;
  observed_at: string;
  source: string;
  raw_line?: string | null;
};

export type UsageLimitWindow = {
  window_start: string | null;
  window_end: string;
  total_tokens: number | null;
  total_cost_usd: number | null;
  message_count: number | null;
  complete: boolean;
};

export type UsageLimitCurrentWindow = {
  window_start: string;
  window_end: string;
  total_tokens: number | null;
  total_cost_usd: number | null;
  message_count: number | null;
};

export type UsageLimitCurrentResponse = {
  primary: UsageLimitCurrentWindow | null;
  secondary: UsageLimitCurrentWindow | null;
};

export type LimitsResponse = {
  primary: UsageLimitSnapshot | null;
  secondary: UsageLimitSnapshot | null;
};

export type PricingRule = {
  id?: number | null;
  model_pattern: string;
  input_per_1m: number;
  cached_input_per_1m: number;
  output_per_1m: number;
  effective_from: string;
  effective_to?: string | null;
};

export type PricingRuleApi = PricingRule & {
  input_per_1k?: number;
  cached_input_per_1k?: number;
  output_per_1k?: number;
};

export type CodexHome = {
  id: number;
  label: string;
  path: string;
  created_at: string;
  last_seen_at?: string | null;
};

export type IngestStats = {
  files_scanned: number;
  files_skipped: number;
  events_inserted: number;
  bytes_read: number;
  issues: { file_path: string; message: string }[];
};

export type HomesResponse = {
  active_home_id: number | null;
  homes: CodexHome[];
};

export type SettingsResponse = {
  codex_home: string;
  active_home_id: number;
  context_active_minutes?: number;
  db_path?: string;
  pricing_defaults_path?: string;
  app_data_dir?: string;
  legacy_backup_dir?: string | null;
};

export type RangeParams = {
  range?: string;
  start?: string;
  end?: string;
};

export type TimeSeriesParams = RangeParams & {
  bucket?: string;
  metric?: string;
};

export type EventsParams = RangeParams & {
  limit?: number;
  offset?: number;
  model?: string;
};

export type ActiveSessionsParams = {
  active_minutes?: number;
};
