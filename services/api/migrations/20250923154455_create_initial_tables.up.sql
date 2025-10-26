-- services/api/migrations/YYYYMMDDHHMMSS_create_initial_tables.up.sql
-- This migration creates the initial set of tables for the application.

-- The `users` table will store a unique identifier for each user.
CREATE TABLE users (
    user_id UUID PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- The `documents` table will store the original text uploaded by users.
CREATE TABLE documents (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(user_id),
    original_text TEXT NOT NULL,
    -- title and created_at can be added later if needed by the application
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- The `sessions` table links a user to a document and tracks their progress.
CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(user_id),
    document_id UUID NOT NULL REFERENCES documents(id),
    reading_progress_index INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- The `qa_pairs` table stores the history of questions and answers for a session.
CREATE TABLE qa_pairs (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id),
    question_text TEXT NOT NULL,
    answer_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- The `notes` table stores the final, summarized notes generated from Q&A pairs.
CREATE TABLE notes (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id),
    generated_note_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
