mod api;
mod auto_refresh;
mod commands;
mod credentials;
mod error;
mod history;
mod notifications;
mod tray;
mod types;
mod validation;

use auto_refresh::auto_refresh_loop;
use commands::{
    cleanup_history, clear_credentials, get_default_settings, get_is_configured, get_usage,
    get_usage_history_by_range, get_usage_stats, refresh_now, save_credentials, set_auto_refresh,
    set_notification_settings,
};
use tray::create_tray;
use types::{AppState, AutoRefreshConfig, NotificationSettings, NotificationState};

use specta_typescript::{BigIntExportBehavior, Typescript};
use std::backtrace::Backtrace;
use std::sync::Arc;
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_store::StoreExt;
use tauri_specta::{collect_commands, Builder};
use tokio::sync::{watch, Mutex};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = Builder::<tauri::Wry>::new().commands(collect_commands![
        get_usage,
        get_default_settings,
        save_credentials,
        get_is_configured,
        clear_credentials,
        set_auto_refresh,
        refresh_now,
        set_notification_settings,
        get_usage_history_by_range,
        get_usage_stats,
        cleanup_history
    ]);

    #[cfg(debug_assertions)]
    builder
        .export(
            Typescript::default().bigint(BigIntExportBehavior::Number),
            "../src/lib/bindings.generated.ts",
        )
        .expect("Failed to export typescript bindings");

    // Initialize platform-agnostic plugins
    let app_builder = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ));

    // Add platform-specific plugins
    #[cfg(target_os = "macos")]
    let app_builder = app_builder.plugin(tauri_plugin_nspopover::init());

    #[cfg(not(target_os = "macos"))]
    let app_builder = app_builder.plugin(tauri_plugin_positioner::init());

    // Add updater and process plugins for auto-updates
    let app_builder = app_builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    app_builder
        .invoke_handler(builder.invoke_handler())
        .setup(|app| {
            use tauri::Manager;

            // Set up panic hook to log panics before crashing
            let default_panic = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let backtrace = Backtrace::force_capture();
                log::error!("Panic: {info}\n{backtrace}");
                default_panic(info);
            }));

            // Try to load credentials from OS keychain
            let initial_credentials = credentials::load_credentials();

            // Create initial config with loaded credentials
            let initial_config = match initial_credentials {
                Some((org_id, token)) => AutoRefreshConfig {
                    organization_id: Some(org_id),
                    session_token: Some(token),
                    enabled: true,
                    interval_minutes: 5,
                },
                None => AutoRefreshConfig::default(),
            };

            // Load notification settings from store
            let notification_settings = match app.store("settings.json") {
                Ok(store) => {
                    store
                        .get("notificationSettings")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default()
                }
                Err(_) => NotificationSettings::default(),
            };

            // Load notification state from store
            let notification_state = match app.store("settings.json") {
                Ok(store) => {
                    store
                        .get("notificationState")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default()
                }
                Err(_) => NotificationState::default(),
            };

            // Initialize history database (ignore errors - analytics is non-critical)
            let _ = history::init_database(app.handle());

            // Create app state with watch channel for restart signals
            let (restart_tx, _) = watch::channel(());
            let state = Arc::new(AppState {
                config: Mutex::new(initial_config),
                restart_tx,
                notification_settings: Mutex::new(notification_settings),
                notification_state: Mutex::new(notification_state),
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
                use tauri_plugin_nspopover::{ToPopoverOptions, WindowExt};

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
