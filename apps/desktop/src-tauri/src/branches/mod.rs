use anyhow::Result;
use rusqlite::{Connection, Row, params};
use serde::{Deserialize, Serialize};
use tracing::{info, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub asset_id: i64,
    pub parent_version_id: i64,
    pub created_by: i64,
    pub created_at: String,
    pub updated_at: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub asset_id: i64,
    pub parent_version_id: i64,
    pub parent_version_number: String,
    pub parent_version_status: String,
    pub created_by: i64,
    pub created_by_username: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_active: bool,
    pub version_count: i64,
}

impl From<Branch> for BranchInfo {
    fn from(branch: Branch) -> Self {
        BranchInfo {
            id: branch.id,
            name: branch.name,
            description: branch.description,
            asset_id: branch.asset_id,
            parent_version_id: branch.parent_version_id,
            parent_version_number: String::new(), // Will be populated by join query
            parent_version_status: String::new(), // Will be populated by join query
            created_by: branch.created_by,
            created_by_username: String::new(), // Will be populated by join query
            created_at: branch.created_at,
            updated_at: branch.updated_at,
            is_active: branch.is_active,
            version_count: 0, // Will be populated by join query
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateBranchRequest {
    pub name: String,
    pub description: Option<String>,
    pub asset_id: i64,
    pub parent_version_id: i64,
    pub created_by: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchVersion {
    pub id: i64,
    pub branch_id: i64,
    pub version_id: i64,
    pub branch_version_number: String,
    pub is_latest: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchVersionInfo {
    pub id: i64,
    pub branch_id: i64,
    pub branch_name: String,
    pub version_id: i64,
    pub branch_version_number: String,
    pub is_latest: bool,
    pub is_branch_latest: bool,
    pub created_at: String,
    // Configuration version details
    pub asset_id: i64,
    pub version_number: String,
    pub file_name: String,
    pub file_size: i64,
    pub content_hash: String,
    pub author: i64,
    pub author_username: String,
    pub notes: String,
    pub version_created_at: String,
}

#[derive(Debug, Clone)]
pub struct CreateBranchVersionRequest {
    pub branch_id: i64,
    pub file_path: String,
    pub notes: String,
    pub author: i64,
}

pub trait BranchRepository {
    fn create_branch(&self, request: CreateBranchRequest) -> Result<Branch>;
    fn get_branches(&self, asset_id: i64) -> Result<Vec<BranchInfo>>;
    fn get_branch_by_id(&self, branch_id: i64) -> Result<Option<BranchInfo>>;
    fn update_branch(&self, branch_id: i64, name: Option<String>, description: Option<String>) -> Result<()>;
    fn delete_branch(&self, branch_id: i64) -> Result<()>;
    fn get_branch_count(&self, asset_id: i64) -> Result<i64>;
    fn validate_branch_name(&self, asset_id: i64, name: &str, exclude_id: Option<i64>) -> Result<bool>;
    
    // Branch version methods
    fn import_version_to_branch(&self, request: CreateBranchVersionRequest) -> Result<BranchVersion>;
    fn get_branch_versions(&self, branch_id: i64, page: Option<i32>, limit: Option<i32>) -> Result<Vec<BranchVersionInfo>>;
    fn get_branch_latest_version(&self, branch_id: i64) -> Result<Option<BranchVersionInfo>>;
    fn compare_branch_versions(&self, branch_id: i64, version1_id: i64, version2_id: i64) -> Result<Vec<u8>>;
    fn get_branch_version_count(&self, branch_id: i64) -> Result<i64>;
}

pub struct SqliteBranchRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteBranchRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS branches (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                asset_id INTEGER NOT NULL,
                parent_version_id INTEGER NOT NULL,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                is_active BOOLEAN DEFAULT 1,
                latest_version_id INTEGER,
                latest_branch_version TEXT,
                FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
                FOREIGN KEY (parent_version_id) REFERENCES configuration_versions(id),
                FOREIGN KEY (created_by) REFERENCES users(id),
                FOREIGN KEY (latest_version_id) REFERENCES configuration_versions(id),
                UNIQUE(asset_id, name)
            );

            CREATE TABLE IF NOT EXISTS branch_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                branch_id INTEGER NOT NULL,
                version_id INTEGER NOT NULL,
                branch_version_number TEXT NOT NULL,
                is_latest BOOLEAN DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (branch_id) REFERENCES branches(id) ON DELETE CASCADE,
                FOREIGN KEY (version_id) REFERENCES configuration_versions(id) ON DELETE CASCADE,
                UNIQUE(branch_id, version_id),
                UNIQUE(branch_id, branch_version_number)
            );

            CREATE INDEX IF NOT EXISTS idx_branches_asset_id ON branches(asset_id);
            CREATE INDEX IF NOT EXISTS idx_branches_parent_version ON branches(parent_version_id);
            CREATE INDEX IF NOT EXISTS idx_branches_created_by ON branches(created_by);
            CREATE INDEX IF NOT EXISTS idx_branches_created_at ON branches(created_at);
            CREATE INDEX IF NOT EXISTS idx_branches_active ON branches(is_active);
            CREATE INDEX IF NOT EXISTS idx_branches_latest_version ON branches(latest_version_id);
            
            CREATE INDEX IF NOT EXISTS idx_branch_versions_branch_id ON branch_versions(branch_id);
            CREATE INDEX IF NOT EXISTS idx_branch_versions_version_id ON branch_versions(version_id);
            CREATE INDEX IF NOT EXISTS idx_branch_versions_latest ON branch_versions(branch_id, is_latest);
            CREATE INDEX IF NOT EXISTS idx_branch_versions_created_at ON branch_versions(created_at);
            "#,
        )?;
        Ok(())
    }

    fn row_to_branch(row: &Row) -> rusqlite::Result<Branch> {
        Ok(Branch {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            asset_id: row.get("asset_id")?,
            parent_version_id: row.get("parent_version_id")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            is_active: row.get("is_active")?,
        })
    }

    fn row_to_branch_info(row: &Row) -> rusqlite::Result<BranchInfo> {
        Ok(BranchInfo {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            asset_id: row.get("asset_id")?,
            parent_version_id: row.get("parent_version_id")?,
            parent_version_number: row.get("parent_version_number")?,
            parent_version_status: row.get("parent_version_status")?,
            created_by: row.get("created_by")?,
            created_by_username: row.get("created_by_username")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            is_active: row.get("is_active")?,
            version_count: row.get("version_count").unwrap_or(0),
        })
    }

    fn row_to_branch_version(row: &Row) -> rusqlite::Result<BranchVersion> {
        Ok(BranchVersion {
            id: row.get("id")?,
            branch_id: row.get("branch_id")?,
            version_id: row.get("version_id")?,
            branch_version_number: row.get("branch_version_number")?,
            is_latest: row.get("is_latest")?,
            created_at: row.get("created_at")?,
        })
    }

    fn row_to_branch_version_info(row: &Row) -> rusqlite::Result<BranchVersionInfo> {
        Ok(BranchVersionInfo {
            id: row.get("id")?,
            branch_id: row.get("branch_id")?,
            branch_name: row.get("branch_name")?,
            version_id: row.get("version_id")?,
            branch_version_number: row.get("branch_version_number")?,
            is_latest: row.get("is_latest")?,
            is_branch_latest: row.get("is_branch_latest")?,
            created_at: row.get("created_at")?,
            asset_id: row.get("asset_id")?,
            version_number: row.get("version_number")?,
            file_name: row.get("file_name")?,
            file_size: row.get("file_size")?,
            content_hash: row.get("content_hash")?,
            author: row.get("author")?,
            author_username: row.get("author_username")?,
            notes: row.get("notes")?,
            version_created_at: row.get("version_created_at")?,
        })
    }

    fn validate_branch_name_input(&self, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow::anyhow!("Branch name cannot be empty"));
        }
        if name.len() < 2 {
            return Err(anyhow::anyhow!("Branch name must be at least 2 characters long"));
        }
        if name.len() > 100 {
            return Err(anyhow::anyhow!("Branch name cannot exceed 100 characters"));
        }
        
