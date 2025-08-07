// Settings management commands
// Extracted from lib.rs.backup

use crate::auth::SessionManager;
use crate::user_settings::{UserSettings, RetryPreferences, UserSettingsRepository, SqliteUserSettingsRepository, settings_utils};
use crate::database::Database;
use std::sync::Mutex;
use tauri::State;
use tracing::{error, info};

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;

#[tauri::command]
pub async fn get_user_settings(
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<UserSettings, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let settings_repo = SqliteUserSettingsRepository::new(db.get_connection());
            
            settings_repo.get_or_create_settings(session.user_id)
                .map_err(|e| format!("Failed to get user settings: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn update_user_settings(
    token: String,
    retry_preferences: RetryPreferences,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Validate retry preferences
    retry_preferences.validate()
        .map_err(|e| format!("Invalid retry preferences: {}", e))?;

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let settings_repo = SqliteUserSettingsRepository::new(db.get_connection());
            
            // Get current settings or create new ones
            let mut settings = settings_repo.get_or_create_settings(session.user_id)
                .map_err(|e| format!("Failed to get current settings: {}", e))?;
            
            // Update retry preferences
            settings.retry_preferences = retry_preferences;
            settings.updated_at = chrono::Utc::now();
            
            // Save updated settings
            settings_repo.update_settings(&settings)
                .map_err(|e| format!("Failed to update user settings: {}", e))?;
            
            info!("User settings updated by {}", session.username);
            Ok(())
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_retry_presets() -> Result<std::collections::HashMap<String, RetryPreferences>, String> {
    let mut presets = std::collections::HashMap::new();
    
    presets.insert("conservative".to_string(), RetryPreferences::conservative_preset());
    presets.insert("aggressive".to_string(), RetryPreferences::aggressive_preset());
    presets.insert("minimal".to_string(), RetryPreferences::minimal_preset());
    presets.insert("default".to_string(), RetryPreferences::default());
    
    Ok(presets)
}

#[tauri::command]
pub async fn apply_settings_preset(
    token: String,
    preset_name: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<UserSettings, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(_) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Get the preset
    let retry_preferences = match preset_name.as_str() {
        "conservative" => RetryPreferences::conservative_preset(),
        "aggressive" => RetryPreferences::aggressive_preset(),
        "minimal" => RetryPreferences::minimal_preset(),
        "default" => RetryPreferences::default(),
        _ => return Err("Unknown preset name".to_string()),
    };

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let settings_repo = SqliteUserSettingsRepository::new(db.get_connection());
            
            // Get current settings or create new ones
            let mut settings = settings_repo.get_or_create_settings(session.user_id)
                .map_err(|e| format!("Failed to get current settings: {}", e))?;
            
            // Apply the preset
            settings.retry_preferences = retry_preferences;
            settings.updated_at = chrono::Utc::now();
            
            // Save updated settings
            settings_repo.update_settings(&settings)
                .map_err(|e| format!("Failed to apply settings preset: {}", e))?;
            
            info!("Settings preset '{}' applied by {}", preset_name, session.username);
            Ok(settings)
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn get_operation_retry_configs() -> Result<std::collections::HashMap<String, crate::user_settings::RetryStrategy>, String> {
    Ok(settings_utils::get_default_operation_configs())
}

#[tauri::command]
pub async fn get_circuit_breaker_configs() -> Result<std::collections::HashMap<String, crate::user_settings::CircuitBreakerConfig>, String> {
    Ok(settings_utils::get_default_circuit_breaker_configs())
}

#[tauri::command]
pub async fn validate_retry_preferences(
    retry_preferences: RetryPreferences,
) -> Result<bool, String> {
    match retry_preferences.validate() {
        Ok(()) => Ok(true),
        Err(e) => Err(format!("Validation failed: {}", e)),
    }
}