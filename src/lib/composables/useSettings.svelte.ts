/**
 * Settings composable - manages provider selection, credentials, general settings, and notifications
 */

import { appStore, autostart, commands } from "$lib/electrobunClient";
import { cleanupOldData } from "$lib/historyStorage";
import type { NotificationSettings, ProviderKind, ProviderStatus } from "$lib/types";
import {
  PROVIDER_LABELS,
  getDefaultNotificationSettings,
  normalizeNotificationSettings,
} from "$lib/types";
import { debounce } from "$lib/utils";

export interface SettingsCallbacks {
  onSuccess?: (message: string) => void;
  onError?: (message: string) => void;
}

function emptyProviderStatuses(): Record<ProviderKind, ProviderStatus> {
  return {
    claude: {
      provider: "claude",
      configured: false,
      source: "keychain",
      message: "Add your Claude organization ID and session token.",
    },
    codex: {
      provider: "codex",
      configured: false,
      source: "auth-json",
      message: "Run `codex login` to enable Codex monitoring.",
    },
    ollama: {
      provider: "ollama",
      configured: false,
      source: "keychain",
      message: "Add your Ollama session cookie to enable monitoring.",
    },
  };
}

export function useSettings(callbacks: SettingsCallbacks = {}) {
  const { onSuccess, onError } = callbacks;
  const store = appStore;

  let showSettings = $state(false);
  let settingsTab: "account" | "notifications" | "general" | "updates" =
    $state("account");

  let activeProvider: ProviderKind = $state("claude");
  let providerStatuses: Record<ProviderKind, ProviderStatus> = $state(
    emptyProviderStatuses(),
  );

  let orgIdInput = $state("");
  let tokenInput = $state("");
  let ollamaTokenInput = $state("");

  let refreshIntervalMinutes = $state(5);
  let autoRefreshEnabled = $state(true);
  let hourlyRefreshEnabled = $state(false);
  let autostartEnabled = $state(false);
  let dataRetentionDays = $state(30);
  let notificationSettings: NotificationSettings = $state(
    getDefaultNotificationSettings(),
  );

  let loading = $state(false);
  let error = $state<string | null>(null);

  async function refreshProviderStatuses() {
    try {
      const result = await commands.getProviderStatuses();
      if (result.status === "ok") {
        providerStatuses = result.data.reduce(
          (acc, status) => {
            acc[status.provider] = status;
            return acc;
          },
          emptyProviderStatuses(),
        );
      }
    } catch (e) {
      console.error("Failed to load provider statuses:", e);
    }
  }

  function isAuthExpiredError() {
    if (!error) {
      return false;
    }

    const normalized = error.toLowerCase();
    return normalized.includes("expired") || normalized.includes("authentication");
  }

  async function init() {
    const savedProvider = await store.get<ProviderKind>("active_provider");
    activeProvider = savedProvider ?? "claude";

    const savedInterval = await store.get<number>("refresh_interval_minutes");
    const savedAutoRefresh = await store.get<boolean>("auto_refresh_enabled");
    const savedHourlyRefresh = await store.get<boolean>("hourly_refresh_enabled");
    const savedNotificationSettings = await store.get<unknown>(
      "notification_settings",
    );
    const savedRetention = await store.get<number>("data_retention_days");

    refreshIntervalMinutes = savedInterval ?? 5;
    autoRefreshEnabled = savedAutoRefresh ?? true;
    hourlyRefreshEnabled = savedHourlyRefresh ?? false;
    notificationSettings = normalizeNotificationSettings(savedNotificationSettings);
    dataRetentionDays = savedRetention ?? 30;

    if (savedNotificationSettings) {
      await store.set("notification_settings", notificationSettings);
    }

    await refreshProviderStatuses();

    const syncResults = await Promise.all([
      commands.setActiveProvider(activeProvider),
      commands.setNotificationSettings(notificationSettings),
      commands.setAutoRefresh(autoRefreshEnabled, refreshIntervalMinutes),
      commands.setHourlyRefresh(hourlyRefreshEnabled),
    ]);

    if (syncResults.some((result) => result.status === "error")) {
      console.error("Failed to sync settings to backend:", syncResults);
    }

    try {
      autostartEnabled = await autostart.isEnabled();
    } catch {
      autostartEnabled = false;
    }

    try {
      await cleanupOldData(dataRetentionDays);
    } catch {
      // ignore cleanup errors
    }
  }

  async function setActiveProvider(nextProvider: ProviderKind) {
    activeProvider = nextProvider;
    error = null;

    try {
      await store.set("active_provider", nextProvider);
      const result = await commands.setActiveProvider(nextProvider);
      if (result.status === "error") {
        throw new Error(result.error ?? "Failed to switch provider");
      }
      await refreshProviderStatuses();
      onSuccess?.(`${PROVIDER_LABELS[nextProvider]} selected`);
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Failed to switch provider";
      onError?.(msg);
    }
  }

  async function saveCredentials() {
    loading = true;
    error = null;

    const result = await commands.saveCredentials(orgIdInput, tokenInput);
    if (result.status === "error") {
      error = result.error;
      loading = false;
      onError?.(error);
      return;
    }

    orgIdInput = "";
    tokenInput = "";
    await refreshProviderStatuses();
    showSettings = false;
    loading = false;
    onSuccess?.("Claude credentials saved");
  }

  async function saveOllamaCredentials() {
    loading = true;
    error = null;

    const result = await commands.saveOllamaCredentials(ollamaTokenInput);
    if (result.status === "error") {
      error = result.error;
      loading = false;
      onError?.(error);
      return;
    }

    ollamaTokenInput = "";
    await refreshProviderStatuses();
    showSettings = false;
    loading = false;
    onSuccess?.("Ollama credentials saved");
  }

  async function logoutOllama() {
    const result = await commands.clearOllamaCredentials();
    if (result.status === "error") {
      onError?.(result.error);
      return;
    }

    ollamaTokenInput = "";
    error = null;
    showSettings = false;
    await refreshProviderStatuses();
    onSuccess?.("Ollama credentials cleared");
  }

  async function persistNotifications(newSettings: NotificationSettings) {
    try {
      await store.set("notification_settings", newSettings);
      const result = await commands.setNotificationSettings(newSettings);
      if (result.status === "error") {
        throw new Error(result.error ?? "Failed to save notification settings");
      }
      onSuccess?.("Notification settings saved");
    } catch (e) {
      onError?.(e instanceof Error ? e.message : "Failed to save notification settings");
    }
  }

  const debouncedPersistNotifications = debounce(persistNotifications, 1000);

  function saveNotifications(newSettings: NotificationSettings) {
    notificationSettings = newSettings;
    debouncedPersistNotifications(newSettings);
  }

  async function persistGeneral(enabled: boolean, intervalMinutes: number) {
    try {
      await store.set("auto_refresh_enabled", enabled);
      await store.set("refresh_interval_minutes", intervalMinutes);
      const result = await commands.setAutoRefresh(enabled, intervalMinutes);
      if (result.status === "error") {
        throw new Error(result.error ?? "Failed to save settings");
      }
      onSuccess?.("Settings saved");
    } catch (e) {
      onError?.(e instanceof Error ? e.message : "Failed to save settings");
    }
  }

  const debouncedPersistGeneral = debounce(persistGeneral, 1000);

  function saveGeneral(enabled: boolean, intervalMinutes: number) {
    autoRefreshEnabled = enabled;
    refreshIntervalMinutes = intervalMinutes;
    debouncedPersistGeneral(enabled, intervalMinutes);
  }

  async function toggleAutostart(enabled: boolean) {
    try {
      if (enabled) {
        await autostart.enable();
      } else {
        await autostart.disable();
      }
      autostartEnabled = enabled;
      onSuccess?.(enabled ? "Autostart enabled" : "Autostart disabled");
    } catch (e) {
      onError?.(e instanceof Error ? e.message : "Failed to toggle autostart");
    }
  }

  async function toggleHourlyRefresh(enabled: boolean) {
    try {
      await store.set("hourly_refresh_enabled", enabled);
      const result = await commands.setHourlyRefresh(enabled);
      if (result.status === "error") {
        throw new Error(result.error ?? "Failed to save hourly refresh setting");
      }
      hourlyRefreshEnabled = enabled;
      onSuccess?.(enabled ? "Hourly refresh enabled" : "Hourly refresh disabled");
    } catch (e) {
      onError?.(e instanceof Error ? e.message : "Failed to save hourly refresh setting");
    }
  }

  async function persistRetention(days: number) {
    try {
      await store.set("data_retention_days", days);
      await cleanupOldData(days);
      onSuccess?.("Data retention updated");
    } catch (e) {
      onError?.(e instanceof Error ? e.message : "Failed to save retention settings");
    }
  }

  const debouncedPersistRetention = debounce(persistRetention, 1000);

  function saveRetention(days: number) {
    dataRetentionDays = days;
    debouncedPersistRetention(days);
  }

  async function logout() {
    const result = await commands.clearCredentials();
    if (result.status === "error") {
      onError?.(result.error);
      return;
    }

    orgIdInput = "";
    tokenInput = "";
    error = null;
    showSettings = false;
    await refreshProviderStatuses();
    onSuccess?.("Claude credentials cleared");
  }

  async function resetAll() {
    const results = await Promise.all([
      commands.clearCredentials(),
      commands.clearOllamaCredentials(),
    ]);
    if (results.some((r) => r.status === "error")) {
      const errResult = results.find((r) => r.status === "error");
      if (errResult && errResult.status === "error") {
        onError?.(errResult.error);
      }
      return;
    }

    await store.clear();
    activeProvider = "claude";
    refreshIntervalMinutes = 5;
    autoRefreshEnabled = true;
    hourlyRefreshEnabled = false;
    dataRetentionDays = 30;
    orgIdInput = "";
    tokenInput = "";
    notificationSettings = getDefaultNotificationSettings();
    showSettings = false;
    error = null;

    await commands.setActiveProvider("claude");
    await commands.setNotificationSettings(notificationSettings);
    await commands.setAutoRefresh(true, 5);
    await commands.setHourlyRefresh(false);
    await refreshProviderStatuses();
    onSuccess?.("All settings reset");
  }

  function open() {
    showSettings = true;
  }

  function close() {
    showSettings = false;
  }

  function toggle() {
    showSettings = !showSettings;
  }

  function openCredentials() {
    settingsTab = "account";
    showSettings = true;
    error = null;
  }

  return {
    store,
    get showSettings() {
      return showSettings;
    },
    set showSettings(value: boolean) {
      showSettings = value;
    },
    get settingsTab() {
      return settingsTab;
    },
    set settingsTab(value: "account" | "notifications" | "general" | "updates") {
      settingsTab = value;
    },
    get activeProvider() {
      return activeProvider;
    },
    get providerStatuses() {
      return providerStatuses;
    },
    get isConfigured() {
      return providerStatuses[activeProvider].configured;
    },
    get activeProviderStatus() {
      return providerStatuses[activeProvider];
    },
    get orgIdInput() {
      return orgIdInput;
    },
    set orgIdInput(value: string) {
      orgIdInput = value;
    },
    get tokenInput() {
      return tokenInput;
    },
    set tokenInput(value: string) {
      tokenInput = value;
    },
    get ollamaTokenInput() {
      return ollamaTokenInput;
    },
    set ollamaTokenInput(value: string) {
      ollamaTokenInput = value;
    },
    get refreshIntervalMinutes() {
      return refreshIntervalMinutes;
    },
    get autoRefreshEnabled() {
      return autoRefreshEnabled;
    },
    get hourlyRefreshEnabled() {
      return hourlyRefreshEnabled;
    },
    get autostartEnabled() {
      return autostartEnabled;
    },
    get dataRetentionDays() {
      return dataRetentionDays;
    },
    get notificationSettings() {
      return notificationSettings;
    },
    get loading() {
      return loading;
    },
    set loading(value: boolean) {
      loading = value;
    },
    get error() {
      return error;
    },
    set error(value: string | null) {
      error = value;
    },
    get isAuthExpired() {
      return isAuthExpiredError();
    },
    init,
    refreshProviderStatuses,
    setActiveProvider,
    saveCredentials,
    saveOllamaCredentials,
    saveNotifications,
    saveGeneral,
    toggleAutostart,
    toggleHourlyRefresh,
    saveRetention,
    logout,
    logoutOllama,
    resetAll,
    open,
    close,
    toggle,
    openCredentials,
  };
}
