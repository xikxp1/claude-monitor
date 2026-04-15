use crate::types::{NotificationRule, NotificationSettings, NotificationState, UsageSnapshot};
use chrono::{DateTime, Utc};
use tauri_plugin_notification::NotificationExt;

fn compound_key(provider: crate::types::ProviderKind, window_key: &str) -> String {
    format!("{}:{window_key}", provider.as_str())
}

fn get_rule<'a>(
    settings: &'a NotificationSettings,
    provider: crate::types::ProviderKind,
    window_key: &str,
) -> NotificationRule {
    settings
        .rules
        .get(&compound_key(provider, window_key))
        .cloned()
        .unwrap_or_default()
}

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

    (current_level > last_level && current_level > 0).then_some(current_level)
}

fn check_threshold_notification(
    current_utilization: f64,
    last_notified: f64,
    thresholds: &[u32],
    fired_thresholds: &[String],
    key: &str,
) -> Option<u32> {
    thresholds.iter().copied().find(|threshold| {
        let threshold_key = format!("{key}:{threshold}");
        current_utilization >= *threshold as f64
            && last_notified < *threshold as f64
            && !fired_thresholds.contains(&threshold_key)
    })
}

fn check_time_remaining_notification(
    resets_at: Option<&String>,
    time_thresholds_minutes: &[u32],
    fired_time_remaining: &[String],
    key: &str,
) -> Option<u32> {
    let resets_at = resets_at?;
    let reset_time = DateTime::parse_from_rfc3339(resets_at)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))?;
    let minutes_remaining = reset_time.signed_duration_since(Utc::now()).num_minutes();

    if minutes_remaining <= 0 {
        return None;
    }

    let mut sorted_thresholds = time_thresholds_minutes.to_vec();
    sorted_thresholds.sort_by(|a, b| b.cmp(a));

    sorted_thresholds.into_iter().find(|threshold| {
        let threshold_key = format!("{key}:time:{threshold}");
        minutes_remaining <= *threshold as i64 && !fired_time_remaining.contains(&threshold_key)
    })
}

fn format_time_remaining(minutes: u32) -> String {
    if minutes >= 60 {
        let hours = minutes / 60;
        let mins = minutes % 60;
        if mins > 0 {
            format!("{hours}h {mins}m")
        } else {
            format!("{hours}h")
        }
    } else {
        format!("{minutes}m")
    }
}

pub fn process_notifications<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    usage: &UsageSnapshot,
    settings: &NotificationSettings,
    state: &NotificationState,
) -> NotificationState {
    if !settings.enabled {
        return state.clone();
    }

    let mut new_state = state.clone();

    for window in &usage.windows {
        let key = compound_key(usage.provider, &window.key);
        let rule = get_rule(settings, usage.provider, &window.key);
        let last_notified = *new_state.last_notified.get(&key).unwrap_or(&0.0);
        let mut notifications = Vec::new();

        if rule.interval_enabled {
            if let Some(level) = check_interval_notification(
                window.utilization,
                last_notified,
                rule.interval_percent,
            ) {
                notifications.push(format!("reached {level}%"));
            }
        }

        if rule.threshold_enabled {
            if let Some(threshold) = check_threshold_notification(
                window.utilization,
                last_notified,
                &rule.thresholds,
                &new_state.fired_thresholds,
                &key,
            ) {
                notifications.push(format!("crossed {threshold}% threshold"));
                new_state
                    .fired_thresholds
                    .push(format!("{key}:{threshold}"));
            }
        }

        if rule.time_remaining_enabled {
            if let Some(threshold_minutes) = check_time_remaining_notification(
                window.resets_at.as_ref(),
                &rule.time_remaining_minutes,
                &new_state.fired_time_remaining,
                &key,
            ) {
                notifications.push(format!(
                    "resets in < {}",
                    format_time_remaining(threshold_minutes)
                ));
                new_state
                    .fired_time_remaining
                    .push(format!("{key}:time:{threshold_minutes}"));
            }
        }

        if !notifications.is_empty() {
            let title = format!("{} Usage Alert", window.label);
            let body = format!(
                "{} {} ({:.0}% used)",
                usage.provider.as_str().to_uppercase(),
                notifications.join(" and "),
                window.utilization
            );

            let _ = app
                .notification()
                .builder()
                .title(&title)
                .body(&body)
                .show();
        }

        new_state.last_notified.insert(key, window.utilization);
    }

    new_state
}

pub fn reset_notification_state_if_needed(
    usage: &UsageSnapshot,
    state: &NotificationState,
) -> NotificationState {
    let mut new_state = state.clone();

    for window in &usage.windows {
        let key = compound_key(usage.provider, &window.key);
        let last_notified = *new_state.last_notified.get(&key).unwrap_or(&0.0);

        if last_notified - window.utilization > 20.0 {
            new_state.last_notified.insert(key.clone(), 0.0);
            new_state
                .fired_thresholds
                .retain(|item| !item.starts_with(&format!("{key}:")));
            new_state
                .fired_time_remaining
                .retain(|item| !item.starts_with(&format!("{key}:time:")));
        }
    }

    new_state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NotificationSettings, ProviderKind, UsageSnapshot, UsageWindow};
    use std::collections::BTreeMap;

    fn snapshot(utilization: f64) -> UsageSnapshot {
        UsageSnapshot {
            provider: ProviderKind::Codex,
            windows: vec![UsageWindow {
                key: "primary".to_string(),
                label: "5 Hour".to_string(),
                utilization,
                resets_at: None,
                window_duration_seconds: Some(18_000),
            }],
            account_email: None,
            plan_type: None,
        }
    }

    #[test]
    fn resets_state_when_window_drops_significantly() {
        let mut state = NotificationState::default();
        state
            .last_notified
            .insert("codex:primary".to_string(), 90.0);
        state.fired_thresholds.push("codex:primary:80".to_string());

        let new_state = reset_notification_state_if_needed(&snapshot(10.0), &state);
        assert_eq!(new_state.last_notified.get("codex:primary"), Some(&0.0));
        assert!(new_state.fired_thresholds.is_empty());
    }

    #[test]
    fn uses_default_rule_when_no_specific_rule_exists() {
        let settings = NotificationSettings {
            enabled: true,
            rules: BTreeMap::new(),
        };

        let rule = get_rule(&settings, ProviderKind::Claude, "five_hour");
        assert_eq!(rule.thresholds, vec![80, 90]);
    }
}
