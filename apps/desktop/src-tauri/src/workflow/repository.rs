//! Workflow Repository
//! 
//! Database operations for workflow persistence and retrieval

use rusqlite::{params, Connection, Row};
use serde_json;
use std::sync::{Arc, Mutex};

use crate::database::DatabaseManager;
use super::{
    WorkflowError, WorkflowResult, WorkflowState, WorkflowDraft, WorkflowSession,
    WorkflowType, WorkflowStatus, WorkflowStepName, WorkflowData, AutoSaveConfig
};

pub struct WorkflowRepository {
    db_manager: Arc<Mutex<DatabaseManager>>,
}

impl WorkflowRepository {
    pub fn new(db_manager: Arc<Mutex<DatabaseManager>>) -> Self {
        Self { db_manager }
    }

    /// Initialize workflow tables
    pub async fn initialize_tables(&self) -> WorkflowResult<()> {
        let db_manager = self.db_manager.lock().map_err(|e| {
            WorkflowError::Database(format!("Failed to acquire database lock: {}", e))
        })?;

        db_manager.execute_batch(&[
            // Workflow states table
            r#"
            CREATE TABLE IF NOT EXISTS workflow_states (
                id TEXT PRIMARY KEY,
                workflow_type TEXT NOT NULL,
                current_step TEXT NOT NULL,
                user_id INTEGER NOT NULL,
                status TEXT NOT NULL,
                data TEXT NOT NULL, -- JSON
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT,
                FOREIGN KEY (user_id) REFERENCES users (id)
            )
            "#,

            // Workflow sessions table
            r#"
            CREATE TABLE IF NOT EXISTS workflow_sessions (
                workflow_id TEXT PRIMARY KEY,
                session_token TEXT NOT NULL UNIQUE,
                user_id INTEGER NOT NULL,
                expires_at TEXT NOT NULL,
                auto_save_config TEXT NOT NULL, -- JSON
                created_at TEXT NOT NULL,
                FOREIGN KEY (workflow_id) REFERENCES workflow_states (id),
                FOREIGN KEY (user_id) REFERENCES users (id)
            )
            "#,

            // Workflow drafts table
            r#"
            CREATE TABLE IF NOT EXISTS workflow_drafts (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                user_id INTEGER NOT NULL,
                draft_data TEXT NOT NULL, -- JSON
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (workflow_id) REFERENCES workflow_states (id),
                FOREIGN KEY (user_id) REFERENCES users (id)
            )
            "#,

            // Workflow step configurations table
            r#"
            CREATE TABLE IF NOT EXISTS workflow_step_configs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workflow_type TEXT NOT NULL,
                step_name TEXT NOT NULL,
                step_order INTEGER NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                component TEXT NOT NULL,
                validation_rules TEXT NOT NULL, -- JSON
                required_fields TEXT NOT NULL, -- JSON array
                optional_fields TEXT NOT NULL, -- JSON array
                created_at TEXT NOT NULL,
                UNIQUE(workflow_type, step_name)
            )
            "#,

            // Indexes for performance
            "CREATE INDEX IF NOT EXISTS idx_workflow_states_user_id ON workflow_states (user_id)",
            "CREATE INDEX IF NOT EXISTS idx_workflow_states_status ON workflow_states (status)",
            "CREATE INDEX IF NOT EXISTS idx_workflow_states_created_at ON workflow_states (created_at)",
            "CREATE INDEX IF NOT EXISTS idx_workflow_sessions_token ON workflow_sessions (session_token)",
            "CREATE INDEX IF NOT EXISTS idx_workflow_sessions_expires_at ON workflow_sessions (expires_at)",
            "CREATE INDEX IF NOT EXISTS idx_workflow_drafts_workflow_id ON workflow_drafts (workflow_id)",
            "CREATE INDEX IF NOT EXISTS idx_workflow_drafts_user_id ON workflow_drafts (user_id)",
        ]).map_err(|e| WorkflowError::Database(e.to_string()))?;

