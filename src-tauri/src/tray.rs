use crate::types::{AppState, UsageData};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, Runtime,
};
#[cfg(not(target_os = "macos"))]
use tauri_plugin_positioner::{on_tray_event, Position, WindowExt};

pub fn update_tray_tooltip<R: Runtime>(app: &tauri::AppHandle<R>, usage: Option<&UsageData>) {
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

pub fn create_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
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
