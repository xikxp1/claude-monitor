use crate::api::fetch_usage_from_api;
use crate::auto_refresh::do_fetch_and_emit;
use crate::credentials;
use crate::error::AppError;
use crate::history::{self, UsageHistoryRecord, UsageStats};
use crate::types::{AppState, NotificationSettings, Settings, UsageData};
use crate::validation::{validate_org_id, validate_session_token};
use std::sync::Arc;

#[tauri::command]
#[specta::specta]
pub async fn get_usage(org_id: String, session_token: String) -> Result<UsageData, AppError> {
    fetch_usage_from_api(&org_id, &session_token).await
}

#[tauri::command]
#[specta::specta]
pub fn get_default_settings() -> Settings {
    Settings::default()
}

/// Save credentials to OS keychain and update in-memory state
#[tauri::command]
#[specta::specta]
pub async fn save_credentials(
    state: tauri::State<'_, Arc<AppState>>,
    org_id: String,
    session_token: String,
) -> Result<(), AppError> {
    // Validate inputs
    validate_org_id(&org_id)?;
    validate_session_token(&session_token)?;

    // Save to OS keychain
    credentials::save_credentials(&org_id, &session_token)?;

    // Update in-memory config
    let mut config = state.config.lock().await;
    config.organization_id = Some(org_id);
    config.session_token = Some(session_token);
    drop(config);

    // Signal the loop to restart
    let _ = state.restart_tx.send(());
    Ok(())
}

/// Check if credentials are configured (without exposing them)
#[tauri::command]
#[specta::specta]
pub async fn get_is_configured(state: tauri::State<'_, Arc<AppState>>) -> Result<bool, ()> {
    let config = state.config.lock().await;
    let is_configured = config.organization_id.is_some() && config.session_token.is_some();
    Ok(is_configured)
}

/// Clear credentials from OS keychain and stop auto-refresh
#[tauri::command]
#[specta::specta]
pub async fn clear_credentials(state: tauri::State<'_, Arc<AppState>>) -> Result<(), AppError> {
    // Delete from OS keychain
    credentials::delete_credentials()?;

    // Clear in-memory config
    let mut config = state.config.lock().await;
    config.organization_id = None;
    config.session_token = None;
    drop(config);

    // Signal the loop to restart (will stop since no credentials)
    let _ = state.restart_tx.send(());
    Ok(())
}

/// Update auto-refresh settings and restart loop
#[tauri::command]
#[specta::specta]
pub async fn set_auto_refresh(
    state: tauri::State<'_, Arc<AppState>>,
    enabled: bool,
    interval_minutes: u32,
) -> Result<(), ()> {
    let mut config = state.config.lock().await;
    config.enabled = enabled;
    config.interval_minutes = interval_minutes;
    drop(config);

    // Signal the loop to restart
    let _ = state.restart_tx.send(());
    Ok(())
}

/// Update hourly refresh setting and restart loop
#[tauri::command]
#[specta::specta]
pub async fn set_hourly_refresh(
    state: tauri::State<'_, Arc<AppState>>,
    enabled: bool,
) -> Result<(), ()> {
    let mut config = state.config.lock().await;
    config.hourly_refresh_enabled = enabled;
    drop(config);

    // Signal the loop to restart so it picks up the new setting
    let _ = state.restart_tx.send(());
    Ok(())
}

/// Trigger immediate refresh
#[tauri::command]
#[specta::specta]
pub async fn refresh_now(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), ()> {
    let config = state.config.lock().await;
    let interval_minutes = config.interval_minutes;
    drop(config);

    do_fetch_and_emit(&app, &state, interval_minutes).await;

    // Signal the loop to restart (resets the timer)
    let _ = state.restart_tx.send(());
    Ok(())
}

/// Update notification settings in memory (frontend saves to store)
#[tauri::command]
#[specta::specta]
pub async fn set_notification_settings(
    state: tauri::State<'_, Arc<AppState>>,
    settings: NotificationSettings,
) -> Result<(), ()> {
    let mut notification_settings = state.notification_settings.lock().await;
    *notification_settings = settings;
    Ok(())
}

