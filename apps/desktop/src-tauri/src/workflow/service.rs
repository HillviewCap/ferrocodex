//! Workflow Service
//! 
//! Business logic for workflow management including lifecycle operations,
//! validation, and integration with other system components

use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::database::DatabaseManager;
use crate::assets::AssetService;
use crate::audit::AuditService;
use crate::auth::AuthService;

use super::{
    WorkflowError, WorkflowResult, WorkflowRepository, WorkflowState, WorkflowSession,
    WorkflowDraft, WorkflowType, WorkflowStatus, WorkflowStepName, WorkflowData,
    models::*,
    validation::WorkflowValidator,
    constants::*,
};

pub struct WorkflowService {
    repository: WorkflowRepository,
    validator: WorkflowValidator,
    asset_service: Arc<AssetService>,
    audit_service: Arc<AuditService>,
    auth_service: Arc<AuthService>,
}

impl WorkflowService {
    pub fn new(
        db_manager: Arc<Mutex<DatabaseManager>>,
        asset_service: Arc<AssetService>,
        audit_service: Arc<AuditService>,
        auth_service: Arc<AuthService>,
    ) -> Self {
        Self {
            repository: WorkflowRepository::new(db_manager.clone()),
            validator: WorkflowValidator::new(db_manager),
            asset_service,
            audit_service,
            auth_service,
        }
    }

    /// Initialize workflow service
    pub async fn initialize(&self) -> WorkflowResult<()> {
        self.repository.initialize_tables().await?;
        Ok(())
    }

    /// Start a new asset creation workflow
    pub async fn start_asset_creation_workflow(
        &self,
        user_id: i64,
        request: StartWorkflowRequest,
    ) -> WorkflowResult<StartWorkflowResponse> {
        // Validate user permissions
        self.validate_user_permissions(user_id, "create_asset").await?;

        // Check concurrent workflow limit
        self.check_concurrent_workflow_limit(user_id).await?;

        // Generate workflow ID
        let workflow_id = Uuid::new_v4().to_string();

        // Create workflow state
        let workflow_state = WorkflowState::new(
            workflow_id.clone(),
            request.workflow_type,
            user_id,
            request.initial_data,
        );

        // Create session
        let session = WorkflowSession::new(workflow_id.clone(), user_id);

        // Save to database
        self.repository.save_workflow_state(&workflow_state).await?;
        self.repository.save_workflow_session(&session).await?;

        // Log audit event
        self.audit_service.log_workflow_event(
            user_id,
            &workflow_id,
            "workflow_started",
            Some(&format!("Started {} workflow", workflow_state.workflow_type)),
        ).await.map_err(|e| WorkflowError::ExternalService(e.to_string()))?;

        Ok(StartWorkflowResponse {
            session,
            state: workflow_state,
        })
    }

    /// Update workflow step data
    pub async fn update_workflow_step(
        &self,
        user_id: i64,
        request: UpdateWorkflowStepRequest,
    ) -> WorkflowResult<UpdateWorkflowStepResponse> {
        // Get and validate workflow
        let mut workflow_state = self.get_and_validate_workflow(&request.workflow_id, user_id).await?;

        // Validate step transition
        if workflow_state.current_step != request.step_name {
            return Err(WorkflowError::InvalidState(
                format!("Cannot update step '{}' when current step is '{}'", 
                    request.step_name, workflow_state.current_step)
            ));
        }

        // Merge step data with existing workflow data
        self.merge_step_data(&mut workflow_state.data, &request.step_data)?;

        // Validate step data
        let validation = self.validator.validate_step(
            &request.step_name,
            &workflow_state.data,
        ).await?;

        // Update workflow state
        workflow_state.update_data(workflow_state.data.clone());
        self.repository.save_workflow_state(&workflow_state).await?;

        // Log audit event
        self.audit_service.log_workflow_event(
            user_id,
            &request.workflow_id,
            "step_updated",
            Some(&format!("Updated step: {}", request.step_name)),
        ).await.map_err(|e| WorkflowError::ExternalService(e.to_string()))?;

        Ok(UpdateWorkflowStepResponse {
            state: workflow_state,
            validation,
        })
    }

