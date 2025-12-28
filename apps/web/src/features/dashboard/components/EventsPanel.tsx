import { useEffect, useMemo, useState } from "react";
import type { SelectOption } from "../../../components/Select";
import { SelectField } from "../../../components/Select";
import type { UsageEvent } from "../../../domain/types";
import { EVENTS_PER_PAGE } from "../../../shared/constants";
import { formatCurrency, formatEffort, formatNumber } from "../../../shared/formatters";

type EventsPanelProps = {
  events: UsageEvent[];
  loading: boolean;
  modelFilter: string;
  modelOptions: SelectOption[];
  onModelFilterChange: (value: string) => void;
  rangeParamsKey: string;
};

export function EventsPanel({
  events,
  loading,
  modelFilter,
  modelOptions,
  onModelFilterChange,
  rangeParamsKey
}: EventsPanelProps) {
  const [eventsPage, setEventsPage] = useState(1);
  const totalEventPages = Math.max(1, Math.ceil(events.length / EVENTS_PER_PAGE));

  const pagedEvents = useMemo(() => {
    const startIndex = (eventsPage - 1) * EVENTS_PER_PAGE;
    return events.slice(startIndex, startIndex + EVENTS_PER_PAGE);
  }, [events, eventsPage]);

  useEffect(() => {
    setEventsPage(1);
  }, [rangeParamsKey, modelFilter]);

  useEffect(() => {
    setEventsPage((prev) => Math.min(Math.max(prev, 1), totalEventPages));
  }, [totalEventPages]);

  return (
    <section className="panel">
      <div className="panel-header">
        <div>
          <h2>Recent Events</h2>
          <p>Filtered by range and model.</p>
        </div>
        <div className="filters">
          <label className="label">Model</label>
          <SelectField
            value={modelFilter}
            onValueChange={onModelFilterChange}
            options={modelOptions}
            ariaLabel="Model filter"
          />
        </div>
      </div>
      <div className="table-wrap events-table">
        <table>
          <thead>
            <tr>
              <th>Timestamp</th>
              <th>Model</th>
              <th>Effort</th>
              <th>Total Tokens</th>
              <th>Input</th>
              <th>Output</th>
              <th>Cost</th>
            </tr>
          </thead>
          <tbody>
            {pagedEvents.length === 0 ? (
              <tr>
                <td colSpan={7} className="empty-cell">
                  {loading ? "Loading events..." : "No events in this range."}
                </td>
              </tr>
            ) : (
              pagedEvents.map((event) => (
                <tr key={event.id}>
                  <td>{new Date(event.ts).toLocaleString()}</td>
                  <td>{event.model}</td>
                  <td>{formatEffort(event.reasoning_effort)}</td>
                  <td>{formatNumber(event.usage.total_tokens)}</td>
                  <td>{formatNumber(event.usage.input_tokens)}</td>
                  <td>{formatNumber(event.usage.output_tokens)}</td>
                  <td>{formatCurrency(event.cost_usd)}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      <div className="table-footer">
        <span className="note">
          Showing {events.length === 0 ? 0 : (eventsPage - 1) * EVENTS_PER_PAGE + 1}-
          {Math.min(eventsPage * EVENTS_PER_PAGE, events.length)} of {events.length}
        </span>
        <div className="pagination">
          <button
            className="button ghost small"
            onClick={() => setEventsPage((page) => Math.max(1, page - 1))}
            disabled={eventsPage === 1}
          >
            Previous
          </button>
          <span className="pagination-status">
            Page {eventsPage} of {totalEventPages}
          </span>
          <button
            className="button ghost small"
            onClick={() => setEventsPage((page) => Math.min(totalEventPages, page + 1))}
            disabled={eventsPage === totalEventPages}
          >
            Next
          </button>
        </div>
      </div>
    </section>
  );
}
