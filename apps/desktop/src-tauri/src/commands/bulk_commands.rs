use crate::bulk::{
    BulkImportRepository, SqliteBulkImportRepository, BulkImportSession, BulkImportSessionDetails,
    CreateBulkImportSessionRequest, BulkImportStatus, ProgressStatus, ValidationResults,
    ProcessingOptions, ImportTemplate, ImportTemplateConfig, CSVParseResult, BulkOperationStats,
    ValidationSummary, BulkImportItem, BulkItemStatus, AssetPreview, ValidationError,
    ImportTemplateRepository,
    operations::{
        BulkOperationsRepository, SqliteBulkOperationsRepository, BulkOperationService,
        BulkMoveRequest, BulkDeleteRequest, BulkExportRequest, BulkClassifyRequest,
        BulkOperationProgress, ValidationResult, UndoResult, BulkOperationHistory
    },
};
use crate::assets::{AssetRepository, SqliteAssetRepository, CreateAssetRequest, AssetType};
use crate::auth::SessionManager;
use crate::{DatabaseState, SessionManagerState};
use tauri::{command, State, AppHandle};
use tracing::{info, error, warn};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

#[command]
pub async fn create_bulk_import_session(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    session_name: String,
    import_type: String,
    template_path: Option<String>,
) -> Result<BulkImportSession, String> {
    info!("Creating bulk import session: {}", session_name);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Create session request
    let request = CreateBulkImportSessionRequest {
        session_name,
        import_type,
        template_path,
    };

    // Create the session
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    let session = bulk_repo.create_session(request, current_user.id)
        .map_err(|e| {
            error!("Failed to create bulk import session: {}", e);
            format!("Failed to create session: {}", e)
        })?;

    info!("Bulk import session created successfully: {}", session.id);
    Ok(session)
}

#[command]
pub async fn get_bulk_import_sessions(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
) -> Result<Vec<BulkImportSession>, String> {
    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Get sessions for current user
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    let sessions = bulk_repo.get_sessions_by_user(current_user.id)
        .map_err(|e| {
            error!("Failed to get bulk import sessions: {}", e);
            format!("Failed to get sessions: {}", e)
        })?;

    Ok(sessions)
}

#[command]
pub async fn get_bulk_import_session_details(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    session_id: i64,
) -> Result<Option<BulkImportSessionDetails>, String> {
    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Get session details
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    let details = bulk_repo.get_session_details(session_id)
        .map_err(|e| {
            error!("Failed to get session details: {}", e);
            format!("Failed to get session details: {}", e)
        })?;

    Ok(details)
}

#[command]
pub async fn delete_bulk_import_session(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    session_id: i64,
) -> Result<(), String> {
    info!("Deleting bulk import session: {}", session_id);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Check session ownership (users can only delete their own sessions)
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    if let Some(session) = bulk_repo.get_session_by_id(session_id).map_err(|e| e.to_string())? {
        if session.created_by != current_user.id {
            return Err("Permission denied: You can only delete your own sessions".to_string());
        }
    } else {
        return Err("Session not found".to_string());
    }

    // Delete the session
    bulk_repo.delete_session(session_id)
        .map_err(|e| {
            error!("Failed to delete bulk import session: {}", e);
            format!("Failed to delete session: {}", e)
        })?;

    info!("Bulk import session deleted successfully: {}", session_id);
    Ok(())
}

#[command]
pub async fn upload_bulk_import_file(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    session_id: i64,
    file_path: String,
) -> Result<ValidationSummary, String> {
    info!("Uploading bulk import file for session: {}", session_id);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Validate file exists
    if !Path::new(&file_path).exists() {
        return Err("File not found".to_string());
    }

    // Parse CSV file
    let csv_result = parse_csv_file(&file_path)
        .map_err(|e| format!("Failed to parse CSV file: {}", e))?;

    // Validate CSV structure
    let validation_summary = validate_csv_structure(&csv_result)?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Convert CSV rows to bulk import items
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    let items = convert_csv_to_items(&csv_result)?;
    
    // Add items to session
    bulk_repo.add_session_items(session_id, items)
        .map_err(|e| {
            error!("Failed to add items to session: {}", e);
            format!("Failed to add items: {}", e)
        })?;

    info!("File uploaded successfully for session: {}", session_id);
    Ok(validation_summary)
}

