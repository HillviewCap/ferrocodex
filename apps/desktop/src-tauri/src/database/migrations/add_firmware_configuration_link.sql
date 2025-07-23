-- Migration: Add firmware_version_id to configuration_versions table
-- This migration adds support for linking firmware versions to configuration versions

-- Add the new column to configuration_versions table
ALTER TABLE configuration_versions 
ADD COLUMN firmware_version_id INTEGER 
REFERENCES firmware_versions(id) ON DELETE SET NULL;

-- Create index for better query performance
CREATE INDEX idx_config_firmware_link 
ON configuration_versions(firmware_version_id);