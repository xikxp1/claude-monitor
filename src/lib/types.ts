export interface UsagePeriod {
  utilization: number;
  resets_at: string | null;
}

export interface UsageData {
  five_hour: UsagePeriod | null;
  seven_day: UsagePeriod | null;
  seven_day_sonnet: UsagePeriod | null;
  seven_day_opus: UsagePeriod | null;
}

export interface Settings {
  organization_id: string | null;
  session_token: string | null;
  refresh_interval_minutes: number;
  auto_refresh_enabled: boolean;
}

// Notification types

export interface NotificationRule {
  interval_enabled: boolean;
  interval_percent: number;
  thresholds: number[];
  threshold_enabled: boolean;
}

export interface NotificationSettings {
  enabled: boolean;
  five_hour: NotificationRule;
  seven_day: NotificationRule;
  seven_day_sonnet: NotificationRule;
  seven_day_opus: NotificationRule;
}

export interface NotificationState {
  five_hour_last: number;
  seven_day_last: number;
  seven_day_sonnet_last: number;
  seven_day_opus_last: number;
  fired_thresholds: string[];
}

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
  };
}