        Ok(())
    }

    /// Save workflow state
    pub async fn save_workflow_state(&self, state: &WorkflowState) -> WorkflowResult<()> {
        let db_manager = self.db_manager.lock().map_err(|e| {
            WorkflowError::Database(format!("Failed to acquire database lock: {}", e))
        })?;

        let data_json = serde_json::to_string(&state.data)?;
        
        db_manager.execute(
            r#"
            INSERT OR REPLACE INTO workflow_states 
            (id, workflow_type, current_step, user_id, status, data, created_at, updated_at, completed_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                state.id,
                state.workflow_type.to_string(),
                state.current_step.to_string(),
                state.user_id,
                workflow_status_to_string(&state.status),
                data_json,
                state.created_at.to_rfc3339(),
                state.updated_at.to_rfc3339(),
                state.completed_at.map(|dt| dt.to_rfc3339())
            ]
        ).map_err(|e| WorkflowError::Database(e.to_string()))?;

        Ok(())
    }

    /// Get workflow state by ID
    pub async fn get_workflow_state(&self, workflow_id: &str) -> WorkflowResult<Option<WorkflowState>> {
        let db_manager = self.db_manager.lock().map_err(|e| {
            WorkflowError::Database(format!("Failed to acquire database lock: {}", e))
        })?;

        let mut stmt = db_manager.prepare(
            "SELECT id, workflow_type, current_step, user_id, status, data, created_at, updated_at, completed_at 
             FROM workflow_states WHERE id = ?1"
        ).map_err(|e| WorkflowError::Database(e.to_string()))?;

        let result = stmt.query_row(params![workflow_id], |row| {
            Ok(self.row_to_workflow_state(row)?)
        });

        match result {
            Ok(state) => Ok(Some(state?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(WorkflowError::Database(e.to_string())),
        }
    }

    /// Get active workflows for a user
    pub async fn get_active_workflows_for_user(&self, user_id: i64) -> WorkflowResult<Vec<WorkflowState>> {
        let db_manager = self.db_manager.lock().map_err(|e| {
            WorkflowError::Database(format!("Failed to acquire database lock: {}", e))
        })?;

        let mut stmt = db_manager.prepare(
            "SELECT id, workflow_type, current_step, user_id, status, data, created_at, updated_at, completed_at 
             FROM workflow_states WHERE user_id = ?1 AND status IN ('Active', 'Paused') 
             ORDER BY updated_at DESC"
        ).map_err(|e| WorkflowError::Database(e.to_string()))?;

        let rows = stmt.query_map(params![user_id], |row| {
            Ok(self.row_to_workflow_state(row)?)
        }).map_err(|e| WorkflowError::Database(e.to_string()))?;

        let mut states = Vec::new();
        for row in rows {
            states.push(row.map_err(|e| WorkflowError::Database(e.to_string()))??);
        }

        Ok(states)
    }

    /// Delete workflow state
    pub async fn delete_workflow_state(&self, workflow_id: &str) -> WorkflowResult<()> {
        let db_manager = self.db_manager.lock().map_err(|e| {
            WorkflowError::Database(format!("Failed to acquire database lock: {}", e))
        })?;

        // Delete related records first
        db_manager.execute("DELETE FROM workflow_sessions WHERE workflow_id = ?1", params![workflow_id])
            .map_err(|e| WorkflowError::Database(e.to_string()))?;
        
        db_manager.execute("DELETE FROM workflow_drafts WHERE workflow_id = ?1", params![workflow_id])
            .map_err(|e| WorkflowError::Database(e.to_string()))?;

        // Delete the workflow state
        db_manager.execute("DELETE FROM workflow_states WHERE id = ?1", params![workflow_id])
            .map_err(|e| WorkflowError::Database(e.to_string()))?;

        Ok(())
    }

    /// Save workflow session
    pub async fn save_workflow_session(&self, session: &WorkflowSession) -> WorkflowResult<()> {
        let db_manager = self.db_manager.lock().map_err(|e| {
            WorkflowError::Database(format!("Failed to acquire database lock: {}", e))
        })?;

        let auto_save_json = serde_json::to_string(&session.auto_save)?;

        db_manager.execute(
            r#"
            INSERT OR REPLACE INTO workflow_sessions 
            (workflow_id, session_token, user_id, expires_at, auto_save_config, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                session.workflow_id,
                session.session_token,
                session.user_id,
                session.expires_at.to_rfc3339(),
                auto_save_json,
                session.created_at.to_rfc3339()
            ]
        ).map_err(|e| WorkflowError::Database(e.to_string()))?;

        Ok(())
    }

    /// Get workflow session by token
    pub async fn get_workflow_session_by_token(&self, token: &str) -> WorkflowResult<Option<WorkflowSession>> {
        let db_manager = self.db_manager.lock().map_err(|e| {
            WorkflowError::Database(format!("Failed to acquire database lock: {}", e))
        })?;

        let mut stmt = db_manager.prepare(
            "SELECT workflow_id, session_token, user_id, expires_at, auto_save_config, created_at 
             FROM workflow_sessions WHERE session_token = ?1"
        ).map_err(|e| WorkflowError::Database(e.to_string()))?;

        let result = stmt.query_row(params![token], |row| {
            Ok(self.row_to_workflow_session(row)?)
        });

        match result {
            Ok(session) => Ok(Some(session?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(WorkflowError::Database(e.to_string())),
        }
    }

    /// Save workflow draft
    pub async fn save_workflow_draft(&self, draft: &WorkflowDraft) -> WorkflowResult<()> {
        let db_manager = self.db_manager.lock().map_err(|e| {
            WorkflowError::Database(format!("Failed to acquire database lock: {}", e))
        })?;

        let draft_data_json = serde_json::to_string(&draft.draft_data)?;

        db_manager.execute(
            r#"
            INSERT OR REPLACE INTO workflow_drafts 
            (id, workflow_id, user_id, draft_data, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                draft.id,
                draft.workflow_id,
                draft.user_id,
                draft_data_json,
                draft.created_at.to_rfc3339(),
                draft.updated_at.to_rfc3339()
            ]
        ).map_err(|e| WorkflowError::Database(e.to_string()))?;

        Ok(())
    }

    /// Get workflow drafts for user
    pub async fn get_workflow_drafts_for_user(&self, user_id: i64) -> WorkflowResult<Vec<WorkflowDraft>> {
        let db_manager = self.db_manager.lock().map_err(|e| {
            WorkflowError::Database(format!("Failed to acquire database lock: {}", e))
        })?;

        let mut stmt = db_manager.prepare(
            "SELECT id, workflow_id, user_id, draft_data, created_at, updated_at 
             FROM workflow_drafts WHERE user_id = ?1 ORDER BY updated_at DESC"
        ).map_err(|e| WorkflowError::Database(e.to_string()))?;

        let rows = stmt.query_map(params![user_id], |row| {
            Ok(self.row_to_workflow_draft(row)?)
        }).map_err(|e| WorkflowError::Database(e.to_string()))?;

        let mut drafts = Vec::new();
        for row in rows {
            drafts.push(row.map_err(|e| WorkflowError::Database(e.to_string()))??);
        }

        Ok(drafts)
    }

    /// Delete expired sessions
    pub async fn cleanup_expired_sessions(&self) -> WorkflowResult<i64> {
        let db_manager = self.db_manager.lock().map_err(|e| {
            WorkflowError::Database(format!("Failed to acquire database lock: {}", e))
        })?;

        let now = chrono::Utc::now().to_rfc3339();
        let rows_affected = db_manager.execute(
            "DELETE FROM workflow_sessions WHERE expires_at < ?1",
            params![now]
        ).map_err(|e| WorkflowError::Database(e.to_string()))?;

        Ok(rows_affected as i64)
    }

    /// Helper method to convert database row to WorkflowState
    fn row_to_workflow_state(&self, row: &Row) -> WorkflowResult<WorkflowState> {
        let data_json: String = row.get("data")?;
        let data: WorkflowData = serde_json::from_str(&data_json)?;
        
        let workflow_type_str: String = row.get("workflow_type")?;
        let workflow_type = string_to_workflow_type(&workflow_type_str)?;
        
        let step_str: String = row.get("current_step")?;
        let current_step = string_to_workflow_step(&step_str)?;
        
        let status_str: String = row.get("status")?;
        let status = string_to_workflow_status(&status_str)?;
        
        let created_at_str: String = row.get("created_at")?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&chrono::Utc);
        
        let updated_at_str: String = row.get("updated_at")?;
        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&chrono::Utc);
        
        let completed_at = if let Ok(completed_at_str) = row.get::<_, String>("completed_at") {
            Some(chrono::DateTime::parse_from_rfc3339(&completed_at_str)?.with_timezone(&chrono::Utc))
        } else {
            None
        };

        Ok(WorkflowState {
            id: row.get("id")?,
            workflow_type,
            current_step,
            user_id: row.get("user_id")?,
            status,
            data,
            created_at,
            updated_at,
            completed_at,
        })
    }

    /// Helper method to convert database row to WorkflowSession
    fn row_to_workflow_session(&self, row: &Row) -> WorkflowResult<WorkflowSession> {
        let auto_save_json: String = row.get("auto_save_config")?;
        let auto_save: AutoSaveConfig = serde_json::from_str(&auto_save_json)?;
        
        let expires_at_str: String = row.get("expires_at")?;
        let expires_at = chrono::DateTime::parse_from_rfc3339(&expires_at_str)?.with_timezone(&chrono::Utc);
        
        let created_at_str: String = row.get("created_at")?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&chrono::Utc);

        Ok(WorkflowSession {
            workflow_id: row.get("workflow_id")?,
            session_token: row.get("session_token")?,
            user_id: row.get("user_id")?,
            expires_at,
            auto_save,
            created_at,
        })
    }

    /// Helper method to convert database row to WorkflowDraft
    fn row_to_workflow_draft(&self, row: &Row) -> WorkflowResult<WorkflowDraft> {
        let draft_data_json: String = row.get("draft_data")?;
        let draft_data: WorkflowData = serde_json::from_str(&draft_data_json)?;
        
        let created_at_str: String = row.get("created_at")?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&chrono::Utc);
        
        let updated_at_str: String = row.get("updated_at")?;
        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&chrono::Utc);

        Ok(WorkflowDraft {
            id: row.get("id")?,
            workflow_id: row.get("workflow_id")?,
            user_id: row.get("user_id")?,
            draft_data,
            created_at,
            updated_at,
        })
    }
}

