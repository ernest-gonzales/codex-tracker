import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis
} from "recharts";
import type { SelectOption } from "../../../components/Select";
import { SelectField } from "../../../components/Select";
import type { TimeSeriesPoint } from "../../../domain/types";
import type { ChartBucketMode } from "../../../shared/constants";
import { formatBucketLabel, formatCurrency, formatNumber } from "../../../shared/formatters";

type ChartsPanelProps = {
  tokensSeries: TimeSeriesPoint[];
  costSeries: TimeSeriesPoint[];
  chartBucketMode: ChartBucketMode;
  onChartBucketChange: (value: ChartBucketMode) => void;
  bucketOptions: SelectOption[];
  loading: boolean;
};

export function ChartsPanel({
  tokensSeries,
  costSeries,
  chartBucketMode,
  onChartBucketChange,
  bucketOptions,
  loading
}: ChartsPanelProps) {
  const chartBucketLabel = chartBucketMode === "hour" ? "hour" : "day";
  const tokensSeriesEmpty = tokensSeries.length === 0;
  const costSeriesEmpty = costSeries.length === 0;

  return (
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
            onValueChange={(value) => onChartBucketChange(value as ChartBucketMode)}
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
  );
}
