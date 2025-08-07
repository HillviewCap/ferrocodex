use anyhow::{anyhow, Result};
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Bulk Operations Types - separate from bulk import functionality

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BulkOperationType {
    Move,
    Delete,
    Export,
    Classify,
}

impl BulkOperationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BulkOperationType::Move => "move",
            BulkOperationType::Delete => "delete",
            BulkOperationType::Export => "export",
            BulkOperationType::Classify => "classify",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "move" => Ok(BulkOperationType::Move),
            "delete" => Ok(BulkOperationType::Delete),
            "export" => Ok(BulkOperationType::Export),
            "classify" => Ok(BulkOperationType::Classify),
            _ => Err(anyhow!("Invalid bulk operation type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BulkOperationStatus {
    Pending,
    Validating,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl BulkOperationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BulkOperationStatus::Pending => "pending",
            BulkOperationStatus::Validating => "validating",
            BulkOperationStatus::Processing => "processing",
            BulkOperationStatus::Completed => "completed",
            BulkOperationStatus::Failed => "failed",
            BulkOperationStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(BulkOperationStatus::Pending),
            "validating" => Ok(BulkOperationStatus::Validating),
            "processing" => Ok(BulkOperationStatus::Processing),
            "completed" => Ok(BulkOperationStatus::Completed),
            "failed" => Ok(BulkOperationStatus::Failed),
            "cancelled" => Ok(BulkOperationStatus::Cancelled),
            _ => Err(anyhow!("Invalid bulk operation status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperation {
    pub id: String,
    pub operation_type: BulkOperationType,
    pub asset_ids: Vec<i32>,
    pub status: BulkOperationStatus,
    pub progress_percent: f64,
    pub created_by: i32,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_details: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationLog {
    pub id: String,
    pub bulk_operation_id: String,
    pub asset_id: i32,
    pub action: String,
    pub status: String, // success, failed, skipped
    pub error_message: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationProgress {
    pub operation_id: String,
    pub status: BulkOperationStatus,
    pub total_items: i32,
    pub processed_items: i32,
    pub failed_items: i32,
    pub current_item: Option<String>,
    pub estimated_completion: Option<String>,
    pub processing_rate: f64,
    pub errors: Vec<BulkOperationError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationError {
    pub asset_id: i32,
    pub asset_name: String,
    pub error_type: String,
    pub error_message: String,
    pub error_details: Option<serde_json::Value>,
}

// Move Operation Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkMoveOptions {
    pub new_parent_id: Option<i32>,
    pub validate_hierarchy: bool,
    pub skip_conflicts: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkMoveRequest {
    pub asset_ids: Vec<i32>,
    pub new_parent_id: Option<i32>,
    pub options: BulkMoveOptions,
}

// Delete Operation Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkDeleteOptions {
    pub force_delete: bool,
    pub delete_children: bool,
    pub skip_protected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkDeleteRequest {
    pub asset_ids: Vec<i32>,
    pub options: BulkDeleteOptions,
}

// Export Operation Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExportFormat {
    Csv,
    Json,
    Xml,
    Yaml,
}

impl ExportFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            ExportFormat::Xml => "xml",
            ExportFormat::Yaml => "yaml",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "csv" => Ok(ExportFormat::Csv),
            "json" => Ok(ExportFormat::Json),
            "xml" => Ok(ExportFormat::Xml),
            "yaml" => Ok(ExportFormat::Yaml),
            _ => Err(anyhow!("Invalid export format: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkExportOptions {
    pub format: ExportFormat,
    pub include_metadata: bool,
    pub include_children: bool,
    pub include_configurations: bool,
    pub export_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkExportRequest {
    pub asset_ids: Vec<i32>,
    pub format: ExportFormat,
    pub options: BulkExportOptions,
}

// Classify Operation Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkClassifyRequest {
    pub asset_ids: Vec<i32>,
    pub new_classification: String,
    pub apply_to_children: bool,
}

// Validation Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub warnings: Vec<ValidationWarning>,
    pub errors: Vec<ValidationError>,
    pub conflicts: Vec<ValidationConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub asset_id: i32,
    pub asset_name: String,
    pub warning_type: String,
    pub message: String,
    pub can_proceed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub asset_id: i32,
    pub asset_name: String,
    pub error_type: String,
    pub message: String,
    pub blocking: bool,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConflict {
    pub asset_id: i32,
    pub asset_name: String,
    pub conflict_type: String,
    pub message: String,
    pub resolution_options: Vec<ConflictResolutionOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolutionOption {
    pub option_id: String,
    pub description: String,
    pub action: String, // skip, force, modify, cancel
    pub parameters: Option<serde_json::Value>,
}

// History and Undo Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationHistory {
    pub operations: Vec<BulkOperation>,
    pub total_count: i32,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoResult {
    pub success: bool,
    pub reverted_items: i32,
    pub failed_reversions: Vec<UndoFailure>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoFailure {
    pub asset_id: i32,
    pub asset_name: String,
    pub reason: String,
}

// Repository Traits
pub trait BulkOperationsRepository {
    fn create_operation(&self, operation: &BulkOperation) -> Result<()>;
    fn get_operation_by_id(&self, operation_id: &str) -> Result<Option<BulkOperation>>;
    fn update_operation_status(&self, operation_id: &str, status: BulkOperationStatus) -> Result<()>;
    fn update_operation_progress(&self, operation_id: &str, progress: f64, processed: i32, failed: i32) -> Result<()>;
    fn get_active_operations(&self, user_id: Option<i32>) -> Result<Vec<BulkOperation>>;
    fn get_operation_history(&self, user_id: Option<i32>, limit: Option<i32>) -> Result<BulkOperationHistory>;
    
    fn add_operation_log(&self, log: &BulkOperationLog) -> Result<()>;
    fn get_operation_logs(&self, operation_id: &str) -> Result<Vec<BulkOperationLog>>;
    
    fn delete_operation(&self, operation_id: &str) -> Result<()>;
}

pub struct SqliteBulkOperationsRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteBulkOperationsRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS bulk_operations (
                id TEXT PRIMARY KEY,
                operation_type TEXT NOT NULL,
                asset_ids TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                progress_percent REAL NOT NULL DEFAULT 0.0,
                created_by INTEGER NOT NULL,
                started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME,
                error_details TEXT,
                metadata TEXT,
                FOREIGN KEY (created_by) REFERENCES users(id)
            );

            CREATE TABLE IF NOT EXISTS bulk_operation_logs (
                id TEXT PRIMARY KEY,
                bulk_operation_id TEXT NOT NULL,
                asset_id INTEGER NOT NULL,
                action TEXT NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (bulk_operation_id) REFERENCES bulk_operations(id) ON DELETE CASCADE,
                FOREIGN KEY (asset_id) REFERENCES assets(id)
            );

            CREATE INDEX IF NOT EXISTS idx_bulk_operations_created_by ON bulk_operations(created_by);
            CREATE INDEX IF NOT EXISTS idx_bulk_operations_status ON bulk_operations(status);
            CREATE INDEX IF NOT EXISTS idx_bulk_operations_started_at ON bulk_operations(started_at);
            CREATE INDEX IF NOT EXISTS idx_bulk_operation_logs_operation_id ON bulk_operation_logs(bulk_operation_id);
            CREATE INDEX IF NOT EXISTS idx_bulk_operation_logs_asset_id ON bulk_operation_logs(asset_id);
            CREATE INDEX IF NOT EXISTS idx_bulk_operation_logs_timestamp ON bulk_operation_logs(timestamp);
            "#,
        )?;
        Ok(())
    }

    fn row_to_operation(row: &Row) -> rusqlite::Result<BulkOperation> {
        let operation_type_str: String = row.get("operation_type")?;
        let operation_type = BulkOperationType::from_str(&operation_type_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "operation_type".to_string(), rusqlite::types::Type::Text))?;

        let status_str: String = row.get("status")?;
        let status = BulkOperationStatus::from_str(&status_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "status".to_string(), rusqlite::types::Type::Text))?;

        let asset_ids_json: String = row.get("asset_ids")?;
        let asset_ids: Vec<i32> = serde_json::from_str(&asset_ids_json)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "asset_ids".to_string(), rusqlite::types::Type::Text))?;

        let metadata_json: Option<String> = row.get("metadata")?;
        let metadata = metadata_json.and_then(|json| serde_json::from_str(&json).ok());

        Ok(BulkOperation {
            id: row.get("id")?,
            operation_type,
            asset_ids,
            status,
            progress_percent: row.get("progress_percent")?,
            created_by: row.get("created_by")?,
            started_at: row.get("started_at")?,
            completed_at: row.get("completed_at")?,
            error_details: row.get("error_details")?,
            metadata,
        })
    }

    fn row_to_log(row: &Row) -> rusqlite::Result<BulkOperationLog> {
        Ok(BulkOperationLog {
            id: row.get("id")?,
            bulk_operation_id: row.get("bulk_operation_id")?,
            asset_id: row.get("asset_id")?,
            action: row.get("action")?,
            status: row.get("status")?,
            error_message: row.get("error_message")?,
            timestamp: row.get("timestamp")?,
        })
    }
}

impl<'a> BulkOperationsRepository for SqliteBulkOperationsRepository<'a> {
    fn create_operation(&self, operation: &BulkOperation) -> Result<()> {
        let asset_ids_json = serde_json::to_string(&operation.asset_ids)?;
        let metadata_json = operation.metadata.as_ref()
            .map(|m| serde_json::to_string(m))
            .transpose()?;

        self.conn.execute(
            "INSERT INTO bulk_operations (id, operation_type, asset_ids, status, progress_percent, created_by, started_at, completed_at, error_details, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            (
                &operation.id,
                operation.operation_type.as_str(),
                &asset_ids_json,
                operation.status.as_str(),
                &operation.progress_percent,
                &operation.created_by,
                &operation.started_at,
                &operation.completed_at,
                &operation.error_details,
                &metadata_json,
            ),
        )?;

        Ok(())
    }

    fn get_operation_by_id(&self, operation_id: &str) -> Result<Option<BulkOperation>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, operation_type, asset_ids, status, progress_percent, created_by, started_at, completed_at, error_details, metadata
             FROM bulk_operations WHERE id = ?1"
        )?;

        let result = stmt.query_row([operation_id], Self::row_to_operation);
        
        match result {
            Ok(operation) => Ok(Some(operation)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn update_operation_status(&self, operation_id: &str, status: BulkOperationStatus) -> Result<()> {
        let completed_at = if matches!(status, BulkOperationStatus::Completed | BulkOperationStatus::Failed | BulkOperationStatus::Cancelled) {
            Some("CURRENT_TIMESTAMP")
        } else {
            None
        };

        let query = if completed_at.is_some() {
            "UPDATE bulk_operations SET status = ?1, completed_at = CURRENT_TIMESTAMP WHERE id = ?2"
        } else {
            "UPDATE bulk_operations SET status = ?1 WHERE id = ?2"
        };

        let rows_affected = self.conn.execute(query, (status.as_str(), operation_id))?;

        if rows_affected == 0 {
            return Err(anyhow!("Operation not found"));
        }

        Ok(())
    }

    fn update_operation_progress(&self, operation_id: &str, progress: f64, processed: i32, failed: i32) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE bulk_operations SET progress_percent = ?1 WHERE id = ?2",
            (progress, operation_id),
        )?;

        if rows_affected == 0 {
            return Err(anyhow!("Operation not found"));
        }

        Ok(())
    }

    fn get_active_operations(&self, user_id: Option<i32>) -> Result<Vec<BulkOperation>> {
        let mut operations = Vec::new();
        
        if let Some(uid) = user_id {
            let query = "SELECT id, operation_type, asset_ids, status, progress_percent, created_by, started_at, completed_at, error_details, metadata
                 FROM bulk_operations WHERE created_by = ?1 AND status IN ('pending', 'validating', 'processing') ORDER BY started_at DESC";
            let mut stmt = self.conn.prepare(query)?;
            let operation_iter = stmt.query_map([uid], Self::row_to_operation)?;
            
            for operation in operation_iter {
                operations.push(operation?);
            }
        } else {
            let query = "SELECT id, operation_type, asset_ids, status, progress_percent, created_by, started_at, completed_at, error_details, metadata
                 FROM bulk_operations WHERE status IN ('pending', 'validating', 'processing') ORDER BY started_at DESC";
            let mut stmt = self.conn.prepare(query)?;
            let operation_iter = stmt.query_map([], Self::row_to_operation)?;
            
            for operation in operation_iter {
                operations.push(operation?);
            }
        }

        Ok(operations)
    }

    fn get_operation_history(&self, user_id: Option<i32>, limit: Option<i32>) -> Result<BulkOperationHistory> {
        let limit_value = limit.unwrap_or(50);
        let mut operations = Vec::new();
        
        if let Some(uid) = user_id {
            let query = format!(
                "SELECT id, operation_type, asset_ids, status, progress_percent, created_by, started_at, completed_at, error_details, metadata
                 FROM bulk_operations WHERE created_by = ?1 ORDER BY started_at DESC LIMIT {}",
                limit_value
            );
            let mut stmt = self.conn.prepare(&query)?;
            let operation_iter = stmt.query_map([uid], Self::row_to_operation)?;
            
            for operation in operation_iter {
                operations.push(operation?);
            }
        } else {
            let query = format!(
                "SELECT id, operation_type, asset_ids, status, progress_percent, created_by, started_at, completed_at, error_details, metadata
                 FROM bulk_operations ORDER BY started_at DESC LIMIT {}",
                limit_value
            );
            let mut stmt = self.conn.prepare(&query)?;
            let operation_iter = stmt.query_map([], Self::row_to_operation)?;
            
            for operation in operation_iter {
                operations.push(operation?);
            }
        }

        // Get total count
        let total_count: i32 = if let Some(uid) = user_id {
            let count_query = "SELECT COUNT(*) FROM bulk_operations WHERE created_by = ?1";
            let mut count_stmt = self.conn.prepare(count_query)?;
            count_stmt.query_row([uid], |row| row.get(0))?
        } else {
            let count_query = "SELECT COUNT(*) FROM bulk_operations";
            let mut count_stmt = self.conn.prepare(count_query)?;
            count_stmt.query_row([], |row| row.get(0))?
        };

        Ok(BulkOperationHistory {
            operations,
            total_count,
            page: 1,
            page_size: limit_value,
        })
    }

    fn add_operation_log(&self, log: &BulkOperationLog) -> Result<()> {
        self.conn.execute(
            "INSERT INTO bulk_operation_logs (id, bulk_operation_id, asset_id, action, status, error_message, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                &log.id,
                &log.bulk_operation_id,
                &log.asset_id,
                &log.action,
                &log.status,
                &log.error_message,
                &log.timestamp,
            ),
        )?;

        Ok(())
    }

    fn get_operation_logs(&self, operation_id: &str) -> Result<Vec<BulkOperationLog>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, bulk_operation_id, asset_id, action, status, error_message, timestamp
             FROM bulk_operation_logs WHERE bulk_operation_id = ?1 ORDER BY timestamp"
        )?;

        let log_iter = stmt.query_map([operation_id], Self::row_to_log)?;
        let mut logs = Vec::new();

        for log in log_iter {
            logs.push(log?);
        }

        Ok(logs)
    }

    fn delete_operation(&self, operation_id: &str) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM bulk_operations WHERE id = ?1",
            [operation_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow!("Operation not found"));
        }

        Ok(())
    }
}

