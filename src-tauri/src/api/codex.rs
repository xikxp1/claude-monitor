use crate::error::AppError;
use crate::types::{ProviderKind, ProviderStatus, UsageSnapshot, UsageWindow};
use chrono::{DateTime, Utc};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct CodexAuthFile {
    tokens: CodexTokens,
}

#[derive(Debug, Deserialize)]
struct CodexTokens {
    access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WhamUsageResponse {
    email: Option<String>,
    plan_type: Option<String>,
    rate_limit: Option<WhamRateLimit>,
}

#[derive(Debug, Deserialize)]
struct WhamRateLimit {
    primary_window: Option<WhamRateLimitWindow>,
    secondary_window: Option<WhamRateLimitWindow>,
}

#[derive(Debug, Deserialize)]
struct WhamRateLimitWindow {
    used_percent: f64,
    #[serde(deserialize_with = "deserialize_reset_at")]
    reset_at: Option<String>,
    limit_window_seconds: Option<i64>,
}

pub async fn fetch_usage() -> Result<UsageSnapshot, AppError> {
    let access_token = load_access_token()?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Claude-Monitor/0.1.0"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {access_token}"))
            .map_err(|_| AppError::InvalidToken)?,
    );

    let response = client
        .get("https://chatgpt.com/backend-api/wham/usage")
        .headers(headers)
        .send()
        .await?;

    match response.status().as_u16() {
        200 => {
            let body = response.text().await?;
            let usage: WhamUsageResponse = serde_json::from_str(&body).map_err(|e| {
                log::error!("Failed to parse Codex WHAM usage response: {e}");
                AppError::Server(format!("Failed to parse Codex usage: {e}"))
            })?;

            Ok(UsageSnapshot {
                provider: ProviderKind::Codex,
                windows: usage.rate_limit.map(map_windows).unwrap_or_default(),
                account_email: usage.email,
                plan_type: usage.plan_type,
            })
        }
        status @ (401 | 403) => {
            log::error!("Codex usage request returned authentication failure (HTTP {status})");
            Err(AppError::InvalidToken)
        }
        429 => {
            log::warn!("Codex usage request was rate limited (HTTP 429)");
            Err(AppError::RateLimited)
        }
        status @ 500..=599 => {
            log::error!("Codex usage request failed with server error HTTP {status}");
            Err(AppError::Server(
                "OpenAI is experiencing issues. Please try again later.".to_string(),
            ))
        }
        status => {
            log::error!("Codex usage request failed with unexpected HTTP status {status}");
            Err(AppError::Server(format!(
                "Unexpected Codex error (HTTP {status}). Please try again."
            )))
        }
    }
}

pub fn get_status() -> ProviderStatus {
    match load_access_token() {
        Ok(_) => ProviderStatus {
            provider: ProviderKind::Codex,
            configured: true,
            source: "auth-json".to_string(),
            message: None,
        },
        Err(_) => ProviderStatus {
            provider: ProviderKind::Codex,
            configured: false,
            source: "auth-json".to_string(),
            message: Some("Run `codex login` to enable Codex monitoring.".to_string()),
        },
    }
}

fn map_windows(rate_limit: WhamRateLimit) -> Vec<UsageWindow> {
    [
        map_window("primary", rate_limit.primary_window),
        map_window("secondary", rate_limit.secondary_window),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn map_window(key: &str, window: Option<WhamRateLimitWindow>) -> Option<UsageWindow> {
    let window = window?;
    let label = label_for_window(window.limit_window_seconds, key);

    Some(UsageWindow {
        key: key.to_string(),
        label,
        utilization: window.used_percent,
        resets_at: window.reset_at,
        window_duration_seconds: window.limit_window_seconds,
    })
}

fn label_for_window(duration_seconds: Option<i64>, fallback_key: &str) -> String {
    match duration_seconds {
        Some(18_000) => "5 Hour".to_string(),
        Some(604_800) => "7 Day".to_string(),
        Some(seconds) if seconds > 0 => {
            let hours = seconds / 3600;
            if hours >= 24 {
                format!("{} Day", hours / 24)
            } else {
                format!("{hours} Hour")
            }
        }
        _ if fallback_key == "primary" => "Primary Window".to_string(),
        _ => "Secondary Window".to_string(),
    }
}

pub fn load_access_token() -> Result<String, AppError> {
    let auth_path = get_auth_path();
    let raw = std::fs::read_to_string(&auth_path).map_err(|_| {
        log::error!(
            "Codex auth file was not found or could not be read at {}",
            auth_path.display()
        );
        AppError::Server("Codex auth not found. Run `codex login` first.".to_string())
    })?;
    let auth: CodexAuthFile = serde_json::from_str(&raw).map_err(|e| {
        log::error!(
            "Failed to parse Codex auth file at {}: {e}",
            auth_path.display()
        );
        AppError::Server(format!("Failed to parse Codex auth.json: {e}"))
    })?;

    auth.tokens
        .access_token
        .filter(|token| !token.is_empty())
        .ok_or_else(|| {
            log::error!(
                "Codex auth file at {} did not contain a usable access token",
                auth_path.display()
            );
            AppError::Server(
                "Codex auth is missing an access token. Run `codex login` again.".to_string(),
            )
        })
}

fn get_auth_path() -> PathBuf {
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        return PathBuf::from(codex_home).join("auth.json");
    }

    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".codex").join("auth.json")
}

fn deserialize_reset_at<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };

    match value {
        Value::Null => Ok(None),
        Value::String(s) => Ok(Some(s)),
        Value::Number(n) => {
            let seconds = n
                .as_i64()
                .ok_or_else(|| serde::de::Error::custom("invalid numeric reset_at value"))?;
            let timestamp = DateTime::<Utc>::from_timestamp(seconds, 0)
                .ok_or_else(|| serde::de::Error::custom("reset_at timestamp out of range"))?;
            Ok(Some(timestamp.to_rfc3339()))
        }
        other => Err(serde::de::Error::custom(format!(
            "unsupported reset_at type: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_window_labels() {
        assert_eq!(label_for_window(Some(18_000), "primary"), "5 Hour");
        assert_eq!(label_for_window(Some(604_800), "secondary"), "7 Day");
    }

    #[test]
    fn maps_unknown_windows_to_duration() {
        assert_eq!(label_for_window(Some(7_200), "primary"), "2 Hour");
        assert_eq!(label_for_window(Some(172_800), "secondary"), "2 Day");
    }

    #[test]
    fn parses_numeric_reset_at() {
        let json = r#"{
            "used_percent": 42.0,
            "reset_at": 1776048334,
            "limit_window_seconds": 18000
        }"#;

        let parsed: WhamRateLimitWindow = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.used_percent, 42.0);
        assert_eq!(parsed.limit_window_seconds, Some(18_000));
        assert!(parsed.reset_at.as_deref().unwrap().starts_with("2026-04-"));
    }

    #[test]
    fn preserves_string_reset_at() {
        let json = r#"{
            "used_percent": 42.0,
            "reset_at": "2026-04-12T10:45:34Z",
            "limit_window_seconds": 18000
        }"#;

        let parsed: WhamRateLimitWindow = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.reset_at.as_deref(), Some("2026-04-12T10:45:34Z"));
    }
}
