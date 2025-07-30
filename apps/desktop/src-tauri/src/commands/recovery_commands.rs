// Recovery management commands
// Extracted from lib.rs.backup

use crate::auth::SessionManager;
use crate::recovery::{RecoveryExporter, RecoveryImporter, RecoveryManifest, RecoveryImportRequest};
use crate::configurations::{ConfigurationRepository, SqliteConfigurationRepository};
use crate::firmware::{FirmwareRepository, SqliteFirmwareRepository};
use crate::vault::{VaultRepository, SqliteVaultRepository};
use crate::audit::{AuditRepository, SqliteAuditRepository};
use crate::database::Database;
use std::sync::Mutex;
use tauri::{AppHandle, State};
use tracing::{error, info};
use serde::{Serialize, Deserialize};

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    include_vault: bool,
    vault_available: bool,
    vault_secret_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundlePreview {
    manifest: RecoveryManifest,
    estimated_size: i64,
}

#[tauri::command]
pub async fn get_export_options(
    token: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<ExportOptions, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let _session = match session_manager_guard.validate_session(&token) {
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
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            // Check if vault exists for this asset
            let vault_info = vault_repo.get_vault_by_asset_id(asset_id)
                .map_err(|e| format!("Failed to check vault availability: {}", e))?;
            
            let (vault_available, vault_secret_count) = match vault_info {
                Some(info) => (true, info.secret_count),
                None => (false, 0),
            };
            
            Ok(ExportOptions {
                include_vault: vault_available, // Default to include if available
                vault_available,
                vault_secret_count,
            })
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn preview_recovery_bundle(
    token: String,
    asset_id: i64,
    config_version_id: i64,
    firmware_version_id: i64,
    include_vault: Option<bool>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<BundlePreview, String> {
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
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            // Get configuration
            let config = config_repo.get_configuration_by_id(config_version_id)
                .map_err(|e| format!("Failed to get configuration: {}", e))?
                .ok_or("Configuration version not found")?;
            
            // Get firmware
            let firmware = firmware_repo.get_firmware_by_id(firmware_version_id)
                .map_err(|e| format!("Failed to get firmware: {}", e))?
                .ok_or("Firmware version not found")?;
            
            let mut estimated_size = config.file_size + firmware.file_size;
            
            // Check vault if requested
            let vault_info = if include_vault.unwrap_or(false) {
                vault_repo.get_vault_by_asset_id(asset_id)
                    .map_err(|e| format!("Failed to get vault info: {}", e))?
            } else {
                None
            };
            
            // Add vault to manifest if available
            let vault_export_info = vault_info.as_ref().map(|info| {
                let vault_size_estimate = info.secret_count * 1024; // Rough estimate
                estimated_size += vault_size_estimate as i64;
                
                crate::recovery::VaultExportInfo {
                    vault_id: info.vault.id,
                    vault_name: info.vault.name.clone(),
                    filename: format!("vault_{}.json", info.vault.id),
                    checksum: "pending".to_string(), // Will be calculated during actual export
                    file_size: vault_size_estimate as i64,
                    secret_count: info.secret_count,
                    encrypted: true,
                }
            });
            
            let manifest = RecoveryManifest {
                asset_id,
                export_date: chrono::Utc::now().to_rfc3339(),
                exported_by: session.username.clone(),
                configuration: crate::recovery::ConfigurationExportInfo {
                    version_id: config.id,
                    version_number: config.version_number.clone(),
                    filename: config.file_name.clone(),
                    checksum: config.content_hash.clone(),
                    file_size: config.file_size,
                },
                firmware: crate::recovery::FirmwareExportInfo {
                    version_id: firmware.id,
                    version: firmware.version.clone(),
                    filename: firmware.file_path.clone(),
                    checksum: firmware.file_hash.clone(),
                    file_size: firmware.file_size,
                    model: firmware.model.clone().unwrap_or_default(),
                    vendor: firmware.vendor.clone().unwrap_or_default(),
                },
                vault: vault_export_info,
                compatibility_verified: config.firmware_version_id == Some(firmware_version_id),
            };
            
            estimated_size += 4096; // Add manifest file size
            
            Ok(BundlePreview {
                manifest,
                estimated_size,
            })
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn import_recovery_bundle(
    app: AppHandle,
    token: String,
    bundle_path: String,
    target_asset_id: i64,
    import_vault: bool,
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
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            let audit_repo = SqliteAuditRepository::new(db.get_connection());
            
            let importer = RecoveryImporter::new(&config_repo, &firmware_repo, &vault_repo, &audit_repo);
            
            let request = RecoveryImportRequest {
                bundle_path,
                target_asset_id,
                import_vault,
            };

            match importer.import_recovery_bundle(
                &app,
                request,
                session.user_id,
                &session.username,
                &session.role,
            ) {
                Ok(manifest) => Ok(manifest),
                Err(e) => Err(e.to_string()),
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn validate_bundle_integrity(
    token: String,
    bundle_path: String,
    session_manager: State<'_, SessionManagerState>,
) -> Result<RecoveryManifest, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let _session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    match RecoveryImporter::validate_bundle_integrity(&bundle_path) {
        Ok(manifest) => Ok(manifest),
        Err(e) => Err(e.to_string()),
    }
}