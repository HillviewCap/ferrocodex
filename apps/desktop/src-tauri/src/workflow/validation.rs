//! Workflow Validation
//! 
//! Validation logic for workflow steps and data integrity

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use regex::Regex;
use serde_json::Value;

use crate::database::DatabaseManager;
use super::{
    WorkflowError, WorkflowResult, WorkflowStepName, WorkflowData,
    ValidationResults, ValidationError, ValidationWarning,
};

pub struct WorkflowValidator {
    db_manager: Arc<Mutex<DatabaseManager>>,
    validation_rules: HashMap<WorkflowStepName, Vec<ValidationRule>>,
}

#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub field: String,
    pub rule_type: ValidationRuleType,
    pub message: String,
    pub severity: ValidationSeverity,
}

#[derive(Debug, Clone)]
pub enum ValidationRuleType {
    Required,
    Pattern(Regex),
    Length { min: Option<usize>, max: Option<usize> },
    Custom(String), // Custom validation function name
    Conditional { condition: String, rule: Box<ValidationRule> },
}

#[derive(Debug, Clone)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

impl WorkflowValidator {
    pub fn new(db_manager: Arc<Mutex<DatabaseManager>>) -> Self {
        let mut validator = Self {
            db_manager,
            validation_rules: HashMap::new(),
        };
        
        validator.initialize_validation_rules();
        validator
    }

