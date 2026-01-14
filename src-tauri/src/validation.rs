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

#[cfg(test)]
mod tests {
    use super::*;

    mod validate_session_token {
        use super::*;

        #[test]
        fn accepts_valid_alphanumeric_token() {
            assert!(validate_session_token("abc123XYZ").is_ok());
        }

        #[test]
        fn accepts_valid_base64_token() {
            assert!(validate_session_token("abc+def/ghi=").is_ok());
        }

        #[test]
        fn accepts_token_with_allowed_special_chars() {
            assert!(validate_session_token("token-with_dots.and-dashes").is_ok());
        }

        #[test]
        fn rejects_empty_token() {
            assert!(validate_session_token("").is_err());
        }

        #[test]
        fn rejects_token_exceeding_max_length() {
            let long_token = "a".repeat(4097);
            assert!(validate_session_token(&long_token).is_err());
        }

        #[test]
        fn accepts_token_at_max_length() {
            let max_token = "a".repeat(4096);
            assert!(validate_session_token(&max_token).is_ok());
        }

        #[test]
        fn rejects_token_with_newline() {
            assert!(validate_session_token("token\ninjection").is_err());
        }

        #[test]
        fn rejects_token_with_carriage_return() {
            assert!(validate_session_token("token\rinjection").is_err());
        }

        #[test]
        fn rejects_token_with_space() {
            assert!(validate_session_token("token with space").is_err());
        }

        #[test]
        fn rejects_token_with_colon() {
            assert!(validate_session_token("token:injection").is_err());
        }

        #[test]
        fn rejects_token_with_semicolon() {
            assert!(validate_session_token("token;injection").is_err());
        }
    }

    mod validate_org_id {
        use super::*;

        #[test]
        fn accepts_valid_uuid_format() {
            assert!(validate_org_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
        }

        #[test]
        fn accepts_alphanumeric_with_dashes() {
            assert!(validate_org_id("my-org-123").is_ok());
        }

        #[test]
        fn accepts_alphanumeric_with_underscores() {
            assert!(validate_org_id("my_org_123").is_ok());
        }

        #[test]
        fn rejects_empty_org_id() {
            assert!(validate_org_id("").is_err());
        }

        #[test]
        fn rejects_org_id_exceeding_max_length() {
            let long_id = "a".repeat(129);
            assert!(validate_org_id(&long_id).is_err());
        }

        #[test]
        fn accepts_org_id_at_max_length() {
            let max_id = "a".repeat(128);
            assert!(validate_org_id(&max_id).is_ok());
        }

        #[test]
        fn rejects_org_id_with_dots() {
            assert!(validate_org_id("org.with.dots").is_err());
        }

        #[test]
        fn rejects_org_id_with_slashes() {
            assert!(validate_org_id("org/with/slashes").is_err());
        }

        #[test]
        fn rejects_org_id_with_spaces() {
            assert!(validate_org_id("org with spaces").is_err());
        }
    }
}
