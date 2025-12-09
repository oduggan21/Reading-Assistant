//! services/api/src/adapters/openai_sst.rs
//!
//! This module contains the adapter for OpenAI's Speech-to-Text (Whisper) service.
//! It implements the `SpeechToTextService` port from the `core` crate.

use async_openai::{
    config::OpenAIConfig,
    types::{audio::{AudioInput, CreateTranscriptionRequest}},
    Client, error::OpenAIError,
};
use async_trait::async_trait;
use reading_assistant_core::ports::{PortError, PortResult, SpeechToTextService};
use hound::{WavSpec, WavWriter};

//=========================================================================================
// The Main Adapter Struct
//=========================================================================================

/// An adapter that implements the `SpeechToTextService` port using the OpenAI Whisper API.
#[derive(Clone)]
pub struct OpenAiSstAdapter {
    client: Client<OpenAIConfig>,
    model: String,
}

impl OpenAiSstAdapter {
    /// Creates a new `OpenAiSstAdapter`.
    pub fn new(client: Client<OpenAIConfig>, model: String) -> Self {
        Self { client, model }
    }
    fn pcm16_to_wav(pcm_data: &[u8], sample_rate: u32) -> Result<Vec<u8>, hound::Error> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        
        let spec = WavSpec {
            channels: 1,           // Mono
            sample_rate,           // 48000 or whatever your frontend uses
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        
        let mut writer = WavWriter::new(&mut cursor, spec)?;
        
        // Convert byte array to i16 samples
        for chunk in pcm_data.chunks_exact(2) {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
            writer.write_sample(sample)?;
        }
        
        writer.finalize()?;
        Ok(cursor.into_inner())
    }
}

//=========================================================================================
// `SpeechToTextService` Trait Implementation
//=========================================================================================

#[async_trait]
impl SpeechToTextService for OpenAiSstAdapter {
    /// Transcribes a slice of audio data into text using the configured Whisper model.
    async fn transcribe_audio(&self, audio_data: &[u8]) -> PortResult<String> {
        let wav_data = Self::pcm16_to_wav(audio_data, 48000)
            .map_err(|e| PortError::Unexpected(format!("Failed to encode WAV: {}", e)))?;
        

        let input = AudioInput::from_vec_u8("user_audio.wav".into(), wav_data);

        let request = CreateTranscriptionRequest {
            file: input,
            model: self.model.clone(),
            ..Default::default()
        };

        // Call the API and manually map the error, which respects the orphan rule.
        let response = self
            .client
            .audio()
            .transcription()
            .create(request)
            .await
            .map_err(|e: OpenAIError| PortError::Unexpected(e.to_string()))?;

        Ok(response.text)
    }
}
