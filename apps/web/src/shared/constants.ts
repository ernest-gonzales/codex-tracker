export const RANGE_OPTIONS = [
  { value: "today", label: "Today" },
  { value: "last7days", label: "Last 7 Days" },
  { value: "last14days", label: "Last 14 Days" },
  { value: "thismonth", label: "This Month" },
  { value: "alltime", label: "All Time" },
  { value: "custom", label: "Custom" }
] as const;

export const AUTO_REFRESH_OPTIONS = [
  { value: "off", label: "Off", ms: 0 },
  { value: "15s", label: "Every 15 seconds", ms: 15_000 },
  { value: "30s", label: "Every 30 seconds", ms: 30_000 },
  { value: "1m", label: "Every 1 minute", ms: 60_000 },
  { value: "5m", label: "Every 5 minutes", ms: 5 * 60_000 },
  { value: "15m", label: "Every 15 minutes", ms: 15 * 60_000 },
  { value: "30m", label: "Every 30 minutes", ms: 30 * 60_000 }
] as const;

export const EVENTS_PER_PAGE = 10;
export const COST_BREAKDOWN_PAGE_SIZE = 10;

export const STORAGE_KEYS = {
  range: "codex-tracker.range",
  rangeStart: "codex-tracker.range.start",
  rangeEnd: "codex-tracker.range.end",
  settingsTab: "codex-tracker.settings.tab"
};

export type RangeValue = (typeof RANGE_OPTIONS)[number]["value"];
export type AutoRefreshValue = (typeof AUTO_REFRESH_OPTIONS)[number]["value"];
export type ChartBucketMode = "day" | "hour";

export type SettingsTabValue =
  | "settings-homes"
  | "settings-display"
  | "settings-storage"
  | "settings-pricing";
