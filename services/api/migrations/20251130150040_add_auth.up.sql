-- services/api/migrations/YYYYMMDDHHMMSS_add_auth.up.sql

-- Add auth columns to existing users table
ALTER TABLE users ADD COLUMN email TEXT UNIQUE;
ALTER TABLE users ADD COLUMN hashed_password TEXT;

-- Create auth_sessions table for login state
CREATE TABLE auth_sessions (
    id TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_auth_sessions_user_id ON auth_sessions(user_id);