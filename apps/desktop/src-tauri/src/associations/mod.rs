use anyhow::Result;
use rusqlite::{Connection, Row, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFileAssociation {
    pub id: i64,
    pub asset_id: i64,
    pub file_id: i64,
    pub file_type: AssociationType,
    pub association_order: i64,
    pub metadata: Option<String>,
    pub created_by: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssociationType {
    Configuration,
    Firmware,
}

impl AssociationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssociationType::Configuration => "configuration",
            AssociationType::Firmware => "firmware",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "configuration" => Ok(AssociationType::Configuration),
            "firmware" => Ok(AssociationType::Firmware),
            _ => Err(anyhow::anyhow!("Invalid association type: {}", s)),
        }
    }
}

impl std::fmt::Display for AssociationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileImportSession {
    pub id: i64,
    pub session_name: String,
    pub asset_id: i64,
    pub file_paths: Vec<String>,
    pub import_status: ImportStatus,
    pub validation_results: Option<String>,
    pub created_by: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImportStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl ImportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImportStatus::Pending => "pending",
            ImportStatus::InProgress => "in_progress",
            ImportStatus::Completed => "completed",
            ImportStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(ImportStatus::Pending),
            "in_progress" => Ok(ImportStatus::InProgress),
            "completed" => Ok(ImportStatus::Completed),
            "failed" => Ok(ImportStatus::Failed),
            _ => Err(anyhow::anyhow!("Invalid import status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociationValidation {
    pub id: i64,
    pub association_id: i64,
    pub validation_type: ValidationType,
    pub validation_result: ValidationResult,
    pub validation_message: String,
    pub validated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationType {
    SecurityClassification,
    FileTypeCompatibility,
    AssetTypeCompatibility,
    DuplicateCheck,
    ReferentialIntegrity,
}

impl ValidationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ValidationType::SecurityClassification => "security_classification",
            ValidationType::FileTypeCompatibility => "file_type_compatibility",
            ValidationType::AssetTypeCompatibility => "asset_type_compatibility",
            ValidationType::DuplicateCheck => "duplicate_check",
            ValidationType::ReferentialIntegrity => "referential_integrity",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "security_classification" => Ok(ValidationType::SecurityClassification),
            "file_type_compatibility" => Ok(ValidationType::FileTypeCompatibility),
            "asset_type_compatibility" => Ok(ValidationType::AssetTypeCompatibility),
            "duplicate_check" => Ok(ValidationType::DuplicateCheck),
            "referential_integrity" => Ok(ValidationType::ReferentialIntegrity),
            _ => Err(anyhow::anyhow!("Invalid validation type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationResult {
    Passed,
    Failed,
    Warning,
}

impl ValidationResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            ValidationResult::Passed => "passed",
            ValidationResult::Failed => "failed",
            ValidationResult::Warning => "warning",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "passed" => Ok(ValidationResult::Passed),
            "failed" => Ok(ValidationResult::Failed),
            "warning" => Ok(ValidationResult::Warning),
            _ => Err(anyhow::anyhow!("Invalid validation result: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociationInfo {
    pub id: i64,
    pub asset_id: i64,
    pub asset_name: String,
    pub file_id: i64,
    pub file_name: String,
    pub file_type: AssociationType,
    pub association_order: i64,
    pub metadata: Option<String>,
    pub created_by: i64,
    pub created_by_username: String,
    pub created_at: String,
    pub validation_status: ValidationResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociationSummary {
    pub asset_id: i64,
    pub asset_name: String,
    pub configuration_count: i64,
    pub firmware_count: i64,
    pub total_associations: i64,
    pub validation_issues: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
    pub last_checked: String,
}

#[derive(Debug, Clone)]
pub struct CreateAssociationRequest {
    pub asset_id: i64,
    pub file_id: i64,
    pub file_type: AssociationType,
    pub metadata: Option<String>,
    pub created_by: i64,
}

pub trait AssociationRepository {
    fn create_file_association(&self, request: CreateAssociationRequest) -> Result<AssetFileAssociation>;
    fn get_asset_associations(&self, asset_id: i64) -> Result<Vec<AssociationInfo>>;
    fn get_association_by_id(&self, association_id: i64) -> Result<Option<AssetFileAssociation>>;
    fn remove_association(&self, association_id: i64) -> Result<()>;
    fn reorder_associations(&self, asset_id: i64, association_order: Vec<(i64, i64)>) -> Result<()>;
    fn validate_file_association(&self, asset_id: i64, file_id: i64, file_type: &AssociationType) -> Result<Vec<AssociationValidation>>;
    fn get_association_health_status(&self, asset_id: i64) -> Result<HealthStatus>;
    fn get_broken_associations(&self) -> Result<Vec<AssociationInfo>>;
    fn repair_association(&self, association_id: i64) -> Result<()>;
    
    // File import session management
    fn create_import_session(&self, session_name: String, asset_id: i64, file_paths: Vec<String>, created_by: i64) -> Result<FileImportSession>;
    fn update_import_session_status(&self, session_id: i64, status: ImportStatus, validation_results: Option<String>) -> Result<()>;
    fn get_import_session(&self, session_id: i64) -> Result<Option<FileImportSession>>;
    
    // Search and filtering
    fn search_associations(&self, query: String, file_type: Option<AssociationType>) -> Result<Vec<AssociationInfo>>;
    fn get_associations_by_validation_status(&self, status: ValidationResult) -> Result<Vec<AssociationInfo>>;
}

pub struct SqliteAssociationRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteAssociationRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS asset_file_associations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                asset_id INTEGER NOT NULL,
                file_id INTEGER NOT NULL,
                file_type TEXT NOT NULL CHECK(file_type IN ('configuration', 'firmware')),
                association_order INTEGER NOT NULL DEFAULT 0,
                metadata TEXT,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
                FOREIGN KEY (created_by) REFERENCES users(id),
                UNIQUE(asset_id, file_id, file_type)
            );

            CREATE TABLE IF NOT EXISTS file_import_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_name TEXT NOT NULL,
                asset_id INTEGER NOT NULL,
                file_paths TEXT NOT NULL, -- JSON array of file paths
                import_status TEXT NOT NULL CHECK(import_status IN ('pending', 'in_progress', 'completed', 'failed')),
                validation_results TEXT, -- JSON validation results
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
                FOREIGN KEY (created_by) REFERENCES users(id)
            );

            CREATE TABLE IF NOT EXISTS association_validations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                association_id INTEGER NOT NULL,
                validation_type TEXT NOT NULL CHECK(validation_type IN ('security_classification', 'file_type_compatibility', 'asset_type_compatibility', 'duplicate_check', 'referential_integrity')),
                validation_result TEXT NOT NULL CHECK(validation_result IN ('passed', 'failed', 'warning')),
                validation_message TEXT NOT NULL,
                validated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (association_id) REFERENCES asset_file_associations(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_associations_asset_id ON asset_file_associations(asset_id);
            CREATE INDEX IF NOT EXISTS idx_associations_file_id ON asset_file_associations(file_id);
            CREATE INDEX IF NOT EXISTS idx_associations_file_type ON asset_file_associations(file_type);
            CREATE INDEX IF NOT EXISTS idx_associations_created_by ON asset_file_associations(created_by);
            CREATE INDEX IF NOT EXISTS idx_associations_created_at ON asset_file_associations(created_at);
            CREATE INDEX IF NOT EXISTS idx_associations_order ON asset_file_associations(asset_id, association_order);
            
            CREATE INDEX IF NOT EXISTS idx_import_sessions_asset_id ON file_import_sessions(asset_id);
            CREATE INDEX IF NOT EXISTS idx_import_sessions_status ON file_import_sessions(import_status);
            CREATE INDEX IF NOT EXISTS idx_import_sessions_created_by ON file_import_sessions(created_by);
            
            CREATE INDEX IF NOT EXISTS idx_validations_association_id ON association_validations(association_id);
            CREATE INDEX IF NOT EXISTS idx_validations_result ON association_validations(validation_result);
            CREATE INDEX IF NOT EXISTS idx_validations_type ON association_validations(validation_type);
            "#,
        )?;
        Ok(())
    }

    fn row_to_association(row: &Row) -> rusqlite::Result<AssetFileAssociation> {
        let file_type_str: String = row.get("file_type")?;
        let file_type = AssociationType::from_str(&file_type_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "file_type".to_string(), rusqlite::types::Type::Text))?;
            
        Ok(AssetFileAssociation {
            id: row.get("id")?,
            asset_id: row.get("asset_id")?,
            file_id: row.get("file_id")?,
            file_type,
            association_order: row.get("association_order")?,
            metadata: row.get("metadata")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    fn row_to_association_info(row: &Row) -> rusqlite::Result<AssociationInfo> {
        let file_type_str: String = row.get("file_type")?;
        let file_type = AssociationType::from_str(&file_type_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "file_type".to_string(), rusqlite::types::Type::Text))?;
            
        let validation_status_str: String = row.get("validation_status")?;
        let validation_status = ValidationResult::from_str(&validation_status_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "validation_status".to_string(), rusqlite::types::Type::Text))?;
            
        Ok(AssociationInfo {
            id: row.get("id")?,
            asset_id: row.get("asset_id")?,
            asset_name: row.get("asset_name")?,
            file_id: row.get("file_id")?,
            file_name: row.get("file_name")?,
            file_type,
            association_order: row.get("association_order")?,
            metadata: row.get("metadata")?,
            created_by: row.get("created_by")?,
            created_by_username: row.get("created_by_username")?,
            created_at: row.get("created_at")?,
            validation_status,
        })
    }

    fn row_to_import_session(row: &Row) -> rusqlite::Result<FileImportSession> {
        let import_status_str: String = row.get("import_status")?;
        let import_status = ImportStatus::from_str(&import_status_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "import_status".to_string(), rusqlite::types::Type::Text))?;
            
        let file_paths_json: String = row.get("file_paths")?;
        let file_paths: Vec<String> = serde_json::from_str(&file_paths_json)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "file_paths".to_string(), rusqlite::types::Type::Text))?;
            
