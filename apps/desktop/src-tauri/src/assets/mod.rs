use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

pub mod tree_navigation;

// Re-export tree navigation commands
pub use tree_navigation::{
    batch_load_tree_nodes,
    search_tree_nodes,
    get_tree_statistics,
    preload_tree_nodes,
    get_node_metadata,
};

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
    pub security_classification: SecurityClassification,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssetType {
    Folder,
    Device,
    Equipment,
    System,
    Area,
    Line,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::Folder => "folder",
            AssetType::Device => "device",
            AssetType::Equipment => "equipment",
            AssetType::System => "system",
            AssetType::Area => "area",
            AssetType::Line => "line",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "folder" => Ok(AssetType::Folder),
            "device" => Ok(AssetType::Device),
            "equipment" => Ok(AssetType::Equipment),
            "system" => Ok(AssetType::System),
            "area" => Ok(AssetType::Area),
            "line" => Ok(AssetType::Line),
            _ => Err(anyhow::anyhow!("Invalid asset type: {}", s)),
        }
    }
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Security classification of an asset, serialized with the same names as
/// `SecurityClassificationLevel` in security_commands and the frontend enum.
/// Variants are declared lowest to highest, and the derived ordering is what
/// the inheritance rules compare, so don't reorder them.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum SecurityClassification {
    #[serde(rename = "PUBLIC")]
    Public,
    #[serde(rename = "INTERNAL")]
    #[default]
    Internal,
    #[serde(rename = "CONFIDENTIAL")]
    Confidential,
    #[serde(rename = "RESTRICTED")]
    Restricted,
    #[serde(rename = "SECRET")]
    Secret,
}

impl SecurityClassification {
    pub fn as_str(&self) -> &'static str {
        match self {
            SecurityClassification::Public => "PUBLIC",
            SecurityClassification::Internal => "INTERNAL",
            SecurityClassification::Confidential => "CONFIDENTIAL",
            SecurityClassification::Restricted => "RESTRICTED",
            SecurityClassification::Secret => "SECRET",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "PUBLIC" => Ok(SecurityClassification::Public),
            "INTERNAL" => Ok(SecurityClassification::Internal),
            "CONFIDENTIAL" => Ok(SecurityClassification::Confidential),
            "RESTRICTED" => Ok(SecurityClassification::Restricted),
            "SECRET" => Ok(SecurityClassification::Secret),
            _ => Err(anyhow::anyhow!(
                "Invalid security classification: {}. Must be one of PUBLIC, INTERNAL, CONFIDENTIAL, RESTRICTED, SECRET",
                s
            )),
        }
    }
}

impl std::fmt::Display for SecurityClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl rusqlite::types::FromSql for SecurityClassification {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        SecurityClassification::from_str(value.as_str()?)
            .map_err(|e| rusqlite::types::FromSqlError::Other(e.into()))
    }
}

impl rusqlite::types::ToSql for SecurityClassification {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.as_str()))
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
    pub security_classification: SecurityClassification,
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
            security_classification: asset.security_classification,
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
    /// `None` inherits the parent folder's classification (INTERNAL at the root).
    pub security_classification: Option<SecurityClassification>,
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
    pub security_classification: SecurityClassification,
}

/// Outcome of `update_asset_classification`: the updated asset, its previous
/// level, and the descendants that were raised, as they were before the raise.
#[derive(Debug, Clone)]
pub struct ClassificationChange {
    pub asset: Asset,
    pub previous: SecurityClassification,
    pub raised: Vec<Asset>,
}

#[derive(Debug, Clone)]
pub struct MoveAssetRequest {
    pub asset_id: i64,
    pub new_parent_id: Option<i64>,
    pub new_sort_order: Option<i64>,
}

// Thread-safe service wrapper
pub struct AssetService {
    db_manager: std::sync::Arc<std::sync::Mutex<crate::database::DatabaseManager>>,
}

impl AssetService {
    pub fn new(db_manager: std::sync::Arc<std::sync::Mutex<crate::database::DatabaseManager>>) -> Self {
        Self { db_manager }
    }

