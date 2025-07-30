// Configuration business logic handlers

use crate::assets::{AssetRepository, SqliteAssetRepository, AssetInfo, CreateAssetRequest};
use crate::configurations::{ConfigurationRepository, SqliteConfigurationRepository, ConfigurationVersionInfo, ConfigurationStatus, StatusChangeRecord, FileMetadata, CreateConfigurationRequest};
use crate::branches::{BranchRepository, SqliteBranchRepository};
use crate::users::UserRole;
use crate::validation::InputSanitizer;
use crate::database::Database;
use std::fs;
use tracing::{error, info, warn};

pub struct ConfigurationHandler;

impl ConfigurationHandler {
    pub fn new() -> Self {
        Self
    }

    /// Import a configuration file and create a new asset
    pub fn import_configuration(
        &self,
        db: &Database,
        asset_name: &str,
        file_path: &str,
        notes: &str,
        user_id: i64,
    ) -> Result<AssetInfo, String> {
        let asset_repo = SqliteAssetRepository::new(db.get_connection());
        let config_repo = SqliteConfigurationRepository::new(db.get_connection());
        
        let start_time = std::time::Instant::now();
        
        // Read file content
        let file_content = fs::read(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Create asset first
        let asset_request = CreateAssetRequest {
            name: asset_name.to_string(),
            description: format!("Configuration asset - imported from {}", file_name),
            created_by: user_id,
        };

        let asset = asset_repo.create_asset(asset_request)
            .map_err(|e| format!("Failed to create asset: {}", e))?;

        // Store configuration
        let config_request = CreateConfigurationRequest {
            asset_id: asset.id,
            file_name,
            file_content,
            author: user_id,
            notes: notes.to_string(),
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

    /// Get all configuration versions for an asset
    pub fn get_configuration_versions(
        &self,
        db: &Database,
        asset_id: i64,
    ) -> Result<Vec<ConfigurationVersionInfo>, String> {
        let config_repo = SqliteConfigurationRepository::new(db.get_connection());
        
        config_repo.get_configuration_versions(asset_id)
            .map_err(|e| format!("Failed to get configuration versions: {}", e))
    }

    /// Update the status of a configuration version
    pub fn update_configuration_status(
        &self,
        db: &Database,
        version_id: i64,
        new_status: ConfigurationStatus,
        user_id: i64,
        user_role: &UserRole,
        change_reason: Option<String>,
    ) -> Result<(), String> {
        let config_repo = SqliteConfigurationRepository::new(db.get_connection());

        // Check if user has permission for this status transition
        let available_transitions = config_repo.get_available_status_transitions(version_id, &user_role.to_string())
            .map_err(|e| format!("Failed to check available transitions: {}", e))?;

        if !available_transitions.contains(&new_status) {
            return Err("You don't have permission to change to this status".to_string());
        }

        config_repo.update_configuration_status(version_id, new_status.clone(), user_id, change_reason)
            .map_err(|e| format!("Failed to update configuration status: {}", e))
    }

    /// Get the status change history for a configuration version
    pub fn get_configuration_status_history(
        &self,
        db: &Database,
        version_id: i64,
    ) -> Result<Vec<StatusChangeRecord>, String> {
        let config_repo = SqliteConfigurationRepository::new(db.get_connection());
        
        config_repo.get_configuration_status_history(version_id)
            .map_err(|e| format!("Failed to get configuration status history: {}", e))
    }

    /// Get available status transitions for a configuration version
    pub fn get_available_status_transitions(
        &self,
        db: &Database,
        version_id: i64,
        user_role: &str,
    ) -> Result<Vec<ConfigurationStatus>, String> {
        let config_repo = SqliteConfigurationRepository::new(db.get_connection());
        
        config_repo.get_available_status_transitions(version_id, user_role)
            .map_err(|e| format!("Failed to get available status transitions: {}", e))
    }

    /// Promote a configuration version to Golden status
    pub fn promote_to_golden(
        &self,
        db: &Database,
        version_id: i64,
        user_id: i64,
        user_role: &UserRole,
        promotion_reason: Option<String>,
    ) -> Result<(), String> {
        // Only Engineers and Administrators can promote to Golden
        if *user_role != UserRole::Engineer && *user_role != UserRole::Administrator {
            return Err("Only Engineers and Administrators can promote versions to Golden".to_string());
        }

        let config_repo = SqliteConfigurationRepository::new(db.get_connection());

        // Check promotion eligibility
        let is_eligible = config_repo.get_promotion_eligibility(version_id)
            .map_err(|e| format!("Failed to check promotion eligibility: {}", e))?;

        if !is_eligible {
            return Err("Version is not eligible for Golden promotion. Only Approved versions can be promoted.".to_string());
        }

        config_repo.promote_to_golden(version_id, user_id, promotion_reason)
            .map_err(|e| format!("Failed to promote to Golden: {}", e))
    }

    /// Promote a branch to Silver status by creating a new configuration version
    pub fn promote_branch_to_silver(
        &self,
        db: &Database,
        branch_id: i64,
        user_id: i64,
        user_role: &UserRole,
        promotion_notes: Option<String>,
    ) -> Result<i64, String> {
        // Both Engineers and Administrators can promote branches to Silver
        if *user_role != UserRole::Engineer && *user_role != UserRole::Administrator {
            return Err("Insufficient permissions to promote branch to Silver".to_string());
        }

        let branch_repo = SqliteBranchRepository::new(db.get_connection());
        let config_repo = SqliteConfigurationRepository::new(db.get_connection());
        
        // Get the branch details
        let branch = branch_repo.get_branch_by_id(branch_id)
            .map_err(|e| format!("Failed to get branch details: {}", e))?
            .ok_or("Branch not found".to_string())?;
        
        // Get the latest version of the branch
        let latest_version = branch_repo.get_branch_latest_version(branch_id)
            .map_err(|e| format!("Failed to get branch latest version: {}", e))?
            .ok_or("Branch has no versions to promote".to_string())?;
        
        // Get the configuration content
        let content = config_repo.get_configuration_content(latest_version.version_id)
            .map_err(|e| format!("Failed to get configuration content: {}", e))?;
        
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
            author: user_id,
            notes,
        };
        
        // Store the new configuration
        let new_config = config_repo.store_configuration(config_request)
            .map_err(|e| format!("Failed to create Silver configuration: {}", e))?;
        
        // Update the status to Silver
        config_repo.update_configuration_status(
            new_config.id,
            ConfigurationStatus::Silver,
            user_id,
            Some("Promoted from branch".to_string())
        )
        .map_err(|e| format!("Failed to set Silver status: {}", e))?;
        
        Ok(new_config.id)
    }

    /// Get the current Golden version for an asset
    pub fn get_golden_version(
        &self,
        db: &Database,
        asset_id: i64,
    ) -> Result<Option<ConfigurationVersionInfo>, String> {
        let config_repo = SqliteConfigurationRepository::new(db.get_connection());
        
        config_repo.get_golden_version(asset_id)
            .map_err(|e| format!("Failed to get Golden version: {}", e))
    }

    /// Check if a configuration version is eligible for Golden promotion
    pub fn get_promotion_eligibility(
        &self,
        db: &Database,
        version_id: i64,
    ) -> Result<bool, String> {
        let config_repo = SqliteConfigurationRepository::new(db.get_connection());
        
        config_repo.get_promotion_eligibility(version_id)
            .map_err(|e| format!("Failed to check promotion eligibility: {}", e))
    }

    /// Export a configuration version to a file
    pub fn export_configuration_version(
        &self,
        db: &Database,
        version_id: i64,
        export_path: &str,
    ) -> Result<(), String> {
        // Validate export path
        if export_path.trim().is_empty() {
            return Err("Export path cannot be empty".to_string());
        }
        
        if let Err(e) = InputSanitizer::validate_file_path(export_path) {
            error!("Invalid export path: {}", e);
            return Err(format!("Invalid export path: {}", e));
        }

        let config_repo = SqliteConfigurationRepository::new(db.get_connection());
        
        let start_time = std::time::Instant::now();
        
        match config_repo.export_configuration_version(version_id, export_path) {
            Ok(_) => {
                let duration = start_time.elapsed();
                
                // Log performance metrics
                if duration.as_secs() >= 2 {
                    warn!("Export operation took {} seconds, exceeding 2-second requirement", duration.as_secs_f64());
                } else {
                    info!("Export completed in {:.2} seconds", duration.as_secs_f64());
                }
                
                Ok(())
            }
            Err(e) => {
                error!("Failed to export configuration: {}", e);
                Err(format!("Failed to export configuration: {}", e))
            }
        }
    }

    /// Get metadata for a file
    pub fn get_file_metadata(
        &self,
        file_path: &str,
    ) -> Result<FileMetadata, String> {
        // Validate file path
        if let Err(e) = InputSanitizer::validate_file_path(file_path) {
            error!("Invalid file path: {}", e);
            return Err(format!("Invalid file path: {}", e));
        }

        crate::configurations::file_utils::get_file_metadata(file_path)
            .map_err(|e| format!("Failed to get file metadata: {}", e))
    }

    /// Archive a configuration version
    pub fn archive_version(
        &self,
        db: &Database,
        version_id: i64,
        user_id: i64,
        user_role: &UserRole,
        archive_reason: Option<String>,
    ) -> Result<(), String> {
        // Only Engineers and Administrators can archive versions
        if *user_role != UserRole::Engineer && *user_role != UserRole::Administrator {
            return Err("Only Engineers and Administrators can archive versions".to_string());
        }

        let config_repo = SqliteConfigurationRepository::new(db.get_connection());

        config_repo.archive_version(version_id, user_id, archive_reason)
            .map_err(|e| format!("Failed to archive version: {}", e))
    }

    /// Restore an archived configuration version
    pub fn restore_version(
        &self,
        db: &Database,
        version_id: i64,
        user_id: i64,
        user_role: &UserRole,
        restore_reason: Option<String>,
    ) -> Result<(), String> {
        // Only Engineers and Administrators can restore versions
        if *user_role != UserRole::Engineer && *user_role != UserRole::Administrator {
            return Err("Only Engineers and Administrators can restore versions".to_string());
        }

        let config_repo = SqliteConfigurationRepository::new(db.get_connection());

        config_repo.restore_version(version_id, user_id, restore_reason)
            .map_err(|e| format!("Failed to restore version: {}", e))
    }
}