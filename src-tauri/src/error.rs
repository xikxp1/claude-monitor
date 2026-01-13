use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Invalid session token")]
    InvalidToken,
    #[error("Rate limited")]
    RateLimited,
    #[error("Server error: {0}")]
    Server(String),
    #[error("Missing configuration: {0}")]
    MissingConfig(String),
    #[error("Storage error: {0}")]
    Storage(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
