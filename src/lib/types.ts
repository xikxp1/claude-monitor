// Re-export generated types from Rust via tauri-specta
// Types are auto-generated when running the app in debug mode
export type {
  MetricStats,
  NotificationRule,
  NotificationSettings,
  Settings,
  UsageData,
  UsageHistoryRecord,
  UsagePeriod,
  UsageStats,
} from "./bindings.generated";

// Import for use in this file
import type { NotificationRule, NotificationSettings, UsageData } from "./bindings.generated";

// Event payload types (not generated since they're only used in event listeners)
export interface UsageUpdateEvent {
  usage: UsageData;
  nextRefreshAt: number | null;
}

export interface UsageErrorEvent {
  error: string;
}

// Notification state (frontend-only, not a command parameter)
export interface NotificationState {
  five_hour_last: number;
  seven_day_last: number;
  seven_day_sonnet_last: number;
  seven_day_opus_last: number;
  fired_thresholds: string[];
  fired_time_remaining: string[];
}

// Frontend-only types (not shared with Rust)
export type UsageType = "five_hour" | "seven_day" | "seven_day_sonnet" | "seven_day_opus";

export const USAGE_TYPE_LABELS: Record<UsageType, string> = {
  five_hour: "5 Hour",
  seven_day: "7 Day",
  seven_day_sonnet: "Sonnet (7 Day)",
  seven_day_opus: "Opus (7 Day)",
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
    five_hour: getDefaultNotificationRule(),
    seven_day: getDefaultNotificationRule(),
    seven_day_sonnet: getDefaultNotificationRule(),
    seven_day_opus: getDefaultNotificationRule(),
  };
}

export function getDefaultNotificationState(): NotificationState {
  return {
    five_hour_last: 0,
    seven_day_last: 0,
    seven_day_sonnet_last: 0,
    seven_day_opus_last: 0,
    fired_thresholds: [],
    fired_time_remaining: [],
  };
}