        Ok(FileImportSession {
            id: row.get("id")?,
            session_name: row.get("session_name")?,
            asset_id: row.get("asset_id")?,
            file_paths,
            import_status,
            validation_results: row.get("validation_results")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
        })
    }

    fn row_to_validation(row: &Row) -> rusqlite::Result<AssociationValidation> {
        let validation_type_str: String = row.get("validation_type")?;
        let validation_type = ValidationType::from_str(&validation_type_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "validation_type".to_string(), rusqlite::types::Type::Text))?;
            
        let validation_result_str: String = row.get("validation_result")?;
        let validation_result = ValidationResult::from_str(&validation_result_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "validation_result".to_string(), rusqlite::types::Type::Text))?;
            
        Ok(AssociationValidation {
            id: row.get("id")?,
            association_id: row.get("association_id")?,
            validation_type,
            validation_result,
            validation_message: row.get("validation_message")?,
            validated_at: row.get("validated_at")?,
        })
    }

    fn get_next_association_order(&self, asset_id: i64) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(MAX(association_order), -1) + 1 FROM asset_file_associations WHERE asset_id = ?1"
        )?;
        
        let next_order: i64 = stmt.query_row([asset_id], |row| row.get(0))?;
        Ok(next_order)
    }

    fn validate_association_constraints(&self, request: &CreateAssociationRequest) -> Result<()> {
        // Check if asset exists
        let mut stmt = self.conn.prepare("SELECT id FROM assets WHERE id = ?1")?;
        let asset_exists = stmt.query_row([request.asset_id], |_| Ok(()))
            .optional()?
            .is_some();
        
        if !asset_exists {
            return Err(anyhow::anyhow!("Asset with ID {} does not exist", request.asset_id));
        }

        // Check if file exists based on type
        match request.file_type {
            AssociationType::Configuration => {
                let mut stmt = self.conn.prepare("SELECT id FROM configuration_versions WHERE id = ?1")?;
                let file_exists = stmt.query_row([request.file_id], |_| Ok(()))
                    .optional()?
                    .is_some();
                
                if !file_exists {
                    return Err(anyhow::anyhow!("Configuration file with ID {} does not exist", request.file_id));
                }
            }
            AssociationType::Firmware => {
                let mut stmt = self.conn.prepare("SELECT id FROM firmware_versions WHERE id = ?1")?;
                let file_exists = stmt.query_row([request.file_id], |_| Ok(()))
                    .optional()?
                    .is_some();
                
                if !file_exists {
                    return Err(anyhow::anyhow!("Firmware file with ID {} does not exist", request.file_id));
                }
            }
        }

        // Check for duplicate association
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM asset_file_associations WHERE asset_id = ?1 AND file_id = ?2 AND file_type = ?3"
        )?;
        let count: i64 = stmt.query_row((request.asset_id, request.file_id, request.file_type.as_str()), |row| row.get(0))?;
        
        if count > 0 {
            return Err(anyhow::anyhow!("Association already exists for this asset and file"));
        }

        Ok(())
    }
}

