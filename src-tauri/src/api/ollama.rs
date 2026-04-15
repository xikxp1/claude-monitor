use crate::error::AppError;
use crate::types::{ProviderKind, ProviderStatus, UsageSnapshot, UsageWindow};
use regex::Regex;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, USER_AGENT};
use scraper::{Html, Selector};

/// Cookie name for Ollama session authentication.
/// This may need to be updated if Ollama changes their cookie naming.
const OLLAMA_COOKIE_NAME: &str = "__Secure-session";

struct OllamaSettingsData {
    plan_type: Option<String>,
    session_usage: Option<f64>,
    session_resets_at: Option<String>,
    weekly_usage: Option<f64>,
    weekly_resets_at: Option<String>,
    account_email: Option<String>,
}

pub async fn fetch_usage(session_token: &str) -> Result<UsageSnapshot, AppError> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Claude-Monitor/0.1.0"));
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("{}={session_token}", OLLAMA_COOKIE_NAME))
            .map_err(|_| AppError::InvalidToken)?,
    );

    let response = client
        .get("https://ollama.com/settings")
        .headers(headers)
        .send()
        .await?;

    match response.status().as_u16() {
        200 => {
            let html = response.text().await?;
            let data = parse_ollama_settings(&html)?;
            Ok(UsageSnapshot {
                provider: ProviderKind::Ollama,
                windows: build_windows(&data),
                account_email: data.account_email,
                plan_type: data.plan_type,
            })
        }
        401 | 403 => {
            log::error!(
                "Ollama settings request returned authentication failure (HTTP {})",
                response.status()
            );
            Err(AppError::InvalidToken)
        }
        429 => {
            log::warn!("Ollama settings request was rate limited (HTTP 429)");
            Err(AppError::RateLimited)
        }
        status @ 500..=599 => {
            log::error!("Ollama settings request failed with server error (HTTP {status})");
            Err(AppError::Server(
                "Ollama is experiencing issues. Please try again later.".to_string(),
            ))
        }
        status => {
            log::error!("Ollama settings request failed with unexpected HTTP status {status}");
            Err(AppError::Server(format!(
                "Unexpected error (HTTP {status}). Please try again."
            )))
        }
    }
}

pub fn get_status(session_token: Option<&str>) -> ProviderStatus {
    let configured = session_token.is_some();
    ProviderStatus {
        provider: ProviderKind::Ollama,
        configured,
        source: "keychain".to_string(),
        message: if configured {
            None
        } else {
            Some("Add your Ollama session cookie to enable monitoring.".to_string())
        },
    }
}

fn parse_ollama_settings(html: &str) -> Result<OllamaSettingsData, AppError> {
    let document = Html::parse_document(html);

    let plan_type = parse_plan_type(&document);
    let account_email = parse_email(&document);

    // Parse usage sections: find "Session usage" and "Weekly usage" labels,
    // then extract the adjacent percentage and reset time
    let (session_usage, session_resets_at) = parse_window_section(&document, "Session usage");
    let (weekly_usage, weekly_resets_at) = parse_window_section(&document, "Weekly usage");

    if session_usage.is_none() && weekly_usage.is_none() {
        log::warn!("Ollama settings page did not contain any usage data");
    }

    Ok(OllamaSettingsData {
        plan_type,
        session_usage,
        session_resets_at,
        weekly_usage,
        weekly_resets_at,
        account_email,
    })
}

