use tauri::{command, State};
use crate::database::Database;
use crate::associations::{
    AssociationRepository, SqliteAssociationRepository, CreateAssociationRequest,
    AssetFileAssociation, AssociationInfo, FileImportSession, ImportStatus,
    HealthStatus, AssociationType, ValidationResult, AssociationValidation
};
use crate::auth::SessionManager;
use anyhow::Result;
use std::sync::Mutex;

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;

#[command]
pub async fn create_file_association(
    request: CreateAssociationRequest,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>
) -> Result<AssetFileAssociation, String> {
    // Validate session
    let session_manager = session_state.lock().map_err(|_| "Failed to acquire session lock")?;
    if !session_manager.has_active_session() {
        return Err("No active session".to_string());
    }
    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Create association
    let repo = SqliteAssociationRepository::new(&conn);
    repo.create_file_association(request)
        .map_err(|e| format!("Failed to create association: {}", e))
}

#[command]
pub async fn get_asset_file_associations(
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>
) -> Result<Vec<AssociationInfo>, String> {
    // Validate session
    let session_manager = session_state.lock().map_err(|_| "Failed to acquire session lock")?;
    if !session_manager.has_active_session() {
        return Err("No active session".to_string());
    }
    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get associations
    let repo = SqliteAssociationRepository::new(&conn);
    repo.get_asset_associations(asset_id)
        .map_err(|e| format!("Failed to get associations: {}", e))
}

#[command]
pub async fn remove_file_association(
    association_id: i64,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>
) -> Result<(), String> {
    // Validate session
    let session_manager = session_state.lock().map_err(|_| "Failed to acquire session lock")?;
    if !session_manager.has_active_session() {
        return Err("No active session".to_string());
    }
    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Remove association
    let repo = SqliteAssociationRepository::new(&conn);
    repo.remove_association(association_id)
        .map_err(|e| format!("Failed to remove association: {}", e))
}

#[command]
pub async fn reorder_file_associations(
    asset_id: i64,
    association_order: Vec<(i64, i64)>,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>
) -> Result<(), String> {
    // Validate session
    let session_manager = session_state.lock().map_err(|_| "Failed to acquire session lock")?;
    if !session_manager.has_active_session() {
        return Err("No active session".to_string());
    }
    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Reorder associations
    let repo = SqliteAssociationRepository::new(&conn);
    repo.reorder_associations(asset_id, association_order)
        .map_err(|e| format!("Failed to reorder associations: {}", e))
}

#[command]
pub async fn search_associations(
    query: String,
    file_type: Option<String>,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>
) -> Result<Vec<AssociationInfo>, String> {
    // Validate session
    let session_manager = session_state.lock().map_err(|_| "Failed to acquire session lock")?;
    if !session_manager.has_active_session() {
        return Err("No active session".to_string());
    }
    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Parse file type
    let association_type = if let Some(ft) = file_type {
        Some(AssociationType::from_str(&ft).map_err(|e| format!("Invalid file type: {}", e))?)
    } else {
        None
    };

    // Search associations
    let repo = SqliteAssociationRepository::new(&conn);
    repo.search_associations(query, association_type)
        .map_err(|e| format!("Failed to search associations: {}", e))
}

#[command]
pub async fn get_association_health_status(
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>
) -> Result<HealthStatus, String> {
    // Validate session
    let session_manager = session_state.lock().map_err(|_| "Failed to acquire session lock")?;
    if !session_manager.has_active_session() {
        return Err("No active session".to_string());
    }
    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get health status
    let repo = SqliteAssociationRepository::new(&conn);
    repo.get_association_health_status(asset_id)
        .map_err(|e| format!("Failed to get health status: {}", e))
}

