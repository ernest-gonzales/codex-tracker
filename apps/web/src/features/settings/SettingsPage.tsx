import { useEffect } from "react";
import type { SettingsState } from "./useSettingsState";
import { HomesSection } from "./components/HomesSection";
import { DisplaySection } from "./components/DisplaySection";
import { StorageSection } from "./components/StorageSection";
import { PricingSection } from "./components/PricingSection";

type SettingsPageProps = {
  state: SettingsState;
  isOpen: boolean;
  onClose: () => void;
};

export function SettingsPage({ state, isOpen, onClose }: SettingsPageProps) {
  const {
    settingsTab,
    setSettingsTab,
    homes,
    activeHomeId,
    activeHome,
    homeSelectOptions,
    newHomePath,
    newHomeLabel,
    homeStatus,
    dangerStatus,
    deleteConfirm,
    deleteReady,
    pricingStatus,
    settingsStatus,
    pricingDirty,
    pricingFilter,
    pricingBusy,
    pricingLastRecompute,
    pricingHasIssues,
    pricingIssueMap,
    pricingRows,
    storageInfo,
    tauriAvailable,
    activeMinutesInput,
    setActiveMinutesInput,
    setNewHomeLabel,
    setNewHomePath,
    setDeleteConfirm,
    setPricingFilter,
    refreshHomes,
    handlePickHomePath,
    handleAddHome,
    handleSetActiveHome,
    handleDeleteHome,
    handleDeleteData,
    handleSavePricing,
    handleRecomputeCosts,
    handleSaveActiveMinutes,
    handleCopyPath,
    handleRevealPath,
    updatePricingRule,
    addPricingRule,
    duplicatePricingRule,
    deletePricingRule
  } = state;

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    const target = document.getElementById(settingsTab);
    if (target) {
      target.scrollIntoView({ block: "start", behavior: "smooth" });
    }
  }, [isOpen, settingsTab]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose]);

  return (
    <section className="settings-page">
      <header className="settings-header">
        <div>
          <h1 className="settings-title">Settings</h1>
          <p className="settings-subtitle">
            Codex homes, active window, pricing rules, and storage paths.
          </p>
        </div>
        <button className="button ghost" type="button" onClick={onClose}>
          Back to Dashboard
        </button>
      </header>
      <div className="settings-layout">
        <nav className="settings-nav" aria-label="Settings sections">
          <a
            href="#settings-homes"
            className={settingsTab === "settings-homes" ? "active" : undefined}
            aria-current={settingsTab === "settings-homes" ? "page" : undefined}
            onClick={() => setSettingsTab("settings-homes")}
          >
            Homes
          </a>
          <a
            href="#settings-display"
            className={settingsTab === "settings-display" ? "active" : undefined}
            aria-current={settingsTab === "settings-display" ? "page" : undefined}
            onClick={() => setSettingsTab("settings-display")}
          >
            Display
          </a>
          <a
            href="#settings-storage"
            className={settingsTab === "settings-storage" ? "active" : undefined}
            aria-current={settingsTab === "settings-storage" ? "page" : undefined}
            onClick={() => setSettingsTab("settings-storage")}
          >
            Storage
          </a>
          <a
            href="#settings-pricing"
            className={settingsTab === "settings-pricing" ? "active" : undefined}
            aria-current={settingsTab === "settings-pricing" ? "page" : undefined}
            onClick={() => setSettingsTab("settings-pricing")}
          >
            Pricing
          </a>
        </nav>
        <div className="settings-content">
          <HomesSection
            homes={homes}
            activeHomeId={activeHomeId}
            activeHome={activeHome}
            homeSelectOptions={homeSelectOptions}
            newHomePath={newHomePath}
            newHomeLabel={newHomeLabel}
            homeStatus={homeStatus}
            deleteConfirm={deleteConfirm}
            deleteReady={deleteReady}
            dangerStatus={dangerStatus}
            tauriAvailable={tauriAvailable}
            onRefreshHomes={refreshHomes}
            onSetActiveHome={handleSetActiveHome}
            onDeleteHome={handleDeleteHome}
            onPickHomePath={handlePickHomePath}
            onAddHome={handleAddHome}
            onDeleteData={handleDeleteData}
            onNewHomePathChange={setNewHomePath}
            onNewHomeLabelChange={setNewHomeLabel}
            onDeleteConfirmChange={setDeleteConfirm}
          />
          <DisplaySection
            activeMinutesInput={activeMinutesInput}
            onActiveMinutesInputChange={setActiveMinutesInput}
            onSaveActiveMinutes={handleSaveActiveMinutes}
            settingsStatus={settingsStatus}
          />
          <StorageSection
            storageInfo={storageInfo}
            onCopyPath={handleCopyPath}
            onRevealPath={handleRevealPath}
            revealAvailable={tauriAvailable}
          />
          <PricingSection
            pricingRows={pricingRows}
            pricingIssueMap={pricingIssueMap}
            pricingHasIssues={pricingHasIssues}
            pricingDirty={pricingDirty}
            pricingStatus={pricingStatus}
            pricingBusy={pricingBusy}
            pricingFilter={pricingFilter}
            pricingLastRecompute={pricingLastRecompute}
            onPricingFilterChange={setPricingFilter}
            onAddRule={addPricingRule}
            onSavePricing={handleSavePricing}
            onRecomputeCosts={handleRecomputeCosts}
            onUpdateRule={updatePricingRule}
            onDuplicateRule={duplicatePricingRule}
            onDeleteRule={deletePricingRule}
          />
        </div>
      </div>
    </section>
  );
}
