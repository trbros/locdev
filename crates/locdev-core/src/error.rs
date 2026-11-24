use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    IncompatibleProtocol,
    InvalidInput,
    NotFound,
    AlreadyExists,
    NotRunnable,
    /// The operation would break something in use (a running site, a
    /// pinned version, unsaved work); the message says what.
    Conflict,
    /// The operation needs a confirmation (password or typed phrase) it
    /// did not get, or got a wrong one.
    Unauthorized,
    /// A remote service (GitHub, a package registry) refused or failed.
    Remote,
    Io,
    Config,
    Internal,
}

/// Error shared by the daemon and its clients; it travels over IPC unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, message)
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorCode::Io, error.to_string())
    }
}

impl From<crate::DomainError> for AppError {
    fn from(error: crate::DomainError) -> Self {
        Self::invalid(error.to_string())
    }
}
