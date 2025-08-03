//! Workflow Management Module
//! 
//! This module provides asset creation workflow capabilities including:
//! - Multi-step workflow execution
//! - State persistence and resumption  
//! - Draft management with auto-save
//! - Validation and security compliance
//! - Integration with asset, metadata, and security systems

pub mod models;
pub mod repository;
pub mod service;
pub mod state;
pub mod validation;

pub use models::*;
pub use service::WorkflowService;
pub use state::WorkflowStateManager;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Workflow execution errors
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    NotFound(String),
    
    #[error("Invalid workflow state: {0}")]
    InvalidState(String),
    
    #[error("Step validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("External service error: {0}")]
    ExternalService(String),
}

impl From<serde_json::Error> for WorkflowError {
    fn from(err: serde_json::Error) -> Self {
        WorkflowError::Serialization(err.to_string())
    }
}

impl From<crate::database::DatabaseError> for WorkflowError {
    fn from(err: crate::database::DatabaseError) -> Self {
        WorkflowError::Database(err.to_string())
    }
}

/// Workflow execution result
pub type WorkflowResult<T> = Result<T, WorkflowError>;

/// Workflow types supported by the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkflowType {
    AssetCreation,
}

impl std::fmt::Display for WorkflowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowType::AssetCreation => write!(f, "asset_creation"),
        }
    }
}

/// Workflow execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkflowStatus {
    Active,
    Paused,
    Completed,
    Cancelled,
    Error,
}

/// Step names for asset creation workflow
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkflowStepName {
    AssetTypeSelection,
    HierarchySelection,
    MetadataConfiguration,
    SecurityValidation,
    ReviewConfirmation,
}

impl std::fmt::Display for WorkflowStepName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowStepName::AssetTypeSelection => write!(f, "asset_type_selection"),
            WorkflowStepName::HierarchySelection => write!(f, "hierarchy_selection"),
            WorkflowStepName::MetadataConfiguration => write!(f, "metadata_configuration"),
            WorkflowStepName::SecurityValidation => write!(f, "security_validation"),
            WorkflowStepName::ReviewConfirmation => write!(f, "review_confirmation"),
        }
    }
}

/// Workflow data container for asset creation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowData {
    pub asset_type: Option<String>,
    pub asset_name: Option<String>,
    pub asset_description: Option<String>,
    pub parent_id: Option<i64>,
    pub parent_path: Option<String>,
    pub metadata_schema_id: Option<i64>,
    pub metadata_values: Option<HashMap<String, serde_json::Value>>,
    pub security_classification: Option<String>,
    pub validation_results: Option<ValidationResults>,
}

/// Validation results for workflow steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResults {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

/// Validation error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub code: String,
}

/// Validation warning details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
    pub code: String,
}

/// Auto-save configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSaveConfig {
    pub enabled: bool,
    pub interval: u32, // seconds
    pub last_saved: Option<chrono::DateTime<chrono::Utc>>,
    pub save_in_progress: bool,
}

impl Default for AutoSaveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: 30,
            last_saved: None,
            save_in_progress: false,
        }
    }
}

/// Workflow session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSession {
    pub workflow_id: String,
    pub session_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub auto_save: AutoSaveConfig,
}

/// Constants for workflow configuration
pub mod constants {
    pub const DEFAULT_AUTO_SAVE_INTERVAL: u32 = 30; // seconds
    pub const WORKFLOW_SESSION_DURATION: i64 = 3600; // 1 hour in seconds
    pub const MAX_DRAFT_AGE_DAYS: i64 = 30;
    pub const MAX_CONCURRENT_WORKFLOWS_PER_USER: usize = 5;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_type_display() {
        assert_eq!(WorkflowType::AssetCreation.to_string(), "asset_creation");
    }

    #[test]
    fn test_workflow_step_display() {
        assert_eq!(WorkflowStepName::AssetTypeSelection.to_string(), "asset_type_selection");
        assert_eq!(WorkflowStepName::HierarchySelection.to_string(), "hierarchy_selection");
        assert_eq!(WorkflowStepName::MetadataConfiguration.to_string(), "metadata_configuration");
        assert_eq!(WorkflowStepName::SecurityValidation.to_string(), "security_validation");
        assert_eq!(WorkflowStepName::ReviewConfirmation.to_string(), "review_confirmation");
    }

    #[test]
    fn test_workflow_data_default() {
        let data = WorkflowData::default();
        assert!(data.asset_type.is_none());
        assert!(data.asset_name.is_none());
        assert!(data.metadata_values.is_none());
    }

    #[test]
    fn test_auto_save_config_default() {
        let config = AutoSaveConfig::default();
        assert!(config.enabled);
        assert_eq!(config.interval, 30);
        assert!(!config.save_in_progress);
    }
}