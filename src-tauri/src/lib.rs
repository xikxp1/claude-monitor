use reqwest::header::{HeaderMap, HeaderValue, COOKIE, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, Runtime,
};
#[cfg(not(target_os = "macos"))]
use tauri_plugin_positioner::{on_tray_event, Position, WindowExt};
use thiserror::Error;
use tokio::sync::{watch, Mutex};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum AppError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Invalid session token")]
    InvalidToken,
    #[error("Rate limited")]
    RateLimited,
    #[error("Server error: {0}")]
    Server(String),
    #[error("Missing configuration: {0}")]
    MissingConfig(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// ============================================================================
// API Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    pub five_hour: Option<UsagePeriod>,
    pub seven_day: Option<UsagePeriod>,
    pub seven_day_sonnet: Option<UsagePeriod>,
    pub seven_day_opus: Option<UsagePeriod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePeriod {
    pub utilization: f64,
    pub resets_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub organization_id: Option<String>,
    pub session_token: Option<String>,
    pub refresh_interval_minutes: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            organization_id: None,
            session_token: None,
            refresh_interval_minutes: 5,
        }
    }
}

// ============================================================================
// Notification Types
// ============================================================================

/// Notification rule for a single usage type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRule {
    /// Enable interval-based notifications (every X%)
    pub interval_enabled: bool,
    /// Interval percentage (e.g., 10 means notify at 10%, 20%, 30%, etc.)
    pub interval_percent: u32,
    /// Enable threshold-based notifications
    pub threshold_enabled: bool,
    /// List of threshold percentages to notify at (e.g., [50, 80, 90])
    pub thresholds: Vec<u32>,
}

impl Default for NotificationRule {
    fn default() -> Self {
        Self {
            interval_enabled: false,
            interval_percent: 10,
            threshold_enabled: true,
            thresholds: vec![80, 90],
        }
    }
}

/// Notification settings for all usage types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub five_hour: NotificationRule,
    pub seven_day: NotificationRule,
    pub seven_day_sonnet: NotificationRule,
    pub seven_day_opus: NotificationRule,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            five_hour: NotificationRule::default(),
            seven_day: NotificationRule::default(),
            seven_day_sonnet: NotificationRule::default(),
            seven_day_opus: NotificationRule::default(),
        }
    }
}

/// Tracks which notifications have been sent to avoid duplicates
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationState {
    pub five_hour_last: f64,
    pub seven_day_last: f64,
    pub seven_day_sonnet_last: f64,
    pub seven_day_opus_last: f64,
    /// Set of already-fired threshold notifications (format: "type:threshold")
    pub fired_thresholds: Vec<String>,
}

// ============================================================================
// Auto-Refresh State
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoRefreshConfig {
    pub organization_id: Option<String>,
    pub session_token: Option<String>,
    pub enabled: bool,
    pub interval_minutes: u32,
}

impl Default for AutoRefreshConfig {
    fn default() -> Self {
        Self {
            organization_id: None,
            session_token: None,
            enabled: true,
            interval_minutes: 5,
        }
    }
}

/// Event payload sent to frontend when usage is updated
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageUpdateEvent {
    pub usage: UsageData,
    pub next_refresh_at: Option<i64>, // Unix timestamp in milliseconds
}

/// Event payload sent to frontend when an error occurs
#[derive(Debug, Clone, Serialize)]
pub struct UsageErrorEvent {
    pub error: String,
}

/// Shared application state
pub struct AppState {
    pub config: Mutex<AutoRefreshConfig>,
    /// Channel to signal the refresh loop to restart
    pub restart_tx: watch::Sender<()>,
}

// ============================================================================
// API Client
// ============================================================================

async fn fetch_usage_from_api(org_id: &str, session_token: &str) -> Result<UsageData, AppError> {
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Claude-Monitor/0.1.0"),
    );
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("sessionKey={}", session_token))
            .map_err(|_| AppError::InvalidToken)?,
    );

    let url = format!(
        "https://claude.ai/api/organizations/{}/usage",
        org_id
    );

    let response = client.get(&url).headers(headers).send().await?;
    let status = response.status().as_u16();

    match status {
        200 => {
            let body = response.text().await?;
            match serde_json::from_str::<UsageData>(&body) {
                Ok(data) => Ok(data),
                Err(e) => {
                    eprintln!("Failed to parse usage response: {}", e);
                    eprintln!("Response body: {}", body);
                    Err(AppError::Server(format!("Failed to parse response: {}", e)))
                }
            }
        }
        401 => Err(AppError::InvalidToken),
        429 => Err(AppError::RateLimited),
        status => {
            let body = response.text().await.unwrap_or_default();
            eprintln!("HTTP error {}: {}", status, body);
            Err(AppError::Server(format!("HTTP {}", status)))
        }
    }
}

// ============================================================================
// Auto-Refresh Loop
// ============================================================================

