use serde_json::json;
use super::{MetadataFieldTemplate, FieldType, FieldCategory};

/// Get all system field templates for industrial assets
pub fn get_system_field_templates() -> Vec<MetadataFieldTemplate> {
    vec![
        // Network-related templates
        create_ip_address_template(),
        create_mac_address_template(),
        create_subnet_mask_template(),
        create_default_gateway_template(),
        create_dns_server_template(),
        create_network_port_template(),
        create_vlan_id_template(),
        create_protocol_template(),
        
        // Physical location templates
        create_facility_template(),
        create_building_template(),
        create_floor_template(),
        create_room_template(),
        create_rack_template(),
        create_panel_template(),
        create_coordinates_template(),
        
        // Device specification templates
        create_manufacturer_template(),
        create_model_number_template(),
        create_serial_number_template(),
        create_firmware_version_template(),
        create_hardware_revision_template(),
        create_part_number_template(),
        
        // Operational templates
        create_install_date_template(),
        create_warranty_expiration_template(),
        create_maintenance_schedule_template(),
        create_operational_status_template(),
        create_commissioning_date_template(),
        create_notes_template(),
        
        // Security templates
        create_security_classification_template(),
        create_access_level_template(),
        create_compliance_tags_template(),
        create_certificate_expiry_template(),
    ]
}

// Network-related templates

fn create_ip_address_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "IP Address".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "pattern": "^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)(?:\\/(?:3[0-2]|[12]?[0-9]))?$",
            "title": "IPv4 Address",
            "description": "IPv4 address with optional CIDR notation (e.g., 192.168.1.100 or 192.168.1.0/24)"
        }).to_string(),
        None,
        FieldCategory::Network,
        "IPv4 address field with CIDR notation support for network configuration".to_string(),
        true,
    )
}

fn create_mac_address_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "MAC Address".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "pattern": "^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$",
            "title": "MAC Address",
            "description": "Hardware MAC address in standard format (e.g., 00:1B:44:11:3A:B7)"
        }).to_string(),
        None,
        FieldCategory::Network,
        "Media Access Control (MAC) address for network interface identification".to_string(),
        true,
    )
}

fn create_subnet_mask_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Subnet Mask".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "pattern": "^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$",
            "title": "Subnet Mask",
            "description": "Network subnet mask (e.g., 255.255.255.0)"
        }).to_string(),
        None,
        FieldCategory::Network,
        "Network subnet mask for IP configuration".to_string(),
        true,
    )
}

fn create_default_gateway_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Default Gateway".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "pattern": "^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$",
            "title": "Default Gateway",
            "description": "Default gateway IP address (e.g., 192.168.1.1)"
        }).to_string(),
        None,
        FieldCategory::Network,
        "Default gateway IP address for network routing".to_string(),
        true,
    )
}

fn create_dns_server_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "DNS Server".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "pattern": "^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$",
            "title": "DNS Server",
            "description": "Domain Name System server IP address"
        }).to_string(),
        None,
        FieldCategory::Network,
        "DNS server IP address for name resolution".to_string(),
        true,
    )
}

fn create_network_port_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Network Port".to_string(),
        FieldType::Number,
        json!({
            "type": "integer",
            "minimum": 1,
            "maximum": 65535,
            "title": "Network Port",
            "description": "TCP/UDP port number (1-65535)"
        }).to_string(),
        None,
        FieldCategory::Network,
        "Network port number for service configuration".to_string(),
        true,
    )
}

fn create_vlan_id_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "VLAN ID".to_string(),
        FieldType::Number,
        json!({
            "type": "integer",
            "minimum": 1,
            "maximum": 4094,
            "title": "VLAN ID",
            "description": "Virtual LAN identifier (1-4094)"
        }).to_string(),
        None,
        FieldCategory::Network,
        "Virtual LAN identifier for network segmentation".to_string(),
        true,
    )
}

