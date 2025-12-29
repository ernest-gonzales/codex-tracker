import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ChangeEvent, KeyboardEvent } from "react";
import type { SelectOption } from "../../components/Select";
import { openLogsDir } from "../../data/codexApi";
import type { ActiveSession, UsageLimitWindow } from "../../domain/types";
import {
  AUTO_REFRESH_OPTIONS,
  RANGE_OPTIONS,
  STORAGE_KEYS,
  type AutoRefreshValue,
  type ChartBucketMode,
  type RangeValue
} from "../../shared/constants";
import { formatPercent, formatNumber } from "../../shared/formatters";
import { isEditableTarget } from "../../shared/keyboard";
import { clampPercent } from "../../shared/math";
import { formatDateInputValue } from "../../shared/dates";
import { buildRangeParams, rangeLabel } from "../../shared/range";
import { safeStorageGet, safeStorageSet } from "../../shared/storage";
import type { ToastMessage } from "../shared/Toast";
import { DashboardHeader } from "./components/DashboardHeader";
import { ActiveSessionsPanel } from "./components/ActiveSessionsPanel";
import { LimitsPanel } from "./components/LimitsPanel";
import { ChartsPanel } from "./components/ChartsPanel";
import { CostBreakdownPanel } from "./components/CostBreakdownPanel";
import { EventsPanel } from "./components/EventsPanel";
import { SessionDetailModal } from "./components/SessionDetailModal";
import { useDashboardData } from "./useDashboardData";

type DashboardPageProps = {
  activeMinutes: number;
  refreshToken?: number;
  onOpenSettings: () => void;
  onToast?: (toast: ToastMessage) => void;
};

type LimitWindowRow = UsageLimitWindow & { delta?: number | null };

