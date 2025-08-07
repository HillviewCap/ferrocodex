// Enhanced tree navigation commands for virtualized tree performance

use crate::auth::SessionManager;
use crate::assets::{AssetRepository, SqliteAssetRepository, AssetInfo, AssetHierarchy, AssetType};
use crate::database::Database;
use crate::validation::InputSanitizer;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;
use tracing::{error, info, debug};

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;

#[derive(Debug, Serialize, Deserialize)]
pub struct TreeBatch {
    pub node_ids: Vec<i64>,
    pub include_children: bool,
    pub max_depth: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchLoadResult {
    pub nodes: HashMap<i64, AssetHierarchy>,
    pub metadata: HashMap<i64, AssetMetadata>,
    pub load_time_ms: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub id: i64,
    pub has_children: bool,
    pub child_count: i32,
    pub depth: i32,
    pub last_modified: String,
    pub size_estimate: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TreeSearchRequest {
    pub query: String,
    pub search_mode: String, // "name", "description", "all"
    pub case_sensitive: bool,
    pub max_results: Option<i32>,
    pub parent_filter: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TreeSearchResult {
    pub matches: Vec<AssetHierarchy>,
    pub total_count: i32,
    pub search_time_ms: f64,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TreeStatistics {
    pub total_nodes: i32,
    pub max_depth: i32,
    pub folder_count: i32,
    pub device_count: i32,
    pub average_children_per_folder: f32,
    pub last_modified: String,
}

// Batch loading for performance optimization
#[tauri::command]
pub async fn batch_load_tree_nodes(
    token: String,
    batch: TreeBatch,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<BatchLoadResult, String> {
    let start_time = std::time::Instant::now();
    
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

    // Enhanced input validation
    if batch.node_ids.is_empty() {
        return Err("Batch cannot be empty".to_string());
    }
    
    if batch.node_ids.len() > 100 {
        return Err("Batch size too large. Maximum 100 nodes per batch.".to_string());
    }
    
    // Validate node IDs are positive
    for &node_id in &batch.node_ids {
        if node_id <= 0 {
            return Err("Invalid node ID: must be positive".to_string());
        }
    }
    
    // Validate max_depth if provided
    if let Some(depth) = batch.max_depth {
        if depth < 0 || depth > 10 {
            return Err("Invalid max_depth: must be between 0 and 10".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            let mut nodes = HashMap::new();
            let mut metadata = HashMap::new();

            for node_id in &batch.node_ids {
                // Load node data
                if let Ok(Some(node)) = load_node_hierarchy(&conn, *node_id, batch.max_depth.unwrap_or(1)) {
                    // Get metadata
                    if let Ok(meta) = get_node_metadata_internal(&conn, *node_id) {
                        metadata.insert(*node_id, meta);
                    }
                    nodes.insert(*node_id, node);
                }
            }

            let load_time = start_time.elapsed().as_millis() as f64;
            debug!("Batch loaded {} nodes in {}ms for user: {}", nodes.len(), load_time, session.username);

            Ok(BatchLoadResult {
                nodes,
                metadata,
                load_time_ms: load_time,
            })
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Optimized tree search with caching
#[tauri::command]
pub async fn search_tree_nodes(
    token: String,
    search_request: TreeSearchRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<TreeSearchResult, String> {
    let start_time = std::time::Instant::now();
    
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

    // Sanitize search query
    let query = InputSanitizer::sanitize_string(&search_request.query);
    if query.trim().is_empty() {
        return Err("Search query cannot be empty".to_string());
    }

    if query.len() > 100 {
        return Err("Search query too long. Maximum 100 characters.".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            let limit = search_request.max_results.unwrap_or(50).min(100);
            
            let matches = search_assets(
                &conn,
                &query,
                &search_request.search_mode,
                search_request.case_sensitive,
                search_request.parent_filter,
                limit + 1, // Load one extra to check if there are more results
            )?;

            let has_more = matches.len() > limit as usize;
            let final_matches = if has_more {
                matches.into_iter().take(limit as usize).collect()
            } else {
                matches
            };

            let search_time = start_time.elapsed().as_millis() as f64;
            info!("Tree search completed in {}ms for user: {} (query: '{}')", search_time, session.username, query);

            Ok(TreeSearchResult {
                total_count: final_matches.len() as i32,
                matches: final_matches,
                search_time_ms: search_time,
                has_more,
            })
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Get tree statistics for performance optimization
#[tauri::command]
pub async fn get_tree_statistics(
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<TreeStatistics, String> {
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
            
            // Get total node count
            let total_nodes: i32 = conn
                .query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))
                .map_err(|e| format!("Failed to count assets: {}", e))?;

            // Get folder and device counts
            let folder_count: i32 = conn
                .query_row("SELECT COUNT(*) FROM assets WHERE asset_type = 'Folder'", [], |row| row.get(0))
                .map_err(|e| format!("Failed to count folders: {}", e))?;

            let device_count: i32 = conn
                .query_row("SELECT COUNT(*) FROM assets WHERE asset_type = 'Device'", [], |row| row.get(0))
                .map_err(|e| format!("Failed to count devices: {}", e))?;

            // Calculate max depth (this might be expensive for large trees)
            let max_depth = calculate_max_depth(&conn)?;

            // Calculate average children per folder
            let avg_children: f32 = if folder_count > 0 {
                let total_children: i32 = conn
                    .query_row("SELECT COUNT(*) FROM assets WHERE parent_id IS NOT NULL", [], |row| row.get(0))
                    .unwrap_or(0);
                total_children as f32 / folder_count as f32
            } else {
                0.0
            };

            // Get last modification time
            let last_modified: String = conn
                .query_row(
                    "SELECT MAX(updated_at) FROM assets",
                    [],
                    |row| row.get::<_, Option<String>>(0)
                )
                .map_err(|e| format!("Failed to get last modified: {}", e))?
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

            debug!("Tree statistics calculated for user: {}", session.username);

            Ok(TreeStatistics {
                total_nodes,
                max_depth,
                folder_count,
                device_count,
                average_children_per_folder: avg_children,
                last_modified,
            })
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Preload nodes based on predicted navigation patterns
#[tauri::command]
pub async fn preload_tree_nodes(
    token: String,
    node_ids: Vec<i64>,
    depth: Option<i32>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<HashMap<i64, AssetHierarchy>, String> {
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

    if node_ids.len() > 50 {
        return Err("Too many nodes to preload. Maximum 50 nodes.".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            let mut preloaded = HashMap::new();
            let max_depth = depth.unwrap_or(2).min(5); // Limit depth to prevent excessive loading

            for node_id in node_ids {
                if let Ok(Some(node)) = load_node_hierarchy(&conn, node_id, max_depth) {
                    preloaded.insert(node_id, node);
                }
            }

            debug!("Preloaded {} nodes for user: {}", preloaded.len(), session.username);
            Ok(preloaded)
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Get node metadata for performance optimization
#[tauri::command]
pub async fn get_node_metadata(
    token: String,
    node_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<AssetMetadata, String> {
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
            let conn = db.get_connection();
            get_node_metadata_internal(&conn, node_id)
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Helper functions

fn load_node_hierarchy(conn: &Connection, node_id: i64, max_depth: i32) -> Result<Option<AssetHierarchy>, String> {
    // Load the node itself
    let mut stmt = conn
        .prepare("SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at FROM assets WHERE id = ?")
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let asset_result = stmt.query_row(params![node_id], |row| {
        Ok(AssetHierarchy {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            asset_type: AssetType::from_str(&row.get::<_, String>(3)?).map_err(|_| rusqlite::Error::InvalidColumnType(3, "asset_type".to_string(), rusqlite::types::Type::Text))?,
            parent_id: row.get(4)?,
            sort_order: row.get(5)?,
            children: Vec::new(), // Will be populated below
            created_by: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    });

    match asset_result {
        Ok(mut asset) => {
            // Load children if this is a folder and we haven't reached max depth
            if asset.asset_type.to_string() == "Folder" && max_depth > 0 {
                asset.children = load_children_hierarchy(conn, node_id, max_depth - 1)?;
            }
            Ok(Some(asset))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Database error: {}", e)),
    }
}

fn load_children_hierarchy(conn: &Connection, parent_id: i64, max_depth: i32) -> Result<Vec<AssetHierarchy>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at FROM assets WHERE parent_id = ? ORDER BY sort_order, name")
        .map_err(|e| format!("Failed to prepare children statement: {}", e))?;

    let children_iter = stmt.query_map(params![parent_id], |row| {
        Ok(AssetHierarchy {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            asset_type: AssetType::from_str(&row.get::<_, String>(3)?).map_err(|_| rusqlite::Error::InvalidColumnType(3, "asset_type".to_string(), rusqlite::types::Type::Text))?,
            parent_id: row.get(4)?,
            sort_order: row.get(5)?,
            children: Vec::new(), // Will be populated recursively
            created_by: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    }).map_err(|e| format!("Failed to query children: {}", e))?;

    let mut children = Vec::new();
    for child_result in children_iter {
        let mut child = child_result.map_err(|e| format!("Failed to process child: {}", e))?;
        
        // Recursively load grandchildren if this is a folder and we haven't reached max depth
        if child.asset_type.to_string() == "Folder" && max_depth > 0 {
            child.children = load_children_hierarchy(conn, child.id, max_depth - 1)?;
        }
        
        children.push(child);
    }

    Ok(children)
}

fn get_node_metadata_internal(conn: &Connection, node_id: i64) -> Result<AssetMetadata, String> {
    // Get basic asset info
    let mut stmt = conn
        .prepare("SELECT asset_type, updated_at FROM assets WHERE id = ?")
        .map_err(|e| format!("Failed to prepare metadata statement: {}", e))?;

    let (asset_type, last_modified): (String, String) = stmt
        .query_row(params![node_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| format!("Failed to get asset metadata: {}", e))?;

    // Count children
    let child_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM assets WHERE parent_id = ?", params![node_id], |row| row.get(0))
        .unwrap_or(0);

    let has_children = child_count > 0;

    // Calculate depth (this could be expensive for deep trees, consider caching)
    let depth = calculate_node_depth(conn, node_id)?;

    // Estimate size (count of all descendants)
    let size_estimate = if asset_type == "Folder" {
        Some(count_descendants(conn, node_id)?)
    } else {
        None
    };

    Ok(AssetMetadata {
        id: node_id,
        has_children,
        child_count,
        depth,
        last_modified,
        size_estimate,
    })
}

fn calculate_node_depth(conn: &Connection, node_id: i64) -> Result<i32, String> {
    let mut current_id = node_id;
    let mut depth = 0;
    
    loop {
        let parent_id: Option<i64> = conn
            .query_row("SELECT parent_id FROM assets WHERE id = ?", params![current_id], |row| row.get(0))
            .map_err(|e| format!("Failed to get parent: {}", e))?;
        
        match parent_id {
            Some(pid) => {
                current_id = pid;
                depth += 1;
                if depth > 100 { // Prevent infinite loops
                    return Err("Tree depth exceeds maximum allowed depth".to_string());
                }
            }
            None => break,
        }
    }
    
    Ok(depth)
}

fn calculate_max_depth(conn: &Connection) -> Result<i32, String> {
    // This is an expensive operation for large trees
    // Consider implementing WITH RECURSIVE for better performance
    let mut max_depth = 0;
    let mut stmt = conn
        .prepare("SELECT id FROM assets WHERE parent_id IS NULL")
        .map_err(|e| format!("Failed to prepare root query: {}", e))?;

    let root_iter = stmt.query_map([], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Failed to query roots: {}", e))?;

    for root_result in root_iter {
        let root_id = root_result.map_err(|e| format!("Failed to process root: {}", e))?;
        let depth = calculate_subtree_depth(conn, root_id, 0)?;
        max_depth = max_depth.max(depth);
    }

    Ok(max_depth)
}

fn calculate_subtree_depth(conn: &Connection, node_id: i64, current_depth: i32) -> Result<i32, String> {
    if current_depth > 100 { // Prevent stack overflow
        return Ok(current_depth);
    }

    let mut stmt = conn
        .prepare("SELECT id FROM assets WHERE parent_id = ?")
        .map_err(|e| format!("Failed to prepare children query: {}", e))?;

    let children_iter = stmt.query_map(params![node_id], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Failed to query children: {}", e))?;

    let mut max_child_depth = current_depth;
    for child_result in children_iter {
        let child_id = child_result.map_err(|e| format!("Failed to process child: {}", e))?;
        let child_depth = calculate_subtree_depth(conn, child_id, current_depth + 1)?;
        max_child_depth = max_child_depth.max(child_depth);
    }

    Ok(max_child_depth)
}

fn count_descendants(conn: &Connection, node_id: i64) -> Result<i64, String> {
    // Count all descendants recursively
    let direct_children: i64 = conn
        .query_row("SELECT COUNT(*) FROM assets WHERE parent_id = ?", params![node_id], |row| row.get(0))
        .unwrap_or(0);

    let mut total = direct_children;
    
    let mut stmt = conn
        .prepare("SELECT id FROM assets WHERE parent_id = ?")
        .map_err(|e| format!("Failed to prepare descendants query: {}", e))?;

    let children_iter = stmt.query_map(params![node_id], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Failed to query children for counting: {}", e))?;

    for child_result in children_iter {
        let child_id = child_result.map_err(|e| format!("Failed to process child for counting: {}", e))?;
        total += count_descendants(conn, child_id)?;
    }

    Ok(total)
}

fn search_assets(
    conn: &Connection,
    query: &str,
    search_mode: &str,
    case_sensitive: bool,
    parent_filter: Option<i64>,
    limit: i32,
) -> Result<Vec<AssetHierarchy>, String> {
    let mut sql = String::from("SELECT id, name, description, asset_type, parent_id, sort_order, created_by, created_at, updated_at FROM assets WHERE ");
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    // Build search condition
    match search_mode {
        "name" => {
            if case_sensitive {
                sql.push_str("name LIKE ?");
            } else {
                sql.push_str("LOWER(name) LIKE LOWER(?)");
            }
            params.push(Box::new(format!("%{}%", query)));
        }
        "description" => {
            if case_sensitive {
                sql.push_str("description LIKE ?");
            } else {
                sql.push_str("LOWER(description) LIKE LOWER(?)");
            }
            params.push(Box::new(format!("%{}%", query)));
        }
        _ => { // "all" or default
            if case_sensitive {
                sql.push_str("(name LIKE ? OR description LIKE ?)");
                params.push(Box::new(format!("%{}%", query)));
                params.push(Box::new(format!("%{}%", query)));
            } else {
                sql.push_str("(LOWER(name) LIKE LOWER(?) OR LOWER(description) LIKE LOWER(?))");
                params.push(Box::new(format!("%{}%", query)));
                params.push(Box::new(format!("%{}%", query)));
            }
        }
    }

    // Add parent filter if specified
    if let Some(parent_id) = parent_filter {
        sql.push_str(" AND parent_id = ?");
        params.push(Box::new(parent_id));
    }

    sql.push_str(" ORDER BY name LIMIT ?");
    params.push(Box::new(limit));

    let mut stmt = conn.prepare(&sql)
        .map_err(|e| format!("Failed to prepare search statement: {}", e))?;

    // Convert to references for query_map
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let matches_iter = stmt.query_map(&param_refs[..], |row| {
        Ok(AssetHierarchy {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            asset_type: AssetType::from_str(&row.get::<_, String>(3)?).map_err(|_| rusqlite::Error::InvalidColumnType(3, "asset_type".to_string(), rusqlite::types::Type::Text))?,
            parent_id: row.get(4)?,
            sort_order: row.get(5)?,
            children: Vec::new(), // Don't load children for search results
            created_by: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    }).map_err(|e| format!("Failed to execute search: {}", e))?;

    let mut matches = Vec::new();
    for match_result in matches_iter {
        matches.push(match_result.map_err(|e| format!("Failed to process search result: {}", e))?);
    }

    Ok(matches)
}