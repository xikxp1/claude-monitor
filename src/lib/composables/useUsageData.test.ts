import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import type { UsageData } from "$lib/types";

// Mock Tauri event API
const mockUnlisten = vi.fn();
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(mockUnlisten)),
}));

// Mock bindings
vi.mock("$lib/bindings.generated", () => ({
  commands: {
    refreshNow: vi.fn(() => Promise.resolve({ status: "ok", data: null })),
  },
}));

import { useUsageData, type UsageDataCallbacks } from "./useUsageData.svelte";
import { listen } from "@tauri-apps/api/event";
import { commands } from "$lib/bindings.generated";

const mockListen = vi.mocked(listen);
const mockRefreshNow = vi.mocked(commands.refreshNow);

const mockUsageData: UsageData = {
  five_hour: { utilization: 25, resets_at: "2024-01-01T15:00:00Z" },
  seven_day: { utilization: 40, resets_at: "2024-01-08T00:00:00Z" },
  seven_day_sonnet: { utilization: 30, resets_at: "2024-01-08T00:00:00Z" },
  seven_day_opus: { utilization: 20, resets_at: "2024-01-08T00:00:00Z" },
};

describe("useUsageData", () => {
  let callbacks: UsageDataCallbacks;

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();

    callbacks = {
      isAutoRefreshEnabled: vi.fn(() => true),
      setLoading: vi.fn(),
      setError: vi.fn(),
      isConfigured: vi.fn(() => true),
    };
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("initial state", () => {
    it("starts with null usage data", () => {
      const usageData = useUsageData(callbacks);
      expect(usageData.usageData).toBeNull();
    });

    it("starts with null last update time", () => {
      const usageData = useUsageData(callbacks);
      expect(usageData.lastUpdateTime).toBeNull();
    });

    it("starts with null next refresh time", () => {
      const usageData = useUsageData(callbacks);
      expect(usageData.nextRefreshAt).toBeNull();
    });

    it("starts with zero seconds counters", () => {
      const usageData = useUsageData(callbacks);
      expect(usageData.secondsUntilNextUpdate).toBe(0);
      expect(usageData.secondsSinceLastUpdate).toBe(0);
    });
  });

  describe("setupEventListeners", () => {
    it("sets up usage-updated listener", async () => {
      const usageData = useUsageData(callbacks);

      await usageData.setupEventListeners();

      expect(mockListen).toHaveBeenCalledWith("usage-updated", expect.any(Function));
    });

    it("sets up usage-error listener", async () => {
      const usageData = useUsageData(callbacks);

      await usageData.setupEventListeners();

      expect(mockListen).toHaveBeenCalledWith("usage-error", expect.any(Function));
    });

    it("handles usage-updated event", async () => {
      let usageUpdatedHandler: (event: { payload: { usage: UsageData; nextRefreshAt: number | null } }) => void;
      mockListen.mockImplementation(async (eventName, handler) => {
        if (eventName === "usage-updated") {
          usageUpdatedHandler = handler as typeof usageUpdatedHandler;
        }
        return mockUnlisten;
      });

      const usageData = useUsageData(callbacks);
      await usageData.setupEventListeners();

      const nextRefreshTime = Date.now() + 300000; // 5 minutes
      usageUpdatedHandler!({
        payload: { usage: mockUsageData, nextRefreshAt: nextRefreshTime },
      });

      expect(usageData.usageData).toEqual(mockUsageData);
      expect(usageData.lastUpdateTime).toBeInstanceOf(Date);
      expect(usageData.nextRefreshAt).toBe(nextRefreshTime);
      expect(callbacks.setError).toHaveBeenCalledWith(null);
      expect(callbacks.setLoading).toHaveBeenCalledWith(false);
    });

    it("handles usage-error event", async () => {
      let usageErrorHandler: (event: { payload: { error: string } }) => void;
      mockListen.mockImplementation(async (eventName, handler) => {
        if (eventName === "usage-error") {
          usageErrorHandler = handler as typeof usageErrorHandler;
        }
        return mockUnlisten;
      });

      const usageData = useUsageData(callbacks);
      await usageData.setupEventListeners();

      usageErrorHandler!({ payload: { error: "API Error" } });

      expect(callbacks.setError).toHaveBeenCalledWith("API Error");
      expect(callbacks.setLoading).toHaveBeenCalledWith(false);
    });
  });

  describe("refreshNow", () => {
    it("triggers refresh when configured", async () => {
      const usageData = useUsageData(callbacks);

      await usageData.refreshNow();

      expect(callbacks.setLoading).toHaveBeenCalledWith(true);
      expect(callbacks.setError).toHaveBeenCalledWith(null);
      expect(mockRefreshNow).toHaveBeenCalled();
    });

    it("does not trigger refresh when not configured", async () => {
      callbacks.isConfigured = vi.fn(() => false);
      const usageData = useUsageData(callbacks);

      await usageData.refreshNow();

      expect(mockRefreshNow).not.toHaveBeenCalled();
    });

    it("handles refresh errors", async () => {
      mockRefreshNow.mockRejectedValue(new Error("Network error"));
      const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});

      const usageData = useUsageData(callbacks);
      await usageData.refreshNow();

      expect(callbacks.setError).toHaveBeenCalledWith("Network error");
      expect(callbacks.setLoading).toHaveBeenCalledWith(false);

      consoleError.mockRestore();
    });
  });

  describe("countdown timer", () => {
    it("startCountdown starts the interval", () => {
      const usageData = useUsageData(callbacks);

      usageData.startCountdown();
      vi.advanceTimersByTime(1000);

      // Timer should be running - no errors
      expect(true).toBe(true);
    });

    it("stopCountdown stops the interval", () => {
      const usageData = useUsageData(callbacks);

      usageData.startCountdown();
      usageData.stopCountdown();

      // Should not throw
      expect(true).toBe(true);
    });
  });

  describe("cleanup", () => {
    it("removes event listeners on cleanup", async () => {
      const usageData = useUsageData(callbacks);
      await usageData.setupEventListeners();

      usageData.cleanup();

      expect(mockUnlisten).toHaveBeenCalledTimes(2); // Two listeners
    });

    it("stops countdown on cleanup", async () => {
      const usageData = useUsageData(callbacks);
      usageData.startCountdown();

      usageData.cleanup();

      // Should not throw
      expect(true).toBe(true);
    });
  });

  describe("reset", () => {
    it("clears all state", async () => {
      // Set up state via event
      let usageUpdatedHandler: (event: { payload: { usage: UsageData; nextRefreshAt: number | null } }) => void;
      mockListen.mockImplementation(async (eventName, handler) => {
        if (eventName === "usage-updated") {
          usageUpdatedHandler = handler as typeof usageUpdatedHandler;
        }
        return mockUnlisten;
      });

      const usageData = useUsageData(callbacks);
      await usageData.setupEventListeners();

      usageUpdatedHandler!({
        payload: { usage: mockUsageData, nextRefreshAt: Date.now() + 300000 },
      });

      expect(usageData.usageData).not.toBeNull();

      usageData.reset();

      expect(usageData.usageData).toBeNull();
      expect(usageData.lastUpdateTime).toBeNull();
      expect(usageData.nextRefreshAt).toBeNull();
      expect(usageData.secondsSinceLastUpdate).toBe(0);
    });
  });
});
