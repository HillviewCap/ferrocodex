use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::metadata::{AssetMetadata, AssetMetadataSchema, ValidationResult, ValidationError, SqliteMetadataRepository, MetadataRepository};
use crate::assets::{Asset, SqliteAssetRepository, AssetRepository};
use super::Pagination;
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Write;

/// Export configuration for metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub filters: Option<ExportFilters>,
    pub options: Option<ExportOptions>,
    pub output_settings: Option<OutputSettings>,
}

/// Supported export formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Csv,
    Xml,
}

/// Export filters to control what data is included
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFilters {
    pub asset_ids: Option<Vec<i64>>,
    pub schema_ids: Option<Vec<i64>>,
    pub asset_types: Option<Vec<String>>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub updated_after: Option<String>,
    pub updated_before: Option<String>,
    pub include_system_schemas: bool,
    pub include_archived: bool,
}

/// Export options for customization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub include_schemas: bool,
    pub include_metadata_values: bool,
    pub include_asset_details: bool,
    pub include_timestamps: bool,
    pub include_relationships: bool,
    pub flatten_nested_objects: bool,
    pub custom_field_mapping: Option<HashMap<String, String>>,
    pub exclude_fields: Option<Vec<String>>,
    pub include_fields: Option<Vec<String>>,
}

/// Output format settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSettings {
    pub pretty_print: bool,
    pub compression: Option<CompressionType>,
    pub encoding: Option<String>,
    pub csv_delimiter: Option<String>,
    pub csv_quote_char: Option<String>,
    pub xml_root_element: Option<String>,
    pub json_array_wrapper: Option<String>,
}

/// Compression types for export
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    Gzip,
    Zip,
    None,
}

/// Export result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub format: ExportFormat,
    pub record_count: u64,
    pub file_size_bytes: u64,
    pub export_time_ms: u64,
    pub exported_at: String,
    pub checksum: Option<String>,
    pub warnings: Vec<String>,
}

/// Import configuration for metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConfig {
    pub format: ImportFormat,
    pub validation_mode: ImportValidationMode,
    pub conflict_resolution: ImportConflictResolution,
    pub field_mapping: Option<HashMap<String, String>>,
    pub schema_mapping: Option<HashMap<String, i64>>,
    pub default_values: Option<HashMap<String, Value>>,
    pub options: Option<ImportOptions>,
}

/// Supported import formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportFormat {
    Json,
    Csv,
    Xml,
}

/// Import validation modes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportValidationMode {
    Strict,
    Lenient,
    Skip,
}

/// Import conflict resolution strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportConflictResolution {
    Skip,
    Overwrite,
    Merge,
    Error,
    CreateNew,
}

/// Import options for customization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOptions {
    pub batch_size: u32,
    pub max_errors: Option<u32>,
    pub create_missing_schemas: bool,
    pub create_missing_assets: bool,
    pub preserve_ids: bool,
    pub update_timestamps: bool,
    pub dry_run: bool,
}

/// Import result with statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub format: ImportFormat,
    pub total_records: u64,
    pub imported_records: u64,
    pub skipped_records: u64,
    pub failed_records: u64,
    pub created_schemas: u64,
    pub created_assets: u64,
    pub import_time_ms: u64,
    pub imported_at: String,
    pub warnings: Vec<String>,
    pub errors: Vec<ImportError>,
}

/// Import error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    pub record_number: u64,
    pub field_path: Option<String>,
    pub error_type: String,
    pub error_message: String,
    pub raw_data: Option<String>,
}

/// Validation result for import data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportValidationResult {
    pub is_valid: bool,
    pub record_count: u64,
    pub validation_errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
    pub schema_compatibility: HashMap<String, bool>,
    pub missing_schemas: Vec<String>,
    pub missing_assets: Vec<String>,
}

/// Export/Import API implementation
pub struct MetadataExportImportApi<'a> {
    conn: &'a Connection,
    metadata_repo: SqliteMetadataRepository<'a>,
    asset_repo: SqliteAssetRepository<'a>,
}

