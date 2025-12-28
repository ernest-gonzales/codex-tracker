import { useEffect, useMemo, useRef, useState } from "react";
import type { ChangeEvent, KeyboardEvent } from "react";
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
import { SelectField, SelectOption } from "./components/Select";
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
  openLogsDir,
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

const resolvedLocale = typeof navigator !== "undefined" ? navigator.language : "en-US";
const currency = new Intl.NumberFormat(resolvedLocale, {
  style: "currency",
  currency: "USD",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2
});
const numberFormat = new Intl.NumberFormat(resolvedLocale);
const compactNumberFormat = new Intl.NumberFormat(resolvedLocale, {
  notation: "compact",
  compactDisplay: "short",
  maximumFractionDigits: 1
});
const dateTimeFormat = new Intl.DateTimeFormat(resolvedLocale, {
  month: "short",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit"
});
const dateFormat = new Intl.DateTimeFormat(resolvedLocale, {
  year: "numeric",
  month: "short",
  day: "2-digit"
});
const hourFormat = new Intl.DateTimeFormat(resolvedLocale, {
  hour: "2-digit",
  minute: "2-digit"
});
const EVENTS_PER_PAGE = 10;
const COST_BREAKDOWN_PAGE_SIZE = 10;
const STORAGE_KEYS = {
  range: "codex-tracker.range",
  rangeStart: "codex-tracker.range.start",
  rangeEnd: "codex-tracker.range.end",
  settingsTab: "codex-tracker.settings.tab"
};

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

