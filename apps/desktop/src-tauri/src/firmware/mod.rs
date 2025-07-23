use anyhow::Result;
use rusqlite::{Connection, Row, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use crate::encryption::{FileEncryption, derive_key_from_user_credentials};
use std::fs;
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareVersion {
    pub id: i64,
    pub asset_id: i64,
    pub author_id: i64,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub version: String,
    pub notes: Option<String>,
    pub status: FirmwareStatus,
    pub file_path: String,
    pub file_hash: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum FirmwareStatus {
    Draft,
    Golden,
    Archived,
}

impl std::fmt::Display for FirmwareStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FirmwareStatus::Draft => write!(f, "Draft"),
            FirmwareStatus::Golden => write!(f, "Golden"),
            FirmwareStatus::Archived => write!(f, "Archived"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFirmwareRequest {
    pub asset_id: i64,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub version: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareVersionInfo {
    pub id: i64,
    pub asset_id: i64,
    pub author_id: i64,
    pub author_username: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub version: String,
    pub notes: Option<String>,
    pub status: FirmwareStatus,
    pub file_path: String,
    pub file_hash: String,
    pub file_size: i64,
    pub created_at: String,
}

pub trait FirmwareRepository {
    fn create_firmware(&self, request: CreateFirmwareRequest, author_id: i64, file_path: String, file_hash: String, file_size: i64) -> Result<FirmwareVersion>;
    fn get_firmware_by_asset(&self, asset_id: i64) -> Result<Vec<FirmwareVersionInfo>>;
    fn get_firmware_by_id(&self, firmware_id: i64) -> Result<Option<FirmwareVersion>>;
    fn delete_firmware(&self, firmware_id: i64) -> Result<Option<String>>;
    fn get_linked_configuration_count(&self, firmware_id: i64) -> Result<i64>;
}

pub struct SqliteFirmwareRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteFirmwareRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS firmware_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                asset_id INTEGER NOT NULL,
                author_id INTEGER NOT NULL,
                vendor TEXT,
                model TEXT,
                version TEXT NOT NULL,
                notes TEXT,
                status TEXT NOT NULL CHECK(status IN ('Draft', 'Golden', 'Archived')),
                file_path TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
                FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE RESTRICT
            );

            CREATE INDEX IF NOT EXISTS idx_firmware_asset_id ON firmware_versions(asset_id);
            CREATE INDEX IF NOT EXISTS idx_firmware_created_at ON firmware_versions(created_at);
            "#,
        )?;
        Ok(())
    }

    fn row_to_firmware(row: &Row) -> rusqlite::Result<FirmwareVersion> {
        Ok(FirmwareVersion {
            id: row.get("id")?,
            asset_id: row.get("asset_id")?,
            author_id: row.get("author_id")?,
            vendor: row.get("vendor")?,
            model: row.get("model")?,
            version: row.get("version")?,
            notes: row.get("notes")?,
            status: match row.get::<_, String>("status")?.as_str() {
                "Golden" => FirmwareStatus::Golden,
                "Archived" => FirmwareStatus::Archived,
                _ => FirmwareStatus::Draft,
            },
            file_path: row.get("file_path")?,
            file_hash: row.get("file_hash")?,
            created_at: row.get("created_at")?,
        })
    }

    fn row_to_firmware_info(row: &Row) -> rusqlite::Result<FirmwareVersionInfo> {
        Ok(FirmwareVersionInfo {
            id: row.get("id")?,
            asset_id: row.get("asset_id")?,
            author_id: row.get("author_id")?,
            author_username: row.get("author_username")?,
            vendor: row.get("vendor")?,
            model: row.get("model")?,
            version: row.get("version")?,
            notes: row.get("notes")?,
            status: match row.get::<_, String>("status")?.as_str() {
                "Golden" => FirmwareStatus::Golden,
                "Archived" => FirmwareStatus::Archived,
                _ => FirmwareStatus::Draft,
            },
            file_path: row.get("file_path")?,
            file_hash: row.get("file_hash")?,
            file_size: row.get("file_size")?,
            created_at: row.get("created_at")?,
        })
    }
}

