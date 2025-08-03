use anyhow::Result;
use jsonschema::{JSONSchema, Draft, ValidationError as JsonSchemaValidationError};
use serde_json::{Value, json};
use std::collections::HashMap;
use regex::Regex;
use tracing::{debug, warn};

use super::{ValidationResult, ValidationError, AssetMetadataSchema};

/// Schema validation service with caching and custom validators
pub struct SchemaValidator {
    compiled_schemas: HashMap<String, JSONSchema>,
    custom_validators: HashMap<String, Box<dyn Fn(&Value) -> Result<(), String> + Send + Sync>>,
}

impl SchemaValidator {
    pub fn new() -> Self {
        let mut validator = Self {
            compiled_schemas: HashMap::new(),
            custom_validators: HashMap::new(),
        };

        // Register custom validators for industrial use cases
        validator.register_custom_validators();
        validator
    }

    /// Validate a JSON Schema definition
    pub fn validate_schema_definition(&mut self, schema_json: &str) -> ValidationResult {
        let schema_value: Value = match serde_json::from_str(schema_json) {
            Ok(value) => value,
            Err(e) => {
                return ValidationResult::with_errors(vec![ValidationError::new(
                    "root".to_string(),
                    "json_parse_error".to_string(),
                    format!("Invalid JSON: {}", e),
                    None,
                    None,
                )]);
            }
        };

        // Try to compile the schema
        match self.compile_schema(&schema_value) {
            Ok(_) => ValidationResult::success(),
            Err(e) => ValidationResult::with_errors(vec![ValidationError::new(
                "root".to_string(),
                "schema_compilation_error".to_string(),
                format!("Invalid JSON Schema: {}", e),
                None,
                None,
            )]),
        }
    }

    /// Validate metadata values against a schema
    pub fn validate_metadata_values(&mut self, schema: &AssetMetadataSchema, values_json: &str) -> ValidationResult {
        let values: Value = match serde_json::from_str(values_json) {
            Ok(value) => value,
            Err(e) => {
                return ValidationResult::with_errors(vec![ValidationError::new(
                    "root".to_string(),
                    "json_parse_error".to_string(),
                    format!("Invalid JSON in values: {}", e),
                    None,
                    None,
                )]);
            }
        };

        let schema_value: Value = match serde_json::from_str(&schema.schema_json) {
            Ok(value) => value,
            Err(e) => {
                return ValidationResult::with_errors(vec![ValidationError::new(
                    "schema".to_string(),
                    "json_parse_error".to_string(),
                    format!("Invalid JSON in schema: {}", e),
                    None,
                    None,
                )]);
            }
        };

        let mut all_errors = Vec::new();

        // Phase 1: JSON Schema validation (requires mutable borrow)
        let validation_errors = {
            let compiled_schema = match self.get_or_compile_schema(&schema.schema_json) {
                Ok(schema) => schema,
                Err(e) => {
                    return ValidationResult::with_errors(vec![ValidationError::new(
                        "schema".to_string(),
                        "schema_compilation_error".to_string(),
                        format!("Failed to compile schema: {}", e),
                        None,
                        None,
                    )]);
                }
            };

            // Collect errors into a vector to avoid iterator lifetime issues
            match compiled_schema.validate(&values) {
                Ok(_) => Vec::new(),
                Err(errors) => errors.collect::<Vec<_>>(),
            }
        }; // Mutable borrow ends here

        // Convert validation errors now that we don't have any borrows
        for error in validation_errors {
            all_errors.push(ValidationError::new(
                "field".to_string(),
                "validation_error".to_string(),
                error.to_string(),
                None,
                None,
            ));
        }

        // Phase 2: Custom validators (requires immutable borrow)
        if let Err(custom_errors) = self.apply_custom_validators(&values, &schema_value) {
            all_errors.extend(custom_errors);
        }

        if all_errors.is_empty() {
            ValidationResult::success()
        } else {
            ValidationResult::with_errors(all_errors)
        }
    }

    /// Get or compile a schema with caching
    fn get_or_compile_schema(&mut self, schema_json: &str) -> Result<&JSONSchema> {
        // Use schema JSON as cache key (in production, consider using a hash)
        let cache_key = schema_json.to_string();
        
        if !self.compiled_schemas.contains_key(&cache_key) {
            let schema_value: Value = serde_json::from_str(schema_json)?;
            let compiled = self.compile_schema(&schema_value)?;
            self.compiled_schemas.insert(cache_key.clone(), compiled);
        }

        Ok(self.compiled_schemas.get(&cache_key).unwrap())
    }

