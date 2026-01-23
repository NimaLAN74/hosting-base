-- Migration: Create Comp AI Service Schema
-- Date: 2026-01-23
-- Description: Creates schema and tables for Comp AI service request history

-- Create schema
CREATE SCHEMA IF NOT EXISTS comp_ai;

-- Create requests table
CREATE TABLE IF NOT EXISTS comp_ai.requests (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    request_text TEXT NOT NULL,
    response_text TEXT,
    model_used VARCHAR(100),
    tokens_used INTEGER,
    processing_time_ms BIGINT,
    status VARCHAR(50) DEFAULT 'completed',
    error_message TEXT,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_comp_ai_requests_user_id ON comp_ai.requests(user_id);
CREATE INDEX IF NOT EXISTS idx_comp_ai_requests_created_at ON comp_ai.requests(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_comp_ai_requests_status ON comp_ai.requests(status);

-- Add comment to table
COMMENT ON TABLE comp_ai.requests IS 'Stores Comp AI service request history';
COMMENT ON COLUMN comp_ai.requests.user_id IS 'Keycloak user ID (sub claim)';
COMMENT ON COLUMN comp_ai.requests.request_text IS 'User prompt/request text';
COMMENT ON COLUMN comp_ai.requests.response_text IS 'AI model response';
COMMENT ON COLUMN comp_ai.requests.model_used IS 'AI model identifier';
COMMENT ON COLUMN comp_ai.requests.tokens_used IS 'Number of tokens consumed';
COMMENT ON COLUMN comp_ai.requests.processing_time_ms IS 'Processing time in milliseconds';
COMMENT ON COLUMN comp_ai.requests.status IS 'Request status: completed, failed, processing';
