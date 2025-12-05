//! services/api/src/web/qa_task.rs
//!
//! This module contains the asynchronous "worker" function responsible for
//! handling a single question-and-answer cycle.

use crate::web::{
    protocol::ServerMessage,
    state::{AppState, SessionState},
};
use axum::extract::ws::{Message, WebSocket};
use futures::{stream::SplitSink, SinkExt};
use reading_assistant_core::{
    domain::QAPair,
    ports::{PortError, PortResult},
};


use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;
use std::time::Instant;


/// Represents the outcome of the `qa_process` task.
/// This tells the main handler what action to take next.
#[derive(Debug, PartialEq, Eq)]
pub enum QaOutcome {
    /// The user's speech was a command to resume reading.
    ResumeReading,
    /// The user's question was successfully answered.
    QuestionAnswered,
}

/// The main asynchronous task for handling a single user question.
pub async fn qa_process(
    app_state: Arc<AppState>,
    session_state_lock: Arc<Mutex<SessionState>>,
    ws_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
) -> PortResult<QaOutcome> {
    let start_time = Instant::now();
    info!("QA process started.");

    let start_msg = ServerMessage::AnsweringStarted;
    let start_json = serde_json::to_string(&start_msg).unwrap();
    if ws_sender.lock().await.send(Message::Text(start_json.into())).await.is_err() {
        return Err(PortError::Unexpected(
            "Failed to send AnsweringStarted message.".to_string(),
        ));
    }

    let (audio_buffer, context, session_id) = {
    let mut session = session_state_lock.lock().await;
    let audio_buffer = std::mem::take(&mut session.audio_buffer);
    
    // Build context using helper function
    let doc_context = get_context_from_document(&session);
    let context = if let (Some(prev_q), Some(prev_a)) = (&session.last_question, &session.last_answer) {
        format!(
            "DOCUMENT CONTEXT:\n{}\n\nPREVIOUS Q&A:\nQ: {}\nA: {}",
            doc_context, prev_q, prev_a
        )
    } else {
        doc_context
    };
    
    let session_id = session.session_id;
    (audio_buffer, context, session_id)
    };

    let stt_start = Instant::now();
    let question_text = app_state
        .sst_adapter
        .transcribe_audio(&audio_buffer)
        .await?;
    let stt_duration = stt_start.elapsed();
    info!("⏱️ STT took: {:?}", stt_duration);
    info!("Transcribed question: '{}'", question_text);

    let lowercased_question = question_text.to_lowercase();
    if lowercased_question.contains("continue reading")
        || lowercased_question.contains("resume reading")
        || lowercased_question.contains("go on")
    {
        info!("'Resume reading' command detected.");
        return Ok(QaOutcome::ResumeReading);
    }

    let llm_start = Instant::now();
    let answer_text = app_state
        .qa_adapter
        .answer_question(&question_text, &context)
        .await?;
    let llm_duration = llm_start.elapsed();
    info!("⏱️ LLM took: {:?}", llm_duration);
    info!("Generated answer: '{}'", answer_text);
    {
    let mut session = session_state_lock.lock().await;
    session.last_question = Some(question_text.clone());
    session.last_answer = Some(answer_text.clone());
    }

    let notes_app_state = app_state.clone();
    let qapair = QAPair {
        id: Uuid::new_v4(),
        session_id,
        question_text,
        answer_text: answer_text.clone(),
    };
    tokio::spawn(generate_and_save_notes(notes_app_state, qapair));

    // ✅ Split into sentences and generate TTS in PARALLEL
    let tts_start = Instant::now();
    let sentences = split_into_sentences(&answer_text);
    
    info!("🔊 Generating audio for {} sentences in parallel", sentences.len());
    
    // Generate all TTS in parallel
    let mut tts_tasks = Vec::new();
    for sentence in sentences.iter() {
        let tts_adapter = app_state.tts_adapter.clone();
        let sentence = sentence.clone();
        tts_tasks.push(tokio::spawn(async move {
            tts_adapter.generate_audio(&sentence).await
        }));
    }

    // Wait for all TTS to complete
    let mut audio_chunks = Vec::new();
    for (i, task) in tts_tasks.into_iter().enumerate() {
        match task.await {
            Ok(Ok(audio_data)) => {
                audio_chunks.push(audio_data);
            }
            Ok(Err(e)) => {
                error!("TTS generation failed for sentence {}: {:?}", i + 1, e);
                return Err(e);
            }
            Err(e) => {
                error!("Task join error for sentence {}: {:?}", i + 1, e);
                return Err(PortError::Unexpected(e.to_string()));
            }
        }
    }

    // Send all chunks in order
    for audio_data in audio_chunks {
        if ws_sender.lock().await.send(Message::Binary(audio_data.into())).await.is_err() {
            return Err(PortError::Unexpected(
                "Failed to send answer audio chunk to client.".to_string(),
            ));
        }
    }
    
    let tts_duration = tts_start.elapsed();
    info!("⏱️ TTS (parallel) took: {:?}", tts_duration);

    let total_duration = start_time.elapsed();
    info!("⏱️ Total QA process took: {:?}", total_duration);
    info!("Finished sending answer audio.");
    
    let end_msg = ServerMessage::AnsweringEnded;
    let end_json = serde_json::to_string(&end_msg).unwrap();
    if ws_sender.lock().await.send(Message::Text(end_json.into())).await.is_err() {
        warn!("Failed to send AnsweringEnded message. Client may have disconnected.");
    }

    Ok(QaOutcome::QuestionAnswered)
}

