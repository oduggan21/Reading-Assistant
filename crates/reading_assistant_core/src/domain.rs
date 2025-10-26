//! crates/reading_assistant_core/src/domain.rs
//!
//! Defines the pure, core data structures for the application.
//! These structs are independent of any database or serialization format.

use uuid::Uuid;

/// Represents a single user session with a document.
#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub document_id: Uuid,
    pub reading_progress_index: usize,
}

/// Represents a text document uploaded by a user.
#[derive(Debug, Clone)]
pub struct Document {
    pub id: Uuid,
    pub user_id: Uuid,
    pub original_text: String,
}

/// Represents a user of the application.
#[derive(Debug, Clone, Copy)]
pub struct User {
    pub user_id: Uuid,
}

/// Represents a single question-and-answer exchange within a session.
#[derive(Debug, Clone)]
pub struct QAPair {
    pub id: Uuid,
    pub session_id: Uuid,
    pub question_text: String,
    pub answer_text: String,
}

/// Represents a single, summarized note generated from a QAPair.
#[derive(Debug, Clone)]
pub struct Note {
    pub id: Uuid,
    pub session_id: Uuid,
    pub generated_note_text: String,
}