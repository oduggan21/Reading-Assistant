//! services/api/src/adapters/db.rs
//!
//! This module contains the database adapter, which is the concrete implementation
//! of the `DatabaseService` port from the `core` crate. It handles all interactions
//! with the PostgreSQL database using `sqlx`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reading_assistant_core::domain::{Document, Note, QAPair, Session, User, UserCredentials, AuthSession};
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

// Update existing UserRecord to include email
#[derive(FromRow)]
struct UserRecord {
    user_id: Uuid,
    email: Option<String>,      // Add this
    created_at: DateTime<Utc>,
}

impl UserRecord {
    fn to_domain(self) -> User {
        User {
            user_id: self.user_id,
            email: self.email,      // Add this
        }
    }
}

// For login - fetches password too
#[derive(FromRow)]
struct UserWithPasswordRecord {
    user_id: Uuid,
    email: String,
    hashed_password: String,
}

impl UserWithPasswordRecord {
    fn to_domain(self) -> UserCredentials {
        UserCredentials {
            user_id: self.user_id,
            email: self.email,
            hashed_password: self.hashed_password,
        }
    }
}

// Auth session record - maps to auth_sessions table
#[derive(FromRow)]
struct AuthSessionRecord {
    id: String,
    user_id: Uuid,
    expires_at: DateTime<Utc>,
}

impl AuthSessionRecord {
    fn to_domain(self) -> AuthSession {
        AuthSession {
            id: self.id,
            user_id: self.user_id,
            expires_at: self.expires_at,
        }
    }
}

#[derive(FromRow)]
struct DocumentRecord {
    id: Uuid,
    user_id: Uuid,
    original_text: String,
    title: Option<String>,  // ✅ Add this
}

impl DocumentRecord {
    fn to_domain(self) -> Document {
        Document {
            id: self.id,
            user_id: self.user_id,
            original_text: self.original_text,
            title: self.title,  // ✅ Add this
        }
    }
}

#[derive(FromRow)]
struct SessionRecord {
    id: Uuid,
    user_id: Uuid,
    document_id: Uuid,
    reading_progress_index: i32,
    created_at: chrono::DateTime<chrono::Utc>,  // ✅ Add this
    last_accessed_at: chrono::DateTime<chrono::Utc>,  // ✅ Add this
}

impl SessionRecord {
    fn to_domain(self) -> Session {
        Session {
            id: self.id,
            user_id: self.user_id,
            document_id: self.document_id,
            reading_progress_index: self.reading_progress_index as usize,
            created_at: self.created_at,  // ✅ Add this
            last_accessed_at: self.last_accessed_at,  // ✅ Add this
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
    created_at: chrono::DateTime<chrono::Utc>, 
}
impl NoteRecord {
    fn to_domain(self) -> Note {
        Note {
            id: self.id,
            session_id: self.session_id,
            generated_note_text: self.generated_note_text,
            created_at: self.created_at,
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
            "SELECT user_id, email, created_at FROM users WHERE user_id = $1",  // Add email here
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
        "SELECT id, user_id, original_text, title FROM documents WHERE id = $1",  // ✅ Add title
        document_id
    )
    .fetch_one(&self.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => PortError::NotFound("Document not found".to_string()),
        _ => PortError::Unexpected(e.to_string()),
    })?;
    
    Ok(record.to_domain())
}

 async fn create_document(
    &self,
    user_id: Uuid,
    title: &str,
    original_text: &str,
) -> PortResult<Document> {
    let record = sqlx::query_as!(
        DocumentRecord,
        "INSERT INTO documents (id, user_id, original_text, title) 
         VALUES ($1, $2, $3, $4) 
         RETURNING id, user_id, original_text, title",  // ✅ Add title to both INSERT and RETURNING
        Uuid::new_v4(),
        user_id,
        original_text,
        Some(title)  // ✅ Initially set to Some(title), will be updated after generation
    )
    .fetch_one(&self.pool)
    .await
    .map_err(|e| PortError::Unexpected(e.to_string()))?;
    
    Ok(record.to_domain())
}
    async fn get_session_by_id(&self, session_id: Uuid) -> PortResult<Session> {
        let record = sqlx::query_as!(
            SessionRecord,
            "SELECT id, user_id, document_id, reading_progress_index, created_at, last_accessed_at 
            FROM sessions 
            WHERE id = $1",
            session_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PortError::NotFound("Session not found".to_string()),
            _ => PortError::Unexpected(e.to_string()),
        })?;
        
        Ok(record.to_domain())
    }

