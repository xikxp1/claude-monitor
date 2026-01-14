import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import type { NotificationSettings } from "$lib/types";

// Mock autostart plugin
vi.mock("@tauri-apps/plugin-autostart", () => ({
  disable: vi.fn(() => Promise.resolve()),
  enable: vi.fn(() => Promise.resolve()),
  isEnabled: vi.fn(() => Promise.resolve(false)),
}));

// Mock store plugin - declare module-level mocks that will be assigned in beforeEach
let mockStoreGet: ReturnType<typeof vi.fn<(key: string) => Promise<unknown>>>;
let mockStoreSet: ReturnType<typeof vi.fn<(key: string, value: unknown) => Promise<void>>>;
let mockStoreClear: ReturnType<typeof vi.fn<() => Promise<void>>>;

vi.mock("@tauri-apps/plugin-store", () => {
  // Create stable references that can be reassigned
  const storeInstance = {
    get: (key: string) => mockStoreGet?.(key),
    set: (key: string, value: unknown) => mockStoreSet?.(key, value),
    clear: () => mockStoreClear?.(),
  };

  return {
    LazyStore: class {
      get = storeInstance.get;
      set = storeInstance.set;
      clear = storeInstance.clear;
    },
  };
});

// Mock bindings
vi.mock("$lib/bindings.generated", () => ({
  commands: {
    getIsConfigured: vi.fn(() => Promise.resolve({ status: "ok", data: false })),
    saveCredentials: vi.fn(() => Promise.resolve({ status: "ok", data: null })),
    clearCredentials: vi.fn(() => Promise.resolve({ status: "ok", data: null })),
    setAutoRefresh: vi.fn(() => Promise.resolve({ status: "ok", data: null })),
    setNotificationSettings: vi.fn(() => Promise.resolve({ status: "ok", data: null })),
  },
}));

// Mock historyStorage
vi.mock("$lib/historyStorage", () => ({
  cleanupOldData: vi.fn(() => Promise.resolve(0)),
}));

import { useSettings } from "./useSettings.svelte";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import { commands } from "$lib/bindings.generated";
import { cleanupOldData } from "$lib/historyStorage";
import { getDefaultNotificationSettings } from "$lib/types";

const mockCommands = vi.mocked(commands);
const mockEnableAutostart = vi.mocked(enableAutostart);
const mockDisableAutostart = vi.mocked(disableAutostart);
const mockIsAutostartEnabled = vi.mocked(isAutostartEnabled);
const mockCleanupOldData = vi.mocked(cleanupOldData);

