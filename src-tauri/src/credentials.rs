use crate::error::AppError;
use keyring::Entry;

const SERVICE_NAME: &str = "dev.xikxp1.claude-monitor";

/// Load credentials from OS keychain.
/// Returns None if credentials don't exist or on any error.
pub fn load_credentials() -> Option<(String, String)> {
    let org_entry = Entry::new(SERVICE_NAME, "organization_id").ok()?;
    let token_entry = Entry::new(SERVICE_NAME, "session_token").ok()?;

    let org_id = org_entry.get_password().ok()?;
    let token = token_entry.get_password().ok()?;

    Some((org_id, token))
}

/// Save credentials to OS keychain.
pub fn save_credentials(org_id: &str, session_token: &str) -> Result<(), AppError> {
    let org_entry = Entry::new(SERVICE_NAME, "organization_id")
        .map_err(|e| AppError::Storage(format!("Failed to create keyring entry: {:?}", e)))?;

    let token_entry = Entry::new(SERVICE_NAME, "session_token")
        .map_err(|e| AppError::Storage(format!("Failed to create keyring entry: {:?}", e)))?;

    org_entry
        .set_password(org_id)
        .map_err(|e| AppError::Storage(format!("Failed to store organization_id: {:?}", e)))?;

    token_entry
        .set_password(session_token)
        .map_err(|e| AppError::Storage(format!("Failed to store session_token: {:?}", e)))?;

    Ok(())
}

/// Delete credentials from OS keychain.
pub fn delete_credentials() -> Result<(), AppError> {
    let org_entry = Entry::new(SERVICE_NAME, "organization_id")
        .map_err(|e| AppError::Storage(format!("Failed to create keyring entry: {:?}", e)))?;

    let token_entry = Entry::new(SERVICE_NAME, "session_token")
        .map_err(|e| AppError::Storage(format!("Failed to create keyring entry: {:?}", e)))?;

    // Ignore NoEntry errors - credential might not exist
    let _ = org_entry.delete_credential();
    let _ = token_entry.delete_credential();

    Ok(())
}
