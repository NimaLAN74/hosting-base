-- Phase B: Evidence file upload and extracted text for AI analysis
-- Add columns for uploaded files and extracted content

ALTER TABLE comp_ai.evidence
  ADD COLUMN IF NOT EXISTS file_path TEXT,
  ADD COLUMN IF NOT EXISTS file_name VARCHAR(500),
  ADD COLUMN IF NOT EXISTS content_type VARCHAR(100),
  ADD COLUMN IF NOT EXISTS extracted_text TEXT;

COMMENT ON COLUMN comp_ai.evidence.file_path IS 'Relative path under evidence storage (uploaded file)';
COMMENT ON COLUMN comp_ai.evidence.file_name IS 'Original filename';
COMMENT ON COLUMN comp_ai.evidence.content_type IS 'MIME type of uploaded file';
COMMENT ON COLUMN comp_ai.evidence.extracted_text IS 'Text extracted for AI analysis (PDF, docx, etc.)';