    /// Advance workflow to next step
    pub async fn advance_workflow_step(
        &self,
        user_id: i64,
        workflow_id: &str,
        target_step: WorkflowStepName,
    ) -> WorkflowResult<WorkflowState> {
        // Get and validate workflow
        let mut workflow_state = self.get_and_validate_workflow(workflow_id, user_id).await?;

        // Validate current step before advancing
        let current_validation = self.validator.validate_step(
            &workflow_state.current_step,
            &workflow_state.data,
        ).await?;

        if !current_validation.is_valid {
            return Err(WorkflowError::ValidationFailed(
                format!("Current step validation failed: {:?}", current_validation.errors)
            ));
        }

        // Validate step progression
        self.validate_step_progression(&workflow_state.current_step, &target_step)?;

        // Advance to target step
        workflow_state.advance_to_step(target_step.clone());
        self.repository.save_workflow_state(&workflow_state).await?;

        // Log audit event
        self.audit_service.log_workflow_event(
            user_id,
            workflow_id,
            "step_advanced",
            Some(&format!("Advanced to step: {}", target_step)),
        ).await.map_err(|e| WorkflowError::ExternalService(e.to_string()))?;

        Ok(workflow_state)
    }

    /// Get workflow state
    pub async fn get_workflow_state(
        &self,
        user_id: i64,
        workflow_id: &str,
    ) -> WorkflowResult<WorkflowState> {
        self.get_and_validate_workflow(workflow_id, user_id).await
    }

    /// Resume workflow from saved state
    pub async fn resume_workflow(
        &self,
        user_id: i64,
        workflow_id: &str,
    ) -> WorkflowResult<WorkflowState> {
        // Get workflow state
        let mut workflow_state = self.get_and_validate_workflow(workflow_id, user_id).await?;

        // Check if workflow can be resumed
        if !workflow_state.can_be_resumed() {
            return Err(WorkflowError::InvalidState(
                format!("Workflow cannot be resumed (status: {:?})", workflow_state.status)
            ));
        }

        // Resume if paused
        if workflow_state.status == WorkflowStatus::Paused {
            workflow_state.resume();
            self.repository.save_workflow_state(&workflow_state).await?;
        }

        // Create new session
        let session = WorkflowSession::new(workflow_id.to_string(), user_id);
        self.repository.save_workflow_session(&session).await?;

        // Log audit event
        self.audit_service.log_workflow_event(
            user_id,
            workflow_id,
            "workflow_resumed",
            Some("Workflow resumed from saved state"),
        ).await.map_err(|e| WorkflowError::ExternalService(e.to_string()))?;

        Ok(workflow_state)
    }

    /// Complete workflow and create asset
    pub async fn complete_workflow(
        &self,
        user_id: i64,
        request: CompleteWorkflowRequest,
    ) -> WorkflowResult<CompleteWorkflowResponse> {
        // Get and validate workflow
        let mut workflow_state = self.get_and_validate_workflow(&request.workflow_id, user_id).await?;

        // Validate final step
        if workflow_state.current_step != WorkflowStepName::ReviewConfirmation {
            return Err(WorkflowError::InvalidState(
                "Workflow must be at review confirmation step to complete".to_string()
            ));
        }

        // Validate all workflow data
        let final_validation = self.validator.validate_complete_workflow(&workflow_state.data).await?;
        if !final_validation.is_valid {
            return Err(WorkflowError::ValidationFailed(
                format!("Final validation failed: {:?}", final_validation.errors)
            ));
        }

        // Convert workflow data to asset creation data
        let asset_data = AssetCreationData::from_workflow_data(&workflow_state.data)?;

        // Create asset using asset service
        let asset_id = self.asset_service.create_asset_from_workflow(user_id, asset_data)
            .map_err(|e| WorkflowError::ExternalService(e.to_string()))?;

        // Mark workflow as completed
        workflow_state.complete();
        self.repository.save_workflow_state(&workflow_state).await?;

        // Clean up workflow session and drafts
        self.cleanup_workflow_resources(&request.workflow_id).await?;

        // Log audit events
        self.audit_service.log_workflow_event(
            user_id,
            &request.workflow_id,
            "workflow_completed",
            Some(&format!("Workflow completed, asset created with ID: {}", asset_id)),
        ).await.map_err(|e| WorkflowError::ExternalService(e.to_string()))?;

        Ok(CompleteWorkflowResponse {
            asset_id,
            workflow_state,
        })
    }

