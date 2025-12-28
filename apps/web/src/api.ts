import {
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
} from "./types";

const isTauriRuntime =
  typeof window !== "undefined" &&
  (Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) ||
    Boolean((window as unknown as { __TAURI__?: unknown }).__TAURI__));

async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

async function fetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, init);
  if (!response.ok) {
    throw new Error(`${response.status} ${response.statusText}`);
  }
  return response.json() as Promise<T>;
}

function buildQuery(params?: Record<string, string | number | undefined | null>): string {
  if (!params) {
    return "";
  }
  const search = new URLSearchParams();
  Object.entries(params).forEach(([key, value]) => {
    if (value === undefined || value === null || value === "") {
      return;
    }
    search.set(key, String(value));
  });
  const query = search.toString();
  return query ? `?${query}` : "";
}

export async function getSummary(params: RangeParams): Promise<UsageSummary> {
  if (isTauriRuntime) {
    return invokeCommand("summary", params);
  }
  return fetchJson(`/api/summary${buildQuery(params)}`);
}

export async function getTimeSeries(params: TimeSeriesParams): Promise<TimeSeriesPoint[]> {
  if (isTauriRuntime) {
    return invokeCommand("timeseries", params);
  }
  return fetchJson(`/api/timeseries${buildQuery(params)}`);
}

export async function getBreakdownCosts(params: RangeParams): Promise<ModelCostBreakdown[]> {
  if (isTauriRuntime) {
    return invokeCommand("breakdown_costs", params);
  }
  return fetchJson(`/api/breakdown/costs${buildQuery(params)}`);
}

export async function getBreakdownEffortCosts(
  params: RangeParams
): Promise<ModelEffortCostBreakdown[]> {
  if (isTauriRuntime) {
    return invokeCommand("breakdown_effort_costs", params);
  }
  return fetchJson(`/api/breakdown/effort/costs${buildQuery(params)}`);
}

export async function getContextStats(params: RangeParams): Promise<ContextPressureStats> {
  if (isTauriRuntime) {
    return invokeCommand("context_stats", params);
  }
  return fetchJson(`/api/context/stats${buildQuery(params)}`);
}

export async function getLimitsLatest(): Promise<LimitsResponse> {
  if (isTauriRuntime) {
    return invokeCommand("limits_latest");
  }
  return fetchJson("/api/limits");
}

export async function getLimitsCurrent(): Promise<UsageLimitCurrentResponse> {
  if (isTauriRuntime) {
    return invokeCommand("limits_current");
  }
  return fetchJson("/api/limits/current");
}

export async function getLimitWindows(limit = 8): Promise<UsageLimitWindow[]> {
  if (isTauriRuntime) {
    return invokeCommand("limits_7d_windows", { limit });
  }
  return fetchJson(`/api/limits/7d/windows${buildQuery({ limit })}`);
}

export async function getEvents(params: EventsParams): Promise<UsageEvent[]> {
  if (isTauriRuntime) {
    return invokeCommand("events", params);
  }
  return fetchJson(`/api/events${buildQuery(params)}`);
}

export async function getActiveSessions(
  params: ActiveSessionsParams
): Promise<ActiveSession[]> {
  if (isTauriRuntime) {
    return invokeCommand("context_sessions", params);
  }
  return fetchJson(`/api/context/sessions${buildQuery(params)}`);
}

export async function runIngest(): Promise<IngestStats> {
  if (isTauriRuntime) {
    return invokeCommand("ingest");
  }
  return fetchJson("/api/ingest/run", { method: "POST" });
}

export async function listPricing(): Promise<PricingRuleApi[]> {
  if (isTauriRuntime) {
    return invokeCommand("pricing_list");
  }
  return fetchJson("/api/pricing");
}

export async function replacePricing(rules: PricingRule[]): Promise<{ updated: number }> {
  if (isTauriRuntime) {
    return invokeCommand("pricing_replace", { rules });
  }
  return fetchJson("/api/pricing", {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(rules)
  });
}

export async function recomputePricing(): Promise<{ updated: number }> {
  if (isTauriRuntime) {
    return invokeCommand("pricing_recompute");
  }
  return fetchJson("/api/pricing/recompute", { method: "POST" });
}

export async function listHomes(): Promise<HomesResponse> {
  if (isTauriRuntime) {
    return invokeCommand("homes_list");
  }
  return fetchJson("/api/homes");
}

export async function createHome(payload: {
  path: string;
  label?: string;
}): Promise<CodexHome> {
  if (isTauriRuntime) {
    return invokeCommand("homes_create", payload);
  }
  return fetchJson("/api/homes", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload)
  });
}

export async function setActiveHome(id: number): Promise<CodexHome> {
  if (isTauriRuntime) {
    return invokeCommand("homes_set_active", { id });
  }
  return fetchJson("/api/homes/active", {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ id })
  });
}

export async function deleteHome(id: number): Promise<{ deleted: number }> {
  if (isTauriRuntime) {
    return invokeCommand("homes_delete", { id });
  }
  return fetchJson(`/api/homes/${id}`, { method: "DELETE" });
}

export async function clearHomeData(id: number): Promise<{ cleared: number }> {
  if (isTauriRuntime) {
    return invokeCommand("homes_clear_data", { id });
  }
  return fetchJson(`/api/homes/${id}/data`, { method: "DELETE" });
}

export async function getSettings(): Promise<SettingsResponse> {
  if (isTauriRuntime) {
    return invokeCommand("settings_get");
  }
  return fetchJson("/api/settings");
}

export async function updateSettings(payload: {
  codex_home?: string;
  context_active_minutes?: number;
}): Promise<SettingsResponse> {
  if (isTauriRuntime) {
    return invokeCommand("settings_put", payload);
  }
  return fetchJson("/api/settings", {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload)
  });
}

export async function openLogsDir(): Promise<void> {
  if (isTauriRuntime) {
    await invokeCommand("open_logs_dir");
    return;
  }
  throw new Error("Open logs is available in the desktop app only");
}
