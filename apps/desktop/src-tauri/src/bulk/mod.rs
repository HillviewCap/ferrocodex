use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod operations;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkImportSession {
    pub id: i64,
    pub session_name: String,
    pub import_type: String,
    pub total_items: i64,
    pub processed_items: i64,
    pub failed_items: i64,
    pub status: BulkImportStatus,
    pub template_path: Option<String>,
    pub error_log: Option<String>,
    pub created_by: i64,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BulkImportStatus {
    Created,
    Validating,
    Processing,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl BulkImportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BulkImportStatus::Created => "Created",
            BulkImportStatus::Validating => "Validating",
            BulkImportStatus::Processing => "Processing",
            BulkImportStatus::Paused => "Paused",
            BulkImportStatus::Completed => "Completed",
            BulkImportStatus::Failed => "Failed",
            BulkImportStatus::Cancelled => "Cancelled",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "Created" => Ok(BulkImportStatus::Created),
            "Validating" => Ok(BulkImportStatus::Validating),
            "Processing" => Ok(BulkImportStatus::Processing),
            "Paused" => Ok(BulkImportStatus::Paused),
            "Completed" => Ok(BulkImportStatus::Completed),
            "Failed" => Ok(BulkImportStatus::Failed),
            "Cancelled" => Ok(BulkImportStatus::Cancelled),
            _ => Err(anyhow::anyhow!("Invalid bulk import status: {}", s)),
        }
    }
}

impl std::fmt::Display for BulkImportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkImportItem {
    pub id: i64,
    pub session_id: i64,
    pub item_data_json: String,
    pub processing_status: BulkItemStatus,
    pub error_message: Option<String>,
    pub asset_id: Option<i64>,
    pub processed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BulkItemStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Skipped,
}

impl BulkItemStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BulkItemStatus::Pending => "Pending",
            BulkItemStatus::Processing => "Processing",
            BulkItemStatus::Completed => "Completed",
            BulkItemStatus::Failed => "Failed",
            BulkItemStatus::Skipped => "Skipped",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "Pending" => Ok(BulkItemStatus::Pending),
            "Processing" => Ok(BulkItemStatus::Processing),
            "Completed" => Ok(BulkItemStatus::Completed),
            "Failed" => Ok(BulkItemStatus::Failed),
            "Skipped" => Ok(BulkItemStatus::Skipped),
            _ => Err(anyhow::anyhow!("Invalid bulk item status: {}", s)),
        }
    }
}

