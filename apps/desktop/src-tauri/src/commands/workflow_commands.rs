//! Workflow Tauri Commands
//! 
//! Tauri IPC commands for workflow management operations

use tauri::State;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::workflow::{
    WorkflowService, WorkflowStateManager, WorkflowType, WorkflowStepName, WorkflowData,
    models::*, ValidationResults,
};
use crate::auth::AuthState;
use crate::handlers::workflow_handler::WorkflowHandler;

/// Start a new asset creation workflow
#[tauri::command]
pub async fn start_asset_creation_workflow(
    workflow_type: String,
    initial_data: Option<serde_json::Value>,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<StartWorkflowResponse, String> {
    // Get authenticated user
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    // Parse workflow type
    let workflow_type = match workflow_type.as_str() {
        "asset_creation" => WorkflowType::AssetCreation,
        _ => return Err(format!("Unknown workflow type: {}", workflow_type)),
    };

    // Parse initial data if provided
    let initial_data = if let Some(data) = initial_data {
        Some(serde_json::from_value(data).map_err(|e| e.to_string())?)
    } else {
        None
    };

    let request = StartWorkflowRequest {
        workflow_type,
        initial_data,
    };

    workflow_handler.start_workflow(user_id, request).await
        .map_err(|e| e.to_string())
}

/// Get current workflow state
#[tauri::command]
pub async fn get_workflow_state(
    workflow_id: String,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<WorkflowState, String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    workflow_handler.get_workflow_state(user_id, &workflow_id).await
        .map_err(|e| e.to_string())
}

/// Update workflow step data
#[tauri::command]
pub async fn update_workflow_step(
    workflow_id: String,
    step_name: String,
    step_data: serde_json::Value,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<UpdateWorkflowStepResponse, String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    // Parse step name
    let step_name = parse_workflow_step_name(&step_name)?;

    // Parse step data
    let step_data: WorkflowData = serde_json::from_value(step_data)
        .map_err(|e| format!("Invalid step data: {}", e))?;

    let request = UpdateWorkflowStepRequest {
        workflow_id,
        step_name,
        step_data,
    };

    workflow_handler.update_workflow_step(user_id, request).await
        .map_err(|e| e.to_string())
}

/// Advance workflow to next step
#[tauri::command]
pub async fn advance_workflow_step(
    workflow_id: String,
    target_step: String,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<WorkflowState, String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    let target_step = parse_workflow_step_name(&target_step)?;

    workflow_handler.advance_workflow_step(user_id, &workflow_id, target_step).await
        .map_err(|e| e.to_string())
}

/// Resume workflow from saved state
#[tauri::command]
pub async fn resume_workflow(
    workflow_id: String,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<WorkflowState, String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    workflow_handler.resume_workflow(user_id, &workflow_id).await
        .map_err(|e| e.to_string())
}

/// Complete workflow and create asset
#[tauri::command]
pub async fn complete_workflow(
    workflow_id: String,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<CompleteWorkflowResponse, String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    let request = CompleteWorkflowRequest { workflow_id };

    workflow_handler.complete_workflow(user_id, request).await
        .map_err(|e| e.to_string())
}

/// Cancel workflow
#[tauri::command]
pub async fn cancel_workflow(
    workflow_id: String,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<(), String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    workflow_handler.cancel_workflow(user_id, &workflow_id).await
        .map_err(|e| e.to_string())
}

/// Save workflow draft
#[tauri::command]
pub async fn save_workflow_draft(
    workflow_id: String,
    draft_data: serde_json::Value,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<(), String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    let draft_data: WorkflowData = serde_json::from_value(draft_data)
        .map_err(|e| format!("Invalid draft data: {}", e))?;

    workflow_handler.save_workflow_draft(user_id, &workflow_id, draft_data).await
        .map_err(|e| e.to_string())
}

/// Get workflow drafts for current user
#[tauri::command]
pub async fn get_workflow_drafts(
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<Vec<WorkflowDraft>, String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    workflow_handler.get_workflow_drafts(user_id).await
        .map_err(|e| e.to_string())
}

/// Delete workflow draft
#[tauri::command]
pub async fn delete_workflow_draft(
    workflow_id: String,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<(), String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    workflow_handler.delete_workflow_draft(user_id, &workflow_id).await
        .map_err(|e| e.to_string())
}

/// Validate workflow step
#[tauri::command]
pub async fn validate_workflow_step(
    workflow_id: String,
    step_name: String,
    step_data: serde_json::Value,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<ValidationResults, String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    let step_name = parse_workflow_step_name(&step_name)?;
    let step_data: WorkflowData = serde_json::from_value(step_data)
        .map_err(|e| format!("Invalid step data: {}", e))?;

    let request = ValidateWorkflowStepRequest {
        workflow_id,
        step_name,
        step_data,
    };

    workflow_handler.validate_workflow_step(user_id, request).await
        .map_err(|e| e.to_string())
}

/// Get active workflows for current user
#[tauri::command]
pub async fn get_active_workflows(
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<Vec<WorkflowState>, String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    workflow_handler.get_active_workflows_for_user(user_id).await
        .map_err(|e| e.to_string())
}

/// Validate asset creation permissions
#[tauri::command]
pub async fn validate_asset_creation_permissions(
    user_id: i64,
    parent_id: Option<i64>,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<bool, String> {
    let current_user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    // Only allow checking own permissions or if admin
    if current_user_id != user_id {
        let is_admin = auth_state.is_admin()
            .ok_or("Cannot check permissions for other users")?;
        if !is_admin {
            return Err("Permission denied".to_string());
        }
    }

    workflow_handler.validate_asset_creation_permissions(user_id, parent_id).await
        .map_err(|e| e.to_string())
}

/// Validate security classification
#[tauri::command]
pub async fn validate_security_classification(
    asset_data: serde_json::Value,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<ValidationResults, String> {
    let _user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    // Parse asset data
    let asset_name = asset_data.get("asset_name")
        .and_then(|v| v.as_str())
        .ok_or("asset_name is required")?;
    
    let security_classification = asset_data.get("security_classification")
        .and_then(|v| v.as_str())
        .ok_or("security_classification is required")?;

    workflow_handler.validate_security_classification(asset_name, security_classification).await
        .map_err(|e| e.to_string())
}

/// Check naming compliance
#[tauri::command]
pub async fn check_naming_compliance(
    asset_name: String,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<ValidationResults, String> {
    let _user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    workflow_handler.validate_naming_compliance(&asset_name).await
        .map_err(|e| e.to_string())
}

/// Audit workflow operation
#[tauri::command]
pub async fn audit_workflow_operation(
    workflow_id: String,
    operation: String,
    details: Option<String>,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<(), String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    workflow_handler.audit_workflow_operation(user_id, &workflow_id, &operation, details.as_deref()).await
        .map_err(|e| e.to_string())
}

/// Get workflow step data
#[tauri::command]
pub async fn get_workflow_step_data(
    workflow_id: String,
    step_name: String,
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<WorkflowData, String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    let step_name = parse_workflow_step_name(&step_name)?;

    workflow_handler.get_workflow_step_data(user_id, &workflow_id, step_name).await
        .map_err(|e| e.to_string())
}

/// Get resumable workflows for current user
#[tauri::command]
pub async fn get_resumable_workflows(
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<Vec<WorkflowState>, String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    workflow_handler.get_resumable_workflows(user_id).await
        .map_err(|e| e.to_string())
}

/// Check if user has resumable workflows
#[tauri::command]
pub async fn has_resumable_workflows(
    auth_state: State<'_, AuthState>,
    workflow_handler: State<'_, WorkflowHandler>,
) -> Result<bool, String> {
    let user_id = auth_state.get_current_user_id()
        .ok_or("User not authenticated")?;

    workflow_handler.has_resumable_workflows(user_id).await
        .map_err(|e| e.to_string())
}

// Helper functions

/// Parse workflow step name from string
fn parse_workflow_step_name(step_name: &str) -> Result<WorkflowStepName, String> {
    match step_name {
        "asset_type_selection" => Ok(WorkflowStepName::AssetTypeSelection),
        "hierarchy_selection" => Ok(WorkflowStepName::HierarchySelection),
        "metadata_configuration" => Ok(WorkflowStepName::MetadataConfiguration),
        "security_validation" => Ok(WorkflowStepName::SecurityValidation),
        "review_confirmation" => Ok(WorkflowStepName::ReviewConfirmation),
        _ => Err(format!("Unknown workflow step: {}", step_name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_workflow_step_name() {
        assert!(matches!(
            parse_workflow_step_name("asset_type_selection"),
            Ok(WorkflowStepName::AssetTypeSelection)
        ));
        
        assert!(matches!(
            parse_workflow_step_name("hierarchy_selection"),
            Ok(WorkflowStepName::HierarchySelection)
        ));
        
        assert!(parse_workflow_step_name("invalid_step").is_err());
    }
}