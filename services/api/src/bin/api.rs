//! services/api/src/bin/api.rs

use api_lib::{
    adapters::{
        db::DbAdapter, notes_llm::OpenAiNotesAdapter, sst::OpenAiSstAdapter,
        tts::OpenAiTtsAdapter, qa_llm::OpenAiQaAdapter,
    },
    config::Config,
    error::ApiError,
    web::{
        create_session_handler, rest::ApiDoc, state::AppState, ws_handler,
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
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
// ✅ Add these imports
use tower_http::cors::{CorsLayer, Any};

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
        .allow_origin(Any) // For development - allow any origin
        .allow_methods(Any) // Allow all HTTP methods
        .allow_headers(Any); // Allow all headers

    // --- 6. Create the Web Router ---
    let api_router = Router::new()
        .route("/sessions", post(create_session_handler))
        .route("/ws", get(ws_handler))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        .layer(cors) // ✅ Add CORS layer here
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