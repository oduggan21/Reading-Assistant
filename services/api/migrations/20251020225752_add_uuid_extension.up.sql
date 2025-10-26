
-- Enable the uuid-ossp extension for automatic UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Modify sessions table to auto-generate UUIDs
ALTER TABLE sessions ALTER COLUMN id SET DEFAULT uuid_generate_v4();

-- Also fix other tables while we're at it
ALTER TABLE documents ALTER COLUMN id SET DEFAULT uuid_generate_v4();
ALTER TABLE qa_pairs ALTER COLUMN id SET DEFAULT uuid_generate_v4();
ALTER TABLE notes ALTER COLUMN id SET DEFAULT uuid_generate_v4();