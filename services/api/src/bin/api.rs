//! services/api/src/bin/api.rs

use api_lib::{
    adapters::{
        db::DbAdapter, notes_llm::OpenAiNotesAdapter, sst::OpenAiSstAdapter,
        tts::OpenAiTtsAdapter, qa_llm::OpenAiQaAdapter,
    },
    config::Config,
    error::ApiError,
    web::{
        auth::{signup_handler, login_handler, logout_handler},
        create_session_handler, rest::ApiDoc, state::AppState, ws_handler,
        middleware::require_auth,
    },
};
use async_openai::{
    config::OpenAIConfig,
    types::{SpeechModel, Voice},
    Client,
};
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
    middleware as axum_middleware,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
// ✅ Add these imports
use tower_http::cors::CorsLayer;
use axum::http::{Method, HeaderValue, header::{AUTHORIZATION, CONTENT_TYPE, ACCEPT}};

#[tokio::main]
async fn main() -> Result<(), ApiError> {
    // --- 1. Load Configuration & Set Up Logging ---
    let config = Arc::new(Config::from_env()?);
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(config.log_level.to_string()))
        .with(tracing_subscriber::fmt::layer())
        .init();
    info!("Configuration loaded. Starting server...");

    // --- 2. Connect to Database & Run Migrations ---
    info!("Connecting to database...");
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;
    let db_adapter = Arc::new(DbAdapter::new(db_pool.clone()));
    info!("Running database migrations...");
    db_adapter.run_migrations().await?;
    info!("Database migrations complete.");

    // --- 3. Initialize Service Adapters ---
    let openai_config = OpenAIConfig::new().with_api_key(
        config
            .openai_api_key
            .as_ref()
            .ok_or_else(|| ApiError::Internal("OPENAI_API_KEY is required".to_string()))?,
    );
    let openai_client = Client::with_config(openai_config);

    let sst_adapter = Arc::new(OpenAiSstAdapter::new(
        openai_client.clone(),
        config.sst_model.clone(),
    ));

    let tts_voice = match config.tts_voice.to_lowercase().as_str() {
        "alloy" => Voice::Alloy,
        "echo" => Voice::Echo,
        "fable" => Voice::Fable,
        "onyx" => Voice::Onyx,
        "nova" => Voice::Nova,
        "shimmer" => Voice::Shimmer,
        _ => {
            return Err(ApiError::Internal(format!(
                "Invalid TTS voice specified in config: '{}'",
                config.tts_voice
            )))
        }
    };
    let tts_adapter = Arc::new(OpenAiTtsAdapter::new(
        openai_client.clone(),
        SpeechModel::Tts1Hd,
        tts_voice,
    ));

    let qa_adapter = Arc::new(OpenAiQaAdapter::new(
        openai_client.clone(),
        config.qa_model.clone(),
    ));
    let notes_adapter = Arc::new(OpenAiNotesAdapter::new(
        openai_client.clone(),
        config.note_model.clone(),
    ));

    // --- 4. Build the Shared AppState ---
    let app_state = Arc::new(AppState {
        db: db_adapter,
        config: config.clone(),
        sst_adapter,
        tts_adapter,
        qa_adapter,
        notes_adapter,
    });

    let cors = CorsLayer::new()
    .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
    .allow_credentials(true)
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE, ACCEPT]);
    // --- 6. Create the Web Router ---
  // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/auth/signup", post(signup_handler))
        .route("/auth/login", post(login_handler))
        .route("/auth/logout", post(logout_handler));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        .route("/sessions", post(create_session_handler))
        .route("/ws", get(ws_handler))
        .layer(axum_middleware::from_fn_with_state(
            app_state.clone(),
            require_auth,
        ));

// Combine API routes
let api_router = Router::new()
    .merge(public_routes)
    .merge(protected_routes)
    .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
    .layer(cors)
    .with_state(app_state);

    // Merge the API router with the Swagger UI router for a complete application.
    let app = Router::new()
        .merge(api_router)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    // --- 7. Start the Server ---
    info!("Starting server on {}", config.bind_address);
    info!(
        "Swagger UI available at http://{}/swagger-ui",
        config.bind_address
    );
    let listener = tokio::net::TcpListener::bind(&config.bind_address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}