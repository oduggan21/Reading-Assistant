//! services/api/src/config.rs
//!
//! Defines the application's configuration structure and loading logic.
//!
//! All configuration is loaded from environment variables at startup. The `.env`
//! file is used for local development.

use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::Level;

/// A custom error type for configuration loading failures.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing the environment variable {0}")]
    MissingVar(String),
    #[error("Invalid value for the environment variable {0}: {1}")]
    InvalidValue(String, String),
}

/// Holds all configuration loaded from the environment at startup.
#[derive(Clone, Debug)]
pub struct Config {
    pub bind_address: SocketAddr,
    pub database_url: String,
    pub log_level: Level,
    pub prompts_path: PathBuf,
    pub openai_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
    pub sst_model: String,
    pub tts_voice: String,
    pub qa_model: String,
    pub note_model: String,
}

impl Config {
    /// Loads configuration from environment variables.
    ///
    /// It will look for a `.env` file in the current directory for development,
    /// but this is skipped in test environments to ensure tests are hermetic.
    pub fn from_env() -> Result<Self, ConfigError> {
        // Only load from .env in non-test mode to avoid contamination.
        if !cfg!(test) {
            dotenvy::dotenv().ok();
        }

        // --- Load Server and Database Settings ---
        let bind_address_str =
            std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
        let bind_address = bind_address_str.parse::<SocketAddr>().map_err(|e| {
            ConfigError::InvalidValue("BIND_ADDRESS".to_string(), e.to_string())
        })?;

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingVar("DATABASE_URL".to_string()))?;

        let log_level_str = std::env::var("RUST_LOG").unwrap_or_else(|_| "INFO".to_string());
        let log_level = log_level_str.parse::<Level>().map_err(|_| {
            ConfigError::InvalidValue(
                "RUST_LOG".to_string(),
                format!("'{}' is not a valid log level", log_level_str),
            )
        })?;

        let prompts_path = std::env::var("PROMPTS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./prompts"));

        // --- Load API Keys (as optional) ---
        let openai_api_key = std::env::var("OPENAI_API_KEY").ok();
        let gemini_api_key = std::env::var("GEMINI_API_KEY").ok();

        // --- Load Adapter-specific Settings ---
        let sst_model =
            std::env::var("SST_MODEL").unwrap_or_else(|_| "whisper-1".to_string());
        let tts_voice = std::env::var("TTS_VOICE").unwrap_or_else(|_| "alloy".to_string());
        let qa_model = std::env::var("QA_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());
        let note_model =
            std::env::var("NOTE_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

        Ok(Self {
            bind_address,
            database_url,
            log_level,
            prompts_path,
            openai_api_key,
            gemini_api_key,
            sst_model,
            tts_voice,
            qa_model,
            note_model,
        })
    }
}