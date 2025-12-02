//! services/api/src/web/ws_handler.rs
//!
//! This is the main entry point and control loop for a WebSocket connection.
//! It manages the session's state machine and delegates tasks.

use crate::{
    web::{
        protocol::{ClientMessage, ServerMessage},
        qa_task::{qa_process, QaOutcome},
        reading_task::reading_process,
        state::{AppState, SessionMode, SessionState},
    },
};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    Extension,
};
use futures::{stream::{SplitSink, StreamExt}, SinkExt};
use std::sync::Arc;
use tokio::{sync::Mutex, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

/// The handler for upgrading HTTP requests to WebSocket connections.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(app_state): State<Arc<AppState>>,
    Extension(user_id): Extension<Uuid>,  // ✅ Add this - from auth middleware
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, app_state, user_id))  // ✅ Pass user_id
}

async fn handle_socket(socket: WebSocket, app_state: Arc<AppState>, user_id: Uuid) {  // ✅ Add user_id param
    info!("New WebSocket connection established for user: {}", user_id);

    // The sender is wrapped in an Arc<Mutex<>> to allow for shared mutable access across tasks.
    let (sender, mut receiver) = socket.split();
    let ws_sender = Arc::new(Mutex::new(sender));

    let session_state_lock: Arc<Mutex<SessionState>>;

    // --- 1. Initialization Phase ---
    if let Some(Ok(Message::Text(init_json))) = receiver.next().await {
        match serde_json::from_str::<ClientMessage>(&init_json) {
            Ok(ClientMessage::Init { session_id }) => {
                info!("Initializing session with ID: {}", session_id);
                
                // ✅ Validate that the session belongs to this user
                match app_state.db.get_session_by_id(session_id).await {
                    Ok(session) => {
                        if session.user_id != user_id {
                            error!("Session {} does not belong to user {}", session_id, user_id);
                            let err_msg = ServerMessage::Error {
                                message: "Unauthorized: Session does not belong to this user.".to_string(),
                            };
                            let err_json = serde_json::to_string(&err_msg).unwrap();
                            let _ = ws_sender.lock().await.send(Message::Text(err_json.into())).await;
                            return;
                        }
                    }
                    Err(e) => {
                        error!("Failed to get session: {:?}", e);
                        let err_msg = ServerMessage::Error {
                            message: "Failed to load session data.".to_string(),
                        };
                        let err_json = serde_json::to_string(&err_msg).unwrap();
                        let _ = ws_sender.lock().await.send(Message::Text(err_json.into())).await;
                        return;
                    }
                }
                
                match SessionState::new(app_state.clone(), session_id).await {
                    Ok(state) => {
                        session_state_lock = Arc::new(Mutex::new(state));
                        let init_msg = ServerMessage::SessionInitialized { session_id };
                        let init_json = serde_json::to_string(&init_msg).unwrap();
                        if ws_sender.lock().await.send(Message::Text(init_json.into())).await.is_err() {
                            error!("Failed to send session initialized message.");
                            return;
                        }
                        let welcome_text = "Hi there! I am looking forward to discussing the information you have provided today! If at any point you have a question, please feel free to interrupt me, or if you need to pause our session, just click pause! I will now begin reading the information!";
                
                        match app_state.tts_adapter.generate_audio(welcome_text).await {
                            Ok(welcome_audio) => {
                                if ws_sender.lock().await.send(Message::Binary(welcome_audio.into())).await.is_err() {
                                    error!("Failed to send welcome audio.");
                                    return;
                                }
                            }
                            Err(e) => {
                                error!("Failed to generate welcome audio: {:?}", e);
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to initialize session state: {:?}", e);
                        let err_msg = ServerMessage::Error {
                            message: "Failed to load session data.".to_string(),
                        };
                        let err_json = serde_json::to_string(&err_msg).unwrap();
                        let _ = ws_sender.lock().await.send(Message::Text(err_json.into())).await;
                        return;
                    }
                }
            }
            _ => {
                error!("First message was not a valid Init message.");
                return;
            }
        }
    } else {
        error!("Client disconnected before sending Init message.");
        return;
    }

    // --- 2. Main Message Loop ---
    // Rest of the function stays exactly the same...
    let mut reading_task_handle: Option<JoinHandle<()>> = {
        let session = session_state_lock.lock().await;
        let task = {
            let app_state = app_state.clone();
            let session_state_lock = session_state_lock.clone();
            let ws_sender = ws_sender.clone();
            let token = session.cancellation_token.clone();
            tokio::spawn(async move {
                if let Err(e) = reading_process(app_state, session_state_lock, ws_sender, token).await {
                    error!("Reading process failed: {:?}", e);
                }
            })
        };
        Some(task)
    };

    loop {
        if let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    handle_text_message(
                        text.to_string(),
                        &app_state,
                        &session_state_lock,
                        &ws_sender,
                        &mut reading_task_handle,
                    )
                    .await;
                }
                Message::Binary(data) => {
                    let mut session = session_state_lock.lock().await;
                    if session.current_mode == SessionMode::InterruptedListening {
                        session.audio_buffer.extend_from_slice(&data);
                    }
                }
                Message::Close(_) => {
                    info!("Client sent close message.");
                    break;
                }
                _ => {}
            }
        } else {
            info!("Client disconnected.");
            break;
        }
    }

    // --- 3. Cleanup ---
    if let Some(handle) = reading_task_handle {
        handle.abort();
    }
    info!("WebSocket connection closed.");
}