fn create_protocol_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Protocol".to_string(),
        FieldType::Dropdown,
        json!({
            "type": "string",
            "enum": [
                "Modbus TCP",
                "Modbus RTU",
                "EtherNet/IP",
                "PROFINET",
                "PROFIBUS",
                "DeviceNet",
                "CAN Bus",
                "OPC UA",
                "OPC DA",
                "DNP3",
                "IEC 61850",
                "BACnet",
                "HART",
                "Foundation Fieldbus",
                "AS-Interface",
                "Ethernet",
                "Serial RS232",
                "Serial RS485",
                "Wireless 802.11",
                "LoRaWAN",
                "Other"
            ],
            "title": "Communication Protocol",
            "description": "Industrial communication protocol used by the device"
        }).to_string(),
        Some(json!({
            "allowCustom": true,
            "placeholder": "Select or enter protocol"
        }).to_string()),
        FieldCategory::Network,
        "Communication protocol for industrial device connectivity".to_string(),
        true,
    )
}

// Physical location templates

fn create_facility_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Facility".to_string(),
        FieldType::Dropdown,
        json!({
            "type": "string",
            "title": "Facility",
            "description": "Production facility or plant location"
        }).to_string(),
        Some(json!({
            "allowCustom": true,
            "placeholder": "Select or enter facility name",
            "suggestions": [
                "Main Plant",
                "Plant A",
                "Plant B",
                "Assembly Line 1",
                "Assembly Line 2",
                "Warehouse",
                "Quality Control Lab",
                "Maintenance Shop",
                "Control Room",
                "Substation"
            ]
        }).to_string()),
        FieldCategory::Physical,
        "Facility or plant where the asset is located".to_string(),
        true,
    )
}

fn create_building_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Building".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "maxLength": 100,
            "title": "Building",
            "description": "Building name or identifier"
        }).to_string(),
        None,
        FieldCategory::Physical,
        "Building name or identifier for asset location".to_string(),
        true,
    )
}

fn create_floor_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Floor".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "maxLength": 20,
            "title": "Floor",
            "description": "Floor level (e.g., Ground, 1st, 2nd, Basement)"
        }).to_string(),
        None,
        FieldCategory::Physical,
        "Floor level where the asset is installed".to_string(),
        true,
    )
}

fn create_room_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Room/Area".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "maxLength": 100,
            "title": "Room or Area",
            "description": "Specific room, area, or zone identifier"
        }).to_string(),
        None,
        FieldCategory::Physical,
        "Specific room, area, or zone where the asset is located".to_string(),
        true,
    )
}

fn create_rack_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Rack/Cabinet".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "maxLength": 50,
            "title": "Rack or Cabinet",
            "description": "Rack, cabinet, or enclosure identifier"
        }).to_string(),
        None,
        FieldCategory::Physical,
        "Rack, cabinet, or enclosure housing the asset".to_string(),
        true,
    )
}

fn create_panel_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Panel Position".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "maxLength": 50,
            "title": "Panel Position",
            "description": "Position within panel or rack (e.g., Slot 1, U42, Left Side)"
        }).to_string(),
        None,
        FieldCategory::Physical,
        "Specific position within a panel, rack, or cabinet".to_string(),
        true,
    )
}

fn create_coordinates_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "GPS Coordinates".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "pattern": "^-?\\d+\\.\\d+,\\s*-?\\d+\\.\\d+$",
            "title": "GPS Coordinates",
            "description": "GPS coordinates in decimal degrees format (latitude, longitude)"
        }).to_string(),
        None,
        FieldCategory::Physical,
        "GPS coordinates for outdoor or precisely located assets".to_string(),
        true,
    )
}

// Device specification templates

