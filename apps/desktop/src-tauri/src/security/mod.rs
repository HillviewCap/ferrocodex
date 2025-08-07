use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{error, warn, info};
use sha2::{Sha256, Digest};
use std::fs;
use std::path::Path;

/// Main security validation module for Ferrocodex
/// Provides comprehensive security validation for asset names and file operations

// Re-export submodules
pub mod asset_name_validator;
pub mod file_security;
pub mod windows_reserved;

pub use asset_name_validator::*;
pub use file_security::*;
pub use windows_reserved::*;

/// Security validation result containing success status and optional error details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityValidationResult {
    pub is_valid: bool,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub suggested_corrections: Vec<String>,
    pub security_flags: Vec<String>,
}

impl SecurityValidationResult {
    pub fn success() -> Self {
        Self {
            is_valid: true,
            error_code: None,
            error_message: None,
            suggested_corrections: Vec::new(),
            security_flags: Vec::new(),
        }
    }

    pub fn error(code: &str, message: &str) -> Self {
        Self {
            is_valid: false,
            error_code: Some(code.to_string()),
            error_message: Some(message.to_string()),
            suggested_corrections: Vec::new(),
            security_flags: Vec::new(),
        }
    }

    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggested_corrections = suggestions;
        self
    }

    pub fn with_security_flags(mut self, flags: Vec<String>) -> Self {
        self.security_flags = flags;
        self
    }
}

/// Comprehensive security error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityError {
    InvalidAssetName { 
        name: String, 
        reason: String, 
        suggestions: Vec<String> 
    },
    ReservedName { 
        name: String, 
        reserved_type: String 
    },
    FileSecurityViolation { 
        filename: String, 
        violation: String 
    },
    HashVerificationFailed { 
        expected: String, 
        actual: String 
    },
    PathTraversalAttempt { 
        path: String 
    },
    MaliciousContentDetected { 
        content_type: String, 
        details: String 
    },
    ValidationBypassAttempt { 
        input: String, 
        technique: String 
    },
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::InvalidAssetName { name, reason, .. } => {
                write!(f, "Invalid asset name '{}': {}", name, reason)
            }
            SecurityError::ReservedName { name, reserved_type } => {
                write!(f, "Asset name '{}' is reserved ({})", name, reserved_type)
            }
            SecurityError::FileSecurityViolation { filename, violation } => {
                write!(f, "File security violation in '{}': {}", filename, violation)
            }
            SecurityError::HashVerificationFailed { expected, actual } => {
                write!(f, "Hash verification failed: expected {}, got {}", expected, actual)
            }
            SecurityError::PathTraversalAttempt { path } => {
                write!(f, "Path traversal attempt detected: {}", path)
            }
            SecurityError::MaliciousContentDetected { content_type, details } => {
                write!(f, "Malicious {} content detected: {}", content_type, details)
            }
            SecurityError::ValidationBypassAttempt { input, technique } => {
                write!(f, "Validation bypass attempt using {}: {}", technique, input)
            }
        }
    }
}

impl std::error::Error for SecurityError {}

/// File integrity verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileIntegrityResult {
    pub sha256_hash: String,
    pub file_size: u64,
    pub is_verified: bool,
    pub security_scan_passed: bool,
    pub detected_issues: Vec<String>,
}

/// Main security validation coordinator
pub struct SecurityValidator {
    asset_name_validator: AssetNameValidator,
    file_security_validator: FileSecurityValidator,
    windows_reserved_checker: WindowsReservedNameChecker,
}

impl SecurityValidator {
    pub fn new() -> Self {
        Self {
            asset_name_validator: AssetNameValidator::new(),
            file_security_validator: FileSecurityValidator::new(),
            windows_reserved_checker: WindowsReservedNameChecker::new(),
        }
    }

