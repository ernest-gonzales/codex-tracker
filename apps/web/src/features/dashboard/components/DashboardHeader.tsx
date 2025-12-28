import type { ChangeEvent, KeyboardEvent, RefObject } from "react";
import type { SelectOption } from "../../../components/Select";
import { SelectField } from "../../../components/Select";
import { CodexTrackerLogo } from "../../../Logo";
import type { UsageSummary } from "../../../domain/types";
import type { AutoRefreshValue, RangeValue } from "../../../shared/constants";
import { SummaryCards } from "./SummaryCards";

type DashboardHeaderProps = {
  range: RangeValue;
  rangeOptions: SelectOption[];
  autoRefresh: AutoRefreshValue;
  autoRefreshOptions: SelectOption[];
  customStart: string;
  customEnd: string;
  customStartRef: RefObject<HTMLInputElement>;
  customEndRef: RefObject<HTMLInputElement>;
  onRangeChange: (value: RangeValue) => void;
  onAutoRefreshChange: (value: AutoRefreshValue) => void;
  onCustomStartChange: (event: ChangeEvent<HTMLInputElement>) => void;
  onCustomEndChange: (event: ChangeEvent<HTMLInputElement>) => void;
  onDateInputKeyDown: (event: KeyboardEvent<HTMLInputElement>) => void;
  onRefresh: () => void;
  onOpenLogs: () => void;
  onOpenSettings: () => void;
  isRefreshing: boolean;
  ingestStatus: string;
  error: string;
  summary: UsageSummary | null;
  showSummarySkeleton: boolean;
};

export function DashboardHeader({
  range,
  rangeOptions,
  autoRefresh,
  autoRefreshOptions,
  customStart,
  customEnd,
  customStartRef,
  customEndRef,
  onRangeChange,
  onAutoRefreshChange,
  onCustomStartChange,
  onCustomEndChange,
  onDateInputKeyDown,
  onRefresh,
  onOpenLogs,
  onOpenSettings,
  isRefreshing,
  ingestStatus,
  error,
  summary,
  showSummarySkeleton
}: DashboardHeaderProps) {
  return (
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
                onValueChange={(value) => onRangeChange(value as RangeValue)}
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
                  onChange={onCustomStartChange}
                  onKeyDown={onDateInputKeyDown}
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
                  onChange={onCustomEndChange}
                  onKeyDown={onDateInputKeyDown}
                  className={`input input-compact ${customEnd ? "" : "input-empty"}`}
                  aria-label="End date"
                  disabled={range !== "custom"}
                />
                {!customEnd && <span className="date-placeholder">End date</span>}
              </div>
            </div>
            <div className="range-group">
              <span className="label">Auto refresh</span>
              <SelectField
                value={autoRefresh}
                onValueChange={(value) => onAutoRefreshChange(value as AutoRefreshValue)}
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
              onClick={onRefresh}
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
              onClick={onOpenLogs}
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
              onClick={onOpenSettings}
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
      <SummaryCards summary={summary} showSkeleton={showSummarySkeleton} />
    </header>
  );
}