// Operation Service for business logic
pub struct BulkOperationService<'a> {
    repo: &'a dyn BulkOperationsRepository,
}

impl<'a> BulkOperationService<'a> {
    pub fn new(repo: &'a dyn BulkOperationsRepository) -> Self {
        Self { repo }
    }

    pub fn create_bulk_move_operation(&self, request: BulkMoveRequest, user_id: i32) -> Result<String> {
        let operation_id = Uuid::new_v4().to_string();
        let metadata = serde_json::to_value(&request.options)?;
        
        let operation = BulkOperation {
            id: operation_id.clone(),
            operation_type: BulkOperationType::Move,
            asset_ids: request.asset_ids,
            status: BulkOperationStatus::Pending,
            progress_percent: 0.0,
            created_by: user_id,
            started_at: Utc::now().to_rfc3339(),
            completed_at: None,
            error_details: None,
            metadata: Some(metadata),
        };

        self.repo.create_operation(&operation)?;
        Ok(operation_id)
    }

    pub fn create_bulk_delete_operation(&self, request: BulkDeleteRequest, user_id: i32) -> Result<String> {
        let operation_id = Uuid::new_v4().to_string();
        let metadata = serde_json::to_value(&request.options)?;
        
        let operation = BulkOperation {
            id: operation_id.clone(),
            operation_type: BulkOperationType::Delete,
            asset_ids: request.asset_ids,
            status: BulkOperationStatus::Pending,
            progress_percent: 0.0,
            created_by: user_id,
            started_at: Utc::now().to_rfc3339(),
            completed_at: None,
            error_details: None,
            metadata: Some(metadata),
        };

        self.repo.create_operation(&operation)?;
        Ok(operation_id)
    }

