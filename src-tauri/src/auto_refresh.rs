use crate::api::fetch_usage_from_api;
use crate::error::AppError;
use crate::history::save_usage_snapshot;
use crate::notifications::{process_notifications, reset_notification_state_if_needed};
use crate::tray::update_tray_tooltip;
use crate::types::{AppState, UsageErrorEvent, UsageUpdateEvent};
use chrono::{Timelike, Utc};
use rand::Rng;
use std::sync::Arc;
use tauri::Emitter;

/// Result of a fetch attempt for backoff handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchResult {
    Success,
    RateLimited,
    OtherError,
    NoCredentials,
}

/// Backoff configuration
pub const INITIAL_BACKOFF_SECS: u64 = 30; // Start with 30 seconds
pub const MAX_BACKOFF_SECS: u64 = 300; // Cap at 5 minutes
pub const BACKOFF_MULTIPLIER: u64 = 2; // Double each time

/// Hourly refresh configuration
pub const HOURLY_REFRESH_INITIAL_GAP_SECS: u64 = 5; // Wait 5 seconds after hour starts
pub const HOURLY_REFRESH_JITTER_MAX_SECS: u64 = 55; // Add up to 55 seconds of jitter

/// Calculate the next backoff duration based on the current backoff and fetch result.
/// Returns the new backoff value in seconds (0 means no backoff active).
pub fn calculate_next_backoff(current_backoff: u64, result: FetchResult) -> u64 {
    match result {
        FetchResult::Success => {
            // Reset backoff on success
            0
        }
        FetchResult::RateLimited => {
            // Apply exponential backoff
            if current_backoff == 0 {
                INITIAL_BACKOFF_SECS
            } else {
                (current_backoff * BACKOFF_MULTIPLIER).min(MAX_BACKOFF_SECS)
            }
        }
        FetchResult::OtherError | FetchResult::NoCredentials => {
            // Don't change backoff for other errors
            current_backoff
        }
    }
}

/// Check if the auto-refresh loop should be active based on config.
pub fn should_refresh(enabled: bool, has_credentials: bool) -> bool {
    enabled && has_credentials
}

/// Calculate seconds until the next hour starts, plus initial gap and jitter.
/// Returns None if hourly refresh is disabled.
/// `seconds_into_hour` is the number of seconds elapsed since the current hour started (0-3599).
/// `jitter` is the random jitter to add (0-55 seconds typically).
pub fn calculate_hourly_refresh_delay_with_params(
    hourly_refresh_enabled: bool,
    seconds_into_hour: u64,
    jitter: u64,
) -> Option<u64> {
    if !hourly_refresh_enabled {
        return None;
    }

    let seconds_until_next_hour = 3600 - seconds_into_hour;
    let total_delay = seconds_until_next_hour + HOURLY_REFRESH_INITIAL_GAP_SECS + jitter;

    Some(total_delay)
}

/// Calculate seconds until the next hour starts, plus initial gap and random jitter.
/// Returns None if hourly refresh is disabled.
pub fn calculate_hourly_refresh_delay(hourly_refresh_enabled: bool) -> Option<u64> {
    if !hourly_refresh_enabled {
        return None;
    }

    let now = Utc::now();
    let seconds_into_hour = now.minute() as u64 * 60 + now.second() as u64;
    let jitter = rand::rng().random_range(0..=HOURLY_REFRESH_JITTER_MAX_SECS);

    calculate_hourly_refresh_delay_with_params(true, seconds_into_hour, jitter)
}

/// Calculate the next refresh timestamp in milliseconds.
/// Takes into account both regular interval and hourly refresh (whichever is sooner).
/// `now_ms` is the current timestamp in milliseconds.
/// `hourly_delay_secs` is the pre-calculated hourly refresh delay (if any).
pub fn calculate_next_refresh_at(
    enabled: bool,
    interval_minutes: u32,
    now_ms: i64,
    hourly_delay_secs: Option<u64>,
) -> Option<i64> {
    if !enabled {
        return None;
    }

    let regular_next = now_ms + (interval_minutes as i64 * 60 * 1000);

    // If hourly refresh delay is provided, use whichever is sooner
    if let Some(delay_secs) = hourly_delay_secs {
        let hourly_next = now_ms + (delay_secs as i64 * 1000);
        Some(regular_next.min(hourly_next))
    } else {
        Some(regular_next)
    }
}

