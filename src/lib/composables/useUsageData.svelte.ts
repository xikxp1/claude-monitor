/**
 * Usage data composable - manages usage data fetching, events, and countdown timer
 */

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { commands } from "$lib/bindings.generated";
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
  let secondsSinceLastUpdate = $state(0);

  let countdownInterval: ReturnType<typeof setInterval> | null = null;
  let visibilityHandler: (() => void) | null = null;
  let unlistenFns: UnlistenFn[] = [];

  function updateTimers() {
    // Update countdown to next refresh
    if (nextRefreshAt && callbacks.isAutoRefreshEnabled()) {
      const remaining = Math.max(
        0,
        Math.floor((nextRefreshAt - Date.now()) / 1000),
      );
      secondsUntilNextUpdate = remaining;
    } else {
      secondsUntilNextUpdate = 0;
    }

    // Update time since last update
    if (lastUpdateTime) {
      secondsSinceLastUpdate = Math.floor(
        (Date.now() - lastUpdateTime.getTime()) / 1000,
      );
    }
  }

  function startInterval() {
    if (countdownInterval) return;
    countdownInterval = setInterval(updateTimers, 1000);
  }

  function stopInterval() {
    if (countdownInterval) {
      clearInterval(countdownInterval);
      countdownInterval = null;
    }
  }

  function handleVisibilityChange() {
    if (document.hidden) {
      stopInterval();
    } else {
      // Update immediately when becoming visible, then start interval
      updateTimers();
      startInterval();
    }
  }

  function startCountdown() {
    stopCountdown();

    // Start the interval
    startInterval();

    // Set up visibility change listener to pause when hidden
    visibilityHandler = handleVisibilityChange;
    document.addEventListener("visibilitychange", visibilityHandler);
  }

  function stopCountdown() {
    stopInterval();

    // Remove visibility change listener
    if (visibilityHandler) {
      document.removeEventListener("visibilitychange", visibilityHandler);
      visibilityHandler = null;
    }
  }

  /**
   * Set up event listeners for backend events
   */
  async function setupEventListeners() {
    unlistenFns.push(
      await listen<{ usage: UsageData; nextRefreshAt: number | null }>(
        "usage-updated",
        (event) => {
          const { usage, nextRefreshAt: nextAt } = event.payload;
          usageData = usage;
          lastUpdateTime = new Date();
          nextRefreshAt = nextAt;
          secondsSinceLastUpdate = 0;
          callbacks.setError(null);
          callbacks.setLoading(false);
          // Usage snapshots and notifications are processed by the Rust backend
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
      await commands.refreshNow();
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
    secondsSinceLastUpdate = 0;
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
    get secondsSinceLastUpdate() {
      return secondsSinceLastUpdate;
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
