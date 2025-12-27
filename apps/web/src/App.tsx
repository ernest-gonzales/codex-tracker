import { useEffect, useMemo, useRef, useState } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis
} from "recharts";
import { CodexTrackerLogo } from "./Logo";
import {
  clearHomeData,
  createHome,
  deleteHome,
  getActiveSessions,
  getBreakdownCosts,
  getBreakdownEffortCosts,
  getContextStats,
  getEvents,
  getLimitWindows,
  getLimitsCurrent,
  getLimitsLatest,
  getSettings,
  getSummary,
  getTimeSeries,
  listHomes,
  listPricing,
  recomputePricing,
  replacePricing,
  runIngest,
  setActiveHome,
  updateSettings
} from "./api";
import {
  ActiveSession,
  CodexHome,
  ContextPressureStats,
  IngestStats,
  LimitsResponse,
  ModelCostBreakdown,
  ModelEffortCostBreakdown,
  PricingRule,
  PricingRuleApi,
  TimeSeriesPoint,
  UsageEvent,
  UsageLimitCurrentResponse,
  UsageLimitWindow,
  UsageSummary
} from "./types";

const RANGE_OPTIONS = [
  { value: "today", label: "Today" },
  { value: "last7days", label: "Last 7 Days" },
  { value: "last14days", label: "Last 14 Days" },
  { value: "thismonth", label: "This Month" },
  { value: "alltime", label: "All Time" },
  { value: "custom", label: "Custom" }
] as const;

const AUTO_REFRESH_OPTIONS = [
  { value: "off", label: "Off", ms: 0 },
  { value: "15s", label: "Every 15 seconds", ms: 15_000 },
  { value: "30s", label: "Every 30 seconds", ms: 30_000 },
  { value: "1m", label: "Every 1 minute", ms: 60_000 },
  { value: "5m", label: "Every 5 minutes", ms: 5 * 60_000 },
  { value: "15m", label: "Every 15 minutes", ms: 15 * 60_000 },
  { value: "30m", label: "Every 30 minutes", ms: 30 * 60_000 }
] as const;

type RangeValue = (typeof RANGE_OPTIONS)[number]["value"];
type AutoRefreshValue = (typeof AUTO_REFRESH_OPTIONS)[number]["value"];
type ChartBucketMode = "day" | "hour";

const currency = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2
});
const numberFormat = new Intl.NumberFormat("fr-FR");
const compactNumberFormat = new Intl.NumberFormat("fr-FR", {
  notation: "compact",
  compactDisplay: "short",
  maximumFractionDigits: 1
});
const dateTimeFormat = new Intl.DateTimeFormat("en-US", {
  month: "short",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit"
});
const dateFormat = new Intl.DateTimeFormat("en-US", {
  year: "numeric",
  month: "short",
  day: "2-digit"
});
const hourFormat = new Intl.DateTimeFormat("en-US", {
  hour: "2-digit",
  minute: "2-digit"
});
const EVENTS_PER_PAGE = 10;
const COST_BREAKDOWN_PAGE_SIZE = 10;

function formatCurrency(value: number | null | undefined) {
  if (value === null || value === undefined) return "n/a";
  return currency.format(value);
}

function formatNumber(value: number | null | undefined) {
  if (value === null || value === undefined) return "-";
  const absValue = Math.abs(value);
  if (absValue >= 1_000_000) {
    return compactNumberFormat.format(value);
  }
  return numberFormat.format(value);
}

function formatBucketLabel(value: string, bucket?: "hour" | "day") {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  if (bucket === "hour") {
    return hourFormat.format(parsed);
  }
  if (bucket === "day") {
    return dateFormat.format(parsed);
  }
  if (
    parsed.getHours() === 0 &&
    parsed.getMinutes() === 0 &&
    parsed.getSeconds() === 0
  ) {
    return dateFormat.format(parsed);
  }
  return dateTimeFormat.format(parsed);
}

function formatPercent(value: number | null | undefined) {
  if (value === null || value === undefined) return "-";
  return `${value.toFixed(1)}%`;
}

function formatPercentWhole(value: number | null | undefined) {
  if (value === null || value === undefined) return "-";
  return `${Math.round(value)}%`;
}

function formatCostPerMillion(cost: number | null | undefined, tokens: number | null | undefined) {
  if (cost === null || cost === undefined || !tokens) {
    return "-";
  }
  const perMillion = (cost / tokens) * 1_000_000;
  return currency.format(perMillion);
}

function formatResetLabel(value: string | null | undefined) {
  if (!value) return "-";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return dateTimeFormat.format(parsed);
}

function formatRelativeReset(value: string | null | undefined) {
  if (!value) return "-";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  const diffMs = parsed.getTime() - Date.now();
  const diffMinutes = Math.round(Math.abs(diffMs) / 60000);
  const hours = Math.floor(diffMinutes / 60);
  const minutes = diffMinutes % 60;
  const label = hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
  return diffMs >= 0 ? `in ${label}` : `${label} ago`;
}

function rangeLabel(value: RangeValue) {
  return RANGE_OPTIONS.find((option) => option.value === value)?.label ?? value;
}

function formatSessionLabel(sessionId: string) {
  const trimmed = sessionId.trim();
  if (trimmed.includes("/")) {
    const parts = trimmed.split("/").filter(Boolean);
    return parts[parts.length - 1] ?? trimmed;
  }
  if (trimmed.length > 18) {
    return `${trimmed.slice(0, 8)}...${trimmed.slice(-6)}`;
  }
  return trimmed;
}

function formatEffort(value: string | null | undefined) {
  if (!value) return "unknown";
  return value;
}

function formatDateTimeLocal(value?: string | null) {
  if (!value) return "";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  const offset = parsed.getTimezoneOffset() * 60 * 1000;
  return new Date(parsed.getTime() - offset).toISOString().slice(0, 16);
}

function parseDateTimeLocal(value: string) {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return parsed.toISOString();
}

function buildRangeParams(range: RangeValue, start?: string, end?: string) {
  if (range === "custom") {
    return {
      start: start ? new Date(start).toISOString() : undefined,
      end: end ? new Date(end).toISOString() : undefined
    };
  }
  return { range };
}

function csvEscape(value: string) {
  if (value.includes(",") || value.includes("\n") || value.includes("\"")) {
    return `"${value.replace(/\"/g, '""')}"`;
  }
  return value;
}

function downloadFile(name: string, contents: string, type: string) {
  const blob = new Blob([contents], { type });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}