    /// Compile a JSON Schema
    fn compile_schema(&self, schema_value: &Value) -> Result<JSONSchema> {
        JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(schema_value)
            .map_err(|e| anyhow::anyhow!("Schema compilation failed: {}", e))
    }

    /// Convert jsonschema validation error to our format
    #[allow(dead_code)]
    fn convert_validation_error(&self, error: JsonSchemaValidationError) -> ValidationError {
        let field_path = error.instance_path.to_string();
        let field_path = if field_path.is_empty() {
            "root".to_string()
        } else {
            field_path
        };

        ValidationError::new(
            field_path,
            format!("{:?}", error.kind), // Use debug format for now
            error.to_string(),
            None, // Could extract expected value from error if needed
            Some(Value::String("placeholder".to_string())), // Placeholder for now
        )
    }

    /// Register custom validators for industrial use cases
    fn register_custom_validators(&mut self) {
        // IPv4 address validator
        self.custom_validators.insert(
            "ipv4".to_string(),
            Box::new(|value| {
                if let Some(ip_str) = value.as_str() {
                    let ip_regex = Regex::new(r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$")
                        .map_err(|_| "Failed to compile IP regex")?;
                    
                    if ip_regex.is_match(ip_str) {
                        Ok(())
                    } else {
                        Err("Invalid IPv4 address format".to_string())
                    }
                } else {
                    Err("Value must be a string for IP validation".to_string())
                }
            }),
        );

        // IPv4 with CIDR notation validator
        self.custom_validators.insert(
            "ipv4_cidr".to_string(),
            Box::new(|value| {
                if let Some(ip_str) = value.as_str() {
                    let cidr_regex = Regex::new(r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)(?:/(?:3[0-2]|[12]?[0-9]))?$")
                        .map_err(|_| "Failed to compile CIDR regex")?;
                    
                    if cidr_regex.is_match(ip_str) {
                        Ok(())
                    } else {
                        Err("Invalid IPv4 address with CIDR notation".to_string())
                    }
                } else {
                    Err("Value must be a string for IP CIDR validation".to_string())
                }
            }),
        );

        // MAC address validator
        self.custom_validators.insert(
            "mac_address".to_string(),
            Box::new(|value| {
                if let Some(mac_str) = value.as_str() {
                    let mac_regex = Regex::new(r"^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$")
                        .map_err(|_| "Failed to compile MAC regex")?;
                    
                    if mac_regex.is_match(mac_str) {
                        Ok(())
                    } else {
                        Err("Invalid MAC address format (expected XX:XX:XX:XX:XX:XX or XX-XX-XX-XX-XX-XX)".to_string())
                    }
                } else {
                    Err("Value must be a string for MAC address validation".to_string())
                }
            }),
        );

        // Semantic version validator
        self.custom_validators.insert(
            "semver".to_string(),
            Box::new(|value| {
                if let Some(version_str) = value.as_str() {
                    let semver_regex = Regex::new(r"^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z\-\.]+))?(?:\+([0-9A-Za-z\-\.]+))?$")
                        .map_err(|_| "Failed to compile semver regex")?;
                    
                    if semver_regex.is_match(version_str) {
                        Ok(())
                    } else {
                        Err("Invalid semantic version format (expected x.y.z)".to_string())
                    }
                } else {
                    Err("Value must be a string for semantic version validation".to_string())
                }
            }),
        );

        // Industrial equipment ID validator (alphanumeric with hyphens/underscores)
        self.custom_validators.insert(
            "equipment_id".to_string(),
            Box::new(|value| {
                if let Some(id_str) = value.as_str() {
                    let id_regex = Regex::new(r"^[A-Za-z0-9][A-Za-z0-9\-_]*[A-Za-z0-9]$|^[A-Za-z0-9]$")
                        .map_err(|_| "Failed to compile equipment ID regex")?;
                    
                    if id_str.len() >= 1 && id_str.len() <= 50 && id_regex.is_match(id_str) {
                        Ok(())
                    } else {
                        Err("Invalid equipment ID format (1-50 alphanumeric characters, hyphens, underscores)".to_string())
                    }
                } else {
                    Err("Value must be a string for equipment ID validation".to_string())
                }
            }),
        );

        // Network port range validator
        self.custom_validators.insert(
            "network_port".to_string(),
            Box::new(|value| {
                match value {
                    Value::Number(n) => {
                        if let Some(port) = n.as_u64() {
                            if port >= 1 && port <= 65535 {
                                Ok(())
                            } else {
                                Err("Port number must be between 1 and 65535".to_string())
                            }
                        } else {
                            Err("Port must be a positive integer".to_string())
                        }
                    }
                    _ => Err("Port value must be a number".to_string()),
                }
            }),
        );

        // VLAN ID validator
        self.custom_validators.insert(
            "vlan_id".to_string(),
            Box::new(|value| {
                match value {
                    Value::Number(n) => {
                        if let Some(vlan) = n.as_u64() {
                            if vlan >= 1 && vlan <= 4094 {
                                Ok(())
                            } else {
                                Err("VLAN ID must be between 1 and 4094".to_string())
                            }
                        } else {
                            Err("VLAN ID must be a positive integer".to_string())
                        }
                    }
                    _ => Err("VLAN ID value must be a number".to_string()),
                }
            }),
        );

        // GPS coordinates validator
        self.custom_validators.insert(
            "gps_coordinates".to_string(),
            Box::new(|value| {
                if let Some(coords_str) = value.as_str() {
                    let coords_regex = Regex::new(r"^-?\d+\.\d+,\s*-?\d+\.\d+$")
                        .map_err(|_| "Failed to compile GPS coordinates regex")?;
                    
                    if coords_regex.is_match(coords_str) {
                        // Parse and validate ranges
                        let parts: Vec<&str> = coords_str.split(',').collect();
                        if parts.len() == 2 {
                            let lat: f64 = parts[0].trim().parse()
                                .map_err(|_| "Invalid latitude number")?;
                            let lng: f64 = parts[1].trim().parse()
                                .map_err(|_| "Invalid longitude number")?;
                            
                            if lat >= -90.0 && lat <= 90.0 && lng >= -180.0 && lng <= 180.0 {
                                Ok(())
                            } else {
                                Err("Latitude must be -90 to 90, longitude must be -180 to 180".to_string())
                            }
                        } else {
                            Err("GPS coordinates must be in format 'latitude, longitude'".to_string())
                        }
                    } else {
                        Err("Invalid GPS coordinates format (expected 'latitude, longitude')".to_string())
                    }
                } else {
                    Err("Value must be a string for GPS coordinates validation".to_string())
                }
            }),
        );
    }

    /// Apply custom validators referenced in the schema
    fn apply_custom_validators(&self, values: &Value, schema: &Value) -> Result<Vec<ValidationError>, Vec<ValidationError>> {
        let mut errors = Vec::new();
        
        // Recursively check schema for custom validator references
        self.check_schema_for_custom_validators(values, schema, "", &mut errors);
        
        if errors.is_empty() {
            Ok(vec![])
        } else {
            Err(errors)
        }
    }

    /// Recursively check schema properties for custom validators
    fn check_schema_for_custom_validators(
        &self,
        values: &Value,
        schema: &Value,
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            for (prop_name, prop_schema) in properties {
                let prop_path = if path.is_empty() {
                    prop_name.clone()
                } else {
                    format!("{}.{}", path, prop_name)
                };

                // Check if this property has a custom validator
                if let Some(custom_format) = prop_schema.get("x-custom-validator").and_then(|v| v.as_str()) {
                    if let Some(prop_value) = values.get(prop_name) {
                        if let Some(validator) = self.custom_validators.get(custom_format) {
                            if let Err(error_msg) = validator(prop_value) {
                                errors.push(ValidationError::new(
                                    prop_path.clone(),
                                    "custom_validation".to_string(),
                                    error_msg,
                                    None,
                                    Some(prop_value.clone()),
                                ));
                            }
                        } else {
                            warn!("Unknown custom validator referenced: {}", custom_format);
                        }
                    }
                }

                // Recursively check nested objects
                if prop_schema.get("type").and_then(|t| t.as_str()) == Some("object") {
                    if let Some(nested_values) = values.get(prop_name) {
                        self.check_schema_for_custom_validators(nested_values, prop_schema, &prop_path, errors);
                    }
                }
            }
        }
    }