function formatLimitPercentLeft(value: number | null | undefined) {
  if (value === null || value === undefined) return "100%";
  if (value === 0) return "100%";
  return formatPercentWhole(value);
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

function formatDateTime(value: string | null | undefined) {
  if (!value) return "-";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return dateTimeFormat.format(parsed);
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

type PricingIssue = {
  index: number;
  message: string;
  field?: "model_pattern" | "input_per_1m" | "cached_input_per_1m" | "output_per_1m" | "range";
};

function validatePricingRules(rules: PricingRule[]): PricingIssue[] {
  const issues: PricingIssue[] = [];
  const rangesByModel = new Map<
    string,
    Array<{ index: number; start: number; end: number | null }>
  >();

  rules.forEach((rule, index) => {
    if (!rule.model_pattern.trim()) {
      issues.push({ index, message: "Model pattern is required.", field: "model_pattern" });
    }
    if (rule.input_per_1m < 0) {
      issues.push({ index, message: "Input price must be zero or higher.", field: "input_per_1m" });
    }
    if (rule.cached_input_per_1m < 0) {
      issues.push({
        index,
        message: "Cached input price must be zero or higher.",
        field: "cached_input_per_1m"
      });
    }
    if (rule.output_per_1m < 0) {
      issues.push({
        index,
        message: "Output price must be zero or higher.",
        field: "output_per_1m"
      });
    }
    const start = new Date(rule.effective_from).getTime();
    const end = rule.effective_to ? new Date(rule.effective_to).getTime() : null;
    if (!Number.isNaN(start) && end !== null && !Number.isNaN(end) && end < start) {
      issues.push({
        index,
        message: "Effective end must be after the start date.",
        field: "range"
      });
    }
    const list = rangesByModel.get(rule.model_pattern) ?? [];
    list.push({ index, start, end });
    rangesByModel.set(rule.model_pattern, list);
  });

  rangesByModel.forEach((ranges) => {
    ranges.sort((a, b) => (a.start || 0) - (b.start || 0));
    for (let i = 0; i < ranges.length; i += 1) {
      const current = ranges[i];
      if (Number.isNaN(current.start)) {
        continue;
      }
      for (let j = i + 1; j < ranges.length; j += 1) {
        const next = ranges[j];
        if (Number.isNaN(next.start)) {
          continue;
        }
        const currentEnd = current.end ?? Number.POSITIVE_INFINITY;
        const nextEnd = next.end ?? Number.POSITIVE_INFINITY;
        const overlaps = current.start <= nextEnd && next.start <= currentEnd;
        if (overlaps) {
          issues.push({
            index: current.index,
            message: "Overlapping effective ranges for this model pattern.",
            field: "range"
          });
          issues.push({
            index: next.index,
            message: "Overlapping effective ranges for this model pattern.",
            field: "range"
          });
        }
      }
    }
  });

  return issues;
}

function formatDateOnlyLocal(value?: string | null) {
  if (!value) return "";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  const offset = parsed.getTimezoneOffset() * 60 * 1000;
  return new Date(parsed.getTime() - offset).toISOString().slice(0, 10);
}

function parseDateOnlyLocal(value: string) {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return parsed.toISOString();
}

function parseDateInput(value: string) {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }
  if (trimmed.includes("T")) {
    const parsed = new Date(trimmed);
    if (!Number.isNaN(parsed.getTime())) {
      return parsed;
    }
  }
  const parts = trimmed.split("-");
  if (parts.length === 3) {
    const [year, month, day] = parts.map((part) => Number(part));
    if ([year, month, day].every((part) => Number.isFinite(part))) {
      return new Date(year, month - 1, day);
    }
  }
  const parsed = new Date(trimmed);
  if (!Number.isNaN(parsed.getTime())) {
    return parsed;
  }
  return null;
}

function formatDateInputValue(value?: string | null) {
  if (!value) return "";
  const parsed = parseDateInput(value);
  if (!parsed) {
    return value;
  }
  const year = String(parsed.getFullYear());
  const month = String(parsed.getMonth() + 1).padStart(2, "0");
  const day = String(parsed.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function toRangeStart(value?: string) {
  if (!value) return undefined;
  const parsed = parseDateInput(value);
  if (!parsed) return undefined;
  const start = new Date(parsed.getFullYear(), parsed.getMonth(), parsed.getDate(), 0, 0, 0, 0);
  return start.toISOString();
}

function toRangeEndExclusive(value?: string) {
  if (!value) return undefined;
  const parsed = parseDateInput(value);
  if (!parsed) return undefined;
  const end = new Date(parsed.getFullYear(), parsed.getMonth(), parsed.getDate(), 0, 0, 0, 0);
  // Shift to next-day start so the selected end date is treated as inclusive.
  end.setDate(end.getDate() + 1);
  return end.toISOString();
}

function buildRangeParams(range: RangeValue, start?: string, end?: string) {
  if (range === "custom") {
    return {
      start: toRangeStart(start),
      end: toRangeEndExclusive(end)
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

function safeStorageGet(key: string) {
  if (typeof window === "undefined") {
    return null;
  }
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function safeStorageSet(key: string, value: string) {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Ignore storage write failures (private mode or restricted storage).
  }
}

function clampPercent(value: number | null | undefined) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return 0;
  }
  return Math.min(100, Math.max(0, value));
}

function isEditableTarget(target: EventTarget | null) {
  const element = target as HTMLElement | null;
  if (!element) {
    return false;
  }
  const tagName = element.tagName?.toLowerCase();
  return tagName === "input" || tagName === "textarea" || element.isContentEditable;
}

export default function App() {
  const [range, setRange] = useState<RangeValue>("today");
  const [customStart, setCustomStart] = useState<string>("");
  const [customEnd, setCustomEnd] = useState<string>("");
  const [isSettingsOpen, setIsSettingsOpen] = useState<boolean>(false);
  const [settingsTab, setSettingsTab] = useState<string>("settings-homes");
  const [summary, setSummary] = useState<UsageSummary | null>(null);
  const [activeSessions, setActiveSessions] = useState<ActiveSession[]>([]);
  const [activeMinutes, setActiveMinutes] = useState<number>(60);
  const [activeMinutesInput, setActiveMinutesInput] = useState<string>("60");
  const [contextStats, setContextStats] = useState<ContextPressureStats | null>(null);
  const [limits, setLimits] = useState<LimitsResponse | null>(null);
  const [limitWindows, setLimitWindows] = useState<UsageLimitWindow[]>([]);
  const [limitCurrent, setLimitCurrent] = useState<UsageLimitCurrentResponse | null>(null);
  const [showLimitDetails, setShowLimitDetails] = useState<boolean>(false);
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
  const [dangerStatus, setDangerStatus] = useState<string>("");
  const [deleteConfirm, setDeleteConfirm] = useState<string>("");
  const [pricingStatus, setPricingStatus] = useState<string>("");
  const [settingsStatus, setSettingsStatus] = useState<string>("");
  const [pricingDirty, setPricingDirty] = useState<boolean>(false);
  const [pricingFilter, setPricingFilter] = useState<string>("");
  const [pricingBusy, setPricingBusy] = useState<boolean>(false);
  const [pricingLastRecompute, setPricingLastRecompute] = useState<string | null>(null);
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
  const ingestInFlight = useRef<boolean>(false);
  const customStartRef = useRef<HTMLInputElement | null>(null);
  const customEndRef = useRef<HTMLInputElement | null>(null);
  const [costBreakdownTab, setCostBreakdownTab] = useState<"model" | "day">("model");
  const [expandedModels, setExpandedModels] = useState<Set<string>>(new Set());
  const [toast, setToast] = useState<{ message: string; tone?: "error" | "info" } | null>(
    null
  );
  const [selectedSession, setSelectedSession] = useState<ActiveSession | null>(null);

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
  const deleteReady = deleteConfirm.trim().toLowerCase() === "delete";
  const pricingIssues = useMemo(() => validatePricingRules(pricingRules), [pricingRules]);
  const pricingIssueMap = useMemo(() => {
    const map = new Map<number, PricingIssue[]>();
    pricingIssues.forEach((issue) => {
      const list = map.get(issue.index) ?? [];
      list.push(issue);
      map.set(issue.index, list);
    });
    return map;
  }, [pricingIssues]);
  const pricingHasIssues = pricingIssues.length > 0;
  const pricingRows = useMemo(() => {
    const rows = pricingRules.map((rule, index) => ({ rule, index }));
    if (!pricingFilter.trim()) {
      return rows;
    }
    const needle = pricingFilter.trim().toLowerCase();
    return rows.filter(({ rule }) => rule.model_pattern.toLowerCase().includes(needle));
  }, [pricingRules, pricingFilter]);

  const rangeOptions = useMemo<SelectOption[]>(
    () =>
      RANGE_OPTIONS.map((option) => ({
        value: option.value,
        label: option.label
      })),
    []
  );
  const autoRefreshOptions = useMemo<SelectOption[]>(
    () =>
      AUTO_REFRESH_OPTIONS.map((option) => ({
        value: option.value,
        label: option.label
      })),
    []
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
      setPricingDirty(false);
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

  useEffect(() => {
    const storedRange = safeStorageGet(STORAGE_KEYS.range);
    if (storedRange && RANGE_OPTIONS.some((option) => option.value === storedRange)) {
      setRange(storedRange as RangeValue);
    }
    const storedStart = safeStorageGet(STORAGE_KEYS.rangeStart);
    if (storedStart) {
      setCustomStart(formatDateInputValue(storedStart));
    }
    const storedEnd = safeStorageGet(STORAGE_KEYS.rangeEnd);
    if (storedEnd) {
      setCustomEnd(formatDateInputValue(storedEnd));
    }
    const storedTab = safeStorageGet(STORAGE_KEYS.settingsTab);
    if (
      storedTab &&
      ["settings-homes", "settings-display", "settings-storage", "settings-pricing"].includes(
        storedTab
      )
    ) {
      setSettingsTab(storedTab);
    }
  }, []);

  useEffect(() => {
    safeStorageSet(STORAGE_KEYS.range, range);
  }, [range]);

  useEffect(() => {
    safeStorageSet(STORAGE_KEYS.rangeStart, customStart);
  }, [customStart]);

  useEffect(() => {
    safeStorageSet(STORAGE_KEYS.rangeEnd, customEnd);
  }, [customEnd]);

  useEffect(() => {
    safeStorageSet(STORAGE_KEYS.settingsTab, settingsTab);
  }, [settingsTab]);

  useEffect(() => {
    if (!error) {
      return;
    }
    setToast({ message: error, tone: "error" });
  }, [error]);

  useEffect(() => {
    if (!toast) {
      return;
    }
    const timeout = window.setTimeout(() => setToast(null), 4500);
    return () => window.clearTimeout(timeout);
  }, [toast]);

  const autoRefreshInterval = useMemo(() => {
    return AUTO_REFRESH_OPTIONS.find((option) => option.value === autoRefresh)?.ms ?? 0;
  }, [autoRefresh]);

  const activeHome = useMemo(() => {
    if (activeHomeId === null) {
      return null;
    }
    return homes.find((home) => home.id === activeHomeId) ?? null;
  }, [homes, activeHomeId]);

  const homeSelectOptions = useMemo<SelectOption[]>(() => {
    if (homes.length === 0) {
      return [{ value: "none", label: "No homes found", disabled: true }];
    }
    return homes.map((home) => ({
      value: String(home.id),
      label: home.label || home.path
    }));
  }, [homes]);

  useEffect(() => {
    if (!autoRefreshInterval) {
      return;
    }
    const intervalId = window.setInterval(() => {
      handleIngest();
    }, autoRefreshInterval);
    return () => window.clearInterval(intervalId);
  }, [autoRefreshInterval, rangeParamsKey, modelFilter, activeMinutes, chartBucket]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        if (cancelled) {
          return;
        }
        unlisten = await listen<IngestStats>("ingest:complete", (event) => {
          if (event.payload) {
            setIngestStats(event.payload);
          }
          refreshAll();
        });
      } catch (err) {
        setToast({
          message: err instanceof Error ? err.message : "Ingest listener unavailable",
          tone: "info"
        });
      }
    })();
    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [rangeParamsKey, modelFilter, activeMinutes, chartBucket]);

  useEffect(() => {
    if (!isSettingsOpen) {
      return;
    }
    const target = document.getElementById(settingsTab);
    if (target) {
      target.scrollIntoView({ block: "start", behavior: "smooth" });
    }
  }, [isSettingsOpen, settingsTab]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.defaultPrevented) {
        return;
      }
      if (event.key === "Escape") {
        if (selectedSession) {
          event.preventDefault();
          setSelectedSession(null);
          return;
        }
        if (isSettingsOpen) {
          event.preventDefault();
          setIsSettingsOpen(false);
        }
        return;
      }
      if (isEditableTarget(event.target)) {
        return;
      }
      if (!event.metaKey) {
        return;
      }
      const key = event.key.toLowerCase();
      if (key === "r") {
        event.preventDefault();
        handleIngest();
      }
      if (key === "l") {
        event.preventDefault();
        handleOpenLogs();
      }
      if (key === ",") {
        event.preventDefault();
        setIsSettingsOpen(true);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    isSettingsOpen,
    selectedSession,
    rangeParamsKey,
    modelFilter,
    activeMinutes,
    chartBucket,
    activeHome?.path
  ]);

  const modelOptions = useMemo(() => {
    const models = new Set(breakdown.map((item) => item.model));
    return ["all", ...Array.from(models).sort()];
  }, [breakdown]);
  const modelSelectOptions = useMemo<SelectOption[]>(
    () =>
      modelOptions.map((model) => ({
        value: model,
        label: model === "all" ? "All models" : model
      })),
    [modelOptions]
  );
  const bucketOptions = useMemo<SelectOption[]>(
    () => [
      { value: "day", label: "Day" },
      { value: "hour", label: "Hour" }
    ],
    []
  );

  const uniformSessionModel = useMemo(() => {
    if (activeSessions.length === 0) {
      return null;
    }
    const model = activeSessions[0]?.model;
    if (!model) {
      return null;
    }
    return activeSessions.every((session) => session.model === model) ? model : null;
  }, [activeSessions]);
  const showSessionModel = uniformSessionModel === null;

  const isRefreshing = loading || isIngesting;
  const showSummarySkeleton = loading && !summary;

  const costChartData = breakdown.map((item) => ({
    model: item.model,
    input: item.input_cost_usd ?? 0,
    cached: item.cached_input_cost_usd ?? 0,
    output: item.output_cost_usd ?? 0
  }));
  const tokensSeriesEmpty = tokensSeries.length === 0;
  const costSeriesEmpty = costSeries.length === 0;
  const costChartEmpty = costChartData.length === 0;

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
  const ingestStatus = ingestStats
    ? `Last scan: ${formatNumber(ingestStats.files_scanned)} files · +${formatNumber(
        ingestStats.events_inserted
      )} events`
    : "Last scan: —";
  const primaryLimitPercent = clampPercent(limits?.primary?.percent_left ?? 100);
  const secondaryLimitPercent = clampPercent(limits?.secondary?.percent_left ?? 100);
  const limitWindowRows = useMemo(() => {
    if (!limitWindows.length) {
      return [];
    }
    const ordered = [...limitWindows].sort((a, b) => {
      const aEnd = new Date(a.window_end).getTime();
      const bEnd = new Date(b.window_end).getTime();
      return aEnd - bEnd;
    });
    let previous: UsageLimitWindow | null = null;
    const withDelta = ordered.map((window) => {
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
    return withDelta.reverse();
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

  async function validateHomePath(path: string) {
    try {
      const { exists } = await import("@tauri-apps/plugin-fs");
      return await exists(path);
    } catch (err) {
      setToast({
        message: err instanceof Error ? err.message : "Path validation unavailable",
        tone: "info"
      });
      return true;
    }
  }

  async function handlePickHomePath() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected === "string") {
        setNewHomePath(selected);
      }
    } catch (err) {
      setHomeStatus(err instanceof Error ? err.message : "Picker failed");
    }
  }

  async function handleAddHome() {
    const path = newHomePath.trim();
    if (!path) {
      setHomeStatus("Path required");
      return;
    }
    const exists = await validateHomePath(path);
    if (!exists) {
      setHomeStatus("Path does not exist");
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
    if (pricingHasIssues) {
      setPricingStatus("Fix validation issues before saving");
      return;
    }
    setPricingStatus("Saving...");
    setPricingBusy(true);
    try {
      await replacePricing(pricingRules);
      setPricingStatus("Saved");
      setPricingDirty(false);
      await refreshAll();
    } catch (err) {
      setPricingStatus(err instanceof Error ? err.message : "Save failed");
    } finally {
      setPricingBusy(false);
    }
  }

  async function handleRecomputeCosts() {
    setPricingStatus("Recomputing...");
    setPricingBusy(true);
    try {
      const result = await recomputePricing();
      setPricingStatus(`Recomputed ${formatNumber(result.updated)} rows`);
      setPricingLastRecompute(new Date().toISOString());
      await refreshAll();
    } catch (err) {
      setPricingStatus(err instanceof Error ? err.message : "Recompute failed");
    } finally {
      setPricingBusy(false);
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
      setDangerStatus("Select a home first");
      return;
    }
    if (!deleteReady) {
      setDangerStatus('Type "DELETE" to confirm');
      return;
    }
    setDangerStatus("Deleting data...");
    try {
      await clearHomeData(activeHomeId);
      await refreshAll();
      setDangerStatus("Data deleted");
      setDeleteConfirm("");
    } catch (err) {
      setDangerStatus(err instanceof Error ? err.message : "Delete failed");
    }
  }

  async function handleCopyPath(value?: string) {
    if (!value) {
      return;
    }
    try {
      await navigator.clipboard.writeText(value);
      setToast({ message: "Path copied to clipboard", tone: "info" });
    } catch (err) {
      setToast({
        message: err instanceof Error ? err.message : "Copy failed",
        tone: "error"
      });
    }
  }

  async function handleRevealPath(value?: string, isDir = false) {
    if (!value) {
      return;
    }
    try {
      if (isDir) {
        const { openPath } = await import("@tauri-apps/plugin-opener");
        await openPath(value);
      } else {
        const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
        await revealItemInDir(value);
      }
    } catch (err) {
      setToast({
        message: err instanceof Error ? err.message : "Reveal failed",
        tone: "error"
      });
    }
  }

  async function handleOpenLogs() {
    try {
      await openLogsDir();
    } catch (err) {
      setToast({ message: err instanceof Error ? err.message : "Open logs failed", tone: "error" });
    }
  }

  function handleCustomStartChange(event: ChangeEvent<HTMLInputElement>) {
    const value = event.target.value;
    setCustomStart(value);
    if (value) {
      window.requestAnimationFrame(() => customEndRef.current?.focus());
    }
  }

  function handleCustomEndChange(event: ChangeEvent<HTMLInputElement>) {
    const value = event.target.value;
    setCustomEnd(value);
    if (value) {
      const target = event.currentTarget;
      window.requestAnimationFrame(() => target.blur());
    }
  }

  function handleDateInputKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Escape") {
      event.currentTarget.blur();
    }
  }

  async function handleCopySessionId(value: string) {
    try {
      await navigator.clipboard.writeText(value);
      setToast({ message: "Session id copied", tone: "info" });
    } catch (err) {
      setToast({
        message: err instanceof Error ? err.message : "Copy failed",
        tone: "error"
      });
    }
  }

  function updatePricingRule(index: number, patch: Partial<PricingRule>) {
    setPricingRules((prev) =>
      prev.map((rule, idx) => (idx === index ? { ...rule, ...patch } : rule))
    );
    setPricingDirty(true);
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
    setPricingDirty(true);
  }

  function duplicatePricingRule(index: number) {
    setPricingRules((prev) => {
      const rule = prev[index];
      if (!rule) {
        return prev;
      }
      const clone = { ...rule, id: undefined };
      const next = [...prev];
      next.splice(index + 1, 0, clone);
      return next;
    });
    setPricingDirty(true);
  }

  function deletePricingRule(index: number) {
    setPricingRules((prev) => prev.filter((_, idx) => idx !== index));
    setPricingDirty(true);
  }

  return (
    <div className="app density-compact">
      <div className="glow" aria-hidden="true" />
      {toast && (
        <div
          className={`toast ${toast.tone === "error" ? "toast-error" : "toast-info"}`}
          role="status"
          aria-live="polite"
        >
          <span>{toast.message}</span>
          <button
            className="icon-button toast-close"
            type="button"
            onClick={() => setToast(null)}
            aria-label="Dismiss notification"
          >
            ×
          </button>
        </div>
      )}
      {selectedSession && (
        <div
          className="modal-overlay"
          role="presentation"
          onClick={() => setSelectedSession(null)}
        >
          <div
            className="modal session-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="session-detail-title"
            onClick={(event) => event.stopPropagation()}
          >
            <header className="modal-header">
              <div>
                <h2 className="modal-title" id="session-detail-title">
                  Session {formatSessionLabel(selectedSession.session_id)}
                </h2>
                <p className="modal-subtitle">Model {selectedSession.model}</p>
              </div>
              <button
                className="icon-button"
                type="button"
                onClick={() => setSelectedSession(null)}
                aria-label="Close session details"
              >
                ×
              </button>
            </header>
            <div className="modal-body">
              <div className="session-details-grid">
                <div>
                  <span className="label">Session ID</span>
                  <div className="session-detail-id">
                    <span className="mono">{selectedSession.session_id}</span>
                    <button
                      className="icon-button icon-button-ghost small"
                      type="button"
                      onClick={() => handleCopySessionId(selectedSession.session_id)}
                      aria-label="Copy full session id"
                      title="Copy session id"
                    >
                      <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
                        <path
                          d="M16 1H6a2 2 0 0 0-2 2v12h2V3h10V1z"
                          fill="currentColor"
                          opacity="0.6"
                        />
                        <path
                          d="M18 5H10a2 2 0 0 0-2 2v14h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2z"
                          fill="currentColor"
                        />
                      </svg>
                    </button>
                  </div>
                </div>
                <div>
                  <span className="label">Started</span>
                  <div className="session-detail-value">
                    {formatDateTime(selectedSession.session_start)}
                  </div>
                </div>
                <div>
                  <span className="label">Last Seen</span>
                  <div className="session-detail-value">
                    {formatDateTime(selectedSession.last_seen)}
                  </div>
                </div>
                <div>
                  <span className="label">Context Used</span>
                  <div className="session-detail-value tabular-nums">
                    {formatNumber(selectedSession.context_used)}
                  </div>
                </div>
                <div>
                  <span className="label">Context Window</span>
                  <div className="session-detail-value tabular-nums">
                    {formatNumber(selectedSession.context_window)}
                  </div>
                </div>
                <div>
                  <span className="label">Pressure</span>
                  <div className="session-detail-value tabular-nums">
                    {Math.round(
                      selectedSession.context_window > 0
                        ? (selectedSession.context_used / selectedSession.context_window) * 100
                        : 0
                    )}
                    %
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
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
          <div className="settings-layout">
            <nav className="settings-nav" aria-label="Settings sections">
              <a
                href="#settings-homes"
                className={settingsTab === "settings-homes" ? "active" : undefined}
                aria-current={settingsTab === "settings-homes" ? "page" : undefined}
                onClick={() => setSettingsTab("settings-homes")}
              >
                Homes
              </a>
              <a
                href="#settings-display"
                className={settingsTab === "settings-display" ? "active" : undefined}
                aria-current={settingsTab === "settings-display" ? "page" : undefined}
                onClick={() => setSettingsTab("settings-display")}
              >
                Display
              </a>
              <a
                href="#settings-storage"
                className={settingsTab === "settings-storage" ? "active" : undefined}
                aria-current={settingsTab === "settings-storage" ? "page" : undefined}
                onClick={() => setSettingsTab("settings-storage")}
              >
                Storage
              </a>
              <a
                href="#settings-pricing"
                className={settingsTab === "settings-pricing" ? "active" : undefined}
                aria-current={settingsTab === "settings-pricing" ? "page" : undefined}
                onClick={() => setSettingsTab("settings-pricing")}
              >
                Pricing
              </a>
            </nav>
            <div className="settings-content">
              <section id="settings-homes" className="panel settings-section">
                <div className="panel-header">
                  <div>
                    <h2>Codex Homes</h2>
                    <p>Switch between tracked log directories.</p>
                  </div>
                  <div className="panel-actions">
                    <button className="button ghost small" onClick={refreshHomes}>
                      Reload
                    </button>
                  </div>
                </div>
                <label className="label">Active Home</label>
                <SelectField
                  value={activeHomeId ? String(activeHomeId) : undefined}
                  onValueChange={(value) => handleSetActiveHome(Number(value))}
                  options={homeSelectOptions}
                  placeholder="Select a home"
                  disabled={homes.length === 0}
                />
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
                          <th>Status</th>
                          <th>Actions</th>
                        </tr>
                      </thead>
                      <tbody>
                        {homes.map((home) => (
                          <tr
                            key={home.id}
                            className={home.id === activeHomeId ? "active-row" : undefined}
                          >
                            <td>{home.label || "—"}</td>
                            <td>
                              <span className="mono">{home.path}</span>
                            </td>
                            <td>
                              {home.last_seen_at
                                ? new Date(home.last_seen_at).toLocaleString()
                                : "-"}
                            </td>
                            <td>
                              {home.id === activeHomeId ? (
                                <span className="badge">Active</span>
                              ) : (
                                "—"
                              )}
                            </td>
                            <td className="table-actions">
                              {home.id !== activeHomeId && (
                                <button
                                  className="button ghost small"
                                  onClick={() => handleSetActiveHome(home.id)}
                                >
                                  Make Active
                                </button>
                              )}
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
                <div className="danger-zone">
                  <div className="danger-zone-header">
                    <div>
                      <h3>Danger Zone</h3>
                      <p>Delete all ingested data for the active home.</p>
                    </div>
                  </div>
                  <label className="label">Type DELETE to confirm</label>
                  <input
                    className="input"
                    value={deleteConfirm}
                    onChange={(event) => setDeleteConfirm(event.target.value)}
                    placeholder="DELETE"
                  />
                  <div className="row">
                    <button
                      className="button danger"
                      onClick={handleDeleteData}
                      disabled={!activeHomeId || !deleteReady}
                    >
                      Delete Ingested Data
                    </button>
                    <span className="status" role="status" aria-live="polite">
                      {dangerStatus}
                    </span>
                  </div>
                </div>
                <label className="label">Add Home</label>
                <div className="input-row">
                  <input
                    className="input"
                    value={newHomePath}
                    onChange={(event) => setNewHomePath(event.target.value)}
                    placeholder="/Users/you/.codex"
                  />
                  <button className="button ghost small" type="button" onClick={handlePickHomePath}>
                    Browse
                  </button>
                </div>
                <input
                  className="input"
                  value={newHomeLabel}
                  onChange={(event) => setNewHomeLabel(event.target.value)}
                  placeholder="Label (optional)"
                />
                <div className="row">
                  <button className="button" onClick={handleAddHome} disabled={!newHomePath.trim()}>
                    Add Home
                  </button>
                  <span className="status" role="status" aria-live="polite">
                    {homeStatus}
                  </span>
                </div>
              </section>

              <section id="settings-display" className="panel settings-section">
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
                  <span className="status" role="status" aria-live="polite">
                    {settingsStatus}
                  </span>
                </div>
                <div className="note">Updates the Active Sessions panel and refresh cycle.</div>
              </section>

              <section id="settings-storage" className="panel settings-section">
                <div className="panel-header">
                  <div>
                    <h2>Storage</h2>
                    <p>Local app data directory for the desktop client.</p>
                  </div>
                </div>
                <div className="settings-kv">
                  <div className="settings-kv-row">
                    <span className="settings-kv-key">App Data</span>
                    <div className="settings-kv-value">
                      <span className="mono">{storageInfo?.appDataDir ?? "—"}</span>
                      <div className="kv-actions">
                        <button
                          className="button ghost small"
                          type="button"
                          onClick={() => handleCopyPath(storageInfo?.appDataDir)}
                          disabled={!storageInfo?.appDataDir}
                        >
                          Copy
                        </button>
                        <button
                          className="button ghost small"
                          type="button"
                          onClick={() => handleRevealPath(storageInfo?.appDataDir, true)}
                          disabled={!storageInfo?.appDataDir}
                        >
                          Reveal
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
                <div className="note">
                  Desktop builds keep all data in this directory.
                </div>
              </section>

              <section id="settings-pricing" className="panel settings-pricing settings-section">
                <div className="panel-header">
                  <div>
                    <h2>Pricing Rules</h2>
                    <p>Override model pricing and recompute stored costs.</p>
                  </div>
                </div>
                <div className="pricing-toolbar">
                  <div className="pricing-filter">
                    <label className="label">Filter</label>
                    <input
                      className="input select-inline"
                      value={pricingFilter}
                      onChange={(event) => setPricingFilter(event.target.value)}
                      placeholder="Filter by model pattern"
                    />
                  </div>
                  <div className="pricing-actions">
                    <button className="button ghost" onClick={addPricingRule} disabled={pricingBusy}>
                      Add Rule
                    </button>
                    <button
                      className="button"
                      onClick={handleSavePricing}
                      disabled={pricingBusy || pricingHasIssues || !pricingDirty}
                    >
                      Save Pricing
                    </button>
                    <button
                      className="button ghost"
                      onClick={handleRecomputeCosts}
                      disabled={pricingBusy}
                    >
                      Recompute Costs
                    </button>
                  </div>
                  <div className="pricing-meta">
                    <span className={`status ${pricingDirty ? "status-warn" : ""}`}>
                      {pricingDirty ? "Unsaved changes" : "All changes saved"}
                    </span>
                    {pricingStatus && (
                      <span className="status" role="status" aria-live="polite">
                        {pricingStatus}
                      </span>
                    )}
                    {pricingLastRecompute && (
                      <span className="note">
                        Last recompute {formatDateTime(pricingLastRecompute)}
                      </span>
                    )}
                  </div>
                </div>
                {pricingHasIssues && (
                  <div className="note error-note">
                    Resolve validation issues before saving pricing rules.
                  </div>
                )}
                {pricingRows.length === 0 ? (
                  <div className="note">No pricing rules match this filter.</div>
                ) : (
                  <div className="table-wrap">
                    <table className="pricing-table">
                      <thead>
                        <tr>
                          <th className="pricing-model-col">Model Pattern</th>
                          <th>Input / 1M</th>
                          <th>Cached / 1M</th>
                          <th>Output / 1M</th>
                          <th>Effective From</th>
                          <th>Effective To</th>
                          <th>Actions</th>
                        </tr>
                      </thead>
                      <tbody>
                        {pricingRows.map(({ rule, index }) => {
                          const effectiveToValue = formatDateOnlyLocal(rule.effective_to);
                          const hasEffectiveTo = Boolean(effectiveToValue);
                          const rowIssues = pricingIssueMap.get(index) ?? [];
                          const hasModelError = rowIssues.some(
                            (issue) => issue.field === "model_pattern"
                          );
                          const hasInputError = rowIssues.some(
                            (issue) => issue.field === "input_per_1m"
                          );
                          const hasCachedError = rowIssues.some(
                            (issue) => issue.field === "cached_input_per_1m"
                          );
                          const hasOutputError = rowIssues.some(
                            (issue) => issue.field === "output_per_1m"
                          );
                          const hasRangeError = rowIssues.some(
                            (issue) => issue.field === "range"
                          );
                          return (
                            <tr key={rule.id ?? `${rule.model_pattern}-${index}`}>
                              <td className="pricing-model-cell">
                                <input
                                  className={`input pricing-model ${hasModelError ? "input-error" : ""}`}
                                  value={rule.model_pattern}
                                  onChange={(event) =>
                                    updatePricingRule(index, { model_pattern: event.target.value })
                                  }
                                />
                              </td>
                              <td>
                                <input
                                  className={`input ${hasInputError ? "input-error" : ""}`}
                                  type="number"
                                  step="0.0001"
                                  inputMode="decimal"
                                  value={rule.input_per_1m}
                                  onChange={(event) =>
                                    updatePricingRule(index, {
                                      input_per_1m: Number(event.target.value)
                                    })
                                  }
                                />
                                <div className="input-hint currency-hint tabular-nums">
                                  {formatCurrency(rule.input_per_1m)}
                                </div>
                              </td>
                              <td>
                                <input
                                  className={`input ${hasCachedError ? "input-error" : ""}`}
                                  type="number"
                                  step="0.0001"
                                  inputMode="decimal"
                                  value={rule.cached_input_per_1m}
                                  onChange={(event) =>
                                    updatePricingRule(index, {
                                      cached_input_per_1m: Number(event.target.value)
                                    })
                                  }
                                />
                                <div className="input-hint currency-hint tabular-nums">
                                  {formatCurrency(rule.cached_input_per_1m)}
                                </div>
                              </td>
                              <td>
                                <input
                                  className={`input ${hasOutputError ? "input-error" : ""}`}
                                  type="number"
                                  step="0.0001"
                                  inputMode="decimal"
                                  value={rule.output_per_1m}
                                  onChange={(event) =>
                                    updatePricingRule(index, {
                                      output_per_1m: Number(event.target.value)
                                    })
                                  }
                                />
                                <div className="input-hint currency-hint tabular-nums">
                                  {formatCurrency(rule.output_per_1m)}
                                </div>
                              </td>
                              <td>
                                <input
                                  className={`input ${hasRangeError ? "input-error" : ""}`}
                                  type="date"
                                  value={formatDateOnlyLocal(rule.effective_from)}
                                  onChange={(event) =>
                                    updatePricingRule(index, {
                                      effective_from: parseDateOnlyLocal(event.target.value)
                                    })
                                  }
                                />
                              </td>
                              <td>
                                <div className="input-wrap">
                                  <input
                                    className={`input ${hasRangeError ? "input-error" : ""} ${
                                      hasEffectiveTo ? "" : "input-empty"
                                    }`}
                                    type="date"
                                    value={effectiveToValue}
                                    onChange={(event) =>
                                      updatePricingRule(index, {
                                        effective_to: event.target.value
                                          ? parseDateOnlyLocal(event.target.value)
                                          : null
                                      })
                                    }
                                  />
                                  {!hasEffectiveTo && (
                                    <span className="input-overlay">No end date</span>
                                  )}
                                </div>
                              </td>
                              <td className="pricing-actions-cell">
                                <div className="table-actions table-actions-compact">
                                  <button
                                    className="icon-button icon-button-ghost small"
                                    type="button"
                                    onClick={() => duplicatePricingRule(index)}
                                    aria-label="Duplicate pricing rule"
                                    title="Duplicate"
                                  >
                                    <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
                                      <path
                                        d="M16 1H6a2 2 0 0 0-2 2v12h2V3h10V1z"
                                        fill="currentColor"
                                        opacity="0.6"
                                      />
                                      <path
                                        d="M18 5H10a2 2 0 0 0-2 2v14h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2z"
                                        fill="currentColor"
                                      />
                                    </svg>
                                  </button>
                                  <button
                                    className="icon-button icon-button-ghost small"
                                    type="button"
                                    onClick={() => deletePricingRule(index)}
                                    aria-label="Delete pricing rule"
                                    title="Delete"
                                  >
                                    <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
                                      <path
                                        d="M9 3h6l1 2h4v2H4V5h4l1-2z"
                                        fill="currentColor"
                                        opacity="0.6"
                                      />
                                      <path
                                        d="M6 7h12l-1 13a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L6 7z"
                                        fill="currentColor"
                                      />
                                    </svg>
                                  </button>
                                </div>
                                {rowIssues[0] && (
                                  <div className="input-hint error-note">
                                    {rowIssues[0].message}
                                  </div>
                                )}
                              </td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  </div>
                )}
              </section>
            </div>
          </div>
        </section>
      ) : (
        <>
          <header className="hero">
            <div className="hero-top">
              <div className="hero-copy">
                <div className="brand">
                  <div className="brand-mark" aria-hidden="true">
                    <CodexTrackerLogo />
                  </div>
                  <div className="brand-text">
                    <h1 className="brand-name">
                      Codex <span>Tracker</span>
                    </h1>
                    <div className="brand-subtitle">
                      <span>Local usage intelligence for Codex.</span>
                      <button
                        className="info-icon"
                        type="button"
                        data-tooltip="Monitor tokens, cost, and context pressure across models with fast refreshes."
                        aria-label="About Codex Tracker"
                      >
                        i
                      </button>
                    </div>
                  </div>
                </div>
              </div>
              <div className="range-toolbar">
                <div className="range-toolbar-row range-toolbar-row-range">
                  <div className="range-group">
                    <span className="label">Range</span>
                    <SelectField
                      value={range}
                      onValueChange={(value) => setRange(value as RangeValue)}
                      options={rangeOptions}
                      size="compact"
                      className="range-select"
                      ariaLabel="Range"
                    />
                  </div>
                  <div
                    className={`custom-range custom-range-toolbar ${
                      range === "custom" ? "is-active" : "is-hidden"
                    }`}
                    data-active={range === "custom"}
                    aria-hidden={range !== "custom"}
                  >
                    <div className="date-field">
                      <input
                        ref={customStartRef}
                        type="date"
                        value={customStart}
                        onChange={handleCustomStartChange}
                        onKeyDown={handleDateInputKeyDown}
                        className={`input input-compact ${customStart ? "" : "input-empty"}`}
                        aria-label="Start date"
                        disabled={range !== "custom"}
                      />
                      {!customStart && <span className="date-placeholder">Start date</span>}
                    </div>
                    <span className="range-divider" aria-hidden="true">
                      -
                    </span>
                    <div className="date-field">
                      <input
                        ref={customEndRef}
                        type="date"
                        value={customEnd}
                        onChange={handleCustomEndChange}
                        onKeyDown={handleDateInputKeyDown}
                        className={`input input-compact ${customEnd ? "" : "input-empty"}`}
                        aria-label="End date"
                        disabled={range !== "custom"}
                      />
                      {!customEnd && <span className="date-placeholder">End date</span>}
                    </div>
                  </div>
                </div>
                <div className="range-toolbar-row range-toolbar-row-refresh">
                  <div className="range-group">
                    <span className="label">Auto refresh</span>
                    <SelectField
                      value={autoRefresh}
                      onValueChange={(value) => setAutoRefresh(value as AutoRefreshValue)}
                      options={autoRefreshOptions}
                      size="compact"
                      className="refresh-select"
                      ariaLabel="Auto refresh"
                    />
                  </div>
                </div>
                <div className="range-toolbar-actions range-toolbar-row range-toolbar-row-actions">
                  <button
                    className="icon-button small"
                    onClick={handleIngest}
                    disabled={isRefreshing}
                    title="Refresh (Cmd+R)"
                    aria-label="Refresh"
                  >
                    {isRefreshing ? (
                      <span className="spinner" aria-hidden="true" />
                    ) : (
                      <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
                        <path
                          d="M20 12a8 8 0 1 1-2.3-5.7"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth="1.8"
                          strokeLinecap="round"
                        />
                        <path
                          d="M20 5v5h-5"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth="1.8"
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        />
                      </svg>
                    )}
                  </button>
                  <button
                    className="icon-button icon-button-ghost small"
                    type="button"
                    onClick={handleOpenLogs}
                    title="Open logs (Cmd+L)"
                    aria-label="Open logs"
                  >
                    <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
                      <path
                        d="M4 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="1.6"
                        strokeLinejoin="round"
                      />
                    </svg>
                  </button>
                  <button
                    className="icon-button small"
                    type="button"
                    onClick={() => setIsSettingsOpen(true)}
                    aria-label="Open settings"
                    title="Settings"
                  >
                    <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
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
                <div className="range-status" title={ingestStatus}>
                  {ingestStatus}
                </div>
                {error && <p className="error">{error}</p>}
              </div>
            </div>
            <section className="grid summary-grid hero-metrics">
              <div className="card kpi-card">
                <p className="card-label">Total Tokens</p>
                <div className="card-value-row">
                  <p className="card-value tabular-nums">
                    {showSummarySkeleton ? (
                      <span className="skeleton-line skeleton-line-lg" />
                    ) : (
                      formatNumber(summary?.total_tokens)
                    )}
                  </p>
                  <p className="card-meta-inline tabular-nums">
                    {showSummarySkeleton ? (
                      <span className="skeleton-line skeleton-line-sm" />
                    ) : (
                      <>
                        <span>Input {formatNumber(summary?.input_tokens)}</span>
                        <span className="meta-sep">•</span>
                        <span>Output {formatNumber(summary?.output_tokens)}</span>
                      </>
                    )}
                  </p>
                </div>
              </div>
              <div className="card kpi-card">
                <p className="card-label">Total Cost</p>
                <div className="card-value-row">
                  <p className="card-value tabular-nums">
                    {showSummarySkeleton ? (
                      <span className="skeleton-line skeleton-line-lg" />
                    ) : (
                      formatCurrency(summary?.total_cost_usd)
                    )}
                  </p>
                  <p className="card-meta-inline tabular-nums">
                    {showSummarySkeleton ? (
                      <span className="skeleton-line skeleton-line-sm" />
                    ) : (
                      <>
                        <span>Input {formatCurrency(summary?.input_cost_usd)}</span>
                        <span className="meta-sep">•</span>
                        <span>Output {formatCurrency(summary?.output_cost_usd)}</span>
                      </>
                    )}
                  </p>
                </div>
              </div>
              <div className="card kpi-card">
                <p className="card-label">Cached Input</p>
                <div className="card-value-row">
                  <p className="card-value tabular-nums">
                    {showSummarySkeleton ? (
                      <span className="skeleton-line skeleton-line-lg" />
                    ) : (
                      formatNumber(summary?.cached_input_tokens)
                    )}
                  </p>
                  <p className="card-meta-inline tabular-nums">
                    {showSummarySkeleton ? (
                      <span className="skeleton-line skeleton-line-sm" />
                    ) : (
                      <span>Cost {formatCurrency(summary?.cached_input_cost_usd)}</span>
                    )}
                  </p>
                </div>
              </div>
              <div className="card kpi-card">
                <p className="card-label">Output Tokens</p>
                <div className="card-value-row">
                  <p className="card-value tabular-nums">
                    {showSummarySkeleton ? (
                      <span className="skeleton-line skeleton-line-lg" />
                    ) : (
                      formatNumber(summary?.output_tokens)
                    )}
                  </p>
                  <p className="card-meta-inline tabular-nums">
                    {showSummarySkeleton ? (
                      <span className="skeleton-line skeleton-line-sm" />
                    ) : (
                      <>
                        <span>Reasoning {formatNumber(summary?.reasoning_output_tokens)}</span>
                        <span className="meta-sep">•</span>
                        <span>Cost {formatCurrency(summary?.output_cost_usd)}</span>
                      </>
                    )}
                  </p>
                </div>
              </div>
            </section>
    </header>

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
          <div className={`session-table ${showSessionModel ? "" : "session-table-compact"}`}>
            <div className="session-row session-row-head">
              <span>Session</span>
              {showSessionModel && <span>Model</span>}
              <span>Context</span>
              <span>Pressure</span>
            </div>
            {activeSessions.map((session) => {
              const percent =
                session.context_window > 0
                  ? Math.min(
                      100,
                      (session.context_used / session.context_window) * 100
                    )
                  : 0;
              const startedLabel = formatBucketLabel(session.session_start);
              const lastSeenLabel = formatBucketLabel(session.last_seen);
              return (
                <div
                  className="session-row"
                  key={session.session_id}
                  role="button"
                  tabIndex={0}
                  aria-label={`Open session ${session.session_id}`}
                  onClick={() => setSelectedSession(session)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      setSelectedSession(session);
                    }
                  }}
                >
                  <div className="session-cell session-cell-primary">
                    <div className="session-title-row">
                      <div className="session-id" title={session.session_id}>
                        {formatSessionLabel(session.session_id)}
                      </div>
                      {!showSessionModel && uniformSessionModel && (
                        <span className="session-model-badge">{uniformSessionModel}</span>
                      )}
                      <button
                        className="icon-button icon-button-ghost small session-copy"
                        type="button"
                        onClick={(event) => {
                          event.stopPropagation();
                          handleCopySessionId(session.session_id);
                        }}
                        aria-label="Copy session id"
                        title="Copy session id"
                      >
                        <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
                          <path
                            d="M16 1H6a2 2 0 0 0-2 2v12h2V3h10V1z"
                            fill="currentColor"
                            opacity="0.6"
                          />
                          <path
                            d="M18 5H10a2 2 0 0 0-2 2v14h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2z"
                            fill="currentColor"
                          />
                        </svg>
                      </button>
                    </div>
                    <div className="session-sub">
                      Started {startedLabel}
                      {!showSessionModel && ` • Last ${lastSeenLabel}`}
                    </div>
                  </div>
                  {showSessionModel && (
                    <div className="session-cell">
                      <div className="session-model">{session.model}</div>
                      <div className="session-sub">Last {lastSeenLabel}</div>
                    </div>
                  )}
                  <div className="session-cell session-cell-metrics">
                    <span className="session-percent">{Math.round(percent)}%</span>
                    <span className="session-meta">
                      {formatNumber(session.context_used)} /{" "}
                      {formatNumber(session.context_window)} tokens
                    </span>
                  </div>
                  <div className="session-cell session-cell-bar">
                    <div className="session-bar">
                      <div className="session-bar-fill" style={{ width: `${percent}%` }} />
                    </div>
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
          <div className="panel-actions">
            <button
              className="button ghost small"
              type="button"
              onClick={() => setShowLimitDetails((prev) => !prev)}
            >
              {showLimitDetails ? "Hide details" : "Show details"}
            </button>
            <span className="tag">Logs</span>
          </div>
        </div>
        <div className="limits-grid">
          <div className="limit-card">
            <p className="card-label">5h Remaining</p>
            <div className="limit-inline-row">
              <p className="limit-value tabular-nums">
                {formatLimitPercentLeft(limits?.primary?.percent_left)}
              </p>
              <p className="card-meta limit-inline-meta">
                Resets {formatResetLabel(limits?.primary?.reset_at)} ·{" "}
                {formatRelativeReset(limits?.primary?.reset_at)}
              </p>
            </div>
            <div className="limit-progress" aria-hidden="true">
              <div
                className="limit-progress-fill"
                style={{ width: `${primaryLimitPercent}%` }}
              />
            </div>
            <p className="card-meta card-meta-compact">
              <span className="limit-inline">Messages</span>{" "}
              {formatNumber(limitCurrent?.primary?.message_count)}
              <span className="meta-sep">•</span>
              <span className="limit-inline">Tokens</span>{" "}
              {formatNumber(limitCurrent?.primary?.total_tokens)}
              <span className="meta-sep">•</span>
              <span className="limit-inline">Cost</span>{" "}
              {formatCurrency(limitCurrent?.primary?.total_cost_usd)}
            </p>
          </div>
          <div className="limit-card">
            <p className="card-label">7d Remaining</p>
            <div className="limit-inline-row">
              <p className="limit-value tabular-nums">
                {formatLimitPercentLeft(limits?.secondary?.percent_left)}
              </p>
              <p className="card-meta limit-inline-meta">
                Resets {formatResetLabel(limits?.secondary?.reset_at)} ·{" "}
                {formatRelativeReset(limits?.secondary?.reset_at)}
              </p>
            </div>
            <div className="limit-progress" aria-hidden="true">
              <div
                className="limit-progress-fill"
                style={{ width: `${secondaryLimitPercent}%` }}
              />
            </div>
            <p className="card-meta card-meta-compact">
              <span className="limit-inline">Messages</span>{" "}
              {formatNumber(limitCurrent?.secondary?.message_count)}
              <span className="meta-sep">•</span>
              <span className="limit-inline">Tokens</span>{" "}
              {formatNumber(limitCurrent?.secondary?.total_tokens)}
              <span className="meta-sep">•</span>
              <span className="limit-inline">Cost</span>{" "}
              {formatCurrency(limitCurrent?.secondary?.total_cost_usd)}
            </p>
          </div>
        </div>
        {showLimitDetails &&
          (limitWindowRows.length === 0 ? (
            <p className="note">No 7-day reset windows captured yet.</p>
          ) : (
            <div className="limits-details">
              <div className="limits-table-scroll">
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
              </div>
            </div>
          ))}
      </section>

      <section className="panel chart-panel">
        <div className="panel-header">
          <div>
            <h2>Token + Cost Trends</h2>
            <p>Compare tokens and spend by {chartBucketLabel}.</p>
          </div>
          <div className="panel-actions">
            <label className="label">Bucket</label>
            <SelectField
              value={chartBucketMode}
              onValueChange={(value) => setChartBucketMode(value as ChartBucketMode)}
              options={bucketOptions}
              size="inline"
              ariaLabel="Bucket"
            />
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
              {tokensSeriesEmpty ? (
                <div className="chart-empty">
                  {loading ? "Loading token history..." : "No token activity in this range."}
                </div>
              ) : (
                <ResponsiveContainer width="100%" height={200}>
                  <LineChart data={tokensSeries}>
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--chart-grid)" />
                    <XAxis
                      dataKey="bucket_start"
                      tick={{ fill: "var(--chart-axis)", fontSize: 11 }}
                      tickFormatter={(value) => formatBucketLabel(value as string, chartBucket)}
                    />
                    <YAxis tick={{ fill: "var(--chart-axis)", fontSize: 11 }} />
                    <Tooltip
                      contentStyle={{
                        background: "var(--chart-tooltip-bg)",
                        border: "1px solid var(--border)"
                      }}
                      labelFormatter={(value) =>
                        formatBucketLabel(value as string, chartBucket)
                      }
                      formatter={(value) => [formatNumber(value as number), "Tokens"]}
                    />
                    <Line
                      type="monotone"
                      dataKey="value"
                      stroke="var(--chart-token)"
                      strokeWidth={2}
                      dot={false}
                    />
                  </LineChart>
                </ResponsiveContainer>
              )}
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
              {costSeriesEmpty ? (
                <div className="chart-empty">
                  {loading ? "Loading cost history..." : "No cost activity in this range."}
                </div>
              ) : (
                <ResponsiveContainer width="100%" height={200}>
                  <LineChart data={costSeries}>
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--chart-grid)" />
                    <XAxis
                      dataKey="bucket_start"
                      tick={{ fill: "var(--chart-axis)", fontSize: 11 }}
                      tickFormatter={(value) => formatBucketLabel(value as string, chartBucket)}
                    />
                    <YAxis tick={{ fill: "var(--chart-axis)", fontSize: 11 }} />
                    <Tooltip
                      contentStyle={{
                        background: "var(--chart-tooltip-bg)",
                        border: "1px solid var(--border)"
                      }}
                      labelFormatter={(value) =>
                        formatBucketLabel(value as string, chartBucket)
                      }
                      formatter={(value) => [formatCurrency(value as number), "Cost"]}
                    />
                    <Line
                      type="monotone"
                      dataKey="value"
                      stroke="var(--chart-cost)"
                      strokeWidth={2}
                      dot={false}
                    />
                  </LineChart>
                </ResponsiveContainer>
              )}
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
            <div className="chart cost-breakdown-chart">
              {costChartEmpty ? (
                <div className="chart-empty">
                  {loading ? "Loading breakdown..." : "No cost data in this range."}
                </div>
              ) : (
                <ResponsiveContainer width="100%" height={220}>
                  <BarChart
                    data={costChartData}
                    margin={{ top: 10, right: 20, left: 0, bottom: 10 }}
                  >
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--chart-grid)" />
                    <XAxis dataKey="model" tick={{ fill: "var(--chart-axis)", fontSize: 11 }} />
                    <YAxis tick={{ fill: "var(--chart-axis)", fontSize: 11 }} />
                    <Tooltip
                      contentStyle={{
                        background: "var(--chart-tooltip-bg)",
                        border: "1px solid var(--border)"
                      }}
                      labelFormatter={formatBucketLabel}
                      formatter={(value) => formatCurrency(value as number)}
                    />
                    <Legend
                      iconSize={10}
                      wrapperStyle={{
                        color: "var(--text)",
                        fontSize: 10,
                        lineHeight: "12px"
                      }}
                    />
                    <Bar dataKey="input" stackId="cost" fill="var(--chart-token)" />
                    <Bar dataKey="cached" stackId="cost" fill="var(--chart-cached)" />
                    <Bar dataKey="output" stackId="cost" fill="var(--chart-cost)" />
                  </BarChart>
                </ResponsiveContainer>
              )}
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
                  {pagedCostBreakdown.length === 0 ? (
                    <tr>
                      <td colSpan={9} className="empty-cell">
                        {loading ? "Loading cost breakdown..." : "No cost data for this range."}
                      </td>
                    </tr>
                  ) : (
                    pagedCostBreakdown.flatMap((item) => {
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
                  }))}
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
              {costSeriesEmpty ? (
                <div className="chart-empty">
                  {loading ? "Loading breakdown..." : "No cost data in this range."}
                </div>
              ) : (
                <ResponsiveContainer width="100%" height={220}>
                  <BarChart
                    data={costSeries}
                    margin={{ top: 10, right: 20, left: 0, bottom: 10 }}
                  >
                    <CartesianGrid strokeDasharray="3 3" stroke="var(--chart-grid)" />
                    <XAxis
                      dataKey="bucket_start"
                      tick={{ fill: "var(--chart-axis)", fontSize: 11 }}
                      tickFormatter={(value) => formatBucketLabel(value as string, chartBucket)}
                    />
                    <YAxis tick={{ fill: "var(--chart-axis)", fontSize: 11 }} />
                    <Tooltip
                      contentStyle={{
                        background: "var(--chart-tooltip-bg)",
                        border: "1px solid var(--border)"
                      }}
                      labelFormatter={(value) =>
                        formatBucketLabel(value as string, chartBucket)
                      }
                      formatter={(value) => formatCurrency(value as number)}
                    />
                    <Bar dataKey="value" fill="var(--chart-cost)" />
                  </BarChart>
                </ResponsiveContainer>
              )}
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
                  {pagedCostSeries.length === 0 ? (
                    <tr>
                      <td colSpan={2} className="empty-cell">
                        {loading ? "Loading cost history..." : "No cost data for this range."}
                      </td>
                    </tr>
                  ) : (
                    pagedCostSeries.map((point) => (
                      <tr key={point.bucket_start}>
                        <td>{formatBucketLabel(point.bucket_start, chartBucket)}</td>
                        <td>{formatCurrency(point.value)}</td>
                      </tr>
                    ))
                  )}
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
            <SelectField
              value={modelFilter}
              onValueChange={setModelFilter}
              options={modelSelectOptions}
              ariaLabel="Model filter"
            />
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
              {pagedEvents.length === 0 ? (
                <tr>
                  <td colSpan={7} className="empty-cell">
                    {loading ? "Loading events..." : "No events in this range."}
                  </td>
                </tr>
              ) : (
                pagedEvents.map((event) => (
                  <tr key={event.id}>
                    <td>{new Date(event.ts).toLocaleString()}</td>
                    <td>{event.model}</td>
                    <td>{formatEffort(event.reasoning_effort)}</td>
                    <td>{formatNumber(event.usage.total_tokens)}</td>
                    <td>{formatNumber(event.usage.input_tokens)}</td>
                    <td>{formatNumber(event.usage.output_tokens)}</td>
                    <td>{formatCurrency(event.cost_usd)}</td>
                  </tr>
                ))
              )}
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
