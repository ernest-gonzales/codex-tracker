import type { ActiveSession } from "../../../domain/types";
import {
  formatDateTime,
  formatNumber,
  formatSessionLabel
} from "../../../shared/formatters";

type SessionDetailModalProps = {
  session: ActiveSession;
  onClose: () => void;
  onCopySessionId: (value: string) => void;
};

export function SessionDetailModal({
  session,
  onClose,
  onCopySessionId
}: SessionDetailModalProps) {
  return (
    <div className="modal-overlay" role="presentation" onClick={onClose}>
      <div
        className="modal session-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="session-detail-title"
        onClick={(event) => event.stopPropagation()}
      >
        <header className="modal-header">
          <div>
            <h2 className="modal-title" id="session-detail-title">
              Session {formatSessionLabel(session.session_id)}
            </h2>
            <p className="modal-subtitle">Model {session.model}</p>
          </div>
          <button
            className="icon-button"
            type="button"
            onClick={onClose}
            aria-label="Close session details"
          >
            ×
          </button>
        </header>
        <div className="modal-body">
          <div className="session-details-grid">
            <div>
              <span className="label">Session ID</span>
              <div className="session-detail-id">
                <span className="mono">{session.session_id}</span>
                <button
                  className="icon-button icon-button-ghost small"
                  type="button"
                  onClick={() => onCopySessionId(session.session_id)}
                  aria-label="Copy full session id"
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
            </div>
            <div>
              <span className="label">Started</span>
              <div className="session-detail-value">{formatDateTime(session.session_start)}</div>
            </div>
            <div>
              <span className="label">Last Seen</span>
              <div className="session-detail-value">{formatDateTime(session.last_seen)}</div>
            </div>
            <div>
              <span className="label">Context Used</span>
              <div className="session-detail-value tabular-nums">
                {formatNumber(session.context_used)}
              </div>
            </div>
            <div>
              <span className="label">Context Window</span>
              <div className="session-detail-value tabular-nums">
                {formatNumber(session.context_window)}
              </div>
            </div>
            <div>
              <span className="label">Pressure</span>
              <div className="session-detail-value tabular-nums">
                {Math.round(
                  session.context_window > 0
                    ? (session.context_used / session.context_window) * 100
                    : 0
                )}
                %
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