    /// Validate a specific workflow step
    pub async fn validate_step(
        &self,
        step_name: &WorkflowStepName,
        data: &WorkflowData,
    ) -> WorkflowResult<ValidationResults> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Get validation rules for this step
        if let Some(rules) = self.validation_rules.get(step_name) {
            for rule in rules {
                match self.apply_validation_rule(rule, data).await {
                    Ok(None) => {} // Validation passed
                    Ok(Some(message)) => {
                        match rule.severity {
                            ValidationSeverity::Error => {
                                errors.push(ValidationError {
                                    field: rule.field.clone(),
                                    message,
                                    code: self.get_error_code(&rule.rule_type),
                                });
                            }
                            ValidationSeverity::Warning => {
                                warnings.push(ValidationWarning {
                                    field: rule.field.clone(),
                                    message,
                                    code: self.get_warning_code(&rule.rule_type),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(ValidationError {
                            field: rule.field.clone(),
                            message: format!("Validation error: {}", e),
                            code: "VALIDATION_ERROR".to_string(),
                        });
                    }
                }
            }
        }

        // Step-specific validations
        match step_name {
            WorkflowStepName::AssetTypeSelection => {
                self.validate_asset_type_selection(data, &mut errors, &mut warnings).await?;
            }
            WorkflowStepName::HierarchySelection => {
                self.validate_hierarchy_selection(data, &mut errors, &mut warnings).await?;
            }
            WorkflowStepName::MetadataConfiguration => {
                self.validate_metadata_configuration(data, &mut errors, &mut warnings).await?;
            }
            WorkflowStepName::SecurityValidation => {
                self.validate_security_validation(data, &mut errors, &mut warnings).await?;
            }
            WorkflowStepName::ReviewConfirmation => {
                self.validate_review_confirmation(data, &mut errors, &mut warnings).await?;
            }
        }

        Ok(ValidationResults {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// Validate complete workflow data before asset creation
    pub async fn validate_complete_workflow(&self, data: &WorkflowData) -> WorkflowResult<ValidationResults> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate all required fields are present
        if data.asset_name.is_none() {
            errors.push(ValidationError {
                field: "asset_name".to_string(),
                message: "Asset name is required".to_string(),
                code: "REQUIRED_FIELD".to_string(),
            });
        }

        if data.asset_type.is_none() {
            errors.push(ValidationError {
                field: "asset_type".to_string(),
                message: "Asset type is required".to_string(),
                code: "REQUIRED_FIELD".to_string(),
            });
        }

        // Validate asset type specific requirements
        if let Some(asset_type) = &data.asset_type {
            if asset_type == "Device" && data.parent_id.is_none() {
                errors.push(ValidationError {
                    field: "parent_id".to_string(),
                    message: "Devices must be placed in a folder".to_string(),
                    code: "DEVICE_PARENT_REQUIRED".to_string(),
                });
            }
        }

        // Validate security classification
        if data.security_classification.is_none() {
            errors.push(ValidationError {
                field: "security_classification".to_string(),
                message: "Security classification is required".to_string(),
                code: "REQUIRED_FIELD".to_string(),
            });
        }

        // Cross-field validations
        self.validate_cross_field_constraints(data, &mut errors, &mut warnings).await?;

        Ok(ValidationResults {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// Validate security classification and naming compliance
    pub async fn validate_security_classification(
        &self,
        asset_name: &str,
        classification: &str,
    ) -> WorkflowResult<ValidationResults> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate naming compliance
        let naming_validation = self.validate_naming_compliance(asset_name).await?;
        if !naming_validation.is_valid {
            errors.extend(naming_validation.errors);
            warnings.extend(naming_validation.warnings);
        }

        // Validate classification format
        let valid_classifications = ["Public", "Internal", "Confidential", "Restricted"];
        if !valid_classifications.contains(&classification) {
            errors.push(ValidationError {
                field: "security_classification".to_string(),
                message: format!("Invalid security classification: {}", classification),
                code: "INVALID_CLASSIFICATION".to_string(),
            });
        }

        // Classification-specific naming rules
        match classification.to_lowercase().as_str() {
            "restricted" => {
                if asset_name.len() > 50 {
                    warnings.push(ValidationWarning {
                        field: "asset_name".to_string(),
                        message: "Restricted assets should have shorter names for security".to_string(),
                        code: "RESTRICTED_NAMING".to_string(),
                    });
                }
            }
            "confidential" => {
                if asset_name.to_lowercase().contains("test") || asset_name.to_lowercase().contains("demo") {
                    warnings.push(ValidationWarning {
                        field: "asset_name".to_string(),
                        message: "Confidential assets should not contain 'test' or 'demo' in name".to_string(),
                        code: "CONFIDENTIAL_NAMING".to_string(),
                    });
                }
            }
            _ => {}
        }

        Ok(ValidationResults {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// Validate naming compliance against security standards
    pub async fn validate_naming_compliance(&self, name: &str) -> WorkflowResult<ValidationResults> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Basic character validation
        let prohibited_chars = Regex::new(r#"[<>:"/\\|?*]"#).unwrap();
        if prohibited_chars.is_match(name) {
            errors.push(ValidationError {
                field: "asset_name".to_string(),
                message: "Asset name contains prohibited characters: < > : \" / \\ | ? *".to_string(),
                code: "PROHIBITED_CHARACTERS".to_string(),
            });
        }

        // Reserved names check
        let reserved_names = ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", 
                             "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", 
                             "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"];
        
        if reserved_names.contains(&name.to_uppercase().as_str()) {
            errors.push(ValidationError {
                field: "asset_name".to_string(),
                message: format!("Asset name '{}' is a reserved system name", name),
                code: "RESERVED_NAME".to_string(),
            });
        }

        // Length validation
        if name.len() > 100 {
            errors.push(ValidationError {
                field: "asset_name".to_string(),
                message: "Asset name cannot exceed 100 characters".to_string(),
                code: "NAME_TOO_LONG".to_string(),
            });
        }

        if name.trim().is_empty() {
            errors.push(ValidationError {
                field: "asset_name".to_string(),
                message: "Asset name cannot be empty".to_string(),
                code: "NAME_EMPTY".to_string(),
            });
        }

        // Whitespace validation
        if name != name.trim() {
            errors.push(ValidationError {
                field: "asset_name".to_string(),
                message: "Asset name cannot have leading or trailing whitespace".to_string(),
                code: "INVALID_WHITESPACE".to_string(),
            });
        }

        // Consecutive spaces
        if name.contains("  ") {
            warnings.push(ValidationWarning {
                field: "asset_name".to_string(),
                message: "Asset name contains consecutive spaces".to_string(),
                code: "CONSECUTIVE_SPACES".to_string(),
            });
        }

        // Security-oriented naming recommendations
        if name.to_lowercase().contains("password") || name.to_lowercase().contains("secret") {
            warnings.push(ValidationWarning {
                field: "asset_name".to_string(),
                message: "Avoid including sensitive terms in asset names".to_string(),
                code: "SENSITIVE_TERMS".to_string(),
            });
        }

        Ok(ValidationResults {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    // Private methods for step-specific validations

    async fn validate_asset_type_selection(
        &self,
        data: &WorkflowData,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> WorkflowResult<()> {
        // Validate asset type
        if let Some(asset_type) = &data.asset_type {
            let valid_types = ["Folder", "Device"];
            if !valid_types.contains(&asset_type.as_str()) {
                errors.push(ValidationError {
                    field: "asset_type".to_string(),
                    message: format!("Invalid asset type: {}", asset_type),
                    code: "INVALID_ASSET_TYPE".to_string(),
                });
            }
        }

        // Validate asset name
        if let Some(name) = &data.asset_name {
            let naming_validation = self.validate_naming_compliance(name).await?;
            errors.extend(naming_validation.errors);
            warnings.extend(naming_validation.warnings);
        }

        Ok(())
    }

    async fn validate_hierarchy_selection(
        &self,
        data: &WorkflowData,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> WorkflowResult<()> {
        // Device assets must have a parent folder
        if let Some(asset_type) = &data.asset_type {
            if asset_type == "Device" && data.parent_id.is_none() {
                errors.push(ValidationError {
                    field: "parent_id".to_string(),
                    message: "Device assets must be placed inside a folder".to_string(),
                    code: "DEVICE_PARENT_REQUIRED".to_string(),
                });
            }
        }

        // TODO: Validate parent folder exists and user has permissions
        
        Ok(())
    }

    async fn validate_metadata_configuration(
        &self,
        data: &WorkflowData,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> WorkflowResult<()> {
        // Metadata schema is required
        if data.metadata_schema_id.is_none() {
            errors.push(ValidationError {
                field: "metadata_schema_id".to_string(),
                message: "Metadata schema selection is required".to_string(),
                code: "REQUIRED_FIELD".to_string(),
            });
        }

        // TODO: Validate metadata values against schema
        
        Ok(())
    }

    async fn validate_security_validation(
        &self,
        data: &WorkflowData,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> WorkflowResult<()> {
        // Security classification is required
        if data.security_classification.is_none() {
            errors.push(ValidationError {
                field: "security_classification".to_string(),
                message: "Security classification is required".to_string(),
                code: "REQUIRED_FIELD".to_string(),
            });
        }

        // Validate classification and naming compliance
        if let (Some(name), Some(classification)) = (&data.asset_name, &data.security_classification) {
            let security_validation = self.validate_security_classification(name, classification).await?;
            errors.extend(security_validation.errors);
            warnings.extend(security_validation.warnings);
        }

        Ok(())
    }

    async fn validate_review_confirmation(
        &self,
        _data: &WorkflowData,
        _errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> WorkflowResult<()> {
        // No specific validation for review step
        // All validation should have been done in previous steps
        Ok(())
    }

    async fn validate_cross_field_constraints(
        &self,
        data: &WorkflowData,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> WorkflowResult<()> {
        // Example: Certain metadata schemas might not be compatible with certain asset types
        if let (Some(asset_type), Some(_schema_id)) = (&data.asset_type, &data.metadata_schema_id) {
            // TODO: Implement schema-asset type compatibility checks
            if asset_type == "Folder" {
                // Folder-specific validations
            }
        }

        Ok(())
    }

    // Helper methods

    fn initialize_validation_rules(&mut self) {
        // Asset Type Selection rules
        let mut asset_type_rules = Vec::new();
        asset_type_rules.push(ValidationRule {
            field: "asset_type".to_string(),
            rule_type: ValidationRuleType::Required,
            message: "Asset type is required".to_string(),
            severity: ValidationSeverity::Error,
        });
        asset_type_rules.push(ValidationRule {
            field: "asset_name".to_string(),
            rule_type: ValidationRuleType::Required,
            message: "Asset name is required".to_string(),
            severity: ValidationSeverity::Error,
        });
        asset_type_rules.push(ValidationRule {
            field: "asset_name".to_string(),
            rule_type: ValidationRuleType::Length { min: Some(2), max: Some(100) },
            message: "Asset name must be between 2 and 100 characters".to_string(),
            severity: ValidationSeverity::Error,
        });

        self.validation_rules.insert(WorkflowStepName::AssetTypeSelection, asset_type_rules);

        // Add rules for other steps...
        // This would be expanded based on requirements
    }

    async fn apply_validation_rule(
        &self,
        rule: &ValidationRule,
        data: &WorkflowData,
    ) -> WorkflowResult<Option<String>> {
        let field_value = self.get_field_value(data, &rule.field);

        match &rule.rule_type {
            ValidationRuleType::Required => {
                if field_value.is_none() || field_value == Some(Value::Null) {
                    return Ok(Some(rule.message.clone()));
                }
            }
            ValidationRuleType::Pattern(regex) => {
                if let Some(Value::String(s)) = &field_value {
                    if !regex.is_match(s) {
                        return Ok(Some(rule.message.clone()));
                    }
                }
            }
            ValidationRuleType::Length { min, max } => {
                if let Some(Value::String(s)) = &field_value {
                    let len = s.len();
                    if let Some(min_len) = min {
                        if len < *min_len {
                            return Ok(Some(rule.message.clone()));
                        }
                    }
                    if let Some(max_len) = max {
                        if len > *max_len {
                            return Ok(Some(rule.message.clone()));
                        }
                    }
                }
            }
            ValidationRuleType::Custom(_function_name) => {
                // TODO: Implement custom validation functions
            }
            ValidationRuleType::Conditional { condition: _, rule: _ } => {
                // TODO: Implement conditional validation
            }
        }

        Ok(None)
    }

    fn get_field_value(&self, data: &WorkflowData, field: &str) -> Option<Value> {
        // Convert WorkflowData to JSON for field access
        // This is a simplified implementation
        match field {
            "asset_type" => data.asset_type.as_ref().map(|s| Value::String(s.clone())),
            "asset_name" => data.asset_name.as_ref().map(|s| Value::String(s.clone())),
            "asset_description" => data.asset_description.as_ref().map(|s| Value::String(s.clone())),
            _ => None,
        }
    }

    fn get_error_code(&self, rule_type: &ValidationRuleType) -> String {
        match rule_type {
            ValidationRuleType::Required => "REQUIRED_FIELD".to_string(),
            ValidationRuleType::Pattern(_) => "PATTERN_MISMATCH".to_string(),
            ValidationRuleType::Length { .. } => "LENGTH_VALIDATION".to_string(),
            ValidationRuleType::Custom(_) => "CUSTOM_VALIDATION".to_string(),
            ValidationRuleType::Conditional { .. } => "CONDITIONAL_VALIDATION".to_string(),
        }
    }

    fn get_warning_code(&self, rule_type: &ValidationRuleType) -> String {
        format!("{}_WARNING", self.get_error_code(rule_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabaseManager;
    use tempfile::NamedTempFile;

    fn create_test_validator() -> WorkflowValidator {
        let temp_file = NamedTempFile::new().unwrap();
        let db_manager = Arc::new(Mutex::new(
            DatabaseManager::new(temp_file.path().to_str().unwrap()).unwrap()
        ));
        WorkflowValidator::new(db_manager)
    }

    #[tokio::test]
    async fn test_naming_compliance_validation() {
        let validator = create_test_validator();

        // Valid name
        let result = validator.validate_naming_compliance("Valid Asset Name").await.unwrap();
        assert!(result.is_valid);

        // Invalid characters
        let result = validator.validate_naming_compliance("Invalid<>Name").await.unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "PROHIBITED_CHARACTERS"));

        // Reserved name
        let result = validator.validate_naming_compliance("CON").await.unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "RESERVED_NAME"));

        // Too long
        let long_name = "a".repeat(101);
        let result = validator.validate_naming_compliance(&long_name).await.unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "NAME_TOO_LONG"));
    }

    #[tokio::test]
    async fn test_security_classification_validation() {
        let validator = create_test_validator();

        // Valid classification
        let result = validator.validate_security_classification("Test Asset", "Internal").await.unwrap();
        assert!(result.is_valid);

        // Invalid classification
        let result = validator.validate_security_classification("Test Asset", "Invalid").await.unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "INVALID_CLASSIFICATION"));
    }

    #[tokio::test]
    async fn test_step_validation() {
        let validator = create_test_validator();

        // Test asset type selection step
        let mut data = WorkflowData::default();
        data.asset_name = Some("Test Asset".to_string());
        data.asset_type = Some("Device".to_string());

        let result = validator.validate_step(&WorkflowStepName::AssetTypeSelection, &data).await.unwrap();
        assert!(result.is_valid);

        // Test missing required field
        data.asset_name = None;
        let result = validator.validate_step(&WorkflowStepName::AssetTypeSelection, &data).await.unwrap();
        assert!(!result.is_valid);
    }
}