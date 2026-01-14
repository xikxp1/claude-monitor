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

export function useSettings() {
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
  let error: string | null = $state(null);

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
    } catch (e) {
      error = e instanceof Error ? e.message : "Failed to save settings";
      loading = false;
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
    } catch (e) {
      console.error("Failed to save notification settings:", e);
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
    } catch (e) {
      console.error("Failed to save general settings:", e);
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
    } catch (e) {
      console.error("Failed to toggle autostart:", e);
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
    } catch {
      // Ignore errors - non-critical
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
    } catch (e) {
      console.error("Failed to clear settings:", e);
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
  };
}