impl std::fmt::Display for BulkItemStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkImportSessionDetails {
    pub session: BulkImportSession,
    pub items: Vec<BulkImportItem>,
    pub errors: Vec<BulkImportError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkImportError {
    pub id: i64,
    pub session_id: i64,
    pub item_id: Option<i64>,
    pub error_type: String,
    pub error_message: String,
    pub error_details: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBulkImportSessionRequest {
    pub session_name: String,
    pub import_type: String,
    pub template_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub total_items: i64,
    pub valid_items: i64,
    pub invalid_items: i64,
    pub errors: Vec<ValidationError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub row: i64,
    pub field: String,
    pub value: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResults {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub preview_items: Vec<AssetPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub row: i64,
    pub field: String,
    pub value: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPreview {
    pub row: i64,
    pub name: String,
    pub description: String,
    pub asset_type: String,
    pub parent_name: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingOptions {
    pub skip_existing: bool,
    pub update_existing: bool,
    pub create_missing_parents: bool,
    pub validation_mode: ValidationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationMode {
    Strict,
    Permissive,
}

impl ValidationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ValidationMode::Strict => "strict",
            ValidationMode::Permissive => "permissive",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "strict" => Ok(ValidationMode::Strict),
            "permissive" => Ok(ValidationMode::Permissive),
            _ => Err(anyhow::anyhow!("Invalid validation mode: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressStatus {
    pub session_id: i64,
    pub total_items: i64,
    pub processed_items: i64,
    pub failed_items: i64,
    pub current_item: Option<String>,
    pub estimated_completion: Option<String>,
    pub processing_rate: f64,
    pub status: BulkImportStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTemplate {
    pub id: i64,
    pub template_name: String,
    pub template_type: String,
    pub field_mapping: HashMap<String, String>,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
    pub validation_rules: HashMap<String, serde_json::Value>,
    pub created_by: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTemplateConfig {
    pub template_name: String,
    pub template_type: String,
    pub asset_type: String,
    pub field_mapping: HashMap<String, String>,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
    pub validation_rules: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CSVParseResult {
    pub headers: Vec<String>,
    pub rows: Vec<HashMap<String, String>>,
    pub total_rows: i64,
    pub errors: Vec<CSVParseError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CSVParseError {
    pub row: i64,
    pub column: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationStats {
    pub total_sessions: i64,
    pub active_sessions: i64,
    pub completed_sessions: i64,
    pub failed_sessions: i64,
    pub total_items_processed: i64,
    pub average_processing_time: f64,
    pub success_rate: f64,
}

pub trait BulkImportRepository {
    fn create_session(&self, request: CreateBulkImportSessionRequest, created_by: i64) -> Result<BulkImportSession>;
    fn get_session_by_id(&self, session_id: i64) -> Result<Option<BulkImportSession>>;
    fn get_sessions_by_user(&self, user_id: i64) -> Result<Vec<BulkImportSession>>;
    fn get_session_details(&self, session_id: i64) -> Result<Option<BulkImportSessionDetails>>;
    fn update_session_status(&self, session_id: i64, status: BulkImportStatus) -> Result<()>;
    fn update_session_progress(&self, session_id: i64, processed: i64, failed: i64) -> Result<()>;
    fn delete_session(&self, session_id: i64) -> Result<()>;
    
    fn add_session_items(&self, session_id: i64, items: Vec<BulkImportItem>) -> Result<()>;
    fn get_session_items(&self, session_id: i64) -> Result<Vec<BulkImportItem>>;
    fn update_item_status(&self, item_id: i64, status: BulkItemStatus, error_message: Option<String>, asset_id: Option<i64>) -> Result<()>;
    
    fn add_session_error(&self, error: BulkImportError) -> Result<i64>;
    fn get_session_errors(&self, session_id: i64) -> Result<Vec<BulkImportError>>;
    
    fn get_bulk_operation_stats(&self) -> Result<BulkOperationStats>;
}

pub trait ImportTemplateRepository {
    fn create_template(&self, config: ImportTemplateConfig, created_by: i64) -> Result<ImportTemplate>;
    fn get_template_by_id(&self, template_id: i64) -> Result<Option<ImportTemplate>>;
    fn get_templates_by_type(&self, template_type: &str) -> Result<Vec<ImportTemplate>>;
    fn get_templates_by_user(&self, user_id: i64) -> Result<Vec<ImportTemplate>>;
    fn update_template(&self, template: &ImportTemplate) -> Result<()>;
    fn delete_template(&self, template_id: i64) -> Result<()>;
}

pub struct SqliteBulkImportRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteBulkImportRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS bulk_import_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_name TEXT NOT NULL,
                import_type TEXT NOT NULL,
                total_items INTEGER NOT NULL DEFAULT 0,
                processed_items INTEGER NOT NULL DEFAULT 0,
                failed_items INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'Created',
                template_path TEXT,
                error_log TEXT,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME,
                FOREIGN KEY (created_by) REFERENCES users(id)
            );

            CREATE TABLE IF NOT EXISTS bulk_import_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                item_data_json TEXT NOT NULL,
                processing_status TEXT NOT NULL DEFAULT 'Pending',
                error_message TEXT,
                asset_id INTEGER,
                processed_at DATETIME,
                FOREIGN KEY (session_id) REFERENCES bulk_import_sessions(id) ON DELETE CASCADE,
                FOREIGN KEY (asset_id) REFERENCES assets(id)
            );

            CREATE TABLE IF NOT EXISTS bulk_import_errors (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                item_id INTEGER,
                error_type TEXT NOT NULL,
                error_message TEXT NOT NULL,
                error_details TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES bulk_import_sessions(id) ON DELETE CASCADE,
                FOREIGN KEY (item_id) REFERENCES bulk_import_items(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS import_templates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                template_name TEXT NOT NULL UNIQUE,
                template_type TEXT NOT NULL,
                field_mapping TEXT NOT NULL,
                required_fields TEXT NOT NULL,
                optional_fields TEXT NOT NULL,
                validation_rules TEXT NOT NULL,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES users(id)
            );

            CREATE INDEX IF NOT EXISTS idx_bulk_sessions_created_by ON bulk_import_sessions(created_by);
            CREATE INDEX IF NOT EXISTS idx_bulk_sessions_status ON bulk_import_sessions(status);
            CREATE INDEX IF NOT EXISTS idx_bulk_sessions_created_at ON bulk_import_sessions(created_at);
            CREATE INDEX IF NOT EXISTS idx_bulk_items_session_id ON bulk_import_items(session_id);
            CREATE INDEX IF NOT EXISTS idx_bulk_items_status ON bulk_import_items(processing_status);
            CREATE INDEX IF NOT EXISTS idx_bulk_errors_session_id ON bulk_import_errors(session_id);
            CREATE INDEX IF NOT EXISTS idx_templates_type ON import_templates(template_type);
            "#,
        )?;
        Ok(())
    }

    fn row_to_session(row: &Row) -> rusqlite::Result<BulkImportSession> {
        let status_str: String = row.get("status")?;
        let status = BulkImportStatus::from_str(&status_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "status".to_string(), rusqlite::types::Type::Text))?;
            
        Ok(BulkImportSession {
            id: row.get("id")?,
            session_name: row.get("session_name")?,
            import_type: row.get("import_type")?,
            total_items: row.get("total_items")?,
            processed_items: row.get("processed_items")?,
            failed_items: row.get("failed_items")?,
            status,
            template_path: row.get("template_path")?,
            error_log: row.get("error_log")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            completed_at: row.get("completed_at")?,
        })
    }

    fn row_to_item(row: &Row) -> rusqlite::Result<BulkImportItem> {
        let status_str: String = row.get("processing_status")?;
        let status = BulkItemStatus::from_str(&status_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "processing_status".to_string(), rusqlite::types::Type::Text))?;
            
        Ok(BulkImportItem {
            id: row.get("id")?,
            session_id: row.get("session_id")?,
            item_data_json: row.get("item_data_json")?,
            processing_status: status,
            error_message: row.get("error_message")?,
            asset_id: row.get("asset_id")?,
            processed_at: row.get("processed_at")?,
        })
    }

    fn row_to_error(row: &Row) -> rusqlite::Result<BulkImportError> {
        Ok(BulkImportError {
            id: row.get("id")?,
            session_id: row.get("session_id")?,
            item_id: row.get("item_id")?,
            error_type: row.get("error_type")?,
            error_message: row.get("error_message")?,
            error_details: row.get("error_details")?,
            created_at: row.get("created_at")?,
        })
    }
}

impl<'a> BulkImportRepository for SqliteBulkImportRepository<'a> {
    fn create_session(&self, request: CreateBulkImportSessionRequest, created_by: i64) -> Result<BulkImportSession> {
        // Validate session name
        if request.session_name.trim().is_empty() {
            return Err(anyhow::anyhow!("Session name cannot be empty"));
        }
        if request.session_name.len() < 3 {
            return Err(anyhow::anyhow!("Session name must be at least 3 characters long"));
        }
        if request.session_name.len() > 100 {
            return Err(anyhow::anyhow!("Session name cannot exceed 100 characters"));
        }

        // Validate import type
        let valid_types = ["assets", "configurations", "metadata"];
        if !valid_types.contains(&request.import_type.as_str()) {
            return Err(anyhow::anyhow!("Invalid import type. Must be one of: assets, configurations, metadata"));
        }

        let mut stmt = self.conn.prepare(
            "INSERT INTO bulk_import_sessions (session_name, import_type, template_path, created_by) 
             VALUES (?1, ?2, ?3, ?4) RETURNING *"
        )?;

        let session = stmt.query_row(
            (&request.session_name, &request.import_type, &request.template_path, &created_by),
            Self::row_to_session,
        )?;

        Ok(session)
    }

    fn get_session_by_id(&self, session_id: i64) -> Result<Option<BulkImportSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_name, import_type, total_items, processed_items, failed_items, 
                    status, template_path, error_log, created_by, created_at, updated_at, completed_at
             FROM bulk_import_sessions WHERE id = ?1"
        )?;

        let result = stmt.query_row([session_id], Self::row_to_session);
        
        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn get_sessions_by_user(&self, user_id: i64) -> Result<Vec<BulkImportSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_name, import_type, total_items, processed_items, failed_items, 
                    status, template_path, error_log, created_by, created_at, updated_at, completed_at
             FROM bulk_import_sessions WHERE created_by = ?1 ORDER BY created_at DESC"
        )?;

        let session_iter = stmt.query_map([user_id], Self::row_to_session)?;
        let mut sessions = Vec::new();

        for session in session_iter {
            sessions.push(session?);
        }

        Ok(sessions)
    }

    fn get_session_details(&self, session_id: i64) -> Result<Option<BulkImportSessionDetails>> {
        if let Some(session) = self.get_session_by_id(session_id)? {
            let items = self.get_session_items(session_id)?;
            let errors = self.get_session_errors(session_id)?;
            
            Ok(Some(BulkImportSessionDetails {
                session,
                items,
                errors,
            }))
        } else {
            Ok(None)
        }
    }

    fn update_session_status(&self, session_id: i64, status: BulkImportStatus) -> Result<()> {
        let completed_at = if status == BulkImportStatus::Completed || status == BulkImportStatus::Failed || status == BulkImportStatus::Cancelled {
            Some("CURRENT_TIMESTAMP")
        } else {
            None
        };

        let query = if completed_at.is_some() {
            "UPDATE bulk_import_sessions SET status = ?1, completed_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?2"
        } else {
            "UPDATE bulk_import_sessions SET status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2"
        };

        let rows_affected = self.conn.execute(query, (&status.as_str(), &session_id))?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Session not found"));
        }

        Ok(())
    }

    fn update_session_progress(&self, session_id: i64, processed: i64, failed: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE bulk_import_sessions SET processed_items = ?1, failed_items = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
            (&processed, &failed, &session_id),
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Session not found"));
        }

        Ok(())
    }

    fn delete_session(&self, session_id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM bulk_import_sessions WHERE id = ?1",
            [session_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Session not found"));
        }

        Ok(())
    }

    fn add_session_items(&self, session_id: i64, items: Vec<BulkImportItem>) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        
        {
            let mut stmt = tx.prepare(
                "INSERT INTO bulk_import_items (session_id, item_data_json, processing_status) 
                 VALUES (?1, ?2, ?3)"
            )?;

            for item in items {
                stmt.execute((&session_id, &item.item_data_json, item.processing_status.as_str()))?;
            }
        }

        // Update total_items count
        tx.execute(
            "UPDATE bulk_import_sessions SET total_items = (
                SELECT COUNT(*) FROM bulk_import_items WHERE session_id = ?1
             ), updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [session_id],
        )?;

