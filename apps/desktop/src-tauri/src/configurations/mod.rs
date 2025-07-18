use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use crate::encryption::{FileEncryption, derive_key_from_user_credentials, validate_file_size, compress_data, decompress_data};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationVersion {
    pub id: i64,
    pub asset_id: i64,
    pub version_number: String,
    pub file_name: String,
    pub file_content: Vec<u8>, // Encrypted content
    pub file_size: i64,
    pub content_hash: String,
    pub author: i64,
    pub notes: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationVersionInfo {
    pub id: i64,
    pub asset_id: i64,
    pub version_number: String,
    pub file_name: String,
    pub file_size: i64,
    pub content_hash: String,
    pub author: i64,
    pub author_username: String,
    pub notes: String,
    pub created_at: String,
}

impl From<ConfigurationVersion> for ConfigurationVersionInfo {
    fn from(config: ConfigurationVersion) -> Self {
        ConfigurationVersionInfo {
            id: config.id,
            asset_id: config.asset_id,
            version_number: config.version_number,
            file_name: config.file_name,
            file_size: config.file_size,
            content_hash: config.content_hash,
            author: config.author,
            author_username: String::new(), // Will be populated by join query
            notes: config.notes,
            created_at: config.created_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateConfigurationRequest {
    pub asset_id: i64,
    pub file_name: String,
    pub file_content: Vec<u8>,
    pub author: i64,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub name: String,
    pub size: i64,
    pub content_type: String,
    pub hash: String,
}

pub trait ConfigurationRepository {
    fn store_configuration(&self, request: CreateConfigurationRequest) -> Result<ConfigurationVersion>;
    fn get_configuration_versions(&self, asset_id: i64) -> Result<Vec<ConfigurationVersionInfo>>;
    fn get_configuration_content(&self, version_id: i64) -> Result<Vec<u8>>;
    fn get_configuration_by_id(&self, version_id: i64) -> Result<Option<ConfigurationVersion>>;
    fn get_latest_version_number(&self, asset_id: i64) -> Result<Option<String>>;
    fn delete_configuration_version(&self, version_id: i64) -> Result<()>;
    fn get_configuration_count(&self, asset_id: i64) -> Result<i64>;
}

pub struct SqliteConfigurationRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteConfigurationRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS configuration_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                asset_id INTEGER NOT NULL,
                version_number TEXT NOT NULL,
                file_name TEXT NOT NULL,
                file_content BLOB NOT NULL,
                file_size INTEGER NOT NULL,
                content_hash TEXT NOT NULL,
                author INTEGER NOT NULL,
                notes TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
                FOREIGN KEY (author) REFERENCES users(id),
                UNIQUE(asset_id, version_number)
            );

            CREATE INDEX IF NOT EXISTS idx_config_asset_id ON configuration_versions(asset_id);
            CREATE INDEX IF NOT EXISTS idx_config_version ON configuration_versions(asset_id, version_number);
            CREATE INDEX IF NOT EXISTS idx_config_author ON configuration_versions(author);
            CREATE INDEX IF NOT EXISTS idx_config_created_at ON configuration_versions(created_at);
            "#,
        )?;
        Ok(())
    }

    fn row_to_configuration(row: &Row) -> rusqlite::Result<ConfigurationVersion> {
        Ok(ConfigurationVersion {
            id: row.get("id")?,
            asset_id: row.get("asset_id")?,
            version_number: row.get("version_number")?,
            file_name: row.get("file_name")?,
            file_content: row.get("file_content")?,
            file_size: row.get("file_size")?,
            content_hash: row.get("content_hash")?,
            author: row.get("author")?,
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
        })
    }

    fn row_to_configuration_info(row: &Row) -> rusqlite::Result<ConfigurationVersionInfo> {
        Ok(ConfigurationVersionInfo {
            id: row.get("id")?,
            asset_id: row.get("asset_id")?,
            version_number: row.get("version_number")?,
            file_name: row.get("file_name")?,
            file_size: row.get("file_size")?,
            content_hash: row.get("content_hash")?,
            author: row.get("author")?,
            author_username: row.get("author_username")?,
            notes: row.get("notes")?,
            created_at: row.get("created_at")?,
        })
    }

    fn generate_next_version_number(&self, asset_id: i64) -> Result<String> {
        let mut stmt = self.conn.prepare(
            "SELECT version_number FROM configuration_versions WHERE asset_id = ?1 ORDER BY created_at DESC LIMIT 1"
        )?;

        let result = stmt.query_row([asset_id], |row| row.get::<_, String>(0));
        
        match result {
            Ok(last_version) => {
                // Parse version number (e.g., "v1" -> 1, "v2" -> 2)
                if let Some(version_num) = last_version.strip_prefix('v') {
                    if let Ok(num) = version_num.parse::<i32>() {
                        return Ok(format!("v{}", num + 1));
                    }
                }
                // If parsing fails, default to v2
                Ok("v2".to_string())
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok("v1".to_string()),
            Err(e) => Err(e.into()),
        }
    }

    fn calculate_content_hash(&self, content: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

impl<'a> ConfigurationRepository for SqliteConfigurationRepository<'a> {
    fn store_configuration(&self, request: CreateConfigurationRequest) -> Result<ConfigurationVersion> {
        // Validate input
        if request.file_name.trim().is_empty() {
            return Err(anyhow::anyhow!("File name cannot be empty"));
        }
        if request.file_content.is_empty() {
            return Err(anyhow::anyhow!("File content cannot be empty"));
        }
        
        // Validate file size (100MB limit)
        validate_file_size(&request.file_content, 100 * 1024 * 1024)?;

        // Generate version number
        let version_number = self.generate_next_version_number(request.asset_id)?;
        
        // Calculate content hash before encryption
        let content_hash = self.calculate_content_hash(&request.file_content);
        
        // Compress data if beneficial
        let compressed_data = match compress_data(&request.file_content) {
            Ok(compressed) if compressed.len() < request.file_content.len() => {
                tracing::info!("Compressed file from {} to {} bytes", request.file_content.len(), compressed.len());
                compressed
            }
            _ => request.file_content.clone(),
        };
        
        // Encrypt the file content
        let encryption_key = derive_key_from_user_credentials(request.author, "ferrocodex");
        let encryption = FileEncryption::new(&encryption_key);
        let encrypted_content = encryption.encrypt(&compressed_data)?;
        
        // Store configuration
        let mut stmt = self.conn.prepare(
            "INSERT INTO configuration_versions (asset_id, version_number, file_name, file_content, file_size, content_hash, author, notes) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) RETURNING *"
        )?;

        let config = stmt.query_row(
            (
                &request.asset_id,
                &version_number,
                &request.file_name,
                &encrypted_content,
                &(request.file_content.len() as i64), // Store original size
                &content_hash,
                &request.author,
                &request.notes,
            ),
            Self::row_to_configuration,
        )?;

        Ok(config)
    }

    fn get_configuration_versions(&self, asset_id: i64) -> Result<Vec<ConfigurationVersionInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT cv.id, cv.asset_id, cv.version_number, cv.file_name, cv.file_size, 
                    cv.content_hash, cv.author, u.username as author_username, cv.notes, cv.created_at
             FROM configuration_versions cv
             JOIN users u ON cv.author = u.id
             WHERE cv.asset_id = ?1
             ORDER BY cv.created_at DESC"
        )?;

        let config_iter = stmt.query_map([asset_id], Self::row_to_configuration_info)?;
        let mut configurations = Vec::new();

        for config in config_iter {
            configurations.push(config?);
        }

        Ok(configurations)
    }

    fn get_configuration_content(&self, version_id: i64) -> Result<Vec<u8>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_content, author FROM configuration_versions WHERE id = ?1"
        )?;

        let (encrypted_content, author): (Vec<u8>, i64) = stmt.query_row([version_id], |row| {
            Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, i64>(1)?))
        })?;

        // Decrypt the content
        let encryption_key = derive_key_from_user_credentials(author, "ferrocodex");
        let encryption = FileEncryption::new(&encryption_key);
        let compressed_data = encryption.decrypt(&encrypted_content)?;
        
        // Try to decompress, fallback to original if decompression fails
        let content = match decompress_data(&compressed_data) {
            Ok(decompressed) => decompressed,
            Err(_) => compressed_data, // Wasn't compressed
        };
        
        Ok(content)
    }

    fn get_configuration_by_id(&self, version_id: i64) -> Result<Option<ConfigurationVersion>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, asset_id, version_number, file_name, file_content, file_size, 
                    content_hash, author, notes, created_at
             FROM configuration_versions WHERE id = ?1"
        )?;

        let result = stmt.query_row([version_id], Self::row_to_configuration);
        
        match result {
            Ok(config) => Ok(Some(config)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn get_latest_version_number(&self, asset_id: i64) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT version_number FROM configuration_versions WHERE asset_id = ?1 ORDER BY created_at DESC LIMIT 1"
        )?;

        let result = stmt.query_row([asset_id], |row| row.get::<_, String>(0));
        
        match result {
            Ok(version) => Ok(Some(version)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn delete_configuration_version(&self, version_id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM configuration_versions WHERE id = ?1",
            [version_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Configuration version not found"));
        }

        Ok(())
    }

    fn get_configuration_count(&self, asset_id: i64) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM configuration_versions WHERE asset_id = ?1"
        )?;
        let count: i64 = stmt.query_row([asset_id], |row| row.get(0))?;
        Ok(count)
    }
}

