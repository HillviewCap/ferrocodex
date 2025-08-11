// Firmware management commands

use crate::auth::SessionManager;
use crate::database::Database;
use crate::users::UserRole;
use crate::validation::{InputSanitizer, RateLimiter};
use crate::assets::{AssetRepository, SqliteAssetRepository};
use crate::configurations::{ConfigurationRepository, SqliteConfigurationRepository, ConfigurationVersionInfo, file_utils};
use crate::audit::{AuditRepository, SqliteAuditRepository, AuditEventRequest, AuditEventType};
use crate::firmware::{FirmwareRepository, SqliteFirmwareRepository, CreateFirmwareRequest, FirmwareVersionInfo, FirmwareFileStorage, FirmwareStatus, FirmwareStatusHistory, get_firmware_storage_dir};
use crate::firmware_analysis::{FirmwareAnalysisRepository, SqliteFirmwareAnalysisRepository, FirmwareAnalysisResult, AnalysisQueue, AnalysisJob};
use crate::recovery::{RecoveryExporter, RecoveryExportRequest, RecoveryManifest};
use crate::vault::{VaultRepository, SqliteVaultRepository};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State, Manager};
use tracing::{error, info, warn};
use serde_json;

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;
type RateLimiterState = Mutex<RateLimiter>;

fn get_or_create_analysis_queue(app: &AppHandle) -> Arc<AnalysisQueue> {
    match app.try_state::<Arc<AnalysisQueue>>() {
        Some(queue) => queue.inner().clone(),
        None => {
            // Initialize the queue on first use
            let new_queue = Arc::new(AnalysisQueue::new(app.clone()));
            app.manage(new_queue.clone());
            new_queue
        }
    }
}

