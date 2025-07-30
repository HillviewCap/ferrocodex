// Branch management commands

use crate::auth::SessionManager;
use crate::branches::{BranchRepository, SqliteBranchRepository, CreateBranchRequest, BranchInfo, CreateBranchVersionRequest, BranchVersionInfo};
use crate::database::Database;
use crate::validation::InputSanitizer;
use std::sync::Mutex;
use tauri::State;
use tracing::{error, info};

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;

#[tauri::command]
pub async fn create_branch(
    token: String,
    name: String,
    description: Option<String>,
    asset_id: i64,
    parent_version_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<BranchInfo, String> {
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
    let name = InputSanitizer::sanitize_string(&name);
    let description = description.map(|d| InputSanitizer::sanitize_string(&d));

    if InputSanitizer::is_potentially_malicious(&name) {
        error!("Potentially malicious input detected in create_branch");
        return Err("Invalid input detected".to_string());
    }

    if let Some(ref desc) = description {
        if InputSanitizer::is_potentially_malicious(desc) {
            error!("Potentially malicious input detected in create_branch description");
            return Err("Invalid input detected".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            let request = CreateBranchRequest {
                name,
                description,
                asset_id,
                parent_version_id,
                created_by: session.user_id,
            };

            match branch_repo.create_branch(request) {
                Ok(branch) => {
                    info!("Branch created by {}: {} for asset {}", session.username, branch.name, asset_id);
                    // For a newly created branch, we need to fetch the full BranchInfo with proper metadata
                    match branch_repo.get_branch_by_id(branch.id) {
                        Ok(Some(branch_info)) => Ok(branch_info),
                        Ok(None) => {
                            error!("Created branch not found: {}", branch.id);
                            Err("Failed to retrieve created branch".to_string())
                        }
                        Err(e) => {
                            error!("Failed to retrieve created branch: {}", e);
                            Err(format!("Failed to retrieve created branch: {}", e))
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to create branch: {}", e);
                    Err(format!("Failed to create branch: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_branches(
    token: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<BranchInfo>, String> {
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
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            match branch_repo.get_branches(asset_id) {
                Ok(branches) => {
                    info!("Branches accessed by {}: Asset ID {}", session.username, asset_id);
                    Ok(branches)
                }
                Err(e) => {
                    error!("Failed to get branches: {}", e);
                    Err(format!("Failed to get branches: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_branch_details(
    token: String,
    branch_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<BranchInfo, String> {
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
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            match branch_repo.get_branch_by_id(branch_id) {
                Ok(Some(branch)) => {
                    info!("Branch details accessed by {}: Branch ID {}", session.username, branch_id);
                    Ok(branch)
                }
                Ok(None) => Err("Branch not found".to_string()),
                Err(e) => {
                    error!("Failed to get branch details: {}", e);
                    Err(format!("Failed to get branch details: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn import_version_to_branch(
    token: String,
    branch_id: i64,
    file_path: String,
    notes: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<BranchVersionInfo, String> {
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
    let notes = InputSanitizer::sanitize_string(&notes);
    
    if InputSanitizer::is_potentially_malicious(&notes) {
        error!("Potentially malicious input detected in import_version_to_branch");
        return Err("Invalid input detected. Please avoid using special characters or script-like patterns in your notes.".to_string());
    }

    if notes.len() > 1000 {
        return Err("Notes cannot exceed 1000 characters".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            let request = CreateBranchVersionRequest {
                branch_id,
                file_path,
                notes,
                author: session.user_id,
            };

            match branch_repo.import_version_to_branch(request) {
                Ok(_branch_version) => {
                    // Get the full branch version info
                    match branch_repo.get_branch_latest_version(branch_id) {
                        Ok(Some(version_info)) => {
                            info!("Version imported to branch by {}: Branch ID {}", session.username, branch_id);
                            Ok(version_info)
                        }
                        Ok(None) => Err("Failed to retrieve imported version info".to_string()),
                        Err(e) => {
                            error!("Failed to get branch version info: {}", e);
                            Err(format!("Failed to get branch version info: {}", e))
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to import version to branch: {}", e);
                    Err(format!("Failed to import version to branch: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_branch_versions(
    token: String,
    branch_id: i64,
    page: Option<i32>,
    limit: Option<i32>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<BranchVersionInfo>, String> {
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
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            match branch_repo.get_branch_versions(branch_id, page, limit) {
                Ok(versions) => {
                    info!("Branch versions accessed by {}: Branch ID {}", session.username, branch_id);
                    Ok(versions)
                }
                Err(e) => {
                    error!("Failed to get branch versions: {}", e);
                    Err(format!("Failed to get branch versions: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_branch_latest_version(
    token: String,
    branch_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Option<BranchVersionInfo>, String> {
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
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            match branch_repo.get_branch_latest_version(branch_id) {
                Ok(version) => {
                    info!("Branch latest version accessed by {}: Branch ID {}", session.username, branch_id);
                    Ok(version)
                }
                Err(e) => {
                    error!("Failed to get branch latest version: {}", e);
                    Err(format!("Failed to get branch latest version: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn compare_branch_versions(
    token: String,
    branch_id: i64,
    version1_id: i64,
    version2_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<String, String> {
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
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            match branch_repo.compare_branch_versions(branch_id, version1_id, version2_id) {
                Ok(diff_content) => {
                    // Convert bytes to string for frontend
                    match String::from_utf8(diff_content) {
                        Ok(diff_str) => {
                            info!("Branch versions compared by {}: Branch ID {}", session.username, branch_id);
                            Ok(diff_str)
                        }
                        Err(_) => Err("Failed to convert diff content to string".to_string()),
                    }
                }
                Err(e) => {
                    error!("Failed to compare branch versions: {}", e);
                    Err(format!("Failed to compare branch versions: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}