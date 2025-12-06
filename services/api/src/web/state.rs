//! services/api/src/web/state.rs
//!
//! Defines the application's shared and session-specific states.

use crate::config::Config;
use reading_assistant_core::ports::{
    DatabaseService, NoteGenerationService, PortResult, QuestionAnsweringService,
    SpeechToTextService, TextToSpeechService,TitleGenerationService
};
use std::sync::Arc;
use tokio_util::sync::CancellationToken; // Import the CancellationToken
use uuid::Uuid;

//=========================================================================================
// AppState (Shared Across All Connections)
//=========================================================================================

/// The shared application state, created once at startup and passed to all handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<dyn DatabaseService>,
    pub config: Arc<Config>,
    pub sst_adapter: Arc<dyn SpeechToTextService>,
    pub tts_adapter: Arc<dyn TextToSpeechService>,
    pub qa_adapter: Arc<dyn QuestionAnsweringService>,
    pub notes_adapter: Arc<dyn NoteGenerationService>,
    pub title_adapter: Arc<dyn TitleGenerationService>,
}

//=========================================================================================
// SessionState (Specific to One WebSocket Connection)
//=========================================================================================

/// An enum representing the current mode of the user's session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMode {
    Reading,
    InterruptedListening,
    ProcessingQuestion,
    Answering,
    Paused,
}

/// The state for a single, active WebSocket connection.
pub struct SessionState {
    pub user_id: Uuid,
    pub document_id: Uuid,
    pub session_id: Uuid,
    pub chunked_document: Vec<String>,
    pub reading_progress_index: usize,
    pub current_mode: SessionMode,
    pub audio_buffer: Vec<u8>,
    pub last_question: Option<String>,
    pub last_answer: Option<String>,
    /// A token to gracefully cancel the current reading task.
    pub cancellation_token: CancellationToken,
}

//=========================================================================================
// SessionState Implementation (Constructor)
//=========================================================================================

impl SessionState {
    /// Creates a new `SessionState` by fetching the required data from the database.
    pub async fn new(app_state: Arc<AppState>, session_id: Uuid) -> PortResult<Self> {
        let session_domain = app_state.db.get_session_by_id(session_id).await?;
        let document_domain = app_state
            .db
            .get_document_by_id(session_domain.document_id)
            .await?;

        let sentences = chunk_into_sentences(&document_domain.original_text);

        Ok(Self {
            user_id: session_domain.user_id,
            document_id: session_domain.document_id,
            session_id,
            chunked_document: sentences,
            reading_progress_index: session_domain.reading_progress_index,
            current_mode: SessionMode::Reading,
            audio_buffer: Vec::new(),
            last_question: None,
            last_answer: None,
            // The token is initialized here for the first reading task.
            cancellation_token: CancellationToken::new(),
        })
    }
}

/// A helper function to split a block of text into sentences.
fn chunk_into_sentences(text: &str) -> Vec<String> {
    text.split(|c: char| c == '.' || c == '?' || c == '!')
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!("{}.", s.trim()))
        .collect()
}
