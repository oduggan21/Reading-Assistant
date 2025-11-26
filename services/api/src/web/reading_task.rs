//! services/api/src/web/reading_task.rs
//!
//! This module contains the asynchronous "worker" function responsible for
//! the document reading process.

use crate::web::{
    protocol::ServerMessage,
    state::{AppState, SessionState},
};
use axum::extract::ws::{Message, WebSocket};
use futures::{stream::SplitSink, SinkExt};
use reading_assistant_core::ports::{PortError, PortResult};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

/// The main asynchronous task for reading the document aloud.
///
/// This is a long-running task that loops through the document's sentences,
/// generates audio for each one, and streams it to the client.
/// It is designed to be gracefully cancelled via a `CancellationToken`.
pub async fn reading_process(
    app_state: Arc<AppState>,
    session_state_lock: Arc<Mutex<SessionState>>,
    ws_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>, // Now accepts the shared sender
    cancellation_token: CancellationToken,
) -> PortResult<()> {
    info!("Reading process started.");

    let start_msg = ServerMessage::ReadingStarted;
    let start_json = serde_json::to_string(&start_msg).unwrap();
    if ws_sender.lock().await.send(Message::Text(start_json.into())).await.is_err() {
        return Err(PortError::Unexpected(
            "Failed to send ReadingStarted message.".to_string(),
        ));
    }

    loop {
        if cancellation_token.is_cancelled() {
            info!("Reading process cancelled.");
            return Ok(());
        }

        let (current_index, sentence_to_read, session_id) = {
            let session = session_state_lock.lock().await;
            let current_index = session.reading_progress_index;
            if current_index >= session.chunked_document.len() {
                break;
            }
            let sentence_to_read = session.chunked_document[current_index].clone();
            let session_id = session.session_id;
            (current_index, sentence_to_read, session_id)
        };

        let audio_data = app_state
            .tts_adapter
            .generate_audio(&sentence_to_read)
            .await?;

        if ws_sender.lock().await.send(Message::Binary(audio_data.into())).await.is_err() {
            error!("Failed to send audio chunk to client. Ending reading task.");
            break;
        }

        {
            let mut session = session_state_lock.lock().await;
            session.reading_progress_index += 1;
        }

        app_state
            .db
            .update_session_progress(session_id, current_index + 1)
            .await?;
    }

    info!("Document reading finished.");
    let end_msg = ServerMessage::ReadingEnded;
    let end_json = serde_json::to_string(&end_msg).unwrap();
    if ws_sender.lock().await.send(Message::Text(end_json.into())).await.is_err() {
        error!("Failed to send ReadingEnded message.");
    }

    Ok(())
}
