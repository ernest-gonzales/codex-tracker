import type {
  ActiveSession,
  ActiveSessionsParams,
  CodexHome,
  ContextPressureStats,
  EventsParams,
  HomesResponse,
  IngestStats,
  LimitsResponse,
  ModelCostBreakdown,
  ModelEffortCostBreakdown,
  PricingRule,
  PricingRuleApi,
  RangeParams,
  SettingsResponse,
  TimeSeriesParams,
  TimeSeriesPoint,
  UsageEvent,
  UsageLimitCurrentResponse,
  UsageLimitWindow,
  UsageSummary
} from "../domain/types";
import { invokeCommand } from "./client";

export async function getSummary(params: RangeParams): Promise<UsageSummary> {
  return invokeCommand("summary", params);
}

export async function getTimeSeries(params: TimeSeriesParams): Promise<TimeSeriesPoint[]> {
  return invokeCommand("timeseries", params);
}

export async function getBreakdownCosts(params: RangeParams): Promise<ModelCostBreakdown[]> {
  return invokeCommand("breakdown_costs", params);
}

export async function getBreakdownEffortCosts(
  params: RangeParams
): Promise<ModelEffortCostBreakdown[]> {
  return invokeCommand("breakdown_effort_costs", params);
}

export async function getContextStats(params: RangeParams): Promise<ContextPressureStats> {
  return invokeCommand("context_stats", params);
}

export async function getLimitsLatest(): Promise<LimitsResponse> {
  return invokeCommand("limits_latest");
}

export async function getLimitsCurrent(): Promise<UsageLimitCurrentResponse> {
  return invokeCommand("limits_current");
}

export async function getLimitWindows(limit = 8): Promise<UsageLimitWindow[]> {
  return invokeCommand("limits_7d_windows", { limit });
}

export async function getEvents(params: EventsParams): Promise<UsageEvent[]> {
  return invokeCommand("events", params);
}

export async function getActiveSessions(
  params: ActiveSessionsParams
): Promise<ActiveSession[]> {
  return invokeCommand("context_sessions", params);
}

export async function runIngest(): Promise<IngestStats> {
  return invokeCommand("ingest");
}

export async function listPricing(): Promise<PricingRuleApi[]> {
  return invokeCommand("pricing_list");
}

export async function replacePricing(rules: PricingRule[]): Promise<{ updated: number }> {
  return invokeCommand("pricing_replace", { rules });
}

export async function recomputePricing(): Promise<{ updated: number }> {
  return invokeCommand("pricing_recompute");
}

export async function listHomes(): Promise<HomesResponse> {
  return invokeCommand("homes_list");
}

export async function createHome(payload: {
  path: string;
  label?: string;
}): Promise<CodexHome> {
  return invokeCommand("homes_create", payload);
}

export async function setActiveHome(id: number): Promise<CodexHome> {
  return invokeCommand("homes_set_active", { id });
}

export async function deleteHome(id: number): Promise<{ deleted: number }> {
  return invokeCommand("homes_delete", { id });
}

export async function clearHomeData(id: number): Promise<{ cleared: number }> {
  return invokeCommand("homes_clear_data", { id });
}

export async function getSettings(): Promise<SettingsResponse> {
  return invokeCommand("settings_get");
}

export async function updateSettings(payload: {
  codex_home?: string;
  context_active_minutes?: number;
}): Promise<SettingsResponse> {
  return invokeCommand("settings_put", payload);
}

export async function openLogsDir(): Promise<void> {
  await invokeCommand("open_logs_dir");
}
