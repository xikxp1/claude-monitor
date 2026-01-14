use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Network error. Check your internet connection.")]
    Http(#[from] reqwest::Error),
    #[error("Session expired. Please update your session token in Settings.")]
    InvalidToken,
    #[error("Rate limited by Claude. Please wait a moment and try again.")]
    RateLimited,
    #[error("{0}")]
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
