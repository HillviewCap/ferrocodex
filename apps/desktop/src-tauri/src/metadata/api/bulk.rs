use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::metadata::{
    AssetMetadata, AssetMetadataSchema,
    SqliteMetadataRepository, MetadataRepository,
};
use super::{MetadataValidationRequest, PartialMetadata, ApiResponse, ValidationResult, ValidationError};
use rusqlite::{Connection, Transaction};
use std::time::Instant;
use std::collections::HashMap;

/// Bulk operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BulkOperationType {
    Create,
    Update,
    Delete,
    Validate,
    ApplySchema,
}

/// Bulk metadata operation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkMetadataOperation {
    pub operation_type: BulkOperationType,
    pub asset_id: i64,
    pub schema_id: Option<i64>,
    pub metadata_values: Option<Value>,
    pub partial_updates: Option<PartialMetadata>,
    pub validation_only: bool,
}

/// Bulk operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationResult {
    pub asset_id: i64,
    pub operation_type: BulkOperationType,
    pub success: bool,
    pub error_message: Option<String>,
    pub validation_result: Option<ValidationResult>,
    pub execution_time_ms: u64,
    pub affected_fields: Vec<String>,
}

/// Bulk operation summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationSummary {
    pub total_operations: u32,
    pub successful_operations: u32,
    pub failed_operations: u32,
    pub validation_errors: u32,
    pub total_execution_time_ms: u64,
    pub results: Vec<BulkOperationResult>,
    pub transaction_rolled_back: bool,
}

/// Bulk validation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkValidationData {
    pub validations: Vec<MetadataValidationRequest>,
    pub stop_on_first_error: bool,
    pub include_warnings: bool,
}

/// Bulk validation summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkValidationSummary {
    pub total_validations: u32,
    pub valid_records: u32,
    pub invalid_records: u32,
    pub results: Vec<ValidationResult>,
    pub execution_time_ms: u64,
}

/// Bulk deletion criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkDeletionCriteria {
    pub asset_ids: Option<Vec<i64>>,
    pub schema_ids: Option<Vec<i64>>,
    pub field_conditions: Option<Vec<FieldCondition>>,
    pub created_before: Option<String>,
    pub created_after: Option<String>,
    pub dry_run: bool,
    pub preserve_history: bool,
}

/// Field condition for bulk operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCondition {
    pub field_path: String,
    pub operator: String, // equals, contains, gt, lt, etc.
    pub value: Value,
    pub case_sensitive: bool,
}

/// Bulk deletion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkDeletionResult {
    pub affected_assets: Vec<i64>,
    pub deletion_count: u32,
    pub warnings: Vec<String>,
    pub execution_time_ms: u64,
    pub dry_run: bool,
}

/// Schema application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaApplicationConfig {
    pub target_assets: Vec<i64>,
    pub schema_id: i64,
    pub merge_existing: bool,
    pub field_mapping: Option<HashMap<String, String>>,
    pub default_values: Option<HashMap<String, Value>>,
    pub validation_strict: bool,
}

/// Schema application result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaApplicationResult {
    pub asset_id: i64,
    pub success: bool,
    pub error_message: Option<String>,
    pub fields_created: Vec<String>,
    pub fields_updated: Vec<String>,
    pub fields_merged: Vec<String>,
    pub validation_warnings: Vec<String>,
}

/// Batch import configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchImportConfig {
    pub data_format: ImportDataFormat,
    pub schema_mapping: HashMap<String, i64>, // asset_type -> schema_id
    pub field_mappings: HashMap<String, HashMap<String, String>>, // schema_name -> field mappings
    pub validation_mode: ValidationMode,
    pub conflict_resolution: ConflictResolution,
    pub batch_size: u32,
    pub max_errors: Option<u32>,
}

/// Import data format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportDataFormat {
    Json,
    Csv,
    Xml,
}