// File handling utilities
pub mod file_utils {
    use super::*;
    use std::fs;
    use std::path::Path;

    pub fn read_file_content(file_path: &str) -> Result<Vec<u8>> {
        let path = Path::new(file_path);
        
        if !path.exists() {
            return Err(anyhow::anyhow!("File does not exist: {}", file_path));
        }

        let content = fs::read(path)?;
        
        // Validate file size
        if content.len() > 100 * 1024 * 1024 { // 100MB limit
            return Err(anyhow::anyhow!("File size exceeds maximum limit of 100MB"));
        }

        Ok(content)
    }

    pub fn get_file_metadata(file_path: &str) -> Result<FileMetadata> {
        let path = Path::new(file_path);
        
        if !path.exists() {
            return Err(anyhow::anyhow!("File does not exist: {}", file_path));
        }

        let metadata = fs::metadata(path)?;
        let content = fs::read(path)?;
        
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let content_type = detect_content_type(&content, &name);
        let hash = calculate_file_hash(&content);

        Ok(FileMetadata {
            name,
            size: metadata.len() as i64,
            content_type,
            hash,
        })
    }

    pub fn validate_file_type(file_path: &str) -> Result<bool> {
        let path = Path::new(file_path);
        
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            let allowed_extensions = vec![
                "json", "xml", "yaml", "yml", "txt", "cfg", "conf", "ini",
                "csv", "log", "properties", "config", "settings", "toml",
                "bin", "dat", "hex", "raw", "dump"
            ];
            
            if allowed_extensions.contains(&extension.to_lowercase().as_str()) {
                return Ok(true);
            }
        }

