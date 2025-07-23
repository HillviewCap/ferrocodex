use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;
use tracing::info;
use crate::users::SqliteUserRepository;
use crate::audit::{SqliteAuditRepository, AuditRepository};
use crate::assets::SqliteAssetRepository;
use crate::configurations::SqliteConfigurationRepository;
use crate::branches::SqliteBranchRepository;
use crate::firmware::SqliteFirmwareRepository;
use crate::firmware_analysis::{SqliteFirmwareAnalysisRepository, FirmwareAnalysisRepository};

pub struct Database {
    conn: Connection,
}

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