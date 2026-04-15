use crate::error::AppError;
use crate::types::{ProviderKind, ProviderStatus, UsageSnapshot, UsageWindow};
use crate::validation::{validate_org_id, validate_session_token};
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ClaudeUsageData {
    five_hour: Option<ClaudeUsagePeriod>,
    seven_day: Option<ClaudeUsagePeriod>,
    seven_day_sonnet: Option<ClaudeUsagePeriod>,
    seven_day_opus: Option<ClaudeUsagePeriod>,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsagePeriod {
    utilization: f64,
    resets_at: Option<String>,
}

pub async fn fetch_usage(
    org_id: Option<&str>,
    session_token: Option<&str>,
) -> Result<UsageSnapshot, AppError> {
    let org_id = org_id.ok_or_else(|| AppError::MissingConfig("organization_id".to_string()))?;
    let session_token =
        session_token.ok_or_else(|| AppError::MissingConfig("session_token".to_string()))?;

    validate_org_id(org_id)?;
    validate_session_token(session_token)?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Claude-Monitor/0.1.0"));
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("sessionKey={session_token}"))
            .map_err(|_| AppError::InvalidToken)?,
    );

    let url = format!("https://claude.ai/api/organizations/{org_id}/usage");
    let response = client.get(&url).headers(headers).send().await?;
    let status = response.status().as_u16();

    match status {
        200 => {
            let body = response.text().await?;
            let usage: ClaudeUsageData = serde_json::from_str(&body).map_err(|e| {
                log::error!("Failed to parse Claude usage response: {e}");
                AppError::Server(format!("Failed to parse response: {e}"))
            })?;

            Ok(UsageSnapshot {
                provider: ProviderKind::Claude,
                windows: [
                    map_window("five_hour", "5 Hour", usage.five_hour),
                    map_window("seven_day", "7 Day", usage.seven_day),
                    map_window("seven_day_sonnet", "Sonnet (7 Day)", usage.seven_day_sonnet),
                    map_window("seven_day_opus", "Opus (7 Day)", usage.seven_day_opus),
                ]
                .into_iter()
                .flatten()
                .collect(),
                account_email: None,
                plan_type: None,
            })
        }
        401 => {
            log::error!("Claude usage request returned authentication failure (HTTP 401)");
            Err(AppError::InvalidToken)
        }
        429 => {
            log::warn!("Claude usage request was rate limited (HTTP 429)");
            Err(AppError::RateLimited)
        }
        403 => {
            log::error!("Claude usage request returned HTTP 403 for org_id {org_id}");
            Err(AppError::Server(
                "Access denied. Check your organization ID.".to_string(),
            ))
        }
        404 => {
            log::error!("Claude usage request returned HTTP 404 for org_id {org_id}");
            Err(AppError::Server(
                "Organization not found. Check your organization ID.".to_string(),
            ))
        }
        500..=599 => {
            log::error!("Claude usage request failed with server error HTTP {status}");
            Err(AppError::Server(
                "Claude is experiencing issues. Please try again later.".to_string(),
            ))
        }
        status => {
            log::error!("Claude usage request failed with unexpected HTTP status {status}");
            Err(AppError::Server(format!(
                "Unexpected error (HTTP {status}). Please try again."
            )))
        }
    }
}

pub fn get_status(org_id: Option<&str>, session_token: Option<&str>) -> ProviderStatus {
    let configured = org_id.is_some() && session_token.is_some();
    ProviderStatus {
        provider: ProviderKind::Claude,
        configured,
        source: "keychain".to_string(),
        message: if configured {
            None
        } else {
            Some("Add your Claude organization ID and session token.".to_string())
        },
    }
}

fn map_window(key: &str, label: &str, period: Option<ClaudeUsagePeriod>) -> Option<UsageWindow> {
    let period = period?;
    Some(UsageWindow {
        key: key.to_string(),
        label: label.to_string(),
        utilization: period.utilization,
        resets_at: period.resets_at,
        window_duration_seconds: None,
    })
}
