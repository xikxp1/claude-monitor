import { describe, it, expect, vi, beforeEach } from "vitest";
import type { UsageHistoryRecord } from "$lib/historyStorage";

// Mock historyStorage before importing the composable
vi.mock("$lib/historyStorage", () => ({
  getUsageHistoryByRange: vi.fn(),
}));

// Import after mocking
import { useAnalytics } from "./useAnalytics.svelte";
import { getUsageHistoryByRange } from "$lib/historyStorage";

const mockGetUsageHistoryByRange = vi.mocked(getUsageHistoryByRange);

const mockHistoryData: UsageHistoryRecord[] = [
  {
    id: 1,
    timestamp: "2024-01-01T10:00:00Z",
    five_hour_utilization: 25,
    five_hour_resets_at: "2024-01-01T15:00:00Z",
    seven_day_utilization: 40,
    seven_day_resets_at: "2024-01-08T00:00:00Z",
    sonnet_utilization: 30,
    sonnet_resets_at: "2024-01-08T00:00:00Z",
    opus_utilization: 20,
    opus_resets_at: "2024-01-08T00:00:00Z",
  },
  {
    id: 2,
    timestamp: "2024-01-01T11:00:00Z",
    five_hour_utilization: 35,
    five_hour_resets_at: "2024-01-01T16:00:00Z",
    seven_day_utilization: 45,
    seven_day_resets_at: "2024-01-08T00:00:00Z",
    sonnet_utilization: 35,
    sonnet_resets_at: "2024-01-08T00:00:00Z",
    opus_utilization: 25,
    opus_resets_at: "2024-01-08T00:00:00Z",
  },
];

describe("useAnalytics", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetUsageHistoryByRange.mockResolvedValue(mockHistoryData);
  });

  describe("initial state", () => {
    it("starts with analytics panel closed", () => {
      const analytics = useAnalytics();
      expect(analytics.showAnalytics).toBe(false);
    });

    it("starts with default time range of 24h", () => {
      const analytics = useAnalytics();
      expect(analytics.timeRange).toBe("24h");
    });

    it("starts with empty history", () => {
      const analytics = useAnalytics();
      expect(analytics.history).toEqual([]);
    });

    it("starts with loading false", () => {
      const analytics = useAnalytics();
      expect(analytics.loading).toBe(false);
    });

    it("starts with all chart filters enabled", () => {
      const analytics = useAnalytics();
      expect(analytics.showFiveHour).toBe(true);
      expect(analytics.showSevenDay).toBe(true);
      expect(analytics.showSonnet).toBe(true);
      expect(analytics.showOpus).toBe(true);
    });
  });

  describe("open/close/toggle", () => {
    it("open() sets showAnalytics to true and loads data", async () => {
      const analytics = useAnalytics();

      await analytics.open();

      expect(analytics.showAnalytics).toBe(true);
      expect(mockGetUsageHistoryByRange).toHaveBeenCalledWith("24h");
      expect(analytics.history).toEqual(mockHistoryData);
    });

    it("close() sets showAnalytics to false", async () => {
      const analytics = useAnalytics();
      await analytics.open();

      analytics.close();

      expect(analytics.showAnalytics).toBe(false);
    });

    it("toggle() opens when closed", async () => {
      const analytics = useAnalytics();

      await analytics.toggle();

      expect(analytics.showAnalytics).toBe(true);
    });

    it("toggle() closes when open", async () => {
      const analytics = useAnalytics();
      await analytics.open();

      await analytics.toggle();

      expect(analytics.showAnalytics).toBe(false);
    });
  });

  describe("load", () => {
    it("sets loading to true during fetch", async () => {
      let resolvePromise: (value: UsageHistoryRecord[]) => void;
      mockGetUsageHistoryByRange.mockImplementation(
        () =>
          new Promise((resolve) => {
            resolvePromise = resolve;
          }),
      );

      const analytics = useAnalytics();
      const loadPromise = analytics.load();

      expect(analytics.loading).toBe(true);

      resolvePromise!(mockHistoryData);
      await loadPromise;

      expect(analytics.loading).toBe(false);
    });

    it("updates history with fetched data", async () => {
      const analytics = useAnalytics();

      await analytics.load();

      expect(analytics.history).toEqual(mockHistoryData);
    });

    it("handles fetch errors gracefully", async () => {
      const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
      mockGetUsageHistoryByRange.mockRejectedValue(new Error("Network error"));

      const analytics = useAnalytics();
      await analytics.load();

      expect(analytics.loading).toBe(false);
      expect(consoleError).toHaveBeenCalledWith(
        "Failed to load analytics:",
        expect.any(Error),
      );

      consoleError.mockRestore();
    });
  });

  describe("changeTimeRange", () => {
    it("updates time range and reloads data", async () => {
      const analytics = useAnalytics();

      await analytics.changeTimeRange("7d");

      expect(analytics.timeRange).toBe("7d");
      expect(mockGetUsageHistoryByRange).toHaveBeenCalledWith("7d");
    });

    it("supports all time ranges", async () => {
      const analytics = useAnalytics();

      for (const range of ["1h", "6h", "24h", "7d", "30d"] as const) {
        await analytics.changeTimeRange(range);
        expect(analytics.timeRange).toBe(range);
        expect(mockGetUsageHistoryByRange).toHaveBeenCalledWith(range);
      }
    });
  });

  describe("chart filters", () => {
    it("allows toggling showFiveHour", () => {
      const analytics = useAnalytics();

      analytics.showFiveHour = false;
      expect(analytics.showFiveHour).toBe(false);

      analytics.showFiveHour = true;
      expect(analytics.showFiveHour).toBe(true);
    });

    it("allows toggling showSevenDay", () => {
      const analytics = useAnalytics();

      analytics.showSevenDay = false;
      expect(analytics.showSevenDay).toBe(false);
    });

    it("allows toggling showSonnet", () => {
      const analytics = useAnalytics();

      analytics.showSonnet = false;
      expect(analytics.showSonnet).toBe(false);
    });

    it("allows toggling showOpus", () => {
      const analytics = useAnalytics();

      analytics.showOpus = false;
      expect(analytics.showOpus).toBe(false);
    });

    it("allows setting showAnalytics directly", () => {
      const analytics = useAnalytics();

      analytics.showAnalytics = true;
      expect(analytics.showAnalytics).toBe(true);

      analytics.showAnalytics = false;
      expect(analytics.showAnalytics).toBe(false);
    });
  });
});