#[command]
pub async fn validate_bulk_import_data(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    session_id: i64,
) -> Result<ValidationResults, String> {
    info!("Validating bulk import data for session: {}", session_id);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Get session items
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    let items = bulk_repo.get_session_items(session_id)
        .map_err(|e| {
            error!("Failed to get session items: {}", e);
            format!("Failed to get items: {}", e)
        })?;

    // Validate each item
    let asset_repo = SqliteAssetRepository::new(db.get_connection());
    let validation_results = validate_import_items(&items, &asset_repo)?;

    // Update session status to validated
    if validation_results.is_valid {
        bulk_repo.update_session_status(session_id, BulkImportStatus::Validating)
            .map_err(|e| format!("Failed to update session status: {}", e))?;
    }

    info!("Validation completed for session: {}", session_id);
    Ok(validation_results)
}

#[command]
pub async fn start_bulk_import_processing(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    session_id: i64,
    options: ProcessingOptions,
) -> Result<(), String> {
    info!("Starting bulk import processing for session: {}", session_id);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Update session status to processing
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    bulk_repo.update_session_status(session_id, BulkImportStatus::Processing)
        .map_err(|e| format!("Failed to update session status: {}", e))?;

    // Process items (this would be done in a background task in a real implementation)
    let items = bulk_repo.get_session_items(session_id)
        .map_err(|e| format!("Failed to get items: {}", e))?;

    let asset_repo = SqliteAssetRepository::new(db.get_connection());
    let mut processed = 0;
    let mut failed = 0;

    for item in items {
        match process_import_item(&item, &asset_repo, &bulk_repo, current_user.id, &options) {
            Ok(_) => {
                processed += 1;
                bulk_repo.update_item_status(item.id, BulkItemStatus::Completed, None, None)
                    .map_err(|e| format!("Failed to update item status: {}", e))?;
            }
            Err(e) => {
                failed += 1;
                bulk_repo.update_item_status(item.id, BulkItemStatus::Failed, Some(e.to_string()), None)
                    .map_err(|e| format!("Failed to update item status: {}", e))?;
                warn!("Failed to process item {}: {}", item.id, e);
            }
        }

        // Update progress
        bulk_repo.update_session_progress(session_id, processed, failed)
            .map_err(|e| format!("Failed to update progress: {}", e))?;
    }

    // Update final status
    let final_status = if failed == 0 {
        BulkImportStatus::Completed
    } else if processed == 0 {
        BulkImportStatus::Failed
    } else {
        BulkImportStatus::Completed // Partial completion still counts as completed
    };

    bulk_repo.update_session_status(session_id, final_status)
        .map_err(|e| format!("Failed to update final status: {}", e))?;

    info!("Bulk import processing completed for session: {} (processed: {}, failed: {})", session_id, processed, failed);
    Ok(())
}

#[command]
pub async fn get_bulk_import_progress(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    session_id: i64,
) -> Result<ProgressStatus, String> {
    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Get session
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    let session = bulk_repo.get_session_by_id(session_id)
        .map_err(|e| format!("Failed to get session: {}", e))?
        .ok_or("Session not found")?;

    // Calculate processing rate and estimated completion
    let processing_rate = if session.processed_items > 0 {
        session.processed_items as f64 / 60.0 // Simple rate calculation
    } else {
        0.0
    };

    let estimated_completion = if processing_rate > 0.0 && session.total_items > session.processed_items {
        let remaining = session.total_items - session.processed_items;
        let seconds = remaining as f64 / processing_rate;
        Some(format!("{}s", seconds as i64))
    } else {
        None
    };

    let progress = ProgressStatus {
        session_id: session.id,
        total_items: session.total_items,
        processed_items: session.processed_items,
        failed_items: session.failed_items,
        current_item: None, // Could be enhanced to track current item
        estimated_completion,
        processing_rate,
        status: session.status,
    };

    Ok(progress)
}

