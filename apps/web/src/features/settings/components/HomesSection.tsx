import type { SelectOption } from "../../../components/Select";
import { SelectField } from "../../../components/Select";
import type { CodexHome } from "../../../domain/types";

type HomesSectionProps = {
  homes: CodexHome[];
  activeHomeId: number | null;
  activeHome: CodexHome | null;
  homeSelectOptions: SelectOption[];
  newHomePath: string;
  newHomeLabel: string;
  homeStatus: string;
  deleteConfirm: string;
  deleteReady: boolean;
  dangerStatus: string;
  tauriAvailable: boolean;
  onRefreshHomes: () => void;
  onSetActiveHome: (id: number) => void;
  onDeleteHome: (id: number) => void;
  onPickHomePath: () => void;
  onAddHome: () => void;
  onDeleteData: () => void;
  onNewHomePathChange: (value: string) => void;
  onNewHomeLabelChange: (value: string) => void;
  onDeleteConfirmChange: (value: string) => void;
};

export function HomesSection({
  homes,
  activeHomeId,
  activeHome,
  homeSelectOptions,
  newHomePath,
  newHomeLabel,
  homeStatus,
  deleteConfirm,
  deleteReady,
  dangerStatus,
  tauriAvailable,
  onRefreshHomes,
  onSetActiveHome,
  onDeleteHome,
  onPickHomePath,
  onAddHome,
  onDeleteData,
  onNewHomePathChange,
  onNewHomeLabelChange,
  onDeleteConfirmChange
}: HomesSectionProps) {
  return (
    <section id="settings-homes" className="panel settings-section">
      <div className="panel-header">
        <div>
          <h2>Codex Homes</h2>
          <p>Switch between tracked log directories.</p>
        </div>
        <div className="panel-actions">
          <button className="button ghost small" onClick={onRefreshHomes}>
            Reload
          </button>
        </div>
      </div>
      <label className="label">Active Home</label>
      <SelectField
        value={activeHomeId ? String(activeHomeId) : undefined}
        onValueChange={(value) => onSetActiveHome(Number(value))}
        options={homeSelectOptions}
        placeholder="Select a home"
        disabled={homes.length === 0}
      />
      <div className="note">{activeHome ? `Path: ${activeHome.path}` : "Select a home to see details."}</div>
      {homes.length > 0 && (
        <div className="table-wrap">
          <table className="compact-table">
            <thead>
              <tr>
                <th>Label</th>
                <th>Path</th>
                <th>Last Seen</th>
                <th>Status</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {homes.map((home) => (
                <tr key={home.id} className={home.id === activeHomeId ? "active-row" : undefined}>
                  <td>{home.label || "—"}</td>
                  <td>
                    <span className="mono">{home.path}</span>
                  </td>
                  <td>{home.last_seen_at ? new Date(home.last_seen_at).toLocaleString() : "-"}</td>
                  <td>{home.id === activeHomeId ? <span className="badge">Active</span> : "—"}</td>
                  <td className="table-actions">
                    {home.id !== activeHomeId && (
                      <button className="button ghost small" onClick={() => onSetActiveHome(home.id)}>
                        Make Active
                      </button>
                    )}
                    <button
                      className="button ghost small"
                      onClick={() => onDeleteHome(home.id)}
                      disabled={homes.length === 1}
                    >
                      Delete
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <div className="danger-zone">
        <div className="danger-zone-header">
          <div>
            <h3>Danger Zone</h3>
            <p>Delete all ingested data for the active home.</p>
          </div>
        </div>
        <label className="label">Type DELETE to confirm</label>
        <input
          className="input"
          value={deleteConfirm}
          onChange={(event) => onDeleteConfirmChange(event.target.value)}
          placeholder="DELETE"
        />
        <div className="row">
          <button
            className="button danger"
            onClick={onDeleteData}
            disabled={!activeHomeId || !deleteReady}
          >
            Delete Ingested Data
          </button>
          <span className="status" role="status" aria-live="polite">
            {dangerStatus}
          </span>
        </div>
      </div>
      <label className="label">Add Home</label>
      <div className="input-row">
        <input
          className="input"
          value={newHomePath}
          onChange={(event) => onNewHomePathChange(event.target.value)}
          placeholder="/Users/you/.codex"
        />
        <button
          className="button ghost small"
          type="button"
          onClick={onPickHomePath}
          disabled={!tauriAvailable}
        >
          Browse
        </button>
      </div>
      <input
        className="input"
        value={newHomeLabel}
        onChange={(event) => onNewHomeLabelChange(event.target.value)}
        placeholder="Label (optional)"
      />
      <div className="row">
        <button className="button" onClick={onAddHome} disabled={!newHomePath.trim()}>
          Add Home
        </button>
        <span className="status" role="status" aria-live="polite">
          {homeStatus}
        </span>
      </div>
    </section>
  );
}