        // Check for invalid characters
        if name.contains('/') || name.contains('\\') || name.contains('\0') {
            return Err(anyhow::anyhow!("Branch name contains invalid characters"));
        }
        
        Ok(())
    }

    fn validate_description_input(&self, description: &Option<String>) -> Result<()> {
        if let Some(desc) = description {
            if desc.len() > 500 {
                return Err(anyhow::anyhow!("Branch description cannot exceed 500 characters"));
            }
        }
        Ok(())
    }

    fn generate_next_branch_version_number(&self, branch_id: i64) -> Result<String> {
        let mut stmt = self.conn.prepare(
            "SELECT branch_version_number FROM branch_versions WHERE branch_id = ?1 ORDER BY created_at DESC LIMIT 1"
        )?;

        let result = stmt.query_row([branch_id], |row| row.get::<_, String>(0));
        
        match result {
            Ok(last_version) => {
                // Parse version number (e.g., "branch-v1" -> 1, "branch-v2" -> 2)
                if let Some(version_num) = last_version.strip_prefix("branch-v") {
                    if let Ok(num) = version_num.parse::<i32>() {
                        return Ok(format!("branch-v{}", num + 1));
                    }
                }
                // If parsing fails, default to branch-v2
                Ok("branch-v2".to_string())
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok("branch-v1".to_string()),
            Err(e) => Err(e.into()),
        }
    }

    fn update_branch_latest_version(&self, branch_id: i64, version_id: i64, branch_version_number: &str) -> Result<()> {
        // Set all other versions in this branch to not latest
        self.conn.execute(
            "UPDATE branch_versions SET is_latest = 0 WHERE branch_id = ?1",
            [branch_id],
        )?;

        // Set the new version as latest
        self.conn.execute(
            "UPDATE branch_versions SET is_latest = 1 WHERE branch_id = ?1 AND version_id = ?2",
            params![branch_id, version_id],
        )?;

        // Update branch record with latest version info
        self.conn.execute(
            "UPDATE branches SET latest_version_id = ?1, latest_branch_version = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
            params![version_id, branch_version_number, branch_id],
        )?;

        Ok(())
    }

    fn copy_parent_version_to_branch(&self, branch_id: i64, parent_version_id: i64, created_by: i64) -> Result<BranchVersion> {
        use crate::configurations::{SqliteConfigurationRepository, ConfigurationRepository, CreateConfigurationRequest};
        
        // Get the parent version details and content
        let config_repo = SqliteConfigurationRepository::new(self.conn);
        
        let parent_config = config_repo.get_configuration_by_id(parent_version_id)?
            .ok_or_else(|| anyhow::anyhow!("Parent version not found"))?;
        
        let parent_content = config_repo.get_configuration_content(parent_version_id)?;
        
        // Get the asset_id from the branch
        let asset_id: i64 = self.conn.query_row(
            "SELECT asset_id FROM branches WHERE id = ?1",
            [branch_id],
            |row| row.get(0)
        )?;
        
        // Create a new configuration version as a copy of the parent
        let config_request = CreateConfigurationRequest {
            asset_id,
            file_name: parent_config.file_name.clone(),
            file_content: parent_content,
            author: created_by,
            notes: format!("Initial branch version created from parent version {}", parent_config.version_number),
        };
        
        let new_config = config_repo.store_configuration(config_request)?;
        
        // Create the branch version entry
        let branch_version_number = self.generate_next_branch_version_number(branch_id)?;
        
        let mut stmt = self.conn.prepare(
            "INSERT INTO branch_versions (branch_id, version_id, branch_version_number, is_latest)
             VALUES (?1, ?2, ?3, 1) RETURNING *"
        )?;
        
        let branch_version = stmt.query_row(
            params![branch_id, new_config.id, &branch_version_number],
            Self::row_to_branch_version,
        )?;
        
        // Update branch with latest version info
        self.update_branch_latest_version(branch_id, new_config.id, &branch_version_number)?;
        
        Ok(branch_version)
    }
}