    /// Create asset from workflow data
    pub fn create_asset_from_workflow(&self, user_id: i64, asset_data: crate::workflow::AssetCreationData) -> Result<i64> {
        let db_manager = self.db_manager.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire database lock: {}", e)
        })?;
        let conn = db_manager.get_connection();
        let repo = SqliteAssetRepository::new(conn);
        repo.create_asset_from_workflow(user_id, asset_data)
    }

    /// Check if user has permission to perform action on asset
    pub fn check_asset_permission(&self, user_id: i64, asset_id: i64, permission: &str) -> Result<bool> {
        let db_manager = self.db_manager.lock().map_err(|e| {
            anyhow::anyhow!("Failed to acquire database lock: {}", e)
        })?;
        let conn = db_manager.get_connection();
        let repo = SqliteAssetRepository::new(conn);
        repo.check_asset_permission(user_id, asset_id, permission)
    }
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
                security_classification TEXT NOT NULL DEFAULT 'INTERNAL',
                FOREIGN KEY (created_by) REFERENCES users(id),
                FOREIGN KEY (parent_id) REFERENCES assets(id) ON DELETE CASCADE
            );
            "#,
        )?;

        // Databases created before v0.5.0 have an assets table without the
        // hierarchy columns, and CREATE TABLE IF NOT EXISTS won't add them.
        // They must exist before the indexes below are created. Classification
        // arrived after v0.6.0: existing rows get INTERNAL and are left alone,
        // even where that leaves a child below its folder's level.
        for (column, definition) in [
            ("asset_type", "TEXT NOT NULL DEFAULT 'device'"),
            ("parent_id", "INTEGER REFERENCES assets(id) ON DELETE CASCADE"),
            ("sort_order", "INTEGER NOT NULL DEFAULT 0"),
            ("security_classification", "TEXT NOT NULL DEFAULT 'INTERNAL'"),
        ] {
            let exists: bool = self.conn.query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('assets') WHERE name = ?1",
                [column],
                |row| row.get(0),
            )?;
            if !exists {
                self.conn.execute(&format!("ALTER TABLE assets ADD COLUMN {} {}", column, definition), [])?;
            }
        }

        self.conn.execute_batch(
            r#"
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
            security_classification: row.get("security_classification")?,
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
            security_classification: row.get("security_classification")?,
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
        let parent = match request.parent_id {
            Some(parent_id) => match self.get_asset_by_id(parent_id)? {
                Some(parent) if parent.asset_type != AssetType::Folder => {
                    return Err(anyhow::anyhow!("Parent asset must be a folder"));
                }
                None => {
                    return Err(anyhow::anyhow!("Parent asset not found"));
                }
                Some(parent) => Some(parent),
            },
            None => None,
        };

        // Inherit the folder's classification unless a level is given, and
        // never allow one below it
        let security_classification = match (request.security_classification, &parent) {
            (Some(requested), Some(parent)) if requested < parent.security_classification => {
                return Err(anyhow::anyhow!(
                    "Security classification {} is below folder '{}' ({}). An asset must be classified at or above its folder.",
                    requested, parent.name, parent.security_classification
                ));
            }
            (Some(requested), _) => requested,
            (None, Some(parent)) => parent.security_classification,
            (None, None) => SecurityClassification::default(),
        };

        // Get next sort order for this parent
        let sort_order = self.get_next_sort_order(request.parent_id)?;

        let mut stmt = self.conn.prepare(
            "INSERT INTO assets (name, description, asset_type, parent_id, sort_order, created_by, security_classification)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING *"
        )?;

        let asset = stmt.query_row(
            (&request.name, &request.description, request.asset_type.as_str(), &request.parent_id, &sort_order, &request.created_by, security_classification),
            Self::row_to_asset,
        )?;

        Ok(asset)
    }

    fn get_asset_by_id(&self, asset_id: i64) -> Result<Option<Asset>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at, security_classification 
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
            "SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at, security_classification 
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
            "SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at, security_classification 
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
                    a.security_classification,
                    u.username as created_by_username,
                    COUNT(cv.id) as version_count,
                    MAX(cv.version_number) as latest_version,
                    latest_cv.notes as latest_version_notes
             FROM assets a
             INNER JOIN users u ON a.created_by = u.id
             LEFT JOIN configuration_versions cv ON a.id = cv.asset_id
             LEFT JOIN configuration_versions latest_cv ON a.id = latest_cv.asset_id 
                AND latest_cv.version_number = (SELECT MAX(version_number) FROM configuration_versions WHERE asset_id = a.id)
             GROUP BY a.id, a.name, a.description, a.asset_type, a.parent_id, a.sort_order, a.created_by, a.created_at, a.security_classification, u.username, latest_cv.notes
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
        // security_classification is deliberately not written here; it changes
        // only through update_asset_classification, which enforces the folder rules
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
            "SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at, security_classification 
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
                security_classification: asset.security_classification,
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
            "SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at, security_classification 
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
        let new_parent = match self.get_asset_by_id(new_parent_id)? {
            Some(parent) if parent.asset_type == AssetType::Folder => parent,
            Some(_) => return Ok(false), // Parent is not a folder
            None => return Ok(false), // Parent doesn't exist
        };
        
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

        // Nothing in the moved subtree may end up below the destination
        // folder's classification. This is an error rather than `false` so the
        // caller can show which level blocks the move; it is never auto-raised.
        let target = new_parent.security_classification;
        let subtree = self.get_subtree(asset_id)?;
        if let Some(moved) = subtree.iter().find(|a| a.id == asset_id) {
            if moved.security_classification < target {
                return Err(anyhow::anyhow!(
                    "Cannot move '{}' ({}) into folder '{}' ({}): an asset cannot be classified below its folder. Raise it to {} or higher first.",
                    moved.name, moved.security_classification, new_parent.name, target, target
                ));
            }
            let blocked: Vec<&Asset> = subtree.iter()
                .filter(|a| a.id != asset_id && a.security_classification < target)
                .collect();
            if let Some(lowest) = blocked.iter().min_by_key(|a| a.security_classification) {
                return Err(anyhow::anyhow!(
                    "Cannot move '{}' into folder '{}' ({}): {} item(s) inside it are classified below {}, including '{}' ({}). Raise them to {} or higher first.",
                    moved.name, new_parent.name, target, blocked.len(), target, lowest.name, lowest.security_classification, target
                ));
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

impl<'a> SqliteAssetRepository<'a> {
    /// The asset and everything below it, in no particular order. UNION rather
    /// than UNION ALL so a corrupt parent cycle can't recurse forever.
    fn get_subtree(&self, asset_id: i64) -> Result<Vec<Asset>> {
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE subtree(id) AS (
                 SELECT id FROM assets WHERE id = ?1
                 UNION
                 SELECT a.id FROM assets a JOIN subtree s ON a.parent_id = s.id
             )
             SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at, security_classification
             FROM assets WHERE id IN (SELECT id FROM subtree)"
        )?;

        let asset_iter = stmt.query_map([asset_id], Self::row_to_asset)?;
        let mut assets = Vec::new();

        for asset in asset_iter {
            assets.push(asset?);
        }

        Ok(assets)
    }

    /// Change an asset's classification. It may not go below its parent
    /// folder's level. Raising a folder also raises every descendant that would
    /// otherwise sit below it; lowering one leaves its descendants as they are.
    /// All updates happen in one transaction.
    pub fn update_asset_classification(
        &self,
        asset_id: i64,
        classification: SecurityClassification,
    ) -> Result<ClassificationChange> {
        let asset = self.get_asset_by_id(asset_id)?
            .ok_or_else(|| anyhow::anyhow!("Asset not found"))?;

        if let Some(parent_id) = asset.parent_id {
            if let Some(parent) = self.get_asset_by_id(parent_id)? {
                if classification < parent.security_classification {
                    return Err(anyhow::anyhow!(
                        "Cannot set '{}' to {}: its folder '{}' is classified {}. An asset cannot be classified below its folder.",
                        asset.name, classification, parent.name, parent.security_classification
                    ));
                }
            }
        }

        let raised: Vec<Asset> = self.get_subtree(asset_id)?
            .into_iter()
            .filter(|a| a.id != asset_id && a.security_classification < classification)
            .collect();

        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE assets SET security_classification = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            (classification, asset_id),
        )?;
        for descendant in &raised {
            tx.execute(
                "UPDATE assets SET security_classification = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                (classification, descendant.id),
            )?;
        }
        tx.commit()?;

        let updated = self.get_asset_by_id(asset_id)?
            .ok_or_else(|| anyhow::anyhow!("Asset not found"))?;

        Ok(ClassificationChange {
            previous: asset.security_classification,
            asset: updated,
            raised,
        })
    }

    /// Create asset from workflow data
    pub fn create_asset_from_workflow(&self, user_id: i64, asset_data: crate::workflow::AssetCreationData) -> Result<i64> {
        use crate::assets::AssetType;
        
        // Convert asset type string to enum
        let asset_type = match asset_data.asset_type.as_str() {
            "Equipment" => AssetType::Equipment,
            "System" => AssetType::System,
            "Device" => AssetType::Device,
            "Area" => AssetType::Area,
            "Line" => AssetType::Line,
            _ => AssetType::Equipment, // Default fallback
        };

        let request = CreateAssetRequest {
            name: asset_data.name,
            description: asset_data.description.unwrap_or_default(),
            asset_type,
            parent_id: asset_data.parent_id,
            created_by: user_id,
            security_classification: None,
        };

        let asset = self.create_asset(request)?;
        Ok(asset.id)
    }

    /// Check if user has permission to perform action on asset
    pub fn check_asset_permission(&self, user_id: i64, asset_id: i64, permission: &str) -> Result<bool> {
        // For now, implement basic permission check
        // In a full implementation, this would check role-based permissions, asset ownership, etc.
        
        // Check if asset exists
        let asset = self.get_asset_by_id(asset_id)?;
        if asset.is_none() {
            return Ok(false);
        }
        
        // Basic permission: user can access assets they created or if they're admin
        // This is a simplified implementation - extend based on requirements
        let asset = asset.unwrap();
        Ok(asset.created_by == user_id || permission == "read")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use rusqlite::Connection;

    #[test]
    fn test_initialize_schema_upgrades_pre_hierarchy_assets_table() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            CREATE TABLE users (id INTEGER PRIMARY KEY, username TEXT NOT NULL, password_hash TEXT NOT NULL, role TEXT NOT NULL);
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'testuser', 'hash', 'Engineer');
            -- assets as created by v0.4.x
            CREATE TABLE assets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES users(id)
            );
            INSERT INTO assets (id, name, description, created_by) VALUES (1, 'PLC-1', 'old', 1);
            "#,
        ).unwrap();

        let repo = SqliteAssetRepository::new(&conn);
        repo.initialize_schema().unwrap();
        // Running it again must be a no-op
        repo.initialize_schema().unwrap();

        let asset = repo.get_asset_by_id(1).unwrap().unwrap();
        assert_eq!(asset.name, "PLC-1");
        assert_eq!(asset.asset_type, AssetType::Device);
        assert_eq!(asset.parent_id, None);
        assert_eq!(asset.sort_order, 0);
        let index_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name IN ('idx_assets_parent_id', 'idx_assets_type', 'idx_assets_sort_order')",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(index_count, 3);
    }

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
            security_classification: None,
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
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
            security_classification: None,
        };
        assert!(repo.create_asset(request).is_err());

        // Test short name
        let request = CreateAssetRequest {
            name: "A".to_string(),
            description: "Test".to_string(),
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
            security_classification: None,
        };
        assert!(repo.create_asset(request).is_err());

        // Test long name
        let request = CreateAssetRequest {
            name: "A".repeat(101),
            description: "Test".to_string(),
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
            security_classification: None,
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
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
            security_classification: None,
        };

        let request2 = CreateAssetRequest {
            name: "PLC-Line5".to_string(),
            description: "Second PLC".to_string(),
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
            security_classification: None,
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
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
            security_classification: None,
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
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
            security_classification: None,
        };
        let request2 = CreateAssetRequest {
            name: "Asset2".to_string(),
            description: "Description2".to_string(),
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
            security_classification: None,
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
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
            security_classification: None,
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
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
            security_classification: None,
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
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
            security_classification: None,
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
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
            security_classification: None,
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
            security_classification: None,
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
            security_classification: None,
        };
        let folder = repo.create_asset(folder_request).unwrap();

        // Create device in folder
        let device_request = CreateAssetRequest {
            name: "PLC-001".to_string(),
            description: "Primary PLC".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder.id),
            created_by: 1,
            security_classification: None,
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
            security_classification: None,
        };
        let device = repo.create_asset(device_request).unwrap();

        // Try to create child device with device parent (should fail)
        let child_request = CreateAssetRequest {
            name: "Sub-Device".to_string(),
            description: "Child device".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(device.id),
            created_by: 1,
            security_classification: None,
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
            security_classification: None,
        };
        let folder = repo.create_asset(folder_request).unwrap();

        // Create first device
        let device1_request = CreateAssetRequest {
            name: "PLC-001".to_string(),
            description: "First PLC".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder.id),
            created_by: 1,
            security_classification: None,
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
            security_classification: None,
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
            security_classification: None,
        };
        let folder = repo.create_asset(folder_request).unwrap();

        // Create child assets
        let device1_request = CreateAssetRequest {
            name: "PLC-001".to_string(),
            description: "First PLC".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder.id),
            created_by: 1,
            security_classification: None,
        };
        repo.create_asset(device1_request).unwrap();

        let device2_request = CreateAssetRequest {
            name: "PLC-002".to_string(),
            description: "Second PLC".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder.id),
            created_by: 1,
            security_classification: None,
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
            security_classification: None,
        };
        let folder1 = repo.create_asset(folder1_request).unwrap();

        let folder2_request = CreateAssetRequest {
            name: "Folder 2".to_string(),
            description: "Second folder".to_string(),
            asset_type: AssetType::Folder,
            parent_id: None,
            created_by: 1,
            security_classification: None,
        };
        let folder2 = repo.create_asset(folder2_request).unwrap();

        let device_request = CreateAssetRequest {
            name: "Device".to_string(),
            description: "Test device".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder1.id),
            created_by: 1,
            security_classification: None,
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
            security_classification: None,
        };
        let folder1 = repo.create_asset(folder1_request).unwrap();

        let folder2_request = CreateAssetRequest {
            name: "Folder 2".to_string(),
            description: "Second folder".to_string(),
            asset_type: AssetType::Folder,
            parent_id: Some(folder1.id),
            created_by: 1,
            security_classification: None,
        };
        let folder2 = repo.create_asset(folder2_request).unwrap();

        let device_request = CreateAssetRequest {
            name: "Device".to_string(),
            description: "Test device".to_string(),
            asset_type: AssetType::Device,
            parent_id: None,
            created_by: 1,
            security_classification: None,
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
            security_classification: None,
        };
        let folder1 = repo.create_asset(folder1_request).unwrap();

        let folder2_request = CreateAssetRequest {
            name: "Folder 2".to_string(),
            description: "Second folder".to_string(),
            asset_type: AssetType::Folder,
            parent_id: Some(folder1.id),
            created_by: 1,
            security_classification: None,
        };
        let folder2 = repo.create_asset(folder2_request).unwrap();

        let device_request = CreateAssetRequest {
            name: "Device".to_string(),
            description: "Test device".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder2.id),
            created_by: 1,
            security_classification: None,
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
            security_classification: None,
        };
        let folder1 = repo.create_asset(folder1_request).unwrap();

        let device1_request = CreateAssetRequest {
            name: "PLC-001".to_string(),
            description: "Primary PLC".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(folder1.id),
            created_by: 1,
            security_classification: None,
        };
        repo.create_asset(device1_request).unwrap();

        let subfolder_request = CreateAssetRequest {
            name: "Control Room".to_string(),
            description: "Control room assets".to_string(),
            asset_type: AssetType::Folder,
            parent_id: Some(folder1.id),
            created_by: 1,
            security_classification: None,
        };
        let subfolder = repo.create_asset(subfolder_request).unwrap();

        let device2_request = CreateAssetRequest {
            name: "HMI-001".to_string(),
            description: "Main HMI".to_string(),
            asset_type: AssetType::Device,
            parent_id: Some(subfolder.id),
            created_by: 1,
            security_classification: None,
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

    // Security classification tests

    fn create_classified(
        repo: &SqliteAssetRepository,
        name: &str,
        asset_type: AssetType,
        parent_id: Option<i64>,
        security_classification: Option<SecurityClassification>,
    ) -> Result<Asset> {
        repo.create_asset(CreateAssetRequest {
            name: name.to_string(),
            description: String::new(),
            asset_type,
            parent_id,
            created_by: 1,
            security_classification,
        })
    }

    fn classification_of(repo: &SqliteAssetRepository, asset_id: i64) -> SecurityClassification {
        repo.get_asset_by_id(asset_id).unwrap().unwrap().security_classification
    }

    #[test]
    fn test_classification_ordering() {
        use SecurityClassification::*;
        assert!(Public < Internal && Internal < Confidential && Confidential < Restricted && Restricted < Secret);
        assert_eq!(SecurityClassification::default(), Internal);
        assert_eq!(SecurityClassification::from_str("SECRET").unwrap(), Secret);
        assert!(SecurityClassification::from_str("secret").is_err());
        assert!(SecurityClassification::from_str("TOP_SECRET").is_err());
        assert_eq!(serde_json::to_string(&Confidential).unwrap(), "\"CONFIDENTIAL\"");
    }

    #[test]
    fn test_initialize_schema_adds_classification_to_existing_assets_table() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE users (id INTEGER PRIMARY KEY, username TEXT NOT NULL, password_hash TEXT NOT NULL, role TEXT NOT NULL);
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'testuser', 'hash', 'Engineer');
            -- assets as created by v0.6.0
            CREATE TABLE assets (
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
            INSERT INTO assets (id, name, description, asset_type, parent_id, created_by) VALUES (1, 'Plant', '', 'folder', NULL, 1);
            INSERT INTO assets (id, name, description, asset_type, parent_id, created_by) VALUES (2, 'PLC-1', '', 'device', 1, 1);
            "#,
        ).unwrap();

        let repo = SqliteAssetRepository::new(&conn);
        repo.initialize_schema().unwrap();
        // Running it again must be a no-op
        repo.initialize_schema().unwrap();

        let column_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('assets') WHERE name = 'security_classification'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(column_count, 1);
        // Legacy rows default to INTERNAL
        assert_eq!(classification_of(&repo, 1), SecurityClassification::Internal);
        assert_eq!(classification_of(&repo, 2), SecurityClassification::Internal);
        let hierarchy = repo.get_asset_hierarchy().unwrap();
        assert_eq!(hierarchy[0].security_classification, SecurityClassification::Internal);
        assert_eq!(hierarchy[0].children[0].security_classification, SecurityClassification::Internal);
    }

    #[test]
    fn test_inconsistent_legacy_classification_is_left_alone() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);
        conn.execute_batch(
            r#"
            INSERT INTO assets (id, name, description, asset_type, parent_id, created_by, security_classification) VALUES (1, 'Vault Room', '', 'folder', NULL, 1, 'SECRET');
            INSERT INTO assets (id, name, description, asset_type, parent_id, created_by, security_classification) VALUES (2, 'Kiosk', '', 'device', 1, 1, 'PUBLIC');
            "#,
        ).unwrap();

        // Startup doesn't fail or rewrite the child that sits below its folder
        repo.initialize_schema().unwrap();
        assert_eq!(classification_of(&repo, 2), SecurityClassification::Public);
        assert_eq!(repo.get_all_assets().unwrap().len(), 2);
    }

    #[test]
    fn test_create_without_classification_inherits_from_parent() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let root = create_classified(&repo, "Plant", AssetType::Folder, None, None).unwrap();
        assert_eq!(root.security_classification, SecurityClassification::Internal);

        let secure = create_classified(&repo, "Secure Zone", AssetType::Folder, None, Some(SecurityClassification::Restricted)).unwrap();
        let sub = create_classified(&repo, "Cabinet", AssetType::Folder, Some(secure.id), None).unwrap();
        let device = create_classified(&repo, "PLC-9", AssetType::Device, Some(sub.id), None).unwrap();
        assert_eq!(sub.security_classification, SecurityClassification::Restricted);
        assert_eq!(device.security_classification, SecurityClassification::Restricted);

        // get_assets_with_info joins configuration versions
        conn.execute_batch(
            "CREATE TABLE configuration_versions (id INTEGER PRIMARY KEY, asset_id INTEGER, version_number TEXT, notes TEXT);",
        ).unwrap();
        let info = repo.get_assets_with_info().unwrap();
        let device_info = info.iter().find(|a| a.id == device.id).unwrap();
        assert_eq!(device_info.security_classification, SecurityClassification::Restricted);
    }

    #[test]
    fn test_create_rejects_classification_below_parent() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let folder = create_classified(&repo, "Secure Zone", AssetType::Folder, None, Some(SecurityClassification::Confidential)).unwrap();
        for asset_type in [AssetType::Device, AssetType::Folder] {
            let err = create_classified(&repo, "Too Low", asset_type, Some(folder.id), Some(SecurityClassification::Internal))
                .unwrap_err()
                .to_string();
            assert!(err.contains("INTERNAL"), "{}", err);
            assert!(err.contains("CONFIDENTIAL"), "{}", err);
            assert!(err.contains("Secure Zone"), "{}", err);
        }
        assert!(!repo.asset_exists_by_name("Too Low").unwrap());
    }

    #[test]
    fn test_create_allows_equal_or_higher_classification() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let folder = create_classified(&repo, "Secure Zone", AssetType::Folder, None, Some(SecurityClassification::Confidential)).unwrap();
        let same = create_classified(&repo, "PLC-1", AssetType::Device, Some(folder.id), Some(SecurityClassification::Confidential)).unwrap();
        let higher = create_classified(&repo, "PLC-2", AssetType::Device, Some(folder.id), Some(SecurityClassification::Secret)).unwrap();
        assert_eq!(same.security_classification, SecurityClassification::Confidential);
        assert_eq!(higher.security_classification, SecurityClassification::Secret);

        // At the root any level is allowed
        let public = create_classified(&repo, "Lobby", AssetType::Folder, None, Some(SecurityClassification::Public)).unwrap();
        assert_eq!(public.security_classification, SecurityClassification::Public);
    }

    #[test]
    fn test_move_rejected_when_asset_below_target_folder() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let secret = create_classified(&repo, "Secret Zone", AssetType::Folder, None, Some(SecurityClassification::Secret)).unwrap();
        let device = create_classified(&repo, "PLC-1", AssetType::Device, None, Some(SecurityClassification::Internal)).unwrap();

        let err = repo.validate_asset_move(device.id, Some(secret.id)).unwrap_err().to_string();
        assert!(err.contains("PLC-1") && err.contains("INTERNAL") && err.contains("SECRET"), "{}", err);

        let err = repo.move_asset(MoveAssetRequest { asset_id: device.id, new_parent_id: Some(secret.id), new_sort_order: None })
            .unwrap_err()
            .to_string();
        assert!(err.contains("SECRET"), "{}", err);
        // Not moved, and not auto-raised
        let unchanged = repo.get_asset_by_id(device.id).unwrap().unwrap();
        assert_eq!(unchanged.parent_id, None);
        assert_eq!(unchanged.security_classification, SecurityClassification::Internal);
    }

    #[test]
    fn test_move_rejected_when_descendant_below_target_folder() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let target = create_classified(&repo, "Restricted Zone", AssetType::Folder, None, Some(SecurityClassification::Restricted)).unwrap();
        let moving = create_classified(&repo, "Line 1", AssetType::Folder, None, Some(SecurityClassification::Restricted)).unwrap();
        let sub = create_classified(&repo, "Cell A", AssetType::Folder, Some(moving.id), None).unwrap();
        // Legacy-style data: a grandchild below its folder
        conn.execute(
            "UPDATE assets SET security_classification = 'CONFIDENTIAL' WHERE id = ?1",
            [sub.id],
        ).unwrap();
        create_classified(&repo, "HMI-1", AssetType::Device, Some(sub.id), Some(SecurityClassification::Secret)).unwrap();

        let err = repo.validate_asset_move(moving.id, Some(target.id)).unwrap_err().to_string();
        assert!(err.contains("Line 1") && err.contains("Cell A") && err.contains("CONFIDENTIAL") && err.contains("RESTRICTED"), "{}", err);
        assert!(repo.move_asset(MoveAssetRequest { asset_id: moving.id, new_parent_id: Some(target.id), new_sort_order: None }).is_err());
        assert_eq!(repo.get_asset_by_id(moving.id).unwrap().unwrap().parent_id, None);
    }

    #[test]
    fn test_move_allowed_into_equal_or_lower_folder() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let confidential = create_classified(&repo, "Zone C", AssetType::Folder, None, Some(SecurityClassification::Confidential)).unwrap();
        let public = create_classified(&repo, "Zone P", AssetType::Folder, None, Some(SecurityClassification::Public)).unwrap();
        let moving = create_classified(&repo, "Line 1", AssetType::Folder, Some(confidential.id), None).unwrap();
        create_classified(&repo, "PLC-1", AssetType::Device, Some(moving.id), Some(SecurityClassification::Secret)).unwrap();

        assert!(repo.validate_asset_move(moving.id, Some(public.id)).unwrap());
        repo.move_asset(MoveAssetRequest { asset_id: moving.id, new_parent_id: Some(public.id), new_sort_order: None }).unwrap();
        assert_eq!(repo.get_asset_by_id(moving.id).unwrap().unwrap().parent_id, Some(public.id));

        repo.move_asset(MoveAssetRequest { asset_id: moving.id, new_parent_id: Some(confidential.id), new_sort_order: None }).unwrap();
        repo.move_asset(MoveAssetRequest { asset_id: moving.id, new_parent_id: None, new_sort_order: None }).unwrap();
        // Moves never change classification
        assert_eq!(classification_of(&repo, moving.id), SecurityClassification::Confidential);
    }

    #[test]
    fn test_update_classification_rejects_below_parent() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let folder = create_classified(&repo, "Zone C", AssetType::Folder, None, Some(SecurityClassification::Confidential)).unwrap();
        let device = create_classified(&repo, "PLC-1", AssetType::Device, Some(folder.id), None).unwrap();

        let err = repo.update_asset_classification(device.id, SecurityClassification::Internal).unwrap_err().to_string();
        assert!(err.contains("INTERNAL") && err.contains("CONFIDENTIAL") && err.contains("Zone C"), "{}", err);
        assert_eq!(classification_of(&repo, device.id), SecurityClassification::Confidential);

        let change = repo.update_asset_classification(device.id, SecurityClassification::Secret).unwrap();
        assert_eq!(change.previous, SecurityClassification::Confidential);
        assert_eq!(change.asset.security_classification, SecurityClassification::Secret);
        assert!(change.raised.is_empty());
        assert!(repo.update_asset_classification(999, SecurityClassification::Secret).is_err());
    }

    #[test]
    fn test_update_classification_raise_cascades_to_lower_descendants() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let root = create_classified(&repo, "Plant", AssetType::Folder, None, Some(SecurityClassification::Internal)).unwrap();
        let low = create_classified(&repo, "PLC-1", AssetType::Device, Some(root.id), None).unwrap();
        let sub = create_classified(&repo, "Cell A", AssetType::Folder, Some(root.id), Some(SecurityClassification::Confidential)).unwrap();
        let deep = create_classified(&repo, "PLC-2", AssetType::Device, Some(sub.id), None).unwrap();
        let high = create_classified(&repo, "PLC-3", AssetType::Device, Some(sub.id), Some(SecurityClassification::Secret)).unwrap();
        let unrelated = create_classified(&repo, "Office", AssetType::Folder, None, None).unwrap();

        let change = repo.update_asset_classification(root.id, SecurityClassification::Restricted).unwrap();
        assert_eq!(change.previous, SecurityClassification::Internal);
        let mut raised: Vec<(i64, SecurityClassification)> = change.raised.iter().map(|a| (a.id, a.security_classification)).collect();
        raised.sort();
        // Raised items are reported with the level they had before
        assert_eq!(raised, vec![
            (low.id, SecurityClassification::Internal),
            (sub.id, SecurityClassification::Confidential),
            (deep.id, SecurityClassification::Confidential),
        ]);

        assert_eq!(classification_of(&repo, root.id), SecurityClassification::Restricted);
        assert_eq!(classification_of(&repo, low.id), SecurityClassification::Restricted);
        assert_eq!(classification_of(&repo, sub.id), SecurityClassification::Restricted);
        assert_eq!(classification_of(&repo, deep.id), SecurityClassification::Restricted);
        assert_eq!(classification_of(&repo, high.id), SecurityClassification::Secret);
        assert_eq!(classification_of(&repo, unrelated.id), SecurityClassification::Internal);
    }

    #[test]
    fn test_update_classification_lowering_leaves_descendants() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssetRepository::new(&conn);

        let root = create_classified(&repo, "Plant", AssetType::Folder, None, Some(SecurityClassification::Secret)).unwrap();
        let child = create_classified(&repo, "PLC-1", AssetType::Device, Some(root.id), None).unwrap();

        let change = repo.update_asset_classification(root.id, SecurityClassification::Public).unwrap();
        assert!(change.raised.is_empty());
        assert_eq!(classification_of(&repo, root.id), SecurityClassification::Public);
        assert_eq!(classification_of(&repo, child.id), SecurityClassification::Secret);
    }
}