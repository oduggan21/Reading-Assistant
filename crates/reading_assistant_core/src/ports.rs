//! crates/reading_assistant_core/src/ports.rs
//!
//! Defines the service contracts (traits) for the application's core logic.
//! These traits form the boundary of the hexagonal architecture, allowing the core
//! to be independent of specific external implementations like databases or APIs.

use async_trait::async_trait;
use uuid::Uuid;
use futures::Stream;
use std::pin::Pin;
use chrono::{DateTime, Utc};
use crate::domain::{Document, Note, QAPair, Session, User, UserCredentials};

//=========================================================================================
// Generic Port Error and Result Types
//=========================================================================================

/// A generic error type for all port operations.
/// This abstracts away the specific errors from external services (e.g., database, network).
#[derive(Debug, thiserror::Error)]
pub enum PortError {
    #[error("Item not found: {0}")]
    NotFound(String),
    #[error("An unexpected error occurred: {0}")]
    Unexpected(String),
    #[error("Unauthorized")]
    Unauthorized, 
}

/// A convenience type alias for `Result<T, PortError>`.
pub type PortResult<T> = Result<T, PortError>;

//=========================================================================================
// Service Ports (Traits)
//=========================================================================================

#[async_trait]
pub trait DatabaseService: Send + Sync {
    // --- User Management ---
    async fn get_or_create_user(&self, user_id: Uuid) -> PortResult<User>;
    
    // --- Auth Methods ---
    async fn create_user_with_email(
        &self,
        email: &str,
        hashed_password: &str,
    ) -> PortResult<User>;
    
    async fn get_user_by_email(&self, email: &str) -> PortResult<UserCredentials>;
    
    async fn create_auth_session(
        &self,
        session_id: &str,
        user_id: Uuid,
        expires_at: DateTime<Utc>,
    ) -> PortResult<()>;
    
    async fn validate_auth_session(&self, session_id: &str) -> PortResult<Uuid>;
    
    async fn delete_auth_session(&self, session_id: &str) -> PortResult<()>;

    // --- Document Management ---
    async fn get_document_by_id(&self, document_id: Uuid) -> PortResult<Document>;
    
    async fn create_document(
        &self,
        user_id: Uuid,
        title: &str,
        original_text: &str,
    ) -> PortResult<Document>;

    // --- Session Management (Reading Sessions) ---
    async fn get_session_by_id(&self, session_id: Uuid) -> PortResult<Session>;
    
    async fn create_session(&self, user_id: Uuid, document_id: Uuid) -> PortResult<Session>;
    
    async fn update_session_progress(
        &self,
        session_id: Uuid,
        new_progress_index: usize,
    ) -> PortResult<()>;

    // --- Q&A and Note Management ---
    async fn save_qa_pair(&self, qa_pair: QAPair) -> PortResult<()>;
    
    async fn get_qa_pairs_for_session(&self, session_id: Uuid) -> PortResult<Vec<QAPair>>;
    
    async fn save_note(&self, note: Note) -> PortResult<()>;
    
    async fn get_notes_for_session(&self, session_id: Uuid) -> PortResult<Vec<Note>>;

    async fn get_sessions_by_user(&self, user_id: Uuid) -> PortResult<Vec<Session>>;
}

#[async_trait]
pub trait SpeechToTextService: Send + Sync {
    /// Transcribes a slice of audio data into text.
    async fn transcribe_audio(&self, audio_data: &[u8]) -> PortResult<String>;
}

#[async_trait]
pub trait TextToSpeechService: Send + Sync {
    /// Generates audio data from a string of text.
    async fn generate_audio(&self, text: &str) -> PortResult<Vec<u8>>;
}

#[async_trait]
pub trait QuestionAnsweringService: Send + Sync {
    /// Answers a question based on a provided context.
    async fn answer_question(&self, question: &str, context: &str) -> PortResult<String>;
    async fn answer_question_streaming(
        &self,
        question: &str,
        context: &str,
    ) -> PortResult<Pin<Box<dyn Stream<Item = Result<String, PortError>> + Send>>>;
}

#[async_trait]
pub trait NoteGenerationService: Send + Sync {
    /// Generates a concise note from a QAPair.
    async fn generate_note_from_qapair(&self, qapair: &QAPair) -> PortResult<String>;
}