    pub fn create_bulk_export_operation(&self, request: BulkExportRequest, user_id: i32) -> Result<String> {
        let operation_id = Uuid::new_v4().to_string();
        let metadata = serde_json::to_value(&request.options)?;
        
        let operation = BulkOperation {
            id: operation_id.clone(),
            operation_type: BulkOperationType::Export,
            asset_ids: request.asset_ids,
            status: BulkOperationStatus::Pending,
            progress_percent: 0.0,
            created_by: user_id,
            started_at: Utc::now().to_rfc3339(),
            completed_at: None,
            error_details: None,
            metadata: Some(metadata),
        };

        self.repo.create_operation(&operation)?;
        Ok(operation_id)
    }

    pub fn create_bulk_classify_operation(&self, request: BulkClassifyRequest, user_id: i32) -> Result<String> {
        let operation_id = Uuid::new_v4().to_string();
        let metadata = serde_json::json!({
            "new_classification": request.new_classification,
            "apply_to_children": request.apply_to_children
        });
        
        let operation = BulkOperation {
            id: operation_id.clone(),
            operation_type: BulkOperationType::Classify,
            asset_ids: request.asset_ids,
            status: BulkOperationStatus::Pending,
            progress_percent: 0.0,
            created_by: user_id,
            started_at: Utc::now().to_rfc3339(),
            completed_at: None,
            error_details: None,
            metadata: Some(metadata),
        };

        self.repo.create_operation(&operation)?;
        Ok(operation_id)
    }