#[command]
pub async fn get_broken_associations(
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>
) -> Result<Vec<AssociationInfo>, String> {
    // Validate session
    let session_manager = session_state.lock().map_err(|_| "Failed to acquire session lock")?;
    if !session_manager.has_active_session() {
        return Err("No active session".to_string());
    }
    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get broken associations
    let repo = SqliteAssociationRepository::new(&conn);
    repo.get_broken_associations()
        .map_err(|e| format!("Failed to get broken associations: {}", e))
}

#[command]
pub async fn repair_association(
    association_id: i64,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>
) -> Result<(), String> {
    // Validate session
    let session_manager = session_state.lock().map_err(|_| "Failed to acquire session lock")?;
    if !session_manager.has_active_session() {
        return Err("No active session".to_string());
    }
    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Repair association
    let repo = SqliteAssociationRepository::new(&conn);
    repo.repair_association(association_id)
        .map_err(|e| format!("Failed to repair association: {}", e))
}

#[command]
pub async fn validate_file_association(
    asset_id: i64,
    file_id: i64,
    file_type: String,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>
) -> Result<Vec<AssociationValidation>, String> {
    // Validate session
    let session_manager = session_state.lock().map_err(|_| "Failed to acquire session lock")?;
    if !session_manager.has_active_session() {
        return Err("No active session".to_string());
    }
    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Parse file type
    let association_type = AssociationType::from_str(&file_type)
        .map_err(|e| format!("Invalid file type: {}", e))?;

    // Validate association
    let repo = SqliteAssociationRepository::new(&conn);
    repo.validate_file_association(asset_id, file_id, &association_type)
        .map_err(|e| format!("Failed to validate association: {}", e))
}

#[command]
pub async fn create_import_session(
    session_name: String,
    asset_id: i64,
    file_paths: Vec<String>,
    created_by: i64,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>
) -> Result<FileImportSession, String> {
    // Validate session
    let session_manager = session_state.lock().map_err(|_| "Failed to acquire session lock")?;
    if !session_manager.has_active_session() {
        return Err("No active session".to_string());
    }
    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Create import session
    let repo = SqliteAssociationRepository::new(&conn);
    repo.create_import_session(session_name, asset_id, file_paths, created_by)
        .map_err(|e| format!("Failed to create import session: {}", e))
}

#[command]
pub async fn update_import_session_status(
    session_id: i64,
    status: String,
    validation_results: Option<String>,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>
) -> Result<(), String> {
    // Validate session
    let session_manager = session_state.lock().map_err(|_| "Failed to acquire session lock")?;
    if !session_manager.has_active_session() {
        return Err("No active session".to_string());
    }
    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Parse status
    let import_status = ImportStatus::from_str(&status)
        .map_err(|e| format!("Invalid import status: {}", e))?;

    // Update session status
    let repo = SqliteAssociationRepository::new(&conn);
    repo.update_import_session_status(session_id, import_status, validation_results)
        .map_err(|e| format!("Failed to update import session: {}", e))
}

#[command]
pub async fn get_import_session(
    session_id: i64,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>
) -> Result<Option<FileImportSession>, String> {
    // Validate session
    let session_manager = session_state.lock().map_err(|_| "Failed to acquire session lock")?;
    if !session_manager.has_active_session() {
        return Err("No active session".to_string());
    }
    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Get import session
    let repo = SqliteAssociationRepository::new(&conn);
    repo.get_import_session(session_id)
        .map_err(|e| format!("Failed to get import session: {}", e))
}

