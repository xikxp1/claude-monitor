use crate::types::{NotificationSettings, NotificationState, UsageData};
use chrono::{DateTime, Utc};
use tauri_plugin_notification::NotificationExt;

fn get_label(usage_type: &str) -> &'static str {
    match usage_type {
        "five_hour" => "5 Hour",
        "seven_day" => "7 Day",
        "seven_day_sonnet" => "Sonnet (7 Day)",
        "seven_day_opus" => "Opus (7 Day)",
        _ => "Unknown",
    }
}

/// Check if an interval notification should be sent
/// Returns the interval level that was crossed, or None if no notification needed
fn check_interval_notification(
    current_utilization: f64,
    last_notified: f64,
    interval_percent: u32,
) -> Option<u32> {
    if interval_percent == 0 {
        return None;
    }

    let interval = interval_percent as f64;
    let current_level = (current_utilization / interval).floor() as u32 * interval_percent;
    let last_level = (last_notified / interval).floor() as u32 * interval_percent;

    if current_level > last_level && current_level > 0 {
        Some(current_level)
    } else {
        None
    }
}

/// Check if a threshold notification should be sent
/// Returns the threshold that was crossed, or None if no notification needed
fn check_threshold_notification(
    current_utilization: f64,
    last_notified: f64,
    thresholds: &[u32],
    fired_thresholds: &[String],
    usage_type: &str,
) -> Option<u32> {
    for &threshold in thresholds {
        let key = format!("{}:{}", usage_type, threshold);
        if current_utilization >= threshold as f64
            && last_notified < threshold as f64
            && !fired_thresholds.contains(&key)
        {
            return Some(threshold);
        }
    }
    None
}

/// Check if a time-remaining notification should be sent
/// Returns the time threshold (in minutes) that was crossed, or None if no notification needed
fn check_time_remaining_notification(
    resets_at: Option<&String>,
    time_thresholds_minutes: &[u32],
    fired_time_remaining: &[String],
    usage_type: &str,
) -> Option<u32> {
    let resets_at_str = resets_at?;

    // Parse the reset timestamp
    let reset_time = DateTime::parse_from_rfc3339(resets_at_str)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))?;

    let now = Utc::now();
    let duration_until_reset = reset_time.signed_duration_since(now);

    // If already past reset time, no notification needed
    if duration_until_reset.num_seconds() <= 0 {
        return None;
    }

    let minutes_remaining = duration_until_reset.num_minutes() as u32;

    // Check each threshold (sorted descending to fire the highest applicable one)
    let mut sorted_thresholds: Vec<u32> = time_thresholds_minutes.to_vec();
    sorted_thresholds.sort_by(|a, b| b.cmp(a)); // Sort descending

    for &threshold_minutes in &sorted_thresholds {
        let key = format!("{}:time:{}", usage_type, threshold_minutes);

        // Fire if we're under the threshold and haven't fired this one yet
        if minutes_remaining <= threshold_minutes && !fired_time_remaining.contains(&key) {
            return Some(threshold_minutes);
        }
    }

    None
}

/// Format minutes into a human-readable string
fn format_time_remaining(minutes: u32) -> String {
    if minutes >= 60 {
        let hours = minutes / 60;
        let mins = minutes % 60;
        if mins > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}h", hours)
        }
    } else {
        format!("{}m", minutes)
    }
}

