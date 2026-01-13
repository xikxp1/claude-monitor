/**
 * Usage data composable - manages usage data fetching, events, and countdown timer
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { saveUsageSnapshot } from "$lib/historyStorage";
import type { UsageData } from "$lib/types";

export interface UsageDataCallbacks {
  isAutoRefreshEnabled: () => boolean;
  setLoading: (value: boolean) => void;
  setError: (value: string | null) => void;
  isConfigured: () => boolean;
}

export function useUsageData(callbacks: UsageDataCallbacks) {
  let usageData: UsageData | null = $state(null);
  let lastUpdateTime: Date | null = $state(null);
  let nextRefreshAt: number | null = $state(null);
  let secondsUntilNextUpdate = $state(0);

  let countdownInterval: ReturnType<typeof setInterval> | null = null;
  let unlistenFns: UnlistenFn[] = [];

  function startCountdown() {
    stopCountdown();

    countdownInterval = setInterval(() => {
      if (nextRefreshAt && callbacks.isAutoRefreshEnabled()) {
        const remaining = Math.max(
          0,
          Math.floor((nextRefreshAt - Date.now()) / 1000),
        );
        secondsUntilNextUpdate = remaining;
      } else {
        secondsUntilNextUpdate = 0;
      }
    }, 1000);
  }

  function stopCountdown() {
    if (countdownInterval) {
      clearInterval(countdownInterval);
      countdownInterval = null;
    }
  }

  /**
   * Set up event listeners for backend events
   */
  async function setupEventListeners() {
    unlistenFns.push(
      await listen<{ usage: UsageData; nextRefreshAt: number | null }>(
        "usage-updated",
        async (event) => {
          const { usage, nextRefreshAt: nextAt } = event.payload;
          usageData = usage;
          lastUpdateTime = new Date();
          nextRefreshAt = nextAt;
          callbacks.setError(null);
          callbacks.setLoading(false);

          // Save usage snapshot for analytics
          try {
            await saveUsageSnapshot(usage);
          } catch (e) {
            console.error("Failed to save usage snapshot:", e);
          }

          // Note: Notifications are now processed by the Rust backend
        },
      ),
    );

    unlistenFns.push(
      await listen<{ error: string }>("usage-error", (event) => {
        callbacks.setError(event.payload.error);
        callbacks.setLoading(false);
      }),
    );
  }

  /**
   * Trigger immediate refresh
   */
  async function refreshNow() {
    if (!callbacks.isConfigured()) {
      return;
    }

    callbacks.setLoading(true);
    callbacks.setError(null);

    try {
      await invoke("refresh_now");
    } catch (e) {
      console.error("Failed to trigger refresh:", e);
      callbacks.setError(e instanceof Error ? e.message : "Failed to refresh");
      callbacks.setLoading(false);
    }
  }

  /**
   * Clean up event listeners and countdown
   */
  function cleanup() {
    for (const unlisten of unlistenFns) {
      unlisten();
    }
    unlistenFns = [];
    stopCountdown();
  }

  /**
   * Reset usage data (called when clearing settings)
   */
  function reset() {
    usageData = null;
    lastUpdateTime = null;
    nextRefreshAt = null;
  }

  return {
    // State
    get usageData() {
      return usageData;
    },
    get lastUpdateTime() {
      return lastUpdateTime;
    },
    get nextRefreshAt() {
      return nextRefreshAt;
    },
    get secondsUntilNextUpdate() {
      return secondsUntilNextUpdate;
    },

    // Actions
    setupEventListeners,
    startCountdown,
    stopCountdown,
    refreshNow,
    cleanup,
    reset,
  };
}