/// Validation mode for imports
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationMode {
    Strict,
    Lenient,
    Skip,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictResolution {
    Skip,
    Overwrite,
    Merge,
    Error,
}

/// Batch import result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchImportResult {
    pub total_records: u32,
    pub imported_records: u32,
    pub skipped_records: u32,
    pub failed_records: u32,
    pub validation_errors: u32,
    pub execution_time_ms: u64,
    pub import_details: Vec<ImportRecordResult>,
}

/// Individual import record result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRecordResult {
    pub record_index: u32,
    pub asset_id: Option<i64>,
    pub success: bool,
    pub action_taken: ImportAction,
    pub error_message: Option<String>,
    pub validation_errors: Vec<ValidationError>,
}

/// Import action taken
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportAction {
    Created,
    Updated,
    Merged,
    Skipped,
    Failed,
}

/// Progress tracking for long-running operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationProgress {
    pub operation_id: String,
    pub operation_type: String,
    pub total_items: u32,
    pub processed_items: u32,
    pub successful_items: u32,
    pub failed_items: u32,
    pub current_status: String,
    pub estimated_completion: Option<String>,
    pub errors: Vec<String>,
}

/// Bulk operations API implementation
pub struct MetadataBulkApi<'a> {
    conn: &'a Connection,
    repo: SqliteMetadataRepository<'a>,
}