/// Get usage history by time range preset
#[tauri::command]
#[specta::specta]
pub fn get_usage_history_by_range(range: String) -> Result<Vec<UsageHistoryRecord>, String> {
    history::get_usage_history_by_range(&range).map_err(|e| e.to_string())
}

/// Get usage statistics for a time range
#[tauri::command]
#[specta::specta]
pub fn get_usage_stats(range: String) -> Result<UsageStats, String> {
    history::get_usage_stats(&range).map_err(|e| e.to_string())
}

/// Clean up old history data
#[tauri::command]
#[specta::specta]
pub fn cleanup_history(retention_days: u32) -> Result<usize, String> {
    history::cleanup_old_data(retention_days).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AutoRefreshConfig, NotificationRule, NotificationSettings, NotificationState, UsagePeriod};
    use tokio::sync::watch;

    /// Helper to create test AppState
    fn create_test_state() -> Arc<AppState> {
        let (restart_tx, _) = watch::channel(());
        Arc::new(AppState {
            config: tokio::sync::Mutex::new(AutoRefreshConfig::default()),
            restart_tx,
            notification_settings: tokio::sync::Mutex::new(NotificationSettings::default()),
            notification_state: tokio::sync::Mutex::new(NotificationState::default()),
        })
    }

    /// Helper to create AppState with credentials
    fn create_test_state_with_credentials(org_id: &str, token: &str) -> Arc<AppState> {
        let (restart_tx, _) = watch::channel(());
        Arc::new(AppState {
            config: tokio::sync::Mutex::new(AutoRefreshConfig {
                organization_id: Some(org_id.to_string()),
                session_token: Some(token.to_string()),
                enabled: true,
                interval_minutes: 5,
                hourly_refresh_enabled: false,
            }),
            restart_tx,
            notification_settings: tokio::sync::Mutex::new(NotificationSettings::default()),
            notification_state: tokio::sync::Mutex::new(NotificationState::default()),
        })
    }

    mod get_default_settings_tests {
        use super::*;

        #[test]
        fn returns_default_settings() {
            let settings = get_default_settings();
            assert!(settings.organization_id.is_none());
            assert!(settings.session_token.is_none());
            assert_eq!(settings.refresh_interval_minutes, 5);
        }
    }

    mod get_is_configured_tests {
        use super::*;

        #[tokio::test]
        async fn returns_false_when_no_credentials() {
            let state = create_test_state();
            let config = state.config.lock().await;
            let is_configured = config.organization_id.is_some() && config.session_token.is_some();
            assert!(!is_configured);
        }

        #[tokio::test]
        async fn returns_false_when_only_org_id() {
            let (restart_tx, _) = watch::channel(());
            let state = Arc::new(AppState {
                config: tokio::sync::Mutex::new(AutoRefreshConfig {
                    organization_id: Some("test-org".to_string()),
                    session_token: None,
                    enabled: true,
                    interval_minutes: 5,
                    hourly_refresh_enabled: false,
                }),
                restart_tx,
                notification_settings: tokio::sync::Mutex::new(NotificationSettings::default()),
                notification_state: tokio::sync::Mutex::new(NotificationState::default()),
            });
            let config = state.config.lock().await;
            let is_configured = config.organization_id.is_some() && config.session_token.is_some();
            assert!(!is_configured);
        }

        #[tokio::test]
        async fn returns_false_when_only_token() {
            let (restart_tx, _) = watch::channel(());
            let state = Arc::new(AppState {
                config: tokio::sync::Mutex::new(AutoRefreshConfig {
                    organization_id: None,
                    session_token: Some("test-token".to_string()),
                    enabled: true,
                    interval_minutes: 5,
                    hourly_refresh_enabled: false,
                }),
                restart_tx,
                notification_settings: tokio::sync::Mutex::new(NotificationSettings::default()),
                notification_state: tokio::sync::Mutex::new(NotificationState::default()),
            });
            let config = state.config.lock().await;
            let is_configured = config.organization_id.is_some() && config.session_token.is_some();
            assert!(!is_configured);
        }

        #[tokio::test]
        async fn returns_true_when_both_credentials_present() {
            let state = create_test_state_with_credentials("test-org", "test-token");
            let config = state.config.lock().await;
            let is_configured = config.organization_id.is_some() && config.session_token.is_some();
            assert!(is_configured);
        }
    }

    mod set_auto_refresh_logic_tests {
        use super::*;

        #[tokio::test]
        async fn updates_enabled_state() {
            let state = create_test_state();

            // Initially enabled (default)
            {
                let config = state.config.lock().await;
                assert!(config.enabled);
            }

            // Simulate what set_auto_refresh does
            {
                let mut config = state.config.lock().await;
                config.enabled = false;
                config.interval_minutes = 10;
            }

            {
                let config = state.config.lock().await;
                assert!(!config.enabled);
                assert_eq!(config.interval_minutes, 10);
            }
        }

        #[tokio::test]
        async fn updates_interval_minutes() {
            let state = create_test_state();

            // Initially 5 minutes (default)
            {
                let config = state.config.lock().await;
                assert_eq!(config.interval_minutes, 5);
            }

            // Update to 15 minutes
            {
                let mut config = state.config.lock().await;
                config.enabled = true;
                config.interval_minutes = 15;
            }

            {
                let config = state.config.lock().await;
                assert_eq!(config.interval_minutes, 15);
            }
        }

        #[tokio::test]
        async fn sends_restart_signal() {
            let (restart_tx, mut restart_rx) = watch::channel(());
            let state = Arc::new(AppState {
                config: tokio::sync::Mutex::new(AutoRefreshConfig::default()),
                restart_tx,
                notification_settings: tokio::sync::Mutex::new(NotificationSettings::default()),
                notification_state: tokio::sync::Mutex::new(NotificationState::default()),
            });

            // Mark current value as seen
            restart_rx.mark_changed();

            // Simulate what set_auto_refresh does
            {
                let mut config = state.config.lock().await;
                config.enabled = true;
                config.interval_minutes = 10;
            }
            let _ = state.restart_tx.send(());

            // Should have received a new signal
            assert!(restart_rx.has_changed().unwrap());
        }
    }

    mod set_notification_settings_logic_tests {
        use super::*;

        #[tokio::test]
        async fn updates_notification_settings() {
            let state = create_test_state();

            // Initially enabled (default)
            {
                let settings = state.notification_settings.lock().await;
                assert!(settings.enabled);
            }

            // Disable notifications
            let new_settings = NotificationSettings {
                enabled: false,
                five_hour: NotificationRule::default(),
                seven_day: NotificationRule::default(),
                seven_day_sonnet: NotificationRule::default(),
                seven_day_opus: NotificationRule::default(),
            };

            {
                let mut notification_settings = state.notification_settings.lock().await;
                *notification_settings = new_settings;
            }

            {
                let settings = state.notification_settings.lock().await;
                assert!(!settings.enabled);
            }
        }

        #[tokio::test]
        async fn updates_notification_rules() {
            let state = create_test_state();

            let custom_rule = NotificationRule {
                interval_enabled: true,
                interval_percent: 25,
                threshold_enabled: true,
                thresholds: vec![50, 75, 90],
                time_remaining_enabled: true,
                time_remaining_minutes: vec![15, 30],
            };

            let new_settings = NotificationSettings {
                enabled: true,
                five_hour: custom_rule.clone(),
                seven_day: NotificationRule::default(),
                seven_day_sonnet: NotificationRule::default(),
                seven_day_opus: NotificationRule::default(),
            };

            {
                let mut notification_settings = state.notification_settings.lock().await;
                *notification_settings = new_settings;
            }

            {
                let settings = state.notification_settings.lock().await;
                assert!(settings.five_hour.interval_enabled);
                assert_eq!(settings.five_hour.interval_percent, 25);
                assert_eq!(settings.five_hour.thresholds, vec![50, 75, 90]);
                assert!(settings.five_hour.time_remaining_enabled);
                assert_eq!(settings.five_hour.time_remaining_minutes, vec![15, 30]);
            }
        }
    }

    mod save_credentials_validation_tests {
        use super::*;

        #[test]
        fn rejects_empty_org_id() {
            let result = validate_org_id("");
            assert!(result.is_err());
        }

        #[test]
        fn rejects_empty_session_token() {
            let result = validate_session_token("");
            assert!(result.is_err());
        }

        #[test]
        fn rejects_org_id_with_invalid_chars() {
            let result = validate_org_id("org/with/slashes");
            assert!(result.is_err());
        }

        #[test]
        fn rejects_token_with_newlines() {
            let result = validate_session_token("token\nwith\nnewlines");
            assert!(result.is_err());
        }

        #[test]
        fn rejects_token_with_spaces() {
            let result = validate_session_token("token with spaces");
            assert!(result.is_err());
        }

        #[test]
        fn rejects_token_exceeding_max_length() {
            let long_token = "a".repeat(4097);
            let result = validate_session_token(&long_token);
            assert!(result.is_err());
        }

        #[test]
        fn rejects_org_id_exceeding_max_length() {
            let long_org_id = "a".repeat(129);
            let result = validate_org_id(&long_org_id);
            assert!(result.is_err());
        }

        #[test]
        fn accepts_valid_org_id() {
            let result = validate_org_id("valid-org-id-123");
            assert!(result.is_ok());
        }

        #[test]
        fn accepts_valid_session_token() {
            let result = validate_session_token("validToken123+/=");
            assert!(result.is_ok());
        }
    }

    mod clear_credentials_logic_tests {
        use super::*;

        #[tokio::test]
        async fn clears_in_memory_credentials() {
            let state = create_test_state_with_credentials("test-org", "test-token");

            // Verify credentials are set
            {
                let config = state.config.lock().await;
                assert!(config.organization_id.is_some());
                assert!(config.session_token.is_some());
            }

            // Simulate what clear_credentials does (without keychain)
            {
                let mut config = state.config.lock().await;
                config.organization_id = None;
                config.session_token = None;
            }

            // Verify in-memory credentials are cleared
            {
                let config = state.config.lock().await;
                assert!(config.organization_id.is_none());
                assert!(config.session_token.is_none());
            }
        }

        #[tokio::test]
        async fn sends_restart_signal() {
            let (restart_tx, mut restart_rx) = watch::channel(());
            let state = Arc::new(AppState {
                config: tokio::sync::Mutex::new(AutoRefreshConfig {
                    organization_id: Some("test-org".to_string()),
                    session_token: Some("test-token".to_string()),
                    enabled: true,
                    interval_minutes: 5,
                    hourly_refresh_enabled: false,
                }),
                restart_tx,
                notification_settings: tokio::sync::Mutex::new(NotificationSettings::default()),
                notification_state: tokio::sync::Mutex::new(NotificationState::default()),
            });

            // Mark current value as seen
            restart_rx.mark_changed();

            // Simulate what clear_credentials does
            {
                let mut config = state.config.lock().await;
                config.organization_id = None;
                config.session_token = None;
            }
            let _ = state.restart_tx.send(());

            // Should have received a new signal
            assert!(restart_rx.has_changed().unwrap());
        }
    }

    mod history_command_tests {
        use super::*;

        // Note: These tests verify the command wrapper behavior.
        // The underlying history functions are tested in history.rs.
        // Database-dependent tests require proper initialization which
        // is handled in the history module tests.

        #[test]
        fn get_usage_history_by_range_returns_error_when_db_not_initialized() {
            // Without database initialization, the command should return an error
            let result = get_usage_history_by_range("24h".to_string());
            // Result is Err because database is not initialized in test context
            assert!(result.is_err());
        }

        #[test]
        fn get_usage_stats_returns_error_when_db_not_initialized() {
            let result = get_usage_stats("24h".to_string());
            assert!(result.is_err());
        }

        #[test]
        fn cleanup_history_returns_error_when_db_not_initialized() {
            let result = cleanup_history(30);
            assert!(result.is_err());
        }

        #[test]
        fn get_usage_history_by_range_accepts_valid_range_strings() {
            // These tests verify that the commands don't panic on valid input
            // The error is expected because DB is not initialized
            let ranges = vec!["1h", "6h", "24h", "7d", "30d"];
            for range in ranges {
                let result = get_usage_history_by_range(range.to_string());
                // Should return Err (not panic) because DB not initialized
                assert!(result.is_err());
            }
        }

        #[test]
        fn get_usage_stats_accepts_valid_range_strings() {
            let ranges = vec!["1h", "6h", "24h", "7d", "30d"];
            for range in ranges {
                let result = get_usage_stats(range.to_string());
                assert!(result.is_err());
            }
        }

        #[test]
        fn cleanup_history_accepts_various_retention_days() {
            let retention_values = vec![1, 7, 30, 90, 365];
            for days in retention_values {
                let result = cleanup_history(days);
                assert!(result.is_err());
            }
        }
    }

    mod types_tests {
        use super::*;

        #[test]
        fn usage_data_default_has_none_fields() {
            let usage = UsageData {
                five_hour: None,
                seven_day: None,
                seven_day_sonnet: None,
                seven_day_opus: None,
            };
            assert!(usage.five_hour.is_none());
            assert!(usage.seven_day.is_none());
            assert!(usage.seven_day_sonnet.is_none());
            assert!(usage.seven_day_opus.is_none());
        }

        #[test]
        fn usage_period_stores_utilization() {
            let period = UsagePeriod {
                utilization: 75.5,
                resets_at: Some("2024-01-15T12:00:00Z".to_string()),
            };
            assert_eq!(period.utilization, 75.5);
            assert_eq!(
                period.resets_at,
                Some("2024-01-15T12:00:00Z".to_string())
            );
        }

        #[test]
        fn settings_default_values() {
            let settings = Settings::default();
            assert!(settings.organization_id.is_none());
            assert!(settings.session_token.is_none());
            assert_eq!(settings.refresh_interval_minutes, 5);
        }

        #[test]
        fn notification_rule_default_values() {
            let rule = NotificationRule::default();
            assert!(!rule.interval_enabled);
            assert_eq!(rule.interval_percent, 10);
            assert!(rule.threshold_enabled);
            assert_eq!(rule.thresholds, vec![80, 90]);
            assert!(!rule.time_remaining_enabled);
            assert_eq!(rule.time_remaining_minutes, vec![30, 60]);
        }

        #[test]
        fn notification_settings_default_values() {
            let settings = NotificationSettings::default();
            assert!(settings.enabled);
            // All rules should use defaults
            assert!(!settings.five_hour.interval_enabled);
            assert!(!settings.seven_day.interval_enabled);
            assert!(!settings.seven_day_sonnet.interval_enabled);
            assert!(!settings.seven_day_opus.interval_enabled);
        }

        #[test]
        fn auto_refresh_config_default_values() {
            let config = AutoRefreshConfig::default();
            assert!(config.organization_id.is_none());
            assert!(config.session_token.is_none());
            assert!(config.enabled);
            assert_eq!(config.interval_minutes, 5);
            assert!(!config.hourly_refresh_enabled);
        }
    }

    mod state_management_tests {
        use super::*;

        #[tokio::test]
        async fn app_state_can_be_shared_across_tasks() {
            let state = create_test_state();
            let state_clone = state.clone();

            // Simulate concurrent access from different tasks
            let handle = tokio::spawn(async move {
                let mut config = state_clone.config.lock().await;
                config.interval_minutes = 20;
            });

            handle.await.unwrap();

            let config = state.config.lock().await;
            assert_eq!(config.interval_minutes, 20);
        }

        #[tokio::test]
        async fn notification_state_can_be_updated() {
            let state = create_test_state();

            {
                let mut notification_state = state.notification_state.lock().await;
                notification_state.five_hour_last = 50.0;
                notification_state.fired_thresholds.push("five_hour:80".to_string());
            }

            {
                let notification_state = state.notification_state.lock().await;
                assert_eq!(notification_state.five_hour_last, 50.0);
                assert_eq!(notification_state.fired_thresholds, vec!["five_hour:80".to_string()]);
            }
        }

        #[tokio::test]
        async fn restart_signal_can_be_received_multiple_times() {
            let (restart_tx, mut restart_rx) = watch::channel(());

            // First signal
            restart_rx.mark_changed();
            let _ = restart_tx.send(());
            assert!(restart_rx.has_changed().unwrap());

            // Mark as seen
            restart_rx.mark_changed();

            // Second signal
            let _ = restart_tx.send(());
            assert!(restart_rx.has_changed().unwrap());
        }
    }
}