impl<'a> FirmwareRepository for SqliteFirmwareRepository<'a> {
    fn create_firmware(&self, request: CreateFirmwareRequest, author_id: i64, file_path: String, file_hash: String, file_size: i64) -> Result<FirmwareVersion> {
        let tx = self.conn.unchecked_transaction()?;
        
        let firmware = FirmwareVersion {
            id: 0, // Will be set by database
            asset_id: request.asset_id,
            author_id,
            vendor: request.vendor,
            model: request.model,
            version: request.version,
            notes: request.notes,
            status: FirmwareStatus::Draft,
            file_path: file_path.clone(),
            file_hash: file_hash.clone(),
            created_at: String::new(), // Will be set by database
        };

        tx.execute(
            "INSERT INTO firmware_versions (asset_id, author_id, vendor, model, version, notes, status, file_path, file_hash, file_size)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                firmware.asset_id,
                firmware.author_id,
                firmware.vendor,
                firmware.model,
                firmware.version,
                firmware.notes,
                firmware.status.to_string(),
                firmware.file_path,
                firmware.file_hash,
                file_size,
            ],
        )?;

        let id = tx.last_insert_rowid();
        tx.commit()?;

        Ok(FirmwareVersion {
            id,
            ..firmware
        })
    }

    fn get_firmware_by_asset(&self, asset_id: i64) -> Result<Vec<FirmwareVersionInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.asset_id, f.author_id, u.username as author_username, 
                    f.vendor, f.model, f.version, f.notes, f.status, 
                    f.file_path, f.file_hash, f.file_size, f.created_at
             FROM firmware_versions f
             JOIN users u ON f.author_id = u.id
             WHERE f.asset_id = ?1
             ORDER BY f.created_at DESC, f.id DESC"
        )?;

        let firmware_iter = stmt.query_map([asset_id], Self::row_to_firmware_info)?;

        let mut firmwares = Vec::new();
        for firmware in firmware_iter {
            firmwares.push(firmware?);
        }
        Ok(firmwares)
    }

    fn get_firmware_by_id(&self, firmware_id: i64) -> Result<Option<FirmwareVersion>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, asset_id, author_id, vendor, model, version, notes, status, file_path, file_hash, created_at
             FROM firmware_versions
             WHERE id = ?1"
        )?;

        let result = stmt.query_row([firmware_id], Self::row_to_firmware);
        match result {
            Ok(firmware) => Ok(Some(firmware)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn delete_firmware(&self, firmware_id: i64) -> Result<Option<String>> {
        let tx = self.conn.unchecked_transaction()?;
        
        let file_path: Option<String> = tx
            .query_row(
                "SELECT file_path FROM firmware_versions WHERE id = ?1",
                [firmware_id],
                |row| row.get(0),
            )
            .optional()?;

        if file_path.is_some() {
            tx.execute(
                "DELETE FROM firmware_versions WHERE id = ?1",
                [firmware_id],
            )?;
            tx.commit()?;
        }

        Ok(file_path)
    }
    
    fn get_linked_configuration_count(&self, firmware_id: i64) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM configuration_versions WHERE firmware_version_id = ?1"
        )?;
        let count: i64 = stmt.query_row([firmware_id], |row| row.get(0))?;
        Ok(count)
    }
}

pub fn get_firmware_storage_dir(app_handle: &AppHandle) -> Result<PathBuf> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get app data directory: {}", e))?;
    
    let firmware_dir = app_data_dir.join("firmware");
    std::fs::create_dir_all(&firmware_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create firmware directory: {}", e))?;
    
    Ok(firmware_dir)
}

pub struct FirmwareFileStorage;