/// Helper function to handle the logic for different `ClientMessage` variants.
async fn handle_text_message(
    text: String,
    app_state: &Arc<AppState>,
    session_state_lock: &Arc<Mutex<SessionState>>,
    ws_sender: &Arc<Mutex<SplitSink<WebSocket, Message>>>,
    reading_task_handle: &mut Option<JoinHandle<()>>,
) {
    match serde_json::from_str::<ClientMessage>(&text) {
        Ok(client_msg) => match client_msg {
            ClientMessage::InterruptStarted => {
                info!("InterruptStarted message received. Cancelling reading task.");
                let mut session = session_state_lock.lock().await;
                session.cancellation_token.cancel();
                session.current_mode = SessionMode::InterruptedListening;
                session.audio_buffer.clear();
            }
            ClientMessage::InterruptEnded => {
                info!("InterruptEnded message received.");
                {
                    let mut session = session_state_lock.lock().await;
                    session.current_mode = SessionMode::ProcessingQuestion;
                }

                match qa_process(
                    app_state.clone(),
                    session_state_lock.clone(),
                    ws_sender.clone(), // Cloning the Arc is cheap and correct.
                )
                .await
                {
                    Ok(QaOutcome::ResumeReading) => {
                        info!("QA process resulted in ResumeReading. Restarting reading task.");
                        let mut session = session_state_lock.lock().await;
                            // Check if all audio already generated
                            if session.reading_progress_index >= session.chunked_document.len() {
                                info!("All audio already generated, just resuming frontend playback");
                                let start_msg = ServerMessage::ReadingStarted;
                                let start_json = serde_json::to_string(&start_msg).unwrap();
                                if ws_sender.lock().await.send(Message::Text(start_json.into())).await.is_err() {
                                    error!("Failed to send ReadingStarted message.");
                                }
                                if ws_sender.lock().await.send(Message::Binary(vec![].into())).await.is_err() {
                                    error!("Failed to send empty audio trigger.");
                                }
                        } 
                        else{
                        info!("We entered into here");
                        session.current_mode = SessionMode::Reading;
                        session.cancellation_token = CancellationToken::new();
                        let task = {
                            let app_state = app_state.clone();
                            let session_state_lock = session_state_lock.clone();
                            let ws_sender = ws_sender.clone();
                            let token = session.cancellation_token.clone();
                            tokio::spawn(async move {
                                info!("reading task being started");
                                if let Err(e) = reading_process(app_state, session_state_lock, ws_sender, token).await {
                                    error!("Reading process failed: {:?}", e);
                                }
                            })
                        };
                        *reading_task_handle = Some(task);
                    }
                    }
                    Ok(QaOutcome::QuestionAnswered) => {
                        info!("QA process resulted in QuestionAnswered. Awaiting next interrupt.");
                        let mut session = session_state_lock.lock().await;
                        session.current_mode = SessionMode::InterruptedListening;
                    }
                    Err(e) => {
                        error!("Error in QA process: {:?}", e);
                        let mut session = session_state_lock.lock().await;
                        session.current_mode = SessionMode::InterruptedListening;
                    }
                }
            }
            ClientMessage::PauseReading => {
                info!("PauseReading message received.");
                let mut session = session_state_lock.lock().await;
                session.cancellation_token.cancel();
                session.current_mode = SessionMode::Paused;
            }
            ClientMessage::ResumeReading => {
            info!("ResumeReading message received.");
            let mut session = session_state_lock.lock().await;
            if session.current_mode == SessionMode::Paused {
                // Check if all audio already generated
                if session.reading_progress_index >= session.chunked_document.len() {
                    info!("All audio already generated, just resuming frontend playback");
                    let start_msg = ServerMessage::ReadingStarted;
                    let start_json = serde_json::to_string(&start_msg).unwrap();
                    if ws_sender.lock().await.send(Message::Text(start_json.into())).await.is_err() {
                        error!("Failed to send ReadingStarted message.");
                    }
                    if ws_sender.lock().await.send(Message::Binary(vec![].into())).await.is_err() {
                        error!("Failed to send empty audio trigger.");
                    }
                } else {
                    // Still have sentences to generate
                    session.current_mode = SessionMode::Reading;
                    session.cancellation_token = CancellationToken::new();
                    let task = {
                        let app_state = app_state.clone();
                        let session_state_lock = session_state_lock.clone();
                        let ws_sender = ws_sender.clone();
                        let token = session.cancellation_token.clone();
                        tokio::spawn(async move {
                            if let Err(e) = reading_process(app_state, session_state_lock, ws_sender, token).await {
                                error!("Reading process failed: {:?}", e);
                            }
                        })
                    };
                    *reading_task_handle = Some(task);
                }
            }
        }
            ClientMessage::Init { .. } => {
                warn!("Received subsequent Init message, which is ignored.");
            }
        },
        Err(e) => {
            warn!("Failed to deserialize client message: {}", e);
        }
    }
}