#[command]
pub async fn pause_bulk_import(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    session_id: i64,
) -> Result<(), String> {
    info!("Pausing bulk import session: {}", session_id);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Update session status to paused
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    bulk_repo.update_session_status(session_id, BulkImportStatus::Paused)
        .map_err(|e| {
            error!("Failed to pause session: {}", e);
            format!("Failed to pause session: {}", e)
        })?;

    info!("Bulk import session paused: {}", session_id);
    Ok(())
}

#[command]
pub async fn resume_bulk_import(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    session_id: i64,
) -> Result<(), String> {
    info!("Resuming bulk import session: {}", session_id);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Update session status to processing
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    bulk_repo.update_session_status(session_id, BulkImportStatus::Processing)
        .map_err(|e| {
            error!("Failed to resume session: {}", e);
            format!("Failed to resume session: {}", e)
        })?;

    info!("Bulk import session resumed: {}", session_id);
    Ok(())
}

#[command]
pub async fn cancel_bulk_import(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    session_id: i64,
) -> Result<(), String> {
    info!("Cancelling bulk import session: {}", session_id);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Update session status to cancelled
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    bulk_repo.update_session_status(session_id, BulkImportStatus::Cancelled)
        .map_err(|e| {
            error!("Failed to cancel session: {}", e);
            format!("Failed to cancel session: {}", e)
        })?;

    info!("Bulk import session cancelled: {}", session_id);
    Ok(())
}

#[command]
pub async fn get_bulk_operation_stats(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
) -> Result<BulkOperationStats, String> {
    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Get statistics
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    let stats = bulk_repo.get_bulk_operation_stats()
        .map_err(|e| {
            error!("Failed to get bulk operation stats: {}", e);
            format!("Failed to get stats: {}", e)
        })?;

    Ok(stats)
}

#[command]
pub async fn create_import_template(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    template_config: ImportTemplateConfig,
) -> Result<ImportTemplate, String> {
    info!("Creating import template: {}", template_config.template_name);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Create the template
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    let template = bulk_repo.create_template(template_config, current_user.id)
        .map_err(|e| {
            error!("Failed to create import template: {}", e);
            format!("Failed to create template: {}", e)
        })?;

    info!("Import template created successfully: {}", template.id);
    Ok(template)
}

#[command]
pub async fn get_import_templates(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    template_type: String,
) -> Result<Vec<ImportTemplate>, String> {
    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Get templates by type
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    let templates = bulk_repo.get_templates_by_type(&template_type)
        .map_err(|e| {
            error!("Failed to get import templates: {}", e);
            format!("Failed to get templates: {}", e)
        })?;

    Ok(templates)
}

#[command]
pub async fn delete_import_template(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    template_id: i64,
) -> Result<(), String> {
    info!("Deleting import template: {}", template_id);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Check template ownership (users can only delete their own templates)
    let bulk_repo = SqliteBulkImportRepository::new(db.get_connection());
    if let Some(template) = bulk_repo.get_template_by_id(template_id).map_err(|e| e.to_string())? {
        if template.created_by != current_user.id {
            return Err("Permission denied: You can only delete your own templates".to_string());
        }
    } else {
        return Err("Template not found".to_string());
    }

    // Delete the template
    bulk_repo.delete_template(template_id)
        .map_err(|e| {
            error!("Failed to delete import template: {}", e);
            format!("Failed to delete template: {}", e)
        })?;

    info!("Import template deleted successfully: {}", template_id);
    Ok(())
}

