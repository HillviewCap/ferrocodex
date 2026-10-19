use super::*;
use crate::vault::{VaultRepository, IdentityVault, VaultInfo, VaultSecret, CreateVaultRequest, AddSecretRequest, SecretType};
use crate::vault::{
    UpdateVaultSecretRequest, DeleteVaultSecretRequest, GrantVaultAccessRequest, RevokeVaultAccessRequest,
    CheckVaultAccessRequest, VaultAccessInfo, VaultPermission, AccessType, AccessResult, VaultAccessLog,
    CreatePermissionRequest, ApprovePermissionRequest, DenyPermissionRequest, PermissionRequest,
};
use crate::{
    configurations::{ConfigurationVersion, ConfigurationStatus},
    firmware::{FirmwareVersion, FirmwareStatus},
    users::UserRole,
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
                author: 1,
                version_number: "1.0.0".to_string(),
                file_name: "config.json".to_string(),
                file_content: vec![],  // Empty content for testing
                file_size: 1024,
                content_hash: "abc123".to_string(),
                notes: "Test configuration".to_string(),
                firmware_version_id: Some(1),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            });
            Self { configurations }
        }
    }

    impl ConfigurationRepository for MockConfigurationRepository {
        fn store_configuration(&self, _request: crate::configurations::CreateConfigurationRequest, _key: &DataKey) -> Result<ConfigurationVersion> {
            unimplemented!()
        }

        fn get_configuration_versions(&self, _asset_id: i64) -> Result<Vec<crate::configurations::ConfigurationVersionInfo>> {
            unimplemented!()
        }
        
        fn get_configuration_content(&self, _version_id: i64, _key: &DataKey) -> Result<Vec<u8>> {
            unimplemented!()
        }
        
        fn get_latest_version_number(&self, _asset_id: i64) -> Result<Option<String>> {
            unimplemented!()
        }
        
        fn delete_configuration_version(&self, _version_id: i64) -> Result<()> {
            unimplemented!()
        }
        
        fn get_configuration_count(&self, _asset_id: i64) -> Result<i64> {
            unimplemented!()
        }

        fn get_configuration_by_id(&self, version_id: i64) -> Result<Option<ConfigurationVersion>> {
            Ok(self.configurations.get(&version_id).cloned())
        }

        fn update_configuration_status(&self, _version_id: i64, _new_status: ConfigurationStatus, _user_id: i64, _reason: Option<String>) -> Result<()> {
            unimplemented!()
        }

        fn get_configuration_status_history(&self, _version_id: i64) -> Result<Vec<crate::configurations::StatusChangeRecord>> {
            unimplemented!()
        }

        fn get_available_status_transitions(&self, _version_id: i64, _user_role: &str) -> Result<Vec<ConfigurationStatus>> {
            unimplemented!()
        }

        fn promote_to_golden(&self, _version_id: i64, _promoted_by: i64, _promotion_reason: Option<String>) -> Result<()> {
            unimplemented!()
        }

        fn export_configuration_version(&self, _version_id: i64, export_path: &str, _key: &DataKey) -> Result<()> {
            // Write a test configuration file
            std::fs::write(export_path, r#"{"test": "configuration", "version": "1.0.0"}"#)?;
            Ok(())
        }

        fn get_golden_version(&self, _asset_id: i64) -> Result<Option<crate::configurations::ConfigurationVersionInfo>> {
            unimplemented!()
        }

        fn get_promotion_eligibility(&self, _version_id: i64) -> Result<bool> {
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

        fn archive_version(&self, _version_id: i64, _archived_by: i64, _archive_reason: Option<String>) -> Result<()> {
            unimplemented!()
        }

        fn restore_version(&self, _version_id: i64, _restored_by: i64, _restore_reason: Option<String>) -> Result<()> {
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
                file_size: 1024,
                status_changed_at: None,
                status_changed_by: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                original_filename: None,
            });
            Self { firmwares }
        }
    }

    impl FirmwareRepository for MockFirmwareRepository {
        fn create_firmware(&self, _request: crate::firmware::CreateFirmwareRequest, _author_id: i64, _file_path: String, _file_hash: String, _file_size: i64, _key_version: u32) -> Result<FirmwareVersion> {
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
        
        fn update_firmware_file_path(&self, _firmware_id: i64, _file_path: String) -> Result<()> {
            unimplemented!()
        }
    }

    struct MockVaultRepository {
        vaults: HashMap<i64, VaultInfo>,
    }

    impl MockVaultRepository {
        fn new() -> Self {
            let mut vaults = HashMap::new();
            let vault = IdentityVault {
                id: 1,
                asset_id: 1,
                name: "Test Vault".to_string(),
                description: "Test vault for unit tests".to_string(),
                created_by: 1,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
            };
            
            let secrets = vec![
                VaultSecret {
                    id: 1,
                    vault_id: 1,
                    secret_type: SecretType::Password,
                    label: "Admin Password".to_string(),
                    username: None,
                    encrypted_value: "encrypted_password_data".to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    updated_at: "2024-01-01T00:00:00Z".to_string(),
                    strength_score: Some(90),
                    last_changed: Some("2024-01-01T00:00:00Z".to_string()),
                    generation_method: Some("manual".to_string()),
                    policy_version: Some(1),
                    last_rotated: None,
                    rotation_interval_days: None,
                    next_rotation_due: None,
                    rotation_policy_id: None,
                    key_version: 1,
                },
                VaultSecret {
                    id: 2,
                    vault_id: 1,
                    secret_type: SecretType::IpAddress,
                    label: "PLC IP".to_string(),
                    username: None,
                    encrypted_value: "encrypted_ip_data".to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    updated_at: "2024-01-01T00:00:00Z".to_string(),
                    strength_score: None,
                    last_changed: None,
                    generation_method: None,
                    policy_version: None,
                    last_rotated: None,
                    rotation_interval_days: None,
                    next_rotation_due: None,
                    rotation_policy_id: None,
                    key_version: 1,
                },
            ];
            
            vaults.insert(1, VaultInfo {
                vault,
                secrets: secrets.clone(),
                secret_count: secrets.len(),
            });
            
            Self { vaults }
        }
    }

    impl VaultRepository for MockVaultRepository {
        fn create_vault(&self, _request: CreateVaultRequest) -> Result<IdentityVault> {
            unimplemented!()
        }
        
        fn get_vault_by_id(&self, _vault_id: i64) -> Result<Option<IdentityVault>> {
            unimplemented!()
        }
        
        fn get_vault_by_asset_id(&self, asset_id: i64) -> Result<Option<VaultInfo>> {
            if asset_id == 1 {
                Ok(self.vaults.get(&1).cloned())
            } else {
                Ok(None)
            }
        }
        
        fn update_vault(&self, _vault: &IdentityVault) -> Result<()> {
            unimplemented!()
        }
        
        fn delete_vault(&self, _vault_id: i64) -> Result<()> {
            unimplemented!()
        }
        
        fn add_secret(&self, _request: AddSecretRequest, _key: &DataKey) -> Result<VaultSecret> {
            unimplemented!()
        }
        
        fn get_vault_secrets(&self, _vault_id: i64) -> Result<Vec<VaultSecret>> {
            unimplemented!()
        }
        
        fn get_secret_by_id(&self, _secret_id: i64) -> Result<Option<VaultSecret>> {
            unimplemented!()
        }
        
        fn update_secret(&self, _secret: &VaultSecret, _author_id: i64, _key_version: u32) -> Result<()> {
            unimplemented!()
        }
        
        fn delete_secret(&self, _secret_id: i64, _author_id: i64) -> Result<()> {
            unimplemented!()
        }
        
        fn add_version_history(&self, _vault_id: i64, _change_type: crate::vault::ChangeType, _author: i64, _notes: &str, _changes: HashMap<String, String>) -> Result<()> {
            unimplemented!()
        }
        
        fn get_vault_history(&self, _vault_id: i64) -> Result<Vec<crate::vault::VaultVersion>> {
            unimplemented!()
        }
        
        fn import_vault(&self, _vault_info: &VaultInfo, _author_id: i64, _key_version: u32) -> Result<IdentityVault> {
            Ok(self.vaults.get(&1).unwrap().vault.clone())
        }
        
        fn initialize_schema(&self) -> Result<()> {
            Ok(())
        }
        
        // Password management methods
        fn add_password_history(&self, _secret_id: i64, _password_hash: &str) -> Result<()> {
            unimplemented!()
        }
        
        fn get_password_history(&self, _secret_id: i64) -> Result<Vec<crate::vault::PasswordHistory>> {
            unimplemented!()
        }
        
        
        fn update_password(&self, _request: crate::vault::UpdateCredentialPasswordRequest, _password_hash: &str, _strength_score: i32, _key: &DataKey) -> Result<()> {
            unimplemented!()
        }
        
        fn get_default_password_policy(&self) -> Result<crate::vault::PasswordPolicy> {
            unimplemented!()
        }
        
        fn cleanup_password_history(&self, _secret_id: i64, _keep_count: usize) -> Result<()> {
            unimplemented!()
        }
        
        // Standalone credential methods
        fn create_standalone_credential(&self, _request: crate::vault::CreateStandaloneCredentialRequest, _key: &DataKey) -> Result<crate::vault::StandaloneCredential> {
            unimplemented!()
        }
        
        fn get_standalone_credential(&self, _credential_id: i64) -> Result<Option<crate::vault::StandaloneCredentialInfo>> {
            unimplemented!()
        }
        
        fn update_standalone_credential(&self, _request: crate::vault::UpdateStandaloneCredentialRequest, _key: Option<&DataKey>) -> Result<()> {
            unimplemented!()
        }
        
        fn delete_standalone_credential(&self, _credential_id: i64, _author_id: i64) -> Result<()> {
            unimplemented!()
        }
        
        fn search_standalone_credentials(&self, _request: crate::vault::SearchCredentialsRequest) -> Result<crate::vault::SearchCredentialsResponse> {
            unimplemented!()
        }
        
        fn get_standalone_credential_history(&self, _credential_id: i64) -> Result<Vec<crate::vault::StandaloneCredentialHistory>> {
            unimplemented!()
        }
        
        fn update_credential_last_accessed(&self, _credential_id: i64) -> Result<()> {
            unimplemented!()
        }
        
        // Category management methods
        fn create_credential_category(&self, _request: crate::vault::CreateCategoryRequest) -> Result<crate::vault::CredentialCategory> {
            unimplemented!()
        }
        
        fn get_credential_categories(&self) -> Result<Vec<crate::vault::CategoryWithChildren>> {
            unimplemented!()
        }
        
        fn update_credential_category(&self, _category_id: i64, _request: crate::vault::CreateCategoryRequest) -> Result<()> {
            unimplemented!()
        }
        
        fn delete_credential_category(&self, _category_id: i64) -> Result<()> {
            unimplemented!()
        }
        
        fn get_category_by_id(&self, _category_id: i64) -> Result<Option<crate::vault::CredentialCategory>> {
            unimplemented!()
        }
        
        // Tag management methods
        fn add_credential_tags(&self, _credential_id: i64, _tags: &[String]) -> Result<()> {
            unimplemented!()
        }
        
        fn remove_credential_tag(&self, _credential_id: i64, _tag_name: &str) -> Result<()> {
            unimplemented!()
        }
        
        fn get_all_tags(&self) -> Result<Vec<String>> {
            unimplemented!()
        }
        
        fn update_vault_secret(&self, _request: UpdateVaultSecretRequest, _key: Option<&DataKey>) -> Result<()> {
            unimplemented!()
        }
        
        fn delete_vault_secret(&self, _request: DeleteVaultSecretRequest) -> Result<()> {
            unimplemented!()
        }
        
        fn grant_vault_access(&self, _request: GrantVaultAccessRequest) -> Result<VaultPermission> {
            unimplemented!()
        }
        
        fn revoke_vault_access(&self, _request: RevokeVaultAccessRequest) -> Result<()> {
            unimplemented!()
        }
        
        fn check_vault_access(&self, _request: CheckVaultAccessRequest) -> Result<VaultAccessInfo> {
            unimplemented!()
        }
        
        fn get_user_vault_permissions(&self, _user_id: i64, _vault_id: Option<i64>) -> Result<Vec<VaultPermission>> {
            unimplemented!()
        }
        
        fn get_vault_permissions(&self, _vault_id: i64) -> Result<Vec<VaultPermission>> {
            unimplemented!()
        }
        
        fn update_permission_expiry(&self, _permission_id: i64, _new_expires_at: Option<String>) -> Result<()> {
            unimplemented!()
        }
        
        fn expire_permissions(&self) -> Result<u64> {
            unimplemented!()
        }
        
        fn log_vault_access(&self, _user_id: i64, _vault_id: i64, _access_type: AccessType, _result: AccessResult, _error_message: Option<String>) -> Result<()> {
            unimplemented!()
        }
        
        fn get_vault_access_log(&self, _vault_id: i64, _limit: Option<i32>) -> Result<Vec<VaultAccessLog>> {
            unimplemented!()
        }
        
        fn get_user_access_log(&self, _user_id: i64, _limit: Option<i32>) -> Result<Vec<VaultAccessLog>> {
            unimplemented!()
        }
        
        fn create_permission_request(&self, _request: CreatePermissionRequest, _user_id: i64) -> Result<PermissionRequest> {
            unimplemented!()
        }
        
        fn approve_permission_request(&self, _request: ApprovePermissionRequest) -> Result<PermissionRequest> {
            unimplemented!()
        }
        
        fn deny_permission_request(&self, _request: DenyPermissionRequest) -> Result<PermissionRequest> {
            unimplemented!()
        }
        
        fn get_pending_permission_requests(&self, _admin_id: i64) -> Result<Vec<PermissionRequest>> {
            unimplemented!()
        }
        
        fn get_user_permission_requests(&self, _user_id: i64) -> Result<Vec<PermissionRequest>> {
            unimplemented!()
        }
        
        fn expire_permission_requests(&self) -> Result<u64> {
            unimplemented!()
        }
    }

    struct MockAuditRepository;

    impl AuditRepository for MockAuditRepository {
        fn initialize_schema(&self) -> Result<()> {
            Ok(())
        }
        
        fn log_event(&self, _event: &audit::AuditEventRequest) -> Result<audit::AuditEvent> {
            Ok(audit::AuditEvent {
                id: 1,
                event_type: audit::AuditEventType::UserLoginSuccessful,
                event_code: "USER_LOGIN_SUCCESS".to_string(),
                user_id: Some(1),
                username: Some("test".to_string()),
                admin_user_id: None,
                admin_username: None,
                target_user_id: None,
                target_username: None,
                description: "Test event".to_string(),
                metadata: None,
                ip_address: None,
                user_agent: None,
                timestamp: "2024-01-01T00:00:00Z".to_string(),
            })
        }

        fn get_events(&self, _limit: Option<usize>, _offset: Option<usize>) -> Result<Vec<audit::AuditEvent>> {
            unimplemented!()
        }
        
        fn get_events_by_user(&self, _user_id: i64) -> Result<Vec<audit::AuditEvent>> {
            unimplemented!()
        }
        
        fn get_events_by_type(&self, _event_type: &audit::AuditEventType) -> Result<Vec<audit::AuditEvent>> {
            unimplemented!()
        }
        
        fn cleanup_old_events(&self, _days_to_keep: u32) -> Result<u64> {
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

    /// A firmware export keeps the uploaded file's extension (a Modicon .sedp
    /// used to come out as .bin), but only a plain one; anything else is .bin.
    #[test]
    fn test_firmware_export_filename_uses_original_extension() {
        let name = |original: Option<&str>| RecoveryExporter::firmware_export_filename("PLC-1", "3.2", original);
        assert_eq!(name(Some("M580_fw.sedp")), "PLC-1_firmware_v3.2.sedp");
        assert_eq!(name(Some("image.HEX")), "PLC-1_firmware_v3.2.HEX");
        assert_eq!(name(Some("archive.tar.gz")), "PLC-1_firmware_v3.2.gz");
        // Unknown (records from before the name was kept) or no extension
        assert_eq!(name(None), "PLC-1_firmware_v3.2.bin");
        assert_eq!(name(Some("FIRMWARE")), "PLC-1_firmware_v3.2.bin");
        assert_eq!(name(Some(".hidden")), "PLC-1_firmware_v3.2.bin");
        assert_eq!(name(Some("trailing.")), "PLC-1_firmware_v3.2.bin");
        // Extensions that are not plain ASCII alphanumeric, or are too long
        assert_eq!(name(Some("evil.b/../x")), "PLC-1_firmware_v3.2.bin");
        assert_eq!(name(Some("evil.b\\..\\x")), "PLC-1_firmware_v3.2.bin");
        assert_eq!(name(Some("evil.b n")), "PLC-1_firmware_v3.2.bin");
        assert_eq!(name(Some("fw.bïn")), "PLC-1_firmware_v3.2.bin");
        assert_eq!(name(Some("fw.abcdefghijklmnopq")), "PLC-1_firmware_v3.2.bin");
    }

    #[test]
    fn test_calculate_checksum() {
        let data = b"test data";
        let checksum = RecoveryExporter::calculate_checksum(data);
        assert_eq!(checksum.len(), 64); // SHA-256 produces 64 hex characters
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    #[ignore = "Requires tauri test utilities"]
    fn test_recovery_export_validation() {
        // Test requires proper AppHandle mock
        // TODO: Implement when tauri test utilities are available
    }

    #[test]
    #[ignore = "Requires tauri test utilities"]
#[ignore = "Missing VaultRepository methods - temporary for deployment"]
    fn test_recovery_export_missing_resources() {
        let config_repo = MockConfigurationRepository::new();
        let firmware_repo = MockFirmwareRepository::new();
        let audit_repo = MockAuditRepository;
        let vault_repo = MockVaultRepository::new();
        let exporter = RecoveryExporter::new(&config_repo, &firmware_repo, &vault_repo, &audit_repo);

        let temp_dir = TempDir::new().unwrap();
        // TODO: Replace with proper AppHandle mock when tauri test utilities are available
        // let app = tauri::test::mock_app();

        // Test missing configuration
        let request = RecoveryExportRequest {
            asset_id: 1,
            config_version_id: 999, // Non-existent
            firmware_version_id: 1,
            export_directory: temp_dir.path().to_str().unwrap().to_string(),
            include_vault: Some(false),
        };

        // let result = exporter.export_complete_recovery(
        //     &app,
        //     request,
        //     1,
        //     "testuser",
        //     &UserRole::Engineer,
        //     "Test Asset",
        // );
        // assert!(result.is_err());
        // assert!(result.unwrap_err().to_string().contains("Configuration version not found"));
    }

    #[test]
    #[ignore = "Requires tauri test utilities"]
    fn test_recovery_export_path_validation() {
        let config_repo = MockConfigurationRepository::new();
        let firmware_repo = MockFirmwareRepository::new();
        let audit_repo = MockAuditRepository;
        let vault_repo = MockVaultRepository::new();
        let exporter = RecoveryExporter::new(&config_repo, &firmware_repo, &vault_repo, &audit_repo);

        // TODO: Replace with proper AppHandle mock when tauri test utilities are available
        // let app = tauri::test::mock_app();

        // Test empty export directory
        let request = RecoveryExportRequest {
            asset_id: 1,
            config_version_id: 1,
            firmware_version_id: 1,
            export_directory: "".to_string(),
            include_vault: Some(false),
        };

        // let result = exporter.export_complete_recovery(
        //     &app,
        //     request,
        //     1,
        //     "testuser",
        //     &UserRole::Engineer,
        //     "Test Asset",
        // );
        // assert!(result.is_err());
        // assert!(result.unwrap_err().to_string().contains("Export directory cannot be empty"));
    }

    #[test]
    fn test_bundle_validation_with_vault() {
        let temp_dir = TempDir::new().unwrap();
        let bundle_path = temp_dir.path();

        // Create a mock manifest with vault
        let manifest = RecoveryManifest {
            asset_id: 1,
            export_date: "2024-01-01T00:00:00Z".to_string(),
            exported_by: "test_user".to_string(),
            configuration: ConfigurationExportInfo {
                version_id: 1,
                version_number: "1.0.0".to_string(),
                filename: "test_config.json".to_string(),
                checksum: "config_checksum".to_string(),
                file_size: 1024,
            },
            firmware: FirmwareExportInfo {
                version_id: 1,
                version: "2.0.0".to_string(),
                filename: "test_firmware.bin".to_string(),
                checksum: "firmware_checksum".to_string(),
                vendor: "Test Vendor".to_string(),
                model: "Test Model".to_string(),
                file_size: 2048,
                original_filename: None,
            },
            vault: Some(VaultExportInfo {
                vault_id: 1,
                vault_name: "Test Vault".to_string(),
                filename: "test_vault.json".to_string(),
                checksum: "vault_checksum".to_string(),
                secret_count: 2,
                file_size: 512,
                encrypted: true,
            }),
            compatibility_verified: true,
        };

        // Write manifest file
        let manifest_path = bundle_path.join("test_recovery_manifest.json");
        std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();

        // Test validation fails without actual files
        let result = RecoveryImporter::validate_bundle_integrity(bundle_path.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Configuration file not found"));
    }

    #[test]
    fn test_checksum_consistency() {
        let data = b"test configuration data";
        let checksum1 = RecoveryExporter::calculate_checksum(data);
        let checksum2 = RecoveryExporter::calculate_checksum(data);
        assert_eq!(checksum1, checksum2);
        
        let different_data = b"different data";
        let checksum3 = RecoveryExporter::calculate_checksum(different_data);
        assert_ne!(checksum1, checksum3);
    }
/// Imports against real repositories and a real firmware directory.
mod import_storage {
    use super::super::*;
    use crate::audit::SqliteAuditRepository;
    use crate::configurations::SqliteConfigurationRepository;
    use crate::database::Database;
    use crate::encryption::KdfParams;
    use crate::firmware::SqliteFirmwareRepository;
    use crate::keystore::{KeyStoreRepository, SqliteKeyStoreRepository};
    use crate::users::{CreateUserRequest, SqliteUserRepository, UserRepository, UserRole};
    use crate::vault::SqliteVaultRepository;
    use rusqlite::Connection;
    use tempfile::TempDir;

    fn write_bundle(root: &Path, name: &str, firmware: &[u8]) -> String {
        write_bundle_with_vault(root, name, firmware, None)
    }

    /// `vault_json` is a sealed vault export to include in the bundle.
    fn write_bundle_with_vault(root: &Path, name: &str, firmware: &[u8], vault_json: Option<&str>) -> String {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let config = format!("config of {}", name);
        std::fs::write(dir.join("plc.cfg"), &config).unwrap();
        std::fs::write(dir.join("plc.bin"), firmware).unwrap();
        let vault = vault_json.map(|json| {
            std::fs::write(dir.join("plc_vault.json"), json).unwrap();
            VaultExportInfo {
                vault_id: 1, vault_name: "PLC vault".into(), filename: "plc_vault.json".into(),
                checksum: RecoveryExporter::calculate_checksum(json.as_bytes()), secret_count: 1,
                file_size: json.len() as i64, encrypted: true,
            }
        });
        let manifest = RecoveryManifest {
            asset_id: 1,
            export_date: "2026-10-18T00:00:00Z".into(),
            exported_by: "admin".into(),
            configuration: ConfigurationExportInfo {
                version_id: 1, version_number: "v1".into(), filename: "plc.cfg".into(),
                checksum: RecoveryExporter::calculate_checksum(config.as_bytes()), file_size: config.len() as i64,
            },
            firmware: FirmwareExportInfo {
                version_id: 1, version: name.into(), filename: "plc.bin".into(),
                checksum: RecoveryExporter::calculate_checksum(firmware), vendor: "v".into(), model: "m".into(),
                file_size: firmware.len() as i64, original_filename: Some(format!("../{}\u{7}.sedp", name)),
            },
            vault,
            compatibility_verified: false,
        };
        std::fs::write(dir.join("plc_recovery_manifest.json"), serde_json::to_string(&manifest).unwrap()).unwrap();
        dir.to_string_lossy().to_string()
    }

    /// H1(e): imports used to store every firmware file as `<asset>/0.enc`,
    /// so a second import overwrote the first one's file, and the name did not
    /// match the record. Each import must get `<asset>/<id>.enc`.
    #[test]
    fn test_imported_firmware_is_stored_under_its_record_id() {
        let db = Database::from_connection(Connection::open_in_memory().unwrap()).unwrap();
        let conn = db.get_connection();
        let admin = SqliteUserRepository::new(conn).create_user(CreateUserRequest {
            username: "admin".into(), password: "password123!".into(), role: UserRole::Administrator,
        }).unwrap().id;
        let (key, _) = SqliteKeyStoreRepository::with_params(conn, KdfParams::fast_for_tests())
            .create_data_key(admin, "admin-password").unwrap();
        conn.execute("INSERT INTO assets (id, name, created_by) VALUES (1, 'plc', ?1)", [admin]).unwrap();

        let firmware_dir = TempDir::new().unwrap();
        let bundles = TempDir::new().unwrap();
        let (configs, firmware, vaults, audit) = (
            SqliteConfigurationRepository::new(conn),
            SqliteFirmwareRepository::new(conn),
            SqliteVaultRepository::new(conn),
            SqliteAuditRepository::new(conn),
        );
        let importer = RecoveryImporter::new(&configs, &firmware, &vaults, &audit);
        for (name, image) in [("first", b"first image".as_slice()), ("second", b"second image".as_slice())] {
            let request = RecoveryImportRequest {
                bundle_path: write_bundle(bundles.path(), name, image), target_asset_id: 1, import_vault: false,
            };
            importer.import_recovery_bundle_in(
                firmware_dir.path(), request, admin, "admin", &UserRole::Administrator, &key, None,
            ).unwrap();
        }

        let rows: Vec<(i64, String, String, Option<String>)> = conn
            .prepare("SELECT id, version, file_path, original_filename FROM firmware_versions ORDER BY id").unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap()
            .collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(rows.len(), 2);
        let mut expected_files = Vec::new();
        for (id, version, path, original) in &rows {
            assert_eq!(Path::new(path), Path::new("1").join(format!("{}.enc", id)));
            // The bundle's original name is kept, sanitized, as a label only
            assert_eq!(original.as_deref(), Some(format!("{}.sedp", version).as_str()));
            let content = FirmwareFileStorage::read_firmware_file_in(firmware_dir.path(), path, &key).unwrap();
            assert_eq!(content, format!("{} image", version).into_bytes());
            expected_files.push(format!("{}.enc", id));
        }
        let mut files: Vec<String> = std::fs::read_dir(firmware_dir.path().join("1")).unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        files.sort();
        expected_files.sort();
        assert_eq!(files, expected_files);
    }

    /// A vault imported from a recovery bundle keeps its secrets' usernames.
    #[test]
    fn test_imported_vault_keeps_secret_usernames() {
        use crate::encryption::FileEncryption;
        use crate::vault::{IdentityVault, SecretType, VaultInfo, VaultSecret};

        let db = Database::from_connection(Connection::open_in_memory().unwrap()).unwrap();
        let conn = db.get_connection();
        let admin = SqliteUserRepository::new(conn).create_user(CreateUserRequest {
            username: "admin".into(), password: "password123!".into(), role: UserRole::Administrator,
        }).unwrap().id;
        let (key, _) = SqliteKeyStoreRepository::with_params(conn, KdfParams::fast_for_tests())
            .create_data_key(admin, "admin-password").unwrap();
        conn.execute("INSERT INTO assets (id, name, created_by) VALUES (1, 'plc', ?1)", [admin]).unwrap();

        // A vault exported from another install
        let source_key = DataKey::generate();
        let exported = VaultInfo {
            vault: IdentityVault {
                id: 9, asset_id: 1, name: "PLC vault".into(), description: String::new(), created_by: 1,
                created_at: String::new(), updated_at: String::new(),
            },
            secrets: vec![VaultSecret {
                id: 4, vault_id: 9, secret_type: SecretType::Password, label: "HMI login".into(),
                username: Some("DOMAIN\\Operator".into()),
                encrypted_value: FileEncryption::new(&source_key).encrypt_to_base64(b"Pl4nt-Admin!").unwrap(),
                created_at: String::new(), updated_at: String::new(),
                strength_score: None, last_changed: None, generation_method: None, policy_version: None,
                last_rotated: None, rotation_interval_days: None, next_rotation_due: None, rotation_policy_id: None,
                key_version: 1,
            }],
            secret_count: 1,
        };
        let vault_json = seal_vault_export(&exported, &source_key, "substation-7 recovery").unwrap();

        let firmware_dir = TempDir::new().unwrap();
        let bundles = TempDir::new().unwrap();
        let (configs, firmware, vaults, audit) = (
            SqliteConfigurationRepository::new(conn),
            SqliteFirmwareRepository::new(conn),
            SqliteVaultRepository::new(conn),
            SqliteAuditRepository::new(conn),
        );
        let request = RecoveryImportRequest {
            bundle_path: write_bundle_with_vault(bundles.path(), "with-vault", b"image", Some(&vault_json)),
            target_asset_id: 1,
            import_vault: true,
        };
        RecoveryImporter::new(&configs, &firmware, &vaults, &audit).import_recovery_bundle_in(
            firmware_dir.path(), request, admin, "admin", &UserRole::Administrator, &key, Some("substation-7 recovery"),
        ).unwrap();

        let (username, encrypted_value): (Option<String>, String) = conn.query_row(
            "SELECT username, encrypted_value FROM vault_secrets", [], |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(username.as_deref(), Some("DOMAIN\\Operator"));
        assert_eq!(FileEncryption::new(&key).decrypt_from_base64(&encrypted_value).unwrap(), b"Pl4nt-Admin!");
    }
}
