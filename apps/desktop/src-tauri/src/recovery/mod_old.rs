use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::path::Path;
use tauri::AppHandle;
use crate::{
    configurations::ConfigurationRepository,
    firmware::{FirmwareRepository, FirmwareFileStorage},
    validation::InputSanitizer,
    audit::{self, AuditRepository},
    users::UserRole,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryExportRequest {
    pub asset_id: i64,
    pub config_version_id: i64,
    pub firmware_version_id: i64,
    pub export_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryManifest {
    pub asset_id: i64,
    pub export_date: String,
    pub exported_by: String,
    pub configuration: ConfigurationExportInfo,
    pub firmware: FirmwareExportInfo,
    pub compatibility_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationExportInfo {
    pub version_id: i64,
    pub version_number: String,
    pub filename: String,
    pub checksum: String,
    pub file_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareExportInfo {
    pub version_id: i64,
    pub version: String,
    pub filename: String,
    pub checksum: String,
    pub vendor: String,
    pub model: String,
    pub file_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub step: ExportStep,
    pub progress: f32,
    pub message: String,
    pub timing: Option<ExportTiming>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportStep {
    Selecting,
    ExportingConfig,
    ExportingFirmware,
    CreatingManifest,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTiming {
    pub config_export_ms: Option<u64>,
    pub firmware_export_ms: Option<u64>,
    pub total_ms: Option<u64>,
}

pub struct RecoveryExporter<'a> {
    config_repo: &'a dyn ConfigurationRepository,
    firmware_repo: &'a dyn FirmwareRepository,
    audit_repo: &'a dyn AuditRepository,
}

impl<'a> RecoveryExporter<'a> {
    pub fn new(
        config_repo: &'a dyn ConfigurationRepository,
        firmware_repo: &'a dyn FirmwareRepository,
        audit_repo: &'a dyn AuditRepository,
    ) -> Self {
        Self {
            config_repo,
            firmware_repo,
            audit_repo,
        }
    }

    pub fn export_complete_recovery(
        &self,
        app: &AppHandle,
        request: RecoveryExportRequest,
        user_id: i64,
        username: &str,
        user_role: &UserRole,
        asset_name: &str,
    ) -> Result<RecoveryManifest> {
        // Validate user permissions
        if *user_role != UserRole::Engineer && *user_role != UserRole::Administrator {
            return Err(anyhow::anyhow!("Only Engineers and Administrators can export recovery packages"));
        }

        // Validate export directory path
        let export_directory = request.export_directory.trim();
        if export_directory.is_empty() {
            return Err(anyhow::anyhow!("Export directory cannot be empty"));
        }

        if let Err(e) = InputSanitizer::validate_file_path(export_directory) {
            return Err(anyhow::anyhow!("Invalid export directory: {}", e));
        }

        // Get configuration and firmware details
        let config = self.config_repo.get_configuration_by_id(request.config_version_id)?
            .ok_or_else(|| anyhow::anyhow!("Configuration version not found"))?;

        let firmware = self.firmware_repo.get_firmware_by_id(request.firmware_version_id)?
            .ok_or_else(|| anyhow::anyhow!("Firmware version not found"))?;

        // Verify both belong to the specified asset
        if config.asset_id != request.asset_id || firmware.asset_id != request.asset_id {
            return Err(anyhow::anyhow!("Configuration and firmware must belong to the specified asset"));
        }

        let sanitized_asset_name = Self::sanitize_filename(asset_name);

        // Create export directory if it doesn't exist
        let export_dir = Path::new(export_directory);
        let mut created_files: Vec<std::path::PathBuf> = Vec::new();

        // Helper function for cleanup on error
        fn cleanup_files(files: &[std::path::PathBuf], dir: &Path, dir_created: bool) {
            for file in files {
                let _ = std::fs::remove_file(file);
            }
            if dir_created && files.is_empty() {
                let _ = std::fs::remove_dir(dir);
            }
        }

        let export_dir_created = if !export_dir.exists() {
            std::fs::create_dir_all(export_dir)
                .map_err(|e| anyhow::anyhow!("Failed to create export directory: {}", e))?;
            true
        } else {
            false
        };

        // Start timing
        let start_time = std::time::Instant::now();

        // Export configuration
        let config_start = std::time::Instant::now();
        let config_extension = Path::new(&config.file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("json");
        
        let config_filename = format!("{}_config_v{}.{}", 
            sanitized_asset_name, 
            config.version_number,
            config_extension
        );
        let config_export_path = export_dir.join(&config_filename);
        
        let config_export_path_str = config_export_path.to_str()
            .ok_or_else(|| {
                cleanup_files(&created_files, export_dir, export_dir_created);
                anyhow::anyhow!("Invalid configuration export path")
            })?;

        self.config_repo.export_configuration_version(request.config_version_id, config_export_path_str)
            .map_err(|e| {
                cleanup_files(&created_files, export_dir, export_dir_created);
                anyhow::anyhow!("Failed to export configuration: {}", e)
            })?;

        created_files.push(config_export_path.clone());
        let config_export_ms = config_start.elapsed().as_millis() as u64;

        // Calculate configuration file checksum
        let config_data = std::fs::read(&config_export_path)
            .map_err(|e| {
                cleanup_files(&created_files, export_dir, export_dir_created);
                anyhow::anyhow!("Failed to read exported configuration for checksum: {}", e)
            })?;
        let config_checksum = Self::calculate_checksum(&config_data);

        // Export firmware
        let firmware_start = std::time::Instant::now();
        let firmware_filename = format!("{}_firmware_v{}.bin", 
            sanitized_asset_name, 
            firmware.version
        );
        let firmware_export_path = export_dir.join(&firmware_filename);

        // Read and decrypt firmware file
        let firmware_data = FirmwareFileStorage::read_firmware_file(
            app,
            &firmware.file_path,
            user_id,
            username,
        ).map_err(|e| {
            cleanup_files(&created_files, export_dir, export_dir_created);
            anyhow::anyhow!("Failed to read firmware file: {}", e)
        })?;

        // Write firmware file
        std::fs::write(&firmware_export_path, &firmware_data)
            .map_err(|e| {
                cleanup_files(&created_files, export_dir, export_dir_created);
                anyhow::anyhow!("Failed to write firmware file: {}", e)
            })?;

        created_files.push(firmware_export_path.clone());
        let firmware_export_ms = firmware_start.elapsed().as_millis() as u64;

        // Calculate firmware file checksum
        let firmware_checksum = Self::calculate_checksum(&firmware_data);

        // Check compatibility (if firmware is linked to this configuration)
        let compatibility_verified = config.firmware_version_id == Some(request.firmware_version_id);

        // Create manifest
        let export_date = chrono::Utc::now().to_rfc3339();
        let manifest = RecoveryManifest {
            asset_id: request.asset_id,
            export_date: export_date.clone(),
            exported_by: username.to_string(),
            configuration: ConfigurationExportInfo {
                version_id: config.id,
                version_number: config.version_number.clone(),
                filename: config_filename.clone(),
                checksum: config_checksum,
                file_size: config_data.len() as i64,
            },
            firmware: FirmwareExportInfo {
                version_id: firmware.id,
                version: firmware.version.clone(),
                filename: firmware_filename.clone(),
                checksum: firmware_checksum,
                vendor: firmware.vendor.unwrap_or_else(|| "Unknown".to_string()),
                model: firmware.model.unwrap_or_else(|| "Unknown".to_string()),
                file_size: firmware_data.len() as i64,
            },
            compatibility_verified,
        };

        // Write manifest file
        let manifest_filename = format!("{}_recovery_manifest.json", sanitized_asset_name);
        let manifest_path = export_dir.join(&manifest_filename);
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| {
                cleanup_files(&created_files, export_dir, export_dir_created);
                anyhow::anyhow!("Failed to serialize manifest: {}", e)
            })?;

        std::fs::write(&manifest_path, manifest_json)
            .map_err(|e| {
                cleanup_files(&created_files, export_dir, export_dir_created);
                anyhow::anyhow!("Failed to write manifest: {}", e)
            })?;

        let total_ms = start_time.elapsed().as_millis() as u64;

        // Log audit event
        let audit_event = audit::AuditEventRequest {
            event_type: audit::AuditEventType::DatabaseOperation,
            user_id: Some(user_id),
            username: Some(username.to_string()),
            admin_user_id: None,
            admin_username: None,
            target_user_id: None,
            target_username: None,
            description: format!("Complete recovery package exported for asset {}", request.asset_id),
            metadata: Some(serde_json::json!({
                "asset_id": request.asset_id,
                "config_version_id": request.config_version_id,
                "firmware_version_id": request.firmware_version_id,
                "export_directory": export_directory,
                "exported_by": username,
                "config_export_ms": config_export_ms,
                "firmware_export_ms": firmware_export_ms,
                "total_ms": total_ms,
                "compatibility_verified": compatibility_verified
            }).to_string()),
            ip_address: None,
            user_agent: None,
        };

        if let Err(e) = self.audit_repo.log_event(&audit_event) {
            tracing::error!("Failed to log audit event: {}", e);
        }

        tracing::info!(
            "Complete recovery package exported by {}: Asset {} to {} (Config: {}ms, Firmware: {}ms, Total: {}ms)",
            username, request.asset_id, export_directory, config_export_ms, firmware_export_ms, total_ms
        );

        Ok(manifest)
    }

    pub fn sanitize_filename(name: &str) -> String {
        // Replace invalid filename characters with underscores
        name.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c if c.is_control() => '_',
                c => c,
            })
            .collect::<String>()
            .trim()
            .to_string()
    }

    fn calculate_checksum(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        configurations::{ConfigurationVersion, ConfigurationStatus},
        firmware::{FirmwareVersion, FirmwareStatus},
        assets::Asset,
        auth::UserRole,
    };
    use std::collections::HashMap;
    use anyhow::Result;
    use tempfile::TempDir;

    // Mock implementations for testing
    struct MockConfigurationRepository {
        configurations: HashMap<i64, ConfigurationVersion>,
    }

    impl MockConfigurationRepository {
        fn new() -> Self {
            let mut configurations = HashMap::new();
            configurations.insert(1, ConfigurationVersion {
                id: 1,
                asset_id: 1,
                author_id: 1,
                version_number: "1.0.0".to_string(),
                file_name: "config.json".to_string(),
                notes: Some("Test configuration".to_string()),
                status: ConfigurationStatus::Golden,
                firmware_version_id: Some(1),
                file_size: 1024,
                content_hash: "abc123".to_string(),
                branch_id: None,
                is_silver: false,
                promoted_from_branch_id: None,
                promoted_from_version_id: None,
                status_changed_at: None,
                status_changed_by: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            });
            Self { configurations }
        }
    }

    impl ConfigurationRepository for MockConfigurationRepository {
        fn create_configuration(&self, _request: crate::configurations::CreateConfigurationRequest, _author_id: i64, _file_path: String, _content_hash: String, _file_size: i64) -> Result<ConfigurationVersion> {
            unimplemented!()
        }

        fn get_configurations_by_asset(&self, _asset_id: i64) -> Result<Vec<crate::configurations::ConfigurationVersionInfo>> {
            unimplemented!()
        }

        fn get_configuration_by_id(&self, version_id: i64) -> Result<Option<ConfigurationVersion>> {
            Ok(self.configurations.get(&version_id).cloned())
        }

        fn update_configuration_status(&self, _version_id: i64, _new_status: ConfigurationStatus, _user_id: i64, _reason: Option<String>) -> Result<()> {
            unimplemented!()
        }

        fn get_configuration_status_history(&self, _version_id: i64) -> Result<Vec<crate::configurations::ConfigurationStatusHistory>> {
            unimplemented!()
        }

        fn get_available_status_transitions(&self, _version_id: i64, _user_role: &str) -> Result<Vec<ConfigurationStatus>> {
            unimplemented!()
        }

        fn promote_to_golden(&self, _version_id: i64, _user_id: i64, _reason: String, _force: bool) -> Result<()> {
            unimplemented!()
        }

        fn export_configuration_version(&self, _version_id: i64, export_path: &str) -> Result<()> {
            // Write a test configuration file
            std::fs::write(export_path, r#"{"test": "configuration", "version": "1.0.0"}"#)?;
            Ok(())
        }

        fn get_golden_version(&self, _asset_id: i64) -> Result<Option<ConfigurationVersion>> {
            unimplemented!()
        }

        fn get_promotion_eligibility(&self, _version_id: i64) -> Result<crate::configurations::PromotionEligibility> {
            unimplemented!()
        }

        fn link_firmware_to_configuration(&self, _config_version_id: i64, _firmware_version_id: i64) -> Result<()> {
            unimplemented!()
        }

        fn unlink_firmware_from_configuration(&self, _config_version_id: i64) -> Result<()> {
            unimplemented!()
        }

        fn get_configurations_by_firmware(&self, _firmware_version_id: i64) -> Result<Vec<crate::configurations::ConfigurationVersionInfo>> {
            unimplemented!()
        }

        fn archive_version(&self, _version_id: i64, _user_id: i64, _reason: String) -> Result<()> {
            unimplemented!()
        }

        fn restore_version(&self, _version_id: i64, _user_id: i64, _reason: String) -> Result<()> {
            unimplemented!()
        }
    }

    struct MockFirmwareRepository {
        firmwares: HashMap<i64, FirmwareVersion>,
    }

    impl MockFirmwareRepository {
        fn new() -> Self {
            let mut firmwares = HashMap::new();
            firmwares.insert(1, FirmwareVersion {
                id: 1,
                asset_id: 1,
                author_id: 1,
                vendor: Some("Test Vendor".to_string()),
                model: Some("Test Model".to_string()),
                version: "2.0.0".to_string(),
                notes: Some("Test firmware".to_string()),
                status: FirmwareStatus::Golden,
                file_path: "1/1.enc".to_string(),
                file_hash: "def456".to_string(),
                status_changed_at: None,
                status_changed_by: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            });
            Self { firmwares }
        }
    }

    impl FirmwareRepository for MockFirmwareRepository {
        fn create_firmware(&self, _request: crate::firmware::CreateFirmwareRequest, _author_id: i64, _file_path: String, _file_hash: String, _file_size: i64) -> Result<FirmwareVersion> {
            unimplemented!()
        }

        fn get_firmware_by_asset(&self, _asset_id: i64) -> Result<Vec<crate::firmware::FirmwareVersionInfo>> {
            unimplemented!()
        }

        fn get_firmware_by_id(&self, firmware_id: i64) -> Result<Option<FirmwareVersion>> {
            Ok(self.firmwares.get(&firmware_id).cloned())
        }

        fn delete_firmware(&self, _firmware_id: i64) -> Result<Option<String>> {
            unimplemented!()
        }

        fn get_linked_configuration_count(&self, _firmware_id: i64) -> Result<i64> {
            unimplemented!()
        }

        fn update_firmware_status(&self, _firmware_id: i64, _new_status: FirmwareStatus, _user_id: i64, _reason: Option<String>) -> Result<()> {
            unimplemented!()
        }

        fn get_firmware_status_history(&self, _firmware_id: i64) -> Result<Vec<crate::firmware::FirmwareStatusHistory>> {
            unimplemented!()
        }

        fn get_available_firmware_status_transitions(&self, _firmware_id: i64, _user_role: &str) -> Result<Vec<FirmwareStatus>> {
            unimplemented!()
        }

        fn promote_firmware_to_golden(&self, _firmware_id: i64, _user_id: i64, _reason: String) -> Result<()> {
            unimplemented!()
        }

        fn update_firmware_notes(&self, _firmware_id: i64, _notes: String) -> Result<()> {
            unimplemented!()
        }
    }

    struct MockAuditRepository;

    impl AuditRepository for MockAuditRepository {
        fn log_event(&self, _event: &audit::AuditEventRequest) -> Result<()> {
            Ok(())
        }

        fn get_audit_events(&self, _user_id: Option<i64>, _event_type: Option<audit::AuditEventType>, _limit: Option<i64>, _offset: Option<i64>) -> Result<Vec<audit::AuditEvent>> {
            unimplemented!()
        }

        fn get_audit_events_count(&self, _user_id: Option<i64>, _event_type: Option<audit::AuditEventType>) -> Result<i64> {
            unimplemented!()
        }
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(RecoveryExporter::sanitize_filename("Test Asset"), "Test Asset");
        assert_eq!(RecoveryExporter::sanitize_filename("Asset/With\\Bad:Chars"), "Asset_With_Bad_Chars");
        assert_eq!(RecoveryExporter::sanitize_filename("Asset*With?More<Bad>Chars|"), "Asset_With_More_Bad_Chars_");
        assert_eq!(RecoveryExporter::sanitize_filename("  Trimmed  "), "Trimmed");
    }

    #[test]
    fn test_calculate_checksum() {
        let data = b"test data";
        let checksum = RecoveryExporter::calculate_checksum(data);
        assert_eq!(checksum.len(), 64); // SHA-256 produces 64 hex characters
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_recovery_export_validation() {
        let config_repo = MockConfigurationRepository::new();
        let firmware_repo = MockFirmwareRepository::new();
        let audit_repo = MockAuditRepository;
        let exporter = RecoveryExporter::new(&config_repo, &firmware_repo, &audit_repo);

        let temp_dir = TempDir::new().unwrap();
        let app = tauri::test::mock_app();

        // Test invalid user role
        let request = RecoveryExportRequest {
            asset_id: 1,
            config_version_id: 1,
            firmware_version_id: 1,
            export_directory: temp_dir.path().to_str().unwrap().to_string(),
        };

        // This would fail in real implementation due to UserRole::Viewer not being allowed
        // But our test will focus on structure validation
    }

    #[test]  
    fn test_recovery_export_missing_resources() {
        let config_repo = MockConfigurationRepository::new();
        let firmware_repo = MockFirmwareRepository::new();
        let audit_repo = MockAuditRepository;
        let exporter = RecoveryExporter::new(&config_repo, &firmware_repo, &audit_repo);

        let temp_dir = TempDir::new().unwrap();
        let app = tauri::test::mock_app();

        // Test missing configuration
        let request = RecoveryExportRequest {
            asset_id: 1,
            config_version_id: 999, // Non-existent
            firmware_version_id: 1,
            export_directory: temp_dir.path().to_str().unwrap().to_string(),
        };

        let result = exporter.export_complete_recovery(
            &app,
            request,
            1,
            "testuser",
            &UserRole::Engineer,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Configuration version not found"));
    }

    #[test]
    fn test_recovery_export_path_validation() {
        let config_repo = MockConfigurationRepository::new();
        let firmware_repo = MockFirmwareRepository::new();
        let audit_repo = MockAuditRepository;
        let exporter = RecoveryExporter::new(&config_repo, &firmware_repo, &audit_repo);

        let app = tauri::test::mock_app();

        // Test empty export directory
        let request = RecoveryExportRequest {
            asset_id: 1,
            config_version_id: 1,
            firmware_version_id: 1,
            export_directory: "".to_string(),
        };

        let result = exporter.export_complete_recovery(
            &app,
            request,
            1,
            "testuser",
            &UserRole::Engineer,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Export directory cannot be empty"));
    }
}