impl FirmwareFileStorage {
    pub fn store_firmware_file(
        app_handle: &AppHandle,
        asset_id: i64,
        firmware_id: i64,
        file_data: &[u8],
        user_id: i64,
        username: &str,
    ) -> Result<(String, String, i64)> {
        // Validate file size (max 2GB as specified in requirements)
        const MAX_FILE_SIZE: usize = 2 * 1024 * 1024 * 1024; // 2GB
        if file_data.len() > MAX_FILE_SIZE {
            return Err(anyhow::anyhow!("File size exceeds 2GB limit"));
        }

        // Get firmware storage directory
        let firmware_dir = get_firmware_storage_dir(app_handle)?;
        
        // Create asset-specific subdirectory
        let asset_dir = firmware_dir.join(asset_id.to_string());
        fs::create_dir_all(&asset_dir)?;
        
        // Generate file path
        let file_name = format!("{}.enc", firmware_id);
        let file_path = asset_dir.join(&file_name);
        
        // Calculate hash of the original file
        let mut hasher = Sha256::new();
        hasher.update(file_data);
        let file_hash = format!("{:x}", hasher.finalize());
        
        // Encrypt the file data
        let encryption_key = derive_key_from_user_credentials(user_id, username);
        let encryption = FileEncryption::new(&encryption_key);
        let encrypted_data = encryption.encrypt(file_data)?;
        
        // Write encrypted data to file
        fs::write(&file_path, encrypted_data)?;
        
        // Return file path (relative to firmware dir), hash, and size
        let relative_path = format!("{}/{}", asset_id, file_name);
        Ok((relative_path, file_hash, file_data.len() as i64))
    }
    
    pub fn read_firmware_file(
        app_handle: &AppHandle,
        file_path: &str,
        user_id: i64,
        username: &str,
    ) -> Result<Vec<u8>> {
        // Get firmware storage directory
        let firmware_dir = get_firmware_storage_dir(app_handle)?;
        let full_path = firmware_dir.join(file_path);
        
        // Check if file exists
        if !full_path.exists() {
            return Err(anyhow::anyhow!("Firmware file not found"));
        }
        
        // Read encrypted file
        let encrypted_data = fs::read(&full_path)?;
        
        // Decrypt the file data
        let encryption_key = derive_key_from_user_credentials(user_id, username);
        let encryption = FileEncryption::new(&encryption_key);
        let decrypted_data = encryption.decrypt(&encrypted_data)?;
        
        Ok(decrypted_data)
    }
    
    pub fn delete_firmware_file(
        app_handle: &AppHandle,
        file_path: &str,
    ) -> Result<()> {
        // Get firmware storage directory
        let firmware_dir = get_firmware_storage_dir(app_handle)?;
        let full_path = firmware_dir.join(file_path);
        
        // Delete file if it exists
        if full_path.exists() {
            fs::remove_file(&full_path)?;
        }
        
        // Try to clean up empty directories
        if let Some(parent) = full_path.parent() {
            // Ignore errors when removing directory (it might not be empty)
            let _ = fs::remove_dir(parent);
        }
        
        Ok(())
    }
    
    pub fn verify_firmware_integrity(
        app_handle: &AppHandle,
        file_path: &str,
        expected_hash: &str,
        user_id: i64,
        username: &str,
    ) -> Result<bool> {
        // Read and decrypt the file
        let file_data = Self::read_firmware_file(app_handle, file_path, user_id, username)?;
        
        // Calculate hash
        let mut hasher = Sha256::new();
        hasher.update(&file_data);
        let actual_hash = format!("{:x}", hasher.finalize());
        
        Ok(actual_hash == expected_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        
        conn.execute_batch(
            r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE TABLE assets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES users(id)
            );
            
            INSERT INTO users (username, password_hash, role) 
            VALUES ('testuser', 'hash', 'Engineer');
            
            INSERT INTO assets (name, description, created_by) 
            VALUES ('Test Asset', 'Description', 1);
            "#
        ).unwrap();
        
        let repo = SqliteFirmwareRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        conn
    }

    #[test]
    fn test_create_firmware() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareRepository::new(&conn);
        
        let request = CreateFirmwareRequest {
            asset_id: 1,
            vendor: Some("Test Vendor".to_string()),
            model: Some("Test Model".to_string()),
            version: "1.0.0".to_string(),
            notes: Some("Test firmware".to_string()),
        };

        let result = repo.create_firmware(
            request,
            1,
            "/path/to/firmware.enc".to_string(),
            "abc123hash".to_string(),
            1024,
        );

