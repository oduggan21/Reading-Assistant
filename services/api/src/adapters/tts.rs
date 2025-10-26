//! services/api/src/adapters/openai_tts.rs
//!
//! This module contains the adapter for OpenAI's Text-to-Speech (TTS) service.
//! It implements the `TextToSpeechService` port from the `core` crate.

use async_openai::{
    config::OpenAIConfig,
    types::{CreateSpeechRequest, SpeechModel, Voice},
    Client, error::OpenAIError,
};
use async_trait::async_trait;
use reading_assistant_core::ports::{PortError, PortResult, TextToSpeechService};

//=========================================================================================
// The Main Adapter Struct
//=========================================================================================

/// An adapter that implements the `TextToSpeechService` port using the OpenAI TTS API.
#[derive(Clone)]
pub struct OpenAiTtsAdapter {
    client: Client<OpenAIConfig>,
    model: SpeechModel,
    voice: Voice,
}

impl OpenAiTtsAdapter {
    /// Creates a new `OpenAiTtsAdapter`.
    pub fn new(client: Client<OpenAIConfig>, model: SpeechModel, voice: Voice) -> Self {
        Self {
            client,
            model,
            voice,
        }
    }
}

//=========================================================================================
// `TextToSpeechService` Trait Implementation
//=========================================================================================

#[async_trait]
impl TextToSpeechService for OpenAiTtsAdapter {
    /// Generates a vector of audio data (`Vec<u8>`) from the given text.
    async fn generate_audio(&self, text: &str) -> PortResult<Vec<u8>> {
        let request = CreateSpeechRequest {
            model: self.model.clone(),
            input: text.to_string(),
            voice: self.voice.clone(),
            ..Default::default()
        };

        // Call the API and manually map the error, which respects the orphan rule.
        let response = self
            .client
            .audio()
            .speech(request)
            .await
            .map_err(|e: OpenAIError| PortError::Unexpected(e.to_string()))?;

        // The response contains a `bytes` field. We call `.to_vec()` on that field.
        Ok(response.bytes.to_vec())
    }
}
