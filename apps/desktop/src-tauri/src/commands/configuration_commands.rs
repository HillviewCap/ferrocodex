// Configuration management commands

use crate::auth::SessionManager;
use crate::assets::{AssetRepository, SqliteAssetRepository, AssetInfo, CreateAssetRequest, AssetType};
use crate::configurations::{ConfigurationRepository, SqliteConfigurationRepository, ConfigurationVersionInfo, ConfigurationStatus, StatusChangeRecord, FileMetadata, CreateConfigurationRequest};
use crate::branches::{BranchRepository, SqliteBranchRepository};
use crate::users::UserRole;
use crate::validation::{InputSanitizer, RateLimiter};
use crate::database::Database;
use std::sync::Mutex;
use std::fs;
use tauri::State;
use tracing::{error, info, warn};

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;
type RateLimiterState = Mutex<RateLimiter>;

#[tauri::command]
pub async fn import_configuration(
    token: String,
    asset_name: String,
    file_path: String,
    notes: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    rate_limiter: State<'_, RateLimiterState>,
) -> Result<AssetInfo, String> {
    // Check rate limiting
    let rate_limiter_guard = rate_limiter.lock()
        .map_err(|_| "Failed to acquire rate limiter lock".to_string())?;
    if let Err(e) = rate_limiter_guard.check_rate_limit(&format!("import_config_{}", token)) {
        return Err(e);
    }
    drop(rate_limiter_guard);

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

    // Validate inputs
    let asset_name = InputSanitizer::sanitize_string(&asset_name);
    let notes = InputSanitizer::sanitize_string(&notes);

    if asset_name.trim().is_empty() {
        return Err("Asset name cannot be empty".to_string());
    }
    if asset_name.len() < 2 {
        return Err("Asset name must be at least 2 characters long".to_string());
    }
    if asset_name.len() > 100 {
        return Err("Asset name cannot exceed 100 characters".to_string());
    }
    if notes.len() > 1000 {
        return Err("Notes cannot exceed 1000 characters".to_string());
    }

    // Check for malicious input
    if InputSanitizer::is_potentially_malicious(&asset_name) || InputSanitizer::is_potentially_malicious(&notes) {
        error!("Potentially malicious input detected in import_configuration");
        return Err("Invalid input detected. Please avoid using special characters or script-like patterns.".to_string());
    }

    // Validate file path
    if let Err(e) = InputSanitizer::validate_file_path(&file_path) {
        error!("Invalid file path: {}", e);
        return Err(format!("Invalid file path: {}", e));
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            let start_time = std::time::Instant::now();
            
            // Read file content
            let file_content = match fs::read(&file_path) {
                Ok(content) => content,
                Err(e) => {
                    error!("Failed to read file: {}", e);
                    return Err(format!("Failed to read file: {}", e));
                }
            };
            
            let file_name = std::path::Path::new(&file_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            // Create asset first
            let asset_request = CreateAssetRequest {
                name: asset_name.clone(),
                description: format!("Configuration asset - imported from {}", file_name),
                asset_type: AssetType::Device,
                parent_id: None,
                created_by: session.user_id,
            };

            let asset = match asset_repo.create_asset(asset_request) {
                Ok(asset) => asset,
                Err(e) => {
                    error!("Failed to create asset: {}", e);
                    return Err(format!("Failed to create asset: {}", e));
                }
            };

            // Store configuration
            let config_request = CreateConfigurationRequest {
                asset_id: asset.id,
                file_name,
                file_content,
                author: session.user_id,
                notes,
            };
            
            match config_repo.store_configuration(config_request) {
                Ok(_) => {
                    let duration = start_time.elapsed();
                    
                    // Log performance metrics
                    if duration.as_secs() >= 2 {
                        warn!("Import operation took {} seconds, exceeding 2-second requirement", duration.as_secs_f64());
                    } else {
                        info!("Import completed in {:.2} seconds", duration.as_secs_f64());
                    }
                    
                    info!("Configuration imported by {}: {} (Asset ID: {})", session.username, asset.name, asset.id);
                    Ok(asset.into())
                }
                Err(e) => {
                    error!("Failed to store configuration: {}", e);
                    // Clean up asset if configuration storage failed
                    let _ = asset_repo.delete_asset(asset.id);
                    Err(format!("Failed to store configuration: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn import_configuration_for_asset(
    token: String,
    asset_id: i64,
    file_path: String,
    version_notes: String,
    classification: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    rate_limiter: State<'_, RateLimiterState>,
) -> Result<(), String> {
    // Check rate limiting
    let rate_limiter_guard = rate_limiter.lock()
        .map_err(|_| "Failed to acquire rate limiter lock".to_string())?;
    if let Err(e) = rate_limiter_guard.check_rate_limit(&format!("import_config_{}", token)) {
        return Err(e);
    }
    drop(rate_limiter_guard);

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

    // Validate inputs
    let version_notes = InputSanitizer::sanitize_string(&version_notes);
    let classification = InputSanitizer::sanitize_string(&classification);

    if version_notes.len() > 1000 {
        return Err("Version notes cannot exceed 1000 characters".to_string());
    }

    // Check for malicious input
    if InputSanitizer::is_potentially_malicious(&version_notes) || InputSanitizer::is_potentially_malicious(&classification) {
        error!("Potentially malicious input detected in import_configuration_for_asset");
        return Err("Invalid input detected. Please avoid using special characters or script-like patterns.".to_string());
    }

    // Validate file path
    if let Err(e) = InputSanitizer::validate_file_path(&file_path) {
        error!("Invalid file path: {}", e);
        return Err(format!("Invalid file path: {}", e));
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            let start_time = std::time::Instant::now();
            
            // Verify the asset exists
            let _asset = match asset_repo.get_asset_by_id(asset_id) {
                Ok(Some(asset)) => asset,
                Ok(None) => return Err("Asset not found".to_string()),
                Err(e) => {
                    error!("Failed to retrieve asset: {}", e);
                    return Err("Failed to retrieve asset".to_string());
                }
            };
            
            // Read file content
            let file_content = match fs::read(&file_path) {
                Ok(content) => content,
                Err(e) => {
                    error!("Failed to read file: {}", e);
                    return Err(format!("Failed to read file: {}", e));
                }
            };
            
            let file_name = std::path::Path::new(&file_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Store configuration
            let config_request = CreateConfigurationRequest {
                asset_id,
                file_name,
                file_content,
                author: session.user_id,
                notes: version_notes,
            };
            
            match config_repo.store_configuration(config_request) {
                Ok(_) => {
                    let duration = start_time.elapsed();
                    
                    // Log performance metrics
                    if duration.as_secs() >= 2 {
                        warn!("Import operation took {} seconds, exceeding 2-second requirement", duration.as_secs_f64());
                    } else {
                        info!("Import completed in {:.2} seconds", duration.as_secs_f64());
                    }
                    
                    info!("Configuration imported by {} for asset ID: {}", session.username, asset_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to store configuration: {}", e);
                    Err(format!("Failed to store configuration: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_configuration_versions(
    token: String,
    asset_id: i64,
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
            
            match config_repo.get_configuration_versions(asset_id) {
                Ok(versions) => {
                    info!("Configuration versions accessed by {}: Asset ID {}", session.username, asset_id);
                    Ok(versions)
                }
                Err(e) => {
                    error!("Failed to get configuration versions: {}", e);
                    Err(format!("Failed to get configuration versions: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn update_configuration_status(
    token: String,
    version_id: i64,
    new_status: String,
    change_reason: Option<String>,
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

    // Parse and validate status
    let status = ConfigurationStatus::from_str(&new_status)
        .ok_or_else(|| format!("Invalid status: {}", new_status))?;

    // Prevent direct Golden status changes - must use promote_to_golden
    if status == ConfigurationStatus::Golden {
        return Err("Golden status can only be set through the promotion wizard. Please use 'Promote to Golden' option for Approved versions.".to_string());
    }

    // Validate inputs
    let change_reason = change_reason.map(|r| InputSanitizer::sanitize_string(&r));
    
    if let Some(ref reason) = change_reason {
        if InputSanitizer::is_potentially_malicious(reason) {
            error!("Potentially malicious input detected in update_configuration_status");
            return Err("Invalid input detected".to_string());
        }
        if reason.len() > 500 {
            return Err("Change reason cannot exceed 500 characters".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());

            // Check if user has permission for this status transition
            let available_transitions = config_repo.get_available_status_transitions(version_id, &session.role.to_string())
                .map_err(|e| format!("Failed to check available transitions: {}", e))?;

            if !available_transitions.contains(&status) {
                warn!("User {} attempted unauthorized status transition to {:?} for version {}", session.username, status, version_id);
                return Err("You don't have permission to change to this status".to_string());
            }

            match config_repo.update_configuration_status(version_id, status.clone(), session.user_id, change_reason) {
                Ok(_) => {
                    info!("Configuration status updated by {}: Version {} to {:?}", session.username, version_id, status);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to update configuration status: {}", e);
                    Err(format!("Failed to update configuration status: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_configuration_status_history(
    token: String,
    version_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<StatusChangeRecord>, String> {
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
            
            match config_repo.get_configuration_status_history(version_id) {
                Ok(history) => {
                    info!("Configuration status history accessed by {}: Version ID {}", session.username, version_id);
                    Ok(history)
                }
                Err(e) => {
                    error!("Failed to get configuration status history: {}", e);
                    Err(format!("Failed to get configuration status history: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_available_status_transitions(
    token: String,
    version_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<String>, String> {
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
            
            match config_repo.get_available_status_transitions(version_id, &session.role.to_string()) {
                Ok(transitions) => {
                    let status_strings: Vec<String> = transitions.iter().map(|s| s.as_str().to_string()).collect();
                    info!("Available status transitions accessed by {}: Version ID {}", session.username, version_id);
                    Ok(status_strings)
                }
                Err(e) => {
                    error!("Failed to get available status transitions: {}", e);
                    Err(format!("Failed to get available status transitions: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn promote_to_golden(
    token: String,
    version_id: i64,
    promotion_reason: Option<String>,
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

    // Only Engineers and Administrators can promote to Golden
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to promote to Golden: {}", session.username);
        return Err("Only Engineers and Administrators can promote versions to Golden".to_string());
    }

    // Validate inputs
    let promotion_reason = promotion_reason.map(|r| InputSanitizer::sanitize_string(&r));
    
    if let Some(ref reason) = promotion_reason {
        if InputSanitizer::is_potentially_malicious(reason) {
            error!("Potentially malicious input detected in promote_to_golden");
            return Err("Invalid input detected".to_string());
        }
        if reason.len() > 500 {
            return Err("Promotion reason cannot exceed 500 characters".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());

            // Check promotion eligibility
            let is_eligible = config_repo.get_promotion_eligibility(version_id)
                .map_err(|e| format!("Failed to check promotion eligibility: {}", e))?;

            if !is_eligible {
                return Err("Version is not eligible for Golden promotion. Only Approved versions can be promoted.".to_string());
            }

            match config_repo.promote_to_golden(version_id, session.user_id, promotion_reason) {
                Ok(_) => {
                    info!("Version promoted to Golden by {}: Version ID {}", session.username, version_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to promote to Golden: {}", e);
                    Err(format!("Failed to promote to Golden: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn promote_branch_to_silver(
    token: String,
    branch_id: i64,
    promotion_notes: Option<String>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<i64, String> {
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

    // Both Engineers and Administrators can promote branches to Silver
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to promote branch to Silver: {}", session.username);
        return Err("Insufficient permissions to promote branch to Silver".to_string());
    }

    // Validate inputs
    let promotion_notes = promotion_notes.map(|n| InputSanitizer::sanitize_string(&n));
    
    if let Some(ref notes) = promotion_notes {
        if InputSanitizer::is_potentially_malicious(notes) {
            error!("Potentially malicious input detected in promote_branch_to_silver");
            return Err("Invalid input detected".to_string());
        }
        if notes.len() > 1000 {
            return Err("Promotion notes cannot exceed 1000 characters".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            // Get the branch details
            let branch = match branch_repo.get_branch_by_id(branch_id) {
                Ok(Some(b)) => b,
                Ok(None) => return Err("Branch not found".to_string()),
                Err(e) => {
                    error!("Failed to get branch details: {}", e);
                    return Err(format!("Failed to get branch details: {}", e));
                }
            };
            
            // Get the latest version of the branch
            let latest_version = match branch_repo.get_branch_latest_version(branch_id) {
                Ok(Some(version)) => version,
                Ok(None) => return Err("Branch has no versions to promote".to_string()),
                Err(e) => {
                    error!("Failed to get branch latest version: {}", e);
                    return Err(format!("Failed to get branch latest version: {}", e));
                }
            };
            
            // Get the configuration content
            let content = match config_repo.get_configuration_content(latest_version.version_id) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to get configuration content: {}", e);
                    return Err(format!("Failed to get configuration content: {}", e));
                }
            };
            
            // Create a new configuration version in the main line with Silver status
            let notes = format!(
                "Promoted from branch '{}' (version {}). {}",
                branch.name,
                latest_version.branch_version_number,
                promotion_notes.unwrap_or_default()
            );
            
            let config_request = CreateConfigurationRequest {
                asset_id: branch.asset_id,
                file_name: latest_version.file_name.clone(),
                file_content: content,
                author: session.user_id,
                notes,
            };
            
            // Store the new configuration
            let new_config = match config_repo.store_configuration(config_request) {
                Ok(config) => config,
                Err(e) => {
                    error!("Failed to create Silver configuration: {}", e);
                    return Err(format!("Failed to create Silver configuration: {}", e));
                }
            };
            
            // Update the status to Silver
            match config_repo.update_configuration_status(
                new_config.id,
                ConfigurationStatus::Silver,
                session.user_id,
                Some("Promoted from branch".to_string())
            ) {
                Ok(_) => {
                    info!("Branch promoted to Silver by {}: Branch {} -> Config ID {}", 
                          session.username, branch.name, new_config.id);
                    Ok(new_config.id)
                }
                Err(e) => {
                    error!("Failed to set Silver status: {}", e);
                    Err(format!("Failed to set Silver status: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_golden_version(
    token: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Option<ConfigurationVersionInfo>, String> {
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
            
            match config_repo.get_golden_version(asset_id) {
                Ok(golden_version) => {
                    info!("Golden version accessed by {}: Asset ID {}", session.username, asset_id);
                    Ok(golden_version)
                }
                Err(e) => {
                    error!("Failed to get Golden version: {}", e);
                    Err(format!("Failed to get Golden version: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_promotion_eligibility(
    token: String,
    version_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<bool, String> {
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
            
            match config_repo.get_promotion_eligibility(version_id) {
                Ok(eligibility) => {
                    info!("Promotion eligibility checked by {}: Version ID {}", session.username, version_id);
                    Ok(eligibility)
                }
                Err(e) => {
                    error!("Failed to check promotion eligibility: {}", e);
                    Err(format!("Failed to check promotion eligibility: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn export_configuration_version(
    token: String,
    version_id: i64,
    export_path: String,
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

    // Validate export path
    let export_path = export_path.trim();
    
    if export_path.is_empty() {
        return Err("Export path cannot be empty".to_string());
    }
    
    // Use proper file path validation instead of generic malicious input check
    if let Err(e) = InputSanitizer::validate_file_path(&export_path) {
        error!("Invalid export path: {}", e);
        return Err(format!("Invalid export path: {}", e));
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            let start_time = std::time::Instant::now();
            
            match config_repo.export_configuration_version(version_id, &export_path) {
                Ok(_) => {
                    let duration = start_time.elapsed();
                    
                    // Log performance metrics
                    if duration.as_secs() >= 2 {
                        warn!("Export operation took {} seconds, exceeding 2-second requirement", duration.as_secs_f64());
                    } else {
                        info!("Export completed in {:.2} seconds", duration.as_secs_f64());
                    }
                    
                    info!("Configuration exported by {}: Version {} to {}", session.username, version_id, export_path);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to export configuration: {}", e);
                    Err(format!("Failed to export configuration: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_file_metadata(
    file_path: String,
) -> Result<FileMetadata, String> {
    // Validate file path
    if let Err(e) = InputSanitizer::validate_file_path(&file_path) {
        error!("Invalid file path: {}", e);
        return Err(format!("Invalid file path: {}", e));
    }

    match crate::configurations::file_utils::get_file_metadata(&file_path) {
        Ok(metadata) => {
            info!("File metadata retrieved: {}", file_path);
            Ok(metadata)
        }
        Err(e) => {
            error!("Failed to get file metadata: {}", e);
            Err(format!("Failed to get file metadata: {}", e))
        }
    }
}

#[tauri::command]
pub async fn archive_version(
    token: String,
    version_id: i64,
    archive_reason: Option<String>,
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

    // Only Engineers and Administrators can archive versions
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to archive version: {}", session.username);
        return Err("Only Engineers and Administrators can archive versions".to_string());
    }

    // Validate inputs
    let archive_reason = archive_reason.map(|r| InputSanitizer::sanitize_string(&r));
    
    if let Some(ref reason) = archive_reason {
        if InputSanitizer::is_potentially_malicious(reason) {
            error!("Potentially malicious input detected in archive_version");
            return Err("Invalid input detected".to_string());
        }
        if reason.len() > 500 {
            return Err("Archive reason cannot exceed 500 characters".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());

            match config_repo.archive_version(version_id, session.user_id, archive_reason) {
                Ok(_) => {
                    info!("Version archived by {}: Version ID {}", session.username, version_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to archive version: {}", e);
                    Err(format!("Failed to archive version: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn restore_version(
    token: String,
    version_id: i64,
    restore_reason: Option<String>,
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

    // Only Engineers and Administrators can restore versions
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to restore version: {}", session.username);
        return Err("Only Engineers and Administrators can restore versions".to_string());
    }

    // Validate inputs
    let restore_reason = restore_reason.map(|r| InputSanitizer::sanitize_string(&r));
    
    if let Some(ref reason) = restore_reason {
        if InputSanitizer::is_potentially_malicious(reason) {
            error!("Potentially malicious input detected in restore_version");
            return Err("Invalid input detected".to_string());
        }
        if reason.len() > 500 {
            return Err("Restore reason cannot exceed 500 characters".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());

            match config_repo.restore_version(version_id, session.user_id, restore_reason) {
                Ok(_) => {
                    info!("Version restored by {}: Version ID {}", session.username, version_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to restore version: {}", e);
                    Err(format!("Failed to restore version: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}