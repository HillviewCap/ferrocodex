use super::*;
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

        // Test invalid user role - this should be caught by the validation
        let request = RecoveryExportRequest {
            asset_id: 1,
            config_version_id: 1,
            firmware_version_id: 1,
            export_directory: temp_dir.path().to_str().unwrap().to_string(),
        };

        // Test with Viewer role (should fail)
        let result = exporter.export_complete_recovery(
            &app,
            request.clone(),
            1,
            "testuser",
            &UserRole::Viewer,
            "Test Asset",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Only Engineers and Administrators"));
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
            "Test Asset",
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
            "Test Asset",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Export directory cannot be empty"));
    }