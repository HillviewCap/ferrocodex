-- Migration: Fix firmware metadata issues
-- This migration fixes existing firmware records with invalid or missing metadata

-- Update firmware records with NULL or invalid file_size
UPDATE firmware_versions 
SET file_size = 0 
WHERE file_size IS NULL OR file_size < 0;

-- Update firmware records with NULL or empty file_hash
UPDATE firmware_versions 
SET file_hash = 'unknown' 
WHERE file_hash IS NULL OR file_hash = '';

-- Update firmware records with invalid created_at timestamps
UPDATE firmware_versions 
SET created_at = datetime('now') 
WHERE created_at IS NULL OR created_at = '' OR datetime(created_at) IS NULL;

-- Ensure all firmware records have proper status
UPDATE firmware_versions 
SET status = 'Draft' 
WHERE status IS NULL OR status = '' OR status NOT IN ('Draft', 'Approved', 'Golden', 'Archived');