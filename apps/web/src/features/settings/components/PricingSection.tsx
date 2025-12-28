import type { PricingRule } from "../../../domain/types";
import { formatCurrency, formatDateTime } from "../../../shared/formatters";
import { formatDateOnlyLocal, parseDateOnlyLocal } from "../../../shared/dates";

type PricingRow = { rule: PricingRule; index: number };

type PricingSectionProps = {
  pricingRows: PricingRow[];
  pricingIssueMap: Map<number, Array<{ message: string; field?: string }>>;
  pricingHasIssues: boolean;
  pricingDirty: boolean;
  pricingStatus: string;
  pricingBusy: boolean;
  pricingFilter: string;
  pricingLastRecompute: string | null;
  onPricingFilterChange: (value: string) => void;
  onAddRule: () => void;
  onSavePricing: () => void;
  onRecomputeCosts: () => void;
  onUpdateRule: (index: number, patch: Partial<PricingRule>) => void;
  onDuplicateRule: (index: number) => void;
  onDeleteRule: (index: number) => void;
};

export function PricingSection({
  pricingRows,
  pricingIssueMap,
  pricingHasIssues,
  pricingDirty,
  pricingStatus,
  pricingBusy,
  pricingFilter,
  pricingLastRecompute,
  onPricingFilterChange,
  onAddRule,
  onSavePricing,
  onRecomputeCosts,
  onUpdateRule,
  onDuplicateRule,
  onDeleteRule
}: PricingSectionProps) {
  return (
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
            onChange={(event) => onPricingFilterChange(event.target.value)}
            placeholder="Filter by model pattern"
          />
        </div>
        <div className="pricing-actions">
          <button className="button ghost" onClick={onAddRule} disabled={pricingBusy}>
            Add Rule
          </button>
          <button
            className="button"
            onClick={onSavePricing}
            disabled={pricingBusy || pricingHasIssues || !pricingDirty}
          >
            Save Pricing
          </button>
          <button className="button ghost" onClick={onRecomputeCosts} disabled={pricingBusy}>
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
            <span className="note">Last recompute {formatDateTime(pricingLastRecompute)}</span>
          )}
        </div>
      </div>
      {pricingHasIssues && (
        <div className="note error-note">Resolve validation issues before saving pricing rules.</div>
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
                const hasModelError = rowIssues.some((issue) => issue.field === "model_pattern");
                const hasInputError = rowIssues.some((issue) => issue.field === "input_per_1m");
                const hasCachedError = rowIssues.some(
                  (issue) => issue.field === "cached_input_per_1m"
                );
                const hasOutputError = rowIssues.some((issue) => issue.field === "output_per_1m");
                const hasRangeError = rowIssues.some((issue) => issue.field === "range");
                return (
                  <tr key={rule.id ?? `${rule.model_pattern}-${index}`}>
                    <td className="pricing-model-cell">
                      <input
                        className={`input pricing-model ${hasModelError ? "input-error" : ""}`}
                        value={rule.model_pattern}
                        onChange={(event) =>
                          onUpdateRule(index, { model_pattern: event.target.value })
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
                          onUpdateRule(index, { input_per_1m: Number(event.target.value) })
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
                          onUpdateRule(index, { cached_input_per_1m: Number(event.target.value) })
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
                          onUpdateRule(index, { output_per_1m: Number(event.target.value) })
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
                          onUpdateRule(index, { effective_from: parseDateOnlyLocal(event.target.value) })
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
                            onUpdateRule(index, {
                              effective_to: event.target.value
                                ? parseDateOnlyLocal(event.target.value)
                                : null
                            })
                          }
                        />
                        {!hasEffectiveTo && <span className="input-overlay">No end date</span>}
                      </div>
                    </td>
                    <td className="pricing-actions-cell">
                      <div className="table-actions table-actions-compact">
                        <button
                          className="icon-button icon-button-ghost small"
                          type="button"
                          onClick={() => onDuplicateRule(index)}
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
                          onClick={() => onDeleteRule(index)}
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
                        <div className="input-hint error-note">{rowIssues[0].message}</div>
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
  );
}
