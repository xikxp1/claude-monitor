use serde::Serialize;
use specta::Type;
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

// Implement Type manually since reqwest::Error doesn't implement Type.
// The error is serialized as a string, so we export it as string type.
impl Type for AppError {
    fn inline(
        _type_map: &mut specta::TypeCollection,
        _generics: specta::Generics,
    ) -> specta::datatype::DataType {
        specta::datatype::DataType::Primitive(specta::datatype::PrimitiveType::String)
    }
}
