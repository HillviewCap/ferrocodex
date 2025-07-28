-- Migration: Add request_id column to audit_events for request correlation
-- Story: EH-2.1 - Request ID Tracking and Correlation System

-- Add request_id column to audit_events table
ALTER TABLE audit_events ADD COLUMN request_id TEXT;

-- Create index for efficient request ID lookups
CREATE INDEX IF NOT EXISTS idx_audit_events_request_id ON audit_events(request_id);

-- Add comment to track migration purpose
-- This enables end-to-end request tracking and correlation across system layers