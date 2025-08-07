//! Workflow Handler
//! 
//! Coordinates workflow operations between Tauri commands and workflow services

use std::sync::Arc;

use crate::workflow::{
    WorkflowService, WorkflowStateManager, WorkflowResumptionManager,
    WorkflowResult, WorkflowState, WorkflowStepName, WorkflowData,
    models::*, ValidationResults,
};

pub struct WorkflowHandler {
    workflow_service: Arc<WorkflowService>,
    state_manager: Arc<WorkflowStateManager>,
    resumption_manager: Arc<WorkflowResumptionManager>,
}

impl WorkflowHandler {
    pub fn new(
        workflow_service: Arc<WorkflowService>,
        state_manager: Arc<WorkflowStateManager>,
        resumption_manager: Arc<WorkflowResumptionManager>,
    ) -> Self {
        Self {
            workflow_service,
            state_manager,
            resumption_manager,
        }
    }

    /// Start a new workflow
    pub async fn start_workflow(
        &self,
        user_id: i64,
        request: StartWorkflowRequest,
    ) -> WorkflowResult<StartWorkflowResponse> {
        // Start workflow via service
        let response = self.workflow_service
            .start_asset_creation_workflow(user_id, request)
            .await?;

        // Add to state manager
        self.state_manager
            .add_session(response.session.clone(), response.state.clone())
            .await?;

        Ok(response)
    }

    /// Get workflow state
    pub async fn get_workflow_state(
        &self,
        user_id: i64,
        workflow_id: &str,
    ) -> WorkflowResult<WorkflowState> {
        // First check in-memory state
        if let Some(state) = self.state_manager.get_workflow_state(workflow_id).await? {
            if state.user_id == user_id {
                return Ok(state);
            }
        }

        // Fall back to persistent storage
        self.workflow_service.get_workflow_state(user_id, workflow_id).await
    }

    /// Update workflow step
    pub async fn update_workflow_step(
        &self,
        user_id: i64,
        request: UpdateWorkflowStepRequest,
    ) -> WorkflowResult<UpdateWorkflowStepResponse> {
        // Update via service (handles validation and persistence)
        let response = self.workflow_service
            .update_workflow_step(user_id, request.clone())
            .await?;

        // Update in-memory state
        self.state_manager
            .update_workflow_state(response.state.clone())
            .await?;

        Ok(response)
    }

    /// Advance workflow step
    pub async fn advance_workflow_step(
        &self,
        user_id: i64,
        workflow_id: &str,
        target_step: WorkflowStepName,
    ) -> WorkflowResult<WorkflowState> {
        // Advance via service
        let state = self.workflow_service
            .advance_workflow_step(user_id, workflow_id, target_step)
            .await?;

        // Update in-memory state
        self.state_manager.update_workflow_state(state.clone()).await?;

        Ok(state)
    }

    /// Resume workflow
    pub async fn resume_workflow(
        &self,
        user_id: i64,
        workflow_id: &str,
    ) -> WorkflowResult<WorkflowState> {
        // Use resumption manager to handle the resume operation
        self.resumption_manager
            .resume_interrupted_workflows(user_id)
            .await?;

        // Get the resumed state
        self.get_workflow_state(user_id, workflow_id).await
    }

    /// Complete workflow
    pub async fn complete_workflow(
        &self,
        user_id: i64,
        request: CompleteWorkflowRequest,
    ) -> WorkflowResult<CompleteWorkflowResponse> {
        // Complete via service
        let response = self.workflow_service
            .complete_workflow(user_id, request.clone())
            .await?;

        // Remove from state manager (workflow is completed)
        // We need to find the session token first
        if let Some(state) = self.state_manager.get_workflow_state(&request.workflow_id).await? {
            // Clean up from state manager would require session token
            // For now, the cleanup is handled by the service
        }

        Ok(response)
    }

    /// Cancel workflow
    pub async fn cancel_workflow(
        &self,
        user_id: i64,
        workflow_id: &str,
    ) -> WorkflowResult<()> {
        // Cancel via service
        self.workflow_service.cancel_workflow(user_id, workflow_id).await?;

        // Remove from state manager
        // Similar to complete_workflow, we'd need to clean up the session
        
        Ok(())
    }

    /// Save workflow draft
    pub async fn save_workflow_draft(
        &self,
        user_id: i64,
        workflow_id: &str,
        draft_data: WorkflowData,
    ) -> WorkflowResult<()> {
        // Save via service
        self.workflow_service
            .save_workflow_draft(user_id, workflow_id, draft_data.clone())
            .await?;

        // Update in-memory state if workflow is active
        if let Some(mut state) = self.state_manager.get_workflow_state(workflow_id).await? {
            state.update_data(draft_data);
            self.state_manager.update_workflow_state(state).await?;
        }

        Ok(())
    }

    /// Get workflow drafts for user
    pub async fn get_workflow_drafts(
        &self,
        user_id: i64,
    ) -> WorkflowResult<Vec<WorkflowDraft>> {
        self.workflow_service.get_workflow_drafts(user_id).await
    }

    /// Delete workflow draft (placeholder implementation)
    pub async fn delete_workflow_draft(
        &self,
        user_id: i64,
        workflow_id: &str,
    ) -> WorkflowResult<()> {
        // This would need to be implemented in the workflow service
        log::info!("Delete draft requested for workflow {} by user {}", workflow_id, user_id);
        Ok(())
    }

    /// Validate workflow step
    pub async fn validate_workflow_step(
        &self,
        _user_id: i64,
        request: ValidateWorkflowStepRequest,
    ) -> WorkflowResult<ValidationResults> {
        // This would use the workflow validator directly
        // For now, return a simple implementation
        Ok(ValidationResults {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        })
    }

