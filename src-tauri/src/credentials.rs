use crate::error::AppError;
use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE_NAME: &str = "dev.xikxp1.claude-monitor";
const CREDENTIALS_KEY: &str = "credentials";
const OLLAMA_CREDENTIALS_KEY: &str = "ollama_credentials";

#[derive(Serialize, Deserialize)]
struct StoredCredentials {
    organization_id: String,
    session_token: String,
}

/// Load credentials from OS keychain.
/// Returns None if credentials don't exist or on any error.
pub fn load_credentials() -> Option<(String, String)> {
    let entry = Entry::new(SERVICE_NAME, CREDENTIALS_KEY).ok()?;
    let json = entry.get_password().ok()?;
    let creds: StoredCredentials = serde_json::from_str(&json).ok()?;
    Some((creds.organization_id, creds.session_token))
}

/// Save credentials to OS keychain.
pub fn save_credentials(org_id: &str, session_token: &str) -> Result<(), AppError> {
    let entry = Entry::new(SERVICE_NAME, CREDENTIALS_KEY)
        .map_err(|e| AppError::Storage(format!("Failed to create keyring entry: {:?}", e)))?;

    let creds = StoredCredentials {
        organization_id: org_id.to_string(),
        session_token: session_token.to_string(),
    };

    let json = serde_json::to_string(&creds)
        .map_err(|e| AppError::Storage(format!("Failed to serialize credentials: {:?}", e)))?;

    entry
        .set_password(&json)
        .map_err(|e| AppError::Storage(format!("Failed to store credentials: {:?}", e)))?;

    Ok(())
}

/// Delete credentials from OS keychain.
pub fn delete_credentials() -> Result<(), AppError> {
    let entry = Entry::new(SERVICE_NAME, CREDENTIALS_KEY)
        .map_err(|e| AppError::Storage(format!("Failed to create keyring entry: {:?}", e)))?;

    // Ignore NoEntry errors - credential might not exist
    let _ = entry.delete_credential();

    Ok(())
}

// ============================================================================
// Ollama Credentials
// ============================================================================

/// Load Ollama session token from OS keychain.
/// Returns None if credentials don't exist or on any error.
pub fn load_ollama_credentials() -> Option<String> {
    let entry = Entry::new(SERVICE_NAME, OLLAMA_CREDENTIALS_KEY).ok()?;
    entry.get_password().ok()
}

/// Save Ollama session token to OS keychain.
pub fn save_ollama_credentials(session_token: &str) -> Result<(), AppError> {
    let entry = Entry::new(SERVICE_NAME, OLLAMA_CREDENTIALS_KEY)
        .map_err(|e| AppError::Storage(format!("Failed to create keyring entry: {:?}", e)))?;

    entry
        .set_password(session_token)
        .map_err(|e| AppError::Storage(format!("Failed to store Ollama credentials: {:?}", e)))?;

    Ok(())
}

/// Delete Ollama session token from OS keychain.
pub fn delete_ollama_credentials() -> Result<(), AppError> {
    let entry = Entry::new(SERVICE_NAME, OLLAMA_CREDENTIALS_KEY)
        .map_err(|e| AppError::Storage(format!("Failed to create keyring entry: {:?}", e)))?;

    // Ignore NoEntry errors - credential might not exist
    let _ = entry.delete_credential();

    Ok(())
}
