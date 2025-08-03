use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use jsonschema::{JSONSchema, Draft};

pub mod repository;
pub mod search;
pub mod templates;
pub mod validation;
pub mod performance;
pub mod api;

pub use repository::*;
pub use search::*;
pub use templates::*;
pub use performance::*;
pub use api::*;

/// Field types supported by the metadata system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Number,
    Date,
    Dropdown,
    Checkbox,
    Textarea,
}

impl FieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::Text => "text",
            FieldType::Number => "number", 
            FieldType::Date => "date",
            FieldType::Dropdown => "dropdown",
            FieldType::Checkbox => "checkbox",
            FieldType::Textarea => "textarea",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "text" => Ok(FieldType::Text),
            "number" => Ok(FieldType::Number),
            "date" => Ok(FieldType::Date),
            "dropdown" => Ok(FieldType::Dropdown),
            "checkbox" => Ok(FieldType::Checkbox),
            "textarea" => Ok(FieldType::Textarea),
            _ => Err(anyhow::anyhow!("Invalid field type: {}", s)),
        }
    }
}

/// Categories for organizing field templates
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum FieldCategory {
    Network,
    Physical,
    Device,
    Operational,
    Security,
}

impl FieldCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldCategory::Network => "network",
            FieldCategory::Physical => "physical",
            FieldCategory::Device => "device", 
            FieldCategory::Operational => "operational",
            FieldCategory::Security => "security",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "network" => Ok(FieldCategory::Network),
            "physical" => Ok(FieldCategory::Physical),
            "device" => Ok(FieldCategory::Device),
            "operational" => Ok(FieldCategory::Operational),
            "security" => Ok(FieldCategory::Security),
            _ => Err(anyhow::anyhow!("Invalid field category: {}", s)),
        }
    }
}

/// Metadata schema definition that can be applied to assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadataSchema {
    pub id: Option<i64>,
    pub name: String,
    pub description: String,
    pub schema_json: String,
    pub asset_type_filter: Option<String>,
    pub created_by: i64,
    pub created_at: String,
    pub is_system_template: bool,
    pub version: i32,
}

impl AssetMetadataSchema {
    pub fn new(
        name: String,
        description: String,
        schema_json: String,
        asset_type_filter: Option<String>,
        created_by: i64,
    ) -> Self {
        Self {
            id: None,
            name,
            description,
            schema_json,
            asset_type_filter,
            created_by,
            created_at: String::new(), // Will be set by database
            is_system_template: false,
            version: 1,
        }
    }

    /// Validate that the schema JSON is valid JSON Schema
    pub fn validate_schema(&self) -> Result<()> {
        let schema_value: Value = serde_json::from_str(&self.schema_json)?;
        let _compiled = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema_value)
            .map_err(|e| anyhow::anyhow!("Invalid JSON Schema: {}", e))?;
        Ok(())
    }

    /// Get the compiled JSON Schema for validation
    pub fn get_compiled_schema(&self) -> Result<JSONSchema> {
        let schema_value: Value = serde_json::from_str(&self.schema_json)?;
        JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema_value)
            .map_err(|e| anyhow::anyhow!("Failed to compile schema: {}", e))
    }
}

/// Pre-built field template for common industrial metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataFieldTemplate {
    pub id: Option<i64>,
    pub name: String,
    pub field_type: FieldType,
    pub validation_rules: String, // JSON string containing validation rules
    pub options_json: Option<String>, // JSON string for dropdown options, etc.
    pub category: FieldCategory,
    pub description: String,
    pub is_system: bool,
    pub usage_count: i64,
    pub created_at: String,
}

impl MetadataFieldTemplate {
    pub fn new(
        name: String,
        field_type: FieldType,
        validation_rules: String,
        options_json: Option<String>,
        category: FieldCategory,
        description: String,
        is_system: bool,
    ) -> Self {
        Self {
            id: None,
            name,
            field_type,
            validation_rules,
            options_json,
            category,
            description,
            is_system,
            usage_count: 0,
            created_at: String::new(), // Will be set by database
        }
    }

