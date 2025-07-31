use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub asset_type: AssetType,
    pub parent_id: Option<i64>,
    pub sort_order: i64,
    pub created_by: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssetType {
    Folder,
    Device,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::Folder => "folder",
            AssetType::Device => "device",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "folder" => Ok(AssetType::Folder),
            "device" => Ok(AssetType::Device),
            _ => Err(anyhow::anyhow!("Invalid asset type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub asset_type: AssetType,
    pub parent_id: Option<i64>,
    pub sort_order: i64,
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
            asset_type: asset.asset_type,
            parent_id: asset.parent_id,
            sort_order: asset.sort_order,
            created_by: asset.created_by,
            created_by_username: String::new(), // Will be populated from database
            created_at: asset.created_at,
            version_count: 0,
            latest_version: None,
            latest_version_notes: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_assets: i64,
    pub total_versions: i64,
    pub encryption_type: String,
}

#[derive(Debug, Clone)]
pub struct CreateAssetRequest {
    pub name: String,
    pub description: String,
    pub asset_type: AssetType,
    pub parent_id: Option<i64>,
    pub created_by: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetHierarchy {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub asset_type: AssetType,
    pub parent_id: Option<i64>,
    pub sort_order: i64,
    pub children: Vec<AssetHierarchy>,
    pub created_by: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct MoveAssetRequest {
    pub asset_id: i64,
    pub new_parent_id: Option<i64>,
    pub new_sort_order: Option<i64>,
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

    // Hierarchy management methods
    fn get_asset_hierarchy(&self) -> Result<Vec<AssetHierarchy>>;
    fn get_children_assets(&self, parent_id: Option<i64>) -> Result<Vec<Asset>>;
    fn move_asset(&self, request: MoveAssetRequest) -> Result<()>;
    fn validate_asset_move(&self, asset_id: i64, new_parent_id: Option<i64>) -> Result<bool>;
    fn get_asset_path(&self, asset_id: i64) -> Result<Vec<Asset>>;
    fn get_next_sort_order(&self, parent_id: Option<i64>) -> Result<i64>;
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
                asset_type TEXT NOT NULL DEFAULT 'device',
                parent_id INTEGER,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES users(id),
                FOREIGN KEY (parent_id) REFERENCES assets(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_assets_name ON assets(name);
            CREATE INDEX IF NOT EXISTS idx_assets_created_by ON assets(created_by);
            CREATE INDEX IF NOT EXISTS idx_assets_created_at ON assets(created_at);
            CREATE INDEX IF NOT EXISTS idx_assets_parent_id ON assets(parent_id);
            CREATE INDEX IF NOT EXISTS idx_assets_type ON assets(asset_type);
            CREATE INDEX IF NOT EXISTS idx_assets_sort_order ON assets(parent_id, sort_order);
            "#,
        )?;
        Ok(())
    }

    fn row_to_asset(row: &Row) -> rusqlite::Result<Asset> {
        let asset_type_str: String = row.get("asset_type")?;
        let asset_type = AssetType::from_str(&asset_type_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "asset_type".to_string(), rusqlite::types::Type::Text))?;
            
        Ok(Asset {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            asset_type,
            parent_id: row.get("parent_id")?,
            sort_order: row.get("sort_order")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    fn row_to_asset_info(row: &Row) -> rusqlite::Result<AssetInfo> {
        let asset_type_str: String = row.get("asset_type")?;
        let asset_type = AssetType::from_str(&asset_type_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "asset_type".to_string(), rusqlite::types::Type::Text))?;
            
        Ok(AssetInfo {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            asset_type,
            parent_id: row.get("parent_id")?,
            sort_order: row.get("sort_order")?,
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

        // Validate parent exists if provided
        if let Some(parent_id) = request.parent_id {
            match self.get_asset_by_id(parent_id)? {
                Some(parent) if parent.asset_type != AssetType::Folder => {
                    return Err(anyhow::anyhow!("Parent asset must be a folder"));
                }
                None => {
                    return Err(anyhow::anyhow!("Parent asset not found"));
                }
                _ => {}
            }
        }

        // Get next sort order for this parent
        let sort_order = self.get_next_sort_order(request.parent_id)?;

        let mut stmt = self.conn.prepare(
            "INSERT INTO assets (name, description, asset_type, parent_id, sort_order, created_by) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING *"
        )?;

        let asset = stmt.query_row(
            (&request.name, &request.description, request.asset_type.as_str(), &request.parent_id, &sort_order, &request.created_by),
            Self::row_to_asset,
        )?;

        Ok(asset)
    }

    fn get_asset_by_id(&self, asset_id: i64) -> Result<Option<Asset>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at 
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
            "SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at 
             FROM assets WHERE created_by = ?1 ORDER BY parent_id, sort_order, created_at DESC"
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
            "SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at 
             FROM assets ORDER BY parent_id, sort_order, created_at DESC"
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
            "SELECT a.id, a.name, a.description, a.asset_type, a.parent_id, a.sort_order, a.created_by, a.created_at,
                    u.username as created_by_username,
                    COUNT(cv.id) as version_count,
                    MAX(cv.version_number) as latest_version,
                    latest_cv.notes as latest_version_notes
             FROM assets a
             INNER JOIN users u ON a.created_by = u.id
             LEFT JOIN configuration_versions cv ON a.id = cv.asset_id
             LEFT JOIN configuration_versions latest_cv ON a.id = latest_cv.asset_id 
                AND latest_cv.version_number = (SELECT MAX(version_number) FROM configuration_versions WHERE asset_id = a.id)
             GROUP BY a.id, a.name, a.description, a.asset_type, a.parent_id, a.sort_order, a.created_by, a.created_at, u.username, latest_cv.notes
             ORDER BY a.parent_id, a.sort_order, a.created_at DESC"
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
            "UPDATE assets SET name = ?1, description = ?2, asset_type = ?3, parent_id = ?4, sort_order = ?5, updated_at = CURRENT_TIMESTAMP WHERE id = ?6",
            (&asset.name, &asset.description, asset.asset_type.as_str(), &asset.parent_id, &asset.sort_order, &asset.id),
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

    // Hierarchy management methods implementation
    fn get_asset_hierarchy(&self) -> Result<Vec<AssetHierarchy>> {
        // Optimized query that orders by parent_id and sort_order for efficient tree building
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at 
             FROM assets 
             ORDER BY parent_id NULLS FIRST, sort_order, created_at"
        )?;

        let asset_iter = stmt.query_map([], Self::row_to_asset)?;
        let mut assets = Vec::new();

        for asset in asset_iter {
            assets.push(asset?);
        }

        // Use HashMap for O(1) lookups during tree building
        let mut hierarchy_map: std::collections::HashMap<Option<i64>, Vec<AssetHierarchy>> = std::collections::HashMap::new();
        
        // Convert assets to AssetHierarchy and group by parent_id
        for asset in assets {
            let hierarchy_asset = AssetHierarchy {
                id: asset.id,
                name: asset.name,
                description: asset.description,
                asset_type: asset.asset_type,
                parent_id: asset.parent_id,
                sort_order: asset.sort_order,
                children: Vec::new(),
                created_by: asset.created_by,
                created_at: asset.created_at,
                updated_at: asset.updated_at,
            };
            
            hierarchy_map.entry(asset.parent_id).or_insert_with(Vec::new).push(hierarchy_asset);
        }
        
        // Build the hierarchy tree efficiently
        fn build_tree(
            hierarchy_map: &mut std::collections::HashMap<Option<i64>, Vec<AssetHierarchy>>,
            parent_id: Option<i64>
        ) -> Vec<AssetHierarchy> {
            let mut result = Vec::new();
            
            if let Some(mut children) = hierarchy_map.remove(&parent_id) {
                // Children are already sorted by the query ORDER BY clause
                for mut child in children {
                    child.children = build_tree(hierarchy_map, Some(child.id));
                    result.push(child);
                }
            }
            
            result
        }
        
        Ok(build_tree(&mut hierarchy_map, None))
    }

    fn get_children_assets(&self, parent_id: Option<i64>) -> Result<Vec<Asset>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at 
             FROM assets WHERE parent_id IS ?1 ORDER BY sort_order, created_at"
        )?;

        let asset_iter = stmt.query_map([parent_id], Self::row_to_asset)?;
        let mut assets = Vec::new();

        for asset in asset_iter {
            assets.push(asset?);
        }

        Ok(assets)
    }

    fn move_asset(&self, request: MoveAssetRequest) -> Result<()> {
        // Validate the move won't create a circular reference
        if !self.validate_asset_move(request.asset_id, request.new_parent_id)? {
            return Err(anyhow::anyhow!("Move would create a circular reference"));
        }

        // Get current asset to preserve other data
        let asset = self.get_asset_by_id(request.asset_id)?
            .ok_or_else(|| anyhow::anyhow!("Asset not found"))?;

        // If no new sort order provided, append to end
        let sort_order = match request.new_sort_order {
            Some(order) => order,
            None => self.get_next_sort_order(request.new_parent_id)?,
        };

        // Update the asset with new parent and sort order
        let rows_affected = self.conn.execute(
            "UPDATE assets SET parent_id = ?1, sort_order = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
            (&request.new_parent_id, &sort_order, &request.asset_id),
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Asset not found"));
        }

        Ok(())
    }

    fn validate_asset_move(&self, asset_id: i64, new_parent_id: Option<i64>) -> Result<bool> {
        // If moving to root level, always valid
        if new_parent_id.is_none() {
            return Ok(true);
        }
        
        let new_parent_id = new_parent_id.unwrap();
        
        // Can't move asset to itself
        if asset_id == new_parent_id {
            return Ok(false);
        }
        
        // Check if new parent exists and is a folder
        match self.get_asset_by_id(new_parent_id)? {
            Some(parent) if parent.asset_type == AssetType::Folder => {},
            Some(_) => return Ok(false), // Parent is not a folder
            None => return Ok(false), // Parent doesn't exist
        }
        
        // Check for circular reference by walking up the parent chain
        let mut current_parent_id = Some(new_parent_id);
        let mut visited = std::collections::HashSet::new();
        
        while let Some(parent_id) = current_parent_id {
            if visited.contains(&parent_id) {
                // Infinite loop detected - should not happen with good data
                return Ok(false);
            }
            visited.insert(parent_id);
            
            if parent_id == asset_id {
                // Found circular reference
                return Ok(false);
            }
            
            match self.get_asset_by_id(parent_id)? {
                Some(parent) => current_parent_id = parent.parent_id,
                None => break,
            }
        }
        
        Ok(true)
    }

    fn get_asset_path(&self, asset_id: i64) -> Result<Vec<Asset>> {
        let mut path = Vec::new();
        let mut current_id = Some(asset_id);
        
        while let Some(id) = current_id {
            match self.get_asset_by_id(id)? {
                Some(asset) => {
                    current_id = asset.parent_id;
                    path.push(asset);
                }
                None => break,
            }
        }
        
        path.reverse(); // Root to asset order
        Ok(path)
    }

    fn get_next_sort_order(&self, parent_id: Option<i64>) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM assets WHERE parent_id IS ?1"
        )?;
        
        let next_order: i64 = stmt.query_row([parent_id], |row| row.get(0))?;
        Ok(next_order)
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
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
        };

        let asset = repo.create_asset(request).unwrap();
        assert_eq!(asset.name, "PLC-Line5");
        assert_eq!(asset.description, "Production Line 5 PLC Configuration");
        assert_eq!(asset.asset_type, AssetType::Device);
        assert_eq!(asset.parent_id, None);
        assert_eq!(asset.sort_order, 0);
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
            asset_type: AssetType::Device,
            parent_id: None,
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

    // Hierarchy-specific tests
    #[test]
    fn test_folder_creation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let request = CreateAssetRequest {
            name: "Production Line 1".to_string(),
            description: "Main production line".to_string(),
            asset_type: AssetType::Folder,
            parent_id: None,
            created_by: 1,
        };

        let asset = repo.create_asset(request).unwrap();
        assert_eq!(asset.asset_type, AssetType::Folder);
        assert_eq!(asset.parent_id, None);
        assert_eq!(asset.sort_order, 0);
    }

    #[test]
    fn test_device_in_folder_creation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        // Create parent folder
        let folder_request = CreateAssetRequest {
            name: "Control Room".to_string(),
            description: "Control room assets".to_string(),
            asset_type: AssetType::Folder,
            parent_id: None,
            created_by: 1,
        };
        let folder = repo.create_asset(folder_request).unwrap();

        // Create device in folder
        let device_request = CreateAssetRequest {
            name: "PLC-001".to_string(),
            description: "Primary PLC".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder.id),
            created_by: 1,
        };
        let device = repo.create_asset(device_request).unwrap();

        assert_eq!(device.asset_type, AssetType::Device);
        assert_eq!(device.parent_id, Some(folder.id));
        assert_eq!(device.sort_order, 0);
    }

    #[test]
    fn test_cannot_create_device_with_device_parent() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        // Create parent device
        let device_request = CreateAssetRequest {
            name: "PLC-001".to_string(),
            description: "Primary PLC".to_string(),
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
        };
        let device = repo.create_asset(device_request).unwrap();

        // Try to create child device with device parent (should fail)
        let child_request = CreateAssetRequest {
            name: "Sub-Device".to_string(),
            description: "Child device".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(device.id),
            created_by: 1,
        };
        
        let result = repo.create_asset(child_request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Parent asset must be a folder"));
    }

    #[test]
    fn test_sort_order_assignment() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        // Create parent folder
        let folder_request = CreateAssetRequest {
            name: "Production Line".to_string(),
            description: "Production line assets".to_string(),
            asset_type: AssetType::Folder,
            parent_id: None,
            created_by: 1,
        };
        let folder = repo.create_asset(folder_request).unwrap();

        // Create first device
        let device1_request = CreateAssetRequest {
            name: "PLC-001".to_string(),
            description: "First PLC".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder.id),
            created_by: 1,
        };
        let device1 = repo.create_asset(device1_request).unwrap();
        assert_eq!(device1.sort_order, 0);

        // Create second device
        let device2_request = CreateAssetRequest {
            name: "PLC-002".to_string(),
            description: "Second PLC".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder.id),
            created_by: 1,
        };
        let device2 = repo.create_asset(device2_request).unwrap();
        assert_eq!(device2.sort_order, 1);
    }

    #[test]
    fn test_get_children_assets() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        // Create parent folder
        let folder_request = CreateAssetRequest {
            name: "Production Line".to_string(),
            description: "Production line assets".to_string(),
            asset_type: AssetType::Folder,
            parent_id: None,
            created_by: 1,
        };
        let folder = repo.create_asset(folder_request).unwrap();

        // Create child assets
        let device1_request = CreateAssetRequest {
            name: "PLC-001".to_string(),
            description: "First PLC".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder.id),
            created_by: 1,
        };
        repo.create_asset(device1_request).unwrap();

        let device2_request = CreateAssetRequest {
            name: "PLC-002".to_string(),
            description: "Second PLC".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder.id),
            created_by: 1,
        };
        repo.create_asset(device2_request).unwrap();

        // Get children
        let children = repo.get_children_assets(Some(folder.id)).unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name, "PLC-001");
        assert_eq!(children[1].name, "PLC-002");

        // Get root level assets
        let root_assets = repo.get_children_assets(None).unwrap();
        assert_eq!(root_assets.len(), 1);
        assert_eq!(root_assets[0].name, "Production Line");
    }

    #[test]
    fn test_move_asset() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        // Create folder structure
        let folder1_request = CreateAssetRequest {
            name: "Folder 1".to_string(),
            description: "First folder".to_string(),
            asset_type: AssetType::Folder,
            parent_id: None,
            created_by: 1,
        };
        let folder1 = repo.create_asset(folder1_request).unwrap();

        let folder2_request = CreateAssetRequest {
            name: "Folder 2".to_string(),
            description: "Second folder".to_string(),
            asset_type: AssetType::Folder,
            parent_id: None,
            created_by: 1,
        };
        let folder2 = repo.create_asset(folder2_request).unwrap();

        let device_request = CreateAssetRequest {
            name: "Device".to_string(),
            description: "Test device".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder1.id),
            created_by: 1,
        };
        let device = repo.create_asset(device_request).unwrap();

        // Move device from folder1 to folder2
        let move_request = MoveAssetRequest {
            asset_id: device.id,
            new_parent_id: Some(folder2.id),
            new_sort_order: None,
        };
        repo.move_asset(move_request).unwrap();

        // Verify move
        let moved_device = repo.get_asset_by_id(device.id).unwrap().unwrap();
        assert_eq!(moved_device.parent_id, Some(folder2.id));
        assert_eq!(moved_device.sort_order, 0);
    }

    #[test]
    fn test_validate_asset_move() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        // Create folder structure
        let folder1_request = CreateAssetRequest {
            name: "Folder 1".to_string(),
            description: "First folder".to_string(),
            asset_type: AssetType::Folder,
            parent_id: None,
            created_by: 1,
        };
        let folder1 = repo.create_asset(folder1_request).unwrap();

        let folder2_request = CreateAssetRequest {
            name: "Folder 2".to_string(),
            description: "Second folder".to_string(),
            asset_type: AssetType::Folder,
            parent_id: Some(folder1.id),
            created_by: 1,
        };
        let folder2 = repo.create_asset(folder2_request).unwrap();

        let device_request = CreateAssetRequest {
            name: "Device".to_string(),
            description: "Test device".to_string(),
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
        };
        let device = repo.create_asset(device_request).unwrap();

        // Valid moves
        assert!(repo.validate_asset_move(device.id, Some(folder1.id)).unwrap());
        assert!(repo.validate_asset_move(device.id, None).unwrap());

        // Invalid moves
        assert!(!repo.validate_asset_move(folder1.id, Some(folder1.id)).unwrap()); // Self reference
        assert!(!repo.validate_asset_move(folder1.id, Some(folder2.id)).unwrap()); // Circular reference
        assert!(!repo.validate_asset_move(device.id, Some(device.id)).unwrap()); // Move to self
        assert!(!repo.validate_asset_move(folder1.id, Some(999)).unwrap()); // Non-existent parent
    }

    #[test]
    fn test_get_asset_path() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        // Create nested structure: Root -> Folder1 -> Folder2 -> Device
        let folder1_request = CreateAssetRequest {
            name: "Folder 1".to_string(),
            description: "First folder".to_string(),
            asset_type: AssetType::Folder,
            parent_id: None,
            created_by: 1,
        };
        let folder1 = repo.create_asset(folder1_request).unwrap();

        let folder2_request = CreateAssetRequest {
            name: "Folder 2".to_string(),
            description: "Second folder".to_string(),
            asset_type: AssetType::Folder,
            parent_id: Some(folder1.id),
            created_by: 1,
        };
        let folder2 = repo.create_asset(folder2_request).unwrap();

        let device_request = CreateAssetRequest {
            name: "Device".to_string(),
            description: "Test device".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder2.id),
            created_by: 1,
        };
        let device = repo.create_asset(device_request).unwrap();

        // Get path for device
        let path = repo.get_asset_path(device.id).unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].name, "Folder 1");
        assert_eq!(path[1].name, "Folder 2");
        assert_eq!(path[2].name, "Device");

        // Get path for folder2
        let path2 = repo.get_asset_path(folder2.id).unwrap();
        assert_eq!(path2.len(), 2);
        assert_eq!(path2[0].name, "Folder 1");
        assert_eq!(path2[1].name, "Folder 2");

        // Get path for root asset
        let path3 = repo.get_asset_path(folder1.id).unwrap();
        assert_eq!(path3.len(), 1);
        assert_eq!(path3[0].name, "Folder 1");
    }

    #[test]
    fn test_get_asset_hierarchy() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        // Create test hierarchy
        let folder1_request = CreateAssetRequest {
            name: "Production Line 1".to_string(),
            description: "Main production line".to_string(),
            asset_type: AssetType::Folder,
            parent_id: None,
            created_by: 1,
        };
        let folder1 = repo.create_asset(folder1_request).unwrap();

        let device1_request = CreateAssetRequest {
            name: "PLC-001".to_string(),
            description: "Primary PLC".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder1.id),
            created_by: 1,
        };
        repo.create_asset(device1_request).unwrap();

        let subfolder_request = CreateAssetRequest {
            name: "Control Room".to_string(),
            description: "Control room assets".to_string(),
            asset_type: AssetType::Folder,
            parent_id: Some(folder1.id),
            created_by: 1,
        };
        let subfolder = repo.create_asset(subfolder_request).unwrap();

        let device2_request = CreateAssetRequest {
            name: "HMI-001".to_string(),
            description: "Main HMI".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(subfolder.id),
            created_by: 1,
        };
        repo.create_asset(device2_request).unwrap();

        // Get hierarchy
        let hierarchy = repo.get_asset_hierarchy().unwrap();
        assert_eq!(hierarchy.len(), 1); // One root folder

        let root_folder = &hierarchy[0];
        assert_eq!(root_folder.name, "Production Line 1");
        assert_eq!(root_folder.children.len(), 2); // PLC-001 and Control Room

        // Find the control room subfolder
        let control_room = root_folder.children.iter()
            .find(|child| child.name == "Control Room")
            .unwrap();
        assert_eq!(control_room.children.len(), 1); // HMI-001
        assert_eq!(control_room.children[0].name, "HMI-001");
    }
}