//! services/api/src/web/middleware.rs
//!
//! Authentication middleware for protecting routes.

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

use crate::web::state::AppState;

/// Middleware that validates the auth session cookie and extracts the user_id.
/// 
/// If valid, inserts the user_id into request extensions for handlers to use.
/// If invalid or missing, returns 401 Unauthorized.
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Extract cookie header
    let cookie_header = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 2. Parse session ID from cookie
    let auth_session_id = cookie_header
        .split(';')
        .find_map(|c| {
            let c = c.trim();
            c.strip_prefix("session=")
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 3. Validate auth session in database, get user_id
    let user_id = state
        .db
        .validate_auth_session(auth_session_id)
        .await
        .map_err(|e| {
            error!("Failed to validate auth session: {:?}", e);
            StatusCode::UNAUTHORIZED
        })?;

    // 4. Insert user_id into request extensions
    req.extensions_mut().insert(user_id);

    // 5. Continue to the handler
    Ok(next.run(req).await)
}