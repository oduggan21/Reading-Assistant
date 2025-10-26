//! services/api/src/web/rest.rs
//!
//! Contains the Axum handlers for the REST API endpoints and the master
//! definition for the OpenAPI specification.

use crate::web::state::AppState;
use axum::{
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
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
    ),
    components(
        schemas(CreateSessionResponse)
    ),
    tags(
        (name = "Reading Assistant API", description = "API endpoints for the interactive audio reader.")
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
/// Accepts a multipart/form-data request with a single file part.
/// A `x-user-id` header is required to associate the session with a user.
#[utoipa::path(
    post,
    path = "/sessions",
    request_body(content_type = "multipart/form-data", description = "The document to upload."),
    responses(
        (status = 201, description = "Session created successfully", body = CreateSessionResponse),
        (status = 400, description = "Bad request (e.g., missing header or file)"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("x-user-id" = Uuid, Header, description = "The unique ID of the user.")
    )
)]
pub async fn create_session_handler(
    State(app_state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_id_str = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "x-user-id header is required".to_string(),
            )
        })?;

    let user_id = Uuid::parse_str(user_id_str).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid x-user-id format".to_string(),
        )
    })?;

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
        db.get_or_create_user(user_id).await?;
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

