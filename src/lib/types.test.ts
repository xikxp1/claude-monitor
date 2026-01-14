import { describe, it, expect } from "vitest";
import {
  getDefaultNotificationRule,
  getDefaultNotificationSettings,
  getDefaultNotificationState,
  USAGE_TYPE_LABELS,
} from "./types";

describe("USAGE_TYPE_LABELS", () => {
  it("contains all four usage types", () => {
    expect(USAGE_TYPE_LABELS.five_hour).toBe("5 Hour");
    expect(USAGE_TYPE_LABELS.seven_day).toBe("7 Day");
    expect(USAGE_TYPE_LABELS.seven_day_sonnet).toBe("Sonnet (7 Day)");
    expect(USAGE_TYPE_LABELS.seven_day_opus).toBe("Opus (7 Day)");
  });
});

describe("getDefaultNotificationRule", () => {
  it("returns correct default values", () => {
    const rule = getDefaultNotificationRule();

    expect(rule.interval_enabled).toBe(false);
    expect(rule.interval_percent).toBe(10);
    expect(rule.threshold_enabled).toBe(true);
    expect(rule.thresholds).toEqual([80, 90]);
    expect(rule.time_remaining_enabled).toBe(false);
    expect(rule.time_remaining_minutes).toEqual([30, 60]);
  });

  it("returns a new object each time", () => {
    const rule1 = getDefaultNotificationRule();
    const rule2 = getDefaultNotificationRule();

    expect(rule1).not.toBe(rule2);
    expect(rule1).toEqual(rule2);
  });

  it("has mutable thresholds array", () => {
    const rule = getDefaultNotificationRule();
    rule.thresholds.push(95);

    expect(rule.thresholds).toEqual([80, 90, 95]);

    // New call should not be affected
    const newRule = getDefaultNotificationRule();
    expect(newRule.thresholds).toEqual([80, 90]);
  });
});

describe("getDefaultNotificationSettings", () => {
  it("returns enabled by default", () => {
    const settings = getDefaultNotificationSettings();
    expect(settings.enabled).toBe(true);
  });

  it("contains all four usage type rules", () => {
    const settings = getDefaultNotificationSettings();

    expect(settings.five_hour).toBeDefined();
    expect(settings.seven_day).toBeDefined();
    expect(settings.seven_day_sonnet).toBeDefined();
    expect(settings.seven_day_opus).toBeDefined();
  });

  it("each usage type has default rule values", () => {
    const settings = getDefaultNotificationSettings();
    const defaultRule = getDefaultNotificationRule();

    expect(settings.five_hour).toEqual(defaultRule);
    expect(settings.seven_day).toEqual(defaultRule);
    expect(settings.seven_day_sonnet).toEqual(defaultRule);
    expect(settings.seven_day_opus).toEqual(defaultRule);
  });

  it("usage type rules are independent objects", () => {
    const settings = getDefaultNotificationSettings();

    settings.five_hour.thresholds.push(95);

    expect(settings.five_hour.thresholds).toEqual([80, 90, 95]);
    expect(settings.seven_day.thresholds).toEqual([80, 90]);
  });
});

describe("getDefaultNotificationState", () => {
  it("initializes all last values to zero", () => {
    const state = getDefaultNotificationState();

    expect(state.five_hour_last).toBe(0);
    expect(state.seven_day_last).toBe(0);
    expect(state.seven_day_sonnet_last).toBe(0);
    expect(state.seven_day_opus_last).toBe(0);
  });

  it("initializes fired arrays as empty", () => {
    const state = getDefaultNotificationState();

    expect(state.fired_thresholds).toEqual([]);
    expect(state.fired_time_remaining).toEqual([]);
  });

  it("returns a new object each time", () => {
    const state1 = getDefaultNotificationState();
    const state2 = getDefaultNotificationState();

    expect(state1).not.toBe(state2);
    expect(state1).toEqual(state2);
  });

  it("has independent fired arrays", () => {
    const state1 = getDefaultNotificationState();
    state1.fired_thresholds.push("five_hour:80");

    const state2 = getDefaultNotificationState();
    expect(state2.fired_thresholds).toEqual([]);
  });
});