    /// Get validation rules as JSON Value
    pub fn get_validation_rules(&self) -> Result<Value> {
        serde_json::from_str(&self.validation_rules).map_err(|e| {
            anyhow::anyhow!("Invalid validation rules JSON: {}", e)
        })
    }

    /// Get options as JSON Value if present
    pub fn get_options(&self) -> Result<Option<Value>> {
        match &self.options_json {
            Some(options) => {
                let value = serde_json::from_str(options)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
}

/// Actual metadata values stored for an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub id: Option<i64>,
    pub asset_id: i64,
    pub schema_id: i64,
    pub metadata_values_json: String,
    pub schema_version: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl AssetMetadata {
    pub fn new(
        asset_id: i64,
        schema_id: i64,
        metadata_values_json: String,
        schema_version: i32,
    ) -> Self {
        Self {
            id: None,
            asset_id,
            schema_id,
            metadata_values_json,
            schema_version,
            created_at: String::new(), // Will be set by database
            updated_at: String::new(), // Will be set by database
        }
    }

    /// Get metadata values as JSON Value
    pub fn get_metadata_values(&self) -> Result<Value> {
        serde_json::from_str(&self.metadata_values_json).map_err(|e| {
            anyhow::anyhow!("Invalid metadata values JSON: {}", e)
        })
    }

    /// Validate metadata values against a schema
    pub fn validate_against_schema(&self, schema: &AssetMetadataSchema) -> Result<Vec<String>> {
        let compiled_schema = schema.get_compiled_schema()?;
        let values = self.get_metadata_values()?;
        
        let result = compiled_schema.validate(&values);
        let mut errors = Vec::new();
        
        if let Err(validation_errors) = result {
            for error in validation_errors {
                errors.push(format!("{}: {}", error.instance_path, error));
            }
        }
        
        Ok(errors)
    }
}

/// Information about field types and their constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldTypeInfo {
    pub field_type: FieldType,
    pub display_name: String,
    pub description: String,
    pub supported_constraints: Vec<String>,
    pub default_validation: Value,
}

impl FieldTypeInfo {
    pub fn get_all_field_types() -> Vec<FieldTypeInfo> {
        vec![
            FieldTypeInfo {
                field_type: FieldType::Text,
                display_name: "Text".to_string(),
                description: "Single line text input".to_string(),
                supported_constraints: vec![
                    "minLength".to_string(),
                    "maxLength".to_string(),
                    "pattern".to_string(),
                    "required".to_string(),
                ],
                default_validation: serde_json::json!({
                    "type": "string",
                    "maxLength": 255
                }),
            },
            FieldTypeInfo {
                field_type: FieldType::Textarea,
                display_name: "Text Area".to_string(),
                description: "Multi-line text input".to_string(),
                supported_constraints: vec![
                    "minLength".to_string(),
                    "maxLength".to_string(),
                    "required".to_string(),
                ],
                default_validation: serde_json::json!({
                    "type": "string",
                    "maxLength": 2000
                }),
            },
            FieldTypeInfo {
                field_type: FieldType::Number,
                display_name: "Number".to_string(),
                description: "Numeric input (integer or decimal)".to_string(),
                supported_constraints: vec![
                    "minimum".to_string(),
                    "maximum".to_string(),
                    "exclusiveMinimum".to_string(),
                    "exclusiveMaximum".to_string(),
                    "multipleOf".to_string(),
                    "required".to_string(),
                ],
                default_validation: serde_json::json!({
                    "type": "number"
                }),
            },
            FieldTypeInfo {
                field_type: FieldType::Date,
                display_name: "Date".to_string(),
                description: "Date picker input".to_string(),
                supported_constraints: vec![
                    "format".to_string(),
                    "minimum".to_string(),
                    "maximum".to_string(),
                    "required".to_string(),
                ],
                default_validation: serde_json::json!({
                    "type": "string",
                    "format": "date"
                }),
            },
            FieldTypeInfo {
                field_type: FieldType::Dropdown,
                display_name: "Dropdown".to_string(),
                description: "Select from predefined options".to_string(),
                supported_constraints: vec![
                    "enum".to_string(),
                    "required".to_string(),
                ],
                default_validation: serde_json::json!({
                    "type": "string",
                    "enum": []
                }),
            },
            FieldTypeInfo {
                field_type: FieldType::Checkbox,
                display_name: "Checkbox".to_string(),
                description: "Boolean true/false value".to_string(),
                supported_constraints: vec![
                    "required".to_string(),
                ],
                default_validation: serde_json::json!({
                    "type": "boolean"
                }),
            },
        ]
    }
}

/// Result of validating metadata schema or values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn with_errors(errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    pub fn with_warnings(warnings: Vec<String>) -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings,
        }
    }
}

