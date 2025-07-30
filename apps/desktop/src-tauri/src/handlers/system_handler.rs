// System business logic handlers
// This module contains core system operations like database initialization and health checks

use crate::database::Database;
use crate::users::{UserRepository, SqliteUserRepository};
use tauri::{AppHandle, Manager};
use tracing::info;

pub struct SystemHandler;

impl SystemHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn initialize_database_handler(&self, app: &AppHandle) -> Result<Database, String> {
        info!("Initializing database...");
        
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))?;
        
        let db_path = app_data_dir.join("ferrocodex.db");
        info!("Database path: {:?}", db_path);
        info!("Database exists before creation: {}", db_path.exists());
        
        let db = Database::new(db_path)
            .map_err(|e| format!("Failed to create database: {}", e))?;
        
        info!("Database initialized successfully");
        Ok(db)
    }

    pub fn database_health_check_handler(&self, db: &Database) -> Result<bool, String> {
        db.health_check()
            .map_err(|e| format!("Health check failed: {}", e))
    }

    pub fn is_first_launch_handler(&self, db: &Database) -> Result<bool, String> {
        info!("Checking if this is first launch");
        
        let user_repo = SqliteUserRepository::new(db.get_connection());
        let has_admins = user_repo.has_admin_users()
            .map_err(|e| format!("Failed to check for admin users: {}", e))?;
        Ok(!has_admins)
    }
}