import type { UsageSummary } from "../../../domain/types";
import { formatCurrency, formatNumber } from "../../../shared/formatters";

type SummaryCardsProps = {
  summary: UsageSummary | null;
  showSkeleton: boolean;
};

export function SummaryCards({ summary, showSkeleton }: SummaryCardsProps) {
  return (
    <section className="grid summary-grid hero-metrics">
      <div className="card kpi-card">
        <p className="card-label">Total Tokens</p>
        <div className="card-value-row">
          <p className="card-value tabular-nums">
            {showSkeleton ? (
              <span className="skeleton-line skeleton-line-lg" />
            ) : (
              formatNumber(summary?.total_tokens)
            )}
          </p>
          <p className="card-meta-inline tabular-nums">
            {showSkeleton ? (
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
            {showSkeleton ? (
              <span className="skeleton-line skeleton-line-lg" />
            ) : (
              formatCurrency(summary?.total_cost_usd)
            )}
          </p>
          <p className="card-meta-inline tabular-nums">
            {showSkeleton ? (
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
            {showSkeleton ? (
              <span className="skeleton-line skeleton-line-lg" />
            ) : (
              formatNumber(summary?.cached_input_tokens)
            )}
          </p>
          <p className="card-meta-inline tabular-nums">
            {showSkeleton ? (
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
            {showSkeleton ? (
              <span className="skeleton-line skeleton-line-lg" />
            ) : (
              formatNumber(summary?.output_tokens)
            )}
          </p>
          <p className="card-meta-inline tabular-nums">
            {showSkeleton ? (
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
  );
}