// Helper function
fn split_into_sentences(text: &str) -> Vec<String> {
    text.split(". ")
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            let trimmed = s.trim();
            if trimmed.ends_with('.') {
                trimmed.to_string()
            } else {
                format!("{}.", trimmed)
            }
        })
        .collect()
}

/// A helper function to extract the last few sentences of context from the document.
fn get_context_from_document(session: &SessionState) -> String {
    let current_index = session.reading_progress_index;
    let total_sentences = session.chunked_document.len();
    
    // Calculate 10-sentence window around current position
    let start_index = if current_index < 5 {
        // Near start: window from 0
        0
    } else if current_index + 5 >= total_sentences {
        // Near end: last 10 sentences
        total_sentences.saturating_sub(10)
    } else {
        // Middle: center around current position
        current_index - 5
    };
    
    let end_index = (start_index + 10).min(total_sentences);
    
    session.chunked_document[start_index..end_index].join(" ")
}

/// A "fire-and-forget" background task to generate and save notes without blocking the user.
async fn generate_and_save_notes(app_state: Arc<AppState>, qapair: QAPair) {
    info!(
        "Spawning background task to save QAPair and generate notes for session {}.",
        qapair.session_id
    );

    if app_state.db.save_qa_pair(qapair.clone()).await.is_err() {
        error!(
            "Failed to save QAPair to database for session {}. Note generation will be skipped.",
            qapair.session_id
        );
        return;
    }

    match app_state
        .notes_adapter
        .generate_note_from_qapair(&qapair)
        .await
    {
        Ok(note_text) => {
            if note_text.trim() == "SKIP_NOTE" {
            info!(
                "Skipping note generation - question was unrelated for session {}",
                qapair.session_id
            );
            return;
            }
            let note = reading_assistant_core::domain::Note {
                id: Uuid::new_v4(),
                session_id: qapair.session_id,
                generated_note_text: note_text,
                created_at: chrono::Utc::now(), 
            };
            if app_state.db.save_note(note).await.is_err() {
                error!(
                    "Failed to save generated note to database for session {}.",
                    qapair.session_id
                );
            } else {
                info!(
                    "Successfully generated and saved note for session {}.",
                    qapair.session_id
                );
            }
        }
        Err(e) => {
            error!("Failed to generate note from QAPair: {}", e);
        }
    }
}
