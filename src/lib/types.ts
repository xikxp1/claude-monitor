export type {
  NotificationRule,
  NotificationSettings,
  ProviderKind,
  ProviderStatus,
  Settings,
  UsageHistoryPoint,
  UsageSnapshot,
  UsageStats,
  UsageWindow,
  WindowStats,
} from "./bindings.generated";

import type {
  NotificationRule,
  NotificationSettings,
  ProviderKind,
  UsageWindow,
} from "./bindings.generated";

export interface UsageUpdateEvent {
  usage: import("./bindings.generated").UsageSnapshot;
  nextRefreshAt: number | null;
}

export interface UsageErrorEvent {
  provider: import("./bindings.generated").ProviderKind;
  error: string;
}

export interface NotificationState {
  last_notified: Record<string, number>;
  fired_thresholds: string[];
  fired_time_remaining: string[];
}

export const PROVIDER_LABELS: Record<ProviderKind, string> = {
  claude: "Claude",
  codex: "Codex",
};

export const PROVIDER_WINDOW_DEFAULTS: Record<ProviderKind, UsageWindow[]> = {
  claude: [
    {
      key: "five_hour",
      label: "5 Hour",
      utilization: 0,
      resetsAt: null,
      windowDurationSeconds: 18_000,
    },
    {
      key: "seven_day",
      label: "7 Day",
      utilization: 0,
      resetsAt: null,
      windowDurationSeconds: 604_800,
    },
    {
      key: "seven_day_sonnet",
      label: "Sonnet (7 Day)",
      utilization: 0,
      resetsAt: null,
      windowDurationSeconds: 604_800,
    },
    {
      key: "seven_day_opus",
      label: "Opus (7 Day)",
      utilization: 0,
      resetsAt: null,
      windowDurationSeconds: 604_800,
    },
  ],
  codex: [
    {
      key: "primary",
      label: "5 Hour",
      utilization: 0,
      resetsAt: null,
      windowDurationSeconds: 18_000,
    },
    {
      key: "secondary",
      label: "7 Day",
      utilization: 0,
      resetsAt: null,
      windowDurationSeconds: 604_800,
    },
  ],
};

export function getDefaultNotificationRule(): NotificationRule {
  return {
    interval_enabled: false,
    interval_percent: 10,
    threshold_enabled: true,
    thresholds: [80, 90],
    time_remaining_enabled: false,
    time_remaining_minutes: [30, 60],
  };
}

export function getDefaultNotificationSettings(): NotificationSettings {
  return {
    enabled: true,
    rules: {},
  };
}

type LegacyNotificationSettings = {
  enabled?: unknown;
  five_hour?: NotificationRule;
  seven_day?: NotificationRule;
  seven_day_sonnet?: NotificationRule;
  seven_day_opus?: NotificationRule;
};

export function getDefaultNotificationState(): NotificationState {
  return {
    last_notified: {},
    fired_thresholds: [],
    fired_time_remaining: [],
  };
}

export function getWindowRuleKey(provider: ProviderKind, windowKey: string): string {
  return `${provider}:${windowKey}`;
}

export function normalizeNotificationSettings(value: unknown): NotificationSettings {
  if (!value || typeof value !== "object") {
    return getDefaultNotificationSettings();
  }

  const candidate = value as Partial<NotificationSettings> & LegacyNotificationSettings;

  if (candidate.rules && typeof candidate.rules === "object") {
    return {
      enabled: candidate.enabled ?? true,
      rules: candidate.rules,
    };
  }

  const legacyRules: Record<string, NotificationRule> = {};
  const legacyEntries = [
    ["five_hour", candidate.five_hour],
    ["seven_day", candidate.seven_day],
    ["seven_day_sonnet", candidate.seven_day_sonnet],
    ["seven_day_opus", candidate.seven_day_opus],
  ] as const;

  for (const [windowKey, rule] of legacyEntries) {
    if (rule) {
      legacyRules[getWindowRuleKey("claude", windowKey)] = rule;
    }
  }

  return {
    enabled: candidate.enabled ?? true,
    rules: legacyRules,
  };
}

export function getProviderWindows(
  provider: ProviderKind,
  snapshot: import("./bindings.generated").UsageSnapshot | null,
): UsageWindow[] {
  if (snapshot && snapshot.provider === provider && snapshot.windows.length > 0) {
    return snapshot.windows;
  }

  return PROVIDER_WINDOW_DEFAULTS[provider];
}