#[tauri::command]
pub async fn link_firmware_to_configuration(
    token: String,
    config_id: i64,
    firmware_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    // Only Engineers and Administrators can link firmware
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to link firmware: {}", session.username);
        return Err("Only Engineers and Administrators can link firmware to configurations".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            let audit_repo = SqliteAuditRepository::new(db.get_connection());
            
            match config_repo.link_firmware_to_configuration(config_id, firmware_id) {
                Ok(_) => {
                    // Create audit event
                    let audit_event = AuditEventRequest {
                        event_type: AuditEventType::DatabaseOperation,
                        user_id: Some(session.user_id),
                        username: Some(session.username.clone()),
                        admin_user_id: None,
                        admin_username: None,
                        target_user_id: None,
                        target_username: None,
                        description: format!("Firmware {} linked to configuration {}", firmware_id, config_id),
                        metadata: Some(serde_json::json!({
                            "config_id": config_id,
                            "firmware_id": firmware_id,
                            "linked_by": session.username
                        }).to_string()),
                        ip_address: None,
                        user_agent: None,
                    };
                    
                    if let Err(e) = audit_repo.log_event(&audit_event) {
                        error!("Failed to log audit event: {}", e);
                    }
                    
                    info!("Firmware linked by {}: Config {} <-> Firmware {}", session.username, config_id, firmware_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to link firmware to configuration: {}", e);
                    Err(format!("Failed to link firmware to configuration: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn unlink_firmware_from_configuration(
    token: String,
    config_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    // Only Engineers and Administrators can unlink firmware
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to unlink firmware: {}", session.username);
        return Err("Only Engineers and Administrators can unlink firmware from configurations".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            let audit_repo = SqliteAuditRepository::new(db.get_connection());
            
            match config_repo.unlink_firmware_from_configuration(config_id) {
                Ok(_) => {
                    // Create audit event
                    let audit_event = AuditEventRequest {
                        event_type: AuditEventType::DatabaseOperation,
                        user_id: Some(session.user_id),
                        username: Some(session.username.clone()),
                        admin_user_id: None,
                        admin_username: None,
                        target_user_id: None,
                        target_username: None,
                        description: format!("Firmware unlinked from configuration {}", config_id),
                        metadata: Some(serde_json::json!({
                            "config_id": config_id,
                            "unlinked_by": session.username
                        }).to_string()),
                        ip_address: None,
                        user_agent: None,
                    };
                    
                    if let Err(e) = audit_repo.log_event(&audit_event) {
                        error!("Failed to log audit event: {}", e);
                    }
                    
                    info!("Firmware unlinked by {}: Config {}", session.username, config_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to unlink firmware from configuration: {}", e);
                    Err(format!("Failed to unlink firmware from configuration: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn upload_firmware(
    app: AppHandle,
    token: String,
    asset_id: i64,
    vendor: Option<String>,
    model: Option<String>,
    version: String,
    notes: Option<String>,
    file_path: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    audit_state: State<'_, DatabaseState>,
    rate_limiter: State<'_, RateLimiterState>,
) -> Result<FirmwareVersionInfo, String> {
    // Check rate limiting - limit firmware uploads per user
    {
        let rate_limiter_guard = rate_limiter.lock()
            .map_err(|_| "Failed to acquire rate limiter lock".to_string())?;
        if let Err(e) = rate_limiter_guard.check_rate_limit(&format!("firmware_upload_{}", token)) {
            return Err(e);
        }
    }

    // Validate session
    let session = {
        let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
        match session_manager_guard.validate_session(&token) {
            Ok(Some(session)) => session,
            Ok(None) => return Err("Invalid or expired session".to_string()),
            Err(e) => {
                error!("Session validation error: {}", e);
                return Err("Session validation error".to_string());
            }
        }
    };

    // Only Engineers and Administrators can upload firmware
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to upload firmware: {}", session.username);
        return Err("Only Engineers and Administrators can upload firmware".to_string());
    }

    // Validate file path first
    let file_path = file_path.trim();
    if file_path.is_empty() {
        return Err("File path cannot be empty".to_string());
    }
    
    if let Err(e) = InputSanitizer::validate_file_path(&file_path) {
        error!("Invalid file path: {}", e);
        return Err(format!("Invalid file path: {}", e));
    }

    // Validate inputs
    let version = InputSanitizer::sanitize_string(&version);
    let vendor = vendor.map(|v| InputSanitizer::sanitize_string(&v));
    let model = model.map(|m| InputSanitizer::sanitize_string(&m));
    let notes = notes.map(|n| InputSanitizer::sanitize_string(&n));
    
    if version.is_empty() {
        return Err("Version cannot be empty".to_string());
    }
    
    if let Some(ref v) = vendor {
        if v.len() > 100 {
            return Err("Vendor name cannot exceed 100 characters".to_string());
        }
    }
    
    if let Some(ref m) = model {
        if m.len() > 100 {
            return Err("Model name cannot exceed 100 characters".to_string());
        }
    }
    
    if let Some(ref n) = notes {
        if n.len() > 500 {
            return Err("Notes cannot exceed 500 characters".to_string());
        }
    }
    
    // Check for malicious input
    if InputSanitizer::is_potentially_malicious(&version) || 
       vendor.as_ref().map_or(false, |v| InputSanitizer::is_potentially_malicious(v)) ||
       model.as_ref().map_or(false, |m| InputSanitizer::is_potentially_malicious(m)) ||
       notes.as_ref().map_or(false, |n| InputSanitizer::is_potentially_malicious(n)) {
        error!("Potentially malicious input detected in upload_firmware");
        return Err("Invalid input detected. Please avoid using special characters or script-like patterns.".to_string());
    }
    
    // Validate file extension
    let allowed_extensions = vec![
        "bin", "hex", "img", "rom", "fw", "elf", "dfu", "upd", 
        "dat", "firmware", "update", "pkg", "ipk", "tar", "gz",
        "bz2", "xz", "zip", "rar", "7z", "cab", "iso", "dmg"
    ];
    
    let file_extension = std::path::Path::new(&file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_default();
    
    if file_extension.is_empty() {
        error!("Firmware file has no extension: {}", file_path);
        return Err("Firmware file must have a valid extension".to_string());
    }
    
    if !allowed_extensions.contains(&file_extension.as_str()) {
        error!("Invalid firmware file type: .{}", file_extension);
        return Err(format!(
            "File type .{} is not allowed. Allowed types: {}", 
            file_extension, 
            allowed_extensions.join(", ")
        ));
    }
    
    info!("Validated firmware file extension: .{}", file_extension);
    
    // Read file content
    let file_size = std::fs::metadata(&file_path)
        .map_err(|e| format!("Failed to get file metadata: {}", e))?
        .len();
    info!("Reading file content: {} bytes expected", file_size);
    let file_data = match file_utils::read_firmware_file_content(&file_path) {
        Ok(content) => {
            info!("Successfully read file content: {} bytes", content.len());
            content
        },
        Err(e) => {
            error!("Failed to read file {}: {}", file_path, e);
            return Err(format!("Failed to read file: {}", e));
        }
    };

    // Basic MIME type validation - check for common executable signatures
    if file_data.len() >= 2 {
        let header = &file_data[0..2];
        // Check for Windows executable (MZ header)
        if header == b"MZ" {
            error!("Detected Windows executable file signature");
            return Err("Executable files are not allowed as firmware".to_string());
        }
        // Check for ELF executable (common on Linux)
        if file_data.len() >= 4 && &file_data[0..4] == b"\x7FELF" {
            // ELF is actually allowed for firmware, so we'll permit this
            info!("Detected ELF format firmware file");
        }
    }

    // Store values needed for analysis before acquiring lock
    let firmware_result = {
        let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
        match db_guard.as_ref() {
            Some(db) => {
                let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
                
                // Generate a temporary firmware ID for file storage
            let temp_firmware_id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            
            // Store the firmware file
            info!("Starting firmware file storage process for {} bytes", file_data.len());
            let (file_path, file_hash, file_size) = match FirmwareFileStorage::store_firmware_file(
                &app,
                asset_id,
                temp_firmware_id,
                &file_data,
                session.user_id,
                &session.username,
            ) {
                Ok(result) => {
                    info!("Successfully stored firmware file: path={}, hash={}, size={}", result.0, result.1, result.2);
                    result
                },
                Err(e) => {
                    error!("Failed to store firmware file: {}", e);
                    return Err(format!("Failed to store firmware file: {}", e));
                }
            };
            
            // Create firmware record
            let request = CreateFirmwareRequest {
                asset_id,
                vendor,
                model,
                version,
                notes,
            };
            
            info!("Creating firmware database record");
            match firmware_repo.create_firmware(request, session.user_id, file_path.clone(), file_hash, file_size) {
                Ok(firmware) => {
                    info!("Successfully created firmware record with ID: {}", firmware.id);
                    // Update the file with the actual firmware ID
                    info!("Checking if file rename is needed");
                    let new_file_path = std::path::PathBuf::from(asset_id.to_string())
                        .join(format!("{}.enc", firmware.id))
                        .to_string_lossy()
                        .to_string();
                    info!("Comparing paths - old: '{}', new: '{}'", file_path, new_file_path);
                    if file_path != new_file_path {
                        // Rename the file if needed
                        let firmware_dir = get_firmware_storage_dir(&app).unwrap();
                        let old_path = firmware_dir.join(&file_path);
                        let new_path = firmware_dir.join(&new_file_path);
                        if let Err(e) = std::fs::rename(&old_path, &new_path) {
                            error!("Failed to rename firmware file: {}", e);
                            // Clean up on failure
                            let _ = firmware_repo.delete_firmware(firmware.id);
                            let _ = FirmwareFileStorage::delete_firmware_file(&app, &file_path);
                            return Err("Failed to finalize firmware storage".to_string());
                        }
                        info!("File renamed successfully from {:?} to {:?}", old_path, new_path);
                        
                        // Update the database with the new file path
                        if let Err(e) = firmware_repo.update_firmware_file_path(firmware.id, new_file_path.clone()) {
                            error!("Failed to update firmware file path in database: {}", e);
                            // Try to rename back on failure
                            let _ = std::fs::rename(&new_path, &old_path);
                            let _ = firmware_repo.delete_firmware(firmware.id);
                            return Err("Failed to update firmware file path".to_string());
                        }
                        info!("Database updated with new file path");
                    } else {
                        info!("File rename not needed, paths match");
                    }
                    
                    // Note: Audit logging moved outside database transaction to prevent deadlock
                    
                    info!("Creating FirmwareVersionInfo");
                    info!("Firmware uploaded by {}: v{} for asset {} (ID: {})", 
                         session.username, firmware.version, asset_id, firmware.id);
                    
                    // Convert to FirmwareVersionInfo
                    let firmware_info = FirmwareVersionInfo {
                        id: firmware.id,
                        asset_id: firmware.asset_id,
                        author_id: firmware.author_id,
                        author_username: session.username.clone(),
                        vendor: firmware.vendor,
                        model: firmware.model,
                        version: firmware.version,
                        notes: firmware.notes,
                        status: firmware.status,
                        file_path: new_file_path,
                        file_hash: firmware.file_hash,
                        file_size,
                        created_at: firmware.created_at,
                    };
                    
                    info!("Returning from database scope with firmware info");
                    Ok((firmware_info, firmware.id))
                }
                Err(e) => {
                    // Clean up file on database error
                    let _ = FirmwareFileStorage::delete_firmware_file(&app, &file_path);
                    error!("Failed to create firmware record: {}", e);
                    Err(format!("Failed to create firmware record: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
        }
    };
    
    // Handle the result and queue analysis if successful
    match firmware_result {
        Ok((firmware_info, firmware_id)) => {
            // Log audit event (moved here to avoid deadlock)
            info!("Logging audit event");
            if let Ok(audit_guard) = audit_state.lock() {
                if let Some(audit_db) = audit_guard.as_ref() {
                    let audit_repo = SqliteAuditRepository::new(audit_db.get_connection());
                    let audit_event = AuditEventRequest {
                        event_type: AuditEventType::FirmwareUpload,
                        user_id: Some(session.user_id),
                        username: Some(session.username.clone()),
                        admin_user_id: None,
                        admin_username: None,
                        target_user_id: None,
                        target_username: None,
                        description: format!("Uploaded firmware v{} for asset {}", firmware_info.version, asset_id),
                        metadata: Some(serde_json::json!({
                            "asset_id": asset_id,
                            "firmware_id": firmware_id,
                            "version": firmware_info.version,
                            "file_size": firmware_info.file_size,
                        }).to_string()),
                        ip_address: None,
                        user_agent: None,
                    };
                    if let Err(e) = audit_repo.log_event(&audit_event) {
                        error!("Failed to log audit event: {}", e);
                    } else {
                        info!("Audit event logged successfully");
                    }
                }
            } else {
                warn!("Could not acquire audit lock for logging");
            }
            
            // Queue firmware analysis
            let queue = get_or_create_analysis_queue(&app);
            
            let analysis_job = AnalysisJob {
                firmware_id,
                user_id: session.user_id,
                username: session.username.clone(),
            };
            
            info!("Attempting to queue firmware analysis for firmware ID: {}", firmware_id);
            if let Err(e) = queue.queue_analysis(analysis_job).await {
                warn!("Failed to queue firmware analysis: {}", e);
                // Don't fail the upload if analysis queueing fails
            } else {
                info!("Firmware analysis queued for firmware ID: {}", firmware_id);
            }
            
            info!("Upload process completed successfully, returning firmware info");
            Ok(firmware_info)
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn get_firmware_list(
    token: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<FirmwareVersionInfo>, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            
            match firmware_repo.get_firmware_by_asset(asset_id) {
                Ok(firmwares) => {
                    info!("Retrieved {} firmware versions for asset {} by {}", 
                         firmwares.len(), asset_id, session.username);
                    Ok(firmwares)
                }
                Err(e) => {
                    error!("Failed to get firmware list: {}", e);
                    Err(format!("Failed to get firmware list: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn delete_firmware(
    app: AppHandle,
    token: String,
    firmware_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    audit_state: State<'_, DatabaseState>,
) -> Result<(), String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    // Only Engineers and Administrators can delete firmware
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to delete firmware: {}", session.username);
        return Err("Only Engineers and Administrators can delete firmware".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            
            // Get firmware details before deletion for audit
            let firmware_info = match firmware_repo.get_firmware_by_id(firmware_id) {
                Ok(Some(fw)) => fw,
                Ok(None) => return Err("Firmware not found".to_string()),
                Err(e) => {
                    error!("Failed to get firmware info: {}", e);
                    return Err("Failed to get firmware info".to_string());
                }
            };
            
            // Delete firmware record and get file path
            match firmware_repo.delete_firmware(firmware_id) {
                Ok(Some(file_path)) => {
                    // Delete the actual file
                    if let Err(e) = FirmwareFileStorage::delete_firmware_file(&app, &file_path) {
                        error!("Failed to delete firmware file: {}", e);
                        // Continue anyway - the database record is already deleted
                    }
                    
                    // Log audit event
                    if let Ok(audit_guard) = audit_state.lock() {
                        if let Some(audit_db) = audit_guard.as_ref() {
                        let audit_repo = SqliteAuditRepository::new(audit_db.get_connection());
                        let audit_event = AuditEventRequest {
                            event_type: AuditEventType::FirmwareDelete,
                            user_id: Some(session.user_id),
                            username: Some(session.username.clone()),
                            admin_user_id: None,
                            admin_username: None,
                            target_user_id: None,
                            target_username: None,
                            description: format!("Deleted firmware v{} for asset {}", firmware_info.version, firmware_info.asset_id),
                            metadata: Some(serde_json::json!({
                                "asset_id": firmware_info.asset_id,
                                "firmware_id": firmware_id,
                                "version": firmware_info.version,
                            }).to_string()),
                            ip_address: None,
                            user_agent: None,
                        };
                        if let Err(e) = audit_repo.log_event(&audit_event) {
                            error!("Failed to log audit event: {}", e);
                        }
                    }
                    }
                    
                    info!("Firmware deleted by {}: ID {} (v{} for asset {})", 
                         session.username, firmware_id, firmware_info.version, firmware_info.asset_id);
                    Ok(())
                }
                Ok(None) => Err("Firmware not found".to_string()),
                Err(e) => {
                    error!("Failed to delete firmware: {}", e);
                    Err(format!("Failed to delete firmware: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_firmware_analysis(
    token: String,
    firmware_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Option<FirmwareAnalysisResult>, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let analysis_repo = SqliteFirmwareAnalysisRepository::new(db.get_connection());
            
            match analysis_repo.get_analysis_by_firmware_id(firmware_id) {
                Ok(analysis) => {
                    info!("Firmware analysis retrieved by {}: Firmware ID {}", session.username, firmware_id);
                    Ok(analysis)
                }
                Err(e) => {
                    error!("Failed to get firmware analysis: {}", e);
                    Err(format!("Failed to get firmware analysis: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn export_complete_recovery(
    app: AppHandle,
    token: String,
    asset_id: i64,
    config_version_id: i64,
    firmware_version_id: i64,
    export_directory: String,
    include_vault: Option<bool>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<RecoveryManifest, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            let audit_repo = SqliteAuditRepository::new(db.get_connection());
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            // Get asset name for proper filename generation
            let asset = match asset_repo.get_asset_by_id(asset_id) {
                Ok(Some(asset)) => asset,
                Ok(None) => return Err("Asset not found".to_string()),
                Err(e) => return Err(format!("Failed to get asset: {}", e)),
            };

            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            let exporter = RecoveryExporter::new(&config_repo, &firmware_repo, &vault_repo, &audit_repo);
            
            let request = RecoveryExportRequest {
                asset_id,
                config_version_id,
                firmware_version_id,
                export_directory,
                include_vault,
            };

            match exporter.export_complete_recovery(
                &app,
                request,
                session.user_id,
                &session.username,
                &session.role,
                &asset.name,
            ) {
                Ok(manifest) => Ok(manifest),
                Err(e) => Err(e.to_string()),
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_configurations_by_firmware(
    token: String,
    firmware_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<ConfigurationVersionInfo>, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            match config_repo.get_configurations_by_firmware(firmware_id) {
                Ok(configs) => {
                    info!("Linked configurations accessed by {}: Firmware ID {}", session.username, firmware_id);
                    Ok(configs)
                }
                Err(e) => {
                    error!("Failed to get linked configurations: {}", e);
                    Err(format!("Failed to get linked configurations: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn retry_firmware_analysis(
    token: String,
    firmware_id: i64,
    session_manager: State<'_, SessionManagerState>,
    app: AppHandle,
) -> Result<(), String> {
    // Validate session
    let session = {
        let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
        match session_manager_guard.validate_session(&token) {
            Ok(Some(session)) => session,
            Ok(None) => return Err("Invalid or expired session".to_string()),
            Err(e) => {
                error!("Session validation error: {}", e);
                return Err("Session validation error".to_string());
            }
        }
    };

    // Only Engineers and Administrators can retry analysis
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to retry firmware analysis: {}", session.username);
        return Err("Only Engineers and Administrators can retry firmware analysis".to_string());
    }

    // Queue the analysis job
    let job = AnalysisJob {
        firmware_id,
        user_id: session.user_id,
        username: session.username.clone(),
    };

    // Get or create the analysis queue
    let queue = get_or_create_analysis_queue(&app);

    match queue.queue_analysis(job).await {
        Ok(_) => {
            info!("Firmware analysis retry queued by {}: Firmware ID {}", session.username, firmware_id);
            Ok(())
        }
        Err(e) => {
            error!("Failed to queue firmware analysis: {}", e);
            Err(format!("Failed to queue firmware analysis: {}", e))
        }
    }
}

#[tauri::command]
pub async fn update_firmware_status(
    token: String,
    firmware_id: i64,
    new_status: FirmwareStatus,
    reason: Option<String>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    audit_state: State<'_, DatabaseState>,
) -> Result<(), String> {
    // Validate session
    let session = {
        let session_manager_guard = session_manager
            .lock()
            .map_err(|_| "Failed to acquire session manager lock".to_string())?;
        match session_manager_guard.validate_session(&token) {
            Ok(Some(session)) => session,
            Ok(None) => return Err("Invalid or expired session".to_string()),
            Err(e) => {
                error!("Session validation error: {}", e);
                return Err("Session validation error".to_string());
            }
        }
    };

    // Only Engineers and Administrators can update firmware status
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!(
            "User without sufficient permissions attempted to update firmware status: {}",
            session.username
        );
        return Err("Only Engineers and Administrators can update firmware status".to_string());
    }

    // 1) Do the database work in a tight scope and drop the db lock ASAP
    {
        let db_guard = db_state
            .lock()
            .map_err(|_| "Failed to acquire database lock".to_string())?;
        let db = match db_guard.as_ref() {
            Some(db) => db,
            None => return Err("Database not initialized".to_string()),
        };

        let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());

        if let Err(e) = firmware_repo.update_firmware_status(
            firmware_id,
            new_status.clone(),
            session.user_id,
            reason.clone(),
        ) {
            error!("Failed to update firmware status: {}", e);
            return Err(format!("Failed to update firmware status: {}", e));
        }
        // db_guard dropped here
    }

    // 2) Perform audit logging in a separate lock scope AFTER db lock is released
    {
        if let Ok(audit_guard) = audit_state.lock() {
            if let Some(audit_db) = audit_guard.as_ref() {
                let audit_repo = SqliteAuditRepository::new(audit_db.get_connection());
                let audit_event = AuditEventRequest {
                    event_type: AuditEventType::FirmwareStatusChange,
                    user_id: Some(session.user_id),
                    username: Some(session.username.clone()),
                    admin_user_id: None,
                    admin_username: None,
                    target_user_id: None,
                    target_username: None,
                    description: format!(
                        "Firmware {} status updated to {}",
                        firmware_id, new_status
                    ),
                    metadata: Some(
                        serde_json::json!({
                            "firmware_id": firmware_id,
                            "new_status": new_status.to_string(),
                            "reason": reason,
                            "changed_by": session.username
                        })
                        .to_string(),
                    ),
                    ip_address: None,
                    user_agent: None,
                };
                if let Err(e) = audit_repo.log_event(&audit_event) {
                    error!("Failed to log audit event: {}", e);
                    // Do not fail the status update if audit logging fails
                }
            }
        }
    }

    info!(
        "Firmware {} status updated to {} by {}",
        firmware_id, new_status, session.username
    );
    Ok(())
}

#[tauri::command]
pub async fn get_firmware_status_history(
    token: String,
    firmware_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<FirmwareStatusHistory>, String> {
    // Validate session
    let session = {
        let guard = session_manager
            .lock()
            .map_err(|_| "Failed to acquire session manager lock".to_string())?;
        match guard.validate_session(&token) {
            Ok(Some(session)) => session,
            Ok(None) => return Err("Invalid or expired session".to_string()),
            Err(e) => {
                error!("Session validation error: {}", e);
                return Err("Session validation error".to_string());
            }
        }
    };

    // Tight DB scope
    let history = {
        let db_guard = db_state
            .lock()
            .map_err(|_| "Failed to acquire database lock".to_string())?;
        let db = match db_guard.as_ref() {
            Some(db) => db,
            None => return Err("Database not initialized".to_string()),
        };
        let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
        match firmware_repo.get_firmware_status_history(firmware_id) {
            Ok(history) => history,
            Err(e) => {
                error!("Failed to get firmware status history: {}", e);
                return Err(format!("Failed to get firmware status history: {}", e));
            }
        }
    };

    info!(
        "Retrieved firmware status history for firmware {} by {}",
        firmware_id, session.username
    );
    Ok(history)
}

#[tauri::command]
pub async fn get_available_firmware_status_transitions(
    token: String,
    firmware_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<FirmwareStatus>, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            
            match firmware_repo.get_available_firmware_status_transitions(firmware_id, &session.role.to_string()) {
                Ok(transitions) => {
                    info!("Retrieved available firmware status transitions for firmware {} by {}", firmware_id, session.username);
                    Ok(transitions)
                }
                Err(e) => {
                    error!("Failed to get available firmware status transitions: {}", e);
                    Err(format!("Failed to get available firmware status transitions: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn promote_firmware_to_golden(
    token: String,
    firmware_id: i64,
    reason: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    audit_state: State<'_, DatabaseState>,
) -> Result<(), String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    // Only Administrators can promote firmware to Golden
    if session.role != UserRole::Administrator {
        warn!("Non-administrator attempted to promote firmware to Golden: {}", session.username);
        return Err("Only Administrators can promote firmware to Golden status".to_string());
    }

    // Validate reason
    if reason.trim().is_empty() {
        return Err("Reason is required for Golden promotion".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            
            match firmware_repo.promote_firmware_to_golden(firmware_id, session.user_id, reason.clone()) {
                Ok(_) => {
                    // Log audit event
                    if let Ok(audit_guard) = audit_state.lock() {
                        if let Some(audit_db) = audit_guard.as_ref() {
                            let audit_repo = SqliteAuditRepository::new(audit_db.get_connection());
                            let audit_event = AuditEventRequest {
                                event_type: AuditEventType::FirmwareGoldenPromotion,
                                user_id: Some(session.user_id),
                                username: Some(session.username.clone()),
                                admin_user_id: None,
                                admin_username: None,
                                target_user_id: None,
                                target_username: None,
                                description: format!("Firmware {} promoted to Golden status", firmware_id),
                                metadata: Some(serde_json::json!({
                                    "firmware_id": firmware_id,
                                    "reason": reason,
                                    "promoted_by": session.username
                                }).to_string()),
                                ip_address: None,
                                user_agent: None,
                            };
                            if let Err(e) = audit_repo.log_event(&audit_event) {
                                error!("Failed to log audit event: {}", e);
                            }
                        }
                    }
                    
                    info!("Firmware {} promoted to Golden by {}", firmware_id, session.username);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to promote firmware to Golden: {}", e);
                    Err(format!("Failed to promote firmware to Golden: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn update_firmware_notes(
    token: String,
    firmware_id: i64,
    notes: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    audit_state: State<'_, DatabaseState>,
) -> Result<(), String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    // Only Engineers and Administrators can update firmware notes
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to update firmware notes: {}", session.username);
        return Err("Only Engineers and Administrators can update firmware notes".to_string());
    }

    // Sanitize notes
    let sanitized_notes = InputSanitizer::sanitize_string(&notes);
    
    // Check for malicious input
    if InputSanitizer::is_potentially_malicious(&sanitized_notes) {
        error!("Potentially malicious input detected in firmware notes");
        return Err("Invalid input detected. Please avoid using special characters or script-like patterns.".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            
            match firmware_repo.update_firmware_notes(firmware_id, sanitized_notes.clone()) {
                Ok(_) => {
                    // Log audit event
                    if let Ok(audit_guard) = audit_state.lock() {
                        if let Some(audit_db) = audit_guard.as_ref() {
                            let audit_repo = SqliteAuditRepository::new(audit_db.get_connection());
                            let audit_event = AuditEventRequest {
                                event_type: AuditEventType::FirmwareNotesUpdate,
                                user_id: Some(session.user_id),
                                username: Some(session.username.clone()),
                                admin_user_id: None,
                                admin_username: None,
                                target_user_id: None,
                                target_username: None,
                                description: format!("Firmware {} notes updated", firmware_id),
                                metadata: Some(serde_json::json!({
                                    "firmware_id": firmware_id,
                                    "updated_by": session.username
                                }).to_string()),
                                ip_address: None,
                                user_agent: None,
                            };
                            if let Err(e) = audit_repo.log_event(&audit_event) {
                                error!("Failed to log audit event: {}", e);
                            }
                        }
                    }
                    
                    info!("Firmware {} notes updated by {}", firmware_id, session.username);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to update firmware notes: {}", e);
                    Err(format!("Failed to update firmware notes: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}