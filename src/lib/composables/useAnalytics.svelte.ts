/**
 * Analytics composable - manages usage history data and chart filters
 */

import {
  getUsageHistoryByRange,
  type TimeRange,
  type UsageHistoryPoint,
} from "$lib/historyStorage";
import type { ProviderKind } from "$lib/types";

export interface AnalyticsCallbacks {
  getActiveProvider: () => ProviderKind;
}

export function useAnalytics(callbacks: AnalyticsCallbacks) {
  let showAnalytics = $state(false);
  let timeRange: TimeRange = $state("24h");
  let history: UsageHistoryPoint[] = $state([]);
  let loading = $state(false);
  let filters: Record<string, boolean> = $state({});

  let availableWindows = $derived.by(() => {
    const windows: Record<string, { key: string; label: string }> = {};
    for (const point of history) {
      if (!(point.windowKey in windows)) {
        windows[point.windowKey] = { key: point.windowKey, label: point.label };
      }
    }
    return Object.values(windows);
  });

  function syncFilters(points: UsageHistoryPoint[]) {
    const nextFilters = { ...filters };
    for (const point of points) {
      if (!(point.windowKey in nextFilters)) {
        nextFilters[point.windowKey] = true;
      }
    }
    filters = nextFilters;
  }

  async function load() {
    loading = true;
    try {
      history = await getUsageHistoryByRange(callbacks.getActiveProvider(), timeRange);
      syncFilters(history);
    } catch (e) {
      console.error("Failed to load analytics:", e);
    } finally {
      loading = false;
    }
  }

  async function changeTimeRange(range: TimeRange) {
    timeRange = range;
    await load();
  }

  async function open() {
    showAnalytics = true;
    await load();
  }

  function close() {
    showAnalytics = false;
  }

  async function toggle() {
    if (showAnalytics) {
      close();
      return;
    }

    await open();
  }

  function setWindowFilter(windowKey: string, enabled: boolean) {
    filters = {
      ...filters,
      [windowKey]: enabled,
    };
  }

  function resetForProviderSwitch() {
    history = [];
    filters = {};
    if (showAnalytics) {
      void load();
    }
  }

  return {
    get showAnalytics() {
      return showAnalytics;
    },
    set showAnalytics(value: boolean) {
      showAnalytics = value;
    },
    get timeRange() {
      return timeRange;
    },
    get history() {
      return history;
    },
    get loading() {
      return loading;
    },
    get filters() {
      return filters;
    },
    get availableWindows() {
      return availableWindows;
    },
    load,
    changeTimeRange,
    open,
    close,
    toggle,
    setWindowFilter,
    resetForProviderSwitch,
  };
}