    /// Clear the schema cache (useful for memory management)
    pub fn clear_cache(&mut self) {
        self.compiled_schemas.clear();
        debug!("Schema validation cache cleared");
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        (self.compiled_schemas.len(), self.custom_validators.len())
    }
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for schema validation
pub fn validate_field_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(anyhow::anyhow!("Field name cannot be empty"));
    }
    
    if name.len() > 100 {
        return Err(anyhow::anyhow!("Field name cannot exceed 100 characters"));
    }
    
    // Field names should be valid identifiers
    let name_regex = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$")
        .map_err(|_| anyhow::anyhow!("Failed to compile field name regex"))?;
    
    if !name_regex.is_match(name) {
        return Err(anyhow::anyhow!("Field name must start with a letter and contain only letters, numbers, and underscores"));
    }
    
    Ok(())
}

/// Validate that a schema has required properties
pub fn validate_required_schema_properties(schema: &Value) -> ValidationResult {
    let mut errors = Vec::new();
    
    // Must have type property
    if schema.get("type").is_none() {
        errors.push(ValidationError::new(
            "root".to_string(),
            "missing_property".to_string(),
            "Schema must have a 'type' property".to_string(),
            Some(Value::String("object".to_string())),
            None,
        ));
    }
    
    // If type is object, should have properties
    if schema.get("type").and_then(|t| t.as_str()) == Some("object") {
        if schema.get("properties").is_none() {
            errors.push(ValidationError::new(
                "root".to_string(),
                "missing_property".to_string(),
                "Object schemas should define properties".to_string(),
                Some(Value::Object(serde_json::Map::new())),
                None,
            ));
        }
    }
    
    if errors.is_empty() {
        ValidationResult::success()
    } else {
        ValidationResult::with_errors(errors)
    }
}

