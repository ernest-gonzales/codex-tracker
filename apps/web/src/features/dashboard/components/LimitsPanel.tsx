import { useState } from "react";
import type { LimitsResponse, UsageLimitCurrentResponse, UsageLimitWindow } from "../../../domain/types";
import {
  formatBucketLabel,
  formatCurrency,
  formatLimitPercentLeft,
  formatNumber,
  formatRelativeReset,
  formatResetLabel
} from "../../../shared/formatters";

type LimitWindowRow = UsageLimitWindow & { delta?: number | null };

type LimitsPanelProps = {
  limits: LimitsResponse | null;
  limitCurrent: UsageLimitCurrentResponse | null;
  limitWindowRows: LimitWindowRow[];
  primaryLimitPercent: number;
  secondaryLimitPercent: number;
};

export function LimitsPanel({
  limits,
  limitCurrent,
  limitWindowRows,
  primaryLimitPercent,
  secondaryLimitPercent
}: LimitsPanelProps) {
  const [showLimitDetails, setShowLimitDetails] = useState(false);

  return (
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
            <div className="limit-progress-fill" style={{ width: `${primaryLimitPercent}%` }} />
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
            <div className="limit-progress-fill" style={{ width: `${secondaryLimitPercent}%` }} />
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
                      <span className="limits-cell" data-label="Window">
                        {startLabel} → {endLabel}
                        {isCurrent && <span className="limit-badge">Current</span>}
                      </span>
                      <span className="limits-cell" data-label="Tokens">
                        {formatNumber(window.total_tokens)}
                      </span>
                      <span className="limits-cell" data-label="Cost">
                        {formatCurrency(window.total_cost_usd)}
                      </span>
                      <span className="limits-cell" data-label="Messages">
                        {formatNumber(window.message_count)}
                      </span>
                      <span className={`limits-cell ${deltaClass}`} data-label="Change">
                        {deltaLabel}
                      </span>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        ))}
    </section>
  );
}