export default function App() {
  const [range, setRange] = useState<RangeValue>("today");
  const [customStart, setCustomStart] = useState<string>("");
  const [customEnd, setCustomEnd] = useState<string>("");
  const [isSettingsOpen, setIsSettingsOpen] = useState<boolean>(false);
  const [summary, setSummary] = useState<UsageSummary | null>(null);
  const [activeSessions, setActiveSessions] = useState<ActiveSession[]>([]);
  const [activeMinutes, setActiveMinutes] = useState<number>(60);
  const [activeMinutesInput, setActiveMinutesInput] = useState<string>("60");
  const [contextStats, setContextStats] = useState<ContextPressureStats | null>(null);
  const [limits, setLimits] = useState<LimitsResponse | null>(null);
  const [limitWindows, setLimitWindows] = useState<UsageLimitWindow[]>([]);
  const [limitCurrent, setLimitCurrent] = useState<UsageLimitCurrentResponse | null>(null);
  const [tokensSeries, setTokensSeries] = useState<TimeSeriesPoint[]>([]);
  const [costSeries, setCostSeries] = useState<TimeSeriesPoint[]>([]);
  const [breakdown, setBreakdown] = useState<ModelCostBreakdown[]>([]);
  const [effortBreakdown, setEffortBreakdown] = useState<ModelEffortCostBreakdown[]>([]);
  const [events, setEvents] = useState<UsageEvent[]>([]);
  const [modelFilter, setModelFilter] = useState<string>("all");
  const [pricingRules, setPricingRules] = useState<PricingRule[]>([]);
  const [homes, setHomes] = useState<CodexHome[]>([]);
  const [activeHomeId, setActiveHomeId] = useState<number | null>(null);
  const [newHomePath, setNewHomePath] = useState<string>("");
  const [newHomeLabel, setNewHomeLabel] = useState<string>("");
  const [homeStatus, setHomeStatus] = useState<string>("");
  const [pricingStatus, setPricingStatus] = useState<string>("");
  const [settingsStatus, setSettingsStatus] = useState<string>("");
  const [storageInfo, setStorageInfo] = useState<{
    dbPath?: string;
    pricingDefaultsPath?: string;
    appDataDir?: string;
    legacyBackupDir?: string | null;
  } | null>(null);
  const [ingestStats, setIngestStats] = useState<IngestStats | null>(null);
  const [isIngesting, setIsIngesting] = useState<boolean>(false);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string>("");
  const [costBreakdownPage, setCostBreakdownPage] = useState<number>(1);
  const [costSeriesPage, setCostSeriesPage] = useState<number>(1);
  const [eventsPage, setEventsPage] = useState<number>(1);
  const [autoRefresh, setAutoRefresh] = useState<AutoRefreshValue>("30s");
  const [chartBucketMode, setChartBucketMode] = useState<ChartBucketMode>("hour");
  const [exportMenuValue, setExportMenuValue] = useState<string>("");
  const ingestInFlight = useRef<boolean>(false);
  const [costBreakdownTab, setCostBreakdownTab] = useState<"model" | "day">("model");
  const [expandedModels, setExpandedModels] = useState<Set<string>>(new Set());

  const rangeParams = useMemo(
    () => buildRangeParams(range, customStart, customEnd),
    [range, customStart, customEnd]
  );

  const rangeParamsKey = useMemo(() => JSON.stringify(rangeParams), [rangeParams]);
  const chartBucket = chartBucketMode;
  const chartBucketLabel = chartBucket === "hour" ? "hour" : "day";
  const totalCostBreakdownPages = Math.max(
    1,
    Math.ceil(breakdown.length / COST_BREAKDOWN_PAGE_SIZE)
  );
  const pagedCostBreakdown = breakdown.slice(
    (costBreakdownPage - 1) * COST_BREAKDOWN_PAGE_SIZE,
    costBreakdownPage * COST_BREAKDOWN_PAGE_SIZE
  );
  const totalCostSeriesPages = Math.max(
    1,
    Math.ceil(costSeries.length / COST_BREAKDOWN_PAGE_SIZE)
  );
  const pagedCostSeries = costSeries.slice(
    (costSeriesPage - 1) * COST_BREAKDOWN_PAGE_SIZE,
    costSeriesPage * COST_BREAKDOWN_PAGE_SIZE
  );

  useEffect(() => {
    if (costBreakdownTab !== "model") {
      return;
    }
    setCostBreakdownPage((page) => Math.min(page, totalCostBreakdownPages));
  }, [breakdown.length, costBreakdownTab, totalCostBreakdownPages]);

  useEffect(() => {
    if (costBreakdownTab !== "day") {
      return;
    }
    setCostSeriesPage((page) => Math.min(page, totalCostSeriesPages));
  }, [costSeries.length, costBreakdownTab, totalCostSeriesPages]);

  async function refreshAll() {
    setLoading(true);
    setError("");
    try {
      const bucket = chartBucket;
      const [
        summaryData,
        tokensData,
        costData,
        breakdownData,
        effortBreakdownData,
        contextStatsData,
        limitsData,
        limitsCurrentData,
        limitWindowsData,
        eventsData,
        sessionsData
      ] = await Promise.all([
        getSummary(rangeParams),
        getTimeSeries({ ...rangeParams, bucket, metric: "tokens" }),
        getTimeSeries({ ...rangeParams, bucket, metric: "cost" }),
        getBreakdownCosts(rangeParams),
        getBreakdownEffortCosts(rangeParams),
        getContextStats(rangeParams),
        getLimitsLatest(),
        getLimitsCurrent(),
        getLimitWindows(8),
        getEvents({
          ...rangeParams,
          limit: 200,
          model: modelFilter === "all" ? undefined : modelFilter
        }),
        getActiveSessions({ active_minutes: activeMinutes })
      ]);

      setSummary(summaryData);
      setTokensSeries(tokensData);
      setCostSeries(costData);
      setBreakdown(breakdownData);
      setEffortBreakdown(effortBreakdownData);
      setContextStats(contextStatsData);
      setLimits(limitsData);
      setLimitCurrent(limitsCurrentData);
      setLimitWindows(limitWindowsData);
      setEvents(eventsData);
      setActiveSessions(sessionsData);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load data");
    } finally {
      setLoading(false);
    }
  }

  async function refreshPricing() {
    try {
      const pricingData = await listPricing();
      const normalized = (pricingData || []).map((rule) => {
        if (
          rule.input_per_1m ||
          rule.cached_input_per_1m ||
          rule.output_per_1m ||
          (!rule.input_per_1k && !rule.cached_input_per_1k && !rule.output_per_1k)
        ) {
          return rule;
        }
        return {
          ...rule,
          input_per_1m: (rule.input_per_1k ?? 0) * 1000,
          cached_input_per_1m: (rule.cached_input_per_1k ?? 0) * 1000,
          output_per_1m: (rule.output_per_1k ?? 0) * 1000
        };
      });
      setPricingRules(normalized);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load pricing");
    }
  }

  async function refreshHomes() {
    try {
      const data = await listHomes();
      setHomes(data.homes || []);
      setActiveHomeId(data.active_home_id ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load homes");
    }
  }

  async function refreshSettings() {
    try {
      const data = await getSettings();
      const minutes = data.context_active_minutes ?? 60;
      setActiveMinutes(minutes);
      setActiveMinutesInput(minutes.toString());
      setStorageInfo({
        dbPath: data.db_path,
        pricingDefaultsPath: data.pricing_defaults_path,
        appDataDir: data.app_data_dir,
        legacyBackupDir: data.legacy_backup_dir ?? null
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load settings");
    }
  }

  useEffect(() => {
    refreshAll();
  }, [rangeParamsKey, modelFilter, activeMinutes, chartBucket]);

  useEffect(() => {
    setEventsPage(1);
  }, [rangeParamsKey, modelFilter]);

  useEffect(() => {
    refreshPricing();
    refreshHomes();
    refreshSettings();
  }, []);

  const autoRefreshInterval = useMemo(() => {
    return AUTO_REFRESH_OPTIONS.find((option) => option.value === autoRefresh)?.ms ?? 0;
  }, [autoRefresh]);

  useEffect(() => {
    if (!autoRefreshInterval) {
      return;
    }
    const intervalId = window.setInterval(() => {
      handleIngest();
    }, autoRefreshInterval);
    return () => window.clearInterval(intervalId);
  }, [autoRefreshInterval, rangeParamsKey, modelFilter, activeMinutes, chartBucket]);

  const modelOptions = useMemo(() => {
    const models = new Set(breakdown.map((item) => item.model));
    return ["all", ...Array.from(models).sort()];
  }, [breakdown]);

  const activeHome = useMemo(() => {
    if (activeHomeId === null) {
      return null;
    }
    return homes.find((home) => home.id === activeHomeId) ?? null;
  }, [homes, activeHomeId]);

  const isRefreshing = loading || isIngesting;

  const costChartData = breakdown.map((item) => ({
    model: item.model,
    input: item.input_cost_usd ?? 0,
    cached: item.cached_input_cost_usd ?? 0,
    output: item.output_cost_usd ?? 0
  }));

  const effortByModel = useMemo(() => {
    const grouped = new Map<string, ModelEffortCostBreakdown[]>();
    effortBreakdown.forEach((item) => {
      if (!grouped.has(item.model)) {
        grouped.set(item.model, []);
      }
      grouped.get(item.model)?.push(item);
    });
    for (const [model, rows] of grouped) {
      rows.sort((a, b) => (b.total_tokens ?? 0) - (a.total_tokens ?? 0));
      grouped.set(model, rows);
    }
    return grouped;
  }, [effortBreakdown]);

  function toggleModelExpansion(model: string) {
    setExpandedModels((prev) => {
      const next = new Set(prev);
      if (next.has(model)) {
        next.delete(model);
      } else {
        next.add(model);
      }
      return next;
    });
  }

  const totalCostKnown = breakdown.some((item) => item.total_cost_usd !== null);
  const totalEventPages = Math.max(1, Math.ceil(events.length / EVENTS_PER_PAGE));
  const contextSampleCount = contextStats?.sample_count ?? 0;
  const rangeAvgLabel =
    contextSampleCount > 0
      ? `Average context size for ${rangeLabel(range)}: ${formatNumber(
          Math.round(contextStats?.avg_context_used ?? 0)
        )} tokens (${formatPercent(contextStats?.avg_pressure_pct ?? 0)}) · n=${contextSampleCount}`
      : `Average context size for ${rangeLabel(range)}: —`;
  const rangeAvgTooltip =
    contextSampleCount > 0
      ? "Average context usage and pressure for events with known context in this range."
      : "No context window data in this range yet.";
  const limitWindowRows = useMemo(() => {
    if (!limitWindows.length) {
      return [];
    }
    let previous: UsageLimitWindow | null = null;
    return limitWindows.map((window) => {
      let delta: number | null = null;
      if (
        window.complete &&
        previous?.total_tokens !== null &&
        previous?.total_tokens !== undefined &&
        window.total_tokens !== null &&
        window.total_tokens !== undefined &&
        previous.total_tokens > 0
      ) {
        delta =
          ((window.total_tokens - previous.total_tokens) / previous.total_tokens) * 100;
      }
      if (window.complete) {
        previous = window;
      }
      return { ...window, delta };
    });
  }, [limitWindows]);
  const pagedEvents = useMemo(() => {
    const startIndex = (eventsPage - 1) * EVENTS_PER_PAGE;
    return events.slice(startIndex, startIndex + EVENTS_PER_PAGE);
  }, [events, eventsPage]);

  useEffect(() => {
    setEventsPage((prev) => Math.min(Math.max(prev, 1), totalEventPages));
  }, [totalEventPages]);

  async function handleSetActiveHome(nextId: number) {
    if (activeHomeId === nextId) {
      return;
    }
    setHomeStatus("Switching...");
    try {
      const updated = await setActiveHome(nextId);
      setActiveHomeId(updated.id);
      await refreshHomes();
      await refreshAll();
      setHomeStatus("Active");
    } catch (err) {
      setHomeStatus(err instanceof Error ? err.message : "Switch failed");
    }
  }

  async function handleAddHome() {
    const path = newHomePath.trim();
    if (!path) {
      setHomeStatus("Path required");
      return;
    }
    setHomeStatus("Adding...");
    try {
      const created = await createHome({
        path,
        label: newHomeLabel.trim().length ? newHomeLabel.trim() : undefined
      });
      setActiveHomeId(created.id);
      setNewHomePath("");
      setNewHomeLabel("");
      await refreshHomes();
      await refreshAll();
      setHomeStatus("Added");
    } catch (err) {
      setHomeStatus(err instanceof Error ? err.message : "Add failed");
    }
  }

  async function handleDeleteHome(homeId: number) {
    if (!window.confirm("Delete this home and all its data?")) {
      return;
    }
    setHomeStatus("Deleting...");
    try {
      await deleteHome(homeId);
      await refreshHomes();
      await refreshAll();
      setHomeStatus("Deleted");
    } catch (err) {
      setHomeStatus(err instanceof Error ? err.message : "Delete failed");
    }
  }

  async function handleSavePricing() {
    setPricingStatus("Saving...");
    try {
      await replacePricing(pricingRules);
      setPricingStatus("Saved");
      await refreshAll();
    } catch (err) {
      setPricingStatus(err instanceof Error ? err.message : "Save failed");
    }
  }

  async function handleRecomputeCosts() {
    setPricingStatus("Recomputing...");
    try {
      await recomputePricing();
      setPricingStatus("Recomputed");
      await refreshAll();
    } catch (err) {
      setPricingStatus(err instanceof Error ? err.message : "Recompute failed");
    }
  }

  async function handleSaveActiveMinutes() {
    const parsed = Number(activeMinutesInput);
    if (!Number.isFinite(parsed) || parsed <= 0) {
      setSettingsStatus("Enter a valid minute value");
      return;
    }
    setSettingsStatus("Saving...");
    try {
      const data = await updateSettings({ context_active_minutes: parsed });
      const minutes = data.context_active_minutes ?? parsed;
      setActiveMinutes(minutes);
      setActiveMinutesInput(minutes.toString());
      setSettingsStatus("Saved");
      setStorageInfo({
        dbPath: data.db_path,
        pricingDefaultsPath: data.pricing_defaults_path,
        appDataDir: data.app_data_dir,
        legacyBackupDir: data.legacy_backup_dir ?? null
      });
      await refreshAll();
    } catch (err) {
      setSettingsStatus(err instanceof Error ? err.message : "Save failed");
    }
  }

  async function handleIngest() {
    if (ingestInFlight.current) {
      return;
    }
    ingestInFlight.current = true;
    setIsIngesting(true);
    try {
      const stats = await runIngest();
      setIngestStats(stats);
      await refreshAll();
    } catch (err) {
      const message = err instanceof Error ? err.message : "Ingest failed";
      setError(message);
    } finally {
      ingestInFlight.current = false;
      setIsIngesting(false);
    }
  }

  async function handleDeleteData() {
    if (!activeHomeId) {
      setHomeStatus("Select a home first");
      return;
    }
    if (!window.confirm("Delete all ingested data for the active home?")) {
      return;
    }
    setHomeStatus("Deleting data...");
    try {
      await clearHomeData(activeHomeId);
      await refreshAll();
      setHomeStatus("Data deleted");
    } catch (err) {
      setHomeStatus(err instanceof Error ? err.message : "Delete failed");
    }
  }

  function updatePricingRule(index: number, patch: Partial<PricingRule>) {
    setPricingRules((prev) =>
      prev.map((rule, idx) => (idx === index ? { ...rule, ...patch } : rule))
    );
  }

  function addPricingRule() {
    setPricingRules((prev) => [
      ...prev,
      {
        model_pattern: "*",
        input_per_1m: 0,
        cached_input_per_1m: 0,
        output_per_1m: 0,
        effective_from: new Date().toISOString(),
        effective_to: null
      }
    ]);
  }

  function exportJson() {
    const payload = {
      summary,
      breakdown,
      effort_breakdown: effortBreakdown,
      events
    };
    downloadFile("codex-tracker-export.json", JSON.stringify(payload, null, 2), "application/json");
  }

  function exportCsv() {
    const header = [
      "ts",
      "model",
      "input_tokens",
      "cached_input_tokens",
      "output_tokens",
      "reasoning_output_tokens",
      "total_tokens",
      "cost_usd",
      "reasoning_effort",
      "source"
    ];
    const rows = events.map((event) => [
      event.ts,
      event.model,
      event.usage.input_tokens.toString(),
      event.usage.cached_input_tokens.toString(),
      event.usage.output_tokens.toString(),
      event.usage.reasoning_output_tokens.toString(),
      event.usage.total_tokens.toString(),
      event.cost_usd?.toString() ?? "",
      event.reasoning_effort ?? "",
      event.source
    ]);
    const csv = [header, ...rows]
      .map((row) => row.map(csvEscape).join(","))
      .join("\n");
    downloadFile("codex-tracker-events.csv", csv, "text/csv");
  }

  function handleExportMenu(event: React.ChangeEvent<HTMLSelectElement>) {
    const value = event.target.value;
    if (!value) {
      return;
    }
    if (value === "json") {
      exportJson();
    } else {
      exportCsv();
    }
    setExportMenuValue("");
  }

  return (
    <div className="app">
      <div className="glow" aria-hidden="true" />
      {isSettingsOpen ? (
        <section className="settings-page">
          <header className="settings-header">
            <div>
              <h1 className="settings-title">Settings</h1>
              <p className="settings-subtitle">
                Codex homes, active window, pricing rules, and storage paths.
              </p>
            </div>
            <button
              className="button ghost"
              type="button"
              onClick={() => setIsSettingsOpen(false)}
            >
              Back to Dashboard
            </button>
          </header>
          <section className="grid settings-grid settings-grid-full">
            <div className="panel">
              <div className="panel-header">
                <div>
                  <h2>Codex Homes</h2>
                  <p>Switch between tracked log directories.</p>
                </div>
                <button className="button ghost" onClick={refreshHomes}>
                  Reload
                </button>
              </div>
              <label className="label">Active Home</label>
              <select
                className="select-native"
                value={activeHomeId ?? ""}
                onChange={(event) => {
                  if (!event.target.value) {
                    return;
                  }
                  handleSetActiveHome(Number(event.target.value));
                }}
              >
                {homes.length === 0 && (
                  <option value="" disabled>
                    No homes found
                  </option>
                )}
                {homes.map((home) => (
                  <option key={home.id} value={home.id}>
                    {home.label || home.path}
                  </option>
                ))}
              </select>
              <div className="note">
                {activeHome ? `Path: ${activeHome.path}` : "Select a home to see details."}
              </div>
              {homes.length > 0 && (
                <div className="table-wrap">
                  <table className="compact-table">
                    <thead>
                      <tr>
                        <th>Label</th>
                        <th>Path</th>
                        <th>Last Seen</th>
                        <th />
                      </tr>
                    </thead>
                    <tbody>
                      {homes.map((home) => (
                        <tr key={home.id}>
                          <td>{home.label}</td>
                          <td>{home.path}</td>
                          <td>
                            {home.last_seen_at
                              ? new Date(home.last_seen_at).toLocaleString()
                              : "-"}
                          </td>
                          <td>
                            <button
                              className="button ghost small"
                              onClick={() => handleDeleteHome(home.id)}
                              disabled={homes.length === 1}
                            >
                              Delete
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
              <div className="row">
                <button
                  className="button danger"
                  onClick={handleDeleteData}
                  disabled={!activeHomeId}
                >
                  Delete Ingested Data
                </button>
              </div>
              <label className="label">Add Home</label>
              <input
                className="input"
                value={newHomePath}
                onChange={(event) => setNewHomePath(event.target.value)}
                placeholder="/Users/you/.codex"
              />
              <input
                className="input"
                value={newHomeLabel}
                onChange={(event) => setNewHomeLabel(event.target.value)}
                placeholder="Label (optional)"
              />
              <div className="row">
                <button className="button" onClick={handleAddHome}>
                  Add Home
                </button>
                <span className="status">{homeStatus}</span>
              </div>
            </div>

            <div className="panel">
              <div className="panel-header">
                <div>
                  <h2>Active Window</h2>
                  <p>Control what counts as active sessions.</p>
                </div>
              </div>
              <label className="label">Active Session Window (Minutes)</label>
              <input
                className="input"
                type="number"
                min="1"
                value={activeMinutesInput}
                onChange={(event) => setActiveMinutesInput(event.target.value)}
              />
              <div className="row">
                <button className="button" onClick={handleSaveActiveMinutes}>
                  Save Window
                </button>
                <span className="status">{settingsStatus}</span>
              </div>
              <div className="note">Updates the Active Sessions panel and refresh cycle.</div>
            </div>

            <div className="panel">
              <div className="panel-header">
                <div>
                  <h2>Storage</h2>
                  <p>Local app data paths for the desktop client.</p>
                </div>
              </div>
              <div className="settings-kv">
                <div className="settings-kv-row">
                  <span className="settings-kv-key">App Data</span>
                  <span className="settings-kv-value">{storageInfo?.appDataDir ?? "—"}</span>
                </div>
                <div className="settings-kv-row">
                  <span className="settings-kv-key">Database</span>
                  <span className="settings-kv-value">{storageInfo?.dbPath ?? "—"}</span>
                </div>
                <div className="settings-kv-row">
                  <span className="settings-kv-key">Pricing</span>
                  <span className="settings-kv-value">
                    {storageInfo?.pricingDefaultsPath ?? "—"}
                  </span>
                </div>
                {storageInfo?.legacyBackupDir && (
                  <div className="settings-kv-row">
                    <span className="settings-kv-key">Legacy Backup</span>
                    <span className="settings-kv-value">{storageInfo.legacyBackupDir}</span>
                  </div>
                )}
              </div>
              <div className="note">
                Desktop builds keep data in the OS app data directory.
              </div>
            </div>
          </section>

          <section className="panel settings-pricing">
            <div className="panel-header">
              <div>
                <h2>Pricing Rules</h2>
                <p>Override model pricing and recompute stored costs.</p>
              </div>
              <div className="row">
                <button className="button ghost" onClick={addPricingRule}>
                  Add Rule
                </button>
                <button className="button" onClick={handleSavePricing}>
                  Save Pricing
                </button>
                <button className="button ghost" onClick={handleRecomputeCosts}>
                  Recompute Costs
                </button>
                <span className="status">{pricingStatus}</span>
              </div>
            </div>
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>Model Pattern</th>
                    <th>Input / 1M</th>
                    <th>Cached / 1M</th>
                    <th>Output / 1M</th>
                    <th>Effective From</th>
                    <th>Effective To</th>
                  </tr>
                </thead>
                <tbody>
                  {pricingRules.map((rule, index) => {
                    const effectiveToValue = formatDateTimeLocal(rule.effective_to);
                    return (
                      <tr key={rule.id ?? `${rule.model_pattern}-${index}`}>
                        <td>
                          <input
                            className="input"
                            value={rule.model_pattern}
                            onChange={(event) =>
                              updatePricingRule(index, { model_pattern: event.target.value })
                            }
                          />
                        </td>
                        <td>
                          <input
                            className="input"
                            type="number"
                            step="0.0001"
                            value={rule.input_per_1m}
                            onChange={(event) =>
                              updatePricingRule(index, {
                                input_per_1m: Number(event.target.value)
                              })
                            }
                          />
                          <div className="input-hint">{formatCurrency(rule.input_per_1m)}</div>
                        </td>
                        <td>
                          <input
                            className="input"
                            type="number"
                            step="0.0001"
                            value={rule.cached_input_per_1m}
                            onChange={(event) =>
                              updatePricingRule(index, {
                                cached_input_per_1m: Number(event.target.value)
                              })
                            }
                          />
                          <div className="input-hint">
                            {formatCurrency(rule.cached_input_per_1m)}
                          </div>
                        </td>
                        <td>
                          <input
                            className="input"
                            type="number"
                            step="0.0001"
                            value={rule.output_per_1m}
                            onChange={(event) =>
                              updatePricingRule(index, {
                                output_per_1m: Number(event.target.value)
                              })
                            }
                          />
                          <div className="input-hint">{formatCurrency(rule.output_per_1m)}</div>
                        </td>
                        <td>
                          <input
                            className="input"
                            type="datetime-local"
                            value={formatDateTimeLocal(rule.effective_from)}
                            onChange={(event) =>
                              updatePricingRule(index, {
                                effective_from: parseDateTimeLocal(event.target.value)
                              })
                            }
                          />
                        </td>
                        <td>
                          <input
                            className="input"
                            type="datetime-local"
                            value={effectiveToValue}
                            onChange={(event) =>
                              updatePricingRule(index, {
                                effective_to: event.target.value
                                  ? parseDateTimeLocal(event.target.value)
                                  : null
                              })
                            }
                          />
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </section>
        </section>
      ) : (
        <>
          <header className="hero">
            <div className="hero-copy">
              <div className="brand">
                <div className="brand-mark" aria-hidden="true">
                  <CodexTrackerLogo />
                </div>
                <div className="brand-text">
                  <h1 className="brand-name">
                    Codex <span>Tracker</span>
                  </h1>
                </div>
              </div>
              <p className="brand-tagline">Local usage intelligence for Codex.</p>
              <p className="subtitle">
                Monitor tokens, cost, and context pressure across models with fresh ranges and fast
                exports.
              </p>
            </div>
            <div className="range-panel">
              <div className="range-panel-header">
                <label className="label">Range</label>
                <button
                  className="icon-button"
                  type="button"
                  onClick={() => setIsSettingsOpen(true)}
                  aria-label="Open settings"
                  title="Settings"
                >
                  <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
                    <path
                      d="M12 15.6a3.6 3.6 0 1 0 0-7.2 3.6 3.6 0 0 0 0 7.2Z"
                      fill="currentColor"
                      opacity="0.9"
                    />
                    <path
                      d="M19.14 12.94a7.8 7.8 0 0 0 0-1.88l2.03-1.58a.9.9 0 0 0 .22-1.16l-1.92-3.32a.9.9 0 0 0-1.1-.4l-2.4.97a7.6 7.6 0 0 0-1.63-.94l-.37-2.55a.9.9 0 0 0-.89-.78H8.9a.9.9 0 0 0-.89.78l-.37 2.55c-.58.23-1.13.54-1.63.94l-2.4-.97a.9.9 0 0 0-1.1.4L.6 8.32a.9.9 0 0 0 .22 1.16l2.03 1.58a7.8 7.8 0 0 0 0 1.88L.82 14.52a.9.9 0 0 0-.22 1.16l1.92 3.32a.9.9 0 0 0 1.1.4l2.4-.97c.5.4 1.05.71 1.63.94l.37 2.55a.9.9 0 0 0 .89.78h4.2a.9.9 0 0 0 .89-.78l.37-2.55c.58-.23 1.13-.54 1.63-.94l2.4.97a.9.9 0 0 0 1.1-.4l1.92-3.32a.9.9 0 0 0-.22-1.16l-2.03-1.58Z"
                      fill="currentColor"
                      opacity="0.55"
                    />
                  </svg>
                </button>
              </div>
              <select
                className="select-native select-compact"
                value={range}
                onChange={(event) => setRange(event.target.value as RangeValue)}
              >
                {RANGE_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
              {range === "custom" && (
                <div className="custom-range custom-range-compact">
                  <input
                    type="datetime-local"
                    value={customStart}
                    onChange={(event) => setCustomStart(event.target.value)}
                  />
                  <input
                    type="datetime-local"
                    value={customEnd}
                    onChange={(event) => setCustomEnd(event.target.value)}
                  />
                </div>
              )}
              <button className="button" onClick={handleIngest} disabled={isRefreshing}>
                <span className="button-content">
                  {isRefreshing && <span className="spinner" aria-hidden="true" />}
                  <span>Refresh</span>
                </span>
              </button>
              <div className="ingest-stats">
                <span>
                  {ingestStats
                    ? `Last scan: ${formatNumber(ingestStats.files_scanned)} files · Inserted ${formatNumber(
                        ingestStats.events_inserted
                      )} events`
                    : "Last scan: —"}
                </span>
              </div>
              <div className="row">
                <label className="label">Export</label>
                <select
                  className="select-native select-compact"
                  value={exportMenuValue}
                  onChange={handleExportMenu}
                >
                  <option value="" disabled>
                    Choose format
                  </option>
                  <option value="json">Export JSON</option>
                  <option value="csv">Export CSV</option>
                </select>
              </div>
              <div className="row">
                <label className="label">Auto refresh</label>
                <select
                  className="select-native select-compact"
                  value={autoRefresh}
                  onChange={(event) => setAutoRefresh(event.target.value as AutoRefreshValue)}
                >
                  {AUTO_REFRESH_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </div>
              {error && <p className="error">{error}</p>}
            </div>
          </header>

      <section className="grid summary-grid">
        <div className="card">
          <p className="card-label">Total Tokens</p>
          <p className="card-value">{formatNumber(summary?.total_tokens)}</p>
          <p className="card-meta">Input {formatNumber(summary?.input_tokens)}</p>
        </div>
        <div className="card">
          <p className="card-label">Total Cost</p>
          <p className="card-value">{formatCurrency(summary?.total_cost_usd)}</p>
          <p className="card-meta">Output {formatCurrency(summary?.output_cost_usd)}</p>
        </div>
        <div className="card">
          <p className="card-label">Cached Input</p>
          <p className="card-value">{formatNumber(summary?.cached_input_tokens)}</p>
          <p className="card-meta">Cost {formatCurrency(summary?.cached_input_cost_usd)}</p>
        </div>
        <div className="card">
          <p className="card-label">Output Tokens</p>
          <p className="card-value">{formatNumber(summary?.output_tokens)}</p>
          <p className="card-meta">
            Reasoning {formatNumber(summary?.reasoning_output_tokens)}
          </p>
        </div>
      </section>

      <section className="panel active-sessions">
        <div className="panel-header">
          <div>
            <h2>Active Sessions</h2>
            <p>Context pressure for sessions seen in the last {activeMinutes} minutes.</p>
            <div className="chip-row">
              <span className="chip" title={rangeAvgTooltip}>
                {rangeAvgLabel}
              </span>
            </div>
          </div>
          <span className="tag">Window {activeMinutes}m</span>
        </div>
        {activeSessions.length === 0 ? (
          <p className="note">No recent sessions in this window.</p>
        ) : (
          <div className="session-list">
            {activeSessions.map((session) => {
              const percent =
                session.context_window > 0
                  ? Math.min(
                      100,
                      (session.context_used / session.context_window) * 100
                    )
                  : 0;
              return (
                <div className="session-card" key={session.session_id}>
                  <div className="session-header">
                    <div>
                      <div className="session-id" title={session.session_id}>
                        {formatSessionLabel(session.session_id)}
                      </div>
                      <div className="session-meta">
                        Model {session.model} · Started{" "}
                        {formatBucketLabel(session.session_start)} · Last{" "}
                        {formatBucketLabel(session.last_seen)}
                      </div>
                    </div>
                    <div className="session-metrics">
                      <span>
                        {formatNumber(session.context_used)} /{" "}
                        {formatNumber(session.context_window)}
                      </span>
                      <span>{Math.round(percent)}%</span>
                    </div>
                  </div>
                  <div className="session-bar">
                    <div className="session-bar-fill" style={{ width: `${percent}%` }} />
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </section>

      <section className="panel limits-panel">
        <div className="panel-header">
          <div>
            <h2>Usage Limits</h2>
            <p>Current 5-hour and 7-day usage limits with recent reset windows.</p>
          </div>
          <span className="tag">Logs</span>
        </div>
        <div className="limits-grid">
          <div className="limit-card">
            <p className="card-label">5h Remaining</p>
            <div className="limit-inline-row">
              <p className="limit-value">
                {formatPercentWhole(limits?.primary?.percent_left)}
              </p>
              <p className="card-meta limit-inline-meta">
                Resets {formatResetLabel(limits?.primary?.reset_at)} ·{" "}
                {formatRelativeReset(limits?.primary?.reset_at)}
              </p>
            </div>
            <p className="card-meta card-meta-compact">
              <span className="limit-inline">Messages</span>{" "}
              {formatNumber(limitCurrent?.primary?.message_count)}{" "}
              <span className="limit-inline">Tokens</span>{" "}
              {formatNumber(limitCurrent?.primary?.total_tokens)}{" "}
              <span className="limit-inline">Cost</span>{" "}
              {formatCurrency(limitCurrent?.primary?.total_cost_usd)}
            </p>
          </div>
          <div className="limit-card">
            <p className="card-label">7d Remaining</p>
            <div className="limit-inline-row">
              <p className="limit-value">
                {formatPercentWhole(limits?.secondary?.percent_left)}
              </p>
              <p className="card-meta limit-inline-meta">
                Resets {formatResetLabel(limits?.secondary?.reset_at)} ·{" "}
                {formatRelativeReset(limits?.secondary?.reset_at)}
              </p>
            </div>
            <p className="card-meta card-meta-compact">
              <span className="limit-inline">Messages</span>{" "}
              {formatNumber(limitCurrent?.secondary?.message_count)}{" "}
              <span className="limit-inline">Tokens</span>{" "}
              {formatNumber(limitCurrent?.secondary?.total_tokens)}{" "}
              <span className="limit-inline">Cost</span>{" "}
              {formatCurrency(limitCurrent?.secondary?.total_cost_usd)}
            </p>
          </div>
        </div>
        {limitWindowRows.length === 0 ? (
          <p className="note">No 7-day reset windows captured yet.</p>
        ) : (
          <div className="limits-table">
            <div className="limits-row limits-header">
              <span>Window</span>
              <span>Tokens</span>
              <span>Cost</span>
              <span>Messages</span>
              <span>Change</span>
            </div>
            {limitWindowRows.map((window) => {
              const startLabel = window.window_start
                ? formatBucketLabel(window.window_start)
                : "—";
              const endLabel = formatBucketLabel(window.window_end);
              const now = Date.now();
              const startMs = window.window_start
                ? new Date(window.window_start).getTime()
                : Number.NaN;
              const endMs = new Date(window.window_end).getTime();
              const isCurrent =
                !Number.isNaN(endMs) &&
                (Number.isNaN(startMs) ? now < endMs : now >= startMs && now < endMs);
              const deltaLabel =
                window.delta === null || window.delta === undefined
                  ? "—"
                  : `${window.delta >= 0 ? "+" : ""}${window.delta.toFixed(1)}%`;
              const deltaClass =
                window.delta === null || window.delta === undefined
                  ? ""
                  : window.delta < 0
                    ? "neg"
                    : "pos";
              return (
                <div
                  key={`${window.window_end}-${window.window_start ?? "none"}`}
                  className={`limits-row ${window.complete ? "" : "incomplete"}`}
                >
                  <span>
                    {startLabel} → {endLabel}
                    {isCurrent && <span className="limit-badge">Current</span>}
                  </span>
                  <span>{formatNumber(window.total_tokens)}</span>
                  <span>{formatCurrency(window.total_cost_usd)}</span>
                  <span>{formatNumber(window.message_count)}</span>
                  <span className={deltaClass}>{deltaLabel}</span>
                </div>
              );
            })}
          </div>
        )}
      </section>

      <section className="panel chart-panel">
        <div className="panel-header">
          <div>
            <h2>Token + Cost Trends</h2>
            <p>Compare tokens and spend by {chartBucketLabel}.</p>
          </div>
          <div className="panel-actions">
            <label className="label">Bucket</label>
            <select
              className="select-native select-inline"
              value={chartBucketMode}
              onChange={(event) => setChartBucketMode(event.target.value as ChartBucketMode)}
            >
              <option value="day">Day</option>
              <option value="hour">Hour</option>
            </select>
          </div>
        </div>
        <div className="chart-grid">
          <div className="chart-card">
            <div className="panel-header">
              <div>
                <h2>Token Velocity</h2>
                <p>Totals by {chartBucketLabel}.</p>
              </div>
            </div>
            <div className="chart">
              <ResponsiveContainer width="100%" height={260}>
                <LineChart data={tokensSeries}>
                  <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.08)" />
                  <XAxis
                  dataKey="bucket_start"
                  tick={{ fill: "#cbd6ff", fontSize: 11 }}
                  tickFormatter={(value) => formatBucketLabel(value as string, chartBucket)}
                />
                <YAxis tick={{ fill: "#cbd6ff", fontSize: 11 }} />
                <Tooltip
                  contentStyle={{ background: "#10142b", border: "1px solid #2f3c6d" }}
                  labelFormatter={(value) =>
                    formatBucketLabel(value as string, chartBucket)
                  }
                  formatter={(value) => [formatNumber(value as number), "Tokens"]}
                />
                  <Line
                    type="monotone"
                    dataKey="value"
                    stroke="#7df9ff"
                    strokeWidth={2}
                    dot={false}
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </div>
          <div className="chart-card">
            <div className="panel-header">
              <div>
                <h2>Cost Drift</h2>
                <p>USD spend by {chartBucketLabel}.</p>
              </div>
            </div>
            <div className="chart">
              <ResponsiveContainer width="100%" height={260}>
                <LineChart data={costSeries}>
                  <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.08)" />
                  <XAxis
                  dataKey="bucket_start"
                  tick={{ fill: "#cbd6ff", fontSize: 11 }}
                  tickFormatter={(value) => formatBucketLabel(value as string, chartBucket)}
                />
                <YAxis tick={{ fill: "#cbd6ff", fontSize: 11 }} />
                <Tooltip
                  contentStyle={{ background: "#10142b", border: "1px solid #2f3c6d" }}
                  labelFormatter={(value) =>
                    formatBucketLabel(value as string, chartBucket)
                  }
                  formatter={(value) => [formatCurrency(value as number), "Cost"]}
                />
                  <Line
                    type="monotone"
                    dataKey="value"
                    stroke="#ffb347"
                    strokeWidth={2}
                    dot={false}
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </div>
        </div>
      </section>

      <section className="panel">
        <div className="panel-header">
          <div>
            <h2>Cost Breakdown</h2>
            <p>Switch between model and time-bucket cost views.</p>
          </div>
          {!totalCostKnown && (
            <span className="tag">Pricing missing for some models</span>
          )}
        </div>
        <div className="tabs">
          <button
            className={`tab ${costBreakdownTab === "model" ? "active" : ""}`}
            onClick={() => setCostBreakdownTab("model")}
            type="button"
          >
            Per Model
          </button>
          <button
            className={`tab ${costBreakdownTab === "day" ? "active" : ""}`}
            onClick={() => setCostBreakdownTab("day")}
            type="button"
          >
            Per {chartBucketLabel === "hour" ? "Hour" : "Day"}
          </button>
        </div>
        {costBreakdownTab === "model" ? (
          <>
            <div className="chart">
              <ResponsiveContainer width="100%" height={320}>
                <BarChart
                  data={costChartData}
                  margin={{ top: 10, right: 20, left: 0, bottom: 10 }}
                >
                  <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.08)" />
                  <XAxis dataKey="model" tick={{ fill: "#cbd6ff", fontSize: 11 }} />
                  <YAxis tick={{ fill: "#cbd6ff", fontSize: 11 }} />
                  <Tooltip
                    contentStyle={{ background: "#10142b", border: "1px solid #2f3c6d" }}
                    labelFormatter={formatBucketLabel}
                    formatter={(value) => formatCurrency(value as number)}
                  />
                  <Legend />
                  <Bar dataKey="input" stackId="cost" fill="#7df9ff" />
                  <Bar dataKey="cached" stackId="cost" fill="#8f7dff" />
                  <Bar dataKey="output" stackId="cost" fill="#ffb347" />
                </BarChart>
              </ResponsiveContainer>
            </div>
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>Model</th>
                    <th>Effort</th>
                    <th>Total Tokens</th>
                    <th>Input</th>
                    <th>Cached</th>
                    <th>Output</th>
                    <th>Reasoning</th>
                    <th>Cost</th>
                    <th>Cost / 1M</th>
                  </tr>
                </thead>
                <tbody>
                  {pagedCostBreakdown.flatMap((item) => {
                    const effortRows = effortByModel.get(item.model) ?? [];
                    const isExpanded = expandedModels.has(item.model);
                    const isExpandable = effortRows.length > 0;
                    const totalRow = effortRows.length
                      ? (() => {
                          const totalCostKnown = effortRows.some(
                            (row) => row.total_cost_usd !== null
                          );
                          const totals = effortRows.reduce(
                            (acc, row) => ({
                              input_tokens: acc.input_tokens + row.input_tokens,
                              cached_input_tokens:
                                acc.cached_input_tokens + row.cached_input_tokens,
                              output_tokens: acc.output_tokens + row.output_tokens,
                              reasoning_output_tokens:
                                acc.reasoning_output_tokens + row.reasoning_output_tokens,
                              total_tokens: acc.total_tokens + row.total_tokens,
                              total_cost_usd:
                                (acc.total_cost_usd ?? 0) + (row.total_cost_usd ?? 0)
                            }),
                            {
                              input_tokens: 0,
                              cached_input_tokens: 0,
                              output_tokens: 0,
                              reasoning_output_tokens: 0,
                              total_tokens: 0,
                              total_cost_usd: 0
                            }
                          );
                          return {
                            ...totals,
                            total_cost_usd: totalCostKnown ? totals.total_cost_usd : null
                          };
                        })()
                      : {
                          input_tokens: item.input_tokens,
                          cached_input_tokens: item.cached_input_tokens,
                          output_tokens: item.output_tokens,
                          reasoning_output_tokens: item.reasoning_output_tokens,
                          total_tokens: item.total_tokens,
                          total_cost_usd: item.total_cost_usd
                        };
                    const rows = [
                      <tr key={`${item.model}-total`} className="model-row">
                        <td>
                          {isExpandable ? (
                            <button
                              type="button"
                              className="model-toggle"
                              onClick={() => toggleModelExpansion(item.model)}
                              aria-expanded={isExpanded}
                            >
                              <span className="caret">{isExpanded ? "▾" : "▸"}</span>
                              {item.model}
                            </button>
                          ) : (
                            item.model
                          )}
                        </td>
                        <td>Total</td>
                        <td>{formatNumber(totalRow.total_tokens)}</td>
                        <td>{formatNumber(totalRow.input_tokens)}</td>
                        <td>{formatNumber(totalRow.cached_input_tokens)}</td>
                        <td>{formatNumber(totalRow.output_tokens)}</td>
                        <td>{formatNumber(totalRow.reasoning_output_tokens)}</td>
                        <td>{formatCurrency(totalRow.total_cost_usd)}</td>
                        <td>
                          {formatCostPerMillion(totalRow.total_cost_usd, totalRow.total_tokens)}
                        </td>
                      </tr>
                    ];
                    if (!isExpanded) {
                      return rows;
                    }
                    effortRows.forEach((effortItem, index) => {
                      rows.push(
                        <tr
                          className="effort-row"
                          key={`${item.model}-effort-${effortItem.reasoning_effort}-${index}`}
                        >
                          <td />
                          <td>{formatEffort(effortItem.reasoning_effort)}</td>
                          <td>{formatNumber(effortItem.total_tokens)}</td>
                          <td>{formatNumber(effortItem.input_tokens)}</td>
                          <td>{formatNumber(effortItem.cached_input_tokens)}</td>
                          <td>{formatNumber(effortItem.output_tokens)}</td>
                          <td>{formatNumber(effortItem.reasoning_output_tokens)}</td>
                          <td>{formatCurrency(effortItem.total_cost_usd)}</td>
                          <td>
                            {formatCostPerMillion(
                              effortItem.total_cost_usd,
                              effortItem.total_tokens
                            )}
                          </td>
                        </tr>
                      );
                    });
                    return rows;
                  })}
                </tbody>
              </table>
            </div>
            {breakdown.length > COST_BREAKDOWN_PAGE_SIZE && (
              <div className="table-footer">
                <span className="note">
                  Showing{" "}
                  {breakdown.length === 0
                    ? 0
                    : (costBreakdownPage - 1) * COST_BREAKDOWN_PAGE_SIZE + 1}
                  -
                  {Math.min(costBreakdownPage * COST_BREAKDOWN_PAGE_SIZE, breakdown.length)} of{" "}
                  {breakdown.length} models
                </span>
                <div className="pagination">
                  <button
                    className="button ghost small"
                    onClick={() => setCostBreakdownPage((page) => Math.max(1, page - 1))}
                    disabled={costBreakdownPage === 1}
                  >
                    Previous
                  </button>
                  <span className="pagination-status">
                    Page {costBreakdownPage} of {totalCostBreakdownPages}
                  </span>
                  <button
                    className="button ghost small"
                    onClick={() =>
                      setCostBreakdownPage((page) =>
                        Math.min(totalCostBreakdownPages, page + 1)
                      )
                    }
                    disabled={costBreakdownPage === totalCostBreakdownPages}
                  >
                    Next
                  </button>
                </div>
              </div>
            )}
          </>
        ) : (
          <>
            <div className="chart">
              <ResponsiveContainer width="100%" height={320}>
                <BarChart data={costSeries} margin={{ top: 10, right: 20, left: 0, bottom: 10 }}>
                  <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.08)" />
                  <XAxis
                    dataKey="bucket_start"
                    tick={{ fill: "#cbd6ff", fontSize: 11 }}
                    tickFormatter={(value) => formatBucketLabel(value as string, chartBucket)}
                  />
                  <YAxis tick={{ fill: "#cbd6ff", fontSize: 11 }} />
                  <Tooltip
                    contentStyle={{ background: "#10142b", border: "1px solid #2f3c6d" }}
                    labelFormatter={(value) =>
                      formatBucketLabel(value as string, chartBucket)
                    }
                    formatter={(value) => formatCurrency(value as number)}
                  />
                  <Bar dataKey="value" fill="#ffb347" />
                </BarChart>
              </ResponsiveContainer>
            </div>
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>{chartBucketLabel === "hour" ? "Hour" : "Day"}</th>
                    <th>Cost</th>
                  </tr>
                </thead>
                <tbody>
                  {pagedCostSeries.map((point) => (
                    <tr key={point.bucket_start}>
                      <td>{formatBucketLabel(point.bucket_start, chartBucket)}</td>
                      <td>{formatCurrency(point.value)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {costSeries.length > COST_BREAKDOWN_PAGE_SIZE && (
              <div className="table-footer">
                <span className="note">
                  Showing{" "}
                  {costSeries.length === 0
                    ? 0
                    : (costSeriesPage - 1) * COST_BREAKDOWN_PAGE_SIZE + 1}
                  -
                  {Math.min(costSeriesPage * COST_BREAKDOWN_PAGE_SIZE, costSeries.length)} of{" "}
                  {costSeries.length} {chartBucketLabel === "hour" ? "hours" : "days"}
                </span>
                <div className="pagination">
                  <button
                    className="button ghost small"
                    onClick={() => setCostSeriesPage((page) => Math.max(1, page - 1))}
                    disabled={costSeriesPage === 1}
                  >
                    Previous
                  </button>
                  <span className="pagination-status">
                    Page {costSeriesPage} of {totalCostSeriesPages}
                  </span>
                  <button
                    className="button ghost small"
                    onClick={() =>
                      setCostSeriesPage((page) =>
                        Math.min(totalCostSeriesPages, page + 1)
                      )
                    }
                    disabled={costSeriesPage === totalCostSeriesPages}
                  >
                    Next
                  </button>
                </div>
              </div>
            )}
          </>
        )}
      </section>

      <section className="panel">
        <div className="panel-header">
          <div>
            <h2>Recent Events</h2>
            <p>Filtered by range and model.</p>
          </div>
          <div className="filters">
            <label className="label">Model</label>
            <select
              className="select-native"
              value={modelFilter}
              onChange={(event) => setModelFilter(event.target.value)}
            >
              {modelOptions.map((model) => (
                <option key={model} value={model}>
                  {model}
                </option>
              ))}
            </select>
          </div>
        </div>
        <div className="table-wrap events-table">
          <table>
            <thead>
              <tr>
                <th>Timestamp</th>
                <th>Model</th>
                <th>Effort</th>
                <th>Total Tokens</th>
                <th>Input</th>
                <th>Output</th>
                <th>Cost</th>
              </tr>
            </thead>
            <tbody>
              {pagedEvents.map((event) => (
                <tr key={event.id}>
                  <td>{new Date(event.ts).toLocaleString()}</td>
                  <td>{event.model}</td>
                  <td>{formatEffort(event.reasoning_effort)}</td>
                  <td>{formatNumber(event.usage.total_tokens)}</td>
                  <td>{formatNumber(event.usage.input_tokens)}</td>
                  <td>{formatNumber(event.usage.output_tokens)}</td>
                  <td>{formatCurrency(event.cost_usd)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="table-footer">
          <span className="note">
            Showing {events.length === 0 ? 0 : (eventsPage - 1) * EVENTS_PER_PAGE + 1}-
            {Math.min(eventsPage * EVENTS_PER_PAGE, events.length)} of {events.length}
          </span>
          <div className="pagination">
            <button
              className="button ghost small"
              onClick={() => setEventsPage((page) => Math.max(1, page - 1))}
              disabled={eventsPage === 1}
            >
              Previous
            </button>
            <span className="pagination-status">
              Page {eventsPage} of {totalEventPages}
            </span>
            <button
              className="button ghost small"
              onClick={() => setEventsPage((page) => Math.min(totalEventPages, page + 1))}
              disabled={eventsPage === totalEventPages}
            >
              Next
            </button>
          </div>
        </div>
      </section>

        </>
      )}
    </div>
  );
}
