//! services/api/src/web/rest.rs
//!
//! Contains the Axum handlers for the REST API endpoints and the master
//! definition for the OpenAPI specification.

use crate::web::state::AppState;
use crate::web::auth::{SignupRequest, LoginRequest, AuthResponse};
use axum::{
    extract::{Multipart, State},
    http::{StatusCode},
    response::{IntoResponse, Json},
    Extension,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::error;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

//=========================================================================================
// OpenAPI Master Definition
//=========================================================================================

#[derive(OpenApi)]
#[openapi(
    paths(
        create_session_handler,
        list_notes_handler,
        list_sessions_handler, 
        crate::web::auth::signup_handler,    // Add
        crate::web::auth::login_handler,     // Add
        crate::web::auth::logout_handler,    // Add
    ),
    components(
        schemas(
            CreateSessionResponse,
            NoteItem,           // ✅ Add this
            ListNotesResponse,
            SessionListItem,        // ✅ Add this
            ListSessionsResponse,
            SignupRequest,      // Add
            LoginRequest,       // Add
            AuthResponse,       // Add
        )
    ),
    tags(
        (name = "Reading Assistant API", description = "API endpoints for the interactive audio reader."),
        (name = "Authentication", description = "User authentication endpoints"),  // Add
    )
)]
pub struct ApiDoc;
//=========================================================================================
// API Response and Payload Structs
//=========================================================================================

/// The response payload sent after successfully creating a session.
#[derive(Serialize, ToSchema)]
pub struct CreateSessionResponse {
    session_id: Uuid,
    document_id: Uuid,
    user_id: Uuid,
}

#[derive(Serialize, ToSchema)]
pub struct SessionListItem {
    session_id: Uuid,
    document_id: Uuid,
    title: Option<String>,
    created_at: String,  // ISO 8601 timestamp
    // Add more fields as needed (document name, preview, etc.)
}

#[derive(Serialize, ToSchema)]
pub struct ListSessionsResponse {
    sessions: Vec<SessionListItem>,
}

#[derive(Serialize, ToSchema)]
pub struct NoteItem {
    note_id: Uuid,
    session_id: Uuid,
    text: String,
    created_at: String,  // ISO 8601 timestamp
}

#[derive(Serialize, ToSchema)]
pub struct ListNotesResponse {
    notes: Vec<NoteItem>,
}

//=========================================================================================
// REST API Handlers
//=========================================================================================

/// Create a new session by uploading a document.
///
/// Requires authentication. The user_id is extracted from the auth session.
#[utoipa::path(
    post,
    path = "/sessions",
    request_body(content_type = "multipart/form-data", description = "The document to upload."),
    responses(
        (status = 201, description = "Session created successfully", body = CreateSessionResponse),
        (status = 400, description = "Bad request (e.g., missing file)"),
        (status = 401, description = "Unauthorized - no valid session"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("session_cookie" = [])
    )
)]
pub async fn create_session_handler(
    State(app_state): State<Arc<AppState>>,
    Extension(user_id): Extension<Uuid>,  // ✅ From auth middleware
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // No need to parse headers or validate user anymore!
    
    let (file_name, file_text) =
        if let Some(field) = multipart.next_field().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read multipart data: {}", e),
            )
        })? {
            let name = field.file_name().unwrap_or("untitled.txt").to_string();
            let data = field.bytes().await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read file bytes: {}", e),
                )
            })?;
            let text = String::from_utf8(data.to_vec()).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Uploaded file is not valid UTF-8 text: {}", e),
                )
            })?;
            (name, text)
        } else {
            return Err((
                StatusCode::BAD_REQUEST,
                "Multipart form must include a file".to_string(),
            ));
        };

    let db = &app_state.db;
    let result = async {
        // User already exists from signup/login, no need to get_or_create_user
        let doc = db.create_document(user_id, &file_name, &file_text).await?;

        if let Ok(title) = app_state.title_adapter.generate_title_from_text(&file_text).await {
            let _ = db.update_document_title(doc.id, &title).await;
        }
        
        db.create_session(user_id, doc.id).await
    }
    .await;

    match result {
        Ok(session) => {
            let response = CreateSessionResponse {
                session_id: session.id,
                document_id: session.document_id,
                user_id: session.user_id,
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            error!("Failed to create session: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create session".to_string(),
            ))
        }
    }
}

 #[utoipa::path(
    get,
    path = "/sessions",
    responses(
        (status = 200, description = "Sessions retrieved successfully", body = ListSessionsResponse),
        (status = 401, description = "Unauthorized - no valid session"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("session_cookie" = [])
    )
)]
pub async fn list_sessions_handler(
    State(app_state): State<Arc<AppState>>,
    Extension(user_id): Extension<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let sessions = app_state
        .db
        .get_sessions_by_user(user_id)
        .await
        .map_err(|e| {
            error!("Failed to fetch sessions: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch sessions".to_string())
        })?;

    let mut session_items = Vec::new();
    
    // ✅ Fetch document title for each session
    for session in sessions {
        let document = app_state
            .db
            .get_document_by_id(session.document_id)
            .await
            .ok(); // Ignore errors, just use None for title
        
        session_items.push(SessionListItem {
            session_id: session.id,
            document_id: session.document_id,
            title: document.and_then(|d| d.title),  // ✅ Get title from document
            created_at: session.created_at.to_rfc3339(),
        });
    }

    let response = ListSessionsResponse {
        sessions: session_items,
    };
    
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    get,
    path = "/sessions/{session_id}/notes",
    params(
        ("session_id" = Uuid, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "Notes retrieved successfully", body = ListNotesResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("session_cookie" = [])
    )
)]
pub async fn list_notes_handler(
    State(app_state): State<Arc<AppState>>,
    Extension(user_id): Extension<Uuid>,
    axum::extract::Path(session_id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // First, verify the session belongs to this user
    let session = app_state
        .db
        .get_session_by_id(session_id)
        .await
        .map_err(|e| {
            error!("Failed to get session: {:?}", e);
            (StatusCode::NOT_FOUND, "Session not found".to_string())
        })?;
    
    if session.user_id != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }
    
    // Fetch notes for this session
    let notes = app_state
        .db
        .get_notes_for_session(session_id)
        .await
        .map_err(|e| {
            error!("Failed to fetch notes: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch notes".to_string())
        })?;
    
    let note_items: Vec<NoteItem> = notes
        .into_iter()
        .map(|n| NoteItem {
            note_id: n.id,
            session_id: n.session_id,
            text: n.generated_note_text,
            created_at: n.created_at.to_rfc3339(),
        })
        .collect();
    
    let response = ListNotesResponse {
        notes: note_items,
    };
    
    Ok((StatusCode::OK, Json(response)))
}