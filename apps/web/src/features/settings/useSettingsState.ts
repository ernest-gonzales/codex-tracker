import { useCallback, useEffect, useMemo, useState } from "react";
import type { CodexHome, PricingRule, PricingRuleApi } from "../../domain/types";
import {
  clearHomeData,
  createHome,
  deleteHome,
  getSettings,
  listHomes,
  listPricing,
  recomputePricing,
  replacePricing,
  setActiveHome,
  updateSettings
} from "../../data/codexApi";
import type { SettingsTabValue } from "../../shared/constants";
import { STORAGE_KEYS } from "../../shared/constants";
import { formatNumber } from "../../shared/formatters";
import { safeStorageGet, safeStorageSet } from "../../shared/storage";
import { validatePricingRules } from "../../shared/validation";
import type { ToastMessage } from "../shared/Toast";

type StorageInfo = {
  dbPath?: string;
  pricingDefaultsPath?: string;
  appDataDir?: string;
  legacyBackupDir?: string | null;
};

type SettingsStateOptions = {
  onToast?: (toast: ToastMessage) => void;
  onDashboardRefresh?: () => void;
};

export function useSettingsState({ onToast, onDashboardRefresh }: SettingsStateOptions) {
  const [settingsTab, setSettingsTab] = useState<SettingsTabValue>("settings-homes");
  const [homes, setHomes] = useState<CodexHome[]>([]);
  const [activeHomeId, setActiveHomeId] = useState<number | null>(null);
  const [newHomePath, setNewHomePath] = useState("");
  const [newHomeLabel, setNewHomeLabel] = useState("");
  const [homeStatus, setHomeStatus] = useState("");
  const [dangerStatus, setDangerStatus] = useState("");
  const [deleteConfirm, setDeleteConfirm] = useState("");
  const [pricingRules, setPricingRules] = useState<PricingRule[]>([]);
  const [pricingStatus, setPricingStatus] = useState("");
  const [settingsStatus, setSettingsStatus] = useState("");
  const [pricingDirty, setPricingDirty] = useState(false);
  const [pricingFilter, setPricingFilter] = useState("");
  const [pricingBusy, setPricingBusy] = useState(false);
  const [pricingLastRecompute, setPricingLastRecompute] = useState<string | null>(null);
  const [storageInfo, setStorageInfo] = useState<StorageInfo | null>(null);
  const [activeMinutes, setActiveMinutes] = useState(60);
  const [activeMinutesInput, setActiveMinutesInput] = useState("60");

  const deleteReady = deleteConfirm.trim().toLowerCase() === "delete";

  const pricingIssues = useMemo(() => validatePricingRules(pricingRules), [pricingRules]);
  const pricingIssueMap = useMemo(() => {
    const map = new Map<number, ReturnType<typeof validatePricingRules>>();
    pricingIssues.forEach((issue) => {
      const list = map.get(issue.index) ?? [];
      list.push(issue);
      map.set(issue.index, list);
    });
    return map;
  }, [pricingIssues]);
  const pricingHasIssues = pricingIssues.length > 0;
  const pricingRows = useMemo(() => {
    const rows = pricingRules.map((rule, index) => ({ rule, index }));
    if (!pricingFilter.trim()) {
      return rows;
    }
    const needle = pricingFilter.trim().toLowerCase();
    return rows.filter(({ rule }) => rule.model_pattern.toLowerCase().includes(needle));
  }, [pricingRules, pricingFilter]);

  const activeHome = useMemo(() => {
    if (activeHomeId === null) {
      return null;
    }
    return homes.find((home) => home.id === activeHomeId) ?? null;
  }, [homes, activeHomeId]);

  const homeSelectOptions = useMemo(() => {
    if (homes.length === 0) {
      return [{ value: "none", label: "No homes found", disabled: true }];
    }
    return homes.map((home) => ({
      value: String(home.id),
      label: home.label || home.path
    }));
  }, [homes]);

  const refreshPricing = useCallback(async () => {
    try {
      const pricingData = await listPricing();
      const normalized = (pricingData || []).map((rule: PricingRuleApi) => {
        if (
          rule.input_per_1m ||
          rule.cached_input_per_1m ||
          rule.output_per_1m ||
          (!rule.input_per_1k && !rule.cached_input_per_1k && !rule.output_per_1k)
        ) {
          return rule;
        }
        return {
          ...rule,
          input_per_1m: (rule.input_per_1k ?? 0) * 1000,
          cached_input_per_1m: (rule.cached_input_per_1k ?? 0) * 1000,
          output_per_1m: (rule.output_per_1k ?? 0) * 1000
        };
      });
      setPricingRules(normalized);
      setPricingDirty(false);
    } catch (err) {
      onToast?.({
        message: err instanceof Error ? err.message : "Failed to load pricing",
        tone: "error"
      });
    }
  }, [onToast]);

  const refreshHomes = useCallback(async () => {
    try {
      const data = await listHomes();
      setHomes(data.homes || []);
      setActiveHomeId(data.active_home_id ?? null);
    } catch (err) {
      onToast?.({
        message: err instanceof Error ? err.message : "Failed to load homes",
        tone: "error"
      });
    }
  }, [onToast]);

  const refreshSettings = useCallback(async () => {
    try {
      const data = await getSettings();
      const minutes = data.context_active_minutes ?? 60;
      setActiveMinutes(minutes);
      setActiveMinutesInput(minutes.toString());
      setStorageInfo({
        dbPath: data.db_path,
        pricingDefaultsPath: data.pricing_defaults_path,
        appDataDir: data.app_data_dir,
        legacyBackupDir: data.legacy_backup_dir ?? null
      });
    } catch (err) {
      onToast?.({
        message: err instanceof Error ? err.message : "Failed to load settings",
        tone: "error"
      });
    }
  }, [onToast]);

  useEffect(() => {
    refreshPricing();
    refreshHomes();
    refreshSettings();
  }, [refreshHomes, refreshPricing, refreshSettings]);

  useEffect(() => {
    const storedTab = safeStorageGet(STORAGE_KEYS.settingsTab);
    if (
      storedTab &&
      ["settings-homes", "settings-display", "settings-storage", "settings-pricing"].includes(
        storedTab
      )
    ) {
      setSettingsTab(storedTab as SettingsTabValue);
    }
  }, []);

  useEffect(() => {
    safeStorageSet(STORAGE_KEYS.settingsTab, settingsTab);
  }, [settingsTab]);

  async function validateHomePath(path: string) {
    try {
      const { exists } = await import("@tauri-apps/plugin-fs");
      return await exists(path);
    } catch (err) {
      onToast?.({
        message: err instanceof Error ? err.message : "Path validation unavailable",
        tone: "info"
      });
      return true;
    }
  }

  async function handlePickHomePath() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected === "string") {
        setNewHomePath(selected);
      }
    } catch (err) {
      setHomeStatus(err instanceof Error ? err.message : "Picker failed");
    }
  }

  async function handleAddHome() {
    const path = newHomePath.trim();
    if (!path) {
      setHomeStatus("Path required");
      return;
    }
    const exists = await validateHomePath(path);
    if (!exists) {
      setHomeStatus("Path does not exist");
      return;
    }
    setHomeStatus("Adding...");
    try {
      const created = await createHome({
        path,
        label: newHomeLabel.trim().length ? newHomeLabel.trim() : undefined
      });
      setActiveHomeId(created.id);
      setNewHomePath("");
      setNewHomeLabel("");
      await refreshHomes();
      onDashboardRefresh?.();
      setHomeStatus("Added");
    } catch (err) {
      setHomeStatus(err instanceof Error ? err.message : "Add failed");
    }
  }

  async function handleSetActiveHome(nextId: number) {
    if (activeHomeId === nextId) {
      return;
    }
    setHomeStatus("Switching...");
    try {
      const updated = await setActiveHome(nextId);
      setActiveHomeId(updated.id);
      await refreshHomes();
      onDashboardRefresh?.();
      setHomeStatus("Active");
    } catch (err) {
      setHomeStatus(err instanceof Error ? err.message : "Switch failed");
    }
  }

  async function handleDeleteHome(homeId: number) {
    if (!window.confirm("Delete this home and all its data?")) {
      return;
    }
    setHomeStatus("Deleting...");
    try {
      await deleteHome(homeId);
      await refreshHomes();
      onDashboardRefresh?.();
      setHomeStatus("Deleted");
    } catch (err) {
      setHomeStatus(err instanceof Error ? err.message : "Delete failed");
    }
  }

  async function handleDeleteData() {
    if (!activeHomeId) {
      setDangerStatus("Select a home first");
      return;
    }
    if (!deleteReady) {
      setDangerStatus('Type "DELETE" to confirm');
      return;
    }
    setDangerStatus("Deleting data...");
    try {
      await clearHomeData(activeHomeId);
      onDashboardRefresh?.();
      setDangerStatus("Data deleted");
      setDeleteConfirm("");
    } catch (err) {
      setDangerStatus(err instanceof Error ? err.message : "Delete failed");
    }
  }

  async function handleSavePricing() {
    if (pricingHasIssues) {
      setPricingStatus("Fix validation issues before saving");
      return;
    }
    setPricingStatus("Saving...");
    setPricingBusy(true);
    try {
      await replacePricing(pricingRules);
      setPricingStatus("Saved");
      setPricingDirty(false);
      onDashboardRefresh?.();
    } catch (err) {
      setPricingStatus(err instanceof Error ? err.message : "Save failed");
    } finally {
      setPricingBusy(false);
    }
  }

  async function handleRecomputeCosts() {
    setPricingStatus("Recomputing...");
    setPricingBusy(true);
    try {
      const result = await recomputePricing();
      setPricingStatus(`Recomputed ${formatNumber(result.updated)} rows`);
      setPricingLastRecompute(new Date().toISOString());
      onDashboardRefresh?.();
    } catch (err) {
      setPricingStatus(err instanceof Error ? err.message : "Recompute failed");
    } finally {
      setPricingBusy(false);
    }
  }

  async function handleSaveActiveMinutes() {
    const parsed = Number(activeMinutesInput);
    if (!Number.isFinite(parsed) || parsed <= 0) {
      setSettingsStatus("Enter a valid minute value");
      return;
    }
    setSettingsStatus("Saving...");
    try {
      const data = await updateSettings({ context_active_minutes: parsed });
      const minutes = data.context_active_minutes ?? parsed;
      setActiveMinutes(minutes);
      setActiveMinutesInput(minutes.toString());
      setSettingsStatus("Saved");
      setStorageInfo({
        dbPath: data.db_path,
        pricingDefaultsPath: data.pricing_defaults_path,
        appDataDir: data.app_data_dir,
        legacyBackupDir: data.legacy_backup_dir ?? null
      });
      onDashboardRefresh?.();
    } catch (err) {
      setSettingsStatus(err instanceof Error ? err.message : "Save failed");
    }
  }

  async function handleCopyPath(value?: string) {
    if (!value) {
      return;
    }
    try {
      await navigator.clipboard.writeText(value);
      onToast?.({ message: "Path copied to clipboard", tone: "info" });
    } catch (err) {
      onToast?.({
        message: err instanceof Error ? err.message : "Copy failed",
        tone: "error"
      });
    }
  }

  async function handleRevealPath(value?: string, isDir = false) {
    if (!value) {
      return;
    }
    try {
      if (isDir) {
        const { openPath } = await import("@tauri-apps/plugin-opener");
        await openPath(value);
      } else {
        const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
        await revealItemInDir(value);
      }
    } catch (err) {
      onToast?.({
        message: err instanceof Error ? err.message : "Reveal failed",
        tone: "error"
      });
    }
  }

  function updatePricingRule(index: number, patch: Partial<PricingRule>) {
    setPricingRules((prev) =>
      prev.map((rule, idx) => (idx === index ? { ...rule, ...patch } : rule))
    );
    setPricingDirty(true);
  }

  function addPricingRule() {
    setPricingRules((prev) => [
      ...prev,
      {
        model_pattern: "*",
        input_per_1m: 0,
        cached_input_per_1m: 0,
        output_per_1m: 0,
        effective_from: new Date().toISOString(),
        effective_to: null
      }
    ]);
    setPricingDirty(true);
  }

  function duplicatePricingRule(index: number) {
    setPricingRules((prev) => {
      const rule = prev[index];
      if (!rule) {
        return prev;
      }
      const clone = { ...rule, id: undefined };
      const next = [...prev];
      next.splice(index + 1, 0, clone);
      return next;
    });
    setPricingDirty(true);
  }

  function deletePricingRule(index: number) {
    setPricingRules((prev) => prev.filter((_, idx) => idx !== index));
    setPricingDirty(true);
  }

  return {
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
    pricingRules,
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
    activeMinutes,
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
  };
}

export type SettingsState = ReturnType<typeof useSettingsState>;