#[command]
pub async fn generate_import_template_csv(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    asset_type: String,
    metadata_schema: Option<String>,
) -> Result<String, String> {
    info!("Generating CSV template for asset type: {}", asset_type);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Generate CSV headers based on asset type and metadata schema
    let mut headers = vec!["name".to_string(), "asset_type".to_string(), "description".to_string()];
    
    // Add optional fields
    headers.push("parent_name".to_string());
    
    // If metadata schema is provided, add metadata fields
    if let Some(_schema) = metadata_schema {
        // TODO: Integrate with metadata system to get schema fields
        headers.push("metadata_field1".to_string());
        headers.push("metadata_field2".to_string());
    }

    // Generate CSV content
    let mut csv_content = headers.join(",") + "\n";
    
    // Add example row
    match asset_type.as_str() {
        "Folder" => {
            csv_content.push_str("Production Line 1,Folder,Main production line folder,\n");
        }
        "Device" => {
            csv_content.push_str("PLC-001,Device,Primary PLC controller,Production Line 1\n");
        }
        _ => {
            csv_content.push_str("Example Asset,Device,Example description,\n");
        }
    }

    info!("CSV template generated successfully");
    Ok(csv_content)
}

// Helper functions

fn parse_csv_file(file_path: &str) -> Result<CSVParseResult, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let mut reader = csv::Reader::from_reader(contents.as_bytes());
    let headers = reader.headers()?.clone();
    let mut rows = Vec::new();
    let mut errors = Vec::new();

    for (row_index, result) in reader.records().enumerate() {
        match result {
            Ok(record) => {
                let mut row_map = HashMap::new();
                for (i, field) in record.iter().enumerate() {
                    if let Some(header) = headers.get(i) {
                        row_map.insert(header.to_string(), field.to_string());
                    }
                }
                rows.push(row_map);
            }
            Err(e) => {
                errors.push(crate::bulk::CSVParseError {
                    row: row_index as i64 + 1,
                    column: "unknown".to_string(),
                    message: e.to_string(),
                });
            }
        }
    }

    let total_rows = rows.len() as i64;
    Ok(CSVParseResult {
        headers: headers.iter().map(|h| h.to_string()).collect(),
        rows,
        total_rows,
        errors,
    })
}

fn validate_csv_structure(csv_result: &CSVParseResult) -> Result<ValidationSummary, String> {
    let mut errors = Vec::new();
    let required_headers = vec!["name", "asset_type"];

    // Check required headers
    for required in &required_headers {
        if !csv_result.headers.contains(&required.to_string()) {
            errors.push(ValidationError {
                row: 0,
                field: required.to_string(),
                value: "".to_string(),
                message: format!("Required header '{}' is missing", required),
            });
        }
    }

    // Validate each row
    let mut valid_items = 0;
    for (row_index, row) in csv_result.rows.iter().enumerate() {
        let mut row_valid = true;

        // Check required fields
        for required in &required_headers {
            if let Some(value) = row.get(&required.to_string()) {
                if value.trim().is_empty() {
                    errors.push(ValidationError {
                        row: row_index as i64 + 1,
                        field: required.to_string(),
                        value: value.clone(),
                        message: format!("Field '{}' cannot be empty", required),
                    });
                    row_valid = false;
                }
            } else {
                errors.push(ValidationError {
                    row: row_index as i64 + 1,
                    field: required.to_string(),
                    value: "".to_string(),
                    message: format!("Field '{}' is missing", required),
                });
                row_valid = false;
            }
        }

        // Validate asset_type
        if let Some(asset_type) = row.get("asset_type") {
            if !["Folder", "Device", "folder", "device"].contains(&asset_type.as_str()) {
                errors.push(ValidationError {
                    row: row_index as i64 + 1,
                    field: "asset_type".to_string(),
                    value: asset_type.clone(),
                    message: "Asset type must be 'Folder' or 'Device'".to_string(),
                });
                row_valid = false;
            }
        }

        if row_valid {
            valid_items += 1;
        }
    }

    Ok(ValidationSummary {
        total_items: csv_result.total_rows,
        valid_items,
        invalid_items: csv_result.total_rows - valid_items,
        errors,
    })
}

