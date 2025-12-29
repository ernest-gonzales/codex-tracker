import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { IngestStats, RangeParams } from "../../domain/types";
import { fetchDashboardData, type DashboardPayload } from "../../data/dashboard";
import { clearCached, getCached, setCached } from "../../data/cache";
import { runIngest } from "../../data/codexApi";
import type { ChartBucketMode } from "../../shared/constants";
import { isTauriRuntime } from "../../shared/tauri";
import type { ToastMessage } from "../shared/Toast";

const DASHBOARD_CACHE_PREFIX = "dashboard:";

type DashboardDataOptions = {
  rangeParams: RangeParams;
  rangeParamsKey: string;
  modelFilter: string;
  activeMinutes: number;
  chartBucket: ChartBucketMode;
  refreshToken?: number;
  onToast?: (toast: ToastMessage) => void;
};

export function useDashboardData({
  rangeParams,
  rangeParamsKey,
  modelFilter,
  activeMinutes,
  chartBucket,
  refreshToken,
  onToast
}: DashboardDataOptions) {
  const [data, setData] = useState<DashboardPayload | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [ingestStats, setIngestStats] = useState<IngestStats | null>(null);
  const [isIngesting, setIsIngesting] = useState(false);
  const ingestInFlight = useRef(false);
  const requestIdRef = useRef(0);
  const lastRefreshTokenRef = useRef<number | undefined>(refreshToken);

  const cacheKey = useMemo(
    () =>
      `${DASHBOARD_CACHE_PREFIX}${rangeParamsKey}:${modelFilter}:${activeMinutes}:${chartBucket}`,
    [rangeParamsKey, modelFilter, activeMinutes, chartBucket]
  );

  const refresh = useCallback(
    async (options?: { force?: boolean }) => {
      const force = options?.force ?? false;
      const requestId = (requestIdRef.current += 1);

      if (force) {
        clearCached(DASHBOARD_CACHE_PREFIX);
      }
      if (!force) {
        const cached = getCached<DashboardPayload>(cacheKey);
        if (cached) {
          setData(cached.data);
        }
      }

      setLoading(true);
      setError("");
      try {
        const result = await fetchDashboardData({
          rangeParams,
          chartBucket,
          modelFilter,
          activeMinutes
        });
        if (requestId !== requestIdRef.current) {
          return;
        }
        setData(result);
        setCached(cacheKey, result);
      } catch (err) {
        if (requestId !== requestIdRef.current) {
          return;
        }
        const message = err instanceof Error ? err.message : "Failed to load data";
        setError(message);
        onToast?.({ message, tone: "error" });
      } finally {
        if (requestId === requestIdRef.current) {
          setLoading(false);
        }
      }
    },
    [activeMinutes, cacheKey, chartBucket, modelFilter, onToast, rangeParams]
  );

  const invalidateCache = useCallback(() => {
    clearCached(DASHBOARD_CACHE_PREFIX);
  }, []);

  const ingest = useCallback(async () => {
    if (ingestInFlight.current) {
      return;
    }
    ingestInFlight.current = true;
    setIsIngesting(true);
    try {
      const stats = await runIngest();
      setIngestStats(stats);
      invalidateCache();
      await refresh({ force: true });
    } catch (err) {
      const message = err instanceof Error ? err.message : "Ingest failed";
      setError(message);
      onToast?.({ message, tone: "error" });
    } finally {
      ingestInFlight.current = false;
      setIsIngesting(false);
    }
  }, [invalidateCache, onToast, refresh]);

  useEffect(() => {
    const force = refreshToken !== undefined && refreshToken !== lastRefreshTokenRef.current;
    lastRefreshTokenRef.current = refreshToken;
    refresh({ force });
  }, [refresh, refreshToken]);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }
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
          invalidateCache();
          refresh({ force: true });
        });
      } catch (err) {
        onToast?.({
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
  }, [invalidateCache, onToast, refresh]);

  return {
    data,
    loading,
    error,
    ingestStats,
    isIngesting,
    refresh,
    ingest,
    invalidateCache
  };
}
