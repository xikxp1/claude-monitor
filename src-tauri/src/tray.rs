use crate::types::UsageSnapshot;
#[cfg(not(target_os = "macos"))]
use tauri::Manager;
use tauri::{
    Emitter, Runtime,
    menu::{Menu, MenuEvent, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
#[cfg(not(target_os = "macos"))]
use tauri_plugin_positioner::{Position, WindowExt, on_tray_event};

pub fn update_tray_tooltip<R: Runtime>(app: &tauri::AppHandle<R>, usage: Option<&UsageSnapshot>) {
    if let Some(tray) = app.tray_by_id("main") {
        let tooltip = match usage {
            Some(snapshot) => {
                let parts = snapshot
                    .windows
                    .iter()
                    .map(|window| format!("{}: {:.0}%", window.label, window.utilization))
                    .collect::<Vec<_>>();

                let provider_name = match snapshot.provider {
                    crate::types::ProviderKind::Claude => "Claude Monitor",
                    crate::types::ProviderKind::Codex => "Codex Monitor",
                    crate::types::ProviderKind::Ollama => "Ollama Monitor",
                };

                if parts.is_empty() {
                    provider_name.to_string()
                } else {
                    format!("{provider_name}\n{}", parts.join(" | "))
                }
            }
            None => "Claude Monitor".to_string(),
        };
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}

fn handle_menu_event<R: Runtime>(app: &tauri::AppHandle<R>, event: MenuEvent) {
    if event.id().as_ref() == "check_updates" {
        // Emit event to frontend to trigger update check
        let _ = app.emit("check-for-updates", ());
    }
}

pub fn create_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    // Get app name and version
    let package_info = app.package_info();
    let app_label = format!("{} v{}", package_info.name, package_info.version);

    // Create menu items
    let app_info = MenuItemBuilder::with_id("app_info", &app_label)
        .enabled(false)
        .build(app)?;
    let check_updates =
        MenuItemBuilder::with_id("check_updates", "Check for Updates").build(app)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_i = PredefinedMenuItem::quit(app, Some("Quit"))?;

    let menu = Menu::with_items(app, &[&app_info, &check_updates, &separator, &quit_i])?;

    let icon = app
        .default_window_icon()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?
        .clone();

    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Claude Monitor")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
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