/// Process usage data and send notifications as needed
/// Returns updated notification state
pub fn process_notifications<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    usage: &UsageData,
    settings: &NotificationSettings,
    state: &NotificationState,
) -> NotificationState {
    if !settings.enabled {
        return state.clone();
    }

    let mut new_state = state.clone();

    // Process each usage type
    let usage_types = [
        ("five_hour", &usage.five_hour, &settings.five_hour),
        ("seven_day", &usage.seven_day, &settings.seven_day),
        ("seven_day_sonnet", &usage.seven_day_sonnet, &settings.seven_day_sonnet),
        ("seven_day_opus", &usage.seven_day_opus, &settings.seven_day_opus),
    ];

    for (usage_type, period_opt, rule) in usage_types {
        let period = match period_opt {
            Some(p) => p,
            None => continue,
        };

        let current_utilization = period.utilization;
        let last_notified = get_last_notified(&new_state, usage_type);

        let mut notifications: Vec<String> = Vec::new();

        // Check interval notification
        if rule.interval_enabled {
            if let Some(level) = check_interval_notification(
                current_utilization,
                last_notified,
                rule.interval_percent,
            ) {
                notifications.push(format!("reached {}%", level));
            }
        }

        // Check threshold notification
        if rule.threshold_enabled {
            if let Some(threshold) = check_threshold_notification(
                current_utilization,
                last_notified,
                &rule.thresholds,
                &state.fired_thresholds,
                usage_type,
            ) {
                notifications.push(format!("crossed {}% threshold", threshold));
                new_state
                    .fired_thresholds
                    .push(format!("{}:{}", usage_type, threshold));
            }
        }

        // Check time-remaining notification
        if rule.time_remaining_enabled {
            if let Some(threshold_minutes) = check_time_remaining_notification(
                period.resets_at.as_ref(),
                &rule.time_remaining_minutes,
                &new_state.fired_time_remaining,
                usage_type,
            ) {
                let time_str = format_time_remaining(threshold_minutes);
                notifications.push(format!("resets in < {}", time_str));
                new_state
                    .fired_time_remaining
                    .push(format!("{}:time:{}", usage_type, threshold_minutes));
            }
        }

        // Send notification if any triggers fired
        if !notifications.is_empty() {
            let title = format!("{} Usage Alert", get_label(usage_type));
            let body = format!(
                "Usage {} ({:.0}% used)",
                notifications.join(" and "),
                current_utilization
            );

            let _ = app.notification().builder()
                .title(&title)
                .body(&body)
                .show();
        }

        // Update last notified level
        set_last_notified(&mut new_state, usage_type, current_utilization);
    }

    new_state
}

/// Reset notification state when usage resets (utilization drops significantly)
pub fn reset_notification_state_if_needed(
    usage: &UsageData,
    state: &NotificationState,
) -> NotificationState {
    let mut new_state = state.clone();

    let usage_types = [
        ("five_hour", &usage.five_hour),
        ("seven_day", &usage.seven_day),
        ("seven_day_sonnet", &usage.seven_day_sonnet),
        ("seven_day_opus", &usage.seven_day_opus),
    ];

    for (usage_type, period_opt) in usage_types {
        let period = match period_opt {
            Some(p) => p,
            None => continue,
        };

        let last_notified = get_last_notified(&new_state, usage_type);

        // If utilization dropped by more than 20%, assume a reset happened
        if last_notified - period.utilization > 20.0 {
            set_last_notified(&mut new_state, usage_type, 0.0);
            // Clear fired thresholds for this usage type
            new_state
                .fired_thresholds
                .retain(|t| !t.starts_with(&format!("{}:", usage_type)));
            // Clear fired time-remaining notifications for this usage type
            new_state
                .fired_time_remaining
                .retain(|t| !t.starts_with(&format!("{}:time:", usage_type)));
        }
    }

    new_state
}

fn get_last_notified(state: &NotificationState, usage_type: &str) -> f64 {
    match usage_type {
        "five_hour" => state.five_hour_last,
        "seven_day" => state.seven_day_last,
        "seven_day_sonnet" => state.seven_day_sonnet_last,
        "seven_day_opus" => state.seven_day_opus_last,
        _ => 0.0,
    }
}

fn set_last_notified(state: &mut NotificationState, usage_type: &str, value: f64) {
    match usage_type {
        "five_hour" => state.five_hour_last = value,
        "seven_day" => state.seven_day_last = value,
        "seven_day_sonnet" => state.seven_day_sonnet_last = value,
        "seven_day_opus" => state.seven_day_opus_last = value,
        _ => {}
    }
}
