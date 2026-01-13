/**
 * Analytics composable - manages usage history data and chart filters
 */

import {
  getUsageHistoryByRange,
  type TimeRange,
  type UsageHistoryRecord,
} from "$lib/historyStorage";

export function useAnalytics() {
  let showAnalytics = $state(false);
  let timeRange: TimeRange = $state("24h");
  let history: UsageHistoryRecord[] = $state([]);
  let loading = $state(false);

  // Chart filter states
  let showFiveHour = $state(true);
  let showSevenDay = $state(true);
  let showSonnet = $state(true);
  let showOpus = $state(true);

  async function load() {
    loading = true;
    try {
      history = await getUsageHistoryByRange(timeRange);
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

  function toggle() {
    if (showAnalytics) {
      close();
    } else {
      open();
    }
  }

  return {
    // State (getters for reactivity)
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
    get showFiveHour() {
      return showFiveHour;
    },
    set showFiveHour(value: boolean) {
      showFiveHour = value;
    },
    get showSevenDay() {
      return showSevenDay;
    },
    set showSevenDay(value: boolean) {
      showSevenDay = value;
    },
    get showSonnet() {
      return showSonnet;
    },
    set showSonnet(value: boolean) {
      showSonnet = value;
    },
    get showOpus() {
      return showOpus;
    },
    set showOpus(value: boolean) {
      showOpus = value;
    },

    // Actions
    load,
    changeTimeRange,
    open,
    close,
    toggle,
  };
}