impl<'a> BranchRepository for SqliteBranchRepository<'a> {
    fn create_branch(&self, request: CreateBranchRequest) -> Result<Branch> {
        // Validate inputs
        self.validate_branch_name_input(&request.name)?;
        self.validate_description_input(&request.description)?;
        
        // Check if branch name is unique for this asset
        if !self.validate_branch_name(request.asset_id, &request.name, None)? {
            return Err(anyhow::anyhow!("Branch name '{}' already exists for this asset", request.name));
        }
        
        // Verify parent version exists and is not archived
        let mut version_check_stmt = self.conn.prepare(
            "SELECT id, status FROM configuration_versions WHERE id = ?1 AND asset_id = ?2"
        )?;
        
        let version_status = version_check_stmt.query_row(
            [request.parent_version_id, request.asset_id],
            |row| {
                let status: String = row.get("status")?;
                Ok(status)
            }
        );
        
        match version_status {
            Ok(status) => {
                if status == "Archived" {
                    return Err(anyhow::anyhow!("Cannot create branch from archived version. Please select a non-archived version."));
                }
            },
            Err(_) => {
                return Err(anyhow::anyhow!("Parent version does not exist or does not belong to this asset"));
            }
        }
        
        // Create the branch
        let mut stmt = self.conn.prepare(
            "INSERT INTO branches (name, description, asset_id, parent_version_id, created_by) 
             VALUES (?1, ?2, ?3, ?4, ?5) RETURNING *"
        )?;

        let branch = stmt.query_row(
            (
                &request.name.trim(),
                &request.description,
                &request.asset_id,
                &request.parent_version_id,
                &request.created_by,
            ),
            Self::row_to_branch,
        )?;

        // Automatically create an initial branch version from the parent version
        let result = self.copy_parent_version_to_branch(branch.id, request.parent_version_id, request.created_by);
        
        match result {
            Ok(branch_version) => {
                info!("Created initial branch version {} from parent version {}", 
                    branch_version.branch_version_number, request.parent_version_id);
            }
            Err(e) => {
                error!("Failed to create initial branch version: {}", e);
                // We don't fail the branch creation if initial version fails
                // The user can still import versions manually
            }
        }

        Ok(branch)
    }