fn convert_csv_to_items(csv_result: &CSVParseResult) -> Result<Vec<BulkImportItem>, String> {
    let mut items = Vec::new();

    for (row_index, row) in csv_result.rows.iter().enumerate() {
        let item_data = serde_json::to_string(row)
            .map_err(|e| format!("Failed to serialize row {}: {}", row_index + 1, e))?;

        items.push(BulkImportItem {
            id: 0, // Will be assigned by database
            session_id: 0, // Will be set when adding to session
            item_data_json: item_data,
            processing_status: BulkItemStatus::Pending,
            error_message: None,
            asset_id: None,
            processed_at: None,
        });
    }

    Ok(items)
}

fn validate_import_items(
    items: &[BulkImportItem],
    asset_repo: &SqliteAssetRepository,
) -> Result<ValidationResults, String> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut preview_items = Vec::new();
    let mut is_valid = true;

    for (index, item) in items.iter().enumerate() {
        let row_data: HashMap<String, serde_json::Value> = serde_json::from_str(&item.item_data_json)
            .map_err(|e| format!("Failed to parse item data: {}", e))?;

        let row = index as i64 + 1;

        // Extract required fields
        let name = row_data.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
            
        let asset_type = row_data.get("asset_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let description = row_data.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let parent_name = row_data.get("parent_name")
            .and_then(|v| v.as_str());

        // Validate name
        if name.is_empty() {
            errors.push(ValidationError {
                row,
                field: "name".to_string(),
                value: name.to_string(),
                message: "Asset name cannot be empty".to_string(),
            });
            is_valid = false;
        } else if name.len() < 2 {
            errors.push(ValidationError {
                row,
                field: "name".to_string(),
                value: name.to_string(),
                message: "Asset name must be at least 2 characters".to_string(),
            });
            is_valid = false;
        } else if asset_repo.asset_exists_by_name(name).unwrap_or(false) {
            warnings.push(crate::bulk::ValidationWarning {
                row,
                field: "name".to_string(),
                value: name.to_string(),
                message: "Asset with this name already exists".to_string(),
            });
        }

        // Create preview item
        let mut metadata = HashMap::new();
        for (key, value) in &row_data {
            if !["name", "asset_type", "description", "parent_name"].contains(&key.as_str()) {
                metadata.insert(key.clone(), value.clone());
            }
        }

        preview_items.push(AssetPreview {
            row,
            name: name.to_string(),
            description: description.to_string(),
            asset_type: asset_type.to_string(),
            parent_name: parent_name.map(|s| s.to_string()),
            metadata,
        });
    }

    Ok(ValidationResults {
        is_valid,
        errors,
        warnings,
        preview_items,
    })
}

fn process_import_item(
    item: &BulkImportItem,
    asset_repo: &SqliteAssetRepository,
    _bulk_repo: &SqliteBulkImportRepository,
    created_by: i64,
    options: &ProcessingOptions,
) -> Result<i64, Box<dyn std::error::Error>> {
    let row_data: HashMap<String, String> = serde_json::from_str(&item.item_data_json)?;

    let name = row_data.get("name")
        .ok_or("Missing required field: name")?;
        
    let asset_type_str = row_data.get("asset_type")
        .ok_or("Missing required field: asset_type")?;

    let description = row_data.get("description")
        .map(|s| s.clone())
        .unwrap_or_default();

    // Parse asset type
    let asset_type = match asset_type_str.to_lowercase().as_str() {
        "folder" => AssetType::Folder,
        "device" => AssetType::Device,
        _ => return Err("Invalid asset type".into()),
    };

    // Check if asset already exists
    if asset_repo.asset_exists_by_name(name)? {
        if options.skip_existing {
            return Err("Asset already exists (skipped)".into());
        } else if !options.update_existing {
            return Err("Asset already exists".into());
        }
        // TODO: Implement update logic for existing assets
    }

    // Create asset
    let request = CreateAssetRequest {
        name: name.clone(),
        description,
        asset_type,
        parent_id: None, // TODO: Resolve parent by name
        created_by,
    };

    let asset = asset_repo.create_asset(request)?;
    Ok(asset.id)
}