fn create_manufacturer_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Manufacturer".to_string(),
        FieldType::Dropdown,
        json!({
            "type": "string",
            "title": "Manufacturer",
            "description": "Device manufacturer or vendor"
        }).to_string(),
        Some(json!({
            "allowCustom": true,
            "placeholder": "Select or enter manufacturer",
            "suggestions": [
                "Siemens",
                "Allen-Bradley (Rockwell)",
                "Schneider Electric",
                "ABB",
                "Mitsubishi Electric",
                "Omron",
                "General Electric",
                "Honeywell",
                "Emerson",
                "Yokogawa",
                "Endress+Hauser",
                "Phoenix Contact",
                "Beckhoff",
                "B&R Automation",
                "WAGO",
                "Pilz",
                "Pepperl+Fuchs",
                "Turck",
                "Balluff",
                "IFM Electronic",
                "Other"
            ]
        }).to_string()),
        FieldCategory::Device,
        "Equipment manufacturer or vendor company".to_string(),
        true,
    )
}

fn create_model_number_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Model Number".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "maxLength": 100,
            "pattern": "^[A-Za-z0-9\\-_\\./\\s]+$",
            "title": "Model Number",
            "description": "Manufacturer's model or part number"
        }).to_string(),
        None,
        FieldCategory::Device,
        "Manufacturer's model number or type designation".to_string(),
        true,
    )
}

fn create_serial_number_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Serial Number".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "maxLength": 100,
            "pattern": "^[A-Za-z0-9\\-_]+$",
            "title": "Serial Number",
            "description": "Unique device serial number"
        }).to_string(),
        None,
        FieldCategory::Device,
        "Unique serial number for device identification".to_string(),
        true,
    )
}

fn create_firmware_version_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Firmware Version".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "pattern": "^v?\\d+(?:\\.\\d+)*(?:[\\-+][A-Za-z0-9\\-+\\.]+)?$",
            "title": "Firmware Version",
            "description": "Firmware version in semantic versioning format (e.g., v1.2.3, 2.1.0-beta)"
        }).to_string(),
        None,
        FieldCategory::Device,
        "Current firmware version installed on the device".to_string(),
        true,
    )
}

fn create_hardware_revision_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Hardware Revision".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "maxLength": 50,
            "pattern": "^[A-Za-z0-9\\-_\\.]+$",
            "title": "Hardware Revision",
            "description": "Hardware revision or version (e.g., Rev A, v2.1, HW-001)"
        }).to_string(),
        None,
        FieldCategory::Device,
        "Hardware revision or version identifier".to_string(),
        true,
    )
}

fn create_part_number_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Part Number".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "maxLength": 100,
            "pattern": "^[A-Za-z0-9\\-_\\./\\s]+$",
            "title": "Part Number",
            "description": "Manufacturer's part number or catalog number"
        }).to_string(),
        None,
        FieldCategory::Device,
        "Manufacturer's part number or catalog identifier".to_string(),
        true,
    )
}

// Operational templates

fn create_install_date_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Install Date".to_string(),
        FieldType::Date,
        json!({
            "type": "string",
            "format": "date",
            "title": "Installation Date",
            "description": "Date when the device was installed and commissioned"
        }).to_string(),
        None,
        FieldCategory::Operational,
        "Date when the asset was installed and put into service".to_string(),
        true,
    )
}

fn create_warranty_expiration_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Warranty Expiration".to_string(),
        FieldType::Date,
        json!({
            "type": "string",
            "format": "date",
            "title": "Warranty Expiration Date",
            "description": "Date when manufacturer warranty expires"
        }).to_string(),
        None,
        FieldCategory::Operational,
        "Warranty expiration date for maintenance planning".to_string(),
        true,
    )
}

fn create_maintenance_schedule_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Maintenance Schedule".to_string(),
        FieldType::Dropdown,
        json!({
            "type": "string",
            "enum": [
                "Weekly",
                "Monthly",
                "Quarterly",
                "Semi-Annual",
                "Annual",
                "Bi-Annual",
                "As Needed",
                "Condition Based",
                "Never"
            ],
            "title": "Maintenance Schedule",
            "description": "Scheduled maintenance frequency"
        }).to_string(),
        None,
        FieldCategory::Operational,
        "Scheduled maintenance frequency for the asset".to_string(),
        true,
    )
}