impl<'a> AssociationRepository for SqliteAssociationRepository<'a> {
    fn create_file_association(&self, request: CreateAssociationRequest) -> Result<AssetFileAssociation> {
        // Validate constraints
        self.validate_association_constraints(&request)?;
        
        // Get next association order
        let association_order = self.get_next_association_order(request.asset_id)?;
        
        // Create the association
        let mut stmt = self.conn.prepare(
            "INSERT INTO asset_file_associations (asset_id, file_id, file_type, association_order, metadata, created_by) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING *"
        )?;

        let association = stmt.query_row(
            (
                &request.asset_id,
                &request.file_id,
                request.file_type.as_str(),
                &association_order,
                &request.metadata,
                &request.created_by,
            ),
            Self::row_to_association,
        )?;

        Ok(association)
    }

    fn get_asset_associations(&self, asset_id: i64) -> Result<Vec<AssociationInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT afa.id, afa.asset_id, a.name as asset_name, afa.file_id, 
                    CASE 
                        WHEN afa.file_type = 'configuration' THEN cv.file_name
                        WHEN afa.file_type = 'firmware' THEN 'firmware_' || fv.version || '.bin'
                        ELSE 'unknown'
                    END as file_name,
                    afa.file_type, afa.association_order, afa.metadata, 
                    afa.created_by, u.username as created_by_username, afa.created_at,
                    COALESCE(
                        (SELECT CASE 
                            WHEN COUNT(CASE WHEN validation_result = 'failed' THEN 1 END) > 0 THEN 'failed'
                            WHEN COUNT(CASE WHEN validation_result = 'warning' THEN 1 END) > 0 THEN 'warning'
                            ELSE 'passed'
                        END FROM association_validations WHERE association_id = afa.id),
                        'passed'
                    ) as validation_status
             FROM asset_file_associations afa
             JOIN assets a ON afa.asset_id = a.id
             JOIN users u ON afa.created_by = u.id
             LEFT JOIN configuration_versions cv ON afa.file_type = 'configuration' AND afa.file_id = cv.id
             LEFT JOIN firmware_versions fv ON afa.file_type = 'firmware' AND afa.file_id = fv.id
             WHERE afa.asset_id = ?1
             ORDER BY afa.association_order, afa.created_at"
        )?;

        let association_iter = stmt.query_map([asset_id], Self::row_to_association_info)?;
        let mut associations = Vec::new();

        for association in association_iter {
            associations.push(association?);
        }

        Ok(associations)
    }

    fn get_association_by_id(&self, association_id: i64) -> Result<Option<AssetFileAssociation>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, asset_id, file_id, file_type, association_order, metadata, created_by, created_at, updated_at
             FROM asset_file_associations WHERE id = ?1"
        )?;

        let result = stmt.query_row([association_id], Self::row_to_association);
        
        match result {
            Ok(association) => Ok(Some(association)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn remove_association(&self, association_id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM asset_file_associations WHERE id = ?1",
            [association_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Association not found"));
        }

        Ok(())
    }

    fn reorder_associations(&self, asset_id: i64, association_order: Vec<(i64, i64)>) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;

        for (association_id, new_order) in association_order {
            // Verify association belongs to the specified asset
            let count: i64 = tx.prepare(
                "SELECT COUNT(*) FROM asset_file_associations WHERE id = ?1 AND asset_id = ?2"
            )?.query_row((association_id, asset_id), |row| row.get(0))?;
            
            if count == 0 {
                return Err(anyhow::anyhow!("Association {} does not belong to asset {}", association_id, asset_id));
            }

            // Update the order
            tx.execute(
                "UPDATE asset_file_associations SET association_order = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                (new_order, association_id),
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    fn validate_file_association(&self, asset_id: i64, file_id: i64, file_type: &AssociationType) -> Result<Vec<AssociationValidation>> {
        let mut validations = Vec::new();
        
        // This is a placeholder implementation
        // In a real implementation, you would run various validation checks
        
        // For now, create a basic validation entry indicating success
        let validation = AssociationValidation {
            id: 0, // This would be assigned by the database
            association_id: 0, // This would be the actual association ID
            validation_type: ValidationType::ReferentialIntegrity,
            validation_result: ValidationResult::Passed,
            validation_message: "Basic validation passed".to_string(),
            validated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        
        validations.push(validation);
        Ok(validations)
    }

    fn get_association_health_status(&self, asset_id: i64) -> Result<HealthStatus> {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();
        
        // Check for broken references
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM asset_file_associations afa
             LEFT JOIN configuration_versions cv ON afa.file_type = 'configuration' AND afa.file_id = cv.id
             LEFT JOIN firmware_versions fv ON afa.file_type = 'firmware' AND afa.file_id = fv.id
             WHERE afa.asset_id = ?1 AND cv.id IS NULL AND fv.id IS NULL"
        )?;
        let broken_refs: i64 = stmt.query_row([asset_id], |row| row.get(0))?;
        
        if broken_refs > 0 {
            issues.push(format!("{} broken file reference(s)", broken_refs));
        }

        // Check for validation failures
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM asset_file_associations afa
             JOIN association_validations av ON afa.id = av.association_id
             WHERE afa.asset_id = ?1 AND av.validation_result = 'failed'"
        )?;
        let failed_validations: i64 = stmt.query_row([asset_id], |row| row.get(0))?;
        
        if failed_validations > 0 {
            issues.push(format!("{} validation failure(s)", failed_validations));
        }

        // Check for validation warnings
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM asset_file_associations afa
             JOIN association_validations av ON afa.id = av.association_id
             WHERE afa.asset_id = ?1 AND av.validation_result = 'warning'"
        )?;
        let warning_validations: i64 = stmt.query_row([asset_id], |row| row.get(0))?;
        
        if warning_validations > 0 {
            warnings.push(format!("{} validation warning(s)", warning_validations));
        }

        Ok(HealthStatus {
            healthy: issues.is_empty(),
            issues,
            warnings,
            last_checked: "2024-01-01T00:00:00Z".to_string(),
        })
    }

    fn get_broken_associations(&self) -> Result<Vec<AssociationInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT afa.id, afa.asset_id, a.name as asset_name, afa.file_id, 
                    'broken_reference' as file_name,
                    afa.file_type, afa.association_order, afa.metadata, 
                    afa.created_by, u.username as created_by_username, afa.created_at,
                    'failed' as validation_status
             FROM asset_file_associations afa
             JOIN assets a ON afa.asset_id = a.id
             JOIN users u ON afa.created_by = u.id
             LEFT JOIN configuration_versions cv ON afa.file_type = 'configuration' AND afa.file_id = cv.id
             LEFT JOIN firmware_versions fv ON afa.file_type = 'firmware' AND afa.file_id = fv.id
             WHERE cv.id IS NULL AND fv.id IS NULL
             ORDER BY afa.created_at DESC"
        )?;

        let association_iter = stmt.query_map([], Self::row_to_association_info)?;
        let mut associations = Vec::new();

        for association in association_iter {
            associations.push(association?);
        }

        Ok(associations)
    }

    fn repair_association(&self, association_id: i64) -> Result<()> {
        // This is a placeholder implementation
        // In a real implementation, you would attempt to repair the association
        // For now, we'll just remove broken associations
        
        // Check if the association is broken
        let mut stmt = self.conn.prepare(
            "SELECT afa.file_type, afa.file_id FROM asset_file_associations afa
             LEFT JOIN configuration_versions cv ON afa.file_type = 'configuration' AND afa.file_id = cv.id
             LEFT JOIN firmware_versions fv ON afa.file_type = 'firmware' AND afa.file_id = fv.id
             WHERE afa.id = ?1 AND cv.id IS NULL AND fv.id IS NULL"
        )?;

        let is_broken = stmt.query_row([association_id], |_| Ok(()))
            .optional()?
            .is_some();

        if is_broken {
            // Remove the broken association
            self.remove_association(association_id)?;
        }

        Ok(())
    }

    fn create_import_session(&self, session_name: String, asset_id: i64, file_paths: Vec<String>, created_by: i64) -> Result<FileImportSession> {
        let file_paths_json = serde_json::to_string(&file_paths)?;
        
        let mut stmt = self.conn.prepare(
            "INSERT INTO file_import_sessions (session_name, asset_id, file_paths, import_status, created_by) 
             VALUES (?1, ?2, ?3, ?4, ?5) RETURNING *"
        )?;

        let session = stmt.query_row(
            (&session_name, &asset_id, &file_paths_json, ImportStatus::Pending.as_str(), &created_by),
            Self::row_to_import_session,
        )?;

        Ok(session)
    }

    fn update_import_session_status(&self, session_id: i64, status: ImportStatus, validation_results: Option<String>) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE file_import_sessions SET import_status = ?1, validation_results = ?2 WHERE id = ?3",
            (status.as_str(), &validation_results, session_id),
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Import session not found"));
        }

        Ok(())
    }

    fn get_import_session(&self, session_id: i64) -> Result<Option<FileImportSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_name, asset_id, file_paths, import_status, validation_results, created_by, created_at
             FROM file_import_sessions WHERE id = ?1"
        )?;

        let result = stmt.query_row([session_id], Self::row_to_import_session);
        
        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn search_associations(&self, query: String, file_type: Option<AssociationType>) -> Result<Vec<AssociationInfo>> {
        let mut sql = String::from(
            "SELECT afa.id, afa.asset_id, a.name as asset_name, afa.file_id, 
                    CASE 
                        WHEN afa.file_type = 'configuration' THEN cv.file_name
                        WHEN afa.file_type = 'firmware' THEN 'firmware_' || fv.version || '.bin'
                        ELSE 'unknown'
                    END as file_name,
                    afa.file_type, afa.association_order, afa.metadata, 
                    afa.created_by, u.username as created_by_username, afa.created_at,
                    'passed' as validation_status
             FROM asset_file_associations afa
             JOIN assets a ON afa.asset_id = a.id
             JOIN users u ON afa.created_by = u.id
             LEFT JOIN configuration_versions cv ON afa.file_type = 'configuration' AND afa.file_id = cv.id
             LEFT JOIN firmware_versions fv ON afa.file_type = 'firmware' AND afa.file_id = fv.id
             WHERE (a.name LIKE ?1 OR a.description LIKE ?1 OR cv.file_name LIKE ?1 OR fv.version LIKE ?1)"
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(format!("%{}%", query))];

        if let Some(ft) = file_type {
            sql.push_str(" AND afa.file_type = ?2");
            params.push(Box::new(ft.as_str().to_string()));
        }

        sql.push_str(" ORDER BY afa.created_at DESC");

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        
        let association_iter = stmt.query_map(param_refs.as_slice(), Self::row_to_association_info)?;
        let mut associations = Vec::new();

        for association in association_iter {
            associations.push(association?);
        }

        Ok(associations)
    }

    fn get_associations_by_validation_status(&self, status: ValidationResult) -> Result<Vec<AssociationInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT afa.id, afa.asset_id, a.name as asset_name, afa.file_id, 
                    CASE 
                        WHEN afa.file_type = 'configuration' THEN cv.file_name
                        WHEN afa.file_type = 'firmware' THEN 'firmware_' || fv.version || '.bin'
                        ELSE 'unknown'
                    END as file_name,
                    afa.file_type, afa.association_order, afa.metadata, 
                    afa.created_by, u.username as created_by_username, afa.created_at,
                    ?1 as validation_status
             FROM asset_file_associations afa
             JOIN assets a ON afa.asset_id = a.id
             JOIN users u ON afa.created_by = u.id
             LEFT JOIN configuration_versions cv ON afa.file_type = 'configuration' AND afa.file_id = cv.id
             LEFT JOIN firmware_versions fv ON afa.file_type = 'firmware' AND afa.file_id = fv.id
             JOIN association_validations av ON afa.id = av.association_id
             WHERE av.validation_result = ?1
             ORDER BY afa.created_at DESC"
        )?;

        let association_iter = stmt.query_map([status.as_str()], Self::row_to_association_info)?;
        let mut associations = Vec::new();

        for association in association_iter {
            associations.push(association?);
        }

        Ok(associations)
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
                asset_type TEXT NOT NULL DEFAULT 'device',
                parent_id INTEGER,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES users(id),
                FOREIGN KEY (parent_id) REFERENCES assets(id) ON DELETE CASCADE
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

            CREATE TABLE firmware_versions (
                id INTEGER PRIMARY KEY,
                asset_id INTEGER NOT NULL,
                author_id INTEGER NOT NULL,
                version TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Draft',
                file_path TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
                FOREIGN KEY (author_id) REFERENCES users(id)
            );
            
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'testuser', 'hash', 'Engineer');
            INSERT INTO assets (id, name, description, created_by) VALUES (1, 'Test Asset', 'Test Description', 1);
            INSERT INTO configuration_versions (id, asset_id, version_number, file_name, file_content, file_size, content_hash, author) 
                VALUES (1, 1, 'v1', 'config.json', 'test', 4, 'hash1', 1);
            INSERT INTO firmware_versions (id, asset_id, author_id, version, file_path, file_hash, file_size) 
                VALUES (1, 1, 1, '1.0.0', '/test/path', 'hash2', 1024);
            "#,
        ).unwrap();
        
        let repo = SqliteAssociationRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_create_configuration_association() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssociationRepository::new(&conn);

        let request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Configuration,
            metadata: Some("Test association".to_string()),
            created_by: 1,
        };

        let association = repo.create_file_association(request).unwrap();
        assert_eq!(association.asset_id, 1);
        assert_eq!(association.file_id, 1);
        assert_eq!(association.file_type, AssociationType::Configuration);
        assert_eq!(association.association_order, 0);
        assert_eq!(association.created_by, 1);
    }

    #[test]
    fn test_create_firmware_association() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssociationRepository::new(&conn);

        let request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Firmware,
            metadata: None,
            created_by: 1,
        };

        let association = repo.create_file_association(request).unwrap();
        assert_eq!(association.file_type, AssociationType::Firmware);
    }

    #[test]
    fn test_duplicate_association_prevention() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssociationRepository::new(&conn);

        let request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Configuration,
            metadata: None,
            created_by: 1,
        };

        // First association should succeed
        repo.create_file_association(request.clone()).unwrap();

        // Second association should fail
        let result = repo.create_file_association(request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Association already exists"));
    }

    #[test]
    fn test_association_ordering() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssociationRepository::new(&conn);

        // Create configuration and firmware associations
        let config_request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Configuration,
            metadata: None,
            created_by: 1,
        };

        let firmware_request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Firmware,
            metadata: None,
            created_by: 1,
        };

        let config_assoc = repo.create_file_association(config_request).unwrap();
        let firmware_assoc = repo.create_file_association(firmware_request).unwrap();

        assert_eq!(config_assoc.association_order, 0);
        assert_eq!(firmware_assoc.association_order, 1);
    }

    #[test]
    fn test_get_asset_associations() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssociationRepository::new(&conn);

        // Create associations
        let config_request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Configuration,
            metadata: Some("Config metadata".to_string()),
            created_by: 1,
        };

        let firmware_request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Firmware,
            metadata: Some("Firmware metadata".to_string()),
            created_by: 1,
        };

        repo.create_file_association(config_request).unwrap();
        repo.create_file_association(firmware_request).unwrap();

        let associations = repo.get_asset_associations(1).unwrap();
        assert_eq!(associations.len(), 2);
        
        let config_assoc = associations.iter().find(|a| a.file_type == AssociationType::Configuration).unwrap();
        assert_eq!(config_assoc.file_name, "config.json");
        assert_eq!(config_assoc.metadata.as_ref().unwrap(), "Config metadata");
        
        let firmware_assoc = associations.iter().find(|a| a.file_type == AssociationType::Firmware).unwrap();
        assert_eq!(firmware_assoc.file_name, "firmware_1.0.0.bin");
        assert_eq!(firmware_assoc.metadata.as_ref().unwrap(), "Firmware metadata");
    }

    #[test]
    fn test_remove_association() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssociationRepository::new(&conn);

        let request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Configuration,
            metadata: None,
            created_by: 1,
        };

        let association = repo.create_file_association(request).unwrap();
        
        // Verify association exists
        let associations = repo.get_asset_associations(1).unwrap();
        assert_eq!(associations.len(), 1);

        // Remove association
        repo.remove_association(association.id).unwrap();

        // Verify association is removed
        let associations = repo.get_asset_associations(1).unwrap();
        assert_eq!(associations.len(), 0);
    }

    #[test]
    fn test_reorder_associations() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssociationRepository::new(&conn);

        // Create two associations
        let config_request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Configuration,
            metadata: None,
            created_by: 1,
        };

        let firmware_request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Firmware,
            metadata: None,
            created_by: 1,
        };

        let config_assoc = repo.create_file_association(config_request).unwrap();
        let firmware_assoc = repo.create_file_association(firmware_request).unwrap();

        // Reorder: firmware first (order 0), config second (order 1)
        let new_order = vec![(firmware_assoc.id, 0), (config_assoc.id, 1)];
        repo.reorder_associations(1, new_order).unwrap();

        // Verify new order
        let associations = repo.get_asset_associations(1).unwrap();
        let firmware_assoc = associations.iter().find(|a| a.file_type == AssociationType::Firmware).unwrap();
        let config_assoc = associations.iter().find(|a| a.file_type == AssociationType::Configuration).unwrap();
        
        assert_eq!(firmware_assoc.association_order, 0);
        assert_eq!(config_assoc.association_order, 1);
    }

    #[test]
    fn test_validation_constraints() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssociationRepository::new(&conn);

        // Test invalid asset
        let invalid_asset_request = CreateAssociationRequest {
            asset_id: 999,
            file_id: 1,
            file_type: AssociationType::Configuration,
            metadata: None,
            created_by: 1,
        };

        let result = repo.create_file_association(invalid_asset_request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Asset with ID 999 does not exist"));

        // Test invalid configuration file
        let invalid_config_request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 999,
            file_type: AssociationType::Configuration,
            metadata: None,
            created_by: 1,
        };

        let result = repo.create_file_association(invalid_config_request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Configuration file with ID 999 does not exist"));

        // Test invalid firmware file
        let invalid_firmware_request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 999,
            file_type: AssociationType::Firmware,
            metadata: None,
            created_by: 1,
        };

        let result = repo.create_file_association(invalid_firmware_request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Firmware file with ID 999 does not exist"));
    }

    #[test]
    fn test_get_association_health_status() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssociationRepository::new(&conn);

        // Create a valid association
        let request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Configuration,
            metadata: None,
            created_by: 1,
        };

        repo.create_file_association(request).unwrap();

        let health = repo.get_association_health_status(1).unwrap();
        assert!(health.healthy);
        assert!(health.issues.is_empty());
        assert!(health.warnings.is_empty());
    }

    #[test]
    fn test_import_session_management() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssociationRepository::new(&conn);

        let file_paths = vec!["config1.json".to_string(), "config2.json".to_string()];
        let session = repo.create_import_session(
            "Test Import".to_string(),
            1,
            file_paths.clone(),
            1
        ).unwrap();

        assert_eq!(session.session_name, "Test Import");
        assert_eq!(session.asset_id, 1);
        assert_eq!(session.file_paths, file_paths);
        assert_eq!(session.import_status, ImportStatus::Pending);

        // Update session status
        repo.update_import_session_status(
            session.id,
            ImportStatus::Completed,
            Some("All files processed".to_string())
        ).unwrap();

        let updated_session = repo.get_import_session(session.id).unwrap().unwrap();
        assert_eq!(updated_session.import_status, ImportStatus::Completed);
        assert_eq!(updated_session.validation_results.unwrap(), "All files processed");
    }

    #[test]
    fn test_search_associations() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteAssociationRepository::new(&conn);

        // Create association
        let request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Configuration,
            metadata: None,
            created_by: 1,
        };

        repo.create_file_association(request).unwrap();

        // Search by asset name
        let results = repo.search_associations("Test Asset".to_string(), None).unwrap();
        assert_eq!(results.len(), 1);

        // Search by file type
        let results = repo.search_associations("config".to_string(), Some(AssociationType::Configuration)).unwrap();
        assert_eq!(results.len(), 1);

        // Search with no results
        let results = repo.search_associations("nonexistent".to_string(), None).unwrap();
        assert_eq!(results.len(), 0);
    }
}