        tx.commit()?;
        Ok(())
    }

    fn get_session_items(&self, session_id: i64) -> Result<Vec<BulkImportItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, item_data_json, processing_status, error_message, asset_id, processed_at
             FROM bulk_import_items WHERE session_id = ?1 ORDER BY id"
        )?;

        let item_iter = stmt.query_map([session_id], Self::row_to_item)?;
        let mut items = Vec::new();

        for item in item_iter {
            items.push(item?);
        }

        Ok(items)
    }

    fn update_item_status(&self, item_id: i64, status: BulkItemStatus, error_message: Option<String>, asset_id: Option<i64>) -> Result<()> {
        let processed_at = if status == BulkItemStatus::Completed || status == BulkItemStatus::Failed {
            Some("CURRENT_TIMESTAMP")
        } else {
            None
        };

        let query = if processed_at.is_some() {
            "UPDATE bulk_import_items SET processing_status = ?1, error_message = ?2, asset_id = ?3, processed_at = CURRENT_TIMESTAMP WHERE id = ?4"
        } else {
            "UPDATE bulk_import_items SET processing_status = ?1, error_message = ?2, asset_id = ?3 WHERE id = ?4"
        };

        let rows_affected = self.conn.execute(query, (&status.as_str(), &error_message, &asset_id, &item_id))?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Item not found"));
        }

        Ok(())
    }

    fn add_session_error(&self, error: BulkImportError) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO bulk_import_errors (session_id, item_id, error_type, error_message, error_details) 
             VALUES (?1, ?2, ?3, ?4, ?5)"
        )?;

        stmt.execute((
            &error.session_id,
            &error.item_id,
            &error.error_type,
            &error.error_message,
            &error.error_details,
        ))?;

        Ok(self.conn.last_insert_rowid())
    }

    fn get_session_errors(&self, session_id: i64) -> Result<Vec<BulkImportError>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, item_id, error_type, error_message, error_details, created_at
             FROM bulk_import_errors WHERE session_id = ?1 ORDER BY created_at DESC"
        )?;

        let error_iter = stmt.query_map([session_id], Self::row_to_error)?;
        let mut errors = Vec::new();

        for error in error_iter {
            errors.push(error?);
        }

        Ok(errors)
    }

    fn get_bulk_operation_stats(&self) -> Result<BulkOperationStats> {
        let mut stmt = self.conn.prepare(
            "SELECT 
                COUNT(*) as total_sessions,
                COALESCE(SUM(CASE WHEN status IN ('Created', 'Validating', 'Processing', 'Paused') THEN 1 ELSE 0 END), 0) as active_sessions,
                COALESCE(SUM(CASE WHEN status = 'Completed' THEN 1 ELSE 0 END), 0) as completed_sessions,
                COALESCE(SUM(CASE WHEN status = 'Failed' THEN 1 ELSE 0 END), 0) as failed_sessions,
                COALESCE(SUM(processed_items), 0) as total_items_processed,
                COALESCE(AVG(CASE WHEN completed_at IS NOT NULL THEN 
                    (julianday(completed_at) - julianday(created_at)) * 24 * 60 * 60 
                    ELSE NULL END), 0.0) as average_processing_time,
                COALESCE(AVG(CASE WHEN total_items > 0 THEN 
                    CAST((processed_items - failed_items) AS REAL) / total_items * 100 
                    ELSE 100 END), 100.0) as success_rate
             FROM bulk_import_sessions"
        )?;

        let result = stmt.query_row([], |row| {
            Ok(BulkOperationStats {
                total_sessions: row.get("total_sessions").unwrap_or(0),
                active_sessions: row.get("active_sessions").unwrap_or(0),
                completed_sessions: row.get("completed_sessions").unwrap_or(0),
                failed_sessions: row.get("failed_sessions").unwrap_or(0),
                total_items_processed: row.get("total_items_processed").unwrap_or(0),
                average_processing_time: row.get("average_processing_time").unwrap_or(0.0),
                success_rate: row.get("success_rate").unwrap_or(100.0),
            })
        })?;

        Ok(result)
    }
}

