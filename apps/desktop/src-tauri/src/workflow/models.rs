//! Workflow Data Models
//! 
//! Database models and structures for workflow persistence

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use super::{WorkflowType, WorkflowStatus, WorkflowStepName, WorkflowData, AutoSaveConfig, ValidationResults};

/// Database model for workflow state persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub id: String,
    pub workflow_type: WorkflowType,
    pub current_step: WorkflowStepName,
    pub user_id: i64,
    pub status: WorkflowStatus,
    pub data: WorkflowData,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl WorkflowState {
    pub fn new(
        id: String,
        workflow_type: WorkflowType,
        user_id: i64,
        initial_data: Option<WorkflowData>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            workflow_type,
            current_step: WorkflowStepName::AssetTypeSelection,
            user_id,
            status: WorkflowStatus::Active,
            data: initial_data.unwrap_or_default(),
            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    pub fn advance_to_step(&mut self, step: WorkflowStepName) {
        self.current_step = step;
        self.updated_at = Utc::now();
    }

    pub fn update_data(&mut self, data: WorkflowData) {
        self.data = data;
        self.updated_at = Utc::now();
    }

    pub fn complete(&mut self) {
        self.status = WorkflowStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn cancel(&mut self) {
        self.status = WorkflowStatus::Cancelled;
        self.updated_at = Utc::now();
    }

    pub fn pause(&mut self) {
        self.status = WorkflowStatus::Paused;
        self.updated_at = Utc::now();
    }

    pub fn resume(&mut self) {
        self.status = WorkflowStatus::Active;
        self.updated_at = Utc::now();
    }

    pub fn set_error(&mut self) {
        self.status = WorkflowStatus::Error;
        self.updated_at = Utc::now();
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, WorkflowStatus::Active)
    }

    pub fn can_be_resumed(&self) -> bool {
        matches!(self.status, WorkflowStatus::Paused | WorkflowStatus::Active)
    }
}

/// Database model for workflow step definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepConfig {
    pub id: i64,
    pub workflow_type: WorkflowType,
    pub step_name: WorkflowStepName,
    pub step_order: i32,
    pub title: String,
    pub description: Option<String>,
    pub component: String,
    pub validation_rules: serde_json::Value,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Database model for workflow drafts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDraft {
    pub id: String,
    pub workflow_id: String,
    pub user_id: i64,
    pub draft_data: WorkflowData,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WorkflowDraft {
    pub fn new(workflow_id: String, user_id: i64, draft_data: WorkflowData) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workflow_id,
            user_id,
            draft_data,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_data(&mut self, data: WorkflowData) {
        self.draft_data = data;
        self.updated_at = Utc::now();
    }
}

/// Model for asset creation workflow requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartWorkflowRequest {
    pub workflow_type: WorkflowType,
    pub initial_data: Option<WorkflowData>,
}

/// Response for workflow start operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartWorkflowResponse {
    pub session: WorkflowSession,
    pub state: WorkflowState,
}

/// Model for workflow session management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSession {
    pub workflow_id: String,
    pub session_token: String,
    pub user_id: i64,
    pub expires_at: DateTime<Utc>,
    pub auto_save: AutoSaveConfig,
    pub created_at: DateTime<Utc>,
}

impl WorkflowSession {
    pub fn new(workflow_id: String, user_id: i64) -> Self {
        let now = Utc::now();
        Self {
            workflow_id,
            session_token: uuid::Uuid::new_v4().to_string(),
            user_id,
            expires_at: now + chrono::Duration::seconds(super::constants::WORKFLOW_SESSION_DURATION),
            auto_save: AutoSaveConfig::default(),
            created_at: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn extend_expiry(&mut self) {
        self.expires_at = Utc::now() + chrono::Duration::seconds(super::constants::WORKFLOW_SESSION_DURATION);
    }
}

/// Request model for updating workflow steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkflowStepRequest {
    pub workflow_id: String,
    pub step_name: WorkflowStepName,
    pub step_data: WorkflowData,
}

/// Response model for step updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkflowStepResponse {
    pub state: WorkflowState,
    pub validation: ValidationResults,
}

/// Request model for workflow completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteWorkflowRequest {
    pub workflow_id: String,
}

/// Response model for workflow completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteWorkflowResponse {
    pub asset_id: i64,
    pub workflow_state: WorkflowState,
}

/// Model for workflow validation requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateWorkflowStepRequest {
    pub workflow_id: String,
    pub step_name: WorkflowStepName,
    pub step_data: WorkflowData,
}