// Bulk Operations Commands (Multi-select operations on existing assets)

#[command]
pub async fn start_bulk_move(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    asset_ids: Vec<i32>,
    new_parent_id: Option<i32>,
    options: serde_json::Value,
) -> Result<String, String> {
    info!("Starting bulk move operation for {} assets", asset_ids.len());

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Parse options
    let bulk_move_options: crate::bulk::operations::BulkMoveOptions = serde_json::from_value(options)
        .map_err(|e| format!("Invalid options format: {}", e))?;

    let request = BulkMoveRequest {
        asset_ids,
        new_parent_id,
        options: bulk_move_options,
    };

    // Create bulk operations service
    let bulk_ops_repo = SqliteBulkOperationsRepository::new(db.get_connection());
    let service = BulkOperationService::new(&bulk_ops_repo);

    let operation_id = service.create_bulk_move_operation(request, current_user.id as i32)
        .map_err(|e| {
            error!("Failed to create bulk move operation: {}", e);
            format!("Failed to create bulk move operation: {}", e)
        })?;

    info!("Created bulk move operation with ID: {}", operation_id);
    Ok(operation_id)
}

#[command]
pub async fn start_bulk_delete(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    asset_ids: Vec<i32>,
    options: serde_json::Value,
) -> Result<String, String> {
    info!("Starting bulk delete operation for {} assets", asset_ids.len());

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Parse options
    let bulk_delete_options: crate::bulk::operations::BulkDeleteOptions = serde_json::from_value(options)
        .map_err(|e| format!("Invalid options format: {}", e))?;

    let request = BulkDeleteRequest {
        asset_ids,
        options: bulk_delete_options,
    };

    // Create bulk operations service
    let bulk_ops_repo = SqliteBulkOperationsRepository::new(db.get_connection());
    let service = BulkOperationService::new(&bulk_ops_repo);

    let operation_id = service.create_bulk_delete_operation(request, current_user.id as i32)
        .map_err(|e| {
            error!("Failed to create bulk delete operation: {}", e);
            format!("Failed to create bulk delete operation: {}", e)
        })?;

    info!("Created bulk delete operation with ID: {}", operation_id);
    Ok(operation_id)
}

#[command]
pub async fn start_bulk_export(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    asset_ids: Vec<i32>,
    format: String,
    options: serde_json::Value,
) -> Result<String, String> {
    info!("Starting bulk export operation for {} assets", asset_ids.len());

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Parse format
    let export_format = crate::bulk::operations::ExportFormat::from_str(&format)
        .map_err(|e| format!("Invalid export format: {}", e))?;

    // Parse options
    let bulk_export_options: crate::bulk::operations::BulkExportOptions = serde_json::from_value(options)
        .map_err(|e| format!("Invalid options format: {}", e))?;

    let request = BulkExportRequest {
        asset_ids,
        format: export_format,
        options: bulk_export_options,
    };

    // Create bulk operations service
    let bulk_ops_repo = SqliteBulkOperationsRepository::new(db.get_connection());
    let service = BulkOperationService::new(&bulk_ops_repo);

    let operation_id = service.create_bulk_export_operation(request, current_user.id as i32)
        .map_err(|e| {
            error!("Failed to create bulk export operation: {}", e);
            format!("Failed to create bulk export operation: {}", e)
        })?;

    info!("Created bulk export operation with ID: {}", operation_id);
    Ok(operation_id)
}

