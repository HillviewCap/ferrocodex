use super::{SecurityValidationResult, SecurityError};
use regex::Regex;
use std::sync::OnceLock;
use tracing::{error, warn, info, debug};
use unicode_normalization::UnicodeNormalization;

/// Asset name validator implementing cybersecurity naming patterns
/// Pattern: ^[A-Z0-9][A-Z0-9_-]{2,49}$
/// - Must start with alphanumeric character (A-Z, 0-9)
/// - Can contain uppercase letters, numbers, underscores, and hyphens
/// - Length: 3-50 characters
/// - Case-insensitive input with uppercase storage conversion
pub struct AssetNameValidator {
    name_pattern: &'static Regex,
    dangerous_patterns: Vec<Regex>,
}

impl AssetNameValidator {
    pub fn new() -> Self {
        Self {
            name_pattern: Self::get_name_pattern(),
            dangerous_patterns: Self::get_dangerous_patterns(),
        }
    }

    /// Get the compiled regex pattern for asset names
    fn get_name_pattern() -> &'static Regex {
        static PATTERN: OnceLock<Regex> = OnceLock::new();
        PATTERN.get_or_init(|| {
            Regex::new(r"^[A-Z0-9][A-Z0-9_-]{2,49}$")
                .expect("Failed to compile asset name regex pattern")
        })
    }

    /// Get dangerous patterns that might indicate bypass attempts
    fn get_dangerous_patterns() -> Vec<Regex> {
        vec![
            // SQL injection patterns
            Regex::new(r"(?i)'|--|/\*|\*/|union\s+select|drop\s+table").unwrap(),
            // Script injection patterns
            Regex::new(r"(?i)<script|javascript:|vbscript:|onload=|onerror=").unwrap(),
            // Path traversal patterns
            Regex::new(r"\.\./|\.\.\\|~|/etc/|\\windows\\").unwrap(),
            // Command injection patterns
            Regex::new(r"(?i)system\(|exec\(|cmd\.|powershell|bash|sh\s").unwrap(),
            // Null bytes and control characters
            Regex::new(r"\x00|\x01|\x02|\x03|\x04|\x05|\x06|\x07|\x08|\x0e|\x0f").unwrap(),
        ]
    }

    /// Validate asset name against security pattern and requirements
    /// Auto-normalizes input for better user experience
    pub fn validate_name(&self, name: &str) -> Result<SecurityValidationResult, SecurityError> {
        info!("Starting asset name validation for: '{}'", name);

        // Input sanitization and normalization
        let normalized_name = self.normalize_input(name)?;
        
        // Basic checks
        if normalized_name.is_empty() {
            return Ok(SecurityValidationResult::error(
                "EMPTY_NAME",
                "Asset name cannot be empty"
            ));
        }

        // Auto-normalize to uppercase and clean characters for user-friendly experience
        let cleaned_name = self.auto_normalize_for_compliance(&normalized_name);
        info!("Normalized '{}' -> cleaned '{}'", normalized_name, cleaned_name);

        // Length validation on cleaned name
        if cleaned_name.len() < 3 {
            return Ok(SecurityValidationResult::error(
                "NAME_TOO_SHORT", 
                "Asset name must be at least 3 characters long"
            ).with_suggestions(vec![
                format!("{}_001", cleaned_name),
                format!("{}_{}", cleaned_name, "ASSET"),
            ]));
        }

        if cleaned_name.len() > 50 {
            return Ok(SecurityValidationResult::error(
                "NAME_TOO_LONG",
                "Asset name cannot exceed 50 characters"
            ).with_suggestions(vec![
                cleaned_name[..47].to_string() + "...",
                self.generate_abbreviated_name(&cleaned_name),
            ]));
        }

        // Check for dangerous patterns on original normalized input
        for pattern in &self.dangerous_patterns {
            if pattern.is_match(&normalized_name) {
                error!("Dangerous pattern detected in asset name: {}", normalized_name);
                return Err(SecurityError::ValidationBypassAttempt {
                    input: name.to_string(),
                    technique: "Pattern injection".to_string(),
                });
            }
        }

        // Validate cleaned name against cybersecurity pattern
        if !self.name_pattern.is_match(&cleaned_name) {
            let suggestions = self.generate_compliant_suggestions(&normalized_name);
            return Ok(SecurityValidationResult::error(
                "INVALID_PATTERN",
                &format!("Asset name '{}' doesn't match required pattern. Must start with a letter or number and contain only letters, numbers, underscores, and hyphens", cleaned_name)
            ).with_suggestions(suggestions));
        }

        // Add security flags if needed
        let mut result = SecurityValidationResult::success();
        let security_flags = self.check_security_concerns(&cleaned_name);
        if !security_flags.is_empty() {
            result = result.with_security_flags(security_flags);
        }
        
        // Success
        info!("Asset name validation successful: {} (normalized from: {})", cleaned_name, name);
        Ok(result)
    }

    /// Sanitize and normalize input name
    fn normalize_input(&self, name: &str) -> Result<String, SecurityError> {
        // Unicode normalization to prevent bypass attempts
        let normalized: String = name.nfc().collect();
        
        // Remove control characters and trim
        let sanitized: String = normalized
            .chars()
            .filter(|c| !c.is_control() || c.is_whitespace())
            .collect::<String>()
            .trim()
            .to_string();

        // Check for null bytes
        if sanitized.contains('\0') {
            return Err(SecurityError::ValidationBypassAttempt {
                input: name.to_string(),
                technique: "Null byte injection".to_string(),
            });
        }

        Ok(sanitized)
    }

    /// Auto-normalize input for compliance (user-friendly)
    fn auto_normalize_for_compliance(&self, input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect::<String>()
            .to_uppercase()
    }

    /// Generate compliant name suggestions
    fn generate_compliant_suggestions(&self, input: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Clean the input to only valid characters
        let cleaned: String = input
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect::<String>()
            .to_uppercase();

        if !cleaned.is_empty() {
            // Ensure it starts with alphanumeric
            let base = if cleaned.chars().next().unwrap_or('A').is_alphanumeric() {
                cleaned
            } else {
                format!("A{}", cleaned)
            };

            // Ensure minimum length
            let padded = if base.len() < 3 {
                format!("{}_001", base)
            } else {
                base
            };

            // Truncate if too long
            let final_name = if padded.len() > 50 {
                padded[..50].to_string()
            } else {
                padded
            };

            suggestions.push(final_name.clone());
            suggestions.push(format!("{}_ASSET", final_name[..45.min(final_name.len())].to_string()));
            suggestions.push(format!("DEVICE_{}", final_name[..43.min(final_name.len())].to_string()));
            
            // Add numbered variations
            for i in 1..=3 {
                let numbered = format!("{}_{:03}", final_name[..46.min(final_name.len())].to_string(), i);
                suggestions.push(numbered);
            }
        } else {
            // Fallback suggestions for completely invalid input
            suggestions.extend(vec![
                "ASSET_001".to_string(),
                "DEVICE_001".to_string(),
                "EQUIPMENT_001".to_string(),
                "UNIT_001".to_string(),
            ]);
        }

        suggestions.into_iter().take(5).collect()
    }

    /// Generate abbreviated name for overly long inputs
    fn generate_abbreviated_name(&self, name: &str) -> String {
        let words: Vec<&str> = name.split(|c: char| c == '_' || c == '-' || c.is_whitespace()).collect();
        
        if words.len() > 1 {
            // Create acronym from first letters
            let acronym: String = words
                .iter()
                .filter(|word| !word.is_empty())
                .map(|word| word.chars().next().unwrap_or('X'))
                .collect::<String>()
                .to_uppercase();
            
            format!("{}_ASSET", acronym)
        } else {
            // Take first 45 characters and add suffix
            format!("{}_A", name[..45.min(name.len())].to_uppercase())
        }
    }

    /// Check for additional security concerns
    fn check_security_concerns(&self, name: &str) -> Vec<String> {
        let mut flags = Vec::new();

        // Check for repetitive patterns that might indicate automated generation
        if self.has_repetitive_pattern(name) {
            flags.push("REPETITIVE_PATTERN".to_string());
        }

        // Check for common test/placeholder names
        if self.is_placeholder_name(name) {
            flags.push("PLACEHOLDER_NAME".to_string());
        }

        // Check for potential encoding attempts
        if name.chars().any(|c| c as u32 > 127) {
            flags.push("NON_ASCII_CHARACTERS".to_string());
        }

        flags
    }

    /// Detect repetitive patterns
    fn has_repetitive_pattern(&self, name: &str) -> bool {
        if name.len() < 6 {
            return false;
        }

        // Check for repeated character sequences
        for len in 2..=4 {
            for start in 0..=(name.len() - len * 2) {
                let pattern = &name[start..start + len];
                let next_part = &name[start + len..start + len * 2];
                if pattern == next_part {
                    return true;
                }
            }
        }

        false
    }

    /// Check if name is a common placeholder
    fn is_placeholder_name(&self, name: &str) -> bool {
        let placeholders = vec![
            "TEST", "DEMO", "SAMPLE", "EXAMPLE", "DEFAULT", "TEMP", "TEMPORARY",
            "PLACEHOLDER", "DUMMY", "MOCK", "FAKE", "NULL", "UNDEFINED",
            "UNTITLED", "UNNAMED", "ASSET", "DEVICE", "EQUIPMENT", "UNIT",
        ];

        placeholders.iter().any(|&placeholder| name.contains(placeholder))
    }

    /// Sanitize name to make it compliant
    pub fn sanitize_name(&self, name: &str) -> Result<String, SecurityError> {
        let normalized = self.normalize_input(name)?;
        
        // Clean to only valid characters
        let cleaned: String = normalized
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect::<String>()
            .to_uppercase();

        if cleaned.is_empty() {
            return Ok("ASSET_001".to_string());
        }

        // Ensure it starts with alphanumeric
        let mut result = if cleaned.chars().next().unwrap().is_alphanumeric() {
            cleaned
        } else {
            format!("A{}", cleaned)
        };

        // Ensure minimum length
        if result.len() < 3 {
            result = format!("{}_001", result);
        }

        // Truncate if too long
        if result.len() > 50 {
            result = result[..50].to_string();
        }

        Ok(result)
    }

    /// Check if a name is available (not checking database, just pattern compliance)
    pub fn is_name_compliant(&self, name: &str) -> bool {
        match self.validate_name(name) {
            Ok(result) => result.is_valid,
            Err(_) => false,
        }
    }
}

