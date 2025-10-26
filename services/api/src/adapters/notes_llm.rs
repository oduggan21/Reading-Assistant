//! services/api/src/adapters/notes_llm.rs
//!
//! This module contains the adapter for the Note-Generating LLM.
//! It implements the `NoteGenerationService` port from the `core` crate.

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client, error::OpenAIError,
};
use async_trait::async_trait;
use reading_assistant_core::{
    domain::QAPair,
    ports::{NoteGenerationService, PortError, PortResult},
};

//=========================================================================================
// The Main Adapter Struct
//=========================================================================================

/// An adapter that implements `NoteGenerationService` using an OpenAI-compatible LLM.
#[derive(Clone)]
pub struct OpenAiNotesAdapter {
    client: Client<OpenAIConfig>,
    model: String,
}

impl OpenAiNotesAdapter {
    /// Creates a new `OpenAiNotesAdapter`.
    pub fn new(client: Client<OpenAIConfig>, model: String) -> Self {
        Self { client, model }
    }
}

//=========================================================================================
// `NoteGenerationService` Trait Implementation
//=========================================================================================

#[async_trait]
impl NoteGenerationService for OpenAiNotesAdapter {
    /// Generates a concise note by summarizing a question and its corresponding answer.
    async fn generate_note_from_qapair(&self, qapair: &QAPair) -> PortResult<String> {
        let messages = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You are a note-taking assistant. Your task is to summarize the following question and answer into a single, concise note. The note should capture the key insight or piece of information from the exchange. Present it as a single bullet point or a short sentence.")
                .build()
                .map_err(|e| PortError::Unexpected(e.to_string()))?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(format!(
                    "QUESTION: {}\n\nANSWER: {}",
                    qapair.question_text, qapair.answer_text
                ))
                .build()
                .map_err(|e| PortError::Unexpected(e.to_string()))?
                .into(),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(messages)
            .n(1)
            .build()
            .map_err(|e| PortError::Unexpected(e.to_string()))?;

        // Call the API and manually map the error if it occurs, which respects the orphan rule.
        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e: OpenAIError| PortError::Unexpected(e.to_string()))?;

        // Extract the text content from the first choice in the response.
        if let Some(choice) = response.choices.into_iter().next() {
            if let Some(content) = choice.message.content {
                Ok(content)
            } else {
                Err(PortError::Unexpected(
                    "Note generation LLM response contained no text content.".to_string(),
                ))
            }
        } else {
            Err(PortError::Unexpected(
                "Note generation LLM returned no choices in its response.".to_string(),
            ))
        }
    }
}