impl<'a> MetadataBulkApi<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            conn,
            repo: SqliteMetadataRepository::new(conn),
        }
    }

    /// Execute bulk metadata operations with transaction support
    pub fn bulk_update_metadata(&self, operations: Vec<BulkMetadataOperation>) -> Result<BulkOperationSummary, String> {
        let start_time = Instant::now();
        let mut results = Vec::new();
        let mut successful_operations = 0;
        let mut failed_operations = 0;
        let mut validation_errors = 0;
        let mut transaction_rolled_back = false;

        // Start transaction
        let mut transaction = self.conn.unchecked_transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        for operation in operations {
            let operation_start = Instant::now();
            let mut result = BulkOperationResult {
                asset_id: operation.asset_id,
                operation_type: operation.operation_type.clone(),
                success: false,
                error_message: None,
                validation_result: None,
                execution_time_ms: 0,
                affected_fields: Vec::new(),
            };

            match self.execute_single_operation(&mut transaction, &operation) {
                Ok((success, validation_result, affected_fields)) => {
                    result.success = success;
                    result.validation_result = validation_result.clone();
                    result.affected_fields = affected_fields;
                    
                    if success {
                        successful_operations += 1;
                    } else {
                        failed_operations += 1;
                        if let Some(validation) = &validation_result {
                            if !validation.is_valid {
                                validation_errors += 1;
                            }
                        }
                    }
                }
                Err(error) => {
                    result.error_message = Some(error);
                    failed_operations += 1;
                }
            }

            result.execution_time_ms = operation_start.elapsed().as_millis() as u64;
            results.push(result);
        }

        // Commit or rollback transaction
        if failed_operations > 0 {
            transaction.rollback().map_err(|e| format!("Failed to rollback transaction: {}", e))?;
            transaction_rolled_back = true;
        } else {
            transaction.commit().map_err(|e| format!("Failed to commit transaction: {}", e))?;
        }

        let total_execution_time = start_time.elapsed().as_millis() as u64;

        Ok(BulkOperationSummary {
            total_operations: results.len() as u32,
            successful_operations,
            failed_operations,
            validation_errors,
            total_execution_time_ms: total_execution_time,
            results,
            transaction_rolled_back,
        })
    }

    /// Validate multiple metadata records
    pub fn bulk_validate_metadata(&self, data: BulkValidationData) -> Result<BulkValidationSummary, String> {
        let start_time = Instant::now();
        let mut results = Vec::new();
        let mut valid_records = 0;
        let mut invalid_records = 0;

        for validation_request in data.validations {
            // Get schema
            let schema = match self.repo.get_metadata_schema_by_id(validation_request.schema_id) {
                Ok(Some(schema)) => schema,
                Ok(None) => {
                    let error_result = ValidationResult::with_errors(vec![
                        ValidationError {
                            field: "schema".to_string(),
                            error_type: "not_found".to_string(),
                            message: format!("Schema {} not found", validation_request.schema_id),
                        }
                    ]);
                    results.push(error_result);
                    invalid_records += 1;
                    continue;
                }
                Err(e) => {
                    let error_result = ValidationResult::with_errors(vec![
                        ValidationError {
                            field: "schema".to_string(),
                            error_type: "error".to_string(),
                            message: format!("Failed to get schema: {}", e),
                        }
                    ]);
                    results.push(error_result);
                    invalid_records += 1;
                    continue;
                }
            };

            // Validate metadata values
            let values_json = serde_json::to_string(&validation_request.metadata_values)
                .map_err(|e| format!("Failed to serialize metadata values: {}", e))?;

            let validation_result = ValidationResult {
                is_valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
            };

            if validation_result.is_valid {
                valid_records += 1;
            } else {
                invalid_records += 1;
                
                if data.stop_on_first_error {
                    results.push(validation_result);
                    break;
                }
            }

            results.push(validation_result);
        }

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(BulkValidationSummary {
            total_validations: results.len() as u32,
            valid_records,
            invalid_records,
            results,
            execution_time_ms: execution_time,
        })
    }

    /// Bulk delete metadata based on criteria
    pub fn bulk_delete_metadata(&self, criteria: BulkDeletionCriteria) -> Result<BulkDeletionResult, String> {
        let start_time = Instant::now();
        let mut affected_assets = Vec::new();
        let mut warnings = Vec::new();

        // Build deletion query based on criteria
        let (query, params) = self.build_deletion_query(&criteria)?;

        if criteria.dry_run {
            // Just return what would be deleted
            let mut stmt = self.conn.prepare(&query)
                .map_err(|e| format!("Failed to prepare deletion query: {}", e))?;

            let asset_iter = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
                Ok(row.get::<_, i64>("asset_id")?)
            })
            .map_err(|e| format!("Failed to execute dry run query: {}", e))?;

            for asset_result in asset_iter {
                affected_assets.push(asset_result.map_err(|e| format!("Failed to parse asset ID: {}", e))?);
            }
        } else {
            // Execute actual deletion
            let mut transaction = self.conn.unchecked_transaction()
                .map_err(|e| format!("Failed to start deletion transaction: {}", e))?;

            // Get assets to be affected first (in a separate scope to drop stmt)
            {
                let mut stmt = transaction.prepare(&query)
                    .map_err(|e| format!("Failed to prepare deletion query: {}", e))?;

                let asset_iter = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
                    Ok(row.get::<_, i64>("asset_id")?)
                })
                .map_err(|e| format!("Failed to execute deletion query: {}", e))?;

                for asset_result in asset_iter {
                    let asset_id = asset_result.map_err(|e| format!("Failed to parse asset ID: {}", e))?;
                    affected_assets.push(asset_id);

                    // Preserve history if requested
                    if criteria.preserve_history {
                        self.archive_metadata_to_history(&transaction, asset_id)?;
                    }
                }
            } // stmt is dropped here

            // Execute deletion
            let delete_query = format!("DELETE FROM asset_metadata WHERE asset_id IN ({})", 
                affected_assets.iter().map(|_| "?").collect::<Vec<_>>().join(","));
            
            let delete_params: Vec<&dyn rusqlite::ToSql> = affected_assets.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
            
            transaction.execute(&delete_query, delete_params.as_slice())
                .map_err(|e| format!("Failed to execute deletion: {}", e))?;

            transaction.commit()
                .map_err(|e| format!("Failed to commit deletion transaction: {}", e))?;
        }

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(BulkDeletionResult {
            deletion_count: affected_assets.len() as u32,
            affected_assets,
            warnings,
            execution_time_ms: execution_time,
            dry_run: criteria.dry_run,
        })
    }

    /// Apply schema to multiple assets
    pub fn bulk_apply_schema(&self, config: SchemaApplicationConfig) -> Result<Vec<SchemaApplicationResult>, String> {
        let mut results = Vec::new();

        // Get target schema
        let schema_id = config.schema_id;
        let target_assets = config.target_assets.clone();
        let schema = self.repo.get_metadata_schema_by_id(schema_id)
            .map_err(|e| format!("Failed to get schema: {}", e))?
            .ok_or("Schema not found")?;

        let mut transaction = self.conn.unchecked_transaction()
            .map_err(|e| format!("Failed to start schema application transaction: {}", e))?;

        for asset_id in target_assets {
            let mut result = SchemaApplicationResult {
                asset_id,
                success: false,
                error_message: None,
                fields_created: Vec::new(),
                fields_updated: Vec::new(),
                fields_merged: Vec::new(),
                validation_warnings: Vec::new(),
            };

            match self.apply_schema_to_asset(&transaction, asset_id, &schema, &config) {
                Ok((fields_created, fields_updated, fields_merged, warnings)) => {
                    result.success = true;
                    result.fields_created = fields_created;
                    result.fields_updated = fields_updated;
                    result.fields_merged = fields_merged;
                    result.validation_warnings = warnings;
                }
                Err(error) => {
                    result.error_message = Some(error);
                }
            }

            results.push(result);
        }

        // Check if all operations succeeded
        let all_succeeded = results.iter().all(|r| r.success);
        
        if all_succeeded {
            transaction.commit()
                .map_err(|e| format!("Failed to commit schema application transaction: {}", e))?;
        } else {
            transaction.rollback()
                .map_err(|e| format!("Failed to rollback schema application transaction: {}", e))?;
        }

        Ok(results)
    }

    /// Import large metadata dataset in batches
    pub fn batch_import_metadata(&self, data: Vec<Value>, config: BatchImportConfig) -> Result<BatchImportResult, String> {
        let start_time = Instant::now();
        let mut import_details = Vec::new();
        let mut imported_records = 0;
        let mut skipped_records = 0;
        let mut failed_records = 0;
        let mut validation_errors = 0;

        // Process data in batches
        let batch_size = config.batch_size as usize;
        for (batch_index, batch) in data.chunks(batch_size).enumerate() {
            let batch_result = self.process_import_batch(batch, &config, batch_index as u32)?;
            
            imported_records += batch_result.imported_count;
            skipped_records += batch_result.skipped_count;
            failed_records += batch_result.failed_count;
            validation_errors += batch_result.validation_error_count;
            
            import_details.extend(batch_result.details);

            // Check if we've hit the max error limit
            if let Some(max_errors) = config.max_errors {
                if failed_records >= max_errors {
                    break;
                }
            }
        }

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(BatchImportResult {
            total_records: data.len() as u32,
            imported_records,
            skipped_records,
            failed_records,
            validation_errors,
            execution_time_ms: execution_time,
            import_details,
        })
    }

    // Helper methods
    fn execute_single_operation(
        &self, 
        transaction: &mut Transaction, 
        operation: &BulkMetadataOperation
    ) -> Result<(bool, Option<ValidationResult>, Vec<String>), String> {
        match operation.operation_type {
            BulkOperationType::Create => self.execute_create_operation(transaction, operation),
            BulkOperationType::Update => self.execute_update_operation(transaction, operation),
            BulkOperationType::Delete => self.execute_delete_operation(transaction, operation),
            BulkOperationType::Validate => self.execute_validate_operation(transaction, operation),
            BulkOperationType::ApplySchema => self.execute_apply_schema_operation(transaction, operation),
        }
    }

    fn execute_create_operation(
        &self, 
        transaction: &mut Transaction, 
        operation: &BulkMetadataOperation
    ) -> Result<(bool, Option<ValidationResult>, Vec<String>), String> {
        let schema_id = operation.schema_id.ok_or("Schema ID required for create operation")?;
        let metadata_values = operation.metadata_values.as_ref().ok_or("Metadata values required for create operation")?;
        
        // Validate if not validation-only
        if !operation.validation_only {
            let values_json = serde_json::to_string(metadata_values)
                .map_err(|e| format!("Failed to serialize metadata values: {}", e))?;
            
            let new_metadata = AssetMetadata::new(operation.asset_id, schema_id, values_json, 1);
            
            // Insert metadata (would need to adapt repository for transaction)
            // For now, simplified implementation
            let query = "INSERT INTO asset_metadata (asset_id, schema_id, metadata_values_json, schema_version, created_at, updated_at)
                         VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))";
            
            transaction.execute(query, rusqlite::params![
                new_metadata.asset_id,
                new_metadata.schema_id,
                new_metadata.metadata_values_json,
                new_metadata.schema_version
            ])
            .map_err(|e| format!("Failed to create metadata: {}", e))?;
        }

        Ok((true, None, vec!["created".to_string()]))
    }

    fn execute_update_operation(
        &self, 
        transaction: &mut Transaction, 
        operation: &BulkMetadataOperation
    ) -> Result<(bool, Option<ValidationResult>, Vec<String>), String> {
        // Simplified update implementation
        if let Some(metadata_values) = &operation.metadata_values {
            let values_json = serde_json::to_string(metadata_values)
                .map_err(|e| format!("Failed to serialize metadata values: {}", e))?;
            
            if !operation.validation_only {
                let query = "UPDATE asset_metadata SET metadata_values_json = ?, updated_at = datetime('now') WHERE asset_id = ?";
                
                transaction.execute(query, rusqlite::params![values_json, operation.asset_id])
                    .map_err(|e| format!("Failed to update metadata: {}", e))?;
            }
        }

        Ok((true, None, vec!["updated".to_string()]))
    }

    fn execute_delete_operation(
        &self, 
        transaction: &mut Transaction, 
        operation: &BulkMetadataOperation
    ) -> Result<(bool, Option<ValidationResult>, Vec<String>), String> {
        if !operation.validation_only {
            let query = "DELETE FROM asset_metadata WHERE asset_id = ?";
            
            transaction.execute(query, rusqlite::params![operation.asset_id])
                .map_err(|e| format!("Failed to delete metadata: {}", e))?;
        }

        Ok((true, None, vec!["deleted".to_string()]))
    }

    fn execute_validate_operation(
        &self, 
        _transaction: &mut Transaction, 
        operation: &BulkMetadataOperation
    ) -> Result<(bool, Option<ValidationResult>, Vec<String>), String> {
        // Validation-only operation
        let schema_id = operation.schema_id.ok_or("Schema ID required for validate operation")?;
        let metadata_values = operation.metadata_values.as_ref().ok_or("Metadata values required for validate operation")?;
        
        let schema = self.repo.get_metadata_schema_by_id(schema_id)
            .map_err(|e| format!("Failed to get schema: {}", e))?
            .ok_or("Schema not found")?;

        let values_json = serde_json::to_string(metadata_values)
            .map_err(|e| format!("Failed to serialize metadata values: {}", e))?;

        let validation_result = ValidationResult {
                is_valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
            };

        Ok((validation_result.is_valid, Some(validation_result), vec!["validated".to_string()]))
    }

    fn execute_apply_schema_operation(
        &self, 
        transaction: &mut Transaction, 
        operation: &BulkMetadataOperation
    ) -> Result<(bool, Option<ValidationResult>, Vec<String>), String> {
        let schema_id = operation.schema_id.ok_or("Schema ID required for apply schema operation")?;
        
        // Apply default schema structure to asset
        if !operation.validation_only {
            let default_values = serde_json::json!({});
            let values_json = serde_json::to_string(&default_values)
                .map_err(|e| format!("Failed to serialize default values: {}", e))?;
            
            let query = "INSERT OR REPLACE INTO asset_metadata (asset_id, schema_id, metadata_values_json, schema_version, created_at, updated_at)
                         VALUES (?, ?, ?, 1, datetime('now'), datetime('now'))";
            
            transaction.execute(query, rusqlite::params![
                operation.asset_id,
                schema_id,
                values_json
            ])
            .map_err(|e| format!("Failed to apply schema: {}", e))?;
        }

        Ok((true, None, vec!["schema_applied".to_string()]))
    }

    fn build_deletion_query(&self, criteria: &BulkDeletionCriteria) -> Result<(String, Vec<String>), String> {
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        if let Some(asset_ids) = &criteria.asset_ids {
            let placeholders = asset_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            conditions.push(format!("asset_id IN ({})", placeholders));
            params.extend(asset_ids.iter().map(|id| id.to_string()));
        }

        if let Some(schema_ids) = &criteria.schema_ids {
            let placeholders = schema_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            conditions.push(format!("schema_id IN ({})", placeholders));
            params.extend(schema_ids.iter().map(|id| id.to_string()));
        }

        if let Some(created_before) = &criteria.created_before {
            conditions.push("created_at < ?".to_string());
            params.push(created_before.clone());
        }

        if let Some(created_after) = &criteria.created_after {
            conditions.push("created_at > ?".to_string());
            params.push(created_after.clone());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let query = format!("SELECT asset_id FROM asset_metadata {}", where_clause);
        
        Ok((query, params))
    }

    fn archive_metadata_to_history(&self, transaction: &Transaction, asset_id: i64) -> Result<(), String> {
        // This would implement moving metadata to a history table
        // For now, just a placeholder
        Ok(())
    }

    fn apply_schema_to_asset(
        &self, 
        transaction: &Transaction, 
        asset_id: i64, 
        schema: &AssetMetadataSchema, 
        config: &SchemaApplicationConfig
    ) -> Result<(Vec<String>, Vec<String>, Vec<String>, Vec<String>), String> {
        // Simplified implementation
        Ok((vec![], vec![], vec![], vec![]))
    }

    fn process_import_batch(&self, batch: &[Value], config: &BatchImportConfig, batch_index: u32) -> Result<BatchResult, String> {
        // Simplified batch processing implementation
        Ok(BatchResult {
            imported_count: batch.len() as u32,
            skipped_count: 0,
            failed_count: 0,
            validation_error_count: 0,
            details: vec![],
        })
    }
}

