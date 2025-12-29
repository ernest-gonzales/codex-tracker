type StorageInfo = {
  dbPath?: string;
  pricingDefaultsPath?: string;
  appDataDir?: string;
  legacyBackupDir?: string | null;
};

type StorageSectionProps = {
  storageInfo: StorageInfo | null;
  onCopyPath: (value?: string) => void;
  onRevealPath: (value?: string, isDir?: boolean) => void;
  revealAvailable: boolean;
};

export function StorageSection({
  storageInfo,
  onCopyPath,
  onRevealPath,
  revealAvailable
}: StorageSectionProps) {
  return (
    <section id="settings-storage" className="panel settings-section">
      <div className="panel-header">
        <div>
          <h2>Storage</h2>
          <p>Local app data directory for the desktop client.</p>
        </div>
      </div>
      <div className="settings-kv">
        <div className="settings-kv-row">
          <span className="settings-kv-key">App Data</span>
          <div className="settings-kv-value">
            <span className="mono">{storageInfo?.appDataDir ?? "—"}</span>
            <div className="kv-actions">
              <button
                className="button ghost small"
                type="button"
                onClick={() => onCopyPath(storageInfo?.appDataDir)}
                disabled={!storageInfo?.appDataDir}
              >
                Copy
              </button>
              <button
                className="button ghost small"
                type="button"
                onClick={() => onRevealPath(storageInfo?.appDataDir, true)}
                disabled={!storageInfo?.appDataDir || !revealAvailable}
              >
                Reveal
              </button>
            </div>
          </div>
        </div>
      </div>
      <div className="note">Desktop builds keep all data in this directory.</div>
    </section>
  );
}