        // Also check file content for text files
        if let Ok(content) = fs::read(path) {
            if content.len() > 1024 * 1024 { // 1MB sample for content detection
                return Ok(true); // Large files are likely binary and valid
            }
            
            // Check if content is mostly text
            let text_chars = content.iter().filter(|&&b| b.is_ascii_graphic() || b.is_ascii_whitespace()).count();
            let text_ratio = text_chars as f64 / content.len() as f64;
            
            if text_ratio > 0.7 { // 70% text characters
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn detect_content_type(content: &[u8], filename: &str) -> String {
        // Basic content type detection
        if let Some(extension) = Path::new(filename).extension().and_then(|ext| ext.to_str()) {
            match extension.to_lowercase().as_str() {
                "json" => return "application/json".to_string(),
                "xml" => return "application/xml".to_string(),
                "yaml" | "yml" => return "application/yaml".to_string(),
                "txt" => return "text/plain".to_string(),
                "csv" => return "text/csv".to_string(),
                "bin" | "dat" => return "application/octet-stream".to_string(),
                _ => {}
            }
        }

        // Check content for text/binary determination
        if content.len() > 0 {
            let text_chars = content.iter().filter(|&&b| b.is_ascii_graphic() || b.is_ascii_whitespace()).count();
            let text_ratio = text_chars as f64 / content.len() as f64;
            
            if text_ratio > 0.7 {
                "text/plain".to_string()
            } else {
                "application/octet-stream".to_string()
            }
        } else {
            "application/octet-stream".to_string()
        }
    }

    fn calculate_file_hash(content: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use rusqlite::Connection;

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        
        // Create required tables
        conn.execute_batch(
            r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                is_active BOOLEAN DEFAULT 1
            );
            
            CREATE TABLE assets (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES users(id)
            );
            
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'testuser', 'hash', 'Engineer');
            INSERT INTO assets (id, name, description, created_by) VALUES (1, 'Test Asset', 'Test Description', 1);
            "#,
        ).unwrap();
        
        let repo = SqliteConfigurationRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_configuration_storage() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteConfigurationRepository::new(&conn);

        let request = CreateConfigurationRequest {
            asset_id: 1,
            file_name: "config.json".to_string(),
            file_content: b"{\"test\": \"value\"}".to_vec(),
            author: 1,
            notes: "Initial configuration".to_string(),
        };

        let config = repo.store_configuration(request).unwrap();
        assert_eq!(config.asset_id, 1);
        assert_eq!(config.file_name, "config.json");
        assert_eq!(config.version_number, "v1");
        assert_eq!(config.author, 1);
        assert_eq!(config.notes, "Initial configuration");
        assert!(!config.content_hash.is_empty());
    }

