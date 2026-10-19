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
use crate::vault::{VaultRepository, SqliteVaultRepository, PermissionType};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State, Manager};
use tracing::{error, info, warn};
use serde_json;

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;
type RateLimiterState = Mutex<RateLimiter>;

/// The app's analysis queue, created on first use.
///
/// Creation is serialized: two uploads racing here used to both create a
/// queue, and the loser's queue (whose jobs hold a data key) was used without
/// being managed, so `begin_data_key_rotation` could not count its jobs.
fn get_or_create_analysis_queue(app: &AppHandle) -> Arc<AnalysisQueue> {
    static CREATE: Mutex<()> = Mutex::new(());
    get_or_create_managed(app, &CREATE, || Arc::new(AnalysisQueue::new(app.clone())))
}

/// Returns the managed `T`, creating and managing it first if needed. Only
/// one caller at a time creates (under `create_lock`), and every caller gets
/// the managed instance back, never an unmanaged one that lost a race.
pub(crate) fn get_or_create_managed<R, T>(app: &AppHandle<R>, create_lock: &Mutex<()>, create: impl FnOnce() -> T) -> T
where
    R: tauri::Runtime,
    T: Clone + Send + Sync + 'static,
{
    if let Some(existing) = app.try_state::<T>() {
        return existing.inner().clone();
    }
    let _creating = create_lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(existing) = app.try_state::<T>() {
        return existing.inner().clone();
    }
    app.manage(create());
    app.state::<T>().inner().clone()
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

    let data_key = session.data_key()?;

    // Only Engineers and Administrators can upload firmware
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to upload firmware: {}", session.username);
        return Err("Only Engineers and Administrators can upload firmware".to_string());
    }

    // Validate file path first
    let file_path = file_path.trim();
    validate_firmware_upload_path(file_path)?;

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
    
    let (file_data, original_filename) = read_firmware_upload(file_path)?;

    // Encrypt, store, and record the firmware under the DB lock. The key was
    // fetched before the (possibly long) file read, so store_uploaded_firmware
    // re-checks that it is still the active key.
    let firmware_result = {
        let db_guard = db_state.lock()
            .map_err(|_| "Failed to acquire database lock".to_string())?;
        match db_guard.as_ref() {
            Some(db) => {
                let firmware_dir = get_firmware_storage_dir(&app)
                    .map_err(|e| format!("Failed to locate firmware storage: {}", e))?;
                let request = CreateFirmwareRequest { asset_id, vendor, model, version, notes, original_filename };
                store_uploaded_firmware(
                    db.get_connection(), &firmware_dir, request, session.user_id, &session.username, &file_data, &data_key,
                )
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
                data_key: data_key.clone(),
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

/// Checks the path of a firmware file chosen for upload. Any file type is
/// accepted, with or without an extension: OT vendors use many proprietary
/// formats (a Modicon M580 image is a `.sedp`), and firmware is only stored,
/// encrypted, never executed.
fn validate_firmware_upload_path(file_path: &str) -> Result<(), String> {
    if file_path.is_empty() {
        return Err("File path cannot be empty".to_string());
    }
    if let Err(e) = InputSanitizer::validate_file_path(file_path) {
        error!("Invalid file path: {}", e);
        return Err(format!("Invalid file path: {}", e));
    }
    Ok(())
}

/// Reads a firmware file for upload (up to 2 GB), after checking its path
/// with [`validate_firmware_upload_path`], and returns its content and its
/// sanitized file name, kept as a label for display and export naming.
fn read_firmware_upload(file_path: &str) -> Result<(Vec<u8>, Option<String>), String> {
    validate_firmware_upload_path(file_path)?;
    let original_filename = crate::firmware::sanitize_original_filename(file_path);

    let file_size = std::fs::metadata(file_path)
        .map_err(|e| format!("Failed to get file metadata: {}", e))?
        .len();
    info!("Reading file content: {} bytes expected", file_size);
    let file_data = match file_utils::read_firmware_file_content(file_path) {
        Ok(content) => {
            info!("Successfully read file content: {} bytes", content.len());
            content
        },
        Err(e) => {
            error!("Failed to read file {}: {}", file_path, e);
            return Err(format!("Failed to read file: {}", e));
        }
    };

    // Informational only: executable formats are legitimate firmware
    // packages, and firmware analysis flags them
    if file_data.starts_with(b"MZ") {
        info!("Firmware file has a Windows PE (MZ) signature");
    } else if file_data.starts_with(b"\x7FELF") {
        info!("Detected ELF format firmware file");
    }

    Ok((file_data, original_filename))
}

/// Encrypts `file_data` under `key`, stores it as `<asset>/<id>.enc`, and
/// records it. Call under the DB lock.
///
/// `key` must still be the active data key with no rotation in progress: an
/// upload fetches it before reading the file (up to 2 GB, without any lock),
/// and a rotation may have begun or finished meanwhile. Data stored under the
/// old key would be unreadable and invisible to every future rotation. The
/// key is checked before anything is written, and again by the insert itself
/// (`create_firmware` and the key-version triggers); if the insert fails, the
/// file just written is deleted.
pub(crate) fn store_uploaded_firmware(
    conn: &rusqlite::Connection,
    firmware_dir: &std::path::Path,
    request: CreateFirmwareRequest,
    author_id: i64,
    author_username: &str,
    file_data: &[u8],
    key: &crate::encryption::DataKey,
) -> Result<(FirmwareVersionInfo, i64), String> {
    crate::keystore::ensure_key_is_current(conn, key).map_err(|e| e.to_string())?;
    let firmware_repo = SqliteFirmwareRepository::new(conn);
    let asset_id = request.asset_id;

    // Stored under a temporary name first: the record id is only known after the insert
    let temp_firmware_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    info!("Starting firmware file storage process for {} bytes", file_data.len());
    let (file_path, file_hash, file_size) = FirmwareFileStorage::store_firmware_file_in(
        firmware_dir, asset_id, temp_firmware_id, file_data, key,
    ).map_err(|e| {
        error!("Failed to store firmware file: {}", e);
        format!("Failed to store firmware file: {}", e)
    })?;

    let firmware = match firmware_repo.create_firmware(request, author_id, file_path.clone(), file_hash, file_size, key.version()) {
        Ok(firmware) => firmware,
        Err(e) => {
            // Never leave a file nobody records (it may be under an old key)
            let _ = FirmwareFileStorage::delete_firmware_file_in(firmware_dir, &file_path);
            error!("Failed to create firmware record: {}", e);
            return Err(format!("Failed to create firmware record: {}", e));
        }
    };
    info!("Successfully created firmware record with ID: {}", firmware.id);

    let new_file_path = std::path::PathBuf::from(asset_id.to_string())
        .join(format!("{}.enc", firmware.id))
        .to_string_lossy()
        .to_string();
    if file_path != new_file_path {
        let old_path = firmware_dir.join(&file_path);
        let new_path = firmware_dir.join(&new_file_path);
        if let Err(e) = std::fs::rename(&old_path, &new_path) {
            error!("Failed to rename firmware file: {}", e);
            let _ = firmware_repo.delete_firmware(firmware.id);
            let _ = FirmwareFileStorage::delete_firmware_file_in(firmware_dir, &file_path);
            return Err("Failed to finalize firmware storage".to_string());
        }
        if let Err(e) = firmware_repo.update_firmware_file_path(firmware.id, new_file_path.clone()) {
            error!("Failed to update firmware file path in database: {}", e);
            let _ = firmware_repo.delete_firmware(firmware.id);
            let _ = FirmwareFileStorage::delete_firmware_file_in(firmware_dir, &new_file_path);
            return Err("Failed to update firmware file path".to_string());
        }
    }

    info!("Firmware uploaded by {}: v{} for asset {} (ID: {})", author_username, firmware.version, asset_id, firmware.id);
    let firmware_info = FirmwareVersionInfo {
        id: firmware.id,
        asset_id: firmware.asset_id,
        author_id: firmware.author_id,
        author_username: author_username.to_string(),
        vendor: firmware.vendor,
        model: firmware.model,
        version: firmware.version,
        notes: firmware.notes,
        status: firmware.status,
        file_path: new_file_path,
        file_hash: firmware.file_hash,
        file_size,
        created_at: firmware.created_at,
        original_filename: firmware.original_filename,
    };
    Ok((firmware_info, firmware.id))
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
    vault_passphrase: Option<String>,
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

    let data_key = session.data_key()?;

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
            if include_vault.unwrap_or(false) {
                let vault = vault_repo.get_vault_by_asset_id(asset_id)
                    .map_err(|e| format!("Failed to get vault: {}", e))?;
                if let Some(vault) = vault {
                    super::vault_commands::require_vault_permission(
                        db.get_connection(), &session, vault.vault.id, PermissionType::Export,
                    )?;
                }
            }
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
                &data_key,
                vault_passphrase.as_deref(),
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

    let data_key = session.data_key()?;

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
        data_key: data_key.clone(),
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::KEY_ROTATION_IN_PROGRESS_ERROR;
    use crate::encryption::{DataKey, KdfParams};
    use crate::keystore::rotation::KeyRotation;
    use crate::keystore::{KeyStoreRepository, SqliteKeyStoreRepository, STALE_KEY_ERROR};
    use crate::users::{CreateUserRequest, SqliteUserRepository, UserRepository};
    use rusqlite::Connection;
    use std::path::Path;

    fn setup() -> (Database, i64, DataKey) {
        let db = Database::from_connection(Connection::open_in_memory().unwrap()).unwrap();
        let conn = db.get_connection();
        let admin = SqliteUserRepository::new(conn).create_user(CreateUserRequest {
            username: "admin".into(), password: "password123!".into(), role: UserRole::Administrator,
        }).unwrap().id;
        let (key, _) = SqliteKeyStoreRepository::with_params(conn, KdfParams::fast_for_tests())
            .create_data_key(admin, "admin-password").unwrap();
        conn.execute("INSERT INTO assets (id, name, created_by) VALUES (1, 'plc', ?1)", [admin]).unwrap();
        (db, admin, key)
    }

    fn request(version: &str) -> CreateFirmwareRequest {
        CreateFirmwareRequest { asset_id: 1, vendor: None, model: None, version: version.into(), notes: None, original_filename: None }
    }

    fn files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir.join("1")).unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        names
    }

    /// M3: an upload fetches the key, then reads the file (up to 2 GB)
    /// without any lock. If a rotation began meanwhile, storing it under the
    /// old key would make it unreadable for good; it is refused, and no file
    /// is left behind.
    #[test]
    fn test_upload_with_a_key_from_before_a_rotation_is_refused() {
        let (db, admin, k1) = setup();
        let conn = db.get_connection();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();

        let (first, id) = store_uploaded_firmware(conn, dir, request("1.0"), admin, "admin", b"image", &k1).unwrap();
        assert_eq!(Path::new(&first.file_path), Path::new("1").join(format!("{}.enc", id)));

        let rotation = KeyRotation::with_params(conn, KdfParams::fast_for_tests());
        let k2 = rotation.begin(admin, "admin", &k1, 0, false).unwrap().new_key;
        let err = store_uploaded_firmware(conn, dir, request("2.0"), admin, "admin", b"late image", &k1).unwrap_err();
        assert_eq!(err, KEY_ROTATION_IN_PROGRESS_ERROR);
        assert_eq!(files(dir), vec![format!("{}.enc", id)]);
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM firmware_versions", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        // After the rotation the old key is still refused; the new one works
        conn.execute("DELETE FROM firmware_versions", []).unwrap();
        rotation.finalize(admin, "admin", false).unwrap();
        let err = store_uploaded_firmware(conn, dir, request("2.0"), admin, "admin", b"late image", &k1).unwrap_err();
        assert_eq!(err, STALE_KEY_ERROR);
        let (_, id) = store_uploaded_firmware(conn, dir, request("2.0"), admin, "admin", b"image", &k2).unwrap();
        let version: u32 = conn.query_row("SELECT key_version FROM firmware_versions WHERE id = ?1", [id], |r| r.get(0)).unwrap();
        assert_eq!(version, 2);
    }

    /// Uploads used to be limited to an extension allowlist (a Modicon M580
    /// `.sedp` image was rejected) and refused anything starting with `MZ`.
    /// Any file is accepted now, and its name is kept as a label only.
    #[test]
    fn test_upload_accepts_any_firmware_file_type() {
        let (db, admin, key) = setup();
        let conn = db.get_connection();
        let source = tempfile::TempDir::new().unwrap();
        let storage = tempfile::TempDir::new().unwrap();

        let cases: [(&str, &[u8]); 4] = [
            ("M580_v3.20.sedp", b"\x00\x01modicon image"),
            ("FIRMWARE", b"no extension at all"),
            ("updater.exe", b"MZ\x90\x00 pe package"),
            ("bundle.bin", b"MZ"),
        ];
        for (i, (name, content)) in cases.iter().enumerate() {
            let path = source.path().join(name);
            std::fs::write(&path, content).unwrap();
            let (data, original) = read_firmware_upload(path.to_str().unwrap()).unwrap();
            assert_eq!(&data, content);
            assert_eq!(original.as_deref(), Some(*name));

            let request = CreateFirmwareRequest { original_filename: original, ..request(&format!("{}.0", i)) };
            let (info, id) = store_uploaded_firmware(conn, storage.path(), request, admin, "admin", &data, &key).unwrap();
            // Stored under the record id, never under the uploaded name
            assert_eq!(Path::new(&info.file_path), Path::new("1").join(format!("{}.enc", id)));
            assert_eq!(info.original_filename.as_deref(), Some(*name));
            let stored = SqliteFirmwareRepository::new(conn).get_firmware_by_id(id).unwrap().unwrap();
            assert_eq!(stored.original_filename.as_deref(), Some(*name));
        }

        // The path checks still apply
        assert!(read_firmware_upload("").is_err());
        assert!(read_firmware_upload("../../etc/firmware.bin").is_err());
        assert!(read_firmware_upload(source.path().join("missing.sedp").to_str().unwrap()).is_err());
    }

    /// L2: callers racing to create the analysis queue all get the managed
    /// instance; none keeps an unmanaged one whose jobs nobody can count.
    #[test]
    fn test_racing_creators_all_get_the_managed_instance() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let handle = app.handle().clone();
        static CREATE: Mutex<()> = Mutex::new(());
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let created = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let threads: Vec<_> = (0..8).map(|_| {
            let (handle, barrier, created) = (handle.clone(), barrier.clone(), created.clone());
            std::thread::spawn(move || {
                barrier.wait();
                get_or_create_managed(&handle, &CREATE, || {
                    created.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    // Widen the race window
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    Arc::new(std::sync::atomic::AtomicUsize::new(0))
                })
            })
        }).collect();
        let results: Vec<Arc<std::sync::atomic::AtomicUsize>> = threads.into_iter().map(|t| t.join().unwrap()).collect();

        let managed = app.state::<Arc<std::sync::atomic::AtomicUsize>>().inner().clone();
        assert!(results.iter().all(|r| Arc::ptr_eq(r, &managed)));
        assert_eq!(created.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
