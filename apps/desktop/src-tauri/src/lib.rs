// Ferrocodex - Secure OT Configuration Management Platform
// Modular Rust backend using Tauri 2.0

// Core modules
mod database;
mod users;
mod auth;
mod audit;
mod validation;
mod assets;
mod configurations;
mod encryption;
mod branches;
mod firmware;
mod firmware_analysis;
mod recovery;
mod vault;
mod error_handling;
mod user_settings;

// Modular command handlers
mod commands;
mod handlers;

// Re-export database and state types for use in commands
use database::Database;
use auth::{SessionManager, LoginAttemptTracker};
use validation::RateLimiter;
use std::sync::Mutex;
use std::time::Duration;
use tauri::State;
use tracing::info;
use serde::{Serialize, Deserialize};

// State type definitions
pub type DatabaseState = Mutex<Option<Database>>;
pub type SessionManagerState = Mutex<SessionManager>;
pub type LoginAttemptTrackerState = Mutex<LoginAttemptTracker>;
pub type RateLimiterState = Mutex<RateLimiter>;


// Main application entry point
pub fn run() {
    tracing_subscriber::fmt::init();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .manage(DatabaseState::default())
        .manage(SessionManagerState::default())
        .manage(LoginAttemptTrackerState::default())
        .manage(RateLimiterState::new(RateLimiter::new(10, Duration::from_secs(60))))
        .invoke_handler(tauri::generate_handler![
            // System commands
            commands::greet,
            commands::initialize_database,
            commands::database_health_check,
            commands::is_first_launch,
            
            // Auth commands
            commands::create_admin_account,
            commands::login,
            commands::logout,
            commands::check_session,
            
            // User management commands
            commands::create_engineer_user,
            commands::list_users,
            commands::deactivate_user,
            commands::reactivate_user,
            
            // Asset management commands
            commands::create_asset,
            commands::create_folder_asset,
            commands::create_device_asset,
            commands::get_dashboard_assets,
            commands::get_dashboard_stats,
            commands::get_asset_details,
            commands::get_asset_hierarchy,
            commands::get_children_assets,
            commands::move_asset,
            commands::validate_asset_move,
            commands::get_asset_path,
            
            // Configuration management commands
            commands::import_configuration,
            commands::get_configuration_versions,
            commands::update_configuration_status,
            commands::get_configuration_status_history,
            commands::get_available_status_transitions,
            commands::promote_to_golden,
            commands::promote_branch_to_silver,
            commands::get_golden_version,
            commands::get_promotion_eligibility,
            commands::export_configuration_version,
            commands::get_file_metadata,
            commands::archive_version,
            commands::restore_version,
            
            // Branch management commands
            commands::create_branch,
            commands::get_branches,
            commands::get_branch_details,
            commands::import_version_to_branch,
            commands::get_branch_versions,
            commands::get_branch_latest_version,
            commands::compare_branch_versions,
            
            // Firmware management commands
            commands::link_firmware_to_configuration,
            commands::unlink_firmware_from_configuration,
            commands::get_configurations_by_firmware,
            commands::export_complete_recovery,
            commands::upload_firmware,
            commands::get_firmware_list,
            commands::delete_firmware,
            commands::get_firmware_analysis,
            commands::retry_firmware_analysis,
            commands::update_firmware_status,
            commands::get_firmware_status_history,
            commands::get_available_firmware_status_transitions,
            commands::promote_firmware_to_golden,
            commands::update_firmware_notes,
            
            // Vault management commands
            commands::create_identity_vault,
            commands::add_vault_secret,
            commands::get_vault_by_asset_id,
            commands::get_vault_history,
            commands::decrypt_vault_secret,
            commands::export_vault,
            commands::import_vault_from_recovery,
            commands::generate_secure_password,
            commands::validate_password_strength,
            commands::check_password_reuse,
            commands::get_password_history,
            commands::update_credential_password,
            commands::update_vault_secret,
            commands::delete_vault_secret,
            
            // Standalone credential commands
            commands::create_standalone_credential,
            commands::search_credentials,
            commands::get_credential_categories,
            commands::manage_credential_categories,
            commands::get_credential_history,
            commands::update_standalone_credential,
            commands::delete_standalone_credential,
            commands::get_standalone_credential,
            commands::decrypt_standalone_credential,
            
            // Recovery bundle commands
            commands::get_export_options,
            commands::preview_recovery_bundle,
            commands::import_recovery_bundle,
            commands::validate_bundle_integrity,
            
            // Vault access/permission commands
            commands::check_vault_access,
            commands::grant_vault_access,
            commands::revoke_vault_access,
            commands::get_user_vault_permissions,
            commands::get_vault_permissions,
            commands::get_vault_access_log,
            commands::create_permission_request,
            
            // Password rotation commands
            commands::rotate_password,
            commands::get_rotation_schedule,
            commands::create_rotation_batch,
            commands::get_rotation_history,
            commands::update_rotation_policy,
            commands::get_rotation_alerts,
            commands::execute_batch_rotation,
            commands::create_rotation_schedule,
            commands::get_rotation_compliance_metrics,
            commands::get_batch_rotation_history,
            
            // User settings commands
            commands::get_user_settings,
            commands::update_user_settings,
            commands::get_retry_presets,
            commands::apply_settings_preset,
            commands::get_operation_retry_configs,
            commands::get_circuit_breaker_configs,
            commands::validate_retry_preferences
        ])
        .setup(|_app| {
            info!("Ferrocodex application starting up...");
            
            // Analysis queue will be initialized after database is ready
            // For now, we'll initialize it on first use
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}