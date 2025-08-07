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
// Epic 5 modules
mod error_handling;
mod user_settings;
mod metadata;
mod security;
mod workflow;
mod associations;
mod bulk;

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
            commands::get_file_info,
            
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
            
            // Enhanced tree navigation commands (TODO: Re-enable after fixing imports)
            // commands::batch_load_tree_nodes,
            // commands::search_tree_nodes,
            // commands::get_tree_statistics,
            // commands::preload_tree_nodes,
            // commands::get_node_metadata,
            
            // Configuration management commands
            commands::import_configuration,
            commands::import_configuration_for_asset,
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
            commands::validate_retry_preferences,
            
            // Epic 5 - Metadata management commands
            commands::get_system_field_templates,
            commands::get_field_templates_by_category,
            commands::get_supported_field_types,
            commands::create_metadata_schema,
            commands::get_metadata_schemas,
            commands::get_metadata_schema_by_id,
            commands::update_metadata_schema,
            commands::delete_metadata_schema,
            commands::validate_metadata_schema,
            commands::get_template_usage_stats,
            commands::import_field_template,
            commands::get_schemas_for_asset_type,
            commands::validate_metadata_values,
            
            // Epic 5 - Metadata search commands (temporarily disabled)
            // commands::search_assets_by_metadata,
            // commands::get_metadata_search_suggestions,
            // commands::create_metadata_filter_preset,
            // commands::get_metadata_filter_presets,
            // commands::delete_filter_preset,
            // commands::get_search_analytics,
            // commands::find_similar_assets,
            // commands::search_assets_in_hierarchy,
            // commands::get_filterable_metadata_fields,
            
            // Epic 5 - Enhanced metadata API commands
            // commands::list_metadata_schemas,
            // commands::duplicate_metadata_schema,
            // commands::archive_metadata_schema,
            // commands::restore_metadata_schema,
            commands::metadata_commands::get_asset_metadata_full,
            commands::metadata_commands::update_asset_metadata_partial,
            // commands::copy_metadata_between_assets,
            // commands::validate_metadata_batch,
            // commands::test_metadata_schema,
            
            // Epic 5 - Advanced query API commands (temporarily disabled)
            // commands::get_metadata_field_statistics,
            // commands::search_metadata_values,
            // commands::aggregate_metadata_data,
            
            // Epic 5 - Bulk operations API commands (temporarily disabled)
            // commands::bulk_update_metadata,
            // commands::bulk_validate_metadata,
            // commands::bulk_delete_metadata,
            // commands::bulk_apply_schema,
            // commands::batch_import_metadata,
            
            // Epic 5 - Export/Import API commands (temporarily disabled)
            // commands::export_metadata_to_json,
            // commands::export_metadata_to_csv,
            // commands::export_metadata_to_xml,
            // commands::import_metadata_from_file,
            // commands::validate_import_data,
            
            // Epic 5 - Integration API commands (temporarily disabled)
            // commands::get_metadata_api_info,
            // commands::create_external_metadata_mapping,
            // commands::create_metadata_webhook,
            // commands::get_metadata_sync_status,
            // commands::sync_external_metadata_source,
            
            // Epic 5 - Security validation commands
            commands::validate_asset_name,
            commands::sanitize_asset_name,
            commands::check_name_compliance,
            commands::suggest_compliant_names,
            commands::validate_file_upload,
            commands::calculate_file_hash,
            commands::sanitize_filename,
            commands::verify_file_integrity,
            commands::get_validation_statistics,
            commands::perform_security_health_check,
            commands::get_audit_events,
            commands::export_audit_log,
            commands::get_security_metrics,
            commands::export_security_report,
            
            // Epic 5 - Workflow management commands (temporarily disabled)
            // commands::start_asset_creation_workflow,
            // commands::get_workflow_state,
            // commands::update_workflow_step,
            // commands::advance_workflow_step,
            // commands::resume_workflow,
            // commands::complete_workflow,
            // commands::cancel_workflow,
            // commands::save_workflow_draft,
            // commands::get_workflow_drafts,
            // commands::delete_workflow_draft,
            // commands::validate_workflow_step,
            // commands::get_active_workflows,
            // commands::validate_asset_creation_permissions,
            // commands::validate_security_classification,
            // commands::check_naming_compliance,
            // commands::audit_workflow_operation,
            // commands::get_workflow_step_data,
            // commands::get_resumable_workflows,
            // commands::has_resumable_workflows,
            
            // Epic 5 - Association management commands (temporarily disabled)
            // commands::create_file_association,
            // commands::get_asset_file_associations,
            // commands::remove_file_association,
            // commands::reorder_file_associations,
            // commands::search_associations,
            // commands::get_association_health_status,
            // commands::get_broken_associations,
            // commands::repair_association,
            // commands::validate_file_association,
            // commands::create_import_session,
            // commands::update_import_session_status,
            // commands::get_import_session,
            // commands::get_associations_by_validation_status,
            
            // Epic 5 - Bulk import commands
            commands::create_bulk_import_session,
            commands::get_bulk_import_sessions,
            commands::get_bulk_import_session_details,
            commands::delete_bulk_import_session,
            commands::upload_bulk_import_file,
            commands::validate_bulk_import_data,
            commands::start_bulk_import_processing,
            commands::get_bulk_import_progress,
            commands::pause_bulk_import,
            commands::resume_bulk_import,
            commands::cancel_bulk_import,
            commands::get_bulk_operation_stats,
            commands::create_import_template,
            commands::get_import_templates,
            commands::delete_import_template,
            commands::generate_import_template_csv,
            
            // Epic 5 - Bulk operations commands
            commands::start_bulk_move,
            commands::start_bulk_delete,
            commands::start_bulk_export,
            commands::start_bulk_classify,
            commands::get_bulk_operation_progress,
            commands::cancel_bulk_operation,
            commands::get_bulk_operation_history,
            commands::validate_bulk_move,
            commands::validate_bulk_delete,
            commands::validate_bulk_export,
            commands::validate_bulk_classify,
            commands::undo_bulk_operation
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