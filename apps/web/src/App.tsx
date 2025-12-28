import { useCallback, useEffect, useState } from "react";
import { DashboardPage } from "./features/dashboard/DashboardPage";
import { SettingsPage } from "./features/settings/SettingsPage";
import { useSettingsState } from "./features/settings/useSettingsState";
import { Toast, type ToastMessage } from "./features/shared/Toast";

export default function App() {
  const [toast, setToast] = useState<ToastMessage | null>(null);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [dashboardRefreshToken, setDashboardRefreshToken] = useState(0);

  const handleToast = useCallback((message: ToastMessage) => {
    setToast(message);
  }, []);

  const requestDashboardRefresh = useCallback(() => {
    setDashboardRefreshToken((prev) => prev + 1);
  }, []);

  const settingsState = useSettingsState({
    onToast: handleToast,
    onDashboardRefresh: requestDashboardRefresh
  });

  useEffect(() => {
    if (!toast) {
      return;
    }
    const timeout = window.setTimeout(() => setToast(null), 4500);
    return () => window.clearTimeout(timeout);
  }, [toast]);

  return (
    <div className="app density-compact">
      <div className="glow" aria-hidden="true" />
      {toast && <Toast toast={toast} onDismiss={() => setToast(null)} />}
      {isSettingsOpen ? (
        <SettingsPage
          state={settingsState}
          isOpen={isSettingsOpen}
          onClose={() => setIsSettingsOpen(false)}
        />
      ) : (
        <DashboardPage
          activeMinutes={settingsState.activeMinutes}
          refreshToken={dashboardRefreshToken}
          onOpenSettings={() => setIsSettingsOpen(true)}
          onToast={handleToast}
        />
      )}
    </div>
  );
}