#[command]
pub async fn start_bulk_classify(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    asset_ids: Vec<i32>,
    new_classification: String,
    apply_to_children: bool,
) -> Result<String, String> {
    info!("Starting bulk classify operation for {} assets", asset_ids.len());

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    let request = BulkClassifyRequest {
        asset_ids,
        new_classification,
        apply_to_children,
    };

    // Create bulk operations service
    let bulk_ops_repo = SqliteBulkOperationsRepository::new(db.get_connection());
    let service = BulkOperationService::new(&bulk_ops_repo);

    let operation_id = service.create_bulk_classify_operation(request, current_user.id as i32)
        .map_err(|e| {
            error!("Failed to create bulk classify operation: {}", e);
            format!("Failed to create bulk classify operation: {}", e)
        })?;

    info!("Created bulk classify operation with ID: {}", operation_id);
    Ok(operation_id)
}

#[command]
pub async fn get_bulk_operation_progress(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    operation_id: String,
) -> Result<BulkOperationProgress, String> {
    info!("Getting progress for bulk operation: {}", operation_id);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Create bulk operations service
    let bulk_ops_repo = SqliteBulkOperationsRepository::new(db.get_connection());
    let service = BulkOperationService::new(&bulk_ops_repo);

    let progress = service.get_operation_progress(&operation_id)
        .map_err(|e| {
            error!("Failed to get operation progress: {}", e);
            format!("Failed to get operation progress: {}", e)
        })?;

    Ok(progress)
}

#[command]
pub async fn cancel_bulk_operation(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    operation_id: String,
) -> Result<(), String> {
    info!("Cancelling bulk operation: {}", operation_id);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Create bulk operations service
    let bulk_ops_repo = SqliteBulkOperationsRepository::new(db.get_connection());
    let service = BulkOperationService::new(&bulk_ops_repo);

    service.cancel_operation(&operation_id)
        .map_err(|e| {
            error!("Failed to cancel operation: {}", e);
            format!("Failed to cancel operation: {}", e)
        })?;

    info!("Cancelled bulk operation: {}", operation_id);
    Ok(())
}

#[command]
pub async fn get_bulk_operation_history(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    user_id: Option<i32>,
    limit: Option<i32>,
) -> Result<BulkOperationHistory, String> {
    info!("Getting bulk operation history");

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Create bulk operations repository
    let bulk_ops_repo = SqliteBulkOperationsRepository::new(db.get_connection());

    let history = bulk_ops_repo.get_operation_history(user_id, limit)
        .map_err(|e| {
            error!("Failed to get operation history: {}", e);
            format!("Failed to get operation history: {}", e)
        })?;

    Ok(history)
}