impl<'a> ImportTemplateRepository for SqliteBulkImportRepository<'a> {
    fn create_template(&self, config: ImportTemplateConfig, created_by: i64) -> Result<ImportTemplate> {
        // Validate template name
        if config.template_name.trim().is_empty() {
            return Err(anyhow::anyhow!("Template name cannot be empty"));
        }
        if config.template_name.len() < 3 {
            return Err(anyhow::anyhow!("Template name must be at least 3 characters long"));
        }
        if config.template_name.len() > 100 {
            return Err(anyhow::anyhow!("Template name cannot exceed 100 characters"));
        }

        // Serialize JSON fields
        let field_mapping_json = serde_json::to_string(&config.field_mapping)
            .map_err(|e| anyhow::anyhow!("Failed to serialize field mapping: {}", e))?;
        
        let required_fields_json = serde_json::to_string(&config.required_fields)
            .map_err(|e| anyhow::anyhow!("Failed to serialize required fields: {}", e))?;
        
        let optional_fields_json = serde_json::to_string(&config.optional_fields)
            .map_err(|e| anyhow::anyhow!("Failed to serialize optional fields: {}", e))?;
        
        let validation_rules_json = serde_json::to_string(&config.validation_rules)
            .map_err(|e| anyhow::anyhow!("Failed to serialize validation rules: {}", e))?;

        let mut stmt = self.conn.prepare(
            "INSERT INTO import_templates (template_name, template_type, field_mapping, required_fields, optional_fields, validation_rules, created_by) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING *"
        )?;

        let template = stmt.query_row(
            (
                &config.template_name,
                &config.template_type,
                &field_mapping_json,
                &required_fields_json,
                &optional_fields_json,
                &validation_rules_json,
                &created_by,
            ),
            |row| {
                let field_mapping: HashMap<String, String> = serde_json::from_str(&row.get::<_, String>("field_mapping")?).unwrap_or_default();
                let required_fields: Vec<String> = serde_json::from_str(&row.get::<_, String>("required_fields")?).unwrap_or_default();
                let optional_fields: Vec<String> = serde_json::from_str(&row.get::<_, String>("optional_fields")?).unwrap_or_default();
                let validation_rules: HashMap<String, serde_json::Value> = serde_json::from_str(&row.get::<_, String>("validation_rules")?).unwrap_or_default();

                Ok(ImportTemplate {
                    id: row.get("id")?,
                    template_name: row.get("template_name")?,
                    template_type: row.get("template_type")?,
                    field_mapping,
                    required_fields,
                    optional_fields,
                    validation_rules,
                    created_by: row.get("created_by")?,
                    created_at: row.get("created_at")?,
                    updated_at: row.get("updated_at")?,
                })
            },
        )?;

        Ok(template)
    }