impl Default for AssetNameValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_asset_names() {
        let validator = AssetNameValidator::new();

        // Test valid names
        let valid_names = vec![
            "MOTOR_001",
            "PLC123",
            "DEVICE-01",
            "PLC-LINE1-1",  // Test the specific case that was failing
            "A1B2C3",
            "CONTROL_SYSTEM_MAIN",
            "HMI_PANEL_01",
            "SENSOR_TEMP_001",
            "9999_TEST",
            "ABC",
            "BUILDING",     // Test case from user
            "BUILDING1",    // Test case from user
        ];

        for name in valid_names {
            let result = validator.validate_name(name).unwrap();
            println!("Testing '{}': valid={}, error={:?}", name, result.is_valid, result.error_message);
            assert!(result.is_valid, "Name '{}' should be valid", name);
        }
    }

    #[test]
    fn test_invalid_asset_names() {
        let validator = AssetNameValidator::new();

        // Test invalid names
        let invalid_names = vec![
            "",              // Empty
            "AB",            // Too short
            "a" * 51,        // Too long
            "_INVALID",      // Starts with underscore
            "-INVALID",      // Starts with hyphen
            "INVALID@123",   // Contains @
            "INVALID.123",   // Contains period
            "INVALID 123",   // Contains space
            "invalid",       // Lowercase (should be converted)
        ];

        for name in invalid_names {
            let result = validator.validate_name(name).unwrap();
            assert!(!result.is_valid, "Name '{}' should be invalid", name);
        }
    }

    #[test]
    fn test_dangerous_patterns() {
        let validator = AssetNameValidator::new();

        // Test dangerous patterns that should be rejected
        let dangerous_names = vec![
            "'; DROP TABLE--",
            "<script>alert('xss')</script>",
            "../../../etc/passwd",
            "system('rm -rf /')",
            "ASSET\x00NULL",
        ];

        for name in dangerous_names {
            let result = validator.validate_name(name);
            assert!(result.is_err(), "Dangerous name '{}' should be rejected", name);
        }
    }

    #[test]
    fn test_name_sanitization() {
        let validator = AssetNameValidator::new();

        // Test sanitization
        assert_eq!(validator.sanitize_name("test asset 123").unwrap(), "TESTASSET123");
        assert_eq!(validator.sanitize_name("@#$%").unwrap(), "ASSET_001");
        assert_eq!(validator.sanitize_name("_invalid").unwrap(), "AINVALID");
        assert_eq!(validator.sanitize_name("motor-control").unwrap(), "MOTOR-CONTROL");
    }

    #[test]
    fn test_suggestion_generation() {
        let validator = AssetNameValidator::new();

        // Test with too short name
        let result = validator.validate_name("AB").unwrap();
        assert!(!result.is_valid);
        assert!(!result.suggested_corrections.is_empty());

        // Test with invalid characters
        let result = validator.validate_name("invalid@name").unwrap();
        assert!(!result.is_valid);
        assert!(!result.suggested_corrections.is_empty());
    }

    #[test]
    fn test_case_conversion() {
        let validator = AssetNameValidator::new();

        // Test that lowercase input is handled correctly
        let result = validator.validate_name("motor_001");
        assert!(result.is_ok());
        
        let sanitized = validator.sanitize_name("motor_001").unwrap();
        assert_eq!(sanitized, "MOTOR_001");
    }
    
    #[test]
    fn test_building_name_validation() {
        let validator = AssetNameValidator::new();
        
        // Test BUILDING specifically
        let result = validator.validate_name("BUILDING").unwrap();
        assert!(result.is_valid, "BUILDING should be valid");
        assert!(result.error_code.is_none());
        assert!(result.error_message.is_none());
        
        // Test BUILDING1 specifically  
        let result = validator.validate_name("BUILDING1").unwrap();
        assert!(result.is_valid, "BUILDING1 should be valid");
        assert!(result.error_code.is_none());
        assert!(result.error_message.is_none());
        
        // Test lowercase variations
        let result = validator.validate_name("building").unwrap();
        assert!(result.is_valid, "building should be valid (auto-uppercased)");
        
        let result = validator.validate_name("Building1").unwrap();
        assert!(result.is_valid, "Building1 should be valid (auto-uppercased)");
        
        // Test PLC-32123 specifically
        let result = validator.validate_name("PLC-32123").unwrap();
        assert!(result.is_valid, "PLC-32123 should be valid");
        assert!(result.error_code.is_none());
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_unicode_normalization() {
        let validator = AssetNameValidator::new();

        // Test unicode normalization
        let result = validator.normalize_input("café");
        assert!(result.is_ok());
    }

    #[test]
    fn test_security_flags() {
        let validator = AssetNameValidator::new();

        // Test placeholder detection
        let result = validator.validate_name("TEST_ASSET_123").unwrap();
        assert!(result.is_valid);
        assert!(result.security_flags.contains(&"PLACEHOLDER_NAME".to_string()));

        // Test repetitive pattern detection
        let result = validator.validate_name("ABCABCABC123").unwrap();
        assert!(result.is_valid);
        assert!(result.security_flags.contains(&"REPETITIVE_PATTERN".to_string()));
    }
}