    /// Cancel workflow
    pub async fn cancel_workflow(
        &self,
        user_id: i64,
        workflow_id: &str,
    ) -> WorkflowResult<()> {
        // Get and validate workflow
        let mut workflow_state = self.get_and_validate_workflow(workflow_id, user_id).await?;

        // Cancel workflow
        workflow_state.cancel();
        self.repository.save_workflow_state(&workflow_state).await?;

        // Clean up resources
        self.cleanup_workflow_resources(workflow_id).await?;

        // Log audit event
        self.audit_service.log_workflow_event(
            user_id,
            workflow_id,
            "workflow_cancelled",
            Some("Workflow cancelled by user"),
        ).await.map_err(|e| WorkflowError::ExternalService(e.to_string()))?;

        Ok(())
    }

    /// Save workflow draft
    pub async fn save_workflow_draft(
        &self,
        user_id: i64,
        workflow_id: &str,
        draft_data: WorkflowData,
    ) -> WorkflowResult<()> {
        // Validate workflow exists and belongs to user
        self.get_and_validate_workflow(workflow_id, user_id).await?;

        // Create or update draft
        let draft = WorkflowDraft::new(workflow_id.to_string(), user_id, draft_data);
        self.repository.save_workflow_draft(&draft).await?;

        Ok(())
    }

    /// Get workflow drafts for user
    pub async fn get_workflow_drafts(
        &self,
        user_id: i64,
    ) -> WorkflowResult<Vec<WorkflowDraft>> {
        self.repository.get_workflow_drafts_for_user(user_id).await
    }

    /// Get active workflows for user
    pub async fn get_active_workflows_for_user(
        &self,
        user_id: i64,
    ) -> WorkflowResult<Vec<WorkflowState>> {
        self.repository.get_active_workflows_for_user(user_id).await
    }

