import { describe, expect, it } from "vitest";
import { shouldRecoverUsageRefresh } from "./useUsageData.svelte";

describe("shouldRecoverUsageRefresh", () => {
  it("returns false when the provider is not configured", () => {
    expect(
      shouldRecoverUsageRefresh({
        isAutoRefreshEnabled: true,
        isConfigured: false,
        lastUpdateAt: null,
        nextRefreshAt: null,
      }),
    ).toBe(false);
  });

  it("returns false when auto refresh is disabled", () => {
    expect(
      shouldRecoverUsageRefresh({
        isAutoRefreshEnabled: false,
        isConfigured: true,
        lastUpdateAt: null,
        nextRefreshAt: null,
      }),
    ).toBe(false);
  });

  it("recovers when configured usage has never been loaded", () => {
    expect(
      shouldRecoverUsageRefresh({
        isAutoRefreshEnabled: true,
        isConfigured: true,
        lastUpdateAt: null,
        nextRefreshAt: null,
      }),
    ).toBe(true);
  });

  it("waits for the grace window before treating a scheduled refresh as missed", () => {
    expect(
      shouldRecoverUsageRefresh({
        isAutoRefreshEnabled: true,
        isConfigured: true,
        lastUpdateAt: 0,
        nextRefreshAt: 0,
        now: 10_000,
        recoveryGraceMs: 15_000,
      }),
    ).toBe(false);
  });

  it("recovers when the scheduled refresh is overdue", () => {
    expect(
      shouldRecoverUsageRefresh({
        isAutoRefreshEnabled: true,
        isConfigured: true,
        lastUpdateAt: 0,
        nextRefreshAt: 0,
        now: 20_000,
        recoveryGraceMs: 15_000,
      }),
    ).toBe(true);
  });

  it("recovers when usage data is stale and there is no next refresh timestamp", () => {
    expect(
      shouldRecoverUsageRefresh({
        isAutoRefreshEnabled: true,
        isConfigured: true,
        lastUpdateAt: 0,
        nextRefreshAt: null,
        now: 10 * 60_000,
        staleDataMs: 5 * 60_000,
      }),
    ).toBe(true);
  });
});
