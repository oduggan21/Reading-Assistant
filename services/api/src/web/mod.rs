pub mod protocol;
pub mod qa_task;
pub mod reading_task;
pub mod state;
pub mod ws_handler;
pub mod rest;
pub mod auth;
pub mod middleware;

// Re-export the main WebSocket handler to make it easily accessible
// to the binary that will build the web server router.
pub use ws_handler::ws_handler;
pub use rest::{create_session_handler, list_sessions_handler, list_notes_handler};
pub use middleware::require_auth;