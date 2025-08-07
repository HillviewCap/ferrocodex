use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;
use tracing::info;
use crate::users::SqliteUserRepository;
use crate::user_settings::SqliteUserSettingsRepository;
use crate::audit::{SqliteAuditRepository, AuditRepository};
use crate::assets::SqliteAssetRepository;
use crate::configurations::SqliteConfigurationRepository;
use crate::branches::SqliteBranchRepository;
use crate::firmware::SqliteFirmwareRepository;
use crate::firmware_analysis::{SqliteFirmwareAnalysisRepository, FirmwareAnalysisRepository};
use crate::vault::{SqliteVaultRepository, VaultRepository};
// Epic 5 imports
use crate::metadata::{SqliteMetadataRepository, SqliteMetadataSearchRepository};
use crate::bulk::{SqliteBulkImportRepository, operations::SqliteBulkOperationsRepository};

pub struct Database {
    conn: Connection,
}

// Type alias for compatibility with workflow module
pub type DatabaseManager = Database;
pub type DatabaseError = anyhow::Error;

impl Database {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        info!("Initializing database at: {:?}", db_path);
        
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;

        let db = Database { conn };
        db.initialize_schema()?;
        
        info!("Database initialized successfully");
        Ok(db)
    }

    fn initialize_schema(&self) -> Result<()> {
        info!("Setting up database schema");
        
        self.conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 268435456; -- 256MB
            
            CREATE TABLE IF NOT EXISTS app_metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            INSERT OR IGNORE INTO app_metadata (key, value) VALUES 
                ('db_version', '1.0'),
                ('created_at', datetime('now'));
            "#,
        )?;

        // Initialize user schema
        let user_repo = SqliteUserRepository::new(&self.conn);
        user_repo.initialize_schema()?;

        // Initialize user settings schema
        let user_settings_repo = SqliteUserSettingsRepository::new(&self.conn);
        user_settings_repo.initialize_schema()?;

        // Initialize audit schema
        let audit_repo = SqliteAuditRepository::new(&self.conn);
        audit_repo.initialize_schema()?;

        // Initialize asset schema
        let asset_repo = SqliteAssetRepository::new(&self.conn);
        asset_repo.initialize_schema()?;

        // Initialize configuration schema
        let config_repo = SqliteConfigurationRepository::new(&self.conn);
        config_repo.initialize_schema()?;

        // Initialize branches schema
        let branch_repo = SqliteBranchRepository::new(&self.conn);
        branch_repo.initialize_schema()?;

        // Initialize firmware schema
        let firmware_repo = SqliteFirmwareRepository::new(&self.conn);
        firmware_repo.initialize_schema()?;

        // Initialize firmware analysis schema
        let firmware_analysis_repo = SqliteFirmwareAnalysisRepository::new(&self.conn);
        firmware_analysis_repo.initialize_schema()?;

        // Initialize vault schema
        let vault_repo = SqliteVaultRepository::new(&self.conn);
        vault_repo.initialize_schema()?;

        // Epic 5 - Initialize metadata schema
        let metadata_repo = SqliteMetadataRepository::new(&self.conn);
        metadata_repo.initialize_schema()?;
        metadata_repo.populate_system_templates()?;
        metadata_repo.create_default_schema()?;

        // Epic 5 - Initialize metadata search schema
        let metadata_search_repo = SqliteMetadataSearchRepository::new(&self.conn);
        metadata_search_repo.initialize_search_schema()?;

        // Epic 5 - Initialize bulk import schema
        let bulk_repo = SqliteBulkImportRepository::new(&self.conn);
        bulk_repo.initialize_schema()?;

        // Epic 5 - Initialize bulk operations schema
        let bulk_ops_repo = SqliteBulkOperationsRepository::new(&self.conn);
        bulk_ops_repo.initialize_schema()?;

        // Run data migrations
        self.run_data_migrations()?;

        info!("Database schema initialized");
        Ok(())
    }

    pub fn health_check(&self) -> Result<bool> {
        let mut stmt = self.conn.prepare("SELECT 1")?;
        let result: i32 = stmt.query_row([], |row| row.get(0))?;
        Ok(result == 1)
    }

    pub fn get_metadata(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM app_metadata WHERE key = ?1")?;
        let result = stmt.query_row([key], |row| row.get(0));
        
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_metadata(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO app_metadata (key, value, updated_at) VALUES (?1, ?2, datetime('now'))",
            [key, value],
        )?;
        Ok(())
    }

    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }

    fn run_data_migrations(&self) -> Result<()> {
        info!("Running data migrations");
        
        // Check if firmware metadata migration has been run
        let migration_key = "firmware_metadata_migration_20250125";
        if let Ok(Some(_)) = self.get_metadata(migration_key) {
            info!("Firmware metadata migration already applied");
            return Ok(());
        }

        // Fix firmware records with invalid metadata
        info!("Fixing firmware metadata issues");
        
        self.conn.execute_batch(r#"
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
        "#)?;

        // Mark migration as complete
        self.set_metadata(migration_key, "applied")?;
        
        info!("Firmware metadata migration completed");
        
        // Story 4.6 - Password rotation migration
        let rotation_migration_key = "password_rotation_migration_20250126";
        if let Ok(None) = self.get_metadata(rotation_migration_key) {
            info!("Applying password rotation migration");
            
            // Check if columns already exist before adding them
            let column_check: Result<i32, rusqlite::Error> = self.conn.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('vault_secrets') WHERE name = 'last_rotated'",
                [],
                |row| row.get(0),
            );
            
            if let Ok(0) = column_check {
                // Add rotation columns to existing vault_secrets table
                self.conn.execute_batch(r#"
                    -- Add rotation management columns to vault_secrets
                    ALTER TABLE vault_secrets ADD COLUMN last_rotated DATETIME;
                    ALTER TABLE vault_secrets ADD COLUMN rotation_interval_days INTEGER;
                    ALTER TABLE vault_secrets ADD COLUMN next_rotation_due DATETIME;
                    ALTER TABLE vault_secrets ADD COLUMN rotation_policy_id INTEGER;
                "#)?;
                
                info!("Added rotation columns to vault_secrets table");
            }
            
            // Mark migration as complete
            self.set_metadata(rotation_migration_key, "applied")?;
            info!("Password rotation migration completed");
        }
        
        // Epic 5 - Metadata archive columns migration
        let metadata_archive_migration_key = "metadata_archive_columns_20250206";
        if let Ok(None) = self.get_metadata(metadata_archive_migration_key) {
            info!("Applying metadata archive columns migration");
            
            // Check if is_archived column already exists
            let column_check: Result<i32, rusqlite::Error> = self.conn.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('asset_metadata_schemas') WHERE name = 'is_archived'",
                [],
                |row| row.get(0),
            );
            
            if let Ok(0) = column_check {
                // Add archive columns to asset_metadata_schemas table
                self.conn.execute_batch(r#"
                    -- Add archive management columns to asset_metadata_schemas
                    ALTER TABLE asset_metadata_schemas ADD COLUMN is_archived BOOLEAN DEFAULT 0;
                    ALTER TABLE asset_metadata_schemas ADD COLUMN archived_at DATETIME;
                    ALTER TABLE asset_metadata_schemas ADD COLUMN archive_reason TEXT;
                    
                    -- Create index for the new is_archived column
                    CREATE INDEX IF NOT EXISTS idx_metadata_schemas_archived ON asset_metadata_schemas(is_archived);
                "#)?;
                
                info!("Added archive columns and index to asset_metadata_schemas table");
            } else {
                info!("Archive columns already exist in asset_metadata_schemas table");
            }
            
            // Mark migration as complete
            self.set_metadata(metadata_archive_migration_key, "applied")?;
            info!("Metadata archive columns migration completed");
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_database_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();
        
        let db = Database::new(db_path).unwrap();
        assert!(db.health_check().unwrap());
    }

    #[test]
    fn test_metadata_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();
        
        let db = Database::new(db_path).unwrap();
        
        // Test setting and getting metadata
        db.set_metadata("test_key", "test_value").unwrap();
        let value = db.get_metadata("test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));
        
        // Test getting non-existent key
        let empty_value = db.get_metadata("non_existent").unwrap();
        assert_eq!(empty_value, None);
    }
}