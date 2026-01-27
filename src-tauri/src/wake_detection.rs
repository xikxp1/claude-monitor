//! macOS System Wake Detection
//!
//! Monitors system wake events using NSWorkspace notifications and triggers
//! usage refresh when the system wakes from sleep.

use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{define_class, msg_send, sel, AllocAnyThread, DeclaredClass};
use objc2_app_kit::{NSWorkspace, NSWorkspaceDidWakeNotification};
use objc2_foundation::{NSNotification, NSObject};
use tokio::sync::watch;

/// Type alias for the wake callback
type WakeCallback = Box<dyn Fn() + Send + Sync + 'static>;

/// Instance variables for the WakeObserver class
pub struct WakeObserverIvars {
    wake_callback: WakeCallback,
}

define_class!(
    /// Observer class that receives system wake notifications
    #[unsafe(super(NSObject))]
    #[name = "WakeObserver"]
    #[ivars = WakeObserverIvars]
    pub struct WakeObserver;

    unsafe impl NSObjectProtocol for WakeObserver {}

    impl WakeObserver {
        #[unsafe(method(handleWakeNotification:))]
        fn handle_wake(&self, _notification: Option<&NSNotification>) {
            (self.ivars().wake_callback)();
        }
    }
);

impl WakeObserver {
    /// Create a new wake observer with a callback for wake events
    pub fn new(wake_callback: impl Fn() + Send + Sync + 'static) -> Retained<Self> {
        let observer = Self::alloc().set_ivars(WakeObserverIvars {
            wake_callback: Box::new(wake_callback),
        });

        // Initialize the NSObject
        let observer: Retained<Self> = unsafe { msg_send![super(observer), init] };

        // Register for wake notification
        unsafe {
            let workspace = NSWorkspace::sharedWorkspace();
            let notification_center = workspace.notificationCenter();

            notification_center.addObserver_selector_name_object(
                &observer,
                sel!(handleWakeNotification:),
                Some(NSWorkspaceDidWakeNotification),
                None,
            );
        }

        observer
    }
}

impl Drop for WakeObserver {
    fn drop(&mut self) {
        // Unregister from notification center
        unsafe {
            let workspace = NSWorkspace::sharedWorkspace();
            let notification_center = workspace.notificationCenter();
            notification_center.removeObserver(self);
        }
    }
}

/// Start monitoring system wake events.
/// Returns a handle that must be kept alive to continue receiving notifications.
pub fn start_wake_monitor(restart_tx: watch::Sender<()>) -> Retained<WakeObserver> {
    WakeObserver::new(move || {
        log::info!("System wake detected, triggering refresh");
        let _ = restart_tx.send(());
    })
}
