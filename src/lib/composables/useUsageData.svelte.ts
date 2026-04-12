/**
 * Usage data composable - manages usage data fetching, events, and countdown timer
 */

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { commands } from "$lib/bindings.generated";
import type { UsageSnapshot } from "$lib/types";

export interface UsageDataCallbacks {
  isAutoRefreshEnabled: () => boolean;
  setLoading: (value: boolean) => void;
  setError: (value: string | null) => void;
  isConfigured: () => boolean;
}

export function useUsageData(callbacks: UsageDataCallbacks) {
  let usageData: UsageSnapshot | null = $state(null);
  let lastUpdateTime: Date | null = $state(null);
  let nextRefreshAt: number | null = $state(null);
  let secondsUntilNextUpdate = $state(0);
  let secondsSinceLastUpdate = $state(0);

  let countdownInterval: ReturnType<typeof setInterval> | null = null;
  let visibilityHandler: (() => void) | null = null;
  let unlistenFns: UnlistenFn[] = [];

  function updateTimers() {
    if (nextRefreshAt && callbacks.isAutoRefreshEnabled()) {
      secondsUntilNextUpdate = Math.max(
        0,
        Math.floor((nextRefreshAt - Date.now()) / 1000),
      );
    } else {
      secondsUntilNextUpdate = 0;
    }

    if (lastUpdateTime) {
      secondsSinceLastUpdate = Math.floor(
        (Date.now() - lastUpdateTime.getTime()) / 1000,
      );
    }
  }

  function startInterval() {
    if (!countdownInterval) {
      countdownInterval = setInterval(updateTimers, 1000);
    }
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
      return;
    }

    updateTimers();
    startInterval();
  }

  function startCountdown() {
    stopCountdown();
    startInterval();
    visibilityHandler = handleVisibilityChange;
    document.addEventListener("visibilitychange", visibilityHandler);
  }

  function stopCountdown() {
    stopInterval();
    if (visibilityHandler) {
      document.removeEventListener("visibilitychange", visibilityHandler);
      visibilityHandler = null;
    }
  }

  async function setupEventListeners() {
    unlistenFns.push(
      await listen<{ usage: UsageSnapshot; nextRefreshAt: number | null }>(
        "usage-updated",
        (event) => {
          usageData = event.payload.usage;
          lastUpdateTime = new Date();
          nextRefreshAt = event.payload.nextRefreshAt;
          secondsSinceLastUpdate = 0;
          callbacks.setError(null);
          callbacks.setLoading(false);
        },
      ),
    );

    unlistenFns.push(
      await listen<{ provider: string; error: string }>("usage-error", (event) => {
        callbacks.setError(event.payload.error);
        callbacks.setLoading(false);
      }),
    );
  }

  async function refreshNow() {
    if (!callbacks.isConfigured()) {
      return;
    }

    callbacks.setLoading(true);
    callbacks.setError(null);

    try {
      const result = await commands.refreshNow();
      if (result.status === "error") {
        throw new Error(result.error ?? "Failed to refresh");
      }
    } catch (e) {
      console.error("Failed to trigger refresh:", e);
      callbacks.setError(e instanceof Error ? e.message : "Failed to refresh");
      callbacks.setLoading(false);
    }
  }

  function cleanup() {
    for (const unlisten of unlistenFns) {
      unlisten();
    }
    unlistenFns = [];
    stopCountdown();
  }

  function reset() {
    usageData = null;
    lastUpdateTime = null;
    nextRefreshAt = null;
    secondsSinceLastUpdate = 0;
    secondsUntilNextUpdate = 0;
  }

  return {
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
    setupEventListeners,
    startCountdown,
    stopCountdown,
    refreshNow,
    cleanup,
    reset,
  };
}
