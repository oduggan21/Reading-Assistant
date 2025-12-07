//! services/api/src/web/protocol.rs
//!
//! Defines the WebSocket message protocol between the browser client and the API server
//! for the interactive audio reader application.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

//=========================================================================================
// Messages Sent FROM the Client (Browser) TO the Server
//=========================================================================================
// NOTE: User's question audio is sent as raw Binary frames, not as part of this enum.
//=========================================================================================

/// Represents the structured text messages a client can send to the server.
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Initializes a session. This must be the first message sent on the connection.
    Init { session_id: Uuid },

    /// Signals that the user has started speaking, interrupting the reader.
    /// The server should cancel the reading process and prepare to receive audio.
    InterruptStarted,

    /// Signals that the user has finished speaking their question.
    /// The server should now process the buffered audio.
    InterruptEnded,

    /// A user-initiated command to continue reading from the last position.
    ResumeReading,

    /// A user-initiated command to pause the reading.
    PauseReading,

    UpdateProgress { session_id: Uuid, sentence_index: usize },
}

//=========================================================================================
// Messages Sent FROM the Server TO the Client (Browser)
//=========================================================================================
// NOTE: The reader's voice (both document and answers) is sent as raw Binary frames,
// not as part of this enum. These messages provide context for that audio.
//=========================================================================================

/// Represents the structured text messages the server can send to the client.
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Confirms successful session initialization.
    SessionInitialized { session_id: Uuid },

    /// Reports a fatal error to the client, which should display an error message.
    Error { message: String },

    /// Signals that the server is now streaming audio for the document reading.
    /// The UI can update to a "playing" state.
    ReadingStarted,

    /// Signals that the reading has been paused.
    ReadingPaused,

    /// Signals that the entire document has been read successfully.
    ReadingEnded,

    /// Signals that the server is processing the user's question and generating an answer.
    /// The UI can update to a "thinking..." or "listening..." state.
    AnsweringStarted,

    /// Signals that the AI has finished speaking its answer.
    /// The UI can transition back to an idle/listening state.
    AnsweringEnded,
}