    #[test]
    fn test_version_number_generation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteConfigurationRepository::new(&conn);

        // First version should be v1
        let request1 = CreateConfigurationRequest {
            asset_id: 1,
            file_name: "config1.json".to_string(),
            file_content: b"{\"test\": \"value1\"}".to_vec(),
            author: 1,
            notes: "Version 1".to_string(),
        };

        let config1 = repo.store_configuration(request1).unwrap();
        assert_eq!(config1.version_number, "v1");

        // Second version should be v2
        let request2 = CreateConfigurationRequest {
            asset_id: 1,
            file_name: "config2.json".to_string(),
            file_content: b"{\"test\": \"value2\"}".to_vec(),
            author: 1,
            notes: "Version 2".to_string(),
        };

        let config2 = repo.store_configuration(request2).unwrap();
        assert_eq!(config2.version_number, "v2");
    }

    #[test]
    fn test_file_validation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteConfigurationRepository::new(&conn);

        // Test empty file name
        let request = CreateConfigurationRequest {
            asset_id: 1,
            file_name: "".to_string(),
            file_content: b"content".to_vec(),
            author: 1,
            notes: "Test".to_string(),
        };
        assert!(repo.store_configuration(request).is_err());

        // Test empty content
        let request = CreateConfigurationRequest {
            asset_id: 1,
            file_name: "config.json".to_string(),
            file_content: Vec::new(),
            author: 1,
            notes: "Test".to_string(),
        };
        assert!(repo.store_configuration(request).is_err());
    }

    #[test]
    fn test_get_configuration_versions() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteConfigurationRepository::new(&conn);

        // Create multiple versions
        let request1 = CreateConfigurationRequest {
            asset_id: 1,
            file_name: "config1.json".to_string(),
            file_content: b"{\"test\": \"value1\"}".to_vec(),
            author: 1,
            notes: "Version 1".to_string(),
        };

        let request2 = CreateConfigurationRequest {
            asset_id: 1,
            file_name: "config2.json".to_string(),
            file_content: b"{\"test\": \"value2\"}".to_vec(),
            author: 1,
            notes: "Version 2".to_string(),
        };

        repo.store_configuration(request1).unwrap();
        repo.store_configuration(request2).unwrap();

        let versions = repo.get_configuration_versions(1).unwrap();
        assert_eq!(versions.len(), 2);
        // Check that we have both versions (order depends on creation time)
        let version_numbers: Vec<String> = versions.iter().map(|v| v.version_number.clone()).collect();
        assert!(version_numbers.contains(&"v1".to_string()));
        assert!(version_numbers.contains(&"v2".to_string()));
    }

    #[test]
    fn test_get_configuration_content() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteConfigurationRepository::new(&conn);

        let content = b"{\"test\": \"value\"}";
        let request = CreateConfigurationRequest {
            asset_id: 1,
            file_name: "config.json".to_string(),
            file_content: content.to_vec(),
            author: 1,
            notes: "Test".to_string(),
        };

        let config = repo.store_configuration(request).unwrap();
        let retrieved_content = repo.get_configuration_content(config.id).unwrap();
        assert_eq!(retrieved_content, content);
    }

    #[test]
    fn test_get_configuration_count() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteConfigurationRepository::new(&conn);

        assert_eq!(repo.get_configuration_count(1).unwrap(), 0);

        let request = CreateConfigurationRequest {
            asset_id: 1,
            file_name: "config.json".to_string(),
            file_content: b"{\"test\": \"value\"}".to_vec(),
            author: 1,
            notes: "Test".to_string(),
        };

        repo.store_configuration(request).unwrap();
        assert_eq!(repo.get_configuration_count(1).unwrap(), 1);
    }

    #[test]
    fn test_delete_configuration_version() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteConfigurationRepository::new(&conn);

        let request = CreateConfigurationRequest {
            asset_id: 1,
            file_name: "config.json".to_string(),
            file_content: b"{\"test\": \"value\"}".to_vec(),
            author: 1,
            notes: "Test".to_string(),
        };

        let config = repo.store_configuration(request).unwrap();
        repo.delete_configuration_version(config.id).unwrap();

        let result = repo.get_configuration_by_id(config.id).unwrap();
        assert!(result.is_none());
    }
}