use crate::api::{fetch_usage_for_provider, get_provider_statuses as collect_provider_statuses};
use crate::auto_refresh::do_fetch_and_emit;
use crate::credentials;
use crate::error::AppError;
use crate::history::{self, UsageHistoryPoint, UsageStats};
use crate::types::{
    AppState, NotificationSettings, ProviderKind, ProviderStatus, Settings, UsageSnapshot,
};
use crate::validation::{validate_org_id, validate_session_token};
use std::sync::Arc;

#[tauri::command]
#[specta::specta]
pub async fn get_usage(
    provider: ProviderKind,
    org_id: Option<String>,
    session_token: Option<String>,
    ollama_session_token: Option<String>,
) -> Result<UsageSnapshot, AppError> {
    fetch_usage_for_provider(
        provider,
        org_id.as_deref(),
        session_token.as_deref(),
        ollama_session_token.as_deref(),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub fn get_default_settings() -> Settings {
    Settings::default()
}

#[tauri::command]
#[specta::specta]
pub async fn save_credentials(
    state: tauri::State<'_, Arc<AppState>>,
    org_id: String,
    session_token: String,
) -> Result<(), AppError> {
    validate_org_id(&org_id)?;
    validate_session_token(&session_token)?;
    credentials::save_credentials(&org_id, &session_token)?;

    let mut config = state.config.lock().await;
    config.organization_id = Some(org_id);
    config.session_token = Some(session_token);
    drop(config);

    let _ = state.restart_tx.send(());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn clear_credentials(state: tauri::State<'_, Arc<AppState>>) -> Result<(), AppError> {
    credentials::delete_credentials()?;

    let mut config = state.config.lock().await;
    config.organization_id = None;
    config.session_token = None;
    drop(config);

    let _ = state.restart_tx.send(());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn save_ollama_credentials(
    state: tauri::State<'_, Arc<AppState>>,
    session_token: String,
) -> Result<(), AppError> {
    validate_session_token(&session_token)?;
    credentials::save_ollama_credentials(&session_token)?;

    let mut config = state.config.lock().await;
    config.ollama_session_token = Some(session_token);
    drop(config);

    let _ = state.restart_tx.send(());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn clear_ollama_credentials(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), AppError> {
    credentials::delete_ollama_credentials()?;

    let mut config = state.config.lock().await;
    config.ollama_session_token = None;
    drop(config);

    let _ = state.restart_tx.send(());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn set_active_provider(
    state: tauri::State<'_, Arc<AppState>>,
    provider: ProviderKind,
) -> Result<(), ()> {
    let mut config = state.config.lock().await;
    config.active_provider = provider;
    drop(config);

    let _ = state.restart_tx.send(());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_provider_statuses(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<ProviderStatus>, ()> {
    let config = state.config.lock().await;
    Ok(collect_provider_statuses(
        config.organization_id.as_deref(),
        config.session_token.as_deref(),
        config.ollama_session_token.as_deref(),
    ))
}

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

    let _ = state.restart_tx.send(());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn set_hourly_refresh(
    state: tauri::State<'_, Arc<AppState>>,
    enabled: bool,
) -> Result<(), ()> {
    let mut config = state.config.lock().await;
    config.hourly_refresh_enabled = enabled;
    drop(config);

    let _ = state.restart_tx.send(());
    Ok(())
}

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
    let _ = state.restart_tx.send(());
    Ok(())
}

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

#[tauri::command]
#[specta::specta]
pub fn get_usage_history_by_range(
    provider: ProviderKind,
    range: String,
) -> Result<Vec<UsageHistoryPoint>, String> {
    history::get_usage_history_by_range(provider, &range).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn get_usage_stats(provider: ProviderKind, range: String) -> Result<UsageStats, String> {
    history::get_usage_stats(provider, &range).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn cleanup_history(retention_days: u32) -> Result<usize, String> {
    history::cleanup_old_data(retention_days).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AutoRefreshConfig, NotificationState};
    use tokio::sync::watch;

    fn create_test_state() -> Arc<AppState> {
        let (restart_tx, _) = watch::channel(());
        Arc::new(AppState {
            config: tokio::sync::Mutex::new(AutoRefreshConfig::default()),
            restart_tx,
            notification_settings: tokio::sync::Mutex::new(NotificationSettings::default()),
            notification_state: tokio::sync::Mutex::new(NotificationState::default()),
            #[cfg(target_os = "macos")]
            wake_observer: tokio::sync::Mutex::new(None),
        })
    }

    #[tokio::test]
    async fn set_active_provider_updates_config() {
        let state = create_test_state();
        {
            let mut config = state.config.lock().await;
            config.active_provider = ProviderKind::Claude;
        }

        {
            let mut config = state.config.lock().await;
            config.active_provider = ProviderKind::Codex;
        }

        let config = state.config.lock().await;
        assert_eq!(config.active_provider, ProviderKind::Codex);
    }

    #[test]
    fn default_settings_use_claude() {
        let settings = get_default_settings();
        assert_eq!(settings.active_provider, ProviderKind::Claude);
        assert_eq!(settings.refresh_interval_minutes, 5);
    }
}
