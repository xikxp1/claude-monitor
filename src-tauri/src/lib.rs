use reqwest::header::{HeaderMap, HeaderValue, COOKIE, USER_AGENT};
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, Runtime,
};
use thiserror::Error;

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

    match response.status().as_u16() {
        200 => {
            let data: UsageData = response.json().await?;
            Ok(data)
        }
        401 => Err(AppError::InvalidToken),
        429 => Err(AppError::RateLimited),
        status => Err(AppError::Server(format!("HTTP {}", status))),
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

// ============================================================================
// System Tray
// ============================================================================

fn create_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let show_i = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let refresh_i = MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_i, &refresh_i, &quit_i])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
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
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("refresh-usage", ());
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
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
        .invoke_handler(tauri::generate_handler![get_usage, get_default_settings])
        .setup(|app| {
            create_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide window instead of closing on macOS
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                #[cfg(target_os = "macos")]
                {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