    /// Comprehensive asset name validation
    pub fn validate_asset_name(&self, name: &str) -> Result<SecurityValidationResult, SecurityError> {
        info!("Validating asset name: {}", name);

        // Check for reserved names first
        if let Err(e) = self.windows_reserved_checker.check_name(name) {
            warn!("Reserved name detected: {}", name);
            return Ok(SecurityValidationResult::error("RESERVED_NAME", &e.to_string()));
        }

        // Validate name pattern
        match self.asset_name_validator.validate_name(name) {
            Ok(result) => {
                info!("Asset name validation successful: {}", name);
                Ok(result)
            }
            Err(e) => {
                warn!("Asset name validation failed: {} - {}", name, e);
                Ok(SecurityValidationResult::error("INVALID_PATTERN", &e.to_string()))
            }
        }
    }

    /// Sanitize and validate asset name with suggestions
    pub fn sanitize_asset_name(&self, name: &str) -> Result<String, SecurityError> {
        self.asset_name_validator.sanitize_name(name)
    }

    /// Comprehensive file security validation
    pub fn validate_file_upload(&self, file_path: &str) -> Result<FileIntegrityResult, SecurityError> {
        info!("Validating file upload: {}", file_path);
        
        // Validate filename first
        let filename = Path::new(file_path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| SecurityError::FileSecurityViolation {
                filename: file_path.to_string(),
                violation: "Invalid filename".to_string(),
            })?;

        let sanitized_filename = self.file_security_validator.sanitize_filename(filename)?;
        
        // Calculate file hash
        let hash = self.calculate_file_hash(file_path)?;
        
        // Get file size
        let metadata = fs::metadata(file_path).map_err(|e| SecurityError::FileSecurityViolation {
            filename: file_path.to_string(),
            violation: format!("Cannot read file metadata: {}", e),
        })?;

        // Perform security scan
        let security_scan = self.file_security_validator.scan_file(file_path)?;

        Ok(FileIntegrityResult {
            sha256_hash: hash,
            file_size: metadata.len(),
            is_verified: true,
            security_scan_passed: security_scan.is_empty(),
            detected_issues: security_scan,
        })
    }

    /// Calculate SHA-256 hash of file
    pub fn calculate_file_hash(&self, file_path: &str) -> Result<String, SecurityError> {
        let file_content = fs::read(file_path).map_err(|e| SecurityError::FileSecurityViolation {
            filename: file_path.to_string(),
            violation: format!("Cannot read file: {}", e),
        })?;

        let mut hasher = Sha256::new();
        hasher.update(&file_content);
        let result = hasher.finalize();
        
        Ok(format!("{:x}", result))
    }

    /// Verify file integrity against expected hash
    pub fn verify_file_integrity(&self, file_path: &str, expected_hash: &str) -> Result<bool, SecurityError> {
        let actual_hash = self.calculate_file_hash(file_path)?;
        
        if actual_hash == expected_hash {
            info!("File integrity verification successful: {}", file_path);
            Ok(true)
        } else {
            error!("File integrity verification failed: {} (expected: {}, actual: {})", 
                   file_path, expected_hash, actual_hash);
            Err(SecurityError::HashVerificationFailed {
                expected: expected_hash.to_string(),
                actual: actual_hash,
            })
        }
    }
}

impl Default for SecurityValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_security_validation_result() {
        let success = SecurityValidationResult::success();
        assert!(success.is_valid);
        assert!(success.error_code.is_none());

        let error = SecurityValidationResult::error("TEST_ERROR", "Test error message");
        assert!(!error.is_valid);
        assert_eq!(error.error_code, Some("TEST_ERROR".to_string()));
        assert_eq!(error.error_message, Some("Test error message".to_string()));
    }

    #[test]
    fn test_security_validator_creation() {
        let validator = SecurityValidator::new();
        // Just test that it can be created without panic
        assert!(true);
    }

    #[test]
    fn test_file_hash_calculation() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello, World!").unwrap();

        let validator = SecurityValidator::new();
        let hash = validator.calculate_file_hash(file_path.to_str().unwrap()).unwrap();
        
        // SHA-256 hash of "Hello, World!"
        assert_eq!(hash, "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");
    }
}