/// Detailed validation error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field_path: String,
    pub error_type: String,
    pub message: String,
    pub expected: Option<Value>,
    pub actual: Option<Value>,
}

impl ValidationError {
    pub fn new(
        field_path: String,
        error_type: String,
        message: String,
        expected: Option<Value>,
        actual: Option<Value>,
    ) -> Self {
        Self {
            field_path,
            error_type,
            message,
            expected,
            actual,
        }
    }
}

/// Request structure for creating/updating metadata schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMetadataSchemaRequest {
    pub name: String,
    pub description: String,
    pub schema_json: String,
    pub asset_type_filter: Option<String>,
}

/// Request structure for updating metadata schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMetadataSchemaRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub schema_json: Option<String>,
    pub asset_type_filter: Option<String>,
}

/// Statistics about template usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateUsageStats {
    pub template_id: i64,
    pub template_name: String,
    pub usage_count: i64,
    pub last_used: Option<String>,
    pub category: FieldCategory,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_field_type_serialization() {
        let field_type = FieldType::Text;
        let serialized = serde_json::to_string(&field_type).unwrap();
        assert_eq!(serialized, "\"text\"");
        
        let deserialized: FieldType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, FieldType::Text);
    }

    #[test]
    fn test_field_category_conversion() {
        assert_eq!(FieldCategory::Network.as_str(), "network");
        assert_eq!(FieldCategory::from_str("network").unwrap(), FieldCategory::Network);
    }

    #[test]
    fn test_asset_metadata_schema_validation() {
        let valid_schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "minLength": 1
                }
            },
            "required": ["name"]
        });

        let schema = AssetMetadataSchema::new(
            "Test Schema".to_string(),
            "Test description".to_string(),
            valid_schema.to_string(),
            None,
            1,
        );

        assert!(schema.validate_schema().is_ok());
    }

    #[test]
    fn test_invalid_schema_validation() {
        let invalid_schema = "{ invalid json }";

        let schema = AssetMetadataSchema::new(
            "Test Schema".to_string(),
            "Test description".to_string(),
            invalid_schema.to_string(),
            None,
            1,
        );

        assert!(schema.validate_schema().is_err());
    }

    #[test]
    fn test_metadata_field_template_creation() {
        let template = MetadataFieldTemplate::new(
            "IP Address".to_string(),
            FieldType::Text,
            json!({"pattern": "^\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}$"}).to_string(),
            None,
            FieldCategory::Network,
            "IPv4 address field".to_string(),
            true,
        );

        assert_eq!(template.name, "IP Address");
        assert_eq!(template.field_type, FieldType::Text);
        assert_eq!(template.category, FieldCategory::Network);
        assert!(template.is_system);
    }

    #[test]
    fn test_field_type_info() {
        let field_types = FieldTypeInfo::get_all_field_types();
        assert_eq!(field_types.len(), 6);
        
        let text_type = field_types.iter()
            .find(|ft| ft.field_type == FieldType::Text)
            .unwrap();
        assert_eq!(text_type.display_name, "Text");
        assert!(text_type.supported_constraints.contains(&"pattern".to_string()));
    }

    #[test]
    fn test_validation_result() {
        let success = ValidationResult::success();
        assert!(success.is_valid);
        assert!(success.errors.is_empty());

        let error = ValidationError::new(
            "field1".to_string(),
            "required".to_string(),
            "Field is required".to_string(),
            None,
            None,
        );
        let failure = ValidationResult::with_errors(vec![error]);
        assert!(!failure.is_valid);
        assert_eq!(failure.errors.len(), 1);
    }
}