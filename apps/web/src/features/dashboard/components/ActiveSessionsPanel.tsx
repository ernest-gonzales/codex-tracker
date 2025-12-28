import { useMemo } from "react";
import type { ActiveSession } from "../../../domain/types";
import {
  formatBucketLabel,
  formatNumber,
  formatSessionLabel
} from "../../../shared/formatters";

type ActiveSessionsPanelProps = {
  activeSessions: ActiveSession[];
  activeMinutes: number;
  rangeAvgLabel: string;
  rangeAvgTooltip: string;
  onSelectSession: (session: ActiveSession) => void;
  onCopySessionId: (sessionId: string) => void;
};

export function ActiveSessionsPanel({
  activeSessions,
  activeMinutes,
  rangeAvgLabel,
  rangeAvgTooltip,
  onSelectSession,
  onCopySessionId
}: ActiveSessionsPanelProps) {
  const uniformSessionModel = useMemo(() => {
    if (activeSessions.length === 0) {
      return null;
    }
    const model = activeSessions[0]?.model;
    if (!model) {
      return null;
    }
    return activeSessions.every((session) => session.model === model) ? model : null;
  }, [activeSessions]);
  const showSessionModel = uniformSessionModel === null;

  return (
    <section className="panel active-sessions">
      <div className="panel-header">
        <div>
          <h2>Active Sessions</h2>
          <p>Context pressure for sessions seen in the last {activeMinutes} minutes.</p>
          <div className="chip-row">
            <span className="chip" title={rangeAvgTooltip}>
              {rangeAvgLabel}
            </span>
          </div>
        </div>
        <span className="tag">Window {activeMinutes}m</span>
      </div>
      {activeSessions.length === 0 ? (
        <p className="note">No recent sessions in this window.</p>
      ) : (
        <div className={`session-table ${showSessionModel ? "" : "session-table-compact"}`}>
          <div className="session-row session-row-head">
            <span>Session</span>
            {showSessionModel && <span>Model</span>}
            <span>Context</span>
            <span>Pressure</span>
          </div>
          {activeSessions.map((session) => {
            const percent =
              session.context_window > 0
                ? Math.min(100, (session.context_used / session.context_window) * 100)
                : 0;
            const startedLabel = formatBucketLabel(session.session_start);
            const lastSeenLabel = formatBucketLabel(session.last_seen);
            return (
              <div
                className="session-row"
                key={session.session_id}
                role="button"
                tabIndex={0}
                aria-label={`Open session ${session.session_id}`}
                onClick={() => onSelectSession(session)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    onSelectSession(session);
                  }
                }}
              >
                <div className="session-cell session-cell-primary">
                  <div className="session-title-row">
                    <div className="session-id" title={session.session_id}>
                      {formatSessionLabel(session.session_id)}
                    </div>
                    {!showSessionModel && uniformSessionModel && (
                      <span className="session-model-badge">{uniformSessionModel}</span>
                    )}
                    <button
                      className="icon-button icon-button-ghost small session-copy"
                      type="button"
                      onClick={(event) => {
                        event.stopPropagation();
                        onCopySessionId(session.session_id);
                      }}
                      aria-label="Copy session id"
                      title="Copy session id"
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
                  </div>
                  <div className="session-sub">
                    Started {startedLabel}
                    {!showSessionModel && ` • Last ${lastSeenLabel}`}
                  </div>
                </div>
                {showSessionModel && (
                  <div className="session-cell">
                    <div className="session-model">{session.model}</div>
                    <div className="session-sub">Last {lastSeenLabel}</div>
                  </div>
                )}
                <div className="session-cell session-cell-metrics">
                  <span className="session-percent">{Math.round(percent)}%</span>
                  <span className="session-meta">
                    {formatNumber(session.context_used)} / {formatNumber(session.context_window)}{" "}
                    tokens
                  </span>
                </div>
                <div className="session-cell session-cell-bar">
                  <div className="session-bar">
                    <div className="session-bar-fill" style={{ width: `${percent}%` }} />
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
