import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import type {
  NotificationRule,
  NotificationSettings,
  NotificationState,
  UsageData,
  UsageType,
} from "./types";
import { USAGE_TYPE_LABELS } from "./types";

/**
 * Request notification permission if not already granted
 */
export async function ensureNotificationPermission(): Promise<boolean> {
  let permissionGranted = await isPermissionGranted();
  if (!permissionGranted) {
    const permission = await requestPermission();
    permissionGranted = permission === "granted";
  }
  return permissionGranted;
}

/**
 * Check if an interval notification should be sent
 * Returns the interval level that was crossed, or null if no notification needed
 */
function checkIntervalNotification(
  currentUtilization: number,
  lastNotified: number,
  intervalPercent: number,
): number | null {
  if (intervalPercent <= 0) return null;

  // Calculate which interval level we're at now vs before
  const currentLevel = Math.floor(currentUtilization / intervalPercent) * intervalPercent;
  const lastLevel = Math.floor(lastNotified / intervalPercent) * intervalPercent;

  // Only notify if we crossed into a new interval level (and it's > 0)
  if (currentLevel > lastLevel && currentLevel > 0) {
    return currentLevel;
  }

  return null;
}

/**
 * Check if a threshold notification should be sent
 * Returns the threshold that was crossed, or null if no notification needed
 */
function checkThresholdNotification(
  currentUtilization: number,
  lastNotified: number,
  thresholds: number[],
  firedThresholds: string[],
  usageType: string,
): number | null {
  for (const threshold of thresholds) {
    const key = `${usageType}:${threshold}`;
    // Check if we crossed this threshold and haven't notified yet
    if (
      currentUtilization >= threshold &&
      lastNotified < threshold &&
      !firedThresholds.includes(key)
    ) {
      return threshold;
    }
  }
  return null;
}

/**
 * Process usage data and send notifications as needed
 * Returns updated notification state
 */
export async function processNotifications(
  usageData: UsageData,
  settings: NotificationSettings,
  state: NotificationState,
): Promise<NotificationState> {
  if (!settings.enabled) {
    return state;
  }

  const hasPermission = await ensureNotificationPermission();
  if (!hasPermission) {
    return state;
  }

  const newState: NotificationState = { ...state };
  const usageTypes: UsageType[] = ["five_hour", "seven_day", "seven_day_sonnet", "seven_day_opus"];

  for (const usageType of usageTypes) {
    const period = usageData[usageType];
    if (!period) continue;

    const rule: NotificationRule = settings[usageType];
    const currentUtilization = period.utilization;
    const lastNotified = state[`${usageType}_last` as keyof NotificationState] as number;

    const notifications: string[] = [];

    // Check interval notification
    if (rule.interval_enabled) {
      const intervalLevel = checkIntervalNotification(
        currentUtilization,
        lastNotified,
        rule.interval_percent,
      );
      if (intervalLevel !== null) {
        notifications.push(`reached ${intervalLevel}%`);
      }
    }

    // Check threshold notification
    if (rule.threshold_enabled) {
      const threshold = checkThresholdNotification(
        currentUtilization,
        lastNotified,
        rule.thresholds,
        state.fired_thresholds,
        usageType,
      );
      if (threshold !== null) {
        notifications.push(`crossed ${threshold}% threshold`);
        newState.fired_thresholds = [...newState.fired_thresholds, `${usageType}:${threshold}`];
      }
    }

    // Send notification if any triggers fired
    if (notifications.length > 0) {
      const title = `${USAGE_TYPE_LABELS[usageType]} Usage Alert`;
      const body = `Usage ${notifications.join(" and ")} (${currentUtilization.toFixed(0)}% used)`;

      sendNotification({
        title,
        body,
      });
    }

    // Update last notified level
    switch (usageType) {
      case "five_hour":
        newState.five_hour_last = currentUtilization;
        break;
      case "seven_day":
        newState.seven_day_last = currentUtilization;
        break;
      case "seven_day_sonnet":
        newState.seven_day_sonnet_last = currentUtilization;
        break;
      case "seven_day_opus":
        newState.seven_day_opus_last = currentUtilization;
        break;
    }
  }

  return newState;
}

/**
 * Reset notification state when usage resets (utilization drops significantly)
 * This allows threshold notifications to fire again after a reset
 */
export function resetNotificationStateIfNeeded(
  usageData: UsageData,
  state: NotificationState,
): NotificationState {
  const newState: NotificationState = { ...state };
  const usageTypes: UsageType[] = ["five_hour", "seven_day", "seven_day_sonnet", "seven_day_opus"];

  for (const usageType of usageTypes) {
    const period = usageData[usageType];
    if (!period) continue;

    let lastNotified: number;
    switch (usageType) {
      case "five_hour":
        lastNotified = state.five_hour_last;
        break;
      case "seven_day":
        lastNotified = state.seven_day_last;
        break;
      case "seven_day_sonnet":
        lastNotified = state.seven_day_sonnet_last;
        break;
      case "seven_day_opus":
        lastNotified = state.seven_day_opus_last;
        break;
    }

    // If utilization dropped by more than 20%, assume a reset happened
    if (lastNotified - period.utilization > 20) {
      switch (usageType) {
        case "five_hour":
          newState.five_hour_last = 0;
          break;
        case "seven_day":
          newState.seven_day_last = 0;
          break;
        case "seven_day_sonnet":
          newState.seven_day_sonnet_last = 0;
          break;
        case "seven_day_opus":
          newState.seven_day_opus_last = 0;
          break;
      }
      // Clear fired thresholds for this usage type
      newState.fired_thresholds = newState.fired_thresholds.filter(
        (t) => !t.startsWith(`${usageType}:`),
      );
    }
  }

  return newState;
}
