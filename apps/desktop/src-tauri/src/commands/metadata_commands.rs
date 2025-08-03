use crate::metadata::{
    SqliteMetadataRepository, MetadataRepository, FieldTemplateRepository, 
    CreateMetadataSchemaRequest, UpdateMetadataSchemaRequest, AssetMetadataSchema, MetadataFieldTemplate,
    FieldCategory, FieldTypeInfo, TemplateUsageStats, validation::SchemaValidator,
    SqliteMetadataSearchRepository, MetadataSearchRepository, SearchQuery, AssetSearchResult,
    SearchSuggestion, FilterPreset, SearchAnalytics, FilterableField,
    api::{
        Pagination, SchemaFilters, SchemaList, DuplicationOptions, AssetMetadataFull,
        PartialMetadata, CopyOptions, MetadataValidationRequest, TestCase, TestResults,
        FixResults, MetadataRelationship, SimilarAsset, SchemaDependency, TimePeriod,
        UsageAnalytics, MetadataQuery, FieldStatsConfig, FieldStatistics, SearchConfig,
        SearchResults, AggregationConfig, AggregationResults, ExportConfig, ImportConfig,
        ImportResult, ImportValidationResult, BulkMetadataOperation, BulkOperationSummary,
        BulkValidationData, BulkValidationSummary, BulkDeletionCriteria, BulkDeletionResult,
        SchemaApplicationConfig, SchemaApplicationResult, BatchImportConfig, BatchImportResult,
        ApiInfo, ExternalMappingConfig, WebhookConfig, WebhookStatus, ExternalSyncConfig,
        SyncResults, SyncJobConfig, SyncProgress, ConflictResolutionData, TransformationConfig,
        TransformationTemplate, MetadataCrudApi, MetadataQueryApi, MetadataBulkApi,
        MetadataExportImportApi, MetadataIntegrationApi,
    },
};
use crate::{DatabaseState, SessionManagerState};
use tauri::State;
use tracing::{info, warn, error};
use rusqlite::params;

/// Get all system field templates
#[tauri::command]
pub async fn get_system_field_templates(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
) -> Result<Vec<MetadataFieldTemplate>, String> {
    info!("Getting system field templates");
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get templates
    let repo = SqliteMetadataRepository::new(conn);
    let templates = repo.get_system_templates()
        .map_err(|e| {
            error!("Failed to get system templates: {}", e);
            "Failed to retrieve system field templates".to_string()
        })?;

    info!("Retrieved {} system field templates for user {}", templates.len(), session.user_id);
    Ok(templates)
}

/// Get field templates by category
#[tauri::command]
pub async fn get_field_templates_by_category(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    category: String,
) -> Result<Vec<MetadataFieldTemplate>, String> {
    info!("Getting field templates for category: {}", category);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Parse category
    let field_category = FieldCategory::from_str(&category)
        .map_err(|e| format!("Invalid category: {}", e))?;

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get templates
    let repo = SqliteMetadataRepository::new(conn);
    let templates = repo.get_templates_by_category(field_category)
        .map_err(|e| {
            error!("Failed to get templates by category {}: {}", category, e);
            "Failed to retrieve field templates".to_string()
        })?;

    info!("Retrieved {} templates for category {} for user {}", templates.len(), category, session.user_id);
    Ok(templates)
}

/// Get supported field types and their constraints
#[tauri::command]
pub async fn get_supported_field_types(
    session_state: State<'_, SessionManagerState>,
    token: String,
) -> Result<Vec<FieldTypeInfo>, String> {
    info!("Getting supported field types");
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let _session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Return field type information
    let field_types = FieldTypeInfo::get_all_field_types();
    Ok(field_types)
}

/// Create a new metadata schema
#[tauri::command]
pub async fn create_metadata_schema(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    name: String,
    description: String,
    schema_json: String,
    asset_type_filter: Option<String>,
) -> Result<AssetMetadataSchema, String> {
    info!("Creating metadata schema: {}", name);
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Engineer+ role can create schemas
    if !matches!(session.role.to_string().as_str(), "Engineer" | "Administrator") {
        return Err("Insufficient permissions to create metadata schemas".to_string());
    }
    drop(session_manager_guard);

    // Validate schema JSON
    let mut validator = SchemaValidator::new();
    let validation_result = validator.validate_schema_definition(&schema_json);
    if !validation_result.is_valid {
        let error_msg = validation_result.errors.iter()
            .map(|e| format!("{}: {}", e.field_path, e.message))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!("Invalid schema: {}", error_msg));
    }

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Create request
    let request = CreateMetadataSchemaRequest {
        name: name.clone(),
        description,
        schema_json,
        asset_type_filter,
    };

    // Create schema
    let repo = SqliteMetadataRepository::new(conn);
    let schema = repo.create_metadata_schema(request, session.user_id)
        .map_err(|e| {
            error!("Failed to create metadata schema '{}': {}", name, e);
            format!("Failed to create metadata schema: {}", e)
        })?;

    // TODO: Add audit logging for metadata schema creation

    info!("Created metadata schema '{}' (id: {:?}) for user {}", name, schema.id, session.user_id);
    Ok(schema)
}