    async fn create_session(&self, user_id: Uuid, document_id: Uuid) -> PortResult<Session> {
    let record = sqlx::query_as!(
        SessionRecord,
        "INSERT INTO sessions (id, user_id, document_id) 
         VALUES ($1, $2, $3) 
         RETURNING id, user_id, document_id, reading_progress_index, created_at, last_accessed_at",
        Uuid::new_v4(),  // ✅ Generate ID here
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
        "SELECT id, session_id, generated_note_text, created_at 
         FROM notes 
         WHERE session_id = $1 
         ORDER BY created_at ASC",
        session_id
    )
    .fetch_all(&self.pool)
    .await
    .map_err(|e| PortError::Unexpected(e.to_string()))?;

    Ok(records.into_iter().map(|r| r.to_domain()).collect())
    }

    async fn create_user_with_email(
        &self,
        email: &str,
        hashed_password: &str,
    ) -> PortResult<User> {
        let user_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (user_id, email, hashed_password) VALUES ($1, $2, $3)",
            user_id,
            email,
            hashed_password
        )
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected(e.to_string()))?;
        
        Ok(User { 
            user_id,
            email: Some(email.to_string()),
        })
    }
    
    async fn get_user_by_email(&self, email: &str) -> PortResult<UserCredentials> {
    let record = sqlx::query!(
        "SELECT user_id, email, hashed_password FROM users WHERE email = $1",
        email
    )
    .fetch_one(&self.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => PortError::NotFound("User not found".to_string()),
        _ => PortError::Unexpected(e.to_string()),
    })?;
    
    // Handle optional email and password
    let email = record.email.ok_or_else(|| {
        PortError::Unexpected("User has no email".to_string())
    })?;
    
    let hashed_password = record.hashed_password.ok_or_else(|| {
        PortError::Unexpected("User has no password".to_string())
    })?;
    
    Ok(UserCredentials {
        user_id: record.user_id,
        email,
        hashed_password,
    })
  }
    
    async fn create_auth_session(
        &self,
        session_id: &str,
        user_id: Uuid,
        expires_at: DateTime<Utc>,
    ) -> PortResult<()> {
        sqlx::query!(
            "INSERT INTO auth_sessions (id, user_id, expires_at) VALUES ($1, $2, $3)",
            session_id,
            user_id,
            expires_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected(e.to_string()))?;
        Ok(())
    }
    
    async fn validate_auth_session(&self, session_id: &str) -> PortResult<Uuid> {
        let record = sqlx::query!(
            "SELECT user_id FROM auth_sessions 
             WHERE id = $1 AND expires_at > NOW()",
            session_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PortError::Unauthorized,
            _ => PortError::Unexpected(e.to_string()),
        })?;
        Ok(record.user_id)
    }
    
    async fn delete_auth_session(&self, session_id: &str) -> PortResult<()> {
        sqlx::query!("DELETE FROM auth_sessions WHERE id = $1", session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| PortError::Unexpected(e.to_string()))?;
        Ok(())
    }

    async fn get_sessions_by_user(&self, user_id: Uuid) -> PortResult<Vec<Session>> {
    let records = sqlx::query_as!(
        SessionRecord,
        "SELECT id, user_id, document_id, reading_progress_index, created_at, last_accessed_at
         FROM sessions 
         WHERE user_id = $1 
         ORDER BY last_accessed_at DESC",
        user_id
    )
    .fetch_all(&self.pool)
    .await
    .map_err(|e| PortError::Unexpected(e.to_string()))?;

    Ok(records.into_iter().map(|r| r.to_domain()).collect())
    }

    async fn update_document_title(&self, document_id: Uuid, title: &str) -> PortResult<()> {
    sqlx::query!(
        "UPDATE documents SET title = $1 WHERE id = $2",
        title,
        document_id
    )
    .execute(&self.pool)
    .await
    .map_err(|e| PortError::Unexpected(e.to_string()))?;
    
    Ok(())
}
}
