// Asset business logic handlers

use crate::assets::{AssetRepository, SqliteAssetRepository, CreateAssetRequest, AssetInfo, Asset, DashboardStats, AssetType};
use crate::database::Database;
use crate::validation::InputSanitizer;
use rusqlite::Connection;
use tracing::{error, info};

pub struct AssetHandler;

impl AssetHandler {
    pub fn new() -> Self {
        Self
    }

    /// Create a new asset with validation
    pub fn create_asset(
        &self,
        db: &Database,
        name: String,
        description: String,
        created_by: i64,
    ) -> Result<AssetInfo, String> {
        // Validate inputs
        let name = InputSanitizer::sanitize_string(&name);
        let description = InputSanitizer::sanitize_string(&description);

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

        let asset_repo = SqliteAssetRepository::new(db.get_connection());
        let request = CreateAssetRequest {
            name,
            description,
            asset_type: AssetType::Device,
            parent_id: None,
            created_by,
        };

        match asset_repo.create_asset(request) {
            Ok(asset) => {
                info!("Asset created: {} (ID: {})", asset.name, asset.id);
                Ok(asset.into())
            }
            Err(e) => {
                error!("Failed to create asset: {}", e);
                Err(format!("Failed to create asset: {}", e))
            }
        }
    }

    /// Get all assets with detailed information for dashboard
    pub fn get_dashboard_assets(&self, db: &Database) -> Result<Vec<AssetInfo>, String> {
        let asset_repo = SqliteAssetRepository::new(db.get_connection());
        
        match asset_repo.get_assets_with_info() {
            Ok(assets) => {
                info!("Retrieved {} assets for dashboard", assets.len());
                Ok(assets)
            }
            Err(e) => {
                error!("Failed to get dashboard assets: {}", e);
                Err(format!("Failed to get dashboard assets: {}", e))
            }
        }
    }

    /// Get dashboard statistics
    pub fn get_dashboard_stats(&self, db: &Database) -> Result<DashboardStats, String> {
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
        
        info!("Retrieved dashboard stats: {} assets, {} versions", total_assets, total_versions);
        Ok(stats)
    }

    /// Get detailed information about a specific asset
    pub fn get_asset_details(&self, db: &Database, asset_id: i64) -> Result<AssetInfo, String> {
        let asset_repo = SqliteAssetRepository::new(db.get_connection());
        
        match asset_repo.get_asset_by_id(asset_id) {
            Ok(Some(asset)) => {
                info!("Retrieved asset details: {} (ID: {})", asset.name, asset.id);
                Ok(asset.into())
            }
            Ok(None) => {
                error!("Asset not found: ID {}", asset_id);
                Err("Asset not found".to_string())
            }
            Err(e) => {
                error!("Failed to get asset details: {}", e);
                Err(format!("Failed to get asset details: {}", e))
            }
        }
    }

    /// Validate asset name
    pub fn validate_asset_name(&self, name: &str) -> Result<(), String> {
        let sanitized_name = InputSanitizer::sanitize_string(name);
        
        if sanitized_name.trim().is_empty() {
            return Err("Asset name cannot be empty".to_string());
        }
        if sanitized_name.len() < 2 {
            return Err("Asset name must be at least 2 characters long".to_string());
        }
        if sanitized_name.len() > 100 {
            return Err("Asset name cannot exceed 100 characters".to_string());
        }
        
        Ok(())
    }

    /// Validate asset description
    pub fn validate_asset_description(&self, description: &str) -> Result<(), String> {
        if description.len() > 500 {
            return Err("Description cannot exceed 500 characters".to_string());
        }
        
        Ok(())
    }
}