/// Create a basic schema template for a field type
pub fn create_field_schema_template(field_type: &str) -> Result<Value> {
    let schema = match field_type {
        "text" => json!({
            "type": "string",
            "maxLength": 255
        }),
        "textarea" => json!({
            "type": "string",
            "maxLength": 2000
        }),
        "number" => json!({
            "type": "number"
        }),
        "date" => json!({
            "type": "string",
            "format": "date"
        }),
        "dropdown" => json!({
            "type": "string",
            "enum": []
        }),
        "checkbox" => json!({
            "type": "boolean"
        }),
        _ => return Err(anyhow::anyhow!("Unknown field type: {}", field_type)),
    };
    
    Ok(schema)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_schema_validator_creation() {
        let validator = SchemaValidator::new();
        let (schemas, validators) = validator.get_cache_stats();
        
        assert_eq!(schemas, 0); // No schemas cached initially
        assert!(validators > 0); // Should have custom validators registered
    }

    #[test]
    fn test_valid_schema_validation() {
        let mut validator = SchemaValidator::new();
        
        let valid_schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "minLength": 1
                },
                "age": {
                    "type": "number",
                    "minimum": 0
                }
            },
            "required": ["name"]
        });
        
        let result = validator.validate_schema_definition(&valid_schema.to_string());
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_invalid_schema_validation() {
        let mut validator = SchemaValidator::new();
        
        let invalid_schema = "{ not valid json }";
        
        let result = validator.validate_schema_definition(invalid_schema);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert_eq!(result.errors[0].error_type, "json_parse_error");
    }

    #[test]
    fn test_metadata_values_validation() {
        let mut validator = SchemaValidator::new();
        
        let schema = AssetMetadataSchema::new(
            "Test Schema".to_string(),
            "Test".to_string(),
            json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "minLength": 1
                    },
                    "count": {
                        "type": "number",
                        "minimum": 0
                    }
                },
                "required": ["name"]
            }).to_string(),
            None,
            1,
        );
        
        // Valid values
        let valid_values = json!({
            "name": "Test Asset",
            "count": 10
        });
        
        let result = validator.validate_metadata_values(&schema, &valid_values.to_string());
        assert!(result.is_valid);
        
        // Invalid values (missing required field)
        let invalid_values = json!({
            "count": 10
        });
        
        let result = validator.validate_metadata_values(&schema, &invalid_values.to_string());
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_custom_ip_validator() {
        let validator = SchemaValidator::new();
        let ip_validator = validator.custom_validators.get("ipv4").unwrap();
        
        // Valid IP
        let valid_ip = Value::String("192.168.1.100".to_string());
        assert!(ip_validator(&valid_ip).is_ok());
        
        // Invalid IP
        let invalid_ip = Value::String("300.300.300.300".to_string());
        assert!(ip_validator(&invalid_ip).is_err());
        
        // Non-string value
        let non_string = Value::Number(serde_json::Number::from(123));
        assert!(ip_validator(&non_string).is_err());
    }

    #[test]
    fn test_custom_mac_validator() {
        let validator = SchemaValidator::new();
        let mac_validator = validator.custom_validators.get("mac_address").unwrap();
        
        // Valid MAC addresses
        let valid_mac_colon = Value::String("00:1B:44:11:3A:B7".to_string());
        assert!(mac_validator(&valid_mac_colon).is_ok());
        
        let valid_mac_hyphen = Value::String("00-1B-44-11-3A-B7".to_string());
        assert!(mac_validator(&valid_mac_hyphen).is_ok());
        
        // Invalid MAC address
        let invalid_mac = Value::String("00:1B:44:11:3A".to_string());
        assert!(mac_validator(&invalid_mac).is_err());
    }

    #[test]
    fn test_custom_semver_validator() {
        let validator = SchemaValidator::new();
        let semver_validator = validator.custom_validators.get("semver").unwrap();
        
        // Valid semantic versions
        let valid_versions = vec![
            "1.0.0",
            "v2.1.3",
            "10.20.30",
            "1.1.2-alpha",
            "1.0.0+build.123",
            "2.0.0-beta.1+exp.sha.5114f85",
        ];
        
        for version in valid_versions {
            let value = Value::String(version.to_string());
            assert!(semver_validator(&value).is_ok(), "Version {} should be valid", version);
        }
        
        // Invalid versions
        let invalid_versions = vec![
            "1.0",
            "1.0.0.0",
            "1.a.0",
            "v",
            "",
        ];
        
        for version in invalid_versions {
            let value = Value::String(version.to_string());
            assert!(semver_validator(&value).is_err(), "Version {} should be invalid", version);
        }
    }

    #[test]
    fn test_field_name_validation() {
        // Valid field names
        assert!(validate_field_name("fieldName").is_ok());
        assert!(validate_field_name("field_name").is_ok());
        assert!(validate_field_name("field123").is_ok());
        assert!(validate_field_name("f").is_ok());
        
        // Invalid field names
        assert!(validate_field_name("").is_err());
        assert!(validate_field_name("123field").is_err());
        assert!(validate_field_name("field-name").is_err());
        assert!(validate_field_name("field name").is_err());
        assert!(validate_field_name(&"a".repeat(101)).is_err());
    }

    #[test]
    fn test_required_schema_properties() {
        // Valid schema
        let valid_schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        
        let result = validate_required_schema_properties(&valid_schema);
        assert!(result.is_valid);
        
        // Schema missing type
        let invalid_schema = json!({
            "properties": {
                "name": {"type": "string"}
            }
        });
        
        let result = validate_required_schema_properties(&invalid_schema);
        assert!(!result.is_valid);
        assert_eq!(result.errors[0].error_type, "missing_property");
    }

    #[test]
    fn test_field_schema_templates() {
        // Text field template
        let text_schema = create_field_schema_template("text").unwrap();
        assert_eq!(text_schema["type"], "string");
        assert_eq!(text_schema["maxLength"], 255);
        
        // Number field template
        let number_schema = create_field_schema_template("number").unwrap();
        assert_eq!(number_schema["type"], "number");
        
        // Date field template
        let date_schema = create_field_schema_template("date").unwrap();
        assert_eq!(date_schema["type"], "string");
        assert_eq!(date_schema["format"], "date");
        
        // Unknown field type
        assert!(create_field_schema_template("unknown").is_err());
    }

    #[test]
    fn test_gps_coordinates_validator() {
        let validator = SchemaValidator::new();
        let gps_validator = validator.custom_validators.get("gps_coordinates").unwrap();
        
        // Valid coordinates
        let valid_coords = vec![
            "40.7128, -74.0060", // New York
            "51.5074, -0.1278",  // London
            "0.0, 0.0",          // Null Island
            "-33.8688, 151.2093", // Sydney
        ];
        
        for coords in valid_coords {
            let value = Value::String(coords.to_string());
            assert!(gps_validator(&value).is_ok(), "Coordinates {} should be valid", coords);
        }
        
        // Invalid coordinates
        let invalid_coords = vec![
            "91.0, 0.0",         // Invalid latitude
            "0.0, 181.0",        // Invalid longitude
            "not coordinates",    // Not numeric
            "40.7128",           // Missing longitude
            "40.7128, -74.0060, 0", // Too many values
        ];
        
        for coords in invalid_coords {
            let value = Value::String(coords.to_string());
            assert!(gps_validator(&value).is_err(), "Coordinates {} should be invalid", coords);
        }
    }
}