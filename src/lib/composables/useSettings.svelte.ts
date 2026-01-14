/**
 * Settings composable - manages credentials, general settings, autostart, and notifications
 */

import { invoke } from "@tauri-apps/api/core";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import { LazyStore } from "@tauri-apps/plugin-store";
import { cleanupOldData } from "$lib/historyStorage";
import type { NotificationSettings } from "$lib/types";
import { getDefaultNotificationSettings } from "$lib/types";

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
      isConfigured = await invoke<boolean>("get_is_configured");
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
      await invoke("set_notification_settings", { settings: notificationSettings });
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
      await invoke("set_auto_refresh", {
        enabled: autoRefreshEnabled,
        intervalMinutes: refreshIntervalMinutes,
      });
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

    try {
      await invoke("save_credentials", {
        orgId: orgIdInput,
        sessionToken: tokenInput,
      });

      // Clear form inputs
      orgIdInput = "";
      tokenInput = "";

      // Update configured state
      isConfigured = await invoke<boolean>("get_is_configured");

      showSettings = false;

      // For initial setup, keep loading=true while waiting for first data fetch
      // The backend event handler will set loading=false when data arrives
      if (wasConfigured) {
        loading = false;
      }

      onSuccess?.("Credentials saved");
    } catch (e) {
      error = e instanceof Error ? e.message : "Failed to save settings";
      loading = false;
      onError?.(error);
    }
  }

  /**
   * Save notification settings
   */
  async function saveNotifications(newSettings: NotificationSettings) {
    notificationSettings = newSettings;
    try {
      await store.set("notification_settings", newSettings);
      // Sync to backend for backend-driven notifications
      await invoke("set_notification_settings", { settings: newSettings });
      onSuccess?.("Notification settings saved");
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Failed to save notification settings";
      onError?.(msg);
    }
  }

  /**
   * Save general settings (auto-refresh)
   */
  async function saveGeneral(
    newAutoRefreshEnabled: boolean,
    newRefreshIntervalMinutes: number,
  ) {
    autoRefreshEnabled = newAutoRefreshEnabled;
    refreshIntervalMinutes = newRefreshIntervalMinutes;

    try {
      await store.set("auto_refresh_enabled", newAutoRefreshEnabled);
      await store.set("refresh_interval_minutes", newRefreshIntervalMinutes);

      await invoke("set_auto_refresh", {
        enabled: newAutoRefreshEnabled,
        intervalMinutes: newRefreshIntervalMinutes,
      });
      onSuccess?.("Settings saved");
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Failed to save settings";
      onError?.(msg);
    }
  }

  /**
   * Toggle autostart
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
   * Save data retention setting
   */
  async function saveRetention(days: number) {
    dataRetentionDays = days;

    try {
      await store.set("data_retention_days", days);
      await cleanupOldData(days);
      onSuccess?.("Data retention updated");
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Failed to save retention settings";
      onError?.(msg);
    }
  }

  /**
   * Clear all settings
   */
  async function clearAll() {
    try {
      await invoke("clear_credentials");
      await store.clear();

      // Reset state variables
      refreshIntervalMinutes = 5;
      autoRefreshEnabled = true;
      isConfigured = false;
      orgIdInput = "";
      tokenInput = "";
      notificationSettings = getDefaultNotificationSettings();
      showSettings = false;

      // Sync reset notification settings to backend
      await invoke("set_notification_settings", { settings: notificationSettings });
      onSuccess?.("Settings cleared");
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Failed to clear settings";
      onError?.(msg);
    }
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
    clearAll,
    open,
    close,
    toggle,
    openCredentials,
  };
}
