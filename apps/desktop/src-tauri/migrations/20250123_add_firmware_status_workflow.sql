-- Migration: Add firmware status workflow
-- This migration adds support for full status workflow for firmware versions

-- First, add Approved status to existing firmware versions table
-- Note: This requires recreating the table due to SQLite limitations with CHECK constraints

-- Create temporary table with new schema
CREATE TABLE firmware_versions_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id INTEGER NOT NULL,
    author_id INTEGER NOT NULL,
    vendor TEXT,
    model TEXT,
    version TEXT NOT NULL,
    notes TEXT,
    status TEXT NOT NULL CHECK(status IN ('Draft', 'Approved', 'Golden', 'Archived')),
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    status_changed_at DATETIME,
    status_changed_by INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
    FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (status_changed_by) REFERENCES users(id) ON DELETE RESTRICT
);

-- Copy existing data to new table
INSERT INTO firmware_versions_new (
    id, asset_id, author_id, vendor, model, version, notes, status, 
    file_path, file_hash, file_size, created_at
)
SELECT 
    id, asset_id, author_id, vendor, model, version, notes, status,
    file_path, file_hash, file_size, created_at
FROM firmware_versions;

-- Drop old table
DROP TABLE firmware_versions;

-- Rename new table
ALTER TABLE firmware_versions_new RENAME TO firmware_versions;

-- Recreate indexes
CREATE INDEX idx_firmware_asset_id ON firmware_versions(asset_id);
CREATE INDEX idx_firmware_created_at ON firmware_versions(created_at);
CREATE INDEX idx_firmware_status ON firmware_versions(status);

-- Create firmware status history table
CREATE TABLE IF NOT EXISTS firmware_status_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    firmware_version_id INTEGER NOT NULL,
    old_status TEXT NOT NULL,
    new_status TEXT NOT NULL,
    changed_by INTEGER NOT NULL,
    changed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    reason TEXT,
    FOREIGN KEY (firmware_version_id) REFERENCES firmware_versions(id) ON DELETE CASCADE,
    FOREIGN KEY (changed_by) REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_firmware_status_history_firmware_id ON firmware_status_history(firmware_version_id);
CREATE INDEX idx_firmware_status_history_changed_at ON firmware_status_history(changed_at);