/// Result of a fetch operation, including the next refresh timestamp
pub struct FetchOutput {
    pub result: FetchResult,
    pub next_refresh_at: Option<i64>,
}

pub async fn do_fetch_and_emit(
    app: &tauri::AppHandle,
    state: &AppState,
    interval_minutes: u32,
) -> FetchOutput {
    let config = state.config.lock().await;
    let org_id = config.organization_id.clone();
    let session_token = config.session_token.clone();
    let enabled = config.enabled;
    let hourly_refresh_enabled = config.hourly_refresh_enabled;
    drop(config);

    let (Some(org_id), Some(session_token)) = (org_id, session_token) else {
        return FetchOutput {
            result: FetchResult::NoCredentials,
            next_refresh_at: None,
        };
    };

    match fetch_usage_from_api(&org_id, &session_token).await {
        Ok(usage) => {
            // Update tray tooltip
            update_tray_tooltip(app, Some(&usage));

            // Save usage snapshot for analytics (ignore errors silently)
            let _ = save_usage_snapshot(&usage);

            // Process notifications
            {
                let notification_settings = state.notification_settings.lock().await;
                let mut notification_state = state.notification_state.lock().await;

                // Check for usage resets and clear notification state if needed
                let reset_state = reset_notification_state_if_needed(&usage, &notification_state);
                *notification_state = reset_state;

                // Process notifications and update state
                let new_state = process_notifications(
                    app,
                    &usage,
                    &notification_settings,
                    &notification_state,
                );
                *notification_state = new_state;
            }

            // Calculate next refresh time (considers both regular interval and hourly refresh)
            let now_ms = Utc::now().timestamp_millis();
            let hourly_delay = calculate_hourly_refresh_delay(hourly_refresh_enabled);
            let next_refresh_at =
                calculate_next_refresh_at(enabled, interval_minutes, now_ms, hourly_delay);

            // Emit usage update event
            let _ = app.emit(
                "usage-updated",
                UsageUpdateEvent {
                    usage,
                    next_refresh_at,
                },
            );

            FetchOutput {
                result: FetchResult::Success,
                next_refresh_at,
            }
        }
        Err(e) => {
            let is_rate_limited = matches!(e, AppError::RateLimited);

            // Calculate next refresh time even on error (for retry countdown)
            let now_ms = Utc::now().timestamp_millis();
            let hourly_delay = calculate_hourly_refresh_delay(hourly_refresh_enabled);
            let next_refresh_at =
                calculate_next_refresh_at(enabled, interval_minutes, now_ms, hourly_delay);

            let _ = app.emit(
                "usage-error",
                UsageErrorEvent {
                    error: e.to_string(),
                },
            );

            FetchOutput {
                result: if is_rate_limited {
                    FetchResult::RateLimited
                } else {
                    FetchResult::OtherError
                },
                next_refresh_at,
            }
        }
    }
}