fn create_operational_status_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Operational Status".to_string(),
        FieldType::Dropdown,
        json!({
            "type": "string",
            "enum": [
                "Active",
                "Standby",
                "Maintenance",
                "Offline",
                "Decommissioned",
                "Testing",
                "Commissioning",
                "Failed",
                "Unknown"
            ],
            "title": "Operational Status",
            "description": "Current operational status of the asset"
        }).to_string(),
        None,
        FieldCategory::Operational,
        "Current operational status and availability".to_string(),
        true,
    )
}

fn create_commissioning_date_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Commissioning Date".to_string(),
        FieldType::Date,
        json!({
            "type": "string",
            "format": "date",
            "title": "Commissioning Date",
            "description": "Date when the device was commissioned and tested"
        }).to_string(),
        None,
        FieldCategory::Operational,
        "Date when the asset was commissioned and became operational".to_string(),
        true,
    )
}

fn create_notes_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Notes".to_string(),
        FieldType::Textarea,
        json!({
            "type": "string",
            "maxLength": 2000,
            "title": "Notes",
            "description": "Additional notes, comments, or observations"
        }).to_string(),
        None,
        FieldCategory::Operational,
        "General notes, comments, or additional information about the asset".to_string(),
        true,
    )
}

// Security templates

fn create_security_classification_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Security Classification".to_string(),
        FieldType::Dropdown,
        json!({
            "type": "string",
            "enum": [
                "Public",
                "Internal",
                "Confidential",
                "Restricted",
                "Critical Infrastructure",
                "Safety Critical",
                "Mission Critical"
            ],
            "title": "Security Classification",
            "description": "Security classification level for the asset"
        }).to_string(),
        None,
        FieldCategory::Security,
        "Security classification level determining access requirements".to_string(),
        true,
    )
}

fn create_access_level_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Access Level".to_string(),
        FieldType::Dropdown,
        json!({
            "type": "string",
            "enum": [
                "Public",
                "Operator",
                "Technician",
                "Engineer",
                "Supervisor",
                "Manager",
                "Administrator",
                "Security Officer"
            ],
            "title": "Required Access Level",
            "description": "Minimum access level required to interact with this asset"
        }).to_string(),
        None,
        FieldCategory::Security,
        "Minimum access level required for asset interaction".to_string(),
        true,
    )
}

fn create_compliance_tags_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Compliance Tags".to_string(),
        FieldType::Text,
        json!({
            "type": "string",
            "maxLength": 200,
            "title": "Compliance Tags",
            "description": "Compliance standards, certifications, or regulations (comma-separated)"
        }).to_string(),
        Some(json!({
            "placeholder": "e.g., NIST, IEC 62443, ISA-99, NERC CIP",
            "suggestions": [
                "NIST Cybersecurity Framework",
                "IEC 62443",
                "ISA-99",
                "NERC CIP",
                "ISO 27001",
                "SOX Compliance",
                "GDPR",
                "HIPAA",
                "FDA 21 CFR Part 11"
            ]
        }).to_string()),
        FieldCategory::Security,
        "Relevant compliance standards and certifications".to_string(),
        true,
    )
}

