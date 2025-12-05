//! crates/reading_assistant_core/src/domain.rs
//!
//! Defines the pure, core data structures for the application.
//! These structs are independent of any database or serialization format.

use uuid::Uuid;
use chrono::{DateTime, Utc};


#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub document_id: Uuid,
    pub reading_progress_index: usize,
    pub created_at: DateTime<Utc>,  // ✅ Add this
    pub last_accessed_at: DateTime<Utc>,  // ✅ Add this
}

/// Represents a text document uploaded by a user.
#[derive(Debug, Clone)]
pub struct Document {
    pub id: Uuid,
    pub user_id: Uuid,
    pub original_text: String,
}

// Represents a user - used throughout app
#[derive(Debug, Clone)]
pub struct User {
    pub user_id: Uuid,
    pub email: Option<String>,  // Optional because old users won't have it
}

// Only used internally for login/signup - contains sensitive data
#[derive(Debug, Clone)]
pub struct UserCredentials {
    pub user_id: Uuid,
    pub email: String,
    pub hashed_password: String,
}

// Represents a browser login session (auth cookie)
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub id: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
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
    pub created_at: DateTime<Utc>,
}