pub async fn auto_refresh_loop(app: tauri::AppHandle, state: Arc<AppState>) {
    let mut restart_rx = state.restart_tx.subscribe();
    let mut backoff_secs: u64 = 0; // 0 means no backoff active

    loop {
        // Get current config
        let config = state.config.lock().await;
        let enabled = config.enabled;
        let interval_minutes = config.interval_minutes;
        let has_credentials = config.organization_id.is_some() && config.session_token.is_some();
        drop(config);

        if !should_refresh(enabled, has_credentials) {
            // Reset backoff when disabled or no credentials
            backoff_secs = 0;
            // Wait for restart signal
            let _ = restart_rx.changed().await;
            continue;
        }

        // Fetch immediately and get the next refresh timestamp
        let fetch_output = do_fetch_and_emit(&app, &state, interval_minutes).await;

        // Update backoff based on result
        backoff_secs = calculate_next_backoff(backoff_secs, fetch_output.result);

        // Calculate wait duration based on the same next_refresh_at that was sent to frontend
        let wait_duration = if backoff_secs > 0 {
            // If in backoff, use backoff duration
            std::time::Duration::from_secs(backoff_secs)
        } else if let Some(next_at) = fetch_output.next_refresh_at {
            // Use the same timestamp that was sent to frontend
            let now = Utc::now().timestamp_millis();
            let wait_ms = (next_at - now).max(0) as u64;
            std::time::Duration::from_millis(wait_ms)
        } else {
            // Fallback to regular interval
            std::time::Duration::from_secs(interval_minutes as u64 * 60)
        };

        tokio::select! {
            _ = tokio::time::sleep(wait_duration) => {
                // Wait elapsed, continue to next iteration
            }
            _ = restart_rx.changed() => {
                // Restart signal received (e.g., new credentials)
                // Reset backoff since user took action
                backoff_secs = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod fetch_result_tests {
        use super::*;

        #[test]
        fn fetch_result_equality() {
            assert_eq!(FetchResult::Success, FetchResult::Success);
            assert_eq!(FetchResult::RateLimited, FetchResult::RateLimited);
            assert_eq!(FetchResult::OtherError, FetchResult::OtherError);
            assert_eq!(FetchResult::NoCredentials, FetchResult::NoCredentials);
        }

        #[test]
        fn fetch_result_inequality() {
            assert_ne!(FetchResult::Success, FetchResult::RateLimited);
            assert_ne!(FetchResult::Success, FetchResult::OtherError);
            assert_ne!(FetchResult::Success, FetchResult::NoCredentials);
            assert_ne!(FetchResult::RateLimited, FetchResult::OtherError);
        }

        #[test]
        fn fetch_result_is_copy() {
            let result = FetchResult::Success;
            let copied = result;
            assert_eq!(result, copied);
        }
    }

    mod backoff_constants_tests {
        use super::*;

        #[test]
        fn initial_backoff_is_30_seconds() {
            assert_eq!(INITIAL_BACKOFF_SECS, 30);
        }

        #[test]
        fn max_backoff_is_5_minutes() {
            assert_eq!(MAX_BACKOFF_SECS, 300);
        }

        #[test]
        fn backoff_multiplier_is_2() {
            assert_eq!(BACKOFF_MULTIPLIER, 2);
        }
    }

    mod calculate_next_backoff_tests {
        use super::*;

        #[test]
        fn success_resets_backoff_to_zero() {
            assert_eq!(calculate_next_backoff(0, FetchResult::Success), 0);
            assert_eq!(calculate_next_backoff(30, FetchResult::Success), 0);
            assert_eq!(calculate_next_backoff(60, FetchResult::Success), 0);
            assert_eq!(calculate_next_backoff(300, FetchResult::Success), 0);
        }

        #[test]
        fn rate_limited_starts_initial_backoff() {
            assert_eq!(
                calculate_next_backoff(0, FetchResult::RateLimited),
                INITIAL_BACKOFF_SECS
            );
        }

        #[test]
        fn rate_limited_doubles_backoff() {
            assert_eq!(calculate_next_backoff(30, FetchResult::RateLimited), 60);
            assert_eq!(calculate_next_backoff(60, FetchResult::RateLimited), 120);
            assert_eq!(calculate_next_backoff(120, FetchResult::RateLimited), 240);
        }

        #[test]
        fn rate_limited_caps_at_max_backoff() {
            assert_eq!(calculate_next_backoff(240, FetchResult::RateLimited), 300);
            assert_eq!(calculate_next_backoff(300, FetchResult::RateLimited), 300);
            assert_eq!(calculate_next_backoff(500, FetchResult::RateLimited), 300);
        }

        #[test]
        fn other_error_preserves_backoff() {
            assert_eq!(calculate_next_backoff(0, FetchResult::OtherError), 0);
            assert_eq!(calculate_next_backoff(30, FetchResult::OtherError), 30);
            assert_eq!(calculate_next_backoff(60, FetchResult::OtherError), 60);
        }

        #[test]
        fn no_credentials_preserves_backoff() {
            assert_eq!(calculate_next_backoff(0, FetchResult::NoCredentials), 0);
            assert_eq!(calculate_next_backoff(30, FetchResult::NoCredentials), 30);
            assert_eq!(calculate_next_backoff(60, FetchResult::NoCredentials), 60);
        }

        #[test]
        fn exponential_backoff_sequence() {
            // Simulate a series of rate limited responses
            let mut backoff = 0u64;

            backoff = calculate_next_backoff(backoff, FetchResult::RateLimited);
            assert_eq!(backoff, 30); // Initial

            backoff = calculate_next_backoff(backoff, FetchResult::RateLimited);
            assert_eq!(backoff, 60); // 30 * 2

            backoff = calculate_next_backoff(backoff, FetchResult::RateLimited);
            assert_eq!(backoff, 120); // 60 * 2

            backoff = calculate_next_backoff(backoff, FetchResult::RateLimited);
            assert_eq!(backoff, 240); // 120 * 2

            backoff = calculate_next_backoff(backoff, FetchResult::RateLimited);
            assert_eq!(backoff, 300); // Capped at max

            backoff = calculate_next_backoff(backoff, FetchResult::RateLimited);
            assert_eq!(backoff, 300); // Stays at max
        }

        #[test]
        fn backoff_resets_after_success() {
            let mut backoff = 0u64;

            // Build up backoff
            backoff = calculate_next_backoff(backoff, FetchResult::RateLimited);
            backoff = calculate_next_backoff(backoff, FetchResult::RateLimited);
            assert_eq!(backoff, 60);

            // Success resets it
            backoff = calculate_next_backoff(backoff, FetchResult::Success);
            assert_eq!(backoff, 0);

            // Next rate limit starts fresh
            backoff = calculate_next_backoff(backoff, FetchResult::RateLimited);
            assert_eq!(backoff, 30);
        }
    }

    mod should_refresh_tests {
        use super::*;

        #[test]
        fn returns_true_when_enabled_and_has_credentials() {
            assert!(should_refresh(true, true));
        }

        #[test]
        fn returns_false_when_disabled() {
            assert!(!should_refresh(false, true));
            assert!(!should_refresh(false, false));
        }

        #[test]
        fn returns_false_when_no_credentials() {
            assert!(!should_refresh(true, false));
            assert!(!should_refresh(false, false));
        }
    }

    mod calculate_hourly_refresh_delay_tests {
        use super::*;

        #[test]
        fn returns_none_when_disabled() {
            assert!(calculate_hourly_refresh_delay_with_params(false, 0, 0).is_none());
            assert!(calculate_hourly_refresh_delay_with_params(false, 1800, 30).is_none());
        }

        #[test]
        fn calculates_delay_at_start_of_hour() {
            // At 00:00 of the hour, with 0 jitter
            let delay = calculate_hourly_refresh_delay_with_params(true, 0, 0).unwrap();
            // Should be 3600 (full hour) + 5 (initial gap) = 3605 seconds
            assert_eq!(delay, 3605);
        }

        #[test]
        fn calculates_delay_at_middle_of_hour() {
            // At 30:00 of the hour (1800 seconds in), with 0 jitter
            let delay = calculate_hourly_refresh_delay_with_params(true, 1800, 0).unwrap();
            // Should be 1800 (remaining) + 5 (initial gap) = 1805 seconds
            assert_eq!(delay, 1805);
        }

        #[test]
        fn calculates_delay_near_end_of_hour() {
            // At 59:00 of the hour (3540 seconds in), with 0 jitter
            let delay = calculate_hourly_refresh_delay_with_params(true, 3540, 0).unwrap();
            // Should be 60 (remaining) + 5 (initial gap) = 65 seconds
            assert_eq!(delay, 65);
        }

        #[test]
        fn adds_jitter_to_delay() {
            // At 30:00 of the hour, with 30 seconds jitter
            let delay = calculate_hourly_refresh_delay_with_params(true, 1800, 30).unwrap();
            // Should be 1800 (remaining) + 5 (initial gap) + 30 (jitter) = 1835 seconds
            assert_eq!(delay, 1835);
        }

        #[test]
        fn adds_max_jitter() {
            // With maximum jitter (55 seconds)
            let delay = calculate_hourly_refresh_delay_with_params(true, 0, 55).unwrap();
            // Should be 3600 + 5 + 55 = 3660 seconds
            assert_eq!(delay, 3660);
        }
    }

    mod calculate_next_refresh_at_tests {
        use super::*;

        const NOW_MS: i64 = 1704067200000; // 2024-01-01 00:00:00 UTC

        #[test]
        fn returns_some_when_enabled() {
            let result = calculate_next_refresh_at(true, 5, NOW_MS, None);
            assert!(result.is_some());

            let timestamp = result.unwrap();
            // Should be exactly 5 minutes in the future
            let expected = NOW_MS + (5 * 60 * 1000);
            assert_eq!(timestamp, expected);
        }

        #[test]
        fn returns_none_when_disabled() {
            assert!(calculate_next_refresh_at(false, 5, NOW_MS, None).is_none());
            assert!(calculate_next_refresh_at(false, 10, NOW_MS, None).is_none());
        }

        #[test]
        fn different_intervals_produce_different_timestamps() {
            let result_1min = calculate_next_refresh_at(true, 1, NOW_MS, None).unwrap();
            let result_5min = calculate_next_refresh_at(true, 5, NOW_MS, None).unwrap();
            let result_10min = calculate_next_refresh_at(true, 10, NOW_MS, None).unwrap();

            assert_eq!(result_1min, NOW_MS + 60_000);
            assert_eq!(result_5min, NOW_MS + 300_000);
            assert_eq!(result_10min, NOW_MS + 600_000);

            // Larger intervals should produce later timestamps
            assert!(result_5min > result_1min);
            assert!(result_10min > result_5min);
        }

        #[test]
        fn uses_hourly_delay_when_sooner() {
            // Regular interval is 30 minutes (1800 seconds)
            // Hourly delay is 10 minutes (600 seconds) - sooner
            let hourly_delay = Some(600u64);
            let result = calculate_next_refresh_at(true, 30, NOW_MS, hourly_delay).unwrap();

            // Should use the hourly delay since it's sooner
            assert_eq!(result, NOW_MS + 600_000);
        }

        #[test]
        fn uses_regular_interval_when_sooner() {
            // Regular interval is 5 minutes (300 seconds)
            // Hourly delay is 50 minutes (3000 seconds) - later
            let hourly_delay = Some(3000u64);
            let result = calculate_next_refresh_at(true, 5, NOW_MS, hourly_delay).unwrap();

            // Should use the regular interval since it's sooner
            assert_eq!(result, NOW_MS + 300_000);
        }

        #[test]
        fn ignores_hourly_delay_when_none() {
            let result = calculate_next_refresh_at(true, 5, NOW_MS, None).unwrap();
            assert_eq!(result, NOW_MS + 300_000);
        }
    }

    mod integration_tests {
        use super::*;

        #[test]
        fn full_backoff_recovery_cycle() {
            let mut backoff = 0u64;

            // Normal operation - no backoff
            assert_eq!(backoff, 0);
            assert!(should_refresh(true, true));

            // First rate limit
            backoff = calculate_next_backoff(backoff, FetchResult::RateLimited);
            assert_eq!(backoff, 30);

            // Second rate limit
            backoff = calculate_next_backoff(backoff, FetchResult::RateLimited);
            assert_eq!(backoff, 60);

            // Other error doesn't change backoff
            backoff = calculate_next_backoff(backoff, FetchResult::OtherError);
            assert_eq!(backoff, 60);

            // Success resets backoff
            backoff = calculate_next_backoff(backoff, FetchResult::Success);
            assert_eq!(backoff, 0);
        }

        #[test]
        fn disabled_state_behavior() {
            // When disabled, should not refresh
            assert!(!should_refresh(false, true));

            // Next refresh should be None
            let now_ms = 1704067200000i64;
            assert!(calculate_next_refresh_at(false, 5, now_ms, None).is_none());
        }

        #[test]
        fn no_credentials_behavior() {
            // Without credentials, should not refresh
            assert!(!should_refresh(true, false));

            // But next refresh timestamp is still calculated (frontend handles display)
            let now_ms = 1704067200000i64;
            assert!(calculate_next_refresh_at(true, 5, now_ms, None).is_some());
        }
    }
}