/// Asset creation data extracted from workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetCreationData {
    pub name: String,
    pub description: Option<String>,
    pub asset_type: String,
    pub parent_id: Option<i64>,
    pub metadata_schema_id: Option<i64>,
    pub metadata_values: Option<HashMap<String, serde_json::Value>>,
    pub security_classification: Option<String>,
}

impl AssetCreationData {
    /// Convert workflow data to asset creation data
    pub fn from_workflow_data(data: &WorkflowData) -> Result<Self, super::WorkflowError> {
        let name = data.asset_name
            .as_ref()
            .ok_or_else(|| super::WorkflowError::ValidationFailed("Asset name is required".to_string()))?
            .clone();

        let asset_type = data.asset_type
            .as_ref()
            .ok_or_else(|| super::WorkflowError::ValidationFailed("Asset type is required".to_string()))?
            .clone();

        Ok(Self {
            name,
            description: data.asset_description.clone(),
            asset_type,
            parent_id: data.parent_id,
            metadata_schema_id: data.metadata_schema_id,
            metadata_values: data.metadata_values.clone(),
            security_classification: data.security_classification.clone(),
        })
    }
}

/// Workflow statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatistics {
    pub total_workflows: i64,
    pub active_workflows: i64,
    pub completed_workflows: i64,
    pub cancelled_workflows: i64,
    pub average_completion_time_minutes: f64,
    pub most_common_exit_step: Option<WorkflowStepName>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_state_creation() {
        let state = WorkflowState::new(
            "test-id".to_string(),
            WorkflowType::AssetCreation,
            1,
            None,
        );

        assert_eq!(state.id, "test-id");
        assert_eq!(state.workflow_type, WorkflowType::AssetCreation);
        assert_eq!(state.current_step, WorkflowStepName::AssetTypeSelection);
        assert_eq!(state.user_id, 1);
        assert_eq!(state.status, WorkflowStatus::Active);
        assert!(state.is_active());
        assert!(state.can_be_resumed());
    }

    #[test]
    fn test_workflow_state_transitions() {
        let mut state = WorkflowState::new(
            "test-id".to_string(),
            WorkflowType::AssetCreation,
            1,
            None,
        );

        // Test advance step
        state.advance_to_step(WorkflowStepName::HierarchySelection);
        assert_eq!(state.current_step, WorkflowStepName::HierarchySelection);

        // Test completion
        state.complete();
        assert_eq!(state.status, WorkflowStatus::Completed);
        assert!(state.completed_at.is_some());
        assert!(!state.is_active());

        // Create new state for other tests
        let mut state = WorkflowState::new(
            "test-id-2".to_string(),
            WorkflowType::AssetCreation,
            1,
            None,
        );

        // Test pause/resume
        state.pause();
        assert_eq!(state.status, WorkflowStatus::Paused);
        assert!(!state.is_active());
        assert!(state.can_be_resumed());

        state.resume();
        assert_eq!(state.status, WorkflowStatus::Active);
        assert!(state.is_active());
    }

    #[test]
    fn test_workflow_session_creation() {
        let session = WorkflowSession::new("workflow-id".to_string(), 1);
        
        assert_eq!(session.workflow_id, "workflow-id");
        assert_eq!(session.user_id, 1);
        assert!(!session.is_expired());
        assert!(!session.session_token.is_empty());
    }

    #[test]
    fn test_asset_creation_data_from_workflow() {
        let mut workflow_data = WorkflowData::default();
        workflow_data.asset_name = Some("Test Asset".to_string());
        workflow_data.asset_type = Some("Device".to_string());
        workflow_data.asset_description = Some("Test description".to_string());

        let asset_data = AssetCreationData::from_workflow_data(&workflow_data).unwrap();
        
        assert_eq!(asset_data.name, "Test Asset");
        assert_eq!(asset_data.asset_type, "Device");
        assert_eq!(asset_data.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_asset_creation_data_validation() {
        let workflow_data = WorkflowData::default();
        
        let result = AssetCreationData::from_workflow_data(&workflow_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_workflow_draft_creation() {
        let draft_data = WorkflowData::default();
        let draft = WorkflowDraft::new("workflow-id".to_string(), 1, draft_data);
        
        assert_eq!(draft.workflow_id, "workflow-id");
        assert_eq!(draft.user_id, 1);
        assert!(!draft.id.is_empty());
    }
}