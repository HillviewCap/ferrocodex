use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_by: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_by: i64,
    pub created_by_username: String,
    pub created_at: String,
    pub version_count: i64,
    pub latest_version: Option<String>,
    pub latest_version_notes: Option<String>,
}

impl From<Asset> for AssetInfo {
    fn from(asset: Asset) -> Self {
        AssetInfo {
            id: asset.id,
            name: asset.name,
            description: asset.description,
            created_by: asset.created_by,
            created_by_username: String::new(), // Will be populated from database
            created_at: asset.created_at,
            version_count: 0,
            latest_version: None,
            latest_version_notes: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateAssetRequest {
    pub name: String,
    pub description: String,
    pub created_by: i64,
}

pub trait AssetRepository {
    fn create_asset(&self, request: CreateAssetRequest) -> Result<Asset>;
    fn get_asset_by_id(&self, asset_id: i64) -> Result<Option<Asset>>;
    fn get_assets_by_user(&self, user_id: i64) -> Result<Vec<Asset>>;
    fn get_all_assets(&self) -> Result<Vec<Asset>>;
    fn get_assets_with_info(&self) -> Result<Vec<AssetInfo>>;
    fn update_asset(&self, asset: &Asset) -> Result<()>;
    fn delete_asset(&self, asset_id: i64) -> Result<()>;
    fn asset_exists_by_name(&self, name: &str) -> Result<bool>;
}

pub struct SqliteAssetRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteAssetRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS assets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES users(id)
            );

            CREATE INDEX IF NOT EXISTS idx_assets_name ON assets(name);
            CREATE INDEX IF NOT EXISTS idx_assets_created_by ON assets(created_by);
            CREATE INDEX IF NOT EXISTS idx_assets_created_at ON assets(created_at);
            "#,
        )?;
        Ok(())
    }

    fn row_to_asset(row: &Row) -> rusqlite::Result<Asset> {
        Ok(Asset {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    fn row_to_asset_info(row: &Row) -> rusqlite::Result<AssetInfo> {
        Ok(AssetInfo {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            created_by: row.get("created_by")?,
            created_by_username: row.get("created_by_username")?,
            created_at: row.get("created_at")?,
            version_count: row.get("version_count")?,
            latest_version: row.get("latest_version")?,
            latest_version_notes: row.get("latest_version_notes")?,
        })
    }
}

impl<'a> AssetRepository for SqliteAssetRepository<'a> {
    fn create_asset(&self, request: CreateAssetRequest) -> Result<Asset> {
        // Validate asset name
        if request.name.trim().is_empty() {
            return Err(anyhow::anyhow!("Asset name cannot be empty"));
        }
        if request.name.len() < 2 {
            return Err(anyhow::anyhow!("Asset name must be at least 2 characters long"));
        }
        if request.name.len() > 100 {
            return Err(anyhow::anyhow!("Asset name cannot exceed 100 characters"));
        }

        // Check for duplicate name
        if self.asset_exists_by_name(&request.name)? {
            return Err(anyhow::anyhow!("Asset with this name already exists"));
        }

        let mut stmt = self.conn.prepare(
            "INSERT INTO assets (name, description, created_by) VALUES (?1, ?2, ?3) RETURNING *"
        )?;

        let asset = stmt.query_row(
            (&request.name, &request.description, &request.created_by),
            Self::row_to_asset,
        )?;

        Ok(asset)
    }

    fn get_asset_by_id(&self, asset_id: i64) -> Result<Option<Asset>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, created_by, created_at, updated_at 
             FROM assets WHERE id = ?1"
        )?;

        let result = stmt.query_row([asset_id], Self::row_to_asset);
        
        match result {
            Ok(asset) => Ok(Some(asset)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn get_assets_by_user(&self, user_id: i64) -> Result<Vec<Asset>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, created_by, created_at, updated_at 
             FROM assets WHERE created_by = ?1 ORDER BY created_at DESC"
        )?;

        let asset_iter = stmt.query_map([user_id], Self::row_to_asset)?;
        let mut assets = Vec::new();

        for asset in asset_iter {
            assets.push(asset?);
        }

        Ok(assets)
    }

    fn get_all_assets(&self) -> Result<Vec<Asset>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, created_by, created_at, updated_at 
             FROM assets ORDER BY created_at DESC"
        )?;

        let asset_iter = stmt.query_map([], Self::row_to_asset)?;
        let mut assets = Vec::new();

        for asset in asset_iter {
            assets.push(asset?);
        }

        Ok(assets)
    }

    fn get_assets_with_info(&self) -> Result<Vec<AssetInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.name, a.description, a.created_by, a.created_at,
                    u.username as created_by_username,
                    COUNT(cv.id) as version_count,
                    MAX(cv.version_number) as latest_version,
                    latest_cv.notes as latest_version_notes
             FROM assets a
             INNER JOIN users u ON a.created_by = u.id
             LEFT JOIN configuration_versions cv ON a.id = cv.asset_id
             LEFT JOIN configuration_versions latest_cv ON a.id = latest_cv.asset_id 
                AND latest_cv.version_number = (SELECT MAX(version_number) FROM configuration_versions WHERE asset_id = a.id)
             GROUP BY a.id, a.name, a.description, a.created_by, a.created_at, u.username, latest_cv.notes
             ORDER BY a.created_at DESC"
        )?;

        let asset_iter = stmt.query_map([], Self::row_to_asset_info)?;
        let mut assets = Vec::new();

        for asset in asset_iter {
            assets.push(asset?);
        }

        Ok(assets)
    }

    fn update_asset(&self, asset: &Asset) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE assets SET name = ?1, description = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
            (&asset.name, &asset.description, &asset.id),
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Asset not found"));
        }

        Ok(())
    }

    fn delete_asset(&self, asset_id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM assets WHERE id = ?1",
            [asset_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Asset not found"));
        }

        Ok(())
    }

    fn asset_exists_by_name(&self, name: &str) -> Result<bool> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM assets WHERE name = ?1"
        )?;
        let count: i64 = stmt.query_row([name], |row| row.get(0))?;
        Ok(count > 0)
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
        
        // Create users table for foreign key constraint
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
            
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'testuser', 'hash', 'Engineer');
            "#,
        ).unwrap();
        
        let repo = SqliteAssetRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_asset_creation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let request = CreateAssetRequest {
            name: "PLC-Line5".to_string(),
            description: "Production Line 5 PLC Configuration".to_string(),
            created_by: 1,
        };

        let asset = repo.create_asset(request).unwrap();
        assert_eq!(asset.name, "PLC-Line5");
        assert_eq!(asset.description, "Production Line 5 PLC Configuration");
        assert_eq!(asset.created_by, 1);
        assert!(!asset.created_at.is_empty());
    }

    #[test]
    fn test_asset_name_validation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        // Test empty name
        let request = CreateAssetRequest {
            name: "".to_string(),
            description: "Test".to_string(),
            created_by: 1,
        };
        assert!(repo.create_asset(request).is_err());

        // Test short name
        let request = CreateAssetRequest {
            name: "A".to_string(),
            description: "Test".to_string(),
            created_by: 1,
        };
        assert!(repo.create_asset(request).is_err());

        // Test long name
        let request = CreateAssetRequest {
            name: "A".repeat(101),
            description: "Test".to_string(),
            created_by: 1,
        };
        assert!(repo.create_asset(request).is_err());
    }

    #[test]
    fn test_asset_name_uniqueness() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let request1 = CreateAssetRequest {
            name: "PLC-Line5".to_string(),
            description: "First PLC".to_string(),
            created_by: 1,
        };

        let request2 = CreateAssetRequest {
            name: "PLC-Line5".to_string(),
            description: "Second PLC".to_string(),
            created_by: 1,
        };

        repo.create_asset(request1).unwrap();
        assert!(repo.create_asset(request2).is_err());
    }

    #[test]
    fn test_get_asset_by_id() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let request = CreateAssetRequest {
            name: "Test Asset".to_string(),
            description: "Test Description".to_string(),
            created_by: 1,
        };

        let created_asset = repo.create_asset(request).unwrap();
        let retrieved_asset = repo.get_asset_by_id(created_asset.id).unwrap().unwrap();
        
        assert_eq!(created_asset.id, retrieved_asset.id);
        assert_eq!(created_asset.name, retrieved_asset.name);
    }

    #[test]
    fn test_get_assets_by_user() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        // Create multiple assets for user
        let request1 = CreateAssetRequest {
            name: "Asset1".to_string(),
            description: "Description1".to_string(),
            created_by: 1,
        };
        let request2 = CreateAssetRequest {
            name: "Asset2".to_string(),
            description: "Description2".to_string(),
            created_by: 1,
        };

        repo.create_asset(request1).unwrap();
        repo.create_asset(request2).unwrap();

        let assets = repo.get_assets_by_user(1).unwrap();
        assert_eq!(assets.len(), 2);
    }

    #[test]
    fn test_get_all_assets() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let request = CreateAssetRequest {
            name: "Test Asset".to_string(),
            description: "Test Description".to_string(),
            created_by: 1,
        };

        repo.create_asset(request).unwrap();

        let assets = repo.get_all_assets().unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].name, "Test Asset");
    }

    #[test]
    fn test_update_asset() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let request = CreateAssetRequest {
            name: "Original Name".to_string(),
            description: "Original Description".to_string(),
            created_by: 1,
        };

        let mut asset = repo.create_asset(request).unwrap();
        asset.name = "Updated Name".to_string();
        asset.description = "Updated Description".to_string();

        repo.update_asset(&asset).unwrap();

        let updated_asset = repo.get_asset_by_id(asset.id).unwrap().unwrap();
        assert_eq!(updated_asset.name, "Updated Name");
        assert_eq!(updated_asset.description, "Updated Description");
    }

    #[test]
    fn test_delete_asset() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let request = CreateAssetRequest {
            name: "Test Asset".to_string(),
            description: "Test Description".to_string(),
            created_by: 1,
        };

        let asset = repo.create_asset(request).unwrap();
        repo.delete_asset(asset.id).unwrap();

        let result = repo.get_asset_by_id(asset.id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_asset_exists_by_name() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let request = CreateAssetRequest {
            name: "Test Asset".to_string(),
            description: "Test Description".to_string(),
            created_by: 1,
        };

        assert!(!repo.asset_exists_by_name("Test Asset").unwrap());
        repo.create_asset(request).unwrap();
        assert!(repo.asset_exists_by_name("Test Asset").unwrap());
    }
}