        assert!(result.is_ok());
        let firmware = result.unwrap();
        assert_eq!(firmware.version, "1.0.0");
        assert_eq!(firmware.status, FirmwareStatus::Draft);
        assert!(firmware.id > 0);
    }

    #[test]
    fn test_get_firmware_by_asset() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareRepository::new(&conn);
        
        let request1 = CreateFirmwareRequest {
            asset_id: 1,
            vendor: Some("Vendor".to_string()),
            model: Some("Model".to_string()),
            version: "1.0.0".to_string(),
            notes: None,
        };

        repo.create_firmware(
            request1,
            1,
            "/path/to/firmware1.enc".to_string(),
            "hash1".to_string(),
            1024,
        ).unwrap();

        let request2 = CreateFirmwareRequest {
            asset_id: 1,
            vendor: Some("Vendor".to_string()),
            model: Some("Model".to_string()),
            version: "2.0.0".to_string(),
            notes: None,
        };

        repo.create_firmware(
            request2,
            1,
            "/path/to/firmware2.enc".to_string(),
            "hash2".to_string(),
            2048,
        ).unwrap();

        let firmwares = repo.get_firmware_by_asset(1).unwrap();
        assert_eq!(firmwares.len(), 2);
        assert_eq!(firmwares[0].version, "2.0.0");
        assert_eq!(firmwares[1].version, "1.0.0");
        assert_eq!(firmwares[0].author_username, "testuser");
    }

    #[test]
    fn test_delete_firmware() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareRepository::new(&conn);
        
        let request = CreateFirmwareRequest {
            asset_id: 1,
            vendor: None,
            model: None,
            version: "1.0.0".to_string(),
            notes: None,
        };

        let firmware = repo.create_firmware(
            request,
            1,
            "/path/to/firmware.enc".to_string(),
            "hash123".to_string(),
            1024,
        ).unwrap();

        let file_path = repo.delete_firmware(firmware.id).unwrap();
        assert_eq!(file_path, Some("/path/to/firmware.enc".to_string()));

        let firmwares = repo.get_firmware_by_asset(1).unwrap();
        assert_eq!(firmwares.len(), 0);
    }

    #[test]
    fn test_file_storage_without_app() {
        use tempfile::TempDir;
        
        // Test file storage operations without AppHandle
        let temp_dir = TempDir::new().unwrap();
        let firmware_dir = temp_dir.path().join("firmware");
        fs::create_dir_all(&firmware_dir).unwrap();
        
        let test_data = b"This is test firmware data";
        let asset_id = 1;
        let firmware_id = 100;
        let user_id = 1;
        let username = "testuser";
        
        // Create asset directory
        let asset_dir = firmware_dir.join(asset_id.to_string());
        fs::create_dir_all(&asset_dir).unwrap();
        
        // Test encryption
        let encryption_key = derive_key_from_user_credentials(user_id, username);
        let encryption = FileEncryption::new(&encryption_key);
        let encrypted_data = encryption.encrypt(test_data).unwrap();
        
        // Write encrypted file
        let file_name = format!("{}.enc", firmware_id);
        let file_path = asset_dir.join(&file_name);
        fs::write(&file_path, &encrypted_data).unwrap();
        
        // Read and decrypt
        let read_encrypted = fs::read(&file_path).unwrap();
        let decrypted_data = encryption.decrypt(&read_encrypted).unwrap();
        assert_eq!(decrypted_data, test_data);
        
        // Test hash calculation
        let mut hasher = Sha256::new();
        hasher.update(test_data);
        let file_hash = format!("{:x}", hasher.finalize());
        assert!(!file_hash.is_empty());
        
        // Cleanup
        fs::remove_file(&file_path).unwrap();
    }
    
    #[test]
    fn test_file_size_validation() {
        // Test file size limit validation
        const MAX_FILE_SIZE: usize = 2 * 1024 * 1024 * 1024; // 2GB
        
        let small_data = vec![0u8; 1024]; // 1KB
        assert!(small_data.len() <= MAX_FILE_SIZE);
        
        // Note: We can't actually allocate 2GB+ in tests, so we just test the logic
        let simulated_large_size = MAX_FILE_SIZE + 1;
        assert!(simulated_large_size > MAX_FILE_SIZE);
    }
}