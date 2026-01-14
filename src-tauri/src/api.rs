use crate::error::AppError;
use crate::types::UsageData;
use crate::validation::{validate_org_id, validate_session_token};
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, USER_AGENT};

pub async fn fetch_usage_from_api(org_id: &str, session_token: &str) -> Result<UsageData, AppError> {
    // Validate inputs before using in HTTP request
    validate_org_id(org_id)?;
    validate_session_token(session_token)?;

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

    let url = format!("https://claude.ai/api/organizations/{}/usage", org_id);

    let response = client.get(&url).headers(headers).send().await?;
    let status = response.status().as_u16();

    match status {
        200 => {
            let body = response.text().await?;
            serde_json::from_str::<UsageData>(&body)
                .map_err(|e| AppError::Server(format!("Failed to parse response: {}", e)))
        }
        401 => Err(AppError::InvalidToken),
        429 => Err(AppError::RateLimited),
        status => {
            Err(AppError::Server(format!("HTTP {}", status)))
        }
    }
}