/// Parse the plan type from the badge element near the "Cloud Usage" heading.
/// The badge looks like: <span class="text-xs font-normal px-2 py-0.5 rounded-full bg-neutral-100 text-neutral-600 capitalize">pro</span>
fn parse_plan_type(document: &Html) -> Option<String> {
    let selector = Selector::parse("span.capitalize").ok()?;
    for element in document.select(&selector) {
        let text = element.text().collect::<String>();
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

/// Parse the account email from the <h2 id="header-email"> element.
fn parse_email(document: &Html) -> Option<String> {
    let selector = Selector::parse("#header-email").ok()?;
    let element = document.select(&selector).next()?;
    let text = element.text().collect::<String>();
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Parse a usage section (Session or Weekly) by finding the label text,
/// then extracting the percentage from the adjacent span and the reset
/// timestamp from the data-time attribute.
fn parse_window_section(document: &Html, label: &str) -> (Option<f64>, Option<String>) {
    let usage = find_usage_percentage(document, label);
    let resets_at = find_reset_time(document, label);
    (usage, resets_at)
}

/// Find the usage percentage near a label like "Session usage" or "Weekly usage".
/// The HTML structure is:
///   <span class="text-sm">Session usage</span>
///   <span class="text-sm">0.1% used</span>
/// We search all text-sm spans and look for the one after the label.
fn find_usage_percentage(document: &Html, label: &str) -> Option<f64> {
    let span_selector = Selector::parse("span.text-sm").ok()?;

    let label_lower = label.to_lowercase();
    let mut found_label = false;

    for span in document.select(&span_selector) {
        let text = span.text().collect::<String>();
        let trimmed = text.trim().to_lowercase();

        if trimmed.contains(&label_lower) {
            found_label = true;
            continue;
        }

        if found_label {
            // This is the span right after the label — it should contain "X% used"
            let pct = parse_percentage(&text);
            return pct;
        }
    }

    None
}

/// Find the reset timestamp near a usage section.
/// The HTML structure is:
///   <div class="text-xs text-neutral-500 mt-1 local-time" data-time="2026-04-15T13:00:00Z">Resets in 4 hours</div>
fn find_reset_time(document: &Html, label: &str) -> Option<String> {
    let div_selector = Selector::parse("div.local-time").ok()?;
    let label_lower = label.to_lowercase();

    // Collect all local-time divs and usage sections, then match by order
    // Session usage comes first, Weekly usage comes second
    let local_times: Vec<_> = document.select(&div_selector).collect();

    // Determine which index this label corresponds to
    let index = if label_lower.contains("session") {
        0
    } else {
        1
    };

    if let Some(element) = local_times.into_iter().nth(index) {
        if let Some(time_val) = element.value().attr("data-time") {
            let trimmed = time_val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

/// Parse a percentage string like "0.1% used" or "0% used" into a float.
fn parse_percentage(text: &str) -> Option<f64> {
    let re = Regex::new(r"(\d+(?:\.\d+)?)\s*%").ok()?;
    let caps = re.captures(text)?;
    let value_str = caps.get(1)?.as_str();
    value_str.parse().ok()
}

fn build_windows(data: &OllamaSettingsData) -> Vec<UsageWindow> {
    let mut windows = Vec::new();

    if let Some(utilization) = data.session_usage {
        windows.push(UsageWindow {
            key: "session".to_string(),
            label: "Session".to_string(),
            utilization,
            resets_at: data.session_resets_at.clone(),
            window_duration_seconds: None,
        });
    }

    if let Some(utilization) = data.weekly_usage {
        windows.push(UsageWindow {
            key: "weekly".to_string(),
            label: "Weekly".to_string(),
            utilization,
            resets_at: data.weekly_resets_at.clone(),
            window_duration_seconds: None,
        });
    }

    windows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plan_type_from_html() {
        let html = r#"<span class="text-xs font-normal px-2 py-0.5 rounded-full bg-neutral-100 text-neutral-600 capitalize">pro</span>"#;
        let doc = Html::parse_document(html);
        assert_eq!(parse_plan_type(&doc), Some("pro".to_string()));
    }

    #[test]
    fn parses_email_from_html() {
        let html = r#"<h2 id="header-email" class="text-neutral-800 text-sm truncate">user@example.com</h2>"#;
        let doc = Html::parse_document(html);
        assert_eq!(parse_email(&doc), Some("user@example.com".to_string()));
    }

    #[test]
    fn parse_percentage_extracts_value() {
        assert_eq!(parse_percentage("0.1% used"), Some(0.1));
        assert_eq!(parse_percentage("0% used"), Some(0.0));
        assert_eq!(parse_percentage("50.5% used"), Some(50.5));
        assert_eq!(parse_percentage("100% used"), Some(100.0));
    }

    #[test]
    fn parse_percentage_returns_none_for_invalid() {
        assert_eq!(parse_percentage("no percentage here"), None);
    }

    #[test]
    fn get_status_configured_when_token_present() {
        let status = get_status(Some("test-token"));
        assert!(status.configured);
        assert_eq!(status.source, "keychain");
        assert!(status.message.is_none());
    }

    #[test]
    fn get_status_not_configured_when_no_token() {
        let status = get_status(None);
        assert!(!status.configured);
        assert!(status.message.is_some());
    }

    #[test]
    fn parses_full_settings_page() {
        let html = include_str!("../../tests/ollama_settings_sample.html");
        let doc = Html::parse_document(html);
        let (session_pct, session_reset) = parse_window_section(&doc, "Session usage");
        let (weekly_pct, weekly_reset) = parse_window_section(&doc, "Weekly usage");

        assert_eq!(session_pct, Some(0.1));
        assert_eq!(session_reset, Some("2026-04-15T13:00:00Z".to_string()));
        assert_eq!(weekly_pct, Some(0.0));
        assert_eq!(weekly_reset, Some("2026-04-20T00:00:00Z".to_string()));
    }
}