    fn get_template_by_id(&self, template_id: i64) -> Result<Option<ImportTemplate>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, template_name, template_type, field_mapping, required_fields, optional_fields, validation_rules, created_by, created_at, updated_at
             FROM import_templates WHERE id = ?1"
        )?;

        let result = stmt.query_row([template_id], |row| {
            let field_mapping: HashMap<String, String> = serde_json::from_str(&row.get::<_, String>("field_mapping")?).unwrap_or_default();
            let required_fields: Vec<String> = serde_json::from_str(&row.get::<_, String>("required_fields")?).unwrap_or_default();
            let optional_fields: Vec<String> = serde_json::from_str(&row.get::<_, String>("optional_fields")?).unwrap_or_default();
            let validation_rules: HashMap<String, serde_json::Value> = serde_json::from_str(&row.get::<_, String>("validation_rules")?).unwrap_or_default();

            Ok(ImportTemplate {
                id: row.get("id")?,
                template_name: row.get("template_name")?,
                template_type: row.get("template_type")?,
                field_mapping,
                required_fields,
                optional_fields,
                validation_rules,
                created_by: row.get("created_by")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        });
        
        match result {
            Ok(template) => Ok(Some(template)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn get_templates_by_type(&self, template_type: &str) -> Result<Vec<ImportTemplate>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, template_name, template_type, field_mapping, required_fields, optional_fields, validation_rules, created_by, created_at, updated_at
             FROM import_templates WHERE template_type = ?1 ORDER BY template_name"
        )?;

        let template_iter = stmt.query_map([template_type], |row| {
            let field_mapping: HashMap<String, String> = serde_json::from_str(&row.get::<_, String>("field_mapping")?).unwrap_or_default();
            let required_fields: Vec<String> = serde_json::from_str(&row.get::<_, String>("required_fields")?).unwrap_or_default();
            let optional_fields: Vec<String> = serde_json::from_str(&row.get::<_, String>("optional_fields")?).unwrap_or_default();
            let validation_rules: HashMap<String, serde_json::Value> = serde_json::from_str(&row.get::<_, String>("validation_rules")?).unwrap_or_default();

            Ok(ImportTemplate {
                id: row.get("id")?,
                template_name: row.get("template_name")?,
                template_type: row.get("template_type")?,
                field_mapping,
                required_fields,
                optional_fields,
                validation_rules,
                created_by: row.get("created_by")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;

        let mut templates = Vec::new();
        for template in template_iter {
            templates.push(template?);
        }

        Ok(templates)
    }

    fn get_templates_by_user(&self, user_id: i64) -> Result<Vec<ImportTemplate>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, template_name, template_type, field_mapping, required_fields, optional_fields, validation_rules, created_by, created_at, updated_at
             FROM import_templates WHERE created_by = ?1 ORDER BY template_name"
        )?;

        let template_iter = stmt.query_map([user_id], |row| {
            let field_mapping: HashMap<String, String> = serde_json::from_str(&row.get::<_, String>("field_mapping")?).unwrap_or_default();
            let required_fields: Vec<String> = serde_json::from_str(&row.get::<_, String>("required_fields")?).unwrap_or_default();
            let optional_fields: Vec<String> = serde_json::from_str(&row.get::<_, String>("optional_fields")?).unwrap_or_default();
            let validation_rules: HashMap<String, serde_json::Value> = serde_json::from_str(&row.get::<_, String>("validation_rules")?).unwrap_or_default();

            Ok(ImportTemplate {
                id: row.get("id")?,
                template_name: row.get("template_name")?,
                template_type: row.get("template_type")?,
                field_mapping,
                required_fields,
                optional_fields,
                validation_rules,
                created_by: row.get("created_by")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;

        let mut templates = Vec::new();
        for template in template_iter {
            templates.push(template?);
        }

        Ok(templates)
    }

    fn update_template(&self, template: &ImportTemplate) -> Result<()> {
        // Serialize JSON fields
        let field_mapping_json = serde_json::to_string(&template.field_mapping)
            .map_err(|e| anyhow::anyhow!("Failed to serialize field mapping: {}", e))?;
        
        let required_fields_json = serde_json::to_string(&template.required_fields)
            .map_err(|e| anyhow::anyhow!("Failed to serialize required fields: {}", e))?;
        
        let optional_fields_json = serde_json::to_string(&template.optional_fields)
            .map_err(|e| anyhow::anyhow!("Failed to serialize optional fields: {}", e))?;
        
        let validation_rules_json = serde_json::to_string(&template.validation_rules)
            .map_err(|e| anyhow::anyhow!("Failed to serialize validation rules: {}", e))?;

        let rows_affected = self.conn.execute(
            "UPDATE import_templates SET template_name = ?1, template_type = ?2, field_mapping = ?3, 
             required_fields = ?4, optional_fields = ?5, validation_rules = ?6, updated_at = CURRENT_TIMESTAMP 
             WHERE id = ?7",
            (
                &template.template_name,
                &template.template_type,
                &field_mapping_json,
                &required_fields_json,
                &optional_fields_json,
                &validation_rules_json,
                &template.id,
            ),
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Template not found"));
        }

        Ok(())
    }

    fn delete_template(&self, template_id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM import_templates WHERE id = ?1",
            [template_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Template not found"));
        }

        Ok(())
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
            
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'testuser', 'hash', 'Engineer');
            "#,
        ).unwrap();
        
        let repo = SqliteBulkImportRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_session_creation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBulkImportRepository::new(&conn);

        let request = CreateBulkImportSessionRequest {
            session_name: "Test Import Session".to_string(),
            import_type: "assets".to_string(),
            template_path: None,
        };

        let session = repo.create_session(request, 1).unwrap();
        assert_eq!(session.session_name, "Test Import Session");
        assert_eq!(session.import_type, "assets");
        assert_eq!(session.status, BulkImportStatus::Created);
        assert_eq!(session.total_items, 0);
        assert_eq!(session.processed_items, 0);
        assert_eq!(session.failed_items, 0);
        assert_eq!(session.created_by, 1);
    }

    #[test]
    fn test_session_validation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBulkImportRepository::new(&conn);

        // Test empty session name
        let request = CreateBulkImportSessionRequest {
            session_name: "".to_string(),
            import_type: "assets".to_string(),
            template_path: None,
        };
        assert!(repo.create_session(request, 1).is_err());

        // Test short session name
        let request = CreateBulkImportSessionRequest {
            session_name: "AB".to_string(),
            import_type: "assets".to_string(),
            template_path: None,
        };
        assert!(repo.create_session(request, 1).is_err());

        // Test invalid import type
        let request = CreateBulkImportSessionRequest {
            session_name: "Valid Name".to_string(),
            import_type: "invalid_type".to_string(),
            template_path: None,
        };
        assert!(repo.create_session(request, 1).is_err());
    }

    #[test]
    fn test_get_session_by_id() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBulkImportRepository::new(&conn);

        let request = CreateBulkImportSessionRequest {
            session_name: "Test Session".to_string(),
            import_type: "assets".to_string(),
            template_path: None,
        };

        let created_session = repo.create_session(request, 1).unwrap();
        let retrieved_session = repo.get_session_by_id(created_session.id).unwrap().unwrap();
        
        assert_eq!(created_session.id, retrieved_session.id);
        assert_eq!(created_session.session_name, retrieved_session.session_name);
    }

    #[test]
    fn test_update_session_status() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBulkImportRepository::new(&conn);

        let request = CreateBulkImportSessionRequest {
            session_name: "Test Session".to_string(),
            import_type: "assets".to_string(),
            template_path: None,
        };

        let session = repo.create_session(request, 1).unwrap();
        repo.update_session_status(session.id, BulkImportStatus::Processing).unwrap();

        let updated_session = repo.get_session_by_id(session.id).unwrap().unwrap();
        assert_eq!(updated_session.status, BulkImportStatus::Processing);
    }

    #[test]
    fn test_session_items() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBulkImportRepository::new(&conn);

        let request = CreateBulkImportSessionRequest {
            session_name: "Test Session".to_string(),
            import_type: "assets".to_string(),
            template_path: None,
        };

        let session = repo.create_session(request, 1).unwrap();

        let items = vec![
            BulkImportItem {
                id: 0, // Will be assigned by database
                session_id: session.id,
                item_data_json: r#"{"name": "Asset1", "type": "device"}"#.to_string(),
                processing_status: BulkItemStatus::Pending,
                error_message: None,
                asset_id: None,
                processed_at: None,
            },
            BulkImportItem {
                id: 0,
                session_id: session.id,
                item_data_json: r#"{"name": "Asset2", "type": "folder"}"#.to_string(),
                processing_status: BulkItemStatus::Pending,
                error_message: None,
                asset_id: None,
                processed_at: None,
            },
        ];

        repo.add_session_items(session.id, items).unwrap();

        let retrieved_items = repo.get_session_items(session.id).unwrap();
        assert_eq!(retrieved_items.len(), 2);

        // Check that total_items was updated
        let updated_session = repo.get_session_by_id(session.id).unwrap().unwrap();
        assert_eq!(updated_session.total_items, 2);
    }
}