impl<'a> MetadataExportImportApi<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            conn,
            metadata_repo: SqliteMetadataRepository::new(conn),
            asset_repo: SqliteAssetRepository::new(conn),
        }
    }

    /// Export metadata to JSON format
    pub fn export_metadata_to_json(&self, config: ExportConfig) -> Result<String, String> {
        let start_time = std::time::Instant::now();
        
        // Get data based on filters
        let (schemas, metadata_records, assets) = self.gather_export_data(&config)?;
        
        // Build JSON structure
        let mut json_data = serde_json::Map::new();
        
        // Add metadata based on options
        let options = config.options.as_ref();
        
        if options.map_or(true, |o| o.include_schemas) {
            json_data.insert("schemas".to_string(), serde_json::to_value(&schemas)
                .map_err(|e| format!("Failed to serialize schemas: {}", e))?);
        }
        
        if options.map_or(true, |o| o.include_metadata_values) {
            let processed_metadata = self.process_metadata_for_export(&metadata_records, &config)?;
            json_data.insert("metadata".to_string(), serde_json::to_value(&processed_metadata)
                .map_err(|e| format!("Failed to serialize metadata: {}", e))?);
        }
        
        if options.map_or(false, |o| o.include_asset_details) {
            json_data.insert("assets".to_string(), serde_json::to_value(&assets)
                .map_err(|e| format!("Failed to serialize assets: {}", e))?);
        }
        
        if options.map_or(true, |o| o.include_timestamps) {
            json_data.insert("exported_at".to_string(), 
                serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            json_data.insert("export_duration_ms".to_string(), 
                serde_json::Value::Number(serde_json::Number::from(start_time.elapsed().as_millis() as u64)));
        }
        
        // Add export metadata
        json_data.insert("export_info".to_string(), serde_json::json!({
            "format": "json",
            "version": "1.0",
            "record_count": metadata_records.len(),
            "schema_count": schemas.len()
        }));
        
        // Format output based on settings
        let output_settings = config.output_settings.as_ref();
        let json_output = if output_settings.map_or(false, |s| s.pretty_print) {
            serde_json::to_string_pretty(&json_data)
        } else {
            serde_json::to_string(&json_data)
        }.map_err(|e| format!("Failed to serialize JSON: {}", e))?;
        
        Ok(json_output)
    }

    /// Export metadata to CSV format
    pub fn export_metadata_to_csv(&self, config: ExportConfig) -> Result<String, String> {
        let (schemas, metadata_records, assets) = self.gather_export_data(&config)?;
        
        // Build CSV structure
        let mut csv_data: Vec<Vec<String>> = Vec::new();
        
        // Determine delimiter and quote character
        let delimiter = config.output_settings.as_ref()
            .and_then(|s| s.csv_delimiter.as_ref())
            .map(|s| s.as_str())
            .unwrap_or(",");
        let quote_char = config.output_settings.as_ref()
            .and_then(|s| s.csv_quote_char.as_ref())
            .map(|s| s.chars().next().unwrap_or('"'))
            .unwrap_or('"');
        
        // Create CSV writer
        let mut wtr = csv::WriterBuilder::new()
            .delimiter(delimiter.as_bytes()[0])
            .quote(quote_char as u8)
            .from_writer(Vec::new());
        
        // Build header row
        let mut headers = vec!["asset_id", "asset_name", "schema_id", "schema_name"];
        let options = config.options.as_ref();
        
        if options.map_or(true, |o| o.include_timestamps) {
            headers.extend(&["created_at", "updated_at"]);
        }
        
        // Add dynamic field headers
        let field_headers = self.extract_all_field_paths(&metadata_records)?;
        headers.extend(field_headers.iter().map(|s| s.as_str()));
        
        wtr.write_record(&headers)
            .map_err(|e| format!("Failed to write CSV headers: {}", e))?;
        
        // Write data rows
        for metadata in &metadata_records {
            let mut row = Vec::new();
            
            // Find corresponding asset and schema
            let asset = assets.iter().find(|a| a.id == metadata.asset_id);
            let schema = schemas.iter().find(|s| s.id == Some(metadata.schema_id));
            
            row.push(metadata.asset_id.to_string());
            row.push(asset.map_or("Unknown".to_string(), |a| a.name.clone()));
            row.push(metadata.schema_id.to_string());
            row.push(schema.map_or("Unknown".to_string(), |s| s.name.clone()));
            
            if options.map_or(true, |o| o.include_timestamps) {
                row.push(metadata.created_at.clone());
                row.push(metadata.updated_at.clone());
            }
            
            // Flatten metadata values for CSV
            let flattened_values = self.flatten_metadata_values(metadata)?;
            for field_path in &field_headers {
                let value = flattened_values.get(field_path)
                    .map_or("".to_string(), |v| self.format_value_for_csv(v));
                row.push(value);
            }
            
            wtr.write_record(&row)
                .map_err(|e| format!("Failed to write CSV row: {}", e))?;
        }
        
        let csv_bytes = wtr.into_inner()
            .map_err(|e| format!("Failed to finalize CSV: {}", e))?;
        
        String::from_utf8(csv_bytes)
            .map_err(|e| format!("Failed to convert CSV to string: {}", e))
    }

    /// Export metadata to XML format
    pub fn export_metadata_to_xml(&self, config: ExportConfig) -> Result<String, String> {
        let (schemas, metadata_records, assets) = self.gather_export_data(&config)?;
        
        let root_element = config.output_settings.as_ref()
            .and_then(|s| s.xml_root_element.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("metadata_export");
        
        let mut xml_output = String::new();
        xml_output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml_output.push_str(&format!("<{}>\n", root_element));
        
        // Add export info
        xml_output.push_str("  <export_info>\n");
        xml_output.push_str(&format!("    <format>xml</format>\n"));
        xml_output.push_str(&format!("    <exported_at>{}</exported_at>\n", chrono::Utc::now().to_rfc3339()));
        xml_output.push_str(&format!("    <record_count>{}</record_count>\n", metadata_records.len()));
        xml_output.push_str("  </export_info>\n");
        
        let options = config.options.as_ref();
        
        // Add schemas if requested
        if options.map_or(true, |o| o.include_schemas) {
            xml_output.push_str("  <schemas>\n");
            for schema in &schemas {
                xml_output.push_str("    <schema>\n");
                xml_output.push_str(&format!("      <id>{}</id>\n", schema.id.unwrap_or(0)));
                xml_output.push_str(&format!("      <name><![CDATA[{}]]></name>\n", schema.name));
                xml_output.push_str(&format!("      <description><![CDATA[{}]]></description>\n", schema.description));
                if options.map_or(true, |o| o.include_timestamps) {
                    xml_output.push_str(&format!("      <created_at>{}</created_at>\n", schema.created_at));
                }
                xml_output.push_str("    </schema>\n");
            }
            xml_output.push_str("  </schemas>\n");
        }
        
        // Add metadata records
        if options.map_or(true, |o| o.include_metadata_values) {
            xml_output.push_str("  <metadata_records>\n");
            for metadata in &metadata_records {
                xml_output.push_str("    <record>\n");
                xml_output.push_str(&format!("      <asset_id>{}</asset_id>\n", metadata.asset_id));
                xml_output.push_str(&format!("      <schema_id>{}</schema_id>\n", metadata.schema_id));
                
                // Add asset details if requested
                if options.map_or(false, |o| o.include_asset_details) {
                    if let Some(asset) = assets.iter().find(|a| a.id == metadata.asset_id) {
                        xml_output.push_str("      <asset>\n");
                        xml_output.push_str(&format!("        <name><![CDATA[{}]]></name>\n", asset.name));
                        xml_output.push_str(&format!("        <type><![CDATA[{}]]></type>\n", asset.asset_type));
                        xml_output.push_str("      </asset>\n");
                    }
                }
                
                // Add metadata values
                xml_output.push_str("      <values>\n");
                let values: Value = metadata.get_metadata_values()
                    .map_err(|e| format!("Failed to parse metadata values: {}", e))?;
                self.add_xml_values(&mut xml_output, &values, 4)?;
                xml_output.push_str("      </values>\n");
                
                if options.map_or(true, |o| o.include_timestamps) {
                    xml_output.push_str(&format!("      <created_at>{}</created_at>\n", metadata.created_at));
                    xml_output.push_str(&format!("      <updated_at>{}</updated_at>\n", metadata.updated_at));
                }
                
                xml_output.push_str("    </record>\n");
            }
            xml_output.push_str("  </metadata_records>\n");
        }
        
        xml_output.push_str(&format!("</{}>\n", root_element));
        
        Ok(xml_output)
    }

    /// Import metadata from file
    pub fn import_metadata_from_file(&self, file_content: String, config: ImportConfig) -> Result<ImportResult, String> {
        let start_time = std::time::Instant::now();
        
        let data = match config.format {
            ImportFormat::Json => self.parse_json_import(&file_content)?,
            ImportFormat::Csv => self.parse_csv_import(&file_content, &config)?,
            ImportFormat::Xml => self.parse_xml_import(&file_content)?,
        };
        
        let import_result = self.process_import_data(data, config)?;
        
        Ok(ImportResult {
            format: import_result.format,
            total_records: import_result.total_records,
            imported_records: import_result.imported_records,
            skipped_records: import_result.skipped_records,
            failed_records: import_result.failed_records,
            created_schemas: import_result.created_schemas,
            created_assets: import_result.created_assets,
            import_time_ms: start_time.elapsed().as_millis() as u64,
            imported_at: chrono::Utc::now().to_rfc3339(),
            warnings: import_result.warnings,
            errors: import_result.errors,
        })
    }

    /// Validate import data before processing
    pub fn validate_import_data(&self, data: String, format: ImportFormat) -> Result<ImportValidationResult, String> {
        let parsed_data = match format {
            ImportFormat::Json => self.parse_json_for_validation(&data)?,
            ImportFormat::Csv => self.parse_csv_for_validation(&data)?,
            ImportFormat::Xml => self.parse_xml_for_validation(&data)?,
        };
        
        let mut validation_errors = Vec::new();
        let mut warnings = Vec::new();
        let mut schema_compatibility = HashMap::new();
        let mut missing_schemas = Vec::new();
        let mut missing_assets = Vec::new();
        
        // Validate each record
        for (index, record) in parsed_data.iter().enumerate() {
            // Check schema existence
            if let Some(schema_id) = record.get("schema_id").and_then(|v| v.as_i64()) {
                if let Ok(None) = self.metadata_repo.get_metadata_schema_by_id(schema_id) {
                    missing_schemas.push(schema_id.to_string());
                    schema_compatibility.insert(schema_id.to_string(), false);
                } else {
                    schema_compatibility.insert(schema_id.to_string(), true);
                }
            }
            
            // Check asset existence  
            if let Some(asset_id) = record.get("asset_id").and_then(|v| v.as_i64()) {
                if let Ok(None) = self.asset_repo.get_asset_by_id(asset_id) {
                    missing_assets.push(asset_id.to_string());
                }
            }
            
            // Validate required fields
            if record.get("asset_id").is_none() {
                validation_errors.push(ValidationError::new(
                    format!("record[{}]", index),
                    "missing_field".to_string(),
                    "Missing required field: asset_id".to_string(),
                    None,
                    None,
                ));
            }
            
            if record.get("schema_id").is_none() {
                validation_errors.push(ValidationError::new(
                    format!("record[{}]", index),
                    "missing_field".to_string(),
                    "Missing required field: schema_id".to_string(),
                    None,
                    None,
                ));
            }
        }
        
        // Remove duplicates
        missing_schemas.sort();
        missing_schemas.dedup();
        missing_assets.sort();
        missing_assets.dedup();
        
        Ok(ImportValidationResult {
            is_valid: validation_errors.is_empty(),
            record_count: parsed_data.len() as u64,
            validation_errors,
            warnings,
            schema_compatibility,
            missing_schemas,
            missing_assets,
        })
    }

    // Helper methods
    fn gather_export_data(&self, config: &ExportConfig) -> Result<(Vec<AssetMetadataSchema>, Vec<AssetMetadata>, Vec<Asset>), String> {
        // Get schemas
        let schemas = if let Some(filters) = &config.filters {
            if let Some(schema_ids) = &filters.schema_ids {
                let mut filtered_schemas = Vec::new();
                for &schema_id in schema_ids {
                    if let Ok(Some(schema)) = self.metadata_repo.get_metadata_schema_by_id(schema_id) {
                        filtered_schemas.push(schema);
                    }
                }
                filtered_schemas
            } else {
                self.metadata_repo.get_metadata_schemas(None)
                    .map_err(|e| format!("Failed to get schemas: {}", e))?
            }
        } else {
            self.metadata_repo.get_metadata_schemas(None)
                .map_err(|e| format!("Failed to get schemas: {}", e))?
        };
        
        // Get metadata records (simplified - would implement proper filtering)
        let metadata_records = self.get_filtered_metadata(config)?;
        
        // Get assets
        let asset_ids: Vec<i64> = metadata_records.iter().map(|m| m.asset_id).collect();
        let mut assets = Vec::new();
        for asset_id in asset_ids {
            if let Ok(Some(asset)) = self.asset_repo.get_asset_by_id(asset_id) {
                assets.push(asset);
            }
        }
        
        Ok((schemas, metadata_records, assets))
    }

    fn get_filtered_metadata(&self, config: &ExportConfig) -> Result<Vec<AssetMetadata>, String> {
        // Simplified implementation - would build proper SQL filters
        let query = "SELECT id, asset_id, schema_id, metadata_values_json, schema_version, created_at, updated_at 
                     FROM asset_metadata ORDER BY created_at DESC";
        
        let mut stmt = self.conn.prepare(query)
            .map_err(|e| format!("Failed to prepare metadata query: {}", e))?;
        
        let metadata_iter = stmt.query_map([], |row| {
            Ok(AssetMetadata {
                id: Some(row.get("id")?),
                asset_id: row.get("asset_id")?,
                schema_id: row.get("schema_id")?,
                metadata_values_json: row.get("metadata_values_json")?,
                schema_version: row.get("schema_version")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .map_err(|e| format!("Failed to execute metadata query: {}", e))?;
        
        let mut metadata_records = Vec::new();
        for metadata_result in metadata_iter {
            metadata_records.push(metadata_result.map_err(|e| format!("Failed to parse metadata: {}", e))?);
        }
        
        Ok(metadata_records)
    }

    fn process_metadata_for_export(&self, metadata_records: &[AssetMetadata], config: &ExportConfig) -> Result<Vec<Value>, String> {
        let mut processed = Vec::new();
        let options = config.options.as_ref();
        
        for metadata in metadata_records {
            let mut values = metadata.get_metadata_values()
                .map_err(|e| format!("Failed to parse metadata values: {}", e))?;
            
            // Apply field filtering
            if let Some(include_fields) = options.and_then(|o| o.include_fields.as_ref()) {
                values = self.filter_fields(&values, include_fields, true)?;
            }
            
            if let Some(exclude_fields) = options.and_then(|o| o.exclude_fields.as_ref()) {
                values = self.filter_fields(&values, exclude_fields, false)?;
            }
            
            // Flatten if requested
            if options.map_or(false, |o| o.flatten_nested_objects) {
                values = self.flatten_json_object(&values)?;
            }
            
            processed.push(values);
        }
        
        Ok(processed)
    }

    fn extract_all_field_paths(&self, metadata_records: &[AssetMetadata]) -> Result<Vec<String>, String> {
        let mut field_paths = std::collections::HashSet::new();
        
        for metadata in metadata_records {
            let values = metadata.get_metadata_values()
                .map_err(|e| format!("Failed to parse metadata values: {}", e))?;
            self.collect_field_paths(&values, "", &mut field_paths);
        }
        
        let mut sorted_paths: Vec<String> = field_paths.into_iter().collect();
        sorted_paths.sort();
        
        Ok(sorted_paths)
    }

    fn collect_field_paths(&self, value: &Value, current_path: &str, paths: &mut std::collections::HashSet<String>) {
        match value {
            Value::Object(obj) => {
                for (key, val) in obj {
                    let new_path = if current_path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", current_path, key)
                    };
                    
                    match val {
                        Value::Object(_) => self.collect_field_paths(val, &new_path, paths),
                        _ => { paths.insert(new_path); }
                    }
                }
            }
            _ => {
                if !current_path.is_empty() {
                    paths.insert(current_path.to_string());
                }
            }
        }
    }

    fn flatten_metadata_values(&self, metadata: &AssetMetadata) -> Result<HashMap<String, Value>, String> {
        let values = metadata.get_metadata_values()
            .map_err(|e| format!("Failed to parse metadata values: {}", e))?;
        
        let mut flattened = HashMap::new();
        self.flatten_json_recursive(&values, "", &mut flattened);
        
        Ok(flattened)
    }

    fn flatten_json_recursive(&self, value: &Value, prefix: &str, result: &mut HashMap<String, Value>) {
        match value {
            Value::Object(obj) => {
                for (key, val) in obj {
                    let new_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    self.flatten_json_recursive(val, &new_key, result);
                }
            }
            _ => {
                result.insert(prefix.to_string(), value.clone());
            }
        }
    }

    fn format_value_for_csv(&self, value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => {
                arr.iter()
                    .map(|v| self.format_value_for_csv(v))
                    .collect::<Vec<_>>()
                    .join(";")
            }
            Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
            Value::Null => String::new(),
        }
    }

    fn add_xml_values(&self, xml_output: &mut String, value: &Value, indent_level: usize) -> Result<(), String> {
        let indent = "  ".repeat(indent_level);
        
        match value {
            Value::Object(obj) => {
                for (key, val) in obj {
                    xml_output.push_str(&format!("{}<{}>\n", indent, key));
                    self.add_xml_values(xml_output, val, indent_level + 1)?;
                    xml_output.push_str(&format!("{}</{}>\n", indent, key));
                }
            }
            Value::Array(arr) => {
                for (index, item) in arr.iter().enumerate() {
                    xml_output.push_str(&format!("{}<item index=\"{}\">\n", indent, index));
                    self.add_xml_values(xml_output, item, indent_level + 1)?;
                    xml_output.push_str(&format!("{}</item>\n", indent));
                }
            }
            _ => {
                let value_str = match value {
                    Value::String(s) => format!("<![CDATA[{}]]>", s),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "".to_string(),
                    _ => unreachable!(),
                };
                xml_output.push_str(&format!("{}{}\n", indent, value_str));
            }
        }
        
        Ok(())
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

    fn flatten_json_object(&self, value: &Value) -> Result<Value, String> {
        let mut flattened = HashMap::new();
        self.flatten_json_recursive(value, "", &mut flattened);
        
        let obj: serde_json::Map<String, Value> = flattened.into_iter().collect();
        Ok(Value::Object(obj))
    }

    fn parse_json_import(&self, content: &str) -> Result<Vec<Value>, String> {
        let data: Value = serde_json::from_str(content)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        
        match data {
            Value::Array(arr) => Ok(arr),
            Value::Object(_) => {
                if let Some(metadata) = data.get("metadata") {
                    if let Value::Array(arr) = metadata {
                        Ok(arr.clone())
                    } else {
                        Err("Metadata field is not an array".to_string())
                    }
                } else {
                    Ok(vec![data])
                }
            }
            _ => Err("Invalid JSON format for import".to_string()),
        }
    }

    fn parse_csv_import(&self, content: &str, config: &ImportConfig) -> Result<Vec<Value>, String> {
        let mut rdr = csv::Reader::from_reader(content.as_bytes());
        let mut records = Vec::new();
        
        let headers = rdr.headers()
            .map_err(|e| format!("Failed to read CSV headers: {}", e))?
            .clone();
        
        for result in rdr.records() {
            let record = result.map_err(|e| format!("Failed to read CSV record: {}", e))?;
            let mut json_record = serde_json::Map::new();
            
            for (i, field) in record.iter().enumerate() {
                if let Some(header) = headers.get(i) {
                    let value = if field.is_empty() {
                        Value::Null
                    } else if let Ok(num) = field.parse::<f64>() {
                        Value::Number(serde_json::Number::from_f64(num).unwrap_or_else(|| serde_json::Number::from(0)))
                    } else if field.eq_ignore_ascii_case("true") || field.eq_ignore_ascii_case("false") {
                        Value::Bool(field.eq_ignore_ascii_case("true"))
                    } else {
                        Value::String(field.to_string())
                    };
                    
                    json_record.insert(header.to_string(), value);
                }
            }
            
            records.push(Value::Object(json_record));
        }
        
        Ok(records)
    }

    fn parse_xml_import(&self, content: &str) -> Result<Vec<Value>, String> {
        // Simplified XML parsing - would use a proper XML parser in production
        Err("XML import not yet implemented".to_string())
    }

    fn parse_json_for_validation(&self, content: &str) -> Result<Vec<Value>, String> {
        self.parse_json_import(content)
    }

    fn parse_csv_for_validation(&self, content: &str) -> Result<Vec<Value>, String> {
        let config = ImportConfig {
            format: ImportFormat::Csv,
            validation_mode: ImportValidationMode::Strict,
            conflict_resolution: ImportConflictResolution::Error,
            field_mapping: None,
            schema_mapping: None,
            default_values: None,
            options: None,
        };
        self.parse_csv_import(content, &config)
    }

    fn parse_xml_for_validation(&self, content: &str) -> Result<Vec<Value>, String> {
        self.parse_xml_import(content)
    }

    fn process_import_data(&self, data: Vec<Value>, config: ImportConfig) -> Result<ImportResult, String> {
        // Simplified import processing - would implement full import logic
        Ok(ImportResult {
            format: config.format,
            total_records: data.len() as u64,
            imported_records: 0,
            skipped_records: 0,
            failed_records: 0,
            created_schemas: 0,
            created_assets: 0,
            import_time_ms: 0,
            imported_at: chrono::Utc::now().to_rfc3339(),
            warnings: vec![],
            errors: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_export_config_serialization() {
        let config = ExportConfig {
            format: ExportFormat::Json,
            filters: Some(ExportFilters {
                asset_ids: Some(vec![1, 2, 3]),
                schema_ids: None,
                asset_types: None,
                created_after: None,
                created_before: None,
                updated_after: None,
                updated_before: None,
                include_system_schemas: true,
                include_archived: false,
            }),
            options: Some(ExportOptions {
                include_schemas: true,
                include_metadata_values: true,
                include_asset_details: false,
                include_timestamps: true,
                include_relationships: false,
                flatten_nested_objects: false,
                custom_field_mapping: None,
                exclude_fields: None,
                include_fields: None,
            }),
            output_settings: None,
        };

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: ExportConfig = serde_json::from_str(&serialized).unwrap();
        
        assert!(matches!(deserialized.format, ExportFormat::Json));
        assert!(deserialized.filters.is_some());
        assert!(deserialized.options.is_some());
    }

    #[test]
    fn test_import_config_structure() {
        let config = ImportConfig {
            format: ImportFormat::Csv,
            validation_mode: ImportValidationMode::Strict,
            conflict_resolution: ImportConflictResolution::Overwrite,
            field_mapping: Some(HashMap::from([("old_name".to_string(), "new_name".to_string())])),
            schema_mapping: Some(HashMap::from([("device".to_string(), 1)])),
            default_values: Some(HashMap::from([("status".to_string(), json!("active"))])),
            options: Some(ImportOptions {
                batch_size: 100,
                max_errors: Some(10),
                create_missing_schemas: false,
                create_missing_assets: false,
                preserve_ids: true,
                update_timestamps: true,
                dry_run: false,
            }),
        };

        assert!(matches!(config.format, ImportFormat::Csv));
        assert!(matches!(config.validation_mode, ImportValidationMode::Strict));
        assert!(config.field_mapping.is_some());
        assert!(config.options.is_some());
    }

    #[test]
    fn test_export_formats() {
        let formats = vec![
            ExportFormat::Json,
            ExportFormat::Csv,
            ExportFormat::Xml,
        ];

        for format in formats {
            let serialized = serde_json::to_string(&format).unwrap();
            let _deserialized: ExportFormat = serde_json::from_str(&serialized).unwrap();
        }
    }
}