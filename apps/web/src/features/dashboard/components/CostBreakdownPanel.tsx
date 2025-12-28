import { useMemo, useState, useEffect } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Legend,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis
} from "recharts";
import type { ModelCostBreakdown, ModelEffortCostBreakdown, TimeSeriesPoint } from "../../../domain/types";
import { COST_BREAKDOWN_PAGE_SIZE } from "../../../shared/constants";
import {
  formatBucketLabel,
  formatCostPerMillion,
  formatCurrency,
  formatEffort,
  formatNumber
} from "../../../shared/formatters";

type CostBreakdownPanelProps = {
  breakdown: ModelCostBreakdown[];
  effortBreakdown: ModelEffortCostBreakdown[];
  costSeries: TimeSeriesPoint[];
  chartBucketMode: "day" | "hour";
  loading: boolean;
};

export function CostBreakdownPanel({
  breakdown,
  effortBreakdown,
  costSeries,
  chartBucketMode,
  loading
}: CostBreakdownPanelProps) {
  const [costBreakdownTab, setCostBreakdownTab] = useState<"model" | "day">("model");
  const [costBreakdownPage, setCostBreakdownPage] = useState(1);
  const [costSeriesPage, setCostSeriesPage] = useState(1);
  const [expandedModels, setExpandedModels] = useState<Set<string>>(new Set());

  const chartBucketLabel = chartBucketMode === "hour" ? "hour" : "day";
  const totalCostKnown = breakdown.some((item) => item.total_cost_usd !== null);

  const costChartData = breakdown.map((item) => ({
    model: item.model,
    input: item.input_cost_usd ?? 0,
    cached: item.cached_input_cost_usd ?? 0,
    output: item.output_cost_usd ?? 0
  }));
  const costChartEmpty = costChartData.length === 0;
  const costSeriesEmpty = costSeries.length === 0;

  const totalCostBreakdownPages = Math.max(
    1,
    Math.ceil(breakdown.length / COST_BREAKDOWN_PAGE_SIZE)
  );
  const totalCostSeriesPages = Math.max(
    1,
    Math.ceil(costSeries.length / COST_BREAKDOWN_PAGE_SIZE)
  );

  const pagedCostBreakdown = breakdown.slice(
    (costBreakdownPage - 1) * COST_BREAKDOWN_PAGE_SIZE,
    costBreakdownPage * COST_BREAKDOWN_PAGE_SIZE
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

  return (
    <section className="panel">
      <div className="panel-header">
        <div>
          <h2>Cost Breakdown</h2>
          <p>Switch between model and time-bucket cost views.</p>
        </div>
        {!totalCostKnown && <span className="tag">Pricing missing for some models</span>}
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
                <BarChart data={costChartData} margin={{ top: 10, right: 20, left: 0, bottom: 10 }}>
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
                              cached_input_tokens: acc.cached_input_tokens + row.cached_input_tokens,
                              output_tokens: acc.output_tokens + row.output_tokens,
                              reasoning_output_tokens:
                                acc.reasoning_output_tokens + row.reasoning_output_tokens,
                              total_tokens: acc.total_tokens + row.total_tokens,
                              total_cost_usd: (acc.total_cost_usd ?? 0) + (row.total_cost_usd ?? 0)
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
                        <td>{formatCostPerMillion(totalRow.total_cost_usd, totalRow.total_tokens)}</td>
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
                  })
                )}
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
                    setCostBreakdownPage((page) => Math.min(totalCostBreakdownPages, page + 1))
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
                <BarChart data={costSeries} margin={{ top: 10, right: 20, left: 0, bottom: 10 }}>
                  <CartesianGrid strokeDasharray="3 3" stroke="var(--chart-grid)" />
                  <XAxis
                    dataKey="bucket_start"
                    tick={{ fill: "var(--chart-axis)", fontSize: 11 }}
                    tickFormatter={(value) => formatBucketLabel(value as string, chartBucketMode)}
                  />
                  <YAxis tick={{ fill: "var(--chart-axis)", fontSize: 11 }} />
                  <Tooltip
                    contentStyle={{
                      background: "var(--chart-tooltip-bg)",
                      border: "1px solid var(--border)"
                    }}
                    labelFormatter={(value) =>
                      formatBucketLabel(value as string, chartBucketMode)
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
                      <td>{formatBucketLabel(point.bucket_start, chartBucketMode)}</td>
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
                    setCostSeriesPage((page) => Math.min(totalCostSeriesPages, page + 1))
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
  );
}