/// Get metadata schemas, optionally filtered by asset type
#[tauri::command]
pub async fn get_metadata_schemas(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    asset_type_filter: Option<String>,
) -> Result<Vec<AssetMetadataSchema>, String> {
    info!("Getting metadata schemas with filter: {:?}", asset_type_filter);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get schemas
    let repo = SqliteMetadataRepository::new(conn);
    let schemas = repo.get_metadata_schemas(asset_type_filter.clone())
        .map_err(|e| {
            error!("Failed to get metadata schemas: {}", e);
            "Failed to retrieve metadata schemas".to_string()
        })?;

    info!("Retrieved {} metadata schemas for user {}", schemas.len(), session.user_id);
    Ok(schemas)
}

/// Get a specific metadata schema by ID
#[tauri::command]
pub async fn get_metadata_schema_by_id(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    schema_id: i64,
) -> Result<Option<AssetMetadataSchema>, String> {
    info!("Getting metadata schema by ID: {}", schema_id);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get schema
    let repo = SqliteMetadataRepository::new(conn);
    let schema = repo.get_metadata_schema_by_id(schema_id)
        .map_err(|e| {
            error!("Failed to get metadata schema {}: {}", schema_id, e);
            "Failed to retrieve metadata schema".to_string()
        })?;

    info!("Retrieved metadata schema {} for user {}", schema_id, session.user_id);
    Ok(schema)
}

/// Update a metadata schema
#[tauri::command]
pub async fn update_metadata_schema(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    schema_id: i64,
    name: Option<String>,
    description: Option<String>,
    schema_json: Option<String>,
    asset_type_filter: Option<String>,
) -> Result<AssetMetadataSchema, String> {
    info!("Updating metadata schema: {}", schema_id);
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Engineer+ role can update schemas
    if !matches!(session.role.to_string().as_str(), "Engineer" | "Administrator") {
        return Err("Insufficient permissions to update metadata schemas".to_string());
    }
    drop(session_manager_guard);

    // Validate schema JSON if provided
    if let Some(ref schema_json) = schema_json {
        let mut validator = SchemaValidator::new();
        let validation_result = validator.validate_schema_definition(schema_json);
        if !validation_result.is_valid {
            let error_msg = validation_result.errors.iter()
                .map(|e| format!("{}: {}", e.field_path, e.message))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!("Invalid schema: {}", error_msg));
        }
    }

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Create update request
    let request = UpdateMetadataSchemaRequest {
        name,
        description,
        schema_json,
        asset_type_filter,
    };

    // Update schema
    let repo = SqliteMetadataRepository::new(conn);
    let schema = repo.update_metadata_schema(schema_id, request)
        .map_err(|e| {
            error!("Failed to update metadata schema {}: {}", schema_id, e);
            format!("Failed to update metadata schema: {}", e)
        })?;

    // TODO: Add audit logging for metadata schema update

    info!("Updated metadata schema {} for user {}", schema_id, session.user_id);
    Ok(schema)
}

