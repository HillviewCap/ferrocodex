use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::metadata::{
    AssetMetadataSchema, AssetMetadata,
    SqliteMetadataRepository, MetadataRepository, AssetMetadataRepository,
    validation::SchemaValidator,
};
use super::{
    Pagination, SchemaFilters, SchemaList, DuplicationOptions,
    AssetMetadataFull, AssetMetadataHistory, RelatedAsset, PartialMetadata,
    CopyOptions, MetadataValidationRequest, TestCase, TestResults, TestCaseResult,
    FixResults, AppliedFix, MetadataRelationship, SimilarAsset, SchemaDependency,
    TimePeriod, UsageAnalytics, FieldUpdate, UpdateOperation, ValidationResult, ValidationError,
};
use rusqlite::Connection;
use std::time::Instant;
use std::cell::RefCell;

/// CRUD API implementation for metadata management
pub struct MetadataCrudApi<'a> {
    repo: SqliteMetadataRepository<'a>,
    validator: RefCell<SchemaValidator>,
}

impl<'a> MetadataCrudApi<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            repo: SqliteMetadataRepository::new(conn),
            validator: RefCell::new(SchemaValidator::new()),
        }
    }

    /// Convert metadata::ValidationResult to api::ValidationResult
    fn convert_validation_result(&self, meta_result: crate::metadata::ValidationResult) -> ValidationResult {
        ValidationResult {
            is_valid: meta_result.is_valid,
            errors: meta_result.errors.into_iter().map(|meta_error| ValidationError {
                field: meta_error.field_path,
                message: meta_error.message,
                error_type: meta_error.error_type,
            }).collect(),
            warnings: meta_result.warnings,
        }
    }

    /// Get metadata schema by ID with full details
    pub fn get_metadata_schema_by_id(&self, schema_id: u32) -> Result<Option<AssetMetadataSchema>, String> {
        let schema_id = schema_id as i64;
        self.repo.get_metadata_schema_by_id(schema_id)
            .map_err(|e| format!("Failed to get metadata schema {}: {}", schema_id, e))
    }

    /// List metadata schemas with pagination and filtering
    pub fn list_metadata_schemas(&self, filters: SchemaFilters, pagination: Pagination) -> Result<SchemaList, String> {
        // Build filter conditions
        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(name_contains) = &filters.name_contains {
            conditions.push("name LIKE ?".to_string());
            params.push(Box::new(format!("%{}%", name_contains)));
        }

        if let Some(asset_type) = &filters.asset_type {
            conditions.push("asset_type_filter = ?".to_string());
            params.push(Box::new(asset_type.clone()));
        }

        if let Some(created_by) = filters.created_by {
            conditions.push("created_by = ?".to_string());
            params.push(Box::new(created_by));
        }

        if let Some(is_system) = filters.is_system_template {
            conditions.push("is_system_template = ?".to_string());
            params.push(Box::new(is_system));
        }

        if let Some(created_after) = &filters.created_after {
            conditions.push("created_at >= ?".to_string());
            params.push(Box::new(created_after.clone()));
        }

        if let Some(created_before) = &filters.created_before {
            conditions.push("created_at <= ?".to_string());
            params.push(Box::new(created_before.clone()));
        }

        // Build WHERE clause
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Build ORDER BY clause
        let order_clause = match pagination.sort_field.as_deref() {
            Some("name") => format!("ORDER BY name {}", 
                if matches!(pagination.sort_direction, Some(super::SortDirection::Desc)) { "DESC" } else { "ASC" }),
            Some("created_at") => format!("ORDER BY created_at {}", 
                if matches!(pagination.sort_direction, Some(super::SortDirection::Desc)) { "DESC" } else { "ASC" }),
            Some("updated_at") => format!("ORDER BY updated_at {}", 
                if matches!(pagination.sort_direction, Some(super::SortDirection::Desc)) { "DESC" } else { "ASC" }),
            _ => "ORDER BY created_at DESC".to_string(),
        };

        // Calculate offset
        let offset = (pagination.page.saturating_sub(1)) * pagination.page_size;

        // Get total count
        let count_query = format!("SELECT COUNT(*) FROM metadata_schemas {}", where_clause);
        let total_count: u32 = self.repo.get_connection().prepare(&count_query)
            .map_err(|e| format!("Failed to prepare count query: {}", e))?
            .query_row(rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
                Ok(row.get::<_, i64>(0)? as u32)
            })
            .map_err(|e| format!("Failed to execute count query: {}", e))?;

        // Get schemas with pagination
        let query = format!(
            "SELECT id, name, description, schema_json, asset_type_filter, created_by, 
                    created_at, is_system_template, version 
             FROM metadata_schemas {} {} LIMIT {} OFFSET {}",
            where_clause, order_clause, pagination.page_size, offset
        );

        let mut stmt = self.repo.get_connection().prepare(&query)
            .map_err(|e| format!("Failed to prepare schemas query: {}", e))?;

        let schema_iter = stmt.query_map(rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
            Ok(AssetMetadataSchema {
                id: Some(row.get("id")?),
                name: row.get("name")?,
                description: row.get("description")?,
                schema_json: row.get("schema_json")?,
                asset_type_filter: row.get("asset_type_filter")?,
                created_by: row.get("created_by")?,
                created_at: row.get("created_at")?,
                is_system_template: row.get("is_system_template")?,
                version: row.get("version")?,
            })
        })
        .map_err(|e| format!("Failed to execute schemas query: {}", e))?;

        let mut schemas = Vec::new();
        for schema_result in schema_iter {
            schemas.push(schema_result.map_err(|e| format!("Failed to parse schema: {}", e))?);
        }

        let total_pages = (total_count + pagination.page_size - 1) / pagination.page_size;

        Ok(SchemaList {
            schemas,
            total_count,
            page: pagination.page,
            page_size: pagination.page_size,
            total_pages,
        })
    }

    /// Duplicate an existing metadata schema
    pub fn duplicate_metadata_schema(&self, schema_id: u32, options: DuplicationOptions) -> Result<AssetMetadataSchema, String> {
        let schema_id = schema_id as i64;
        
        // Get original schema
        let original = self.repo.get_metadata_schema_by_id(schema_id)
            .map_err(|e| format!("Failed to get original schema: {}", e))?
            .ok_or("Schema not found")?;

        // Create new schema with duplicated content
        let mut new_schema = AssetMetadataSchema::new(
            options.new_name,
            options.new_description.unwrap_or_else(|| format!("Copy of {}", original.description)),
            original.schema_json,
            original.asset_type_filter,
            original.created_by,
        );

        new_schema.is_system_template = options.mark_as_template;

        // Create the duplicated schema
        let created_schema = self.repo.create_metadata_schema(
            crate::metadata::CreateMetadataSchemaRequest {
                name: new_schema.name.clone(),
                description: new_schema.description.clone(),
                schema_json: new_schema.schema_json.clone(),
                asset_type_filter: new_schema.asset_type_filter.clone(),
            },
            new_schema.created_by,
        )
        .map_err(|e| format!("Failed to create duplicated schema: {}", e))?;

        Ok(created_schema)
    }

    /// Archive a metadata schema (soft delete)
    pub fn archive_metadata_schema(&self, schema_id: u32, reason: String) -> Result<(), String> {
        let schema_id = schema_id as i64;
        
        // Check if schema exists
        let _schema = self.repo.get_metadata_schema_by_id(schema_id)
            .map_err(|e| format!("Failed to get schema: {}", e))?
            .ok_or("Schema not found")?;

        // Add archived flag to schema (we'll need to modify the database schema for this)
        let query = "UPDATE metadata_schemas SET is_archived = 1, archived_at = datetime('now'), archive_reason = ? WHERE id = ?";
        
        self.repo.get_connection().execute(query, rusqlite::params![reason, schema_id])
            .map_err(|e| format!("Failed to archive schema: {}", e))?;

        Ok(())
    }

    /// Restore an archived metadata schema
    pub fn restore_metadata_schema(&self, schema_id: u32) -> Result<AssetMetadataSchema, String> {
        let schema_id = schema_id as i64;
        
        // Remove archived flag
        let query = "UPDATE metadata_schemas SET is_archived = 0, archived_at = NULL, archive_reason = NULL WHERE id = ?";
        
        self.repo.get_connection().execute(query, rusqlite::params![schema_id])
            .map_err(|e| format!("Failed to restore schema: {}", e))?;

        // Return the restored schema
        self.repo.get_metadata_schema_by_id(schema_id)
            .map_err(|e| format!("Failed to get restored schema: {}", e))?
            .ok_or("Schema not found after restore".to_string())
    }

    /// Get full asset metadata with history and relationships
    pub fn get_asset_metadata_full(&self, asset_id: u32, include_history: bool) -> Result<AssetMetadataFull, String> {
        let asset_id = asset_id as i64;
        
        // Get current metadata
        let current = self.repo.get_asset_metadata(asset_id)
            .map_err(|e| format!("Failed to get current metadata: {}", e))?;

        // Get schema if metadata exists
        let schema = if let Some(ref metadata) = current {
            self.repo.get_metadata_schema_by_id(metadata.schema_id)
                .map_err(|e| format!("Failed to get schema: {}", e))?
        } else {
            None
        };

        // Validate current metadata if it exists
        let validation_status = if let (Some(ref metadata), Some(ref schema)) = (&current, &schema) {
            match metadata.validate_against_schema(schema) {
                Ok(errors) => ValidationResult {
                    is_valid: errors.is_empty(),
                    errors: errors.into_iter().map(|msg| ValidationError {
                        field: "field".to_string(),
                        message: msg,
                        error_type: "validation".to_string(),
                    }).collect(),
                    warnings: Vec::new(),
                },
                Err(e) => ValidationResult::with_errors(vec![
                    ValidationError {
                        field: "schema".to_string(),
                        message: format!("Schema compilation error: {}", e),
                        error_type: "compilation".to_string(),
                    }
                ]),
            }
        } else {
            ValidationResult::success()
        };

        // Get history if requested
        let history = if include_history {
            self.get_asset_metadata_history(asset_id)?
        } else {
            Vec::new()
        };

        // Get related assets (placeholder implementation)
        let related_assets = self.find_related_assets(asset_id)?;

        Ok(AssetMetadataFull {
            current,
            schema,
            history,
            validation_status,
            related_assets,
        })
    }

    /// Update asset metadata partially
    pub fn update_asset_metadata_partial(&self, asset_id: u32, updates: PartialMetadata) -> Result<AssetMetadata, String> {
        let asset_id = asset_id as i64;
        
        // Get current metadata
        let current = self.repo.get_asset_metadata(asset_id)
            .map_err(|e| format!("Failed to get current metadata: {}", e))?;

        let (mut current_values, schema_id) = if let Some(current) = current {
            let values = current.get_metadata_values()
                .map_err(|e| format!("Failed to parse current metadata: {}", e))?;
            (values, updates.schema_id.unwrap_or(current.schema_id))
        } else {
            (serde_json::json!({}), updates.schema_id.ok_or("Schema ID required for new metadata")?)
        };

        // Apply field updates
        for update in updates.field_updates {
            match update.operation {
                super::UpdateOperation::Set => {
                    self.set_field_value(&mut current_values, &update.field_path, update.new_value)?;
                }
                super::UpdateOperation::Delete => {
                    self.delete_field_value(&mut current_values, &update.field_path)?;
                }
                super::UpdateOperation::Append => {
                    self.append_field_value(&mut current_values, &update.field_path, update.new_value)?;
                }
                super::UpdateOperation::Merge => {
                    self.merge_field_value(&mut current_values, &update.field_path, update.new_value)?;
                }
            }
        }

        // Create or update metadata
        let updated_values_json = serde_json::to_string(&current_values)
            .map_err(|e| format!("Failed to serialize updated values: {}", e))?;

        let updated_metadata = if let Some(_) = self.repo.get_asset_metadata(asset_id)
            .map_err(|e| format!("Failed to check existing metadata: {}", e))? 
        {
            self.repo.update_asset_metadata(asset_id, schema_id, updated_values_json)
                .map_err(|e| format!("Failed to update metadata: {}", e))?
        } else {
            self.repo.create_asset_metadata(AssetMetadata::new(asset_id, schema_id, updated_values_json, 1))
                .map_err(|e| format!("Failed to create metadata: {}", e))?
        };

        Ok(updated_metadata)
    }

    /// Delete asset metadata with option to preserve history
    pub fn delete_asset_metadata(&self, asset_id: u32, preserve_history: bool) -> Result<(), String> {
        let asset_id = asset_id as i64;
        
        if preserve_history {
            // Move to history table before deletion
            self.archive_metadata_history(asset_id)?;
        }

        self.repo.delete_asset_metadata(asset_id)
            .map_err(|e| format!("Failed to delete metadata: {}", e))
    }

    /// Copy metadata between assets
    pub fn copy_metadata_between_assets(&self, source_id: u32, target_id: u32, options: CopyOptions) -> Result<(), String> {
        let source_id = source_id as i64;
        let target_id = target_id as i64;
        
        // Get source metadata
        let source_metadata = self.repo.get_asset_metadata(source_id)
            .map_err(|e| format!("Failed to get source metadata: {}", e))?
            .ok_or("Source asset has no metadata")?;

        let mut source_values = source_metadata.get_metadata_values()
            .map_err(|e| format!("Failed to parse source metadata: {}", e))?;

        // Filter fields if specified
        if let Some(include_fields) = &options.include_fields {
            source_values = self.filter_fields(&source_values, include_fields, true)?;
        }

        if let Some(exclude_fields) = &options.exclude_fields {
            source_values = self.filter_fields(&source_values, exclude_fields, false)?;
        }

        // Determine target schema
        let target_schema_id = options.schema_id.unwrap_or(source_metadata.schema_id);

        // Handle existing metadata
        if let Some(existing) = self.repo.get_asset_metadata(target_id)
            .map_err(|e| format!("Failed to check target metadata: {}", e))? 
        {
            if !options.overwrite_existing {
                return Err("Target asset already has metadata and overwrite is disabled".to_string());
            }
            
            // Merge or replace based on options
            let target_values_json = serde_json::to_string(&source_values)
                .map_err(|e| format!("Failed to serialize target values: {}", e))?;
            
            self.repo.update_asset_metadata(target_id, target_schema_id, target_values_json)
                .map_err(|e| format!("Failed to update target metadata: {}", e))?;
        } else {
            // Create new metadata
            let target_values_json = serde_json::to_string(&source_values)
                .map_err(|e| format!("Failed to serialize target values: {}", e))?;
            
            let new_metadata = AssetMetadata::new(target_id, target_schema_id, target_values_json, 1);
            self.repo.create_asset_metadata(new_metadata)
                .map_err(|e| format!("Failed to create target metadata: {}", e))?;
        }

        Ok(())
    }

    /// Validate multiple metadata records in batch
    pub fn validate_metadata_batch(&self, data: Vec<MetadataValidationRequest>) -> Result<Vec<ValidationResult>, String> {
        let mut results = Vec::new();
        
        for request in data {
            let result = match self.repo.get_metadata_schema_by_id(request.schema_id) {
                Ok(Some(schema)) => {
                    let values_json = serde_json::to_string(&request.metadata_values)
                        .map_err(|e| format!("Failed to serialize values: {}", e))?;
                    
                    let meta_result = self.validator.borrow_mut().validate_metadata_values(&schema, &values_json);
                    self.convert_validation_result(meta_result)
                }
                Ok(None) => ValidationResult::with_errors(vec![
                    ValidationError {
                        field: "schema".to_string(),
                        error_type: "not_found".to_string(),
                        message: format!("Schema {} not found", request.schema_id),
                    }
                ]),
                Err(e) => ValidationResult::with_errors(vec![
                    ValidationError {
                        field: "schema".to_string(),
                        error_type: "error".to_string(),
                        message: format!("Failed to get schema: {}", e),
                    }
                ]),
            };
            
            results.push(result);
        }
        
        Ok(results)
    }

    /// Test metadata schema with test cases
    pub fn test_metadata_schema(&self, schema_id: u32, test_data: Vec<TestCase>) -> Result<TestResults, String> {
        let schema_id = schema_id as i64;
        
        let schema = self.repo.get_metadata_schema_by_id(schema_id)
            .map_err(|e| format!("Failed to get schema: {}", e))?
            .ok_or("Schema not found")?;

        let mut results = Vec::new();
        let mut total_tests = 0;
        let mut passed_tests = 0;

        for test_case in test_data {
            total_tests += 1;
            let start_time = Instant::now();
            
            let values_json = serde_json::to_string(&test_case.input_values)
                .map_err(|e| format!("Failed to serialize test values: {}", e))?;
            
            let meta_validation_result = self.validator.borrow_mut().validate_metadata_values(&schema, &values_json);
            let validation_result = self.convert_validation_result(meta_validation_result);
            
            let execution_time = start_time.elapsed().as_millis() as u64;
            let passed = validation_result.is_valid == test_case.expected_valid;
            
            if passed {
                passed_tests += 1;
            }
            
            results.push(TestCaseResult {
                test_name: test_case.name.clone(),
                passed,
                expected: test_case.expected_valid,
                actual_errors: validation_result.errors,
                execution_time_ms: execution_time,
            });
        }

        Ok(TestResults {
            test_name: format!("Schema {} Test Suite", schema_id),
            total_tests,
            passed_tests,
            failed_tests: total_tests - passed_tests,
            results,
        })
    }

    // Helper methods for field operations
    fn set_field_value(&self, values: &mut Value, field_path: &str, new_value: Value) -> Result<(), String> {
        let path_parts: Vec<&str> = field_path.split('.').collect();
        self.set_nested_value(values, &path_parts, new_value)
    }

    fn delete_field_value(&self, values: &mut Value, field_path: &str) -> Result<(), String> {
        let path_parts: Vec<&str> = field_path.split('.').collect();
        self.delete_nested_value(values, &path_parts)
    }

    fn append_field_value(&self, values: &mut Value, field_path: &str, new_value: Value) -> Result<(), String> {
        let path_parts: Vec<&str> = field_path.split('.').collect();
        let current = self.get_nested_value(values, &path_parts)?;
        
        match current {
            Some(Value::Array(mut arr)) => {
                arr.push(new_value);
                self.set_nested_value(values, &path_parts, Value::Array(arr))
            }
            Some(_) => Err(format!("Field {} is not an array", field_path)),
            None => self.set_nested_value(values, &path_parts, Value::Array(vec![new_value])),
        }
    }

    fn merge_field_value(&self, values: &mut Value, field_path: &str, new_value: Value) -> Result<(), String> {
        let path_parts: Vec<&str> = field_path.split('.').collect();
        let current = self.get_nested_value(values, &path_parts)?;
        
        match (current, &new_value) {
            (Some(Value::Object(mut current_obj)), Value::Object(new_obj)) => {
                for (key, value) in new_obj {
                    current_obj.insert(key.clone(), value.clone());
                }
                self.set_nested_value(values, &path_parts, Value::Object(current_obj))
            }
            (None, Value::Object(_)) => self.set_nested_value(values, &path_parts, new_value),
            _ => Err(format!("Cannot merge non-object values at field {}", field_path)),
        }
    }

    fn set_nested_value(&self, values: &mut Value, path: &[&str], new_value: Value) -> Result<(), String> {
        if path.is_empty() {
            return Err("Empty field path".to_string());
        }

        if path.len() == 1 {
            if let Value::Object(ref mut obj) = values {
                obj.insert(path[0].to_string(), new_value);
                return Ok(());
            } else {
                return Err("Cannot set field on non-object".to_string());
            }
        }

        if let Value::Object(ref mut obj) = values {
            let entry = obj.entry(path[0].to_string()).or_insert_with(|| Value::Object(serde_json::Map::new()));
            self.set_nested_value(entry, &path[1..], new_value)
        } else {
            Err("Cannot navigate through non-object".to_string())
        }
    }

    fn delete_nested_value(&self, values: &mut Value, path: &[&str]) -> Result<(), String> {
        if path.is_empty() {
            return Err("Empty field path".to_string());
        }

        if path.len() == 1 {
            if let Value::Object(ref mut obj) = values {
                obj.remove(path[0]);
                return Ok(());
            } else {
                return Err("Cannot delete field from non-object".to_string());
            }
        }

        if let Value::Object(ref mut obj) = values {
            if let Some(nested) = obj.get_mut(path[0]) {
                self.delete_nested_value(nested, &path[1..])
            } else {
                Ok(()) // Field doesn't exist, nothing to delete
            }
        } else {
            Err("Cannot navigate through non-object".to_string())
        }
    }

    fn get_nested_value(&self, values: &Value, path: &[&str]) -> Result<Option<Value>, String> {
        if path.is_empty() {
            return Ok(Some(values.clone()));
        }

        if let Value::Object(obj) = values {
            if path.len() == 1 {
                Ok(obj.get(path[0]).cloned())
            } else if let Some(nested) = obj.get(path[0]) {
                self.get_nested_value(nested, &path[1..])
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn filter_fields(&self, values: &Value, fields: &[String], include: bool) -> Result<Value, String> {
        if let Value::Object(obj) = values {
            let mut result = serde_json::Map::new();
            
            for (key, value) in obj {
                let should_include = if include {
                    fields.contains(key)
                } else {
                    !fields.contains(key)
                };
                
                if should_include {
                    result.insert(key.clone(), value.clone());
                }
            }
            
            Ok(Value::Object(result))
        } else {
            Ok(values.clone())
        }
    }

    fn get_asset_metadata_history(&self, asset_id: i64) -> Result<Vec<AssetMetadataHistory>, String> {
        // This would be implemented with a proper history table
        // For now, return empty vector as placeholder
        Ok(Vec::new())
    }

    fn find_related_assets(&self, asset_id: i64) -> Result<Vec<RelatedAsset>, String> {
        // This would implement similarity search based on metadata
        // For now, return empty vector as placeholder
        Ok(Vec::new())
    }

    fn archive_metadata_history(&self, asset_id: i64) -> Result<(), String> {
        // This would move current metadata to history table
        // For now, just return Ok as placeholder
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::tempdir;

    fn setup_test_db() -> (Database, tempfile::TempDir) {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(db_path.to_str().unwrap()).unwrap();
        db.initialize().unwrap();
        (db, temp_dir)
    }

    #[test]
    fn test_pagination_default_values() {
        let pagination = Pagination::default();
        assert_eq!(pagination.page, 1);
        assert_eq!(pagination.page_size, 20);
    }

    #[test]
    fn test_duplication_options_serialization() {
        let options = DuplicationOptions {
            new_name: "Test Copy".to_string(),
            new_description: Some("A test copy".to_string()),
            copy_usage_stats: true,
            mark_as_template: false,
        };

        let serialized = serde_json::to_string(&options).unwrap();
        let deserialized: DuplicationOptions = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.new_name, "Test Copy");
        assert_eq!(deserialized.new_description, Some("A test copy".to_string()));
        assert!(deserialized.copy_usage_stats);
        assert!(!deserialized.mark_as_template);
    }

    #[test]
    fn test_field_update_operations() {
        use serde_json::json;
        
        let update = FieldUpdate {
            field_path: "nested.field".to_string(),
            new_value: json!("test value"),
            operation: super::UpdateOperation::Set,
        };

        assert_eq!(update.field_path, "nested.field");
        assert_eq!(update.new_value, json!("test value"));
    }
}