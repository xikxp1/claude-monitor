use serde::{Deserialize, Serialize};
use tokio::sync::{watch, Mutex};

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
    pub resets_at: Option<String>,
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
    pub interval_enabled: bool,
    pub interval_percent: u32,
    pub threshold_enabled: bool,
    pub thresholds: Vec<u32>,
    /// Enable time-remaining notifications (notify when close to reset)
    pub time_remaining_enabled: bool,
    /// Time thresholds in minutes (e.g., [30, 60] = notify at 30min and 1hr before reset)
    pub time_remaining_minutes: Vec<u32>,
}

impl Default for NotificationRule {
    fn default() -> Self {
        Self {
            interval_enabled: false,
            interval_percent: 10,
            threshold_enabled: true,
            thresholds: vec![80, 90],
            time_remaining_enabled: false,
            time_remaining_minutes: vec![30, 60],
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
    pub fired_thresholds: Vec<String>,
    /// Tracks fired time-remaining notifications (format: "usage_type:minutes")
    pub fired_time_remaining: Vec<String>,
}

// ============================================================================
// Auto-Refresh Types
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
    /// Notification settings
    pub notification_settings: Mutex<NotificationSettings>,
    /// Notification state (tracks what's been notified)
    pub notification_state: Mutex<NotificationState>,
}