fn update_tray_tooltip_internal<R: Runtime>(app: &tauri::AppHandle<R>, usage: Option<&UsageData>) {
    if let Some(tray) = app.tray_by_id("main") {
        let tooltip = match usage {
            Some(data) => {
                let mut parts = Vec::new();
                if let Some(ref period) = data.five_hour {
                    parts.push(format!("5h: {:.0}%", period.utilization));
                }
                if let Some(ref period) = data.seven_day {
                    parts.push(format!("7d: {:.0}%", period.utilization));
                }
                if let Some(ref period) = data.seven_day_sonnet {
                    parts.push(format!("Sonnet: {:.0}%", period.utilization));
                }
                if let Some(ref period) = data.seven_day_opus {
                    parts.push(format!("Opus: {:.0}%", period.utilization));
                }
                if parts.is_empty() {
                    "Claude Monitor".to_string()
                } else {
                    format!("Claude Monitor\n{}", parts.join(" | "))
                }
            }
            None => "Claude Monitor".to_string(),
        };
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}

async fn do_fetch_and_emit(app: &tauri::AppHandle, state: &AppState, interval_minutes: u32) {
    let config = state.config.lock().await;
    let org_id = config.organization_id.clone();
    let session_token = config.session_token.clone();
    let enabled = config.enabled;
    drop(config);

    if let (Some(org_id), Some(session_token)) = (org_id, session_token) {
        match fetch_usage_from_api(&org_id, &session_token).await {
            Ok(usage) => {
                // Update tray tooltip
                update_tray_tooltip_internal(app, Some(&usage));

                // Calculate next refresh time
                let next_refresh_at = if enabled {
                    Some(chrono::Utc::now().timestamp_millis() + (interval_minutes as i64 * 60 * 1000))
                } else {
                    None
                };

                // Emit usage update event
                let _ = app.emit("usage-updated", UsageUpdateEvent {
                    usage,
                    next_refresh_at,
                });
            }
            Err(e) => {
                eprintln!("Auto-refresh error: {}", e);
                let _ = app.emit("usage-error", UsageErrorEvent {
                    error: e.to_string(),
                });
            }
        }
    }
}

async fn auto_refresh_loop(app: tauri::AppHandle, state: Arc<AppState>) {
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

// ============================================================================
// Tauri Commands
// ============================================================================

#[tauri::command]
async fn get_usage(org_id: String, session_token: String) -> Result<UsageData, AppError> {
    fetch_usage_from_api(&org_id, &session_token).await
}

#[tauri::command]
fn get_default_settings() -> Settings {
    Settings::default()
}

/// Set credentials and restart auto-refresh loop
#[tauri::command]
async fn set_credentials(
    state: tauri::State<'_, Arc<AppState>>,
    org_id: Option<String>,
    session_token: Option<String>,
) -> Result<(), ()> {
    let mut config = state.config.lock().await;
    config.organization_id = org_id;
    config.session_token = session_token;
    drop(config);

    // Signal the loop to restart
    let _ = state.restart_tx.send(());
    Ok(())
}

/// Update auto-refresh settings and restart loop
#[tauri::command]
async fn set_auto_refresh(
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
async fn refresh_now(
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

// ============================================================================
// System Tray
// ============================================================================

fn create_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let show_i = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let refresh_i = MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_i, &refresh_i, &quit_i])?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Claude Monitor")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "refresh" => {
                // Trigger refresh via state
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    let _ = state.restart_tx.send(());
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();

            #[cfg(target_os = "macos")]
            {
                use tauri_plugin_nspopover::AppExt;
                // Use NSPopover on macOS for proper fullscreen support
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    if app.is_popover_shown() {
                        let _ = app.hide_popover();
                    } else {
                        let _ = app.show_popover();
                    }
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                // Track tray position for positioner plugin on other platforms
                on_tray_event(app, &event);

                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    if let Some(window) = app.get_webview_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.hide();
                        } else {
                            let _ = window.move_window(Position::TrayCenter);
                            let _ = window.set_always_on_top(true);
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

// ============================================================================
// App Entry Point
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_nspopover::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_sql::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_usage,
            get_default_settings,
            set_credentials,
            set_auto_refresh,
            refresh_now
        ])
        .setup(|app| {
            // Initialize Stronghold with argon2 key derivation
            let salt_path = app
                .path()
                .app_local_data_dir()
                .expect("could not resolve app local data path")
                .join("salt.txt");
            app.handle().plugin(
                tauri_plugin_stronghold::Builder::with_argon2(&salt_path).build(),
            )?;

            // Create app state with watch channel for restart signals
            let (restart_tx, _) = watch::channel(());
            let state = Arc::new(AppState {
                config: Mutex::new(AutoRefreshConfig::default()),
                restart_tx,
            });

            // Manage state
            app.manage(state.clone());

            // Spawn auto-refresh loop
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(auto_refresh_loop(app_handle, state));

            // Create tray (required by NSPopover plugin which looks up tray by ID "main")
            create_tray(app.handle())?;

            // Set activation policy to Accessory on macOS for proper tray app behavior
            #[cfg(target_os = "macos")]
            {
                use tauri::ActivationPolicy;
                use tauri_plugin_nspopover::{WindowExt, ToPopoverOptions};

                app.set_activation_policy(ActivationPolicy::Accessory);

                // Convert window to popover
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.to_popover(ToPopoverOptions {
                        is_fullsize_content: true,
                    });
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // On non-macOS platforms, handle window events manually
            #[cfg(not(target_os = "macos"))]
            match event {
                // Hide window when it loses focus
                tauri::WindowEvent::Focused(false) => {
                    let _ = window.hide();
                }
                // Hide window instead of closing
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    let _ = window.hide();
                    api.prevent_close();
                }
                _ => {}
            }

            // On macOS, NSPopover handles focus loss automatically
            #[cfg(target_os = "macos")]
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
