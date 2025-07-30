use crate::database::Database;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use tracing::info;

type DatabaseState = Mutex<Option<Database>>;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub async fn initialize_database(app: AppHandle, db_state: State<'_, DatabaseState>) -> Result<bool, String> {
    info!("Initializing database...");
    
    let mut db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    if db_guard.is_some() {
        return Ok(true); // Already initialized
    }

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    let db_path = app_data_dir.join("ferrocodex.db");
    info!("Database path: {:?}", db_path);
    info!("Database exists before creation: {}", db_path.exists());
    
    let db = Database::new(db_path)
        .map_err(|e| format!("Failed to create database: {}", e))?;
    
    *db_guard = Some(db);
    info!("Database initialized successfully");
    Ok(true)
}

#[tauri::command]
pub async fn database_health_check(db_state: State<'_, DatabaseState>) -> Result<bool, String> {
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    match db_guard.as_ref() {
        Some(db) => {
            db.health_check()
                .map_err(|e| format!("Health check failed: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn is_first_launch(db_state: State<'_, DatabaseState>) -> Result<bool, String> {
    info!("Checking if this is first launch");
    
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    match db_guard.as_ref() {
        Some(db) => {
            use crate::users::{UserRepository, SqliteUserRepository};
            let user_repo = SqliteUserRepository::new(db.get_connection());
            let has_admins = user_repo.has_admin_users()
                .map_err(|e| format!("Failed to check for admin users: {}", e))?;
            Ok(!has_admins)
        }
        None => Ok(true), // If DB isn't initialized, it's definitely first launch
    }
}