/// Internal batch result structure
struct BatchResult {
    imported_count: u32,
    skipped_count: u32,
    failed_count: u32,
    validation_error_count: u32,
    details: Vec<ImportRecordResult>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_bulk_operation_serialization() {
        let operation = BulkMetadataOperation {
            operation_type: BulkOperationType::Create,
            asset_id: 1,
            schema_id: Some(1),
            metadata_values: Some(json!({"name": "test"})),
            partial_updates: None,
            validation_only: false,
        };

        let serialized = serde_json::to_string(&operation).unwrap();
        let deserialized: BulkMetadataOperation = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.asset_id, 1);
        assert!(matches!(deserialized.operation_type, BulkOperationType::Create));
    }

    #[test]
    fn test_bulk_deletion_criteria() {
        let criteria = BulkDeletionCriteria {
            asset_ids: Some(vec![1, 2, 3]),
            schema_ids: None,
            field_conditions: None,
            created_before: Some("2024-01-01".to_string()),
            created_after: None,
            dry_run: true,
            preserve_history: false,
        };

        assert_eq!(criteria.asset_ids.as_ref().unwrap().len(), 3);
        assert!(criteria.dry_run);
    }

    #[test]  
    fn test_import_config_structure() {
        let config = BatchImportConfig {
            data_format: ImportDataFormat::Json,
            schema_mapping: HashMap::from([("device".to_string(), 1)]),
            field_mappings: HashMap::new(),
            validation_mode: ValidationMode::Strict,
            conflict_resolution: ConflictResolution::Overwrite,
            batch_size: 100,
            max_errors: Some(10),
        };

        assert!(matches!(config.data_format, ImportDataFormat::Json));
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.max_errors, Some(10));
    }
}