describe("useSettings", () => {
  let onSuccess: ReturnType<typeof vi.fn<(message: string) => void>>;
  let onError: ReturnType<typeof vi.fn<(message: string) => void>>;

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();

    onSuccess = vi.fn();
    onError = vi.fn();

    // Initialize store mocks
    mockStoreGet = vi.fn().mockResolvedValue(undefined);
    mockStoreSet = vi.fn().mockResolvedValue(undefined);
    mockStoreClear = vi.fn().mockResolvedValue(undefined);

    // Reset command mocks to default success responses
    mockCommands.getIsConfigured.mockResolvedValue({ status: "ok", data: false });
    mockCommands.saveCredentials.mockResolvedValue({ status: "ok", data: null });
    mockCommands.clearCredentials.mockResolvedValue({ status: "ok", data: null });
    mockCommands.setAutoRefresh.mockResolvedValue({ status: "ok", data: null });
    mockCommands.setNotificationSettings.mockResolvedValue({ status: "ok", data: null });

    // Reset autostart mock
    mockIsAutostartEnabled.mockResolvedValue(false);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("initial state", () => {
    it("starts with settings panel closed", () => {
      const settings = useSettings();
      expect(settings.showSettings).toBe(false);
    });

    it("starts with credentials tab selected", () => {
      const settings = useSettings();
      expect(settings.settingsTab).toBe("credentials");
    });

    it("starts with not configured", () => {
      const settings = useSettings();
      expect(settings.isConfigured).toBe(false);
    });

    it("starts with empty credential inputs", () => {
      const settings = useSettings();
      expect(settings.orgIdInput).toBe("");
      expect(settings.tokenInput).toBe("");
    });

    it("starts with default general settings", () => {
      const settings = useSettings();
      expect(settings.refreshIntervalMinutes).toBe(5);
      expect(settings.autoRefreshEnabled).toBe(true);
      expect(settings.autostartEnabled).toBe(false);
      expect(settings.dataRetentionDays).toBe(30);
    });

    it("starts with default notification settings", () => {
      const settings = useSettings();
      expect(settings.notificationSettings).toEqual(getDefaultNotificationSettings());
    });

    it("starts with no loading or error state", () => {
      const settings = useSettings();
      expect(settings.loading).toBe(false);
      expect(settings.error).toBeNull();
    });
  });

  describe("init", () => {
    it("checks if configured from backend", async () => {
      mockCommands.getIsConfigured.mockResolvedValue({ status: "ok", data: true });

      const settings = useSettings();
      await settings.init();

      expect(mockCommands.getIsConfigured).toHaveBeenCalled();
      expect(settings.isConfigured).toBe(true);
    });

    it("handles backend error gracefully", async () => {
      mockCommands.getIsConfigured.mockRejectedValue(new Error("Backend error"));
      const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});

      const settings = useSettings();
      await settings.init();

      expect(settings.isConfigured).toBe(false);
      consoleError.mockRestore();
    });

    it("loads general settings from store", async () => {
      mockStoreGet.mockImplementation(async (key: string) => {
        if (key === "refresh_interval_minutes") return 10;
        if (key === "auto_refresh_enabled") return false;
        if (key === "data_retention_days") return 60;
        return undefined;
      });

      const settings = useSettings();
      await settings.init();

      expect(settings.refreshIntervalMinutes).toBe(10);
      expect(settings.autoRefreshEnabled).toBe(false);
      expect(settings.dataRetentionDays).toBe(60);
    });

    it("loads notification settings from store", async () => {
      const customNotifications: NotificationSettings = {
        enabled: false,
        five_hour: {
          interval_enabled: true,
          interval_percent: 20,
          threshold_enabled: false,
          thresholds: [],
          time_remaining_enabled: false,
          time_remaining_minutes: [],
        },
        seven_day: getDefaultNotificationSettings().seven_day,
        seven_day_sonnet: getDefaultNotificationSettings().seven_day_sonnet,
        seven_day_opus: getDefaultNotificationSettings().seven_day_opus,
      };

      mockStoreGet.mockImplementation(async (key: string) => {
        if (key === "notification_settings") return customNotifications;
        return undefined;
      });

      const settings = useSettings();
      await settings.init();

      expect(settings.notificationSettings).toEqual(customNotifications);
    });

    it("syncs notification settings to backend", async () => {
      const settings = useSettings();
      await settings.init();

      expect(mockCommands.setNotificationSettings).toHaveBeenCalled();
    });

    it("loads autostart state", async () => {
      mockIsAutostartEnabled.mockResolvedValue(true);

      const settings = useSettings();
      await settings.init();

      expect(settings.autostartEnabled).toBe(true);
    });

    it("runs data cleanup", async () => {
      const settings = useSettings();
      await settings.init();

      expect(mockCleanupOldData).toHaveBeenCalledWith(30);
    });

    it("sends auto-refresh settings to backend", async () => {
      const settings = useSettings();
      await settings.init();

      expect(mockCommands.setAutoRefresh).toHaveBeenCalledWith(true, 5);
    });
  });

  describe("saveCredentials", () => {
    it("saves credentials to backend with current input values", async () => {
      const settings = useSettings({ onSuccess, onError });
      // Note: Svelte 5 $state setters work in test environment
      settings.orgIdInput = "test-org";
      settings.tokenInput = "test-token";

      // Verify setters worked
      expect(settings.orgIdInput).toBe("test-org");
      expect(settings.tokenInput).toBe("test-token");

      await settings.saveCredentials();

      expect(mockCommands.saveCredentials).toHaveBeenCalledWith("test-org", "test-token");
    });

    it("clears inputs after successful save", async () => {
      const settings = useSettings({ onSuccess, onError });
      settings.orgIdInput = "test-org";
      settings.tokenInput = "test-token";

      await settings.saveCredentials();

      expect(settings.orgIdInput).toBe("");
      expect(settings.tokenInput).toBe("");
    });

    it("closes settings panel after save", async () => {
      const settings = useSettings({ onSuccess, onError });
      settings.showSettings = true;
      expect(settings.showSettings).toBe(true);

      await settings.saveCredentials();

      expect(settings.showSettings).toBe(false);
    });

    it("calls onSuccess callback on successful save", async () => {
      const settings = useSettings({ onSuccess, onError });

      await settings.saveCredentials();

      expect(onSuccess).toHaveBeenCalledWith("Credentials saved");
    });

    it("handles save error", async () => {
      mockCommands.saveCredentials.mockResolvedValue({
        status: "error",
        error: "Invalid credentials",
      });

      const settings = useSettings({ onSuccess, onError });
      await settings.saveCredentials();

      expect(settings.error).toBe("Invalid credentials");
      expect(onError).toHaveBeenCalledWith("Invalid credentials");
    });

    it("keeps loading state for initial setup (wasConfigured=false)", async () => {
      // First call returns false (not configured), second call after save returns true
      mockCommands.getIsConfigured
        .mockResolvedValueOnce({ status: "ok", data: false })
        .mockResolvedValueOnce({ status: "ok", data: true });

      const settings = useSettings({ onSuccess, onError });
      // Not configured initially
      expect(settings.isConfigured).toBe(false);

      await settings.saveCredentials();

      // Loading should stay true for initial setup (wasConfigured was false)
      expect(settings.loading).toBe(true);
    });

    it("resets loading state when already configured (wasConfigured=true)", async () => {
      mockCommands.getIsConfigured.mockResolvedValue({ status: "ok", data: true });

      const settings = useSettings({ onSuccess, onError });
      await settings.init(); // Now configured
      expect(settings.isConfigured).toBe(true);

      await settings.saveCredentials();

      expect(settings.loading).toBe(false);
    });
  });

  describe("saveNotifications", () => {
    it("updates notification settings immediately", () => {
      const settings = useSettings({ onSuccess, onError });
      const newSettings: NotificationSettings = {
        ...getDefaultNotificationSettings(),
        enabled: false,
      };

      settings.saveNotifications(newSettings);

      expect(settings.notificationSettings).toEqual(newSettings);
    });

    it("persists to store after debounce", async () => {
      const settings = useSettings({ onSuccess, onError });
      const newSettings: NotificationSettings = {
        ...getDefaultNotificationSettings(),
        enabled: false,
      };

      settings.saveNotifications(newSettings);

      // Not saved yet (debounced)
      expect(mockStoreSet).not.toHaveBeenCalledWith("notification_settings", newSettings);

      // Advance past debounce time
      vi.advanceTimersByTime(1000);
      await vi.runAllTimersAsync();

      expect(mockStoreSet).toHaveBeenCalledWith("notification_settings", newSettings);
    });
  });

  describe("saveGeneral", () => {
    it("updates general settings immediately", () => {
      const settings = useSettings({ onSuccess, onError });

      settings.saveGeneral(false, 10);

      expect(settings.autoRefreshEnabled).toBe(false);
      expect(settings.refreshIntervalMinutes).toBe(10);
    });

    it("persists to store after debounce", async () => {
      const settings = useSettings({ onSuccess, onError });

      settings.saveGeneral(false, 10);

      // Not saved yet
      expect(mockStoreSet).not.toHaveBeenCalledWith("auto_refresh_enabled", false);

      // Advance past debounce
      vi.advanceTimersByTime(1000);
      await vi.runAllTimersAsync();

      expect(mockStoreSet).toHaveBeenCalledWith("auto_refresh_enabled", false);
      expect(mockStoreSet).toHaveBeenCalledWith("refresh_interval_minutes", 10);
    });
  });

  describe("toggleAutostart", () => {
    it("enables autostart", async () => {
      const settings = useSettings({ onSuccess, onError });

      await settings.toggleAutostart(true);

      expect(mockEnableAutostart).toHaveBeenCalled();
      expect(settings.autostartEnabled).toBe(true);
      expect(onSuccess).toHaveBeenCalledWith("Autostart enabled");
    });

    it("disables autostart", async () => {
      const settings = useSettings({ onSuccess, onError });

      await settings.toggleAutostart(false);

      expect(mockDisableAutostart).toHaveBeenCalled();
      expect(settings.autostartEnabled).toBe(false);
      expect(onSuccess).toHaveBeenCalledWith("Autostart disabled");
    });

    it("handles autostart error", async () => {
      mockEnableAutostart.mockRejectedValue(new Error("Permission denied"));

      const settings = useSettings({ onSuccess, onError });
      await settings.toggleAutostart(true);

      expect(onError).toHaveBeenCalledWith("Permission denied");
    });
  });

  describe("saveRetention", () => {
    it("updates retention setting immediately", () => {
      const settings = useSettings({ onSuccess, onError });

      settings.saveRetention(60);

      expect(settings.dataRetentionDays).toBe(60);
    });

    it("persists and runs cleanup after debounce", async () => {
      const settings = useSettings({ onSuccess, onError });

      settings.saveRetention(60);

      vi.advanceTimersByTime(1000);
      await vi.runAllTimersAsync();

      expect(mockStoreSet).toHaveBeenCalledWith("data_retention_days", 60);
      expect(mockCleanupOldData).toHaveBeenCalledWith(60);
    });
  });

  describe("clearAll", () => {
    it("clears credentials from backend", async () => {
      const settings = useSettings({ onSuccess, onError });

      await settings.clearAll();

      expect(mockCommands.clearCredentials).toHaveBeenCalled();
    });

    it("clears store", async () => {
      const settings = useSettings({ onSuccess, onError });

      await settings.clearAll();

      expect(mockStoreClear).toHaveBeenCalled();
    });

    it("resets all state to defaults", async () => {
      const settings = useSettings({ onSuccess, onError });
      settings.orgIdInput = "test";
      settings.tokenInput = "test";
      settings.showSettings = true;

      await settings.clearAll();

      expect(settings.refreshIntervalMinutes).toBe(5);
      expect(settings.autoRefreshEnabled).toBe(true);
      expect(settings.isConfigured).toBe(false);
      expect(settings.orgIdInput).toBe("");
      expect(settings.tokenInput).toBe("");
      expect(settings.showSettings).toBe(false);
    });

    it("syncs reset notification settings to backend", async () => {
      const settings = useSettings({ onSuccess, onError });

      await settings.clearAll();

      expect(mockCommands.setNotificationSettings).toHaveBeenCalledWith(
        getDefaultNotificationSettings(),
      );
    });

    it("handles clear error", async () => {
      mockCommands.clearCredentials.mockResolvedValue({
        status: "error",
        error: "Failed to clear",
      });

      const settings = useSettings({ onSuccess, onError });
      await settings.clearAll();

      expect(onError).toHaveBeenCalledWith("Failed to clear");
    });
  });

  describe("UI controls", () => {
    it("open() shows settings panel", () => {
      const settings = useSettings();

      settings.open();

      expect(settings.showSettings).toBe(true);
    });

    it("close() hides settings panel", () => {
      const settings = useSettings();
      settings.showSettings = true;

      settings.close();

      expect(settings.showSettings).toBe(false);
    });

    it("toggle() toggles settings panel", () => {
      const settings = useSettings();

      settings.toggle();
      expect(settings.showSettings).toBe(true);

      settings.toggle();
      expect(settings.showSettings).toBe(false);
    });

    it("openCredentials() opens to credentials tab and clears error", () => {
      const settings = useSettings();
      settings.settingsTab = "general";
      settings.error = "Some error";

      settings.openCredentials();

      expect(settings.showSettings).toBe(true);
      expect(settings.settingsTab).toBe("credentials");
      expect(settings.error).toBeNull();
    });
  });

  describe("isSessionExpired", () => {
    it("returns true when error contains 'session expired'", () => {
      const settings = useSettings();
      settings.error = "Session expired, please log in again";

      expect(settings.isSessionExpired).toBe(true);
    });

    it("returns true when error contains 'Session Expired' (case insensitive)", () => {
      const settings = useSettings();
      settings.error = "Session Expired";

      expect(settings.isSessionExpired).toBe(true);
    });

    it("returns false for other errors", () => {
      const settings = useSettings();
      settings.error = "Network error";

      expect(settings.isSessionExpired).toBe(false);
    });

    it("returns false when no error", () => {
      const settings = useSettings();

      expect(settings.isSessionExpired).toBe(false);
    });
  });

  describe("settable properties", () => {
    it("allows setting showSettings", () => {
      const settings = useSettings();

      settings.showSettings = true;
      expect(settings.showSettings).toBe(true);
    });

    it("allows setting settingsTab", () => {
      const settings = useSettings();

      settings.settingsTab = "notifications";
      expect(settings.settingsTab).toBe("notifications");

      settings.settingsTab = "general";
      expect(settings.settingsTab).toBe("general");
    });

    it("allows setting orgIdInput", () => {
      const settings = useSettings();

      settings.orgIdInput = "new-org";
      expect(settings.orgIdInput).toBe("new-org");
    });

    it("allows setting tokenInput", () => {
      const settings = useSettings();

      settings.tokenInput = "new-token";
      expect(settings.tokenInput).toBe("new-token");
    });

    it("allows setting loading", () => {
      const settings = useSettings();

      settings.loading = true;
      expect(settings.loading).toBe(true);
    });

    it("allows setting error", () => {
      const settings = useSettings();

      settings.error = "Test error";
      expect(settings.error).toBe("Test error");
    });
  });
});