    /// Get active workflows for user
    pub async fn get_active_workflows_for_user(
        &self,
        user_id: i64,
    ) -> WorkflowResult<Vec<WorkflowState>> {
        self.workflow_service.get_active_workflows_for_user(user_id).await
    }

    /// Validate asset creation permissions
    pub async fn validate_asset_creation_permissions(
        &self,
        user_id: i64,
        parent_id: Option<i64>,
    ) -> WorkflowResult<bool> {
        self.workflow_service
            .validate_asset_creation_permissions(user_id, parent_id)
            .await
    }

    /// Validate security classification
    pub async fn validate_security_classification(
        &self,
        asset_name: &str,
        classification: &str,
    ) -> WorkflowResult<ValidationResults> {
        // This would use the workflow validator
        // Placeholder implementation
        Ok(ValidationResults {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        })
    }

    /// Validate naming compliance
    pub async fn validate_naming_compliance(&self, asset_name: &str) -> WorkflowResult<ValidationResults> {
        // This would use the workflow validator
        // Placeholder implementation that does basic validation
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if asset_name.is_empty() {
            errors.push(crate::workflow::ValidationError {
                field: "asset_name".to_string(),
                message: "Asset name cannot be empty".to_string(),
                code: "EMPTY_NAME".to_string(),
            });
        }

        if asset_name.len() > 100 {
            errors.push(crate::workflow::ValidationError {
                field: "asset_name".to_string(),
                message: "Asset name cannot exceed 100 characters".to_string(),
                code: "NAME_TOO_LONG".to_string(),
            });
        }

        // Check for prohibited characters
        if asset_name.chars().any(|c| "<>:\"/\\|?*".contains(c)) {
            errors.push(crate::workflow::ValidationError {
                field: "asset_name".to_string(),
                message: "Asset name contains prohibited characters".to_string(),
                code: "PROHIBITED_CHARACTERS".to_string(),
            });
        }

        Ok(ValidationResults {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// Audit workflow operation
    pub async fn audit_workflow_operation(
        &self,
        user_id: i64,
        workflow_id: &str,
        operation: &str,
        details: Option<&str>,
    ) -> WorkflowResult<()> {
        // This would integrate with the audit service
        log::info!(
            "Audit: User {} performed {} on workflow {} - {}",
            user_id,
            operation,
            workflow_id,
            details.unwrap_or("No details")
        );
        Ok(())
    }

    /// Get workflow step data
    pub async fn get_workflow_step_data(
        &self,
        user_id: i64,
        workflow_id: &str,
        _step_name: WorkflowStepName,
    ) -> WorkflowResult<WorkflowData> {
        // Get the workflow state and return its data
        let state = self.get_workflow_state(user_id, workflow_id).await?;
        Ok(state.data)
    }

    /// Get resumable workflows
    pub async fn get_resumable_workflows(
        &self,
        user_id: i64,
    ) -> WorkflowResult<Vec<WorkflowState>> {
        self.resumption_manager.get_resumable_workflows(user_id).await
    }

    /// Check if user has resumable workflows
    pub async fn has_resumable_workflows(&self, user_id: i64) -> WorkflowResult<bool> {
        self.resumption_manager.has_resumable_workflows(user_id).await
    }

    /// Initialize handler (startup tasks)
    pub async fn initialize(&self) -> WorkflowResult<()> {
        // Initialize the workflow service
        self.workflow_service.initialize().await?;

        // Start cleanup tasks
        let _cleanup_handle = self.state_manager.start_cleanup_task().await?;

        log::info!("Workflow handler initialized");
        Ok(())
    }

    /// Shutdown handler (cleanup tasks)
    pub async fn shutdown(&self) -> WorkflowResult<()> {
        // Perform any necessary cleanup
        self.workflow_service.cleanup_expired_resources().await?;
        
        log::info!("Workflow handler shut down");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use crate::database::DatabaseManager;
    use crate::assets::AssetService;
    use crate::audit::AuditService;
    use crate::auth::AuthService;
    use tempfile::NamedTempFile;

    async fn create_test_handler() -> WorkflowHandler {
        let temp_file = NamedTempFile::new().unwrap();
        let db_manager = Arc::new(Mutex::new(
            DatabaseManager::new(temp_file.path().to_str().unwrap()).unwrap()
        ));
        
        let asset_service = Arc::new(AssetService::new(db_manager.clone()));
        let audit_service = Arc::new(AuditService::new(db_manager.clone()));
        let auth_service = Arc::new(AuthService::new(db_manager.clone()));
        
        let workflow_service = Arc::new(WorkflowService::new(
            db_manager,
            asset_service,
            audit_service,
            auth_service,
        ));

        let state_manager = Arc::new(WorkflowStateManager::new(workflow_service.clone()));
        let resumption_manager = Arc::new(WorkflowResumptionManager::new(
            state_manager.clone(),
            workflow_service.clone(),
        ));

        WorkflowHandler::new(workflow_service, state_manager, resumption_manager)
    }

    #[tokio::test]
    async fn test_naming_compliance_validation() {
        let handler = create_test_handler().await;

        // Valid name
        let result = handler.validate_naming_compliance("Valid Name").await.unwrap();
        assert!(result.is_valid);

        // Empty name
        let result = handler.validate_naming_compliance("").await.unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "EMPTY_NAME"));

        // Name with prohibited characters
        let result = handler.validate_naming_compliance("Invalid<Name").await.unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == "PROHIBITED_CHARACTERS"));
    }

    #[tokio::test]
    async fn test_handler_initialization() {
        let handler = create_test_handler().await;
        
        // Test initialization
        assert!(handler.initialize().await.is_ok());
        
        // Test shutdown
        assert!(handler.shutdown().await.is_ok());
    }
}