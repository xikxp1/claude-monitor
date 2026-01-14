/**
 * Settings composable - manages credentials, general settings, autostart, and notifications
 */

import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import { LazyStore } from "@tauri-apps/plugin-store";
import { commands } from "$lib/bindings.generated";
import { cleanupOldData } from "$lib/historyStorage";
import type { NotificationSettings } from "$lib/types";
import { getDefaultNotificationSettings } from "$lib/types";
import { debounce } from "$lib/utils";

export interface SettingsCallbacks {
  onSuccess?: (message: string) => void;
  onError?: (message: string) => void;
}

export function useSettings(callbacks: SettingsCallbacks = {}) {
  const { onSuccess, onError } = callbacks;
  const store = new LazyStore("settings.json", {
    autoSave: true,
    defaults: {},
  });

  // UI state
  let showSettings = $state(false);
  let settingsTab: "credentials" | "notifications" | "general" =
    $state("credentials");

  // Credentials state (form inputs only - actual credentials are server-side)
  let isConfigured = $state(false);
  let orgIdInput = $state("");
  let tokenInput = $state("");

  // General settings
  let refreshIntervalMinutes = $state(5);
  let autoRefreshEnabled = $state(true);
  let autostartEnabled = $state(false);
  let dataRetentionDays = $state(30);

  // Notification settings
  let notificationSettings: NotificationSettings = $state(
    getDefaultNotificationSettings(),
  );

  // Loading/error state
  let loading = $state(false);
  let error = $state<string | null>(null);

  // Helper to check if error is a session expiration
  function checkSessionExpired(): boolean {
    return error !== null && error.toLowerCase().includes("session expired");
  }

  /**
   * Initialize settings from store and backend
   */
  async function init() {
    // Check if configured from backend
    try {
      const result = await commands.getIsConfigured();
      isConfigured = result.status === "ok" ? result.data : false;
    } catch (e) {
      console.error("Failed to check configuration status:", e);
      isConfigured = false;
    }

    // Load general settings from store
    const savedInterval = await store.get<number>("refresh_interval_minutes");
    const savedAutoRefresh = await store.get<boolean>("auto_refresh_enabled");

    refreshIntervalMinutes = savedInterval ?? 5;
    autoRefreshEnabled = savedAutoRefresh ?? true;

    // Load notification settings
    const savedNotificationSettings = await store.get<NotificationSettings>(
      "notification_settings",
    );
    if (savedNotificationSettings) {
      notificationSettings = savedNotificationSettings;
    }

    // Sync notification settings to backend
    try {
      await commands.setNotificationSettings(notificationSettings);
    } catch (e) {
      console.error("Failed to sync notification settings to backend:", e);
    }

    // Load autostart state
    try {
      autostartEnabled = await isAutostartEnabled();
    } catch {
      autostartEnabled = false;
    }

    // Load data retention setting
    const savedRetention = await store.get<number>("data_retention_days");
    dataRetentionDays = savedRetention ?? 30;

    // Cleanup old data based on retention policy
    try {
      await cleanupOldData(dataRetentionDays);
    } catch {
      // Ignore cleanup errors - non-critical
    }

    // Send auto-refresh settings to backend
    try {
      await commands.setAutoRefresh(autoRefreshEnabled, refreshIntervalMinutes);
    } catch (e) {
      console.error("Failed to set auto-refresh settings:", e);
    }
  }

  /**
   * Save credentials to backend
   */
  async function saveCredentials() {
    const wasConfigured = isConfigured;
    loading = true;
    error = null;

    const result = await commands.saveCredentials(orgIdInput, tokenInput);

    if (result.status === "error") {
      error = result.error;
      loading = false;
      onError?.(error);
      return;
    }

    // Clear form inputs
    orgIdInput = "";
    tokenInput = "";

    // Update configured state
    const configResult = await commands.getIsConfigured();
    isConfigured = configResult.status === "ok" ? configResult.data : false;

    showSettings = false;

    // For initial setup, keep loading=true while waiting for first data fetch
    // The backend event handler will set loading=false when data arrives
    if (wasConfigured) {
      loading = false;
    }

    onSuccess?.("Credentials saved");
  }

  /**
   * Internal: persist notification settings (debounced)
   */
  async function persistNotifications(newSettings: NotificationSettings) {
    try {
      await store.set("notification_settings", newSettings);
      await commands.setNotificationSettings(newSettings);
      onSuccess?.("Notification settings saved");
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Failed to save notification settings";
      onError?.(msg);
    }
  }

  const debouncedPersistNotifications = debounce(persistNotifications, 1000);

  /**
   * Save notification settings (immediate UI update, debounced persistence)
   */
  function saveNotifications(newSettings: NotificationSettings) {
    notificationSettings = newSettings;
    debouncedPersistNotifications(newSettings);
  }

  /**
   * Internal: persist general settings (debounced)
   */
  async function persistGeneral(enabled: boolean, intervalMinutes: number) {
    try {
      await store.set("auto_refresh_enabled", enabled);
      await store.set("refresh_interval_minutes", intervalMinutes);
      await commands.setAutoRefresh(enabled, intervalMinutes);
      onSuccess?.("Settings saved");
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Failed to save settings";
      onError?.(msg);
    }
  }

  const debouncedPersistGeneral = debounce(persistGeneral, 1000);

  /**
   * Save general settings (immediate UI update, debounced persistence)
   */
  function saveGeneral(
    newAutoRefreshEnabled: boolean,
    newRefreshIntervalMinutes: number,
  ) {
    autoRefreshEnabled = newAutoRefreshEnabled;
    refreshIntervalMinutes = newRefreshIntervalMinutes;
    debouncedPersistGeneral(newAutoRefreshEnabled, newRefreshIntervalMinutes);
  }

  /**
   * Toggle autostart (not debounced - discrete action)
   */
  async function toggleAutostart(enabled: boolean) {
    try {
      if (enabled) {
        await enableAutostart();
      } else {
        await disableAutostart();
      }
      autostartEnabled = enabled;
      onSuccess?.(enabled ? "Autostart enabled" : "Autostart disabled");
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Failed to toggle autostart";
      onError?.(msg);
    }
  }

  /**
   * Internal: persist retention setting (debounced)
   */
  async function persistRetention(days: number) {
    try {
      await store.set("data_retention_days", days);
      await cleanupOldData(days);
      onSuccess?.("Data retention updated");
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Failed to save retention settings";
      onError?.(msg);
    }
  }

  const debouncedPersistRetention = debounce(persistRetention, 1000);

  /**
   * Save data retention setting (immediate UI update, debounced persistence)
   */
  function saveRetention(days: number) {
    dataRetentionDays = days;
    debouncedPersistRetention(days);
  }

  /**
   * Log out - clears only credentials, keeps other settings
   */
  async function logout() {
    const result = await commands.clearCredentials();
    if (result.status === "error") {
      onError?.(result.error);
      return;
    }

    isConfigured = false;
    orgIdInput = "";
    tokenInput = "";
    showSettings = false;
    error = null;

    onSuccess?.("Logged out");
  }

  /**
   * Reset all settings to defaults (factory reset)
   */
  async function resetAll() {
    const result = await commands.clearCredentials();
    if (result.status === "error") {
      onError?.(result.error);
      return;
    }

    await store.clear();

    // Reset state variables
    refreshIntervalMinutes = 5;
    autoRefreshEnabled = true;
    dataRetentionDays = 30;
    isConfigured = false;
    orgIdInput = "";
    tokenInput = "";
    notificationSettings = getDefaultNotificationSettings();
    showSettings = false;
    error = null;

    // Sync reset notification settings to backend
    await commands.setNotificationSettings(notificationSettings);

    // Reset auto-refresh settings to backend
    await commands.setAutoRefresh(true, 5);

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

  /**
   * Open settings directly to the credentials tab (for re-login)
   */
  function openCredentials() {
    settingsTab = "credentials";
    showSettings = true;
    error = null;
  }

  return {
    // Store reference (for other composables)
    store,

    // UI state
    get showSettings() {
      return showSettings;
    },
    set showSettings(value: boolean) {
      showSettings = value;
    },
    get settingsTab() {
      return settingsTab;
    },
    set settingsTab(value: "credentials" | "notifications" | "general") {
      settingsTab = value;
    },

    // Credentials
    get isConfigured() {
      return isConfigured;
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

    // General settings
    get refreshIntervalMinutes() {
      return refreshIntervalMinutes;
    },
    get autoRefreshEnabled() {
      return autoRefreshEnabled;
    },
    get autostartEnabled() {
      return autostartEnabled;
    },
    get dataRetentionDays() {
      return dataRetentionDays;
    },

    // Notification settings
    get notificationSettings() {
      return notificationSettings;
    },

    // Loading/error
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
    get isSessionExpired() {
      return checkSessionExpired();
    },

    // Actions
    init,
    saveCredentials,
    saveNotifications,
    saveGeneral,
    toggleAutostart,
    saveRetention,
    logout,
    resetAll,
    open,
    close,
    toggle,
    openCredentials,
  };
}
