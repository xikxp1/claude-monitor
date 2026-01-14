use crate::error::AppError;

/// Validate session token format to prevent HTTP header injection.
/// Allows alphanumeric characters, hyphens, underscores, periods, and base64 chars (+, /, =).
pub fn validate_session_token(token: &str) -> Result<(), AppError> {
    if token.is_empty() {
        return Err(AppError::InvalidToken);
    }

    if token.len() > 4096 {
        return Err(AppError::InvalidToken);
    }

    for c in token.chars() {
        if !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '+' | '/' | '=') {
            return Err(AppError::InvalidToken);
        }
    }

    Ok(())
}

/// Validate organization ID format (UUID-like).
pub fn validate_org_id(org_id: &str) -> Result<(), AppError> {
    if org_id.is_empty() {
        return Err(AppError::MissingConfig("organization_id".to_string()));
    }

    if org_id.len() > 128 {
        return Err(AppError::MissingConfig("organization_id too long".to_string()));
    }

    for c in org_id.chars() {
        if !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_') {
            return Err(AppError::MissingConfig(
                "invalid organization_id format".to_string(),
            ));
        }
    }

    Ok(())
}