export function DashboardPage({
  activeMinutes,
  refreshToken,
  onOpenSettings,
  onToast
}: DashboardPageProps) {
  const [range, setRange] = useState<RangeValue>("today");
  const [customStart, setCustomStart] = useState("");
  const [customEnd, setCustomEnd] = useState("");
  const [autoRefresh, setAutoRefresh] = useState<AutoRefreshValue>("15s");
  const [chartBucketMode, setChartBucketMode] = useState<ChartBucketMode>("hour");
  const [modelFilter, setModelFilter] = useState("all");
  const [selectedSession, setSelectedSession] = useState<ActiveSession | null>(null);

  const customStartRef = useRef<HTMLInputElement | null>(null);
  const customEndRef = useRef<HTMLInputElement | null>(null);

  const rangeParams = useMemo(
    () => buildRangeParams(range, customStart, customEnd),
    [range, customStart, customEnd]
  );
  const rangeParamsKey = useMemo(() => JSON.stringify(rangeParams), [rangeParams]);

  const { data, loading, error, ingestStats, isIngesting, ingest } = useDashboardData({
    rangeParams,
    rangeParamsKey,
    modelFilter,
    activeMinutes,
    chartBucket: chartBucketMode,
    refreshToken,
    onToast
  });

  const summary = data?.summary ?? null;
  const tokensSeries = data?.tokensSeries ?? [];
  const costSeries = data?.costSeries ?? [];
  const breakdown = data?.breakdown ?? [];
  const effortBreakdown = data?.effortBreakdown ?? [];
  const contextStats = data?.contextStats ?? null;
  const limits = data?.limits ?? null;
  const limitCurrent = data?.limitCurrent ?? null;
  const limitWindows = data?.limitWindows ?? [];
  const events = data?.events ?? [];
  const activeSessions = data?.activeSessions ?? [];

  const showSummarySkeleton = loading && !summary;
  const isRefreshing = loading || isIngesting;

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
  const bucketOptions = useMemo<SelectOption[]>(
    () => [
      { value: "day", label: "Day" },
      { value: "hour", label: "Hour" }
    ],
    []
  );

  const modelSelectOptions = useMemo<SelectOption[]>(() => {
    const models = new Set(breakdown.map((item) => item.model));
    return ["all", ...Array.from(models).sort()].map((model) => ({
      value: model,
      label: model === "all" ? "All models" : model
    }));
  }, [breakdown]);

  const autoRefreshInterval = useMemo(() => {
    return AUTO_REFRESH_OPTIONS.find((option) => option.value === autoRefresh)?.ms ?? 0;
  }, [autoRefresh]);

  const ingestStatus = ingestStats
    ? `Last scan: ${formatNumber(ingestStats.files_scanned)} files · +${formatNumber(
        ingestStats.events_inserted
      )} events`
    : "Last scan: —";

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

  const primaryLimitPercent = clampPercent(limits?.primary?.percent_left ?? 100);
  const secondaryLimitPercent = clampPercent(limits?.secondary?.percent_left ?? 100);

  const limitWindowRows = useMemo<LimitWindowRow[]>(() => {
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
        previous?.total_tokens !== null &&
        previous?.total_tokens !== undefined &&
        window.total_tokens !== null &&
        window.total_tokens !== undefined &&
        previous.total_tokens > 0
      ) {
        delta = ((window.total_tokens - previous.total_tokens) / previous.total_tokens) * 100;
      }
      if (window.total_tokens !== null && window.total_tokens !== undefined) {
        previous = window;
      }
      return { ...window, delta };
    });
    return withDelta.reverse();
  }, [limitWindows]);

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
    const storedAutoRefresh = safeStorageGet(STORAGE_KEYS.autoRefresh);
    if (
      storedAutoRefresh &&
      AUTO_REFRESH_OPTIONS.some((option) => option.value === storedAutoRefresh)
    ) {
      setAutoRefresh(storedAutoRefresh as AutoRefreshValue);
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
    safeStorageSet(STORAGE_KEYS.autoRefresh, autoRefresh);
  }, [autoRefresh]);

  useEffect(() => {
    if (!autoRefreshInterval) {
      return;
    }
    const intervalId = window.setInterval(() => {
      ingest();
    }, autoRefreshInterval);
    return () => window.clearInterval(intervalId);
  }, [autoRefreshInterval, ingest]);

  const handleOpenLogs = useCallback(async () => {
    try {
      await openLogsDir();
    } catch (err) {
      onToast?.({
        message: err instanceof Error ? err.message : "Open logs failed",
        tone: "error"
      });
    }
  }, [onToast]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.defaultPrevented) {
        return;
      }
      if (event.key === "Escape") {
        if (selectedSession) {
          event.preventDefault();
          setSelectedSession(null);
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
        ingest();
      }
      if (key === "l") {
        event.preventDefault();
        handleOpenLogs();
      }
      if (key === ",") {
        event.preventDefault();
        onOpenSettings();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleOpenLogs, ingest, onOpenSettings, selectedSession]);

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
      onToast?.({ message: "Session id copied", tone: "info" });
    } catch (err) {
      onToast?.({
        message: err instanceof Error ? err.message : "Copy failed",
        tone: "error"
      });
    }
  }

  return (
    <>
      {selectedSession && (
        <SessionDetailModal
          session={selectedSession}
          onClose={() => setSelectedSession(null)}
          onCopySessionId={handleCopySessionId}
        />
      )}
      <DashboardHeader
        range={range}
        rangeOptions={rangeOptions}
        autoRefresh={autoRefresh}
        autoRefreshOptions={autoRefreshOptions}
        customStart={customStart}
        customEnd={customEnd}
        customStartRef={customStartRef}
        customEndRef={customEndRef}
        onRangeChange={setRange}
        onAutoRefreshChange={setAutoRefresh}
        onCustomStartChange={handleCustomStartChange}
        onCustomEndChange={handleCustomEndChange}
        onDateInputKeyDown={handleDateInputKeyDown}
        onRefresh={ingest}
        onOpenLogs={handleOpenLogs}
        onOpenSettings={onOpenSettings}
        isRefreshing={isRefreshing}
        ingestStatus={ingestStatus}
        error={error}
        summary={summary}
        showSummarySkeleton={showSummarySkeleton}
      />
      <ActiveSessionsPanel
        activeSessions={activeSessions}
        activeMinutes={activeMinutes}
        rangeAvgLabel={rangeAvgLabel}
        rangeAvgTooltip={rangeAvgTooltip}
        onSelectSession={setSelectedSession}
        onCopySessionId={handleCopySessionId}
      />
      <LimitsPanel
        limits={limits}
        limitCurrent={limitCurrent}
        limitWindowRows={limitWindowRows}
        primaryLimitPercent={primaryLimitPercent}
        secondaryLimitPercent={secondaryLimitPercent}
      />
      <ChartsPanel
        tokensSeries={tokensSeries}
        costSeries={costSeries}
        chartBucketMode={chartBucketMode}
        onChartBucketChange={setChartBucketMode}
        bucketOptions={bucketOptions}
        loading={loading}
      />
      <CostBreakdownPanel
        breakdown={breakdown}
        effortBreakdown={effortBreakdown}
        costSeries={costSeries}
        chartBucketMode={chartBucketMode}
        loading={loading}
      />
      <EventsPanel
        events={events}
        loading={loading}
        modelFilter={modelFilter}
        modelOptions={modelSelectOptions}
        onModelFilterChange={setModelFilter}
        rangeParamsKey={rangeParamsKey}
      />
    </>
  );
}