    pub fn get_operation_progress(&self, operation_id: &str) -> Result<BulkOperationProgress> {
        let operation = self.repo.get_operation_by_id(operation_id)?
            .ok_or_else(|| anyhow!("Operation not found"))?;

        let logs = self.repo.get_operation_logs(operation_id)?;
        
        let processed_items = logs.iter().filter(|log| log.status == "success" || log.status == "failed").count() as i32;
        let failed_items = logs.iter().filter(|log| log.status == "failed").count() as i32;
        
        let errors: Vec<BulkOperationError> = logs
            .iter()
            .filter(|log| log.status == "failed")
            .map(|log| BulkOperationError {
                asset_id: log.asset_id,
                asset_name: format!("Asset {}", log.asset_id), // Would need to fetch actual name
                error_type: "processing_error".to_string(),
                error_message: log.error_message.clone().unwrap_or_default(),
                error_details: None,
            })
            .collect();

        let current_item = logs.last().map(|log| format!("Asset {}", log.asset_id));

        Ok(BulkOperationProgress {
            operation_id: operation_id.to_string(),
            status: operation.status,
            total_items: operation.asset_ids.len() as i32,
            processed_items,
            failed_items,
            current_item,
            estimated_completion: None, // Would calculate based on rate
            processing_rate: 0.0, // Would calculate based on timing
            errors,
        })
    }