    /// Validate asset creation permissions
    pub async fn validate_asset_creation_permissions(
        &self,
        user_id: i64,
        parent_id: Option<i64>,
    ) -> WorkflowResult<bool> {
        // Check general asset creation permission
        self.validate_user_permissions(user_id, "create_asset").await?;

        // If parent_id is specified, check parent folder permissions
        if let Some(parent_id) = parent_id {
            let has_parent_permission = self.asset_service
                .check_asset_permission(user_id, parent_id, "create_child")
                .map_err(|e| WorkflowError::ExternalService(e.to_string()))?;

            if !has_parent_permission {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Clean up expired workflows and sessions
    pub async fn cleanup_expired_resources(&self) -> WorkflowResult<()> {
        // Clean up expired sessions
        let expired_sessions = self.repository.cleanup_expired_sessions().await?;
        
        // TODO: Clean up abandoned workflows (older than threshold with no activity)
        // This would require additional logic to determine what constitutes "abandoned"
        
        log::info!("Cleaned up {} expired workflow sessions", expired_sessions);
        Ok(())
    }

    // Private helper methods

    async fn get_and_validate_workflow(
        &self,
        workflow_id: &str,
        user_id: i64,
    ) -> WorkflowResult<WorkflowState> {
        let workflow_state = self.repository.get_workflow_state(workflow_id).await?
            .ok_or_else(|| WorkflowError::NotFound(workflow_id.to_string()))?;

        if workflow_state.user_id != user_id {
            return Err(WorkflowError::PermissionDenied(
                "Workflow does not belong to user".to_string()
            ));
        }

        Ok(workflow_state)
    }

    async fn validate_user_permissions(&self, user_id: i64, permission: &str) -> WorkflowResult<()> {
        let has_permission = self.auth_service
            .check_user_permission(user_id, permission)
            .map_err(|e| WorkflowError::ExternalService(e.to_string()))?;

        if !has_permission {
            return Err(WorkflowError::PermissionDenied(
                format!("User lacks permission: {}", permission)
            ));
        }

        Ok(())
    }

    async fn check_concurrent_workflow_limit(&self, user_id: i64) -> WorkflowResult<()> {
        let active_workflows = self.repository.get_active_workflows_for_user(user_id).await?;
        
        if active_workflows.len() >= MAX_CONCURRENT_WORKFLOWS_PER_USER {
            return Err(WorkflowError::InvalidState(
                format!("Maximum concurrent workflows reached ({})", MAX_CONCURRENT_WORKFLOWS_PER_USER)
            ));
        }

        Ok(())
    }

    fn merge_step_data(&self, target: &mut WorkflowData, source: &WorkflowData) -> WorkflowResult<()> {
        // Merge individual fields from source to target
        if source.asset_type.is_some() {
            target.asset_type = source.asset_type.clone();
        }
        if source.asset_name.is_some() {
            target.asset_name = source.asset_name.clone();
        }
        if source.asset_description.is_some() {
            target.asset_description = source.asset_description.clone();
        }
        if source.parent_id.is_some() {
            target.parent_id = source.parent_id;
        }
        if source.parent_path.is_some() {
            target.parent_path = source.parent_path.clone();
        }
        if source.metadata_schema_id.is_some() {
            target.metadata_schema_id = source.metadata_schema_id;
        }
        if source.metadata_values.is_some() {
            target.metadata_values = source.metadata_values.clone();
        }
        if source.security_classification.is_some() {
            target.security_classification = source.security_classification.clone();
        }
        if source.validation_results.is_some() {
            target.validation_results = source.validation_results.clone();
        }

        Ok(())
    }

    fn validate_step_progression(
        &self,
        current_step: &WorkflowStepName,
        target_step: &WorkflowStepName,
    ) -> WorkflowResult<()> {
        // Define valid step progressions
        let valid_next_steps = match current_step {
            WorkflowStepName::AssetTypeSelection => vec![WorkflowStepName::HierarchySelection],
            WorkflowStepName::HierarchySelection => vec![
                WorkflowStepName::AssetTypeSelection,
                WorkflowStepName::MetadataConfiguration,
            ],
            WorkflowStepName::MetadataConfiguration => vec![
                WorkflowStepName::HierarchySelection,
                WorkflowStepName::SecurityValidation,
            ],
            WorkflowStepName::SecurityValidation => vec![
                WorkflowStepName::MetadataConfiguration,
                WorkflowStepName::ReviewConfirmation,
            ],
            WorkflowStepName::ReviewConfirmation => vec![
                WorkflowStepName::SecurityValidation,
            ],
        };

        if !valid_next_steps.contains(target_step) {
            return Err(WorkflowError::InvalidState(
                format!("Invalid step progression from {:?} to {:?}", current_step, target_step)
            ));
        }

        Ok(())
    }

    async fn cleanup_workflow_resources(&self, workflow_id: &str) -> WorkflowResult<()> {
        // Note: This method would clean up sessions and drafts
        // The actual implementation would depend on repository methods
        // For now, we'll just log the cleanup action
        log::info!("Cleaning up resources for workflow: {}", workflow_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabaseManager;
    use tempfile::NamedTempFile;
    use std::sync::{Arc, Mutex};

    // Helper to create test service (would need actual implementations of dependencies)
    async fn create_test_service() -> WorkflowService {
        let temp_file = NamedTempFile::new().unwrap();
        let db_manager = Arc::new(Mutex::new(
            DatabaseManager::new(temp_file.path().to_str().unwrap()).unwrap()
        ));
        
        // Note: In real tests, these would be mock implementations
        let asset_service = Arc::new(AssetService::new(db_manager.clone()));
        let audit_service = Arc::new(AuditService::new(db_manager.clone()));
        let auth_service = Arc::new(AuthService::new(db_manager.clone()));

        let service = WorkflowService::new(
            db_manager,
            asset_service,
            audit_service,
            auth_service,
        );
        
        service.initialize().await.unwrap();
        service
    }

    #[tokio::test]
    async fn test_workflow_step_progression_validation() {
        let service = create_test_service().await;
        
        // Valid progressions
        assert!(service.validate_step_progression(
            &WorkflowStepName::AssetTypeSelection,
            &WorkflowStepName::HierarchySelection
        ).is_ok());

        // Invalid progression
        assert!(service.validate_step_progression(
            &WorkflowStepName::AssetTypeSelection,
            &WorkflowStepName::SecurityValidation
        ).is_err());
    }

    #[test]
    fn test_merge_step_data() {
        let service = WorkflowService::new(
            Arc::new(Mutex::new(DatabaseManager::new(":memory:").unwrap())),
            Arc::new(AssetService::new(Arc::new(Mutex::new(DatabaseManager::new(":memory:").unwrap())))),
            Arc::new(AuditService::new(Arc::new(Mutex::new(DatabaseManager::new(":memory:").unwrap())))),
            Arc::new(AuthService::new(Arc::new(Mutex::new(DatabaseManager::new(":memory:").unwrap())))),
        );

        let mut target = WorkflowData::default();
        let mut source = WorkflowData::default();
        source.asset_name = Some("Test Asset".to_string());
        source.asset_type = Some("Device".to_string());

        service.merge_step_data(&mut target, &source).unwrap();

        assert_eq!(target.asset_name, Some("Test Asset".to_string()));
        assert_eq!(target.asset_type, Some("Device".to_string()));
    }
}