//! services/api/src/web/auth.rs
//!
//! Authentication endpoints for user signup, login, and logout.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;
use utoipa::ToSchema;
use crate::web::state::AppState;

//=========================================================================================
// Request/Response Types
//=========================================================================================

#[derive(Deserialize, ToSchema)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct AuthResponse {
    pub user_id: Uuid,
    pub email: String,
}

//=========================================================================================
// Handlers
//=========================================================================================

/// POST /auth/signup - Create a new user account
#[utoipa::path(
    post,
    path = "/auth/signup",
    request_body = SignupRequest,
    responses(
        (status = 201, description = "User created successfully", body = AuthResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn signup_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SignupRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 1. Hash the password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|e| {
            error!("Failed to hash password: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to hash password".to_string())
        })?
        .to_string();

    // 2. Create user in database
    let user = state
        .db
        .create_user_with_email(&req.email, &password_hash)
        .await
        .map_err(|e| {
            error!("Failed to create user: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create user".to_string())
        })?;

    // 3. Generate auth session ID
    let auth_session_id = Uuid::new_v4().to_string();

    // 4. Set expiration (30 days)
    let expires_at = Utc::now() + Duration::days(30);

    // 5. Create auth session in database
    state
        .db
        .create_auth_session(&auth_session_id, user.user_id, expires_at)
        .await
        .map_err(|e| {
            error!("Failed to create auth session: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create session".to_string())
        })?;

    // 6. Create session cookie
    let cookie = format!(
        "session={}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={}",
        auth_session_id,
        Duration::days(30).num_seconds()
    );

    // 7. Return response with cookie
    let response = AuthResponse {
        user_id: user.user_id,
        email: user.email.unwrap_or_default(),
    };

    Ok((
        StatusCode::CREATED,
        [(header::SET_COOKIE, cookie)],
        Json(response),
    ))
}

/// POST /auth/login - Login with existing account
#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 1. Get user by email
    let user_creds = state
        .db
        .get_user_by_email(&req.email)
        .await
        .map_err(|e| {
            error!("Failed to get user: {:?}", e);
            (StatusCode::UNAUTHORIZED, "Invalid email or password".to_string())
        })?;

    // 2. Verify password
    let parsed_hash = PasswordHash::new(&user_creds.hashed_password).map_err(|e| {
        error!("Failed to parse password hash: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Authentication error".to_string())
    })?;

    let valid = Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .is_ok();

    if !valid {
        return Err((StatusCode::UNAUTHORIZED, "Invalid email or password".to_string()));
    }

    // 3. Generate auth session ID
    let auth_session_id = Uuid::new_v4().to_string();

    // 4. Set expiration (30 days)
    let expires_at = Utc::now() + Duration::days(30);

    // 5. Create auth session in database
    state
        .db
        .create_auth_session(&auth_session_id, user_creds.user_id, expires_at)
        .await
        .map_err(|e| {
            error!("Failed to create auth session: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create session".to_string())
        })?;

    // 6. Create session cookie
    let cookie = format!(
        "session={}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={}",
        auth_session_id,
        Duration::days(30).num_seconds()
    );

    // 7. Return response with cookie
    let response = AuthResponse {
        user_id: user_creds.user_id,
        email: user_creds.email,
    };

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(response),
    ))
}

/// POST /auth/logout - Logout and invalidate session
#[utoipa::path(
    post,
    path = "/auth/logout",
    responses(
        (status = 200, description = "Logout successful"),
        (status = 401, description = "No active session")
    )
)]
pub async fn logout_handler(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 1. Extract session cookie
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or((StatusCode::UNAUTHORIZED, "No session found".to_string()))?;

    // 2. Parse session ID from cookie
    let auth_session_id = cookie_header
        .split(';')
        .find_map(|c| {
            let c = c.trim();
            c.strip_prefix("session=")
        })
        .ok_or((StatusCode::UNAUTHORIZED, "No session found".to_string()))?;

    // 3. Delete auth session from database
    state
        .db
        .delete_auth_session(auth_session_id)
        .await
        .map_err(|e| {
            error!("Failed to delete auth session: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to logout".to_string())
        })?;

    // 4. Clear cookie
    let cookie = "session=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0";

    Ok((StatusCode::OK, [(header::SET_COOKIE, cookie.to_string())]))
}