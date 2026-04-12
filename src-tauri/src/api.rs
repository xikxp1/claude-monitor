mod claude;
mod codex;

use crate::error::AppError;
use crate::types::{ProviderKind, ProviderStatus, UsageSnapshot};

pub async fn fetch_usage_for_provider(
    provider: ProviderKind,
    org_id: Option<&str>,
    session_token: Option<&str>,
) -> Result<UsageSnapshot, AppError> {
    match provider {
        ProviderKind::Claude => claude::fetch_usage(org_id, session_token).await,
        ProviderKind::Codex => codex::fetch_usage().await,
    }
}

pub fn get_provider_statuses(
    claude_org_id: Option<&str>,
    claude_session_token: Option<&str>,
) -> Vec<ProviderStatus> {
    vec![
        claude::get_status(claude_org_id, claude_session_token),
        codex::get_status(),
    ]
}
