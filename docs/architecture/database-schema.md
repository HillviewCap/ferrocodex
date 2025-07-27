# Database Schema

## v0.3.0 Schema Updates

This schema adds the firmware_versions table and updates the configuration_versions table to support the hybrid storage model.

```sql
-- Existing tables remain unchanged
CREATE TABLE users (
    -- ... existing schema ...
);

CREATE TABLE assets (
    -- ... existing schema ...
);

CREATE TABLE branches (
    -- ... existing schema ...
);

CREATE TABLE configuration_versions (
    -- ... existing schema with addition below ...
);

-- New table for firmware management
CREATE TABLE firmware_versions (
    id TEXT PRIMARY KEY NOT NULL,
    asset_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    vendor TEXT,
    model TEXT,
    version TEXT NOT NULL,
    notes TEXT,
    status TEXT NOT NULL CHECK(status IN ('Draft', 'Golden', 'Archived')),
    file_path TEXT NOT NULL, -- Path to the encrypted file on the file system
    file_hash TEXT NOT NULL, -- SHA-256 hash of the encrypted file
    created_at TEXT NOT NULL,
    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
    FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE RESTRICT
);

-- Add column to link configurations to firmware
ALTER TABLE configuration_versions
ADD COLUMN firmware_version_id TEXT
REFERENCES firmware_versions(id) ON DELETE SET NULL;
```

_(The complete SQL DDL `CREATE TABLE` statements for all tables including indexes and constraints.)_