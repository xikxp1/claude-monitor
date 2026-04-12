import { describe, expect, it } from "vitest";
import {
  PROVIDER_LABELS,
  getDefaultNotificationRule,
  getDefaultNotificationSettings,
  getDefaultNotificationState,
  getProviderWindows,
  getWindowRuleKey,
  normalizeNotificationSettings,
} from "./types";

describe("provider helpers", () => {
  it("exposes provider labels", () => {
    expect(PROVIDER_LABELS.claude).toBe("Claude");
    expect(PROVIDER_LABELS.codex).toBe("Codex");
  });

  it("builds compound notification rule keys", () => {
    expect(getWindowRuleKey("codex", "primary")).toBe("codex:primary");
  });

  it("returns provider defaults when no snapshot is present", () => {
    expect(getProviderWindows("claude", null)).toHaveLength(4);
    expect(getProviderWindows("codex", null)).toHaveLength(2);
  });
});

describe("notification defaults", () => {
  it("creates a fresh default rule", () => {
    const rule = getDefaultNotificationRule();
    expect(rule.thresholds).toEqual([80, 90]);
    expect(rule.interval_percent).toBe(10);
  });

  it("creates empty provider-aware notification settings", () => {
    const settings = getDefaultNotificationSettings();
    expect(settings.enabled).toBe(true);
    expect(settings.rules).toEqual({});
  });

  it("creates empty notification state maps", () => {
    const state = getDefaultNotificationState();
    expect(state.last_notified).toEqual({});
    expect(state.fired_thresholds).toEqual([]);
  });

  it("migrates legacy notification settings into provider-window rules", () => {
    const legacy = {
      enabled: true,
      five_hour: getDefaultNotificationRule(),
      seven_day: getDefaultNotificationRule(),
    };

    const normalized = normalizeNotificationSettings(legacy);
    expect(normalized.rules["claude:five_hour"]).toEqual(legacy.five_hour);
    expect(normalized.rules["claude:seven_day"]).toEqual(legacy.seven_day);
  });
});
