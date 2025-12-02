pub mod domain;
pub mod ports;

pub use domain::{Document, Note, QAPair, Session,User, UserCredentials, AuthSession};
pub use ports::{ DatabaseService, NoteGenerationService, PortError, PortResult, QuestionAnsweringService,
    SpeechToTextService, TextToSpeechService};