#[command]
pub async fn validate_bulk_move(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    asset_ids: Vec<i32>,
    new_parent_id: Option<i32>,
) -> Result<ValidationResult, String> {
    info!("Validating bulk move for {} assets", asset_ids.len());

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // Validate bulk move operation
    let asset_repo = crate::assets::SqliteAssetRepository::new(db.get_connection());
    let mut validation_result = ValidationResult {
        is_valid: true,
        warnings: vec![],
        errors: vec![],
        conflicts: vec![],
    };

    // Check if all assets exist and are accessible
    for asset_id in &asset_ids {
        match asset_repo.get_asset_by_id(*asset_id as i64) {
            Ok(Some(_)) => {}, // Asset exists
            Ok(None) => {
                validation_result.errors.push(crate::bulk::operations::ValidationError {
                    asset_id: *asset_id,
                    asset_name: format!("Asset {}", asset_id),
                    error_type: "not_found".to_string(),
                    message: "Asset not found or inaccessible".to_string(),
                    blocking: true,
                    suggested_action: Some("Remove from selection".to_string()),
                });
                validation_result.is_valid = false;
            },
            Err(_) => {
                validation_result.errors.push(crate::bulk::operations::ValidationError {
                    asset_id: *asset_id,
                    asset_name: format!("Asset {}", asset_id),
                    error_type: "access_error".to_string(),
                    message: "Cannot access asset".to_string(),
                    blocking: true,
                    suggested_action: Some("Check permissions".to_string()),
                });
                validation_result.is_valid = false;
            }
        }
    }

    // Validate new parent exists if specified
    if let Some(parent_id) = new_parent_id {
        match asset_repo.get_asset_by_id(parent_id as i64) {
            Ok(Some(parent)) => {
                // Check if parent is a folder
                if parent.asset_type != crate::assets::AssetType::Folder {
                    validation_result.errors.push(crate::bulk::operations::ValidationError {
                        asset_id: parent_id,
                        asset_name: parent.name,
                        error_type: "invalid_parent".to_string(),
                        message: "Parent must be a folder".to_string(),
                        blocking: true,
                        suggested_action: Some("Select a folder as parent".to_string()),
                    });
                    validation_result.is_valid = false;
                }
            },
            Ok(None) => {
                validation_result.errors.push(crate::bulk::operations::ValidationError {
                    asset_id: parent_id,
                    asset_name: format!("Asset {}", parent_id),
                    error_type: "parent_not_found".to_string(),
                    message: "Parent folder not found".to_string(),
                    blocking: true,
                    suggested_action: Some("Select a valid parent folder".to_string()),
                });
                validation_result.is_valid = false;
            },
            Err(_) => {
                validation_result.errors.push(crate::bulk::operations::ValidationError {
                    asset_id: parent_id,
                    asset_name: format!("Asset {}", parent_id),
                    error_type: "parent_access_error".to_string(),
                    message: "Cannot access parent folder".to_string(),
                    blocking: true,
                    suggested_action: Some("Check parent folder permissions".to_string()),
                });
                validation_result.is_valid = false;
            }
        }
    }

    // Check for circular references when moving to a specific parent
    if let Some(parent_id) = new_parent_id {
        for asset_id in &asset_ids {
            if *asset_id == parent_id {
                validation_result.errors.push(crate::bulk::operations::ValidationError {
                    asset_id: *asset_id,
                    asset_name: format!("Asset {}", asset_id),
                    error_type: "circular_reference".to_string(),
                    message: "Cannot move asset into itself".to_string(),
                    blocking: true,
                    suggested_action: Some("Remove from selection or choose different parent".to_string()),
                });
                validation_result.is_valid = false;
            }
        }
    }

    Ok(validation_result)
}

#[command]
pub async fn validate_bulk_delete(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    asset_ids: Vec<i32>,
) -> Result<ValidationResult, String> {
    info!("Validating bulk delete for {} assets", asset_ids.len());

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // TODO: Implement actual validation logic
    let validation_result = ValidationResult {
        is_valid: true,
        warnings: vec![],
        errors: vec![],
        conflicts: vec![],
    };

    Ok(validation_result)
}

#[command]
pub async fn validate_bulk_export(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    asset_ids: Vec<i32>,
    format: String,
) -> Result<ValidationResult, String> {
    info!("Validating bulk export for {} assets", asset_ids.len());

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // TODO: Implement actual validation logic
    let validation_result = ValidationResult {
        is_valid: true,
        warnings: vec![],
        errors: vec![],
        conflicts: vec![],
    };

    Ok(validation_result)
}

#[command]
pub async fn validate_bulk_classify(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    asset_ids: Vec<i32>,
    classification: String,
) -> Result<ValidationResult, String> {
    info!("Validating bulk classify for {} assets", asset_ids.len());

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // TODO: Implement actual validation logic
    let validation_result = ValidationResult {
        is_valid: true,
        warnings: vec![],
        errors: vec![],
        conflicts: vec![],
    };

    Ok(validation_result)
}

#[command]
pub async fn undo_bulk_operation(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    operation_id: String,
) -> Result<UndoResult, String> {
    info!("Undoing bulk operation: {}", operation_id);

    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let _current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or("Database not initialized")?;

    // TODO: Implement actual undo logic
    let undo_result = UndoResult {
        success: false,
        reverted_items: 0,
        failed_reversions: vec![],
        message: "Undo functionality not yet implemented".to_string(),
    };

    Ok(undo_result)
}