    fn get_branches(&self, asset_id: i64) -> Result<Vec<BranchInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT b.id, b.name, b.description, b.asset_id, b.parent_version_id, 
                    cv.version_number as parent_version_number, cv.status as parent_version_status, b.created_by, 
                    u.username as created_by_username, b.created_at, b.updated_at, b.is_active,
                    (SELECT COUNT(*) FROM branch_versions bv WHERE bv.branch_id = b.id) as version_count
             FROM branches b
             JOIN configuration_versions cv ON b.parent_version_id = cv.id
             JOIN users u ON b.created_by = u.id
             WHERE b.asset_id = ?1 AND b.is_active = 1
             ORDER BY b.created_at DESC"
        )?;

        let branch_iter = stmt.query_map([asset_id], Self::row_to_branch_info)?;
        let mut branches = Vec::new();

        for branch in branch_iter {
            branches.push(branch?);
        }

        Ok(branches)
    }

    fn get_branch_by_id(&self, branch_id: i64) -> Result<Option<BranchInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT b.id, b.name, b.description, b.asset_id, b.parent_version_id, 
                    cv.version_number as parent_version_number, cv.status as parent_version_status, b.created_by, 
                    u.username as created_by_username, b.created_at, b.updated_at, b.is_active,
                    (SELECT COUNT(*) FROM branch_versions bv WHERE bv.branch_id = b.id) as version_count
             FROM branches b
             JOIN configuration_versions cv ON b.parent_version_id = cv.id
             JOIN users u ON b.created_by = u.id
             WHERE b.id = ?1 AND b.is_active = 1"
        )?;

        let result = stmt.query_row([branch_id], Self::row_to_branch_info);
        
        match result {
            Ok(branch) => Ok(Some(branch)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn update_branch(&self, branch_id: i64, name: Option<String>, description: Option<String>) -> Result<()> {
        match (name, description) {
            (Some(name), Some(desc)) => {
                self.validate_branch_name_input(&name)?;
                self.validate_description_input(&Some(desc.clone()))?;
                
                let rows_affected = self.conn.execute(
                    "UPDATE branches SET name = ?1, description = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
                    params![name.trim(), desc.as_str(), branch_id],
                )?;
                
                if rows_affected == 0 {
                    return Err(anyhow::anyhow!("Branch not found"));
                }
            }
            (Some(name), None) => {
                self.validate_branch_name_input(&name)?;
                
                let rows_affected = self.conn.execute(
                    "UPDATE branches SET name = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                    params![name.trim(), branch_id],
                )?;
                
                if rows_affected == 0 {
                    return Err(anyhow::anyhow!("Branch not found"));
                }
            }
            (None, Some(desc)) => {
                self.validate_description_input(&Some(desc.clone()))?;
                
                let rows_affected = self.conn.execute(
                    "UPDATE branches SET description = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                    params![desc.as_str(), branch_id],
                )?;
                
                if rows_affected == 0 {
                    return Err(anyhow::anyhow!("Branch not found"));
                }
            }
            (None, None) => {
                // No updates needed
                return Ok(());
            }
        }

        Ok(())
    }

    fn delete_branch(&self, branch_id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE branches SET is_active = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [branch_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Branch not found"));
        }

        Ok(())
    }

    fn get_branch_count(&self, asset_id: i64) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM branches WHERE asset_id = ?1 AND is_active = 1"
        )?;
        let count: i64 = stmt.query_row([asset_id], |row| row.get(0))?;
        Ok(count)
    }

    fn validate_branch_name(&self, asset_id: i64, name: &str, exclude_id: Option<i64>) -> Result<bool> {
        let name = name.trim();
        
        let count: i64 = if let Some(exclude_id) = exclude_id {
            let mut stmt = self.conn.prepare(
                "SELECT COUNT(*) FROM branches WHERE asset_id = ?1 AND name = ?2 AND id != ?3 AND is_active = 1"
            )?;
            stmt.query_row(params![asset_id, name, exclude_id], |row| row.get(0))?
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT COUNT(*) FROM branches WHERE asset_id = ?1 AND name = ?2 AND is_active = 1"
            )?;
            stmt.query_row(params![asset_id, name], |row| row.get(0))?
        };
        
        Ok(count == 0)
    }

    fn import_version_to_branch(&self, request: CreateBranchVersionRequest) -> Result<BranchVersion> {
        // Validate that branch exists and get its asset_id
        let mut branch_stmt = self.conn.prepare(
            "SELECT asset_id FROM branches WHERE id = ?1 AND is_active = 1"
        )?;
        
        let asset_id: i64 = branch_stmt.query_row([request.branch_id], |row| row.get(0))
            .map_err(|_| anyhow::anyhow!("Branch not found or inactive"))?;

        // Import configuration file using existing configuration repository
        use crate::configurations::{SqliteConfigurationRepository, ConfigurationRepository, CreateConfigurationRequest, file_utils};
        
        // Read and validate file
        let file_content = file_utils::read_file_content(&request.file_path)?;
        let file_name = std::path::Path::new(&request.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Create configuration version
        let config_repo = SqliteConfigurationRepository::new(self.conn);
        let config_request = CreateConfigurationRequest {
            asset_id,
            file_name,
            file_content,
            author: request.author,
            notes: request.notes,
        };
        
        let config_version = config_repo.store_configuration(config_request)?;

        // Generate branch version number
        let branch_version_number = self.generate_next_branch_version_number(request.branch_id)?;

        // Create branch version record
        let mut stmt = self.conn.prepare(
            "INSERT INTO branch_versions (branch_id, version_id, branch_version_number, is_latest) 
             VALUES (?1, ?2, ?3, 1) RETURNING *"
        )?;

        let branch_version = stmt.query_row(
            (
                &request.branch_id,
                &config_version.id,
                &branch_version_number,
            ),
            Self::row_to_branch_version,
        )?;

        // Update branch latest version
        self.update_branch_latest_version(request.branch_id, config_version.id, &branch_version_number)?;

        Ok(branch_version)
    }

    fn get_branch_versions(&self, branch_id: i64, page: Option<i32>, limit: Option<i32>) -> Result<Vec<BranchVersionInfo>> {
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(50);
        let offset = (page - 1) * limit;

        let mut stmt = self.conn.prepare(
            "SELECT bv.id, bv.branch_id, b.name as branch_name, bv.version_id, 
                    bv.branch_version_number, bv.is_latest, 
                    (bv.is_latest = 1) as is_branch_latest, bv.created_at,
                    cv.asset_id, cv.version_number, cv.file_name, cv.file_size, 
                    cv.content_hash, cv.author, u.username as author_username, 
                    cv.notes, cv.created_at as version_created_at
             FROM branch_versions bv
             JOIN branches b ON bv.branch_id = b.id
             JOIN configuration_versions cv ON bv.version_id = cv.id
             JOIN users u ON cv.author = u.id
             WHERE bv.branch_id = ?1
             ORDER BY bv.created_at DESC
             LIMIT ?2 OFFSET ?3"
        )?;

        let version_iter = stmt.query_map(params![branch_id, limit, offset], Self::row_to_branch_version_info)?;
        let mut versions = Vec::new();

        for version in version_iter {
            versions.push(version?);
        }

        Ok(versions)
    }

    fn get_branch_latest_version(&self, branch_id: i64) -> Result<Option<BranchVersionInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT bv.id, bv.branch_id, b.name as branch_name, bv.version_id, 
                    bv.branch_version_number, bv.is_latest, 
                    (bv.is_latest = 1) as is_branch_latest, bv.created_at,
                    cv.asset_id, cv.version_number, cv.file_name, cv.file_size, 
                    cv.content_hash, cv.author, u.username as author_username, 
                    cv.notes, cv.created_at as version_created_at
             FROM branch_versions bv
             JOIN branches b ON bv.branch_id = b.id
             JOIN configuration_versions cv ON bv.version_id = cv.id
             JOIN users u ON cv.author = u.id
             WHERE bv.branch_id = ?1 AND bv.is_latest = 1"
        )?;

        let result = stmt.query_row([branch_id], Self::row_to_branch_version_info);
        
        match result {
            Ok(version) => Ok(Some(version)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn compare_branch_versions(&self, branch_id: i64, version1_id: i64, version2_id: i64) -> Result<Vec<u8>> {
        // Verify both versions belong to the branch
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM branch_versions WHERE branch_id = ?1 AND version_id IN (?2, ?3)"
        )?;
        
        let count: i64 = stmt.query_row(params![branch_id, version1_id, version2_id], |row| row.get(0))?;
        
        if count != 2 {
            return Err(anyhow::anyhow!("One or both versions do not belong to the specified branch"));
        }

        // Get configuration content for both versions
        use crate::configurations::{SqliteConfigurationRepository, ConfigurationRepository};
        let config_repo = SqliteConfigurationRepository::new(self.conn);
        
        let content1 = config_repo.get_configuration_content(version1_id)?;
        let content2 = config_repo.get_configuration_content(version2_id)?;

        // Convert to strings for diff comparison
        let content1_str = String::from_utf8_lossy(&content1);
        let content2_str = String::from_utf8_lossy(&content2);
        
        // Perform line-by-line diff
        let lines1: Vec<&str> = content1_str.lines().collect();
        let lines2: Vec<&str> = content2_str.lines().collect();
        
        let mut diff_result = Vec::new();
        diff_result.extend_from_slice(b"=== Configuration Version Comparison ===\n\n");
        
        // Simple line-by-line comparison
        let max_lines = lines1.len().max(lines2.len());
        let mut changes_found = false;
        
        for i in 0..max_lines {
            let line1 = lines1.get(i).unwrap_or(&"");
            let line2 = lines2.get(i).unwrap_or(&"");
            
            if line1 != line2 {
                changes_found = true;
                if !line1.is_empty() {
                    diff_result.extend_from_slice(format!("- Line {}: {}\n", i + 1, line1).as_bytes());
                }
                if !line2.is_empty() {
                    diff_result.extend_from_slice(format!("+ Line {}: {}\n", i + 1, line2).as_bytes());
                }
                diff_result.extend_from_slice(b"\n");
            }
        }
        
        if !changes_found {
            diff_result.extend_from_slice(b"No differences found between the two versions.\n");
        }
        
        diff_result.extend_from_slice(b"\n=== End of Comparison ===\n");
        Ok(diff_result)
    }

    fn get_branch_version_count(&self, branch_id: i64) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM branch_versions WHERE branch_id = ?1"
        )?;
        let count: i64 = stmt.query_row([branch_id], |row| row.get(0))?;
        Ok(count)
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
            
            CREATE TABLE configuration_versions (
                id INTEGER PRIMARY KEY,
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
                FOREIGN KEY (author) REFERENCES users(id)
            );
            
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'testuser', 'hash', 'Engineer');
            INSERT INTO assets (id, name, description, created_by) VALUES (1, 'Test Asset', 'Test Description', 1);
            INSERT INTO configuration_versions (id, asset_id, version_number, file_name, file_content, file_size, content_hash, author, notes) 
            VALUES (1, 1, 'v1', 'test.json', 'content', 7, 'hash', 1, 'Test version');
            "#,
        ).unwrap();
        
        let repo = SqliteBranchRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        // Initialize firmware schema for tests that use firmware functionality
        let firmware_repo = crate::firmware::SqliteFirmwareRepository::new(&conn);
        firmware_repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_branch_creation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBranchRepository::new(&conn);

        let request = CreateBranchRequest {
            name: "test-branch".to_string(),
            description: Some("Test branch description".to_string()),
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };

        let branch = repo.create_branch(request).unwrap();
        assert_eq!(branch.name, "test-branch");
        assert_eq!(branch.description, Some("Test branch description".to_string()));
        assert_eq!(branch.asset_id, 1);
        assert_eq!(branch.parent_version_id, 1);
        assert_eq!(branch.created_by, 1);
        assert!(branch.is_active);
    }

    #[test]
    fn test_branch_name_validation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBranchRepository::new(&conn);

        // Test empty name
        let request = CreateBranchRequest {
            name: "".to_string(),
            description: None,
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };
        assert!(repo.create_branch(request).is_err());

        // Test too long name
        let request = CreateBranchRequest {
            name: "a".repeat(101),
            description: None,
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };
        assert!(repo.create_branch(request).is_err());

        // Test invalid characters
        let request = CreateBranchRequest {
            name: "test/branch".to_string(),
            description: None,
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };
        assert!(repo.create_branch(request).is_err());
    }

    #[test]
    fn test_branch_name_uniqueness() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBranchRepository::new(&conn);

        let request1 = CreateBranchRequest {
            name: "unique-branch".to_string(),
            description: None,
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };
        repo.create_branch(request1).unwrap();

        let request2 = CreateBranchRequest {
            name: "unique-branch".to_string(),
            description: None,
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };
        assert!(repo.create_branch(request2).is_err());
    }

    #[test]
    fn test_get_branches() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBranchRepository::new(&conn);

        let request1 = CreateBranchRequest {
            name: "branch1".to_string(),
            description: Some("First branch".to_string()),
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };

        let request2 = CreateBranchRequest {
            name: "branch2".to_string(),
            description: Some("Second branch".to_string()),
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };

        repo.create_branch(request1).unwrap();
        repo.create_branch(request2).unwrap();

        let branches = repo.get_branches(1).unwrap();
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].parent_version_number, "v1");
        assert_eq!(branches[0].created_by_username, "testuser");
    }

    #[test]
    fn test_get_branch_by_id() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBranchRepository::new(&conn);

        let request = CreateBranchRequest {
            name: "test-branch".to_string(),
            description: Some("Test branch".to_string()),
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };

        let branch = repo.create_branch(request).unwrap();
        let retrieved_branch = repo.get_branch_by_id(branch.id).unwrap();
        
        assert!(retrieved_branch.is_some());
        let retrieved_branch = retrieved_branch.unwrap();
        assert_eq!(retrieved_branch.name, "test-branch");
        assert_eq!(retrieved_branch.parent_version_number, "v1");
        assert_eq!(retrieved_branch.created_by_username, "testuser");
    }

    #[test]
    fn test_branch_count() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBranchRepository::new(&conn);

        assert_eq!(repo.get_branch_count(1).unwrap(), 0);

        let request = CreateBranchRequest {
            name: "test-branch".to_string(),
            description: None,
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };

        repo.create_branch(request).unwrap();
        assert_eq!(repo.get_branch_count(1).unwrap(), 1);
    }

    #[test]
    fn test_delete_branch() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBranchRepository::new(&conn);

        let request = CreateBranchRequest {
            name: "test-branch".to_string(),
            description: None,
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };

        let branch = repo.create_branch(request).unwrap();
        assert_eq!(repo.get_branch_count(1).unwrap(), 1);

        repo.delete_branch(branch.id).unwrap();
        assert_eq!(repo.get_branch_count(1).unwrap(), 0);

        let retrieved_branch = repo.get_branch_by_id(branch.id).unwrap();
        assert!(retrieved_branch.is_none());
    }

    #[test]
    fn test_branch_version_numbering() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBranchRepository::new(&conn);

        // Create a branch first
        let branch_request = CreateBranchRequest {
            name: "version-test-branch".to_string(),
            description: None,
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };
        let branch = repo.create_branch(branch_request).unwrap();

        // Test version number generation
        assert_eq!(repo.generate_next_branch_version_number(branch.id).unwrap(), "branch-v1");
        
        // Insert a version manually to test incrementing
        conn.execute(
            "INSERT INTO branch_versions (branch_id, version_id, branch_version_number, is_latest) VALUES (?1, ?2, ?3, 1)",
            (branch.id, 1, "branch-v1"),
        ).unwrap();
        
        assert_eq!(repo.generate_next_branch_version_number(branch.id).unwrap(), "branch-v2");
    }

    #[test]
    fn test_get_branch_versions() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBranchRepository::new(&conn);

        // Create a branch
        let branch_request = CreateBranchRequest {
            name: "version-history-branch".to_string(),
            description: None,
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };
        let branch = repo.create_branch(branch_request).unwrap();

        // Insert some branch versions
        conn.execute(
            "INSERT INTO branch_versions (branch_id, version_id, branch_version_number, is_latest) VALUES (?1, ?2, ?3, 1)",
            (branch.id, 1, "branch-v1"),
        ).unwrap();

        let versions = repo.get_branch_versions(branch.id, None, None).unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].branch_version_number, "branch-v1");
        assert_eq!(versions[0].branch_name, "version-history-branch");
        assert!(versions[0].is_branch_latest);
    }

    #[test]
    fn test_get_branch_latest_version() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBranchRepository::new(&conn);

        // Create a branch
        let branch_request = CreateBranchRequest {
            name: "latest-version-branch".to_string(),
            description: None,
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };
        let branch = repo.create_branch(branch_request).unwrap();

        // No versions initially
        let latest = repo.get_branch_latest_version(branch.id).unwrap();
        assert!(latest.is_none());

        // Insert a version
        conn.execute(
            "INSERT INTO branch_versions (branch_id, version_id, branch_version_number, is_latest) VALUES (?1, ?2, ?3, 1)",
            (branch.id, 1, "branch-v1"),
        ).unwrap();

        let latest = repo.get_branch_latest_version(branch.id).unwrap();
        assert!(latest.is_some());
        let latest = latest.unwrap();
        assert_eq!(latest.branch_version_number, "branch-v1");
        assert!(latest.is_branch_latest);
    }

    #[test]
    fn test_branch_version_count() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBranchRepository::new(&conn);

        // Create a branch
        let branch_request = CreateBranchRequest {
            name: "count-test-branch".to_string(),
            description: None,
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };
        let branch = repo.create_branch(branch_request).unwrap();

        // Initially no versions
        assert_eq!(repo.get_branch_version_count(branch.id).unwrap(), 0);

        // Add a version
        conn.execute(
            "INSERT INTO branch_versions (branch_id, version_id, branch_version_number, is_latest) VALUES (?1, ?2, ?3, 1)",
            (branch.id, 1, "branch-v1"),
        ).unwrap();

        assert_eq!(repo.get_branch_version_count(branch.id).unwrap(), 1);
    }

    #[test]
    fn test_compare_branch_versions_validation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBranchRepository::new(&conn);

        // Create a branch
        let branch_request = CreateBranchRequest {
            name: "compare-test-branch".to_string(),
            description: None,
            asset_id: 1,
            parent_version_id: 1,
            created_by: 1,
        };
        let branch = repo.create_branch(branch_request).unwrap();

        // Test comparing versions that don't belong to the branch
        let result = repo.compare_branch_versions(branch.id, 1, 999);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("do not belong to the specified branch"));
    }
}