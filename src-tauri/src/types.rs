use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;
use std::collections::BTreeMap;
use tokio::sync::{Mutex, watch};

#[cfg(target_os = "macos")]
use objc2::rc::Retained;

// ============================================================================
// Provider & Usage Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Claude,
    Codex,
    Ollama,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Ollama => "ollama",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub key: String,
    pub label: String,
    pub utilization: f64,
    pub resets_at: Option<String>,
    pub window_duration_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub provider: ProviderKind,
    pub windows: Vec<UsageWindow>,
    pub account_email: Option<String>,
    pub plan_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProviderStatus {
    pub provider: ProviderKind,
    pub configured: bool,
    pub source: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Settings {
    pub active_provider: ProviderKind,
    pub refresh_interval_minutes: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            active_provider: ProviderKind::Claude,
            refresh_interval_minutes: 5,
        }
    }
}

// ============================================================================
// Notification Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct NotificationRule {
    pub interval_enabled: bool,
    pub interval_percent: u32,
    pub threshold_enabled: bool,
    pub thresholds: Vec<u32>,
    pub time_remaining_enabled: bool,
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

#[derive(Debug, Clone, Serialize, Type)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub rules: BTreeMap<String, NotificationRule>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct LegacyNotificationSettings {
    enabled: Option<bool>,
    five_hour: Option<NotificationRule>,
    seven_day: Option<NotificationRule>,
    seven_day_sonnet: Option<NotificationRule>,
    seven_day_opus: Option<NotificationRule>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NotificationSettingsSerde {
    Current {
        enabled: bool,
        rules: BTreeMap<String, NotificationRule>,
    },
    Legacy(LegacyNotificationSettings),
}

impl<'de> Deserialize<'de> for NotificationSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let parsed = NotificationSettingsSerde::deserialize(deserializer)?;
        Ok(match parsed {
            NotificationSettingsSerde::Current { enabled, rules } => Self { enabled, rules },
            NotificationSettingsSerde::Legacy(legacy) => {
                let mut rules = BTreeMap::new();
                if let Some(rule) = legacy.five_hour {
                    rules.insert("claude:five_hour".to_string(), rule);
                }
                if let Some(rule) = legacy.seven_day {
                    rules.insert("claude:seven_day".to_string(), rule);
                }
                if let Some(rule) = legacy.seven_day_sonnet {
                    rules.insert("claude:seven_day_sonnet".to_string(), rule);
                }
                if let Some(rule) = legacy.seven_day_opus {
                    rules.insert("claude:seven_day_opus".to_string(), rule);
                }

                Self {
                    enabled: legacy.enabled.unwrap_or(true),
                    rules,
                }
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NotificationState {
    pub last_notified: BTreeMap<String, f64>,
    pub fired_thresholds: Vec<String>,
    pub fired_time_remaining: Vec<String>,
}

// ============================================================================
// Auto-Refresh Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoRefreshConfig {
    pub active_provider: ProviderKind,
    pub organization_id: Option<String>,
    pub session_token: Option<String>,
    pub ollama_session_token: Option<String>,
    pub enabled: bool,
    pub interval_minutes: u32,
    pub hourly_refresh_enabled: bool,
}

impl Default for AutoRefreshConfig {
    fn default() -> Self {
        Self {
            active_provider: ProviderKind::Claude,
            organization_id: None,
            session_token: None,
            ollama_session_token: None,
            enabled: true,
            interval_minutes: 5,
            hourly_refresh_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UsageUpdateEvent {
    pub usage: UsageSnapshot,
    pub next_refresh_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UsageErrorEvent {
    pub provider: ProviderKind,
    pub error: String,
}

pub struct AppState {
    pub config: Mutex<AutoRefreshConfig>,
    pub restart_tx: watch::Sender<()>,
    pub notification_settings: Mutex<NotificationSettings>,
    pub notification_state: Mutex<NotificationState>,
    #[cfg(target_os = "macos")]
    pub wake_observer: Mutex<Option<Retained<crate::wake_detection::WakeObserver>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_legacy_notification_settings() {
        let json = r#"{
            "enabled": true,
            "five_hour": {
                "interval_enabled": false,
                "interval_percent": 10,
                "threshold_enabled": true,
                "thresholds": [80, 90],
                "time_remaining_enabled": false,
                "time_remaining_minutes": [30, 60]
            }
        }"#;

        let parsed: NotificationSettings = serde_json::from_str(json).unwrap();
        assert!(parsed.enabled);
        assert!(parsed.rules.contains_key("claude:five_hour"));
    }
}
