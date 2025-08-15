-- Create table for storing URLs and analytics
CREATE TABLE urls (
    id SERIAL PRIMARY KEY,
    short_code VARCHAR(16) UNIQUE NOT NULL,
    original_url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    transition_count BIGINT NOT NULL DEFAULT 0
);