    pub fn cancel_operation(&self, operation_id: &str) -> Result<()> {
        self.repo.update_operation_status(operation_id, BulkOperationStatus::Cancelled)
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
        
        // Create users and assets tables for foreign key constraints
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
            INSERT INTO assets (id, name, created_by) VALUES (1, 'Asset1', 1);
            INSERT INTO assets (id, name, created_by) VALUES (2, 'Asset2', 1);
            "#,
        ).unwrap();
        
        let repo = SqliteBulkOperationsRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_create_bulk_move_operation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBulkOperationsRepository::new(&conn);
        let service = BulkOperationService::new(&repo);

        let request = BulkMoveRequest {
            asset_ids: vec![1, 2],
            new_parent_id: None,
            options: BulkMoveOptions {
                new_parent_id: None,
                validate_hierarchy: true,
                skip_conflicts: false,
            },
        };

        let operation_id = service.create_bulk_move_operation(request, 1).unwrap();
        assert!(!operation_id.is_empty());

        let operation = repo.get_operation_by_id(&operation_id).unwrap().unwrap();
        assert_eq!(operation.operation_type, BulkOperationType::Move);
        assert_eq!(operation.asset_ids, vec![1, 2]);
        assert_eq!(operation.status, BulkOperationStatus::Pending);
        assert_eq!(operation.created_by, 1);
    }

    #[test]
    fn test_update_operation_status() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBulkOperationsRepository::new(&conn);
        let service = BulkOperationService::new(&repo);

        let request = BulkDeleteRequest {
            asset_ids: vec![1],
            options: BulkDeleteOptions {
                force_delete: false,
                delete_children: false,
                skip_protected: true,
            },
        };

        let operation_id = service.create_bulk_delete_operation(request, 1).unwrap();
        repo.update_operation_status(&operation_id, BulkOperationStatus::Processing).unwrap();

        let operation = repo.get_operation_by_id(&operation_id).unwrap().unwrap();
        assert_eq!(operation.status, BulkOperationStatus::Processing);
    }

    #[test]
    fn test_operation_logs() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteBulkOperationsRepository::new(&conn);

        let operation_id = "test-operation";
        let log = BulkOperationLog {
            id: Uuid::new_v4().to_string(),
            bulk_operation_id: operation_id.to_string(),
            asset_id: 1,
            action: "move".to_string(),
            status: "success".to_string(),
            error_message: None,
            timestamp: Utc::now().to_rfc3339(),
        };

        repo.add_operation_log(&log).unwrap();

        let logs = repo.get_operation_logs(operation_id).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].asset_id, 1);
        assert_eq!(logs[0].action, "move");
        assert_eq!(logs[0].status, "success");
    }
}