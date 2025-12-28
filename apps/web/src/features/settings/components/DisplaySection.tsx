type DisplaySectionProps = {
  activeMinutesInput: string;
  onActiveMinutesInputChange: (value: string) => void;
  onSaveActiveMinutes: () => void;
  settingsStatus: string;
};

export function DisplaySection({
  activeMinutesInput,
  onActiveMinutesInputChange,
  onSaveActiveMinutes,
  settingsStatus
}: DisplaySectionProps) {
  return (
    <section id="settings-display" className="panel settings-section">
      <div className="panel-header">
        <div>
          <h2>Active Window</h2>
          <p>Control what counts as active sessions.</p>
        </div>
      </div>
      <label className="label">Active Session Window (Minutes)</label>
      <input
        className="input"
        type="number"
        min="1"
        value={activeMinutesInput}
        onChange={(event) => onActiveMinutesInputChange(event.target.value)}
      />
      <div className="row">
        <button className="button" onClick={onSaveActiveMinutes}>
          Save Window
        </button>
        <span className="status" role="status" aria-live="polite">
          {settingsStatus}
        </span>
      </div>
      <div className="note">Updates the Active Sessions panel and refresh cycle.</div>
    </section>
  );
}