#[command]
pub async fn get_associations_by_validation_status(
    status: String,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>
) -> Result<Vec<AssociationInfo>, String> {
    // Validate session
    let session_manager = session_state.lock().map_err(|_| "Failed to acquire session lock")?;
    if !session_manager.has_active_session() {
        return Err("No active session".to_string());
    }
    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.get_connection();

    // Parse validation status
    let validation_result = ValidationResult::from_str(&status)
        .map_err(|e| format!("Invalid validation status: {}", e))?;

    // Get associations by validation status
    let repo = SqliteAssociationRepository::new(&conn);
    repo.get_associations_by_validation_status(validation_result)
        .map_err(|e| format!("Failed to get associations by validation status: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::users::{SqliteUserRepository, UserRepository};
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
    use tauri::Manager;

    fn setup_test_db() -> Database {
        let db = Database::new(":memory:".into()).unwrap();
        let conn = db.get_connection();

        // Seed a user, an asset, and one configuration and firmware file to associate
        conn.execute_batch(
            r#"
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'testuser', 'hash', 'Engineer');
            INSERT INTO assets (id, name, description, created_by) VALUES (1, 'Test Asset', 'Test Description', 1);
            INSERT INTO configuration_versions (id, asset_id, version_number, file_name, file_content, file_size, content_hash, author)
                VALUES (1, 1, 'v1', 'config.json', 'test', 4, 'hash1', 1);
            INSERT INTO firmware_versions (id, asset_id, author_id, version, status, file_path, file_hash, file_size)
                VALUES (1, 1, 1, '1.0.0', 'Draft', '/test/path', 'hash2', 1024);
            "#,
        ).unwrap();

        // Associations schema isn't part of Database::initialize_schema()
        let repo = SqliteAssociationRepository::new(conn);
        repo.initialize_schema().unwrap();

        db
    }

    /// Builds a mock Tauri app managing the database and a session manager with an active session.
    fn setup_test_app() -> tauri::App<MockRuntime> {
        let db = setup_test_db();

        let session_manager = SessionManager::new();
        let user = SqliteUserRepository::new(db.get_connection())
            .find_by_id(1)
            .unwrap()
            .unwrap();
        session_manager.create_session(&user).unwrap();

        mock_builder()
            .manage(DatabaseState::new(Some(db)))
            .manage(SessionManagerState::new(session_manager))
            .build(mock_context(noop_assets()))
            .unwrap()
    }

    #[tokio::test]
    async fn test_create_file_association_command() {
        let app = setup_test_app();

        let request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Configuration,
            metadata: Some("Test association".to_string()),
            created_by: 1,
        };

        let result = create_file_association(
            request,
            app.state(),
            app.state(),
        ).await;

        assert!(result.is_ok());
        let association = result.unwrap();
        assert_eq!(association.asset_id, 1);
        assert_eq!(association.file_id, 1);
        assert_eq!(association.file_type, AssociationType::Configuration);
    }

    #[tokio::test]
    async fn test_get_asset_file_associations_command() {
        let app = setup_test_app();

        // First create an association
        let request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Configuration,
            metadata: Some("Test association".to_string()),
            created_by: 1,
        };

        create_file_association(
            request,
            app.state(),
            app.state(),
        ).await.unwrap();

        // Now get associations
        let result = get_asset_file_associations(
            1,
            app.state(),
            app.state(),
        ).await;

        assert!(result.is_ok());
        let associations = result.unwrap();
        assert_eq!(associations.len(), 1);
        assert_eq!(associations[0].asset_id, 1);
    }

    #[tokio::test]
    async fn test_search_associations_command() {
        let app = setup_test_app();

        // First create an association
        let request = CreateAssociationRequest {
            asset_id: 1,
            file_id: 1,
            file_type: AssociationType::Configuration,
            metadata: Some("Test association".to_string()),
            created_by: 1,
        };

        create_file_association(
            request,
            app.state(),
            app.state(),
        ).await.unwrap();

        // Search associations
        let result = search_associations(
            "Test".to_string(),
            Some("configuration".to_string()),
            app.state(),
            app.state(),
        ).await;

        assert!(result.is_ok());
        let associations = result.unwrap();
        assert_eq!(associations.len(), 1);
    }

    #[tokio::test]
    async fn test_get_health_status_command() {
        let app = setup_test_app();

        let result = get_association_health_status(
            1,
            app.state(),
            app.state(),
        ).await;

        assert!(result.is_ok());
        let health = result.unwrap();
        assert!(health.healthy);
        assert!(health.issues.is_empty());
    }
}