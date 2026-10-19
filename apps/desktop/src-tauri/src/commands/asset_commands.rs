// Asset management commands

use crate::auth::SessionManager;
use crate::assets::{AssetRepository, SqliteAssetRepository, CreateAssetRequest, AssetInfo, DashboardStats, AssetType, AssetHierarchy, MoveAssetRequest, SecurityClassification};
use crate::audit::{AuditRepository, SqliteAuditRepository, AuditEventRequest, AuditEventType};
use crate::database::Database;
use crate::users::UserRole;
use crate::validation::InputSanitizer;
use std::sync::Mutex;
use tauri::State;
use tracing::{error, info};

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;

#[tauri::command]
pub async fn create_asset(
    token: String,
    name: String,
    description: String,
    asset_type: String,
    parent_id: Option<i64>,
    security_classification: Option<String>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<AssetInfo, String> {
    // Validate session and get user info
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
    let description = InputSanitizer::sanitize_string(&description);
    
    // Parse asset type
    let asset_type = match AssetType::from_str(&asset_type) {
        Ok(t) => t,
        Err(_) => return Err("Invalid asset type. Must be 'folder' or 'device'".to_string()),
    };

    // Omitted means inherit from the parent folder (see SqliteAssetRepository::create_asset)
    let security_classification = match security_classification {
        Some(level) => Some(SecurityClassification::from_str(&level).map_err(|e| e.to_string())?),
        None => None,
    };

    if name.trim().is_empty() {
        return Err("Asset name cannot be empty".to_string());
    }
    if name.len() < 2 {
        return Err("Asset name must be at least 2 characters long".to_string());
    }
    if name.len() > 100 {
        return Err("Asset name cannot exceed 100 characters".to_string());
    }
    if description.len() > 500 {
        return Err("Description cannot exceed 500 characters".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            let request = CreateAssetRequest {
                name,
                description,
                asset_type,
                parent_id,
                created_by: session.user_id,
                security_classification,
            };

            match asset_repo.create_asset(request) {
                Ok(asset) => {
                    info!("Asset created by {}: {} (ID: {})", session.username, asset.name, asset.id);
                    Ok(asset.into())
                }
                Err(e) => {
                    error!("Failed to create asset: {}", e);
                    Err(format!("Failed to create asset: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_dashboard_assets(
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<AssetInfo>, String> {
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
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            match asset_repo.get_assets_with_info() {
                Ok(assets) => {
                    info!("Dashboard assets accessed by: {}", session.username);
                    Ok(assets)
                }
                Err(e) => {
                    error!("Failed to get dashboard assets: {}", e);
                    Err(format!("Failed to get dashboard assets: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_dashboard_stats(
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<DashboardStats, String> {
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
            let conn = db.get_connection();
            
            // Get total assets count
            let total_assets: i64 = conn
                .query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))
                .map_err(|e| {
                    error!("Failed to count assets: {}", e);
                    format!("Failed to count assets: {}", e)
                })?;
            
            // Get total versions count across all assets
            let total_versions: i64 = conn
                .query_row("SELECT COUNT(*) FROM configuration_versions", [], |row| row.get(0))
                .map_err(|e| {
                    error!("Failed to count versions: {}", e);
                    format!("Failed to count versions: {}", e)
                })?;
            
            let stats = DashboardStats {
                total_assets,
                total_versions,
                encryption_type: "AES-256".to_string(),
            };
            
            info!("Dashboard stats accessed by: {}", session.username);
            Ok(stats)
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_asset_details(
    token: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<AssetInfo, String> {
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
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            match asset_repo.get_asset_by_id(asset_id) {
                Ok(Some(asset)) => {
                    info!("Asset details accessed by {}: {} (ID: {})", session.username, asset.name, asset.id);
                    Ok(asset.into())
                }
                Ok(None) => Err("Asset not found".to_string()),
                Err(e) => {
                    error!("Failed to get asset details: {}", e);
                    Err(format!("Failed to get asset details: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Hierarchy management commands

#[tauri::command]
pub async fn create_folder_asset(
    token: String,
    name: String,
    description: String,
    parent_id: Option<i64>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<AssetInfo, String> {
    create_asset(token, name, description, "folder".to_string(), parent_id, None, db_state, session_manager).await
}

#[tauri::command]
pub async fn create_device_asset(
    token: String,
    name: String,
    description: String,
    parent_id: Option<i64>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<AssetInfo, String> {
    create_asset(token, name, description, "device".to_string(), parent_id, None, db_state, session_manager).await
}

#[tauri::command]
pub async fn get_asset_hierarchy(
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<AssetHierarchy>, String> {
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
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            match asset_repo.get_asset_hierarchy() {
                Ok(hierarchy) => {
                    info!("Asset hierarchy accessed by: {}", session.username);
                    Ok(hierarchy)
                }
                Err(e) => {
                    error!("Failed to get asset hierarchy: {}", e);
                    Err(format!("Failed to get asset hierarchy: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_children_assets(
    token: String,
    parent_id: Option<i64>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<AssetInfo>, String> {
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
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            match asset_repo.get_children_assets(parent_id) {
                Ok(assets) => {
                    let asset_infos: Vec<AssetInfo> = assets.into_iter().map(|asset| asset.into()).collect();
                    info!("Children assets accessed by {}: parent_id = {:?}", session.username, parent_id);
                    Ok(asset_infos)
                }
                Err(e) => {
                    error!("Failed to get children assets: {}", e);
                    Err(format!("Failed to get children assets: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn move_asset(
    token: String,
    asset_id: i64,
    new_parent_id: Option<i64>,
    new_sort_order: Option<i64>,
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

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            let request = MoveAssetRequest {
                asset_id,
                new_parent_id,
                new_sort_order,
            };

            match asset_repo.move_asset(request) {
                Ok(()) => {
                    info!("Asset moved by {}: asset_id = {}, new_parent_id = {:?}", session.username, asset_id, new_parent_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to move asset: {}", e);
                    Err(format!("Failed to move asset: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn validate_asset_move(
    token: String,
    asset_id: i64,
    new_parent_id: Option<i64>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<bool, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
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
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            match asset_repo.validate_asset_move(asset_id, new_parent_id) {
                Ok(is_valid) => Ok(is_valid),
                // Classification conflicts come back as errors; pass the message
                // through unchanged so the UI can show it as-is
                Err(e) => {
                    error!("Failed to validate asset move: {}", e);
                    Err(e.to_string())
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_asset_path(
    token: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<AssetInfo>, String> {
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
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            match asset_repo.get_asset_path(asset_id) {
                Ok(path) => {
                    let asset_infos: Vec<AssetInfo> = path.into_iter().map(|asset| asset.into()).collect();
                    info!("Asset path accessed by {}: asset_id = {}", session.username, asset_id);
                    Ok(asset_infos)
                }
                Err(e) => {
                    error!("Failed to get asset path: {}", e);
                    Err(format!("Failed to get asset path: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn update_asset_classification(
    token: String,
    asset_id: i64,
    security_classification: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<AssetInfo, String> {
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

    let classification = SecurityClassification::from_str(&security_classification)
        .map_err(|e| e.to_string())?;

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let asset_repo = SqliteAssetRepository::new(db.get_connection());

            // Anyone may raise a classification; only Administrators may lower one
            let allow_lowering = session.role == UserRole::Administrator;
            match asset_repo.update_asset_classification(asset_id, classification, allow_lowering) {
                Ok(change) => {
                    let raised: Vec<serde_json::Value> = change.raised.iter()
                        .map(|a| serde_json::json!({
                            "asset_id": a.id,
                            "name": a.name,
                            "previous": a.security_classification.as_str(),
                        }))
                        .collect();
                    let audit_event = AuditEventRequest {
                        event_type: AuditEventType::DatabaseOperation,
                        user_id: Some(session.user_id),
                        username: Some(session.username.clone()),
                        admin_user_id: None,
                        admin_username: None,
                        target_user_id: None,
                        target_username: None,
                        description: format!(
                            "Security classification of asset '{}' changed from {} to {}; {} contained item(s) raised",
                            change.asset.name, change.previous, classification, change.raised.len()
                        ),
                        metadata: Some(serde_json::json!({
                            "asset_id": asset_id,
                            "previous": change.previous.as_str(),
                            "new": classification.as_str(),
                            "raised": raised,
                        }).to_string()),
                        ip_address: None,
                        user_agent: None,
                    };
                    if let Err(e) = SqliteAuditRepository::new(db.get_connection()).log_event(&audit_event) {
                        error!("Failed to log audit event: {}", e);
                    }

                    info!(
                        "Asset classification changed by {}: asset_id = {}, {} -> {}, {} descendant(s) raised",
                        session.username, asset_id, change.previous, classification, change.raised.len()
                    );
                    Ok(change.asset.into())
                }
                Err(e) => {
                    error!("Failed to update asset classification: {}", e);
                    Err(e.to_string())
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Re-export tree navigation commands
pub use crate::assets::{
    batch_load_tree_nodes,
    search_tree_nodes,
    get_tree_statistics,
    preload_tree_nodes,
    get_node_metadata,
};