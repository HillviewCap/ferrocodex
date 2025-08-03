//! Workflow State Manager
//! 
//! In-memory state management for active workflows with persistence coordination

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use super::{
    WorkflowError, WorkflowResult, WorkflowService, WorkflowState, WorkflowSession,
    WorkflowData, WorkflowStepName, constants::*,
};

/// In-memory workflow state manager
pub struct WorkflowStateManager {
    /// Active workflow sessions indexed by session token
    active_sessions: Arc<RwLock<HashMap<String, WorkflowSession>>>,
    /// Active workflow states indexed by workflow ID
    active_workflows: Arc<RwLock<HashMap<String, WorkflowState>>>,
    /// Draft auto-save timers indexed by workflow ID
    auto_save_timers: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
    /// Reference to workflow service for persistence operations
    workflow_service: Arc<WorkflowService>,
}

impl WorkflowStateManager {
    pub fn new(workflow_service: Arc<WorkflowService>) -> Self {
        Self {
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            active_workflows: Arc::new(RwLock::new(HashMap::new())),
            auto_save_timers: Arc::new(Mutex::new(HashMap::new())),
            workflow_service,
        }
    }

    /// Start managing a workflow session
    pub async fn add_session(&self, session: WorkflowSession, state: WorkflowState) -> WorkflowResult<()> {
        let workflow_id = session.workflow_id.clone();
        let session_token = session.session_token.clone();

        // Add to active collections
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session_token.clone(), session);
        }

        {
            let mut workflows = self.active_workflows.write().await;
            workflows.insert(workflow_id.clone(), state);
        }

        // Start auto-save timer if enabled
        self.start_auto_save_timer(&workflow_id).await?;

        log::debug!("Added workflow session: {} for workflow: {}", session_token, workflow_id);
        Ok(())
    }

    /// Remove a workflow session and cleanup
    pub async fn remove_session(&self, session_token: &str) -> WorkflowResult<()> {
        let workflow_id = {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.remove(session_token) {
                session.workflow_id
            } else {
                return Ok(()); // Session already removed
            }
        };

        // Remove workflow state
        {
            let mut workflows = self.active_workflows.write().await;
            workflows.remove(&workflow_id);
        }

        // Stop auto-save timer
        self.stop_auto_save_timer(&workflow_id).await;

        log::debug!("Removed workflow session: {} for workflow: {}", session_token, workflow_id);
        Ok(())
    }

    /// Get workflow state by session token
    pub async fn get_workflow_by_session(&self, session_token: &str) -> WorkflowResult<Option<WorkflowState>> {
        let sessions = self.active_sessions.read().await;
        if let Some(session) = sessions.get(session_token) {
            if session.is_expired() {
                return Ok(None);
            }

            let workflows = self.active_workflows.read().await;
            Ok(workflows.get(&session.workflow_id).cloned())
        } else {
            Ok(None)
        }
    }

    /// Get workflow state by workflow ID
    pub async fn get_workflow_state(&self, workflow_id: &str) -> WorkflowResult<Option<WorkflowState>> {
        let workflows = self.active_workflows.read().await;
        Ok(workflows.get(workflow_id).cloned())
    }

    /// Update workflow state in memory
    pub async fn update_workflow_state(&self, state: WorkflowState) -> WorkflowResult<()> {
        let mut workflows = self.active_workflows.write().await;
        workflows.insert(state.id.clone(), state);
        Ok(())
    }

    /// Update workflow step data
    pub async fn update_step_data(
        &self,
        workflow_id: &str,
        step: WorkflowStepName,
        data: WorkflowData,
    ) -> WorkflowResult<()> {
        let mut workflows = self.active_workflows.write().await;
        if let Some(workflow_state) = workflows.get_mut(workflow_id) {
            workflow_state.current_step = step;
            workflow_state.update_data(data);
            Ok(())
        } else {
            Err(WorkflowError::NotFound(workflow_id.to_string()))
        }
    }

    /// Validate session token and extend expiry
    pub async fn validate_and_extend_session(&self, session_token: &str) -> WorkflowResult<bool> {
        let mut sessions = self.active_sessions.write().await;
        if let Some(session) = sessions.get_mut(session_token) {
            if session.is_expired() {
                sessions.remove(session_token);
                return Ok(false);
            }

            // Extend session expiry
            session.extend_expiry();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get active workflow count for user
    pub async fn get_active_workflow_count(&self, user_id: i64) -> WorkflowResult<usize> {
        let workflows = self.active_workflows.read().await;
        let count = workflows.values()
            .filter(|w| w.user_id == user_id && w.is_active())
            .count();
        Ok(count)
    }

    /// Start periodic cleanup of expired sessions
    pub async fn start_cleanup_task(&self) -> WorkflowResult<tokio::task::JoinHandle<()>> {
        let state_manager = Arc::new(self.clone());
        
        let handle = tokio::spawn(async move {
            let mut cleanup_interval = interval(Duration::from_secs(300)); // 5 minutes
            
            loop {
                cleanup_interval.tick().await;
                
                if let Err(e) = state_manager.cleanup_expired_sessions().await {
                    log::error!("Failed to cleanup expired sessions: {}", e);
                }
            }
        });

        Ok(handle)
    }

    /// Resume workflow from persistent storage
    pub async fn resume_workflow(&self, workflow_id: &str, user_id: i64) -> WorkflowResult<WorkflowState> {
        // Load from persistent storage via workflow service
        let workflow_state = self.workflow_service
            .resume_workflow(user_id, workflow_id)
            .await?;

        // Create new session
        let session = super::models::WorkflowSession::new(workflow_id.to_string(), user_id);

        // Add to active state
        self.add_session(session, workflow_state.clone()).await?;

        Ok(workflow_state)
    }

    /// Persist workflow state to storage
    pub async fn persist_workflow_state(&self, workflow_id: &str) -> WorkflowResult<()> {
        let workflows = self.active_workflows.read().await;
        if let Some(state) = workflows.get(workflow_id) {
            // Use workflow service to persist
            // Note: This would require a method in workflow service to save state
            log::debug!("Persisting workflow state for: {}", workflow_id);
            // TODO: Implement actual persistence call
            Ok(())
        } else {
            Err(WorkflowError::NotFound(workflow_id.to_string()))
        }
    }

    /// Save workflow draft
    pub async fn save_workflow_draft(&self, workflow_id: &str) -> WorkflowResult<()> {
        let workflows = self.active_workflows.read().await;
        if let Some(state) = workflows.get(workflow_id) {
            self.workflow_service
                .save_workflow_draft(state.user_id, workflow_id, state.data.clone())
                .await?;
            log::debug!("Saved draft for workflow: {}", workflow_id);
        }
        Ok(())
    }

    // Private helper methods

    /// Start auto-save timer for a workflow
    async fn start_auto_save_timer(&self, workflow_id: &str) -> WorkflowResult<()> {
        let workflow_id_clone = workflow_id.to_string();
        let state_manager = self.clone();

        let timer_handle = tokio::spawn(async move {
            let mut auto_save_interval = interval(Duration::from_secs(DEFAULT_AUTO_SAVE_INTERVAL as u64));
            
            loop {
                auto_save_interval.tick().await;
                
                // Check if workflow still exists
                if state_manager.get_workflow_state(&workflow_id_clone).await.unwrap_or(None).is_none() {
                    break; // Workflow no longer active, stop timer
                }
                
                // Save draft
                if let Err(e) = state_manager.save_workflow_draft(&workflow_id_clone).await {
                    log::error!("Auto-save failed for workflow {}: {}", workflow_id_clone, e);
                }
            }
        });

        // Store timer handle
        {
            let mut timers = self.auto_save_timers.lock().map_err(|e| {
                WorkflowError::InvalidState(format!("Failed to acquire timer lock: {}", e))
            })?;
            timers.insert(workflow_id.to_string(), timer_handle);
        }

        Ok(())
    }

    /// Stop auto-save timer for a workflow
    async fn stop_auto_save_timer(&self, workflow_id: &str) {
        let timer_handle = {
            let mut timers = self.auto_save_timers.lock().unwrap();
            timers.remove(workflow_id)
        };

        if let Some(handle) = timer_handle {
            handle.abort();
            log::debug!("Stopped auto-save timer for workflow: {}", workflow_id);
        }
    }

    /// Cleanup expired sessions
    async fn cleanup_expired_sessions(&self) -> WorkflowResult<()> {
        let expired_tokens: Vec<String> = {
            let sessions = self.active_sessions.read().await;
            sessions.iter()
                .filter(|(_, session)| session.is_expired())
                .map(|(token, _)| token.clone())
                .collect()
        };

        for token in expired_tokens {
            self.remove_session(&token).await?;
            log::debug!("Cleaned up expired session: {}", token);
        }

        Ok(())
    }

    /// Create a deep copy for async tasks
    fn clone(&self) -> Self {
        Self {
            active_sessions: self.active_sessions.clone(),
            active_workflows: self.active_workflows.clone(),
            auto_save_timers: self.auto_save_timers.clone(),
            workflow_service: self.workflow_service.clone(),
        }
    }
}

/// Workflow resumption manager
pub struct WorkflowResumptionManager {
    state_manager: Arc<WorkflowStateManager>,
    workflow_service: Arc<WorkflowService>,
}

impl WorkflowResumptionManager {
    pub fn new(
        state_manager: Arc<WorkflowStateManager>,
        workflow_service: Arc<WorkflowService>,
    ) -> Self {
        Self {
            state_manager,
            workflow_service,
        }
    }

    /// Detect and resume interrupted workflows for a user
    pub async fn resume_interrupted_workflows(&self, user_id: i64) -> WorkflowResult<Vec<WorkflowState>> {
        // Get active workflows from persistent storage
        let active_workflows = self.workflow_service
            .get_active_workflows_for_user(user_id)
            .await?;

        let mut resumed_workflows = Vec::new();

        for workflow in active_workflows {
            // Check if already in memory
            if self.state_manager.get_workflow_state(&workflow.id).await?.is_some() {
                continue; // Already active in memory
            }

            // Resume workflow
            match self.state_manager.resume_workflow(&workflow.id, user_id).await {
                Ok(resumed_state) => {
                    resumed_workflows.push(resumed_state);
                    log::info!("Resumed workflow: {} for user: {}", workflow.id, user_id);
                }
                Err(e) => {
                    log::error!("Failed to resume workflow {}: {}", workflow.id, e);
                }
            }
        }

        Ok(resumed_workflows)
    }

    /// Check if user has resumable workflows
    pub async fn has_resumable_workflows(&self, user_id: i64) -> WorkflowResult<bool> {
        let active_workflows = self.workflow_service
            .get_active_workflows_for_user(user_id)
            .await?;

        Ok(!active_workflows.is_empty())
    }

    /// Get resumable workflow list for user
    pub async fn get_resumable_workflows(&self, user_id: i64) -> WorkflowResult<Vec<WorkflowState>> {
        self.workflow_service.get_active_workflows_for_user(user_id).await
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

    async fn create_test_state_manager() -> WorkflowStateManager {
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

        WorkflowStateManager::new(workflow_service)
    }

    #[tokio::test]
    async fn test_session_management() {
        let state_manager = create_test_state_manager().await;
        
        let session = super::models::WorkflowSession::new("workflow-1".to_string(), 1);
        let state = super::WorkflowState::new(
            "workflow-1".to_string(),
            super::WorkflowType::AssetCreation,
            1,
            None,
        );

        // Add session
        state_manager.add_session(session.clone(), state.clone()).await.unwrap();

        // Retrieve workflow by session
        let retrieved = state_manager.get_workflow_by_session(&session.session_token).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "workflow-1");

        // Remove session
        state_manager.remove_session(&session.session_token).await.unwrap();
        let removed = state_manager.get_workflow_by_session(&session.session_token).await.unwrap();
        assert!(removed.is_none());
    }

    #[tokio::test]
    async fn test_workflow_state_updates() {
        let state_manager = create_test_state_manager().await;
        
        let mut state = super::WorkflowState::new(
            "workflow-1".to_string(),
            super::WorkflowType::AssetCreation,
            1,
            None,
        );

        // Add initial state
        let session = super::models::WorkflowSession::new("workflow-1".to_string(), 1);
        state_manager.add_session(session, state.clone()).await.unwrap();

        // Update step data
        let mut new_data = super::WorkflowData::default();
        new_data.asset_name = Some("Test Asset".to_string());
        
        state_manager.update_step_data(
            "workflow-1",
            super::WorkflowStepName::AssetTypeSelection,
            new_data,
        ).await.unwrap();

        // Verify update
        let updated = state_manager.get_workflow_state("workflow-1").await.unwrap().unwrap();
        assert_eq!(updated.current_step, super::WorkflowStepName::AssetTypeSelection);
        assert_eq!(updated.data.asset_name, Some("Test Asset".to_string()));
    }
}