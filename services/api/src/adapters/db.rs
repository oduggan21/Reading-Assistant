//! services/api/src/adapters/db.rs
//!
//! This module contains the database adapter, which is the concrete implementation
//! of the `DatabaseService` port from the `core` crate. It handles all interactions
//! with the PostgreSQL database using `sqlx`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reading_assistant_core::domain::{Document, Note, QAPair, Session, User};
use reading_assistant_core::ports::{DatabaseService, PortError, PortResult};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

//=========================================================================================
// The Main Adapter Struct
//=========================================================================================

/// A database adapter that implements the `DatabaseService` port.
#[derive(Clone)]
pub struct DbAdapter {
    pool: PgPool,
}

impl DbAdapter {
    /// Creates a new `DbAdapter`.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// A helper function to run database migrations at startup.
    pub async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }
}

//=========================================================================================
// "Impure" Database Record Structs
//=========================================================================================

#[derive(FromRow)]
struct UserRecord {
    user_id: Uuid,
    created_at: DateTime<Utc>,
}
impl UserRecord {
    fn to_domain(self) -> User {
        User {
            user_id: self.user_id,
        }
    }
}

#[derive(FromRow)]
struct DocumentRecord {
    id: Uuid,
    user_id: Uuid,
    original_text: String,
}
impl DocumentRecord {
    fn to_domain(self) -> Document {
        Document {
            id: self.id,
            user_id: self.user_id,
            original_text: self.original_text,
        }
    }
}

#[derive(FromRow)]
struct SessionRecord {
    id: Uuid,
    user_id: Uuid,
    document_id: Uuid,
    reading_progress_index: i32,
}
impl SessionRecord {
    fn to_domain(self) -> Session {
        Session {
            id: self.id,
            user_id: self.user_id,
            document_id: self.document_id,
            reading_progress_index: self.reading_progress_index as usize,
        }
    }
}

#[derive(FromRow)]
struct QAPairRecord {
    id: Uuid,
    session_id: Uuid,
    question_text: String,
    answer_text: String,
    created_at: DateTime<Utc>,
}
impl QAPairRecord {
    fn to_domain(self) -> QAPair {
        QAPair {
            id: self.id,
            session_id: self.session_id,
            question_text: self.question_text,
            answer_text: self.answer_text,
        }
    }
}

#[derive(FromRow)]
struct NoteRecord {
    id: Uuid,
    session_id: Uuid,
    generated_note_text: String,
    created_at: DateTime<Utc>,
}
impl NoteRecord {
    fn to_domain(self) -> Note {
        Note {
            id: self.id,
            session_id: self.session_id,
            generated_note_text: self.generated_note_text,
        }
    }
}

//=========================================================================================
// `DatabaseService` Trait Implementation
//=========================================================================================

#[async_trait]
impl DatabaseService for DbAdapter {
    async fn get_or_create_user(&self, user_id: Uuid) -> PortResult<User> {
        sqlx::query!("INSERT INTO users (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING", user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| PortError::Unexpected(e.to_string()))?;

        let record = sqlx::query_as!(
            UserRecord,
            "SELECT user_id, created_at FROM users WHERE user_id = $1",
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PortError::NotFound(format!("User {} not found", user_id)),
            _ => PortError::Unexpected(e.to_string()),
        })?;

        Ok(record.to_domain())
    }

    async fn get_document_by_id(&self, document_id: Uuid) -> PortResult<Document> {
        let record = sqlx::query_as!(
            DocumentRecord,
            "SELECT id, user_id, original_text FROM documents WHERE id = $1",
            document_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PortError::NotFound(format!("Document {} not found", document_id)),
            _ => PortError::Unexpected(e.to_string()),
        })?;
        Ok(record.to_domain())
    }

    async fn create_document(&self, user_id: Uuid, _title: &str, original_text: &str) -> PortResult<Document> {
        let record = sqlx::query_as!(
            DocumentRecord,
            "INSERT INTO documents (id, user_id, original_text) VALUES ($1, $2, $3) RETURNING id, user_id, original_text",
            Uuid::new_v4(),
            user_id,
            original_text
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected(e.to_string()))?;
        Ok(record.to_domain())
    }

    async fn get_session_by_id(&self, session_id: Uuid) -> PortResult<Session> {
        let record = sqlx::query_as!(
            SessionRecord,
            "SELECT id, user_id, document_id, reading_progress_index FROM sessions WHERE id = $1",
            session_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PortError::NotFound(format!("Session {} not found", session_id)),
            _ => PortError::Unexpected(e.to_string()),
        })?;
        Ok(record.to_domain())
    }

    async fn create_session(&self, user_id: Uuid, document_id: Uuid) -> PortResult<Session> {
        let record = sqlx::query_as!(
            SessionRecord,
            "INSERT INTO sessions (user_id, document_id) VALUES ($1, $2) RETURNING id, user_id, document_id, reading_progress_index",
            user_id,
            document_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected(e.to_string()))?;
        Ok(record.to_domain())
    }

    async fn update_session_progress(
        &self,
        session_id: Uuid,
        new_progress_index: usize,
    ) -> PortResult<()> {
        sqlx::query!(
            "UPDATE sessions SET reading_progress_index = $1 WHERE id = $2",
            new_progress_index as i32,
            session_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected(e.to_string()))?;
        Ok(())
    }

    async fn save_qa_pair(&self, qa_pair: QAPair) -> PortResult<()> {
        sqlx::query!(
            "INSERT INTO qa_pairs (id, session_id, question_text, answer_text) VALUES ($1, $2, $3, $4)",
            qa_pair.id,
            qa_pair.session_id,
            qa_pair.question_text,
            qa_pair.answer_text
        )
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected(e.to_string()))?;
        Ok(())
    }

    async fn get_qa_pairs_for_session(&self, session_id: Uuid) -> PortResult<Vec<QAPair>> {
        let records = sqlx::query_as!(
            QAPairRecord,
            "SELECT id, session_id, question_text, answer_text, created_at FROM qa_pairs WHERE session_id = $1 ORDER BY created_at ASC",
            session_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected(e.to_string()))?;

        let qa_pairs = records.into_iter().map(|r| r.to_domain()).collect();
        Ok(qa_pairs)
    }

    async fn save_note(&self, note: Note) -> PortResult<()> {
        sqlx::query!(
            "INSERT INTO notes (id, session_id, generated_note_text) VALUES ($1, $2, $3)",
            note.id,
            note.session_id,
            note.generated_note_text
        )
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected(e.to_string()))?;
        Ok(())
    }

    async fn get_notes_for_session(&self, session_id: Uuid) -> PortResult<Vec<Note>> {
        let records = sqlx::query_as!(
            NoteRecord,
            "SELECT id, session_id, generated_note_text, created_at FROM notes WHERE session_id = $1 ORDER BY created_at ASC",
            session_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected(e.to_string()))?;

        let notes = records.into_iter().map(|r| r.to_domain()).collect();
        Ok(notes)
    }
}