// Helper functions for string conversions
fn workflow_status_to_string(status: &WorkflowStatus) -> &'static str {
    match status {
        WorkflowStatus::Active => "Active",
        WorkflowStatus::Paused => "Paused",
        WorkflowStatus::Completed => "Completed",
        WorkflowStatus::Cancelled => "Cancelled",
        WorkflowStatus::Error => "Error",
    }
}

fn string_to_workflow_status(s: &str) -> WorkflowResult<WorkflowStatus> {
    match s {
        "Active" => Ok(WorkflowStatus::Active),
        "Paused" => Ok(WorkflowStatus::Paused),
        "Completed" => Ok(WorkflowStatus::Completed),
        "Cancelled" => Ok(WorkflowStatus::Cancelled),
        "Error" => Ok(WorkflowStatus::Error),
        _ => Err(WorkflowError::InvalidState(format!("Unknown workflow status: {}", s))),
    }
}

fn string_to_workflow_type(s: &str) -> WorkflowResult<WorkflowType> {
    match s {
        "asset_creation" => Ok(WorkflowType::AssetCreation),
        _ => Err(WorkflowError::InvalidState(format!("Unknown workflow type: {}", s))),
    }
}

fn string_to_workflow_step(s: &str) -> WorkflowResult<WorkflowStepName> {
    match s {
        "asset_type_selection" => Ok(WorkflowStepName::AssetTypeSelection),
        "hierarchy_selection" => Ok(WorkflowStepName::HierarchySelection),
        "metadata_configuration" => Ok(WorkflowStepName::MetadataConfiguration),
        "security_validation" => Ok(WorkflowStepName::SecurityValidation),
        "review_confirmation" => Ok(WorkflowStepName::ReviewConfirmation),
        _ => Err(WorkflowError::InvalidState(format!("Unknown workflow step: {}", s))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabaseManager;
    use tempfile::NamedTempFile;

    async fn create_test_repository() -> WorkflowRepository {
        let temp_file = NamedTempFile::new().unwrap();
        let db_manager = DatabaseManager::new(temp_file.path().to_str().unwrap()).unwrap();
        let repo = WorkflowRepository::new(Arc::new(Mutex::new(db_manager)));
        repo.initialize_tables().await.unwrap();
        repo
    }

    #[tokio::test]
    async fn test_workflow_state_crud() {
        let repo = create_test_repository().await;
        
        let state = WorkflowState::new(
            "test-workflow".to_string(),
            WorkflowType::AssetCreation,
            1,
            None,
        );

        // Save
        repo.save_workflow_state(&state).await.unwrap();

        // Get
        let retrieved = repo.get_workflow_state("test-workflow").await.unwrap().unwrap();
        assert_eq!(retrieved.id, state.id);
        assert_eq!(retrieved.user_id, state.user_id);

        // Delete
        repo.delete_workflow_state("test-workflow").await.unwrap();
        let deleted = repo.get_workflow_state("test-workflow").await.unwrap();
        assert!(deleted.is_none());
    }

    #[test]
    fn test_string_conversions() {
        assert_eq!(workflow_status_to_string(&WorkflowStatus::Active), "Active");
        assert_eq!(string_to_workflow_status("Active").unwrap(), WorkflowStatus::Active);
        assert_eq!(string_to_workflow_type("asset_creation").unwrap(), WorkflowType::AssetCreation);
        
        assert!(string_to_workflow_status("Invalid").is_err());
        assert!(string_to_workflow_type("invalid").is_err());
    }
}