/// Delete a metadata schema
#[tauri::command]
pub async fn delete_metadata_schema(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    schema_id: i64,
) -> Result<(), String> {
    info!("Deleting metadata schema: {}", schema_id);
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Engineer+ role can delete schemas
    if !matches!(session.role.to_string().as_str(), "Engineer" | "Administrator") {
        return Err("Insufficient permissions to delete metadata schemas".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get schema name for audit log
    let repo = SqliteMetadataRepository::new(conn);
    let schema_name = repo.get_metadata_schema_by_id(schema_id)
        .map_err(|e| format!("Failed to get schema for deletion: {}", e))?
        .map(|s| s.name)
        .unwrap_or_else(|| "Unknown".to_string());

    // Delete schema
    repo.delete_metadata_schema(schema_id)
        .map_err(|e| {
            error!("Failed to delete metadata schema {}: {}", schema_id, e);
            format!("Failed to delete metadata schema: {}", e)
        })?;

    // TODO: Add audit logging for metadata schema deletion

    info!("Deleted metadata schema {} for user {}", schema_id, session.user_id);
    Ok(())
}

/// Validate a metadata schema JSON definition
#[tauri::command]
pub async fn validate_metadata_schema(
    session_state: State<'_, SessionManagerState>,
    token: String,
    schema_json: String,
) -> Result<bool, String> {
    info!("Validating metadata schema JSON");
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let _session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Validate schema
    let mut validator = SchemaValidator::new();
    let validation_result = validator.validate_schema_definition(&schema_json);
    
    if validation_result.is_valid {
        info!("Schema validation successful");
        Ok(true)
    } else {
        let error_msg = validation_result.errors.iter()
            .map(|e| format!("{}: {}", e.field_path, e.message))
            .collect::<Vec<_>>()
            .join(", ");
        warn!("Schema validation failed: {}", error_msg);
        Err(format!("Schema validation failed: {}", error_msg))
    }
}

/// Get template usage statistics
#[tauri::command]
pub async fn get_template_usage_stats(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
) -> Result<Vec<TemplateUsageStats>, String> {
    info!("Getting template usage statistics");
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get usage stats
    let repo = SqliteMetadataRepository::new(conn);
    let stats = repo.get_template_usage_stats()
        .map_err(|e| {
            error!("Failed to get template usage stats: {}", e);
            "Failed to retrieve template usage statistics".to_string()
        })?;

    info!("Retrieved usage stats for {} templates for user {}", stats.len(), session.user_id);
    Ok(stats)
}

/// Import a custom field template
#[tauri::command]
pub async fn import_field_template(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    template_json: String,
) -> Result<MetadataFieldTemplate, String> {
    info!("Importing field template");
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Engineer+ role can import templates
    if !matches!(session.role.to_string().as_str(), "Engineer" | "Administrator") {
        return Err("Insufficient permissions to import field templates".to_string());
    }
    drop(session_manager_guard);

    // Parse template JSON
    let template: MetadataFieldTemplate = serde_json::from_str(&template_json)
        .map_err(|e| format!("Invalid template JSON: {}", e))?;

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Import template
    let repo = SqliteMetadataRepository::new(conn);
    let imported_template = repo.import_field_template(template)
        .map_err(|e| {
            error!("Failed to import field template: {}", e);
            format!("Failed to import field template: {}", e)
        })?;

    // TODO: Add audit logging for field template import

    info!("Imported field template '{}' for user {}", imported_template.name, session.user_id);
    Ok(imported_template)
}

/// Get schemas available for a specific asset type
#[tauri::command]
pub async fn get_schemas_for_asset_type(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    asset_type: String,
) -> Result<Vec<AssetMetadataSchema>, String> {
    info!("Getting schemas for asset type: {}", asset_type);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get schemas
    let repo = SqliteMetadataRepository::new(conn);
    let schemas = repo.get_schemas_for_asset_type(asset_type.clone())
        .map_err(|e| {
            error!("Failed to get schemas for asset type {}: {}", asset_type, e);
            "Failed to retrieve schemas for asset type".to_string()
        })?;

    info!("Retrieved {} schemas for asset type '{}' for user {}", schemas.len(), asset_type, session.user_id);
    Ok(schemas)
}

/// Validate metadata values against a schema
#[tauri::command]
pub async fn validate_metadata_values(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    schema_id: i64,
    values_json: String,
) -> Result<bool, String> {
    info!("Validating metadata values against schema {}", schema_id);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let _session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get schema
    let repo = SqliteMetadataRepository::new(conn);
    let schema = repo.get_metadata_schema_by_id(schema_id)
        .map_err(|e| format!("Failed to get schema: {}", e))?
        .ok_or("Schema not found")?;

    // Validate values
    let mut validator = SchemaValidator::new();
    let validation_result = validator.validate_metadata_values(&schema, &values_json);
    
    if validation_result.is_valid {
        info!("Metadata values validation successful");
        Ok(true)
    } else {
        let error_msg = validation_result.errors.iter()
            .map(|e| format!("{}: {}", e.field_path, e.message))
            .collect::<Vec<_>>()
            .join(", ");
        warn!("Metadata values validation failed: {}", error_msg);
        Err(format!("Validation failed: {}", error_msg))
    }
}

// === METADATA SEARCH COMMANDS ===

/// Search assets by metadata with full-text search and filtering
#[tauri::command]
pub async fn search_assets_by_metadata(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    query: SearchQuery,
) -> Result<Vec<AssetSearchResult>, String> {
    info!("Searching assets by metadata with query: {:?}", query.text_query);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Perform search
    let search_repo = SqliteMetadataSearchRepository::new(conn);
    let results = search_repo.search_assets_by_metadata(query)
        .map_err(|e| {
            error!("Failed to search assets by metadata: {}", e);
            format!("Failed to search assets: {}", e)
        })?;

    info!("Found {} assets matching search criteria for user {}", results.len(), session.user_id);
    Ok(results)
}

/// Get search suggestions for auto-complete
#[tauri::command]
pub async fn get_metadata_search_suggestions(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    partial_query: String,
    limit: Option<u32>,
) -> Result<Vec<SearchSuggestion>, String> {
    info!("Getting search suggestions for: {}", partial_query);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get suggestions
    let search_repo = SqliteMetadataSearchRepository::new(conn);
    let suggestions = search_repo.get_metadata_search_suggestions(partial_query, limit)
        .map_err(|e| {
            error!("Failed to get search suggestions: {}", e);
            format!("Failed to get search suggestions: {}", e)
        })?;

    info!("Retrieved {} search suggestions for user {}", suggestions.len(), session.user_id);
    Ok(suggestions)
}

/// Create a new filter preset for commonly used searches
#[tauri::command]
pub async fn create_metadata_filter_preset(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    preset: FilterPreset,
) -> Result<FilterPreset, String> {
    info!("Creating metadata filter preset: {}", preset.name);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Create preset with user ID
    let mut user_preset = preset;
    user_preset.created_by = session.user_id;

    let search_repo = SqliteMetadataSearchRepository::new(conn);
    let created_preset = search_repo.create_metadata_filter_preset(user_preset)
        .map_err(|e| {
            error!("Failed to create filter preset: {}", e);
            format!("Failed to create filter preset: {}", e)
        })?;

    info!("Created filter preset '{}' for user {}", created_preset.name, session.user_id);
    Ok(created_preset)
}

/// Get filter presets for the current user
#[tauri::command]
pub async fn get_metadata_filter_presets(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
) -> Result<Vec<FilterPreset>, String> {
    info!("Getting metadata filter presets");
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get presets
    let search_repo = SqliteMetadataSearchRepository::new(conn);
    let presets = search_repo.get_filter_presets(session.user_id)
        .map_err(|e| {
            error!("Failed to get filter presets: {}", e);
            format!("Failed to get filter presets: {}", e)
        })?;

    info!("Retrieved {} filter presets for user {}", presets.len(), session.user_id);
    Ok(presets)
}

/// Delete a filter preset
#[tauri::command]
pub async fn delete_filter_preset(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    preset_id: i64,
) -> Result<(), String> {
    info!("Deleting filter preset: {}", preset_id);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Delete preset
    conn.execute(
        "DELETE FROM metadata_filter_presets WHERE id = ?1 AND created_by = ?2",
        params![preset_id, session.user_id],
    ).map_err(|e| {
        error!("Failed to delete filter preset {}: {}", preset_id, e);
        format!("Failed to delete filter preset: {}", e)
    })?;

    info!("Deleted filter preset {} for user {}", preset_id, session.user_id);
    Ok(())
}

/// Get search analytics for performance monitoring
#[tauri::command]
pub async fn get_search_analytics(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    start_date: String,
    end_date: String,
) -> Result<SearchAnalytics, String> {
    info!("Getting search analytics from {} to {}", start_date, end_date);
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Administrator role can access analytics
    if !matches!(session.role.to_string().as_str(), "Administrator") {
        return Err("Insufficient permissions to access search analytics".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get analytics
    let search_repo = SqliteMetadataSearchRepository::new(conn);
    let analytics = search_repo.get_search_analytics(start_date, end_date)
        .map_err(|e| {
            error!("Failed to get search analytics: {}", e);
            format!("Failed to get search analytics: {}", e)
        })?;

    info!("Retrieved search analytics for user {}", session.user_id);
    Ok(analytics)
}

/// Find assets similar to a given asset based on metadata
#[tauri::command]
pub async fn find_similar_assets(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    asset_id: i64,
    similarity_threshold: f32,
) -> Result<Vec<AssetSearchResult>, String> {
    info!("Finding assets similar to asset {} with threshold {}", asset_id, similarity_threshold);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Find similar assets
    let search_repo = SqliteMetadataSearchRepository::new(conn);
    let similar_assets = search_repo.find_similar_assets(asset_id, similarity_threshold)
        .map_err(|e| {
            error!("Failed to find similar assets: {}", e);
            format!("Failed to find similar assets: {}", e)
        })?;

    info!("Found {} similar assets to asset {} for user {}", similar_assets.len(), asset_id, session.user_id);
    Ok(similar_assets)
}

/// Search assets within a specific hierarchy scope
#[tauri::command]
pub async fn search_assets_in_hierarchy(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    parent_id: Option<i64>,
    query: SearchQuery,
) -> Result<Vec<AssetSearchResult>, String> {
    info!("Searching assets in hierarchy scope: {:?}", parent_id);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Search in hierarchy
    let search_repo = SqliteMetadataSearchRepository::new(conn);
    let results = search_repo.search_assets_in_hierarchy(parent_id, query)
        .map_err(|e| {
            error!("Failed to search assets in hierarchy: {}", e);
            format!("Failed to search assets in hierarchy: {}", e)
        })?;

    info!("Found {} assets in hierarchy scope for user {}", results.len(), session.user_id);
    Ok(results)
}

/// Get filterable metadata fields for building advanced filters
#[tauri::command]
pub async fn get_filterable_metadata_fields(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
) -> Result<Vec<FilterableField>, String> {
    info!("Getting filterable metadata fields");
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get filterable fields
    let search_repo = SqliteMetadataSearchRepository::new(conn);
    let fields = search_repo.get_filterable_metadata_fields()
        .map_err(|e| {
            error!("Failed to get filterable fields: {}", e);
            format!("Failed to get filterable fields: {}", e)
        })?;

    info!("Retrieved {} filterable fields for user {}", fields.len(), session.user_id);
    Ok(fields)
}

// === ENHANCED METADATA API COMMANDS ===

/// List metadata schemas with pagination and filtering
#[tauri::command]
pub async fn list_metadata_schemas(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    filters: SchemaFilters,
    pagination: Pagination,
) -> Result<SchemaList, String> {
    info!("Listing metadata schemas with filters and pagination");
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use CRUD API
    let crud_api = MetadataCrudApi::new(conn);
    let result = crud_api.list_metadata_schemas(filters, pagination)
        .map_err(|e| {
            error!("Failed to list metadata schemas: {}", e);
            e
        })?;

    info!("Listed {} metadata schemas for user {}", result.schemas.len(), session.user_id);
    Ok(result)
}

/// Duplicate an existing metadata schema
#[tauri::command]
pub async fn duplicate_metadata_schema(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    schema_id: u32,
    options: DuplicationOptions,
) -> Result<AssetMetadataSchema, String> {
    info!("Duplicating metadata schema: {}", schema_id);
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Engineer+ role can duplicate schemas
    if !matches!(session.role.to_string().as_str(), "Engineer" | "Administrator") {
        return Err("Insufficient permissions to duplicate metadata schemas".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use CRUD API
    let crud_api = MetadataCrudApi::new(conn);
    let result = crud_api.duplicate_metadata_schema(schema_id, options)
        .map_err(|e| {
            error!("Failed to duplicate metadata schema {}: {}", schema_id, e);
            e
        })?;

    info!("Duplicated metadata schema {} for user {}", schema_id, session.user_id);
    Ok(result)
}

/// Archive a metadata schema (soft delete)
#[tauri::command]
pub async fn archive_metadata_schema(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    schema_id: u32,
    reason: String,
) -> Result<(), String> {
    info!("Archiving metadata schema: {}", schema_id);
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Administrator role can archive schemas
    if !matches!(session.role.to_string().as_str(), "Administrator") {
        return Err("Insufficient permissions to archive metadata schemas".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use CRUD API
    let crud_api = MetadataCrudApi::new(conn);
    crud_api.archive_metadata_schema(schema_id, reason)
        .map_err(|e| {
            error!("Failed to archive metadata schema {}: {}", schema_id, e);
            e
        })?;

    info!("Archived metadata schema {} for user {}", schema_id, session.user_id);
    Ok(())
}

/// Restore an archived metadata schema
#[tauri::command]
pub async fn restore_metadata_schema(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    schema_id: u32,
) -> Result<AssetMetadataSchema, String> {
    info!("Restoring metadata schema: {}", schema_id);
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Administrator role can restore schemas
    if !matches!(session.role.to_string().as_str(), "Administrator") {
        return Err("Insufficient permissions to restore metadata schemas".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use CRUD API
    let crud_api = MetadataCrudApi::new(conn);
    let result = crud_api.restore_metadata_schema(schema_id)
        .map_err(|e| {
            error!("Failed to restore metadata schema {}: {}", schema_id, e);
            e
        })?;

    info!("Restored metadata schema {} for user {}", schema_id, session.user_id);
    Ok(result)
}

/// Get full asset metadata with history and relationships
#[tauri::command]
pub async fn get_asset_metadata_full(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    asset_id: u32,
    include_history: bool,
) -> Result<AssetMetadataFull, String> {
    info!("Getting full asset metadata for asset: {}", asset_id);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use CRUD API
    let crud_api = MetadataCrudApi::new(conn);
    let result = crud_api.get_asset_metadata_full(asset_id, include_history)
        .map_err(|e| {
            error!("Failed to get full asset metadata for {}: {}", asset_id, e);
            e
        })?;

    info!("Retrieved full asset metadata for asset {} for user {}", asset_id, session.user_id);
    Ok(result)
}

/// Update asset metadata partially
#[tauri::command]
pub async fn update_asset_metadata_partial(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    asset_id: u32,
    updates: PartialMetadata,
) -> Result<crate::metadata::AssetMetadata, String> {
    info!("Updating asset metadata partially for asset: {}", asset_id);
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Engineer+ role can update metadata
    if !matches!(session.role.to_string().as_str(), "Engineer" | "Administrator") {
        return Err("Insufficient permissions to update metadata".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use CRUD API
    let crud_api = MetadataCrudApi::new(conn);
    let result = crud_api.update_asset_metadata_partial(asset_id, updates)
        .map_err(|e| {
            error!("Failed to update asset metadata for {}: {}", asset_id, e);
            e
        })?;

    info!("Updated asset metadata for asset {} for user {}", asset_id, session.user_id);
    Ok(result)
}

/// Copy metadata between assets
#[tauri::command]
pub async fn copy_metadata_between_assets(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    source_id: u32,
    target_id: u32,
    options: CopyOptions,
) -> Result<(), String> {
    info!("Copying metadata from asset {} to asset {}", source_id, target_id);
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Engineer+ role can copy metadata
    if !matches!(session.role.to_string().as_str(), "Engineer" | "Administrator") {
        return Err("Insufficient permissions to copy metadata".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use CRUD API
    let crud_api = MetadataCrudApi::new(conn);
    crud_api.copy_metadata_between_assets(source_id, target_id, options)
        .map_err(|e| {
            error!("Failed to copy metadata from {} to {}: {}", source_id, target_id, e);
            e
        })?;

    info!("Copied metadata from asset {} to asset {} for user {}", source_id, target_id, session.user_id);
    Ok(())
}

/// Validate multiple metadata records in batch
#[tauri::command]
pub async fn validate_metadata_batch(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    data: Vec<MetadataValidationRequest>,
) -> Result<Vec<crate::metadata::api::ValidationResult>, String> {
    info!("Validating {} metadata records in batch", data.len());
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use CRUD API
    let crud_api = MetadataCrudApi::new(conn);
    let results = crud_api.validate_metadata_batch(data)
        .map_err(|e| {
            error!("Failed to validate metadata batch: {}", e);
            e
        })?;

    info!("Validated {} metadata records for user {}", results.len(), session.user_id);
    Ok(results)
}

/// Test metadata schema with test cases
#[tauri::command]
pub async fn test_metadata_schema(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    schema_id: u32,
    test_data: Vec<TestCase>,
) -> Result<TestResults, String> {
    info!("Testing metadata schema {} with {} test cases", schema_id, test_data.len());
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use CRUD API
    let crud_api = MetadataCrudApi::new(conn);
    let results = crud_api.test_metadata_schema(schema_id, test_data)
        .map_err(|e| {
            error!("Failed to test metadata schema {}: {}", schema_id, e);
            e
        })?;

    info!("Tested metadata schema {} for user {}", schema_id, session.user_id);
    Ok(results)
}

// === ADVANCED QUERY API COMMANDS ===

/// Execute complex metadata query
#[tauri::command]
pub async fn query_assets_by_metadata(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    query: MetadataQuery,
) -> Result<Vec<crate::assets::Asset>, String> {
    info!("Executing complex metadata query");
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Query API
    let query_api = MetadataQueryApi::new(conn);
    let results = query_api.query_assets_by_metadata(query)
        .map_err(|e| {
            error!("Failed to execute metadata query: {}", e);
            e
        })?;

    info!("Executed metadata query, found {} assets for user {}", results.len(), session.user_id);
    Ok(results)
}

/// Get metadata field statistics
#[tauri::command]
pub async fn get_metadata_field_statistics(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    config: FieldStatsConfig,
) -> Result<Vec<FieldStatistics>, String> {
    info!("Getting metadata field statistics for {} fields", config.field_paths.len());
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Query API
    let query_api = MetadataQueryApi::new(conn);
    let results = query_api.get_metadata_field_statistics(config)
        .map_err(|e| {
            error!("Failed to get field statistics: {}", e);
            e
        })?;

    info!("Retrieved statistics for {} fields for user {}", results.len(), session.user_id);
    Ok(results)
}

/// Search metadata values with full-text search
#[tauri::command]
pub async fn search_metadata_values(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    config: SearchConfig,
) -> Result<SearchResults, String> {
    info!("Searching metadata values with term: {}", config.search_term);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Query API
    let query_api = MetadataQueryApi::new(conn);
    let results = query_api.search_metadata_values(config)
        .map_err(|e| {
            error!("Failed to search metadata values: {}", e);
            e
        })?;

    info!("Found {} search results for user {}", results.results.len(), session.user_id);
    Ok(results)
}

/// Aggregate metadata data
#[tauri::command]
pub async fn aggregate_metadata_data(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    config: AggregationConfig,
) -> Result<AggregationResults, String> {
    info!("Aggregating metadata data with {} group by fields", config.group_by_fields.len());
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Query API
    let query_api = MetadataQueryApi::new(conn);
    let results = query_api.aggregate_metadata_data(config)
        .map_err(|e| {
            error!("Failed to aggregate metadata data: {}", e);
            e
        })?;

    info!("Aggregated metadata data into {} groups for user {}", results.groups.len(), session.user_id);
    Ok(results)
}

// === BULK OPERATIONS API COMMANDS ===

/// Execute bulk metadata operations
#[tauri::command]
pub async fn bulk_update_metadata(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    operations: Vec<BulkMetadataOperation>,
) -> Result<BulkOperationSummary, String> {
    info!("Executing {} bulk metadata operations", operations.len());
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Engineer+ role can perform bulk operations
    if !matches!(session.role.to_string().as_str(), "Engineer" | "Administrator") {
        return Err("Insufficient permissions to perform bulk metadata operations".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Bulk API
    let bulk_api = MetadataBulkApi::new(conn);
    let results = bulk_api.bulk_update_metadata(operations)
        .map_err(|e| {
            error!("Failed to execute bulk metadata operations: {}", e);
            e
        })?;

    info!("Executed bulk metadata operations: {} successful, {} failed for user {}", 
          results.successful_operations, results.failed_operations, session.user_id);
    Ok(results)
}

/// Validate multiple metadata records
#[tauri::command]
pub async fn bulk_validate_metadata(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    data: BulkValidationData,
) -> Result<BulkValidationSummary, String> {
    info!("Bulk validating {} metadata records", data.validations.len());
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Bulk API
    let bulk_api = MetadataBulkApi::new(conn);
    let results = bulk_api.bulk_validate_metadata(data)
        .map_err(|e| {
            error!("Failed to bulk validate metadata: {}", e);
            e
        })?;

    info!("Bulk validated {} metadata records: {} valid, {} invalid for user {}", 
          results.total_validations, results.valid_records, results.invalid_records, session.user_id);
    Ok(results)
}

/// Bulk delete metadata based on criteria
#[tauri::command]
pub async fn bulk_delete_metadata(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    criteria: BulkDeletionCriteria,
) -> Result<BulkDeletionResult, String> {
    info!("Bulk deleting metadata with criteria");
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Administrator role can perform bulk deletion
    if !matches!(session.role.to_string().as_str(), "Administrator") {
        return Err("Insufficient permissions to perform bulk metadata deletion".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Bulk API
    let bulk_api = MetadataBulkApi::new(conn);
    let results = bulk_api.bulk_delete_metadata(criteria)
        .map_err(|e| {
            error!("Failed to bulk delete metadata: {}", e);
            e
        })?;

    info!("Bulk deleted {} metadata records for user {}", results.deletion_count, session.user_id);
    Ok(results)
}

/// Apply schema to multiple assets
#[tauri::command]
pub async fn bulk_apply_schema(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    config: SchemaApplicationConfig,
) -> Result<Vec<SchemaApplicationResult>, String> {
    info!("Applying schema {} to {} assets", config.schema_id, config.target_assets.len());
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Engineer+ role can apply schemas
    if !matches!(session.role.to_string().as_str(), "Engineer" | "Administrator") {
        return Err("Insufficient permissions to apply schemas".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Bulk API
    let bulk_api = MetadataBulkApi::new(conn);
    let results = bulk_api.bulk_apply_schema(config)
        .map_err(|e| {
            error!("Failed to apply schema to assets: {}", e);
            e
        })?;

    let successful_count = results.iter().filter(|r| r.success).count();
    info!("Applied schema to {} assets successfully for user {}", successful_count, session.user_id);
    Ok(results)
}

/// Import large metadata dataset in batches
#[tauri::command]
pub async fn batch_import_metadata(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    data: Vec<serde_json::Value>,
    config: BatchImportConfig,
) -> Result<BatchImportResult, String> {
    info!("Batch importing {} metadata records", data.len());
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Engineer+ role can import metadata
    if !matches!(session.role.to_string().as_str(), "Engineer" | "Administrator") {
        return Err("Insufficient permissions to import metadata".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Bulk API
    let bulk_api = MetadataBulkApi::new(conn);
    let results = bulk_api.batch_import_metadata(data, config)
        .map_err(|e| {
            error!("Failed to batch import metadata: {}", e);
            e
        })?;

    info!("Batch imported {} metadata records: {} successful, {} failed for user {}", 
          results.total_records, results.imported_records, results.failed_records, session.user_id);
    Ok(results)
}

// === EXPORT/IMPORT API COMMANDS ===

/// Export metadata to JSON format
#[tauri::command]
pub async fn export_metadata_to_json(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    config: ExportConfig,
) -> Result<String, String> {
    info!("Exporting metadata to JSON format");
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Export/Import API
    let export_api = MetadataExportImportApi::new(conn);
    let result = export_api.export_metadata_to_json(config)
        .map_err(|e| {
            error!("Failed to export metadata to JSON: {}", e);
            e
        })?;

    info!("Exported metadata to JSON for user {}", session.user_id);
    Ok(result)
}

/// Export metadata to CSV format
#[tauri::command]
pub async fn export_metadata_to_csv(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    config: ExportConfig,
) -> Result<String, String> {
    info!("Exporting metadata to CSV format");
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Export/Import API
    let export_api = MetadataExportImportApi::new(conn);
    let result = export_api.export_metadata_to_csv(config)
        .map_err(|e| {
            error!("Failed to export metadata to CSV: {}", e);
            e
        })?;

    info!("Exported metadata to CSV for user {}", session.user_id);
    Ok(result)
}

/// Export metadata to XML format
#[tauri::command]
pub async fn export_metadata_to_xml(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    config: ExportConfig,
) -> Result<String, String> {
    info!("Exporting metadata to XML format");
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Export/Import API
    let export_api = MetadataExportImportApi::new(conn);
    let result = export_api.export_metadata_to_xml(config)
        .map_err(|e| {
            error!("Failed to export metadata to XML: {}", e);
            e
        })?;

    info!("Exported metadata to XML for user {}", session.user_id);
    Ok(result)
}

/// Import metadata from file
#[tauri::command]
pub async fn import_metadata_from_file(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    file_content: String,
    config: ImportConfig,
) -> Result<ImportResult, String> {
    info!("Importing metadata from file");
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Engineer+ role can import metadata
    if !matches!(session.role.to_string().as_str(), "Engineer" | "Administrator") {
        return Err("Insufficient permissions to import metadata".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Export/Import API
    let import_api = MetadataExportImportApi::new(conn);
    let result = import_api.import_metadata_from_file(file_content, config)
        .map_err(|e| {
            error!("Failed to import metadata from file: {}", e);
            e
        })?;

    info!("Imported {} metadata records for user {}", result.imported_records, session.user_id);
    Ok(result)
}

/// Validate import data
#[tauri::command]
pub async fn validate_import_data(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    data: String,
    format: crate::metadata::api::export::ImportFormat,
) -> Result<ImportValidationResult, String> {
    info!("Validating import data");
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Export/Import API
    let import_api = MetadataExportImportApi::new(conn);
    let result = import_api.validate_import_data(data, format)
        .map_err(|e| {
            error!("Failed to validate import data: {}", e);
            e
        })?;

    info!("Validated import data: {} records, {} errors for user {}", 
          result.record_count, result.validation_errors.len(), session.user_id);
    Ok(result)
}

// === INTEGRATION API COMMANDS ===

/// Get metadata API information
#[tauri::command]
pub async fn get_metadata_api_info(
    session_state: State<'_, SessionManagerState>,
    token: String,
) -> Result<ApiInfo, String> {
    info!("Getting metadata API information");
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let _session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Return API information (doesn't require database connection)
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let integration_api = MetadataIntegrationApi::new(&conn);
    let result = integration_api.get_metadata_api_info()
        .map_err(|e| {
            error!("Failed to get API info: {}", e);
            e
        })?;

    info!("Retrieved metadata API information");
    Ok(result)
}

/// Create external metadata mapping
#[tauri::command]
pub async fn create_external_metadata_mapping(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    config: ExternalMappingConfig,
) -> Result<i64, String> {
    info!("Creating external metadata mapping: {}", config.name);
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Administrator role can create external mappings
    if !matches!(session.role.to_string().as_str(), "Administrator") {
        return Err("Insufficient permissions to create external metadata mappings".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Integration API
    let integration_api = MetadataIntegrationApi::new(conn);
    let result = integration_api.create_external_metadata_mapping(config)
        .map_err(|e| {
            error!("Failed to create external metadata mapping: {}", e);
            e
        })?;

    info!("Created external metadata mapping {} for user {}", result, session.user_id);
    Ok(result)
}

/// Create metadata webhook
#[tauri::command]
pub async fn create_metadata_webhook(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    config: WebhookConfig,
) -> Result<i64, String> {
    info!("Creating metadata webhook: {}", config.name);
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Administrator role can create webhooks
    if !matches!(session.role.to_string().as_str(), "Administrator") {
        return Err("Insufficient permissions to create metadata webhooks".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Integration API
    let integration_api = MetadataIntegrationApi::new(conn);
    let result = integration_api.create_metadata_webhook(config)
        .map_err(|e| {
            error!("Failed to create metadata webhook: {}", e);
            e
        })?;

    info!("Created metadata webhook {} for user {}", result, session.user_id);
    Ok(result)
}

/// Get metadata sync status
#[tauri::command]
pub async fn get_metadata_sync_status(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    sync_source: String,
) -> Result<SyncResults, String> {
    info!("Getting metadata sync status for: {}", sync_source);
    
    // Verify session
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Integration API
    let integration_api = MetadataIntegrationApi::new(conn);
    let result = integration_api.get_metadata_sync_status(sync_source)
        .map_err(|e| {
            error!("Failed to get sync status: {}", e);
            e
        })?;

    info!("Retrieved sync status for user {}", session.user_id);
    Ok(result)
}

/// Sync external metadata source
#[tauri::command]
pub async fn sync_external_metadata_source(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    token: String,
    config: ExternalSyncConfig,
) -> Result<SyncResults, String> {
    info!("Syncing external metadata source: {}", config.source_system);
    
    // Verify session and check permissions
    let session_manager_guard = session_state.lock().map_err(|_| "Session lock error")?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    
    // Only Administrator role can sync external sources
    if !matches!(session.role.to_string().as_str(), "Administrator") {
        return Err("Insufficient permissions to sync external metadata sources".to_string());
    }
    drop(session_manager_guard);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Database lock error")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Use Integration API
    let integration_api = MetadataIntegrationApi::new(conn);
    let result = integration_api.sync_external_metadata_source(config)
        .map_err(|e| {
            error!("Failed to sync external metadata source: {}", e);
            e
        })?;

    info!("Synced external metadata source for user {}", session.user_id);
    Ok(result)
}