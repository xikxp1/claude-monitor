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
