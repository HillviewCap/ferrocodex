use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::path::Path;
use tauri::AppHandle;
use crate::{
    configurations::{ConfigurationRepository, CreateConfigurationRequest},
    firmware::{FirmwareRepository, FirmwareFileStorage, CreateFirmwareRequest},
    vault::{VaultRepository, VaultInfo},
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
    pub include_vault: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryManifest {
    pub asset_id: i64,
    pub export_date: String,
    pub exported_by: String,
    pub configuration: ConfigurationExportInfo,
    pub firmware: FirmwareExportInfo,
    pub vault: Option<VaultExportInfo>,
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
pub struct VaultExportInfo {
    pub vault_id: i64,
    pub vault_name: String,
    pub filename: String,
    pub checksum: String,
    pub secret_count: usize,
    pub file_size: i64,
    pub encrypted: bool,
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
    ExportingVault,
    CreatingManifest,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTiming {
    pub config_export_ms: Option<u64>,
    pub firmware_export_ms: Option<u64>,
    pub vault_export_ms: Option<u64>,
    pub total_ms: Option<u64>,
}

pub struct RecoveryExporter<'a> {
    config_repo: &'a dyn ConfigurationRepository,
    firmware_repo: &'a dyn FirmwareRepository,
    vault_repo: &'a dyn VaultRepository,
    audit_repo: &'a dyn AuditRepository,
}

impl<'a> RecoveryExporter<'a> {
    pub fn new(
        config_repo: &'a dyn ConfigurationRepository,
        firmware_repo: &'a dyn FirmwareRepository,
        vault_repo: &'a dyn VaultRepository,
        audit_repo: &'a dyn AuditRepository,
    ) -> Self {
        Self {
            config_repo,
            firmware_repo,
            vault_repo,
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

        // Export vault if requested
        let mut vault_export_ms = 0u64;
        let vault_export_info = if request.include_vault.unwrap_or(false) {
            let vault_start = std::time::Instant::now();
            
            // Check if vault exists for this asset
            match self.vault_repo.get_vault_by_asset_id(request.asset_id)? {
                Some(vault_info) => {
                    let vault_filename = format!("{}_vault.json", sanitized_asset_name);
                    let vault_export_path = export_dir.join(&vault_filename);
                    
                    // Export vault data as JSON (encrypted secrets remain encrypted)
                    let vault_json = serde_json::to_string_pretty(&vault_info)
                        .map_err(|e| {
                            cleanup_files(&created_files, export_dir, export_dir_created);
                            anyhow::anyhow!("Failed to serialize vault data: {}", e)
                        })?;
                    
                    std::fs::write(&vault_export_path, &vault_json)
                        .map_err(|e| {
                            cleanup_files(&created_files, export_dir, export_dir_created);
                            anyhow::anyhow!("Failed to write vault file: {}", e)
                        })?;
                    
                    created_files.push(vault_export_path.clone());
                    vault_export_ms = vault_start.elapsed().as_millis() as u64;
                    
                    // Calculate vault file checksum
                    let vault_checksum = Self::calculate_checksum(vault_json.as_bytes());
                    
                    Some(VaultExportInfo {
                        vault_id: vault_info.vault.id,
                        vault_name: vault_info.vault.name.clone(),
                        filename: vault_filename,
                        checksum: vault_checksum,
                        secret_count: vault_info.secret_count,
                        file_size: vault_json.len() as i64,
                        encrypted: true,
                    })
                },
                None => None,
            }
        } else {
            None
        };

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
            vault: vault_export_info,
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
                "vault_export_ms": vault_export_ms,
                "vault_included": request.include_vault.unwrap_or(false),
                "total_ms": total_ms,
                "compatibility_verified": compatibility_verified
            }).to_string()),
            ip_address: None,
            user_agent: None,
        };

        if let Err(e) = self.audit_repo.log_event(&audit_event) {
            tracing::error!("Failed to log audit event: {}", e);
        }

        if request.include_vault.unwrap_or(false) {
            tracing::info!(
                "Complete recovery package exported by {}: Asset {} to {} (Config: {}ms, Firmware: {}ms, Vault: {}ms, Total: {}ms)",
                username, request.asset_id, export_directory, config_export_ms, firmware_export_ms, vault_export_ms, total_ms
            );
        } else {
            tracing::info!(
                "Complete recovery package exported by {}: Asset {} to {} (Config: {}ms, Firmware: {}ms, Total: {}ms)",
                username, request.asset_id, export_directory, config_export_ms, firmware_export_ms, total_ms
            );
        }

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryImportRequest {
    pub bundle_path: String,
    pub target_asset_id: i64,
    pub import_vault: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    pub step: ImportStep,
    pub progress: f32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportStep {
    ValidatingBundle,
    ImportingConfig,
    ImportingFirmware,
    ImportingVault,
    Completed,
    Error,
}

pub struct RecoveryImporter<'a> {
    config_repo: &'a dyn ConfigurationRepository,
    firmware_repo: &'a dyn FirmwareRepository,
    vault_repo: &'a dyn VaultRepository,
    audit_repo: &'a dyn AuditRepository,
}

impl<'a> RecoveryImporter<'a> {
    pub fn new(
        config_repo: &'a dyn ConfigurationRepository,
        firmware_repo: &'a dyn FirmwareRepository,
        vault_repo: &'a dyn VaultRepository,
        audit_repo: &'a dyn AuditRepository,
    ) -> Self {
        Self {
            config_repo,
            firmware_repo,
            vault_repo,
            audit_repo,
        }
    }

    pub fn validate_bundle_integrity(bundle_path: &str) -> Result<RecoveryManifest> {
        let bundle_dir = Path::new(bundle_path);
        if !bundle_dir.exists() || !bundle_dir.is_dir() {
            return Err(anyhow::anyhow!("Bundle directory does not exist or is not a directory"));
        }

        // Read manifest
        let manifest_files: Vec<_> = std::fs::read_dir(bundle_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_name()
                    .to_str()
                    .map(|name| name.ends_with("_recovery_manifest.json"))
                    .unwrap_or(false)
            })
            .collect();

        if manifest_files.is_empty() {
            return Err(anyhow::anyhow!("No manifest file found in bundle"));
        }

        if manifest_files.len() > 1 {
            return Err(anyhow::anyhow!("Multiple manifest files found in bundle"));
        }

        let manifest_path = manifest_files[0].path();
        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let manifest: RecoveryManifest = serde_json::from_str(&manifest_content)?;

        // Validate configuration file
        let config_path = bundle_dir.join(&manifest.configuration.filename);
        if !config_path.exists() {
            return Err(anyhow::anyhow!("Configuration file not found: {}", manifest.configuration.filename));
        }

        let config_data = std::fs::read(&config_path)?;
        let config_checksum = RecoveryExporter::calculate_checksum(&config_data);
        if config_checksum != manifest.configuration.checksum {
            return Err(anyhow::anyhow!("Configuration file checksum mismatch"));
        }

        // Validate firmware file
        let firmware_path = bundle_dir.join(&manifest.firmware.filename);
        if !firmware_path.exists() {
            return Err(anyhow::anyhow!("Firmware file not found: {}", manifest.firmware.filename));
        }

        let firmware_data = std::fs::read(&firmware_path)?;
        let firmware_checksum = RecoveryExporter::calculate_checksum(&firmware_data);
        if firmware_checksum != manifest.firmware.checksum {
            return Err(anyhow::anyhow!("Firmware file checksum mismatch"));
        }

        // Validate vault file if present
        if let Some(ref vault_info) = manifest.vault {
            let vault_path = bundle_dir.join(&vault_info.filename);
            if !vault_path.exists() {
                return Err(anyhow::anyhow!("Vault file not found: {}", vault_info.filename));
            }

            let vault_data = std::fs::read(&vault_path)?;
            let vault_checksum = RecoveryExporter::calculate_checksum(&vault_data);
            if vault_checksum != vault_info.checksum {
                return Err(anyhow::anyhow!("Vault file checksum mismatch"));
            }
        }

        Ok(manifest)
    }

    pub fn import_recovery_bundle(
        &self,
        app: &AppHandle,
        request: RecoveryImportRequest,
        user_id: i64,
        username: &str,
        user_role: &UserRole,
    ) -> Result<RecoveryManifest> {
        // Validate user permissions
        if *user_role != UserRole::Engineer && *user_role != UserRole::Administrator {
            return Err(anyhow::anyhow!("Only Engineers and Administrators can import recovery packages"));
        }

        // Validate bundle integrity
        let manifest = Self::validate_bundle_integrity(&request.bundle_path)?;

        // Verify target asset
        if manifest.asset_id != request.target_asset_id {
            return Err(anyhow::anyhow!("Bundle is for asset {}, but trying to import to asset {}", 
                manifest.asset_id, request.target_asset_id));
        }

        let bundle_dir = Path::new(&request.bundle_path);
        let start_time = std::time::Instant::now();

        // Import configuration
        let config_path = bundle_dir.join(&manifest.configuration.filename);
        let config_content = std::fs::read_to_string(&config_path)?;
        
        let config_request = CreateConfigurationRequest {
            asset_id: request.target_asset_id,
            file_content: config_content.into_bytes(),
            file_name: manifest.configuration.filename.clone(),
            author: user_id,
            notes: format!("Imported from recovery bundle by {} on {}", 
                username, chrono::Utc::now().to_rfc3339()),
        };

        let imported_config = self.config_repo.store_configuration(config_request)?;

        // Import firmware
        let firmware_path = bundle_dir.join(&manifest.firmware.filename);
        let firmware_data = std::fs::read(&firmware_path)?;

        // Store firmware file - need to create a temporary firmware ID
        // This will be replaced with the actual ID after creation
        let temp_firmware_id = 0;
        let (firmware_storage_path, _, _) = FirmwareFileStorage::store_firmware_file(
            app,
            request.target_asset_id,
            temp_firmware_id,
            &firmware_data,
            user_id,
            username,
        )?;

        let firmware_request = CreateFirmwareRequest {
            asset_id: request.target_asset_id,
            version: manifest.firmware.version.clone(),
            vendor: Some(manifest.firmware.vendor.clone()),
            model: Some(manifest.firmware.model.clone()),
            notes: Some(format!("Imported from recovery bundle by {} on {}", 
                username, chrono::Utc::now().to_rfc3339())),
        };

        // Calculate firmware file hash
        let mut hasher = Sha256::new();
        hasher.update(&firmware_data);
        let file_hash = format!("{:x}", hasher.finalize());

        let imported_firmware = self.firmware_repo.create_firmware(
            firmware_request,
            user_id,
            firmware_storage_path,
            file_hash,
            manifest.firmware.file_size,
        )?;

        // Import vault if requested and available
        if request.import_vault && manifest.vault.is_some() {
            if let Some(ref vault_info) = manifest.vault {
                let vault_path = bundle_dir.join(&vault_info.filename);
                let vault_content = std::fs::read_to_string(&vault_path)?;
                let vault_data: VaultInfo = serde_json::from_str(&vault_content)?;

                // Import vault with all secrets
                self.vault_repo.import_vault(&vault_data, user_id)?;
            }
        }

        // Link firmware to configuration if they were linked in the original
        if manifest.compatibility_verified {
            self.config_repo.link_firmware_to_configuration(
                imported_config.id,
                imported_firmware.id,
            )?;
        }

        // Log audit event
        let audit_event = audit::AuditEventRequest {
            event_type: audit::AuditEventType::DatabaseOperation,
            user_id: Some(user_id),
            username: Some(username.to_string()),
            admin_user_id: None,
            admin_username: None,
            target_user_id: None,
            target_username: None,
            description: format!("Recovery bundle imported for asset {}", request.target_asset_id),
            metadata: Some(serde_json::json!({
                "asset_id": request.target_asset_id,
                "config_version_id": imported_config.id,
                "firmware_version_id": imported_firmware.id,
                "bundle_path": request.bundle_path,
                "vault_imported": request.import_vault && manifest.vault.is_some(),
                "imported_by": username,
                "total_ms": start_time.elapsed().as_millis(),
            }).to_string()),
            ip_address: None,
            user_agent: None,
        };

        if let Err(e) = self.audit_repo.log_event(&audit_event) {
            tracing::error!("Failed to log audit event: {}", e);
        }

        tracing::info!(
            "Recovery bundle imported by {}: Asset {} from {} (Total: {}ms)",
            username, request.target_asset_id, request.bundle_path, 
            start_time.elapsed().as_millis()
        );

        Ok(manifest)
    }
}

#[cfg(test)]
mod tests;