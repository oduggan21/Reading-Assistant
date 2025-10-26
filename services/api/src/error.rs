//! services/api/src/error.rs
//!
//! Defines the primary error type for the entire API service.

use crate::config::ConfigError;
use reading_assistant_core::ports::PortError;


/// The primary error type for the `api` service.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Represents an error that occurred during configuration loading.
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Represents an error that propagated up from one of the core service ports.
    #[error("Service Port Error: {0}")]
    Port(#[from] PortError),

    /// Represents an error from the underlying database library.
    #[error("Database Error: {0}")]
    Database(#[from] sqlx::Error),

    /// Represents an error related to the WebSocket connection.
    #[error("WebSocket Error: {0}")]
    Websocket(#[from] axum::Error),

    /// Represents a standard Input/Output error (e.g., binding to a network socket).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A catch-all for any other unexpected errors.
    #[error("An unexpected internal error occurred: {0}")]
    Internal(String),
}