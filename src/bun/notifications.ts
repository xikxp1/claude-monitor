import { Utils } from "electrobun/bun";
import type {
  NotificationRule,
  NotificationSettings,
  NotificationState,
  ProviderKind,
  UsageSnapshot,
} from "../shared/types";

export function defaultNotificationRule(): NotificationRule {
  return {
    interval_enabled: false,
    interval_percent: 10,
    threshold_enabled: true,
    thresholds: [80, 90],
    time_remaining_enabled: false,
    time_remaining_minutes: [30, 60],
  };
}

export function defaultNotificationSettings(): NotificationSettings {
  return { enabled: true, rules: {} };
}

export function defaultNotificationState(): NotificationState {
  return { last_notified: {}, fired_thresholds: [], fired_time_remaining: [] };
}

export function processNotifications(
  usage: UsageSnapshot,
  settings: NotificationSettings,
  state: NotificationState,
): NotificationState {
  if (!settings.enabled) {
    return { ...state };
  }

  const newState: NotificationState = {
    last_notified: { ...state.last_notified },
    fired_thresholds: [...state.fired_thresholds],
    fired_time_remaining: [...state.fired_time_remaining],
  };

  for (const window of usage.windows) {
    const key = compoundKey(usage.provider, window.key);
    const rule = getRule(settings, usage.provider, window.key);
    const lastNotified = newState.last_notified[key] ?? 0;
    const notifications: string[] = [];

    if (rule.interval_enabled) {
      const level = checkIntervalNotification(
        window.utilization,
        lastNotified,
        rule.interval_percent,
      );
      if (level !== null) {
        notifications.push(`reached ${level}%`);
      }
    }

    if (rule.threshold_enabled) {
      const threshold = checkThresholdNotification(
        window.utilization,
        lastNotified,
        rule.thresholds,
        newState.fired_thresholds,
        key,
      );
      if (threshold !== null) {
        notifications.push(`crossed ${threshold}% threshold`);
        newState.fired_thresholds.push(`${key}:${threshold}`);
      }
    }

    if (rule.time_remaining_enabled) {
      const thresholdMinutes = checkTimeRemainingNotification(
        window.resetsAt,
        rule.time_remaining_minutes,
        newState.fired_time_remaining,
        key,
      );
      if (thresholdMinutes !== null) {
        notifications.push(`resets in < ${formatTimeRemaining(thresholdMinutes)}`);
        newState.fired_time_remaining.push(`${key}:time:${thresholdMinutes}`);
      }
    }

    if (notifications.length > 0) {
      Utils.showNotification({
        title: `${window.label} Usage Alert`,
        body: `${usage.provider.toUpperCase()} ${notifications.join(" and ")} (${window.utilization.toFixed(0)}% used)`,
      });
    }

    newState.last_notified[key] = window.utilization;
  }

  return newState;
}

export function resetNotificationStateIfNeeded(
  usage: UsageSnapshot,
  state: NotificationState,
): NotificationState {
  const newState: NotificationState = {
    last_notified: { ...state.last_notified },
    fired_thresholds: [...state.fired_thresholds],
    fired_time_remaining: [...state.fired_time_remaining],
  };

  for (const window of usage.windows) {
    const key = compoundKey(usage.provider, window.key);
    const lastNotified = newState.last_notified[key] ?? 0;
    if (lastNotified - window.utilization > 20) {
      newState.last_notified[key] = 0;
      newState.fired_thresholds = newState.fired_thresholds.filter(
        (item) => !item.startsWith(`${key}:`),
      );
      newState.fired_time_remaining = newState.fired_time_remaining.filter(
        (item) => !item.startsWith(`${key}:time:`),
      );
    }
  }

  return newState;
}

function compoundKey(provider: ProviderKind, windowKey: string): string {
  return `${provider}:${windowKey}`;
}

function getRule(
  settings: NotificationSettings,
  provider: ProviderKind,
  windowKey: string,
): NotificationRule {
  return settings.rules[compoundKey(provider, windowKey)] ?? defaultNotificationRule();
}

export function checkIntervalNotification(
  currentUtilization: number,
  lastNotified: number,
  intervalPercent: number,
): number | null {
  if (intervalPercent === 0) {
    return null;
  }
  const currentLevel = Math.floor(currentUtilization / intervalPercent) * intervalPercent;
  const lastLevel = Math.floor(lastNotified / intervalPercent) * intervalPercent;
  return currentLevel > lastLevel && currentLevel > 0 ? currentLevel : null;
}

function checkThresholdNotification(
  currentUtilization: number,
  lastNotified: number,
  thresholds: number[],
  firedThresholds: string[],
  key: string,
): number | null {
  return (
    thresholds.find((threshold) => {
      const thresholdKey = `${key}:${threshold}`;
      return (
        currentUtilization >= threshold &&
        lastNotified < threshold &&
        !firedThresholds.includes(thresholdKey)
      );
    }) ?? null
  );
}

function checkTimeRemainingNotification(
  resetsAt: string | null,
  timeThresholdsMinutes: number[],
  firedTimeRemaining: string[],
  key: string,
): number | null {
  if (!resetsAt) {
    return null;
  }

  const minutesRemaining = Math.floor((new Date(resetsAt).getTime() - Date.now()) / 60_000);
  if (minutesRemaining <= 0) {
    return null;
  }

  return (
    [...timeThresholdsMinutes]
      .sort((a, b) => b - a)
      .find((threshold) => {
        const thresholdKey = `${key}:time:${threshold}`;
        return minutesRemaining <= threshold && !firedTimeRemaining.includes(thresholdKey);
      }) ?? null
  );
}

function formatTimeRemaining(minutes: number): string {
  if (minutes < 60) {
    return `${minutes}m`;
  }
  const hours = Math.floor(minutes / 60);
  const mins = minutes % 60;
  return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`;
}