fn create_certificate_expiry_template() -> MetadataFieldTemplate {
    MetadataFieldTemplate::new(
        "Certificate Expiry".to_string(),
        FieldType::Date,
        json!({
            "type": "string",
            "format": "date",
            "title": "Certificate Expiry Date",
            "description": "Expiry date of security certificates or credentials"
        }).to_string(),
        None,
        FieldCategory::Security,
        "Expiry date for security certificates and credentials".to_string(),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_get_system_field_templates() {
        let templates = get_system_field_templates();
        
        // Should have multiple categories
        assert!(templates.len() > 20);
        
        // All should be system templates
        for template in &templates {
            assert!(template.is_system);
        }
        
        // Should have templates from all categories
        let categories: std::collections::HashSet<_> = templates.iter()
            .map(|t| &t.category)
            .collect();
        
        assert!(categories.contains(&FieldCategory::Network));
        assert!(categories.contains(&FieldCategory::Physical));
        assert!(categories.contains(&FieldCategory::Device));
        assert!(categories.contains(&FieldCategory::Operational));
        assert!(categories.contains(&FieldCategory::Security));
    }

    #[test]
    fn test_ip_address_template() {
        let template = create_ip_address_template();
        
        assert_eq!(template.name, "IP Address");
        assert_eq!(template.field_type, FieldType::Text);
        assert_eq!(template.category, FieldCategory::Network);
        assert!(template.is_system);
        
        // Validate JSON structure
        let validation_rules: Value = serde_json::from_str(&template.validation_rules).unwrap();
        assert_eq!(validation_rules["type"], "string");
        assert!(validation_rules["pattern"].is_string());
    }

    #[test]
    fn test_manufacturer_template() {
        let template = create_manufacturer_template();
        
        assert_eq!(template.name, "Manufacturer");
        assert_eq!(template.field_type, FieldType::Dropdown);
        assert_eq!(template.category, FieldCategory::Device);
        
        // Should have options JSON
        assert!(template.options_json.is_some());
        
        let options: Value = serde_json::from_str(template.options_json.as_ref().unwrap()).unwrap();
        assert_eq!(options["allowCustom"], true);
        assert!(options["suggestions"].is_array());
        
        // Should include major industrial manufacturers
        let suggestions = options["suggestions"].as_array().unwrap();
        let suggestion_strings: Vec<&str> = suggestions.iter()
            .filter_map(|s| s.as_str())
            .collect();
        
        assert!(suggestion_strings.contains(&"Siemens"));
        assert!(suggestion_strings.contains(&"Allen-Bradley (Rockwell)"));
        assert!(suggestion_strings.contains(&"Schneider Electric"));
    }

    #[test]
    fn test_protocol_template() {
        let template = create_protocol_template();
        
        assert_eq!(template.name, "Protocol");
        assert_eq!(template.field_type, FieldType::Dropdown);
        assert_eq!(template.category, FieldCategory::Network);
        
        let validation_rules: Value = serde_json::from_str(&template.validation_rules).unwrap();
        let protocols = validation_rules["enum"].as_array().unwrap();
        
        // Should include common industrial protocols
        let protocol_strings: Vec<&str> = protocols.iter()
            .filter_map(|p| p.as_str())
            .collect();
        
        assert!(protocol_strings.contains(&"Modbus TCP"));
        assert!(protocol_strings.contains(&"EtherNet/IP"));
        assert!(protocol_strings.contains(&"PROFINET"));
        assert!(protocol_strings.contains(&"OPC UA"));
    }

    #[test]
    fn test_operational_status_template() {
        let template = create_operational_status_template();
        
        assert_eq!(template.name, "Operational Status");
        assert_eq!(template.field_type, FieldType::Dropdown);
        assert_eq!(template.category, FieldCategory::Operational);
        
        let validation_rules: Value = serde_json::from_str(&template.validation_rules).unwrap();
        let statuses = validation_rules["enum"].as_array().unwrap();
        
        let status_strings: Vec<&str> = statuses.iter()
            .filter_map(|s| s.as_str())
            .collect();
        
        assert!(status_strings.contains(&"Active"));
        assert!(status_strings.contains(&"Standby"));
        assert!(status_strings.contains(&"Maintenance"));
        assert!(status_strings.contains(&"Offline"));
    }

    #[test]
    fn test_template_validation_rules_are_valid_json() {
        let templates = get_system_field_templates();
        
        for template in templates {
            // All validation rules should be valid JSON
            let _: Value = serde_json::from_str(&template.validation_rules)
                .unwrap_or_else(|_| panic!("Invalid JSON in template '{}': {}", template.name, template.validation_rules));
            
            // If options_json is present, it should be valid JSON
            if let Some(options) = template.options_json {
                let _: Value = serde_json::from_str(&options)
                    .unwrap_or_else(|_| panic!("Invalid options JSON in template '{}': {}", template.name, options));
            }
        }
    }
}