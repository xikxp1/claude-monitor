use crate::api::fetch_usage_from_api;
use crate::history::save_usage_snapshot;
use crate::notifications::{process_notifications, reset_notification_state_if_needed};
use crate::tray::update_tray_tooltip;
use crate::types::{AppState, UsageErrorEvent, UsageUpdateEvent};
use std::sync::Arc;
use tauri::Emitter;

pub async fn do_fetch_and_emit(app: &tauri::AppHandle, state: &AppState, interval_minutes: u32) {
    let config = state.config.lock().await;
    let org_id = config.organization_id.clone();
    let session_token = config.session_token.clone();
    let enabled = config.enabled;
    drop(config);

    if let (Some(org_id), Some(session_token)) = (org_id, session_token) {
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
                    let reset_state =
                        reset_notification_state_if_needed(&usage, &notification_state);
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

                // Calculate next refresh time
                let next_refresh_at = if enabled {
                    Some(
                        chrono::Utc::now().timestamp_millis()
                            + (interval_minutes as i64 * 60 * 1000),
                    )
                } else {
                    None
                };

                // Emit usage update event
                let _ = app.emit(
                    "usage-updated",
                    UsageUpdateEvent {
                        usage,
                        next_refresh_at,
                    },
                );
            }
            Err(e) => {
                let _ = app.emit(
                    "usage-error",
                    UsageErrorEvent {
                        error: e.to_string(),
                    },
                );
            }
        }
    }
}

pub async fn auto_refresh_loop(app: tauri::AppHandle, state: Arc<AppState>) {
    let mut restart_rx = state.restart_tx.subscribe();

    loop {
        // Get current config
        let config = state.config.lock().await;
        let enabled = config.enabled;
        let interval_minutes = config.interval_minutes;
        let has_credentials = config.organization_id.is_some() && config.session_token.is_some();
        drop(config);

        if !enabled || !has_credentials {
            // Wait for restart signal if disabled or no credentials
            let _ = restart_rx.changed().await;
            continue;
        }

        // Fetch immediately
        do_fetch_and_emit(&app, &state, interval_minutes).await;

        // Wait for either the interval to pass or a restart signal
        let interval_duration = std::time::Duration::from_secs(interval_minutes as u64 * 60);

        tokio::select! {
            _ = tokio::time::sleep(interval_duration) => {
                // Interval elapsed, continue to next iteration
            }
            _ = restart_rx.changed() => {
                // Restart signal received, continue to next iteration immediately
            }
        }
    }
}
