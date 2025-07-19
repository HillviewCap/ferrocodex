use super::*;

#[test]
fn test_configuration_status_string_conversion() {
    assert_eq!(ConfigurationStatus::Draft.as_str(), "Draft");
    assert_eq!(ConfigurationStatus::Approved.as_str(), "Approved");
    assert_eq!(ConfigurationStatus::Golden.as_str(), "Golden");
    assert_eq!(ConfigurationStatus::Archived.as_str(), "Archived");

    assert_eq!(ConfigurationStatus::from_str("Draft"), Some(ConfigurationStatus::Draft));
    assert_eq!(ConfigurationStatus::from_str("Approved"), Some(ConfigurationStatus::Approved));
    assert_eq!(ConfigurationStatus::from_str("Golden"), Some(ConfigurationStatus::Golden));
    assert_eq!(ConfigurationStatus::from_str("Archived"), Some(ConfigurationStatus::Archived));
    assert_eq!(ConfigurationStatus::from_str("Invalid"), None);
}

#[test]
fn test_configuration_status_equality() {
    assert_eq!(ConfigurationStatus::Draft, ConfigurationStatus::Draft);
    assert_eq!(ConfigurationStatus::Approved, ConfigurationStatus::Approved);
    assert_eq!(ConfigurationStatus::Golden, ConfigurationStatus::Golden);
    assert_eq!(ConfigurationStatus::Archived, ConfigurationStatus::Archived);
    
    assert_ne!(ConfigurationStatus::Draft, ConfigurationStatus::Approved);
    assert_ne!(ConfigurationStatus::Approved, ConfigurationStatus::Golden);
    assert_ne!(ConfigurationStatus::Golden, ConfigurationStatus::Archived);
}

#[test]
fn test_configuration_status_serialization() {
    // Test that the enum can be serialized and deserialized
    let status = ConfigurationStatus::Approved;
    let serialized = serde_json::to_string(&status).unwrap();
    let deserialized: ConfigurationStatus = serde_json::from_str(&serialized).unwrap();
    assert_eq!(status, deserialized);
}

#[test]
fn test_status_change_record_structure() {
    let record = StatusChangeRecord {
        id: 1,
        version_id: 100,
        old_status: Some("Draft".to_string()),
        new_status: "Approved".to_string(),
        changed_by: 2,
        changed_by_username: "admin".to_string(),
        change_reason: Some("Ready for production".to_string()),
        created_at: "2023-01-01T12:00:00Z".to_string(),
    };

    assert_eq!(record.id, 1);
    assert_eq!(record.version_id, 100);
    assert_eq!(record.old_status, Some("Draft".to_string()));
    assert_eq!(record.new_status, "Approved".to_string());
    assert_eq!(record.changed_by, 2);
    assert_eq!(record.changed_by_username, "admin");
    assert_eq!(record.change_reason, Some("Ready for production".to_string()));
}

#[test]
fn test_configuration_version_info_structure() {
    let version_info = ConfigurationVersionInfo {
        id: 1,
        asset_id: 10,
        version_number: "v1.0".to_string(),
        file_name: "config.json".to_string(),
        file_size: 1024,
        content_hash: "abc123".to_string(),
        author: 5,
        author_username: "user".to_string(),
        notes: "Initial version".to_string(),
        status: "Draft".to_string(),
        status_changed_by: Some(5),
        status_changed_at: Some("2023-01-01T12:00:00Z".to_string()),
        created_at: "2023-01-01T12:00:00Z".to_string(),
    };

    assert_eq!(version_info.status, "Draft");
    assert_eq!(version_info.status_changed_by, Some(5));
    assert_eq!(version_info.status_changed_at, Some("2023-01-01T12:00:00Z".to_string()));
}

#[test]
fn test_status_change_without_reason() {
    let record = StatusChangeRecord {
        id: 1,
        version_id: 100,
        old_status: None,
        new_status: "Draft".to_string(),
        changed_by: 1,
        changed_by_username: "creator".to_string(),
        change_reason: None,
        created_at: "2023-01-01T12:00:00Z".to_string(),
    };

    assert_eq!(record.old_status, None);
    assert_eq!(record.change_reason, None);
    assert_eq!(record.new_status, "Draft");
}

#[test]
fn test_status_enum_to_string_integration() {
    // Test that our enum can be properly converted to strings for database storage
    let statuses = vec![
        ConfigurationStatus::Draft,
        ConfigurationStatus::Approved,
        ConfigurationStatus::Golden,
        ConfigurationStatus::Archived,
    ];

    for status in statuses {
        let status_str = status.as_str();
        let parsed_back = ConfigurationStatus::from_str(status_str);
        assert_eq!(parsed_back, Some(status));
    }
}

#[test]
fn test_version_info_with_different_statuses() {
    let statuses = vec!["Draft", "Approved", "Golden", "Archived"];
    
    for status in statuses {
        let version_info = ConfigurationVersionInfo {
            id: 1,
            asset_id: 10,
            version_number: "v1.0".to_string(),
            file_name: "config.json".to_string(),
            file_size: 1024,
            content_hash: "abc123".to_string(),
            author: 5,
            author_username: "user".to_string(),
            notes: "Test version".to_string(),
            status: status.to_string(),
            status_changed_by: Some(5),
            status_changed_at: Some("2023-01-01T12:00:00Z".to_string()),
            created_at: "2023-01-01T12:00:00Z".to_string(),
        };

        assert_eq!(version_info.status, status);
    }
}