//! Cross-platform system wake detection
//!
//! This module listens for system wake notifications and triggers
//! a refresh of the usage API when the system wakes from sleep.
//!
//! Platform implementations:
//! - macOS: NSWorkspaceDidWakeNotification via objc2
//! - Windows: WM_POWERBROADCAST via windows crate
//! - Linux: org.freedesktop.login1.PrepareForSleep via zbus

use tokio::sync::watch;

// ============================================================================
// macOS Implementation
// ============================================================================

#[cfg(target_os = "macos")]
mod macos {
    use objc2::rc::Retained;
    use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSNotification, NSNotificationCenter, NSNotificationName, NSObject};
    use std::cell::Cell;
    use tokio::sync::watch;

    struct WakeObserverIvars {
        sender: Cell<Option<watch::Sender<()>>>,
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "WakeObserver"]
        #[ivars = WakeObserverIvars]
        struct WakeObserver;

        impl WakeObserver {
            #[unsafe(method(handleWake:))]
            fn __handle_wake(&self, _notification: &NSNotification) {
                log::info!("macOS: System wake detected, triggering refresh");
                if let Some(sender) = self.ivars().sender.take() {
                    let _ = sender.send(());
                    self.ivars().sender.set(Some(sender));
                }
            }
        }
    );

    impl WakeObserver {
        fn new(sender: watch::Sender<()>, mtm: MainThreadMarker) -> Retained<Self> {
            let this = Self::alloc(mtm);
            let this = this.set_ivars(WakeObserverIvars {
                sender: Cell::new(Some(sender)),
            });
            unsafe { msg_send![super(this), init] }
        }
    }

    pub fn start_wake_listener(restart_tx: watch::Sender<()>) {
        let Some(mtm) = MainThreadMarker::new() else {
            log::warn!("Wake detection must be initialized from the main thread");
            return;
        };

        let observer = WakeObserver::new(restart_tx, mtm);

        unsafe {
            let workspace = NSWorkspace::sharedWorkspace();
            let notification_center: Retained<NSNotificationCenter> =
                msg_send![&workspace, notificationCenter];
            let notification_name: &NSNotificationName =
                &NSNotificationName::from_str("NSWorkspaceDidWakeNotification");

            notification_center.addObserver_selector_name_object(
                &observer,
                sel!(handleWake:),
                Some(notification_name),
                None,
            );

            log::info!("macOS wake detection listener started");
            std::mem::forget(observer);
        }
    }
}

// ============================================================================
// Windows Implementation
// ============================================================================

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::sync::OnceLock;
    use tokio::sync::watch;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Power::{PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
        TranslateMessage, CW_USEDEFAULT, HWND_MESSAGE, MSG, WM_POWERBROADCAST, WNDCLASSW,
        WS_OVERLAPPED,
    };
    use windows::core::{w, PCWSTR};

    static RESTART_TX: OnceLock<watch::Sender<()>> = OnceLock::new();

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_POWERBROADCAST {
            let event = wparam.0 as u32;
            if event == PBT_APMRESUMEAUTOMATIC || event == PBT_APMRESUMESUSPEND {
                log::info!("Windows: System wake detected (event: {}), triggering refresh", event);
                if let Some(tx) = RESTART_TX.get() {
                    let _ = tx.send(());
                }
            }
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    pub fn start_wake_listener(restart_tx: watch::Sender<()>) {
        let _ = RESTART_TX.set(restart_tx);

        std::thread::spawn(|| {
            unsafe {
                let instance = GetModuleHandleW(None).unwrap_or_default();
                let class_name = w!("ClaudeMonitorWakeDetector");

                let wc = WNDCLASSW {
                    lpfnWndProc: Some(window_proc),
                    hInstance: instance.into(),
                    lpszClassName: class_name,
                    ..Default::default()
                };

                if RegisterClassW(&wc) == 0 {
                    log::error!("Windows: Failed to register window class for wake detection");
                    return;
                }

                let hwnd = CreateWindowExW(
                    Default::default(),
                    class_name,
                    PCWSTR::null(),
                    WS_OVERLAPPED,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    HWND_MESSAGE, // Message-only window
                    None,
                    Some(instance.into()),
                    None,
                );

                if hwnd.is_err() || hwnd.as_ref().map(|h| h.is_invalid()).unwrap_or(true) {
                    log::error!("Windows: Failed to create message window for wake detection");
                    return;
                }

                log::info!("Windows wake detection listener started");

                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        });
    }
}

// ============================================================================
// Linux Implementation
// ============================================================================

#[cfg(target_os = "linux")]
mod linux {
    use tokio::sync::watch;
    use zbus::{Connection, proxy};

    #[proxy(
        interface = "org.freedesktop.login1.Manager",
        default_service = "org.freedesktop.login1",
        default_path = "/org/freedesktop/login1"
    )]
    trait Manager {
        #[zbus(signal)]
        fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
    }

    pub fn start_wake_listener(restart_tx: watch::Sender<()>) {
        tauri::async_runtime::spawn(async move {
            if let Err(e) = run_listener(restart_tx).await {
                log::error!("Linux wake detection error: {}", e);
            }
        });
    }

    async fn run_listener(restart_tx: watch::Sender<()>) -> zbus::Result<()> {
        let connection = Connection::system().await?;
        let proxy = ManagerProxy::new(&connection).await?;
        let mut stream = proxy.receive_prepare_for_sleep().await?;

        log::info!("Linux wake detection listener started");

        use futures_util::StreamExt;
        while let Some(signal) = stream.next().await {
            if let Ok(args) = signal.args() {
                // start=false means waking up from sleep
                if !args.start {
                    log::info!("Linux: System wake detected, triggering refresh");
                    let _ = restart_tx.send(());
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Start listening for system wake notifications.
/// Triggers a refresh signal when the system wakes from sleep.
#[cfg(target_os = "macos")]
pub fn start_wake_listener(restart_tx: watch::Sender<()>) {
    macos::start_wake_listener(restart_tx);
}

#[cfg(target_os = "windows")]
pub fn start_wake_listener(restart_tx: watch::Sender<()>) {
    windows_impl::start_wake_listener(restart_tx);
}

#[cfg(target_os = "linux")]
pub fn start_wake_listener(restart_tx: watch::Sender<()>) {
    linux::start_wake_listener(restart_tx);
}

/// No-op on unsupported platforms
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn start_wake_listener(_restart_tx: watch::Sender<()>) {
    log::warn!("Wake detection is not supported on this platform");
}
