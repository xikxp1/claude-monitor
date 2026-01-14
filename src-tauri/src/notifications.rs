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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UsagePeriod;

    mod get_label {
        use super::*;

        #[test]
        fn returns_correct_labels() {
            assert_eq!(get_label("five_hour"), "5 Hour");
            assert_eq!(get_label("seven_day"), "7 Day");
            assert_eq!(get_label("seven_day_sonnet"), "Sonnet (7 Day)");
            assert_eq!(get_label("seven_day_opus"), "Opus (7 Day)");
        }

        #[test]
        fn returns_unknown_for_invalid_type() {
            assert_eq!(get_label("invalid"), "Unknown");
            assert_eq!(get_label(""), "Unknown");
        }
    }

    mod check_interval_notification {
        use super::*;

        #[test]
        fn returns_none_when_interval_is_zero() {
            assert_eq!(check_interval_notification(50.0, 0.0, 0), None);
        }

        #[test]
        fn returns_none_when_same_level() {
            // Both at 10% with 10% interval = same level
            assert_eq!(check_interval_notification(15.0, 10.0, 10), None);
        }

        #[test]
        fn returns_level_when_crossing_interval() {
            // From 5% to 15% with 10% interval should trigger at 10%
            assert_eq!(check_interval_notification(15.0, 5.0, 10), Some(10));
        }

        #[test]
        fn returns_level_when_crossing_multiple_intervals() {
            // From 5% to 35% with 10% interval should trigger at 30%
            assert_eq!(check_interval_notification(35.0, 5.0, 10), Some(30));
        }

        #[test]
        fn returns_none_when_utilization_decreases() {
            assert_eq!(check_interval_notification(5.0, 15.0, 10), None);
        }

        #[test]
        fn returns_none_at_zero_percent() {
            assert_eq!(check_interval_notification(5.0, 0.0, 10), None);
        }

        #[test]
        fn handles_25_percent_interval() {
            assert_eq!(check_interval_notification(30.0, 0.0, 25), Some(25));
            assert_eq!(check_interval_notification(50.0, 30.0, 25), Some(50));
            assert_eq!(check_interval_notification(60.0, 50.0, 25), None);
        }
    }

    mod check_threshold_notification {
        use super::*;

        #[test]
        fn returns_none_when_below_threshold() {
            let thresholds = vec![80, 90];
            assert_eq!(
                check_threshold_notification(70.0, 0.0, &thresholds, &[], "five_hour"),
                None
            );
        }

        #[test]
        fn returns_threshold_when_crossing() {
            let thresholds = vec![80, 90];
            assert_eq!(
                check_threshold_notification(85.0, 70.0, &thresholds, &[], "five_hour"),
                Some(80)
            );
        }

        #[test]
        fn returns_higher_threshold_when_crossing_multiple() {
            let thresholds = vec![80, 90];
            // At 95%, having been at 70%, should return 80 (first threshold crossed)
            assert_eq!(
                check_threshold_notification(95.0, 70.0, &thresholds, &[], "five_hour"),
                Some(80)
            );
        }

        #[test]
        fn returns_none_when_already_fired() {
            let thresholds = vec![80, 90];
            let fired = vec!["five_hour:80".to_string()];
            assert_eq!(
                check_threshold_notification(85.0, 70.0, &thresholds, &fired, "five_hour"),
                None
            );
        }

        #[test]
        fn returns_next_threshold_when_first_fired() {
            let thresholds = vec![80, 90];
            let fired = vec!["five_hour:80".to_string()];
            assert_eq!(
                check_threshold_notification(95.0, 70.0, &thresholds, &fired, "five_hour"),
                Some(90)
            );
        }

        #[test]
        fn respects_usage_type_in_fired_list() {
            let thresholds = vec![80, 90];
            let fired = vec!["seven_day:80".to_string()]; // Different usage type
            assert_eq!(
                check_threshold_notification(85.0, 70.0, &thresholds, &fired, "five_hour"),
                Some(80)
            );
        }
    }

    mod format_time_remaining {
        use super::*;

        #[test]
        fn formats_minutes_only() {
            assert_eq!(format_time_remaining(30), "30m");
            assert_eq!(format_time_remaining(59), "59m");
            assert_eq!(format_time_remaining(1), "1m");
        }

        #[test]
        fn formats_hours_only() {
            assert_eq!(format_time_remaining(60), "1h");
            assert_eq!(format_time_remaining(120), "2h");
            assert_eq!(format_time_remaining(180), "3h");
        }

        #[test]
        fn formats_hours_and_minutes() {
            assert_eq!(format_time_remaining(90), "1h 30m");
            assert_eq!(format_time_remaining(150), "2h 30m");
            assert_eq!(format_time_remaining(61), "1h 1m");
        }

        #[test]
        fn handles_zero() {
            assert_eq!(format_time_remaining(0), "0m");
        }

        #[test]
        fn handles_large_values() {
            assert_eq!(format_time_remaining(1440), "24h"); // 24 hours
            assert_eq!(format_time_remaining(2880), "48h"); // 48 hours
        }
    }

    mod get_and_set_last_notified {
        use super::*;

        #[test]
        fn get_returns_correct_values() {
            let state = NotificationState {
                five_hour_last: 10.0,
                seven_day_last: 20.0,
                seven_day_sonnet_last: 30.0,
                seven_day_opus_last: 40.0,
                fired_thresholds: vec![],
                fired_time_remaining: vec![],
            };

            assert_eq!(get_last_notified(&state, "five_hour"), 10.0);
            assert_eq!(get_last_notified(&state, "seven_day"), 20.0);
            assert_eq!(get_last_notified(&state, "seven_day_sonnet"), 30.0);
            assert_eq!(get_last_notified(&state, "seven_day_opus"), 40.0);
        }

        #[test]
        fn get_returns_zero_for_unknown_type() {
            let state = NotificationState::default();
            assert_eq!(get_last_notified(&state, "unknown"), 0.0);
        }

        #[test]
        fn set_updates_correct_values() {
            let mut state = NotificationState::default();

            set_last_notified(&mut state, "five_hour", 15.0);
            assert_eq!(state.five_hour_last, 15.0);

            set_last_notified(&mut state, "seven_day", 25.0);
            assert_eq!(state.seven_day_last, 25.0);

            set_last_notified(&mut state, "seven_day_sonnet", 35.0);
            assert_eq!(state.seven_day_sonnet_last, 35.0);

            set_last_notified(&mut state, "seven_day_opus", 45.0);
            assert_eq!(state.seven_day_opus_last, 45.0);
        }

        #[test]
        fn set_ignores_unknown_type() {
            let mut state = NotificationState::default();
            set_last_notified(&mut state, "unknown", 100.0);
            // Should not change any values
            assert_eq!(state.five_hour_last, 0.0);
            assert_eq!(state.seven_day_last, 0.0);
        }
    }

    mod reset_notification_state_if_needed {
        use super::*;

        fn make_usage(five_hour: f64, seven_day: f64) -> UsageData {
            UsageData {
                five_hour: Some(UsagePeriod {
                    utilization: five_hour,
                    resets_at: None,
                }),
                seven_day: Some(UsagePeriod {
                    utilization: seven_day,
                    resets_at: None,
                }),
                seven_day_sonnet: None,
                seven_day_opus: None,
            }
        }

        #[test]
        fn resets_state_when_utilization_drops_significantly() {
            let usage = make_usage(10.0, 50.0);
            let state = NotificationState {
                five_hour_last: 80.0, // Dropped from 80% to 10% (>20% drop)
                seven_day_last: 50.0,
                fired_thresholds: vec!["five_hour:80".to_string()],
                ..Default::default()
            };

            let new_state = reset_notification_state_if_needed(&usage, &state);

            assert_eq!(new_state.five_hour_last, 0.0);
            assert!(!new_state.fired_thresholds.contains(&"five_hour:80".to_string()));
        }

        #[test]
        fn preserves_state_for_small_drops() {
            let usage = make_usage(70.0, 50.0);
            let state = NotificationState {
                five_hour_last: 80.0, // Only 10% drop
                seven_day_last: 50.0,
                fired_thresholds: vec!["five_hour:80".to_string()],
                ..Default::default()
            };

            let new_state = reset_notification_state_if_needed(&usage, &state);

            assert_eq!(new_state.five_hour_last, 80.0);
            assert!(new_state.fired_thresholds.contains(&"five_hour:80".to_string()));
        }

        #[test]
        fn clears_time_remaining_on_reset() {
            let usage = make_usage(5.0, 50.0);
            let state = NotificationState {
                five_hour_last: 90.0,
                fired_time_remaining: vec![
                    "five_hour:time:30".to_string(),
                    "seven_day:time:60".to_string(),
                ],
                ..Default::default()
            };

            let new_state = reset_notification_state_if_needed(&usage, &state);

            assert!(!new_state.fired_time_remaining.contains(&"five_hour:time:30".to_string()));
            assert!(new_state.fired_time_remaining.contains(&"seven_day:time:60".to_string()));
        }

        #[test]
        fn handles_missing_usage_periods() {
            let usage = UsageData {
                five_hour: None,
                seven_day: None,
                seven_day_sonnet: None,
                seven_day_opus: None,
            };
            let state = NotificationState {
                five_hour_last: 80.0,
                ..Default::default()
            };

            let new_state = reset_notification_state_if_needed(&usage, &state);

            // Should not reset since there's no usage data to compare
            assert_eq!(new_state.five_hour_last, 80.0);
        }
    }
}
