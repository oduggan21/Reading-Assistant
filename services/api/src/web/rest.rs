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
        crate::web::auth::signup_handler,    // Add
        crate::web::auth::login_handler,     // Add
        crate::web::auth::logout_handler,    // Add
    ),
    components(
        schemas(
            CreateSessionResponse,
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