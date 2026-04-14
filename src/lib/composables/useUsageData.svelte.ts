/**
 * Usage data composable - manages usage data fetching, events, and countdown timer
 */

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { commands } from "$lib/bindings.generated";
import type { UsageSnapshot } from "$lib/types";

const RECOVERY_GRACE_MS = 15_000;
const RECOVERY_STALE_MS = 5 * 60_000;

export interface UsageDataCallbacks {
  isAutoRefreshEnabled: () => boolean;
  setLoading: (value: boolean) => void;
  setError: (value: string | null) => void;
  isConfigured: () => boolean;
}

interface RefreshRecoveryParams {
  isAutoRefreshEnabled: boolean;
  isConfigured: boolean;
  lastUpdateAt: number | null;
  nextRefreshAt: number | null;
  now?: number;
  recoveryGraceMs?: number;
  staleDataMs?: number;
}

export function shouldRecoverUsageRefresh({
  isAutoRefreshEnabled,
  isConfigured,
  lastUpdateAt,
  nextRefreshAt,
  now = Date.now(),
  recoveryGraceMs = RECOVERY_GRACE_MS,
  staleDataMs = RECOVERY_STALE_MS,
}: RefreshRecoveryParams): boolean {
  if (!isConfigured || !isAutoRefreshEnabled) {
    return false;
  }

  if (nextRefreshAt !== null) {
    return now >= nextRefreshAt + recoveryGraceMs;
  }

  if (lastUpdateAt === null) {
    return true;
  }

  return now - lastUpdateAt >= staleDataMs;
}

export function useUsageData(callbacks: UsageDataCallbacks) {
  let usageData: UsageSnapshot | null = $state(null);
  let lastUpdateAt: number | null = $state(null);
  let nextRefreshAt: number | null = $state(null);
  let secondsUntilNextUpdate = $state(0);
  let secondsSinceLastUpdate = $state(0);

  let countdownInterval: ReturnType<typeof setInterval> | null = null;
  let visibilityHandler: (() => void) | null = null;
  let focusHandler: (() => void) | null = null;
  let unlistenFns: UnlistenFn[] = [];
  let recoveryInFlight: Promise<void> | null = null;
  let lastRecoveryAttemptAt = 0;

  function updateTimers() {
    if (nextRefreshAt && callbacks.isAutoRefreshEnabled()) {
      secondsUntilNextUpdate = Math.max(
        0,
        Math.floor((nextRefreshAt - Date.now()) / 1000),
      );
    } else {
      secondsUntilNextUpdate = 0;
    }

    if (lastUpdateAt !== null) {
      secondsSinceLastUpdate = Math.floor((Date.now() - lastUpdateAt) / 1000);
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
    void recoverIfStale();
  }

  function handleFocus() {
    if (document.hidden) {
      return;
    }

    updateTimers();
    startInterval();
    void recoverIfStale();
  }

  async function recoverIfStale() {
    const now = Date.now();

    if (now - lastRecoveryAttemptAt < RECOVERY_GRACE_MS) {
      return;
    }

    if (
      !shouldRecoverUsageRefresh({
        isAutoRefreshEnabled: callbacks.isAutoRefreshEnabled(),
        isConfigured: callbacks.isConfigured(),
        lastUpdateAt,
        nextRefreshAt,
        now,
      })
    ) {
      return;
    }

    lastRecoveryAttemptAt = now;
    recoveryInFlight ??= refreshNow().finally(() => {
      recoveryInFlight = null;
    });

    await recoveryInFlight;
  }

  function startCountdown() {
    stopCountdown();
    startInterval();
    visibilityHandler = handleVisibilityChange;
    focusHandler = handleFocus;
    document.addEventListener("visibilitychange", visibilityHandler);
    window.addEventListener("focus", focusHandler);
  }

  function stopCountdown() {
    stopInterval();
    if (visibilityHandler) {
      document.removeEventListener("visibilitychange", visibilityHandler);
      visibilityHandler = null;
    }
    if (focusHandler) {
      window.removeEventListener("focus", focusHandler);
      focusHandler = null;
    }
  }

  async function setupEventListeners() {
    unlistenFns.push(
      await listen<{ usage: UsageSnapshot; nextRefreshAt: number | null }>(
        "usage-updated",
        (event) => {
          usageData = event.payload.usage;
          lastUpdateAt = Date.now();
          nextRefreshAt = event.payload.nextRefreshAt;
          secondsSinceLastUpdate = 0;
          updateTimers();
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
    lastUpdateAt = null;
    nextRefreshAt = null;
    secondsSinceLastUpdate = 0;
    secondsUntilNextUpdate = 0;
    lastRecoveryAttemptAt = 0;
  }

  return {
    get usageData() {
      return usageData;
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
