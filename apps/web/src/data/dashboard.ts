import type {
  ActiveSession,
  ContextPressureStats,
  ModelCostBreakdown,
  ModelEffortCostBreakdown,
  RangeParams,
  TimeSeriesPoint,
  UsageEvent,
  UsageLimitCurrentResponse,
  UsageLimitWindow,
  UsageSummary,
  LimitsResponse
} from "../domain/types";
import {
  getActiveSessions,
  getBreakdownCosts,
  getBreakdownEffortCosts,
  getContextStats,
  getEvents,
  getLimitWindows,
  getLimitsCurrent,
  getLimitsLatest,
  getSummary,
  getTimeSeries
} from "./codexApi";

export type DashboardQuery = {
  rangeParams: RangeParams;
  chartBucket: "hour" | "day";
  modelFilter?: string;
  eventsLimit?: number;
  activeMinutes: number;
};

export type DashboardPayload = {
  summary: UsageSummary;
  tokensSeries: TimeSeriesPoint[];
  costSeries: TimeSeriesPoint[];
  breakdown: ModelCostBreakdown[];
  effortBreakdown: ModelEffortCostBreakdown[];
  contextStats: ContextPressureStats;
  limits: LimitsResponse;
  limitCurrent: UsageLimitCurrentResponse;
  limitWindows: UsageLimitWindow[];
  events: UsageEvent[];
  activeSessions: ActiveSession[];
};

export async function fetchDashboardData(query: DashboardQuery): Promise<DashboardPayload> {
  const { rangeParams, chartBucket, modelFilter, eventsLimit = 200, activeMinutes } = query;
  const [
    summary,
    tokensSeries,
    costSeries,
    breakdown,
    effortBreakdown,
    contextStats,
    limits,
    limitCurrent,
    limitWindows,
    events,
    activeSessions
  ] = await Promise.all([
    getSummary(rangeParams),
    getTimeSeries({ ...rangeParams, bucket: chartBucket, metric: "tokens" }),
    getTimeSeries({ ...rangeParams, bucket: chartBucket, metric: "cost" }),
    getBreakdownCosts(rangeParams),
    getBreakdownEffortCosts(rangeParams),
    getContextStats(rangeParams),
    getLimitsLatest(),
    getLimitsCurrent(),
    getLimitWindows(8),
    getEvents({
      ...rangeParams,
      limit: eventsLimit,
      model: modelFilter === "all" ? undefined : modelFilter
    }),
    getActiveSessions({ active_minutes: activeMinutes })
  ]);

  return {
    summary,
    tokensSeries,
    costSeries,
    breakdown,
    effortBreakdown,
    contextStats,
    limits,
    limitCurrent,
    limitWindows,
    events,
    activeSessions
  };
}
