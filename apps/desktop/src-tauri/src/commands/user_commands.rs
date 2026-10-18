use crate::auth::{hash_password, SessionManager};
use crate::encryption::DataKey;
use crate::keystore::{KeyStoreRepository, SqliteKeyStoreRepository};
use crate::users::{UserRepository, SqliteUserRepository, UserRole, UserInfo};
use crate::audit::{AuditRepository, SqliteAuditRepository, AuditEventRequest, AuditEventType, create_user_created_event, create_user_deactivated_event, create_user_reactivated_event};
use crate::validation::{UsernameValidator, PasswordValidator, InputSanitizer, RateLimiter};
use crate::database::Database;
use std::sync::Mutex;
use tauri::State;
use tracing::{error, info, warn};

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;
type RateLimiterState = Mutex<RateLimiter>;

#[tauri::command]
pub async fn create_engineer_user(
    token: String,
    username: String,
    initial_password: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    rate_limiter: State<'_, RateLimiterState>,
) -> Result<UserInfo, String> {
    // Check rate limiting
    let rate_limiter_guard = rate_limiter.lock()
        .map_err(|_| "Failed to acquire rate limiter lock".to_string())?;
    if let Err(e) = rate_limiter_guard.check_rate_limit(&format!("create_user_{}", token)) {
        return Err(e);
    }
    drop(rate_limiter_guard);

    // Validate and sanitize inputs
    let username = InputSanitizer::sanitize_username(&username);
    // Passwords are opaque: never trimmed, stripped, or run through the injection heuristic.

    // Check for malicious input
    if InputSanitizer::is_potentially_malicious(&username) {
        error!("Potentially malicious input detected in create_engineer_user");
        return Err("Invalid input detected".to_string());
    }

    // Validate username
    if let Err(e) = UsernameValidator::validate(&username) {
        return Err(e.to_string());
    }

    // Validate password
    if let Err(e) = PasswordValidator::validate(&initial_password) {
        return Err(e.to_string());
    }

    // Validate session and get user info
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    // Ensure user is an Administrator
    if session.role != UserRole::Administrator {
        warn!("Non-admin user attempted to create engineer account: {}", session.username);
        return Err("Only administrators can create engineer accounts".to_string());
    }

    drop(session_manager_guard);

    // The new account gets its own copy of the data key, wrapped under its password
    let data_key = session.data_key()?;

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let user_repo = SqliteUserRepository::new(db.get_connection());
            let key_store = SqliteKeyStoreRepository::new(db.get_connection());

            let tx = db.get_connection().unchecked_transaction()
                .map_err(|e| format!("Failed to start transaction: {}", e))?;
            let created = user_repo.create_engineer_account(username, initial_password.clone(), session.user_id)
                .and_then(|user| {
                    key_store.grant_user(user.id, &initial_password, &data_key)?;
                    tx.commit()?;
                    Ok(user)
                });

            match created {
                Ok(user) => {
                    // Log audit event
                    let audit_repo = SqliteAuditRepository::new(db.get_connection());
                    let audit_event = create_user_created_event(
                        session.user_id,
                        &session.username,
                        user.id,
                        &user.username,
                    );
                    if let Err(e) = audit_repo.log_event(&audit_event) {
                        error!("Failed to log audit event: {}", e);
                    }

                    info!("Engineer account created for user: {} by admin: {}", user.username, session.username);
                    Ok(user.into())
                }
                Err(e) => {
                    error!("Failed to create engineer account: {}", e);
                    Err(format!("Failed to create engineer account: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn list_users(
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<UserInfo>, String> {
    // Validate session and get user info
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    // Ensure user is an Administrator
    if session.role != UserRole::Administrator {
        warn!("Non-admin user attempted to list users: {}", session.username);
        return Err("Only administrators can list users".to_string());
    }

    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let user_repo = SqliteUserRepository::new(db.get_connection());
            
            match user_repo.list_all_users() {
                Ok(users) => {
                    info!("User list accessed by admin: {}", session.username);
                    Ok(users.into_iter().map(|u| u.into()).collect())
                }
                Err(e) => {
                    error!("Failed to list users: {}", e);
                    Err(format!("Failed to list users: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn deactivate_user(
    token: String,
    user_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session and get user info
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    // Ensure user is an Administrator
    if session.role != UserRole::Administrator {
        warn!("Non-admin user attempted to deactivate user: {}", session.username);
        return Err("Only administrators can deactivate users".to_string());
    }

    // Prevent self-deactivation
    if session.user_id == user_id {
        return Err("Administrators cannot deactivate their own account".to_string());
    }

    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;

    let user_repo = SqliteUserRepository::new(db.get_connection());
    let key_store = SqliteKeyStoreRepository::new(db.get_connection());

    // Get the target user info before deactivation
    let target_user = match user_repo.find_by_id(user_id) {
        Ok(Some(user)) => user,
        Ok(None) => return Err("User not found".to_string()),
        Err(e) => return Err(format!("Failed to find user: {}", e)),
    };

    // Deactivate and revoke the key wrap atomically: a deactivated user must not
    // keep a password that still opens the data key.
    let result = db.get_connection().unchecked_transaction()
        .map_err(anyhow::Error::from)
        .and_then(|tx| {
            user_repo.deactivate_user(user_id)?;
            key_store.revoke_user(user_id)?;
            tx.commit()?;
            Ok(())
        });
    if let Err(e) = result {
        error!("Failed to deactivate user: {}", e);
        return Err(format!("Failed to deactivate user: {}", e));
    }

    // Flush the deleted wrap out of the WAL file
    if let Err(e) = db.checkpoint_wal() {
        warn!("Failed to checkpoint WAL after revoking key access: {}", e);
    }

    let audit_repo = SqliteAuditRepository::new(db.get_connection());
    let audit_event = create_user_deactivated_event(
        session.user_id,
        &session.username,
        user_id,
        &target_user.username,
    );
    if let Err(e) = audit_repo.log_event(&audit_event) {
        error!("Failed to log audit event: {}", e);
    }
    drop(db_guard);

    // End the user's live sessions: they hold the unwrapped key in memory
    session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?
        .invalidate_user_sessions(user_id);

    info!("User {} deactivated by admin: {}", target_user.username, session.username);
    Ok(())
}

/// Sets `new_password` for `user_id` and gives them a fresh wrapped copy of the
/// data key, in one transaction. With `reactivate`, also marks the account active.
fn set_password_and_grant_key(
    db: &Database,
    user_id: i64,
    new_password: &str,
    data_key: &DataKey,
    reactivate: bool,
) -> Result<crate::users::User, String> {
    let user_repo = SqliteUserRepository::new(db.get_connection());
    let key_store = SqliteKeyStoreRepository::new(db.get_connection());

    let mut user = match user_repo.find_by_id(user_id) {
        Ok(Some(user)) => user,
        Ok(None) => return Err("User not found".to_string()),
        Err(e) => return Err(format!("Failed to find user: {}", e)),
    };
    if reactivate && user.is_active {
        return Err("User is already active".to_string());
    }
    if !reactivate && !user.is_active {
        return Err("User is deactivated; reactivate the account instead".to_string());
    }

    user.password_hash = hash_password(new_password)
        .map_err(|e| format!("Failed to hash password: {}", e))?;

    let tx = db.get_connection().unchecked_transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;
    if reactivate {
        user_repo.reactivate_user(user_id)
            .map_err(|e| format!("Failed to reactivate user: {}", e))?;
    }
    user_repo.update_user(&crate::users::User { is_active: true, ..user.clone() })
        .map_err(|e| format!("Failed to update password: {}", e))?;
    key_store.grant_user(user_id, new_password, data_key)
        .map_err(|e| format!("Failed to update encryption key: {}", e))?;
    tx.commit()
        .map_err(|e| format!("Failed to save changes: {}", e))?;

    user.is_active = true;
    Ok(user)
}

fn password_audit_event(admin_id: i64, admin_name: &str, target: &crate::users::User, description: String) -> AuditEventRequest {
    AuditEventRequest {
        event_type: AuditEventType::UserPasswordChanged,
        user_id: Some(admin_id),
        username: Some(admin_name.to_string()),
        admin_user_id: Some(admin_id),
        admin_username: Some(admin_name.to_string()),
        target_user_id: Some(target.id),
        target_username: Some(target.username.clone()),
        description,
        metadata: None,
        ip_address: None,
        user_agent: None,
    }
}

#[tauri::command]
pub async fn reactivate_user(
    token: String,
    user_id: i64,
    new_password: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Passwords are opaque: validated for strength only, never sanitized or logged.
    if let Err(e) = PasswordValidator::validate(&new_password) {
        return Err(e.to_string());
    }

    // Validate session and get user info
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    // Ensure user is an Administrator
    if session.role != UserRole::Administrator {
        warn!("Non-admin user attempted to reactivate user: {}", session.username);
        return Err("Only administrators can reactivate users".to_string());
    }

    drop(session_manager_guard);

    // The reactivated account gets its own copy of the data key under the new password
    let data_key = session.data_key()?;

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;

    let target_user = set_password_and_grant_key(db, user_id, &new_password, &data_key, true)
        .map_err(|e| {
            error!("Failed to reactivate user: {}", e);
            e
        })?;

    let audit_repo = SqliteAuditRepository::new(db.get_connection());
    let audit_event = create_user_reactivated_event(
        session.user_id,
        &session.username,
        user_id,
        &target_user.username,
    );
    if let Err(e) = audit_repo.log_event(&audit_event) {
        error!("Failed to log audit event: {}", e);
    }

    info!("User {} reactivated by admin: {}", target_user.username, session.username);
    Ok(())
}

/// Sets a temporary password for an active user and gives them a fresh wrapped
/// copy of the data key. Admin only, and never for the admin's own account.
#[tauri::command]
pub async fn admin_reset_user_password(
    token: String,
    user_id: i64,
    new_password: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    if let Err(e) = PasswordValidator::validate(&new_password) {
        return Err(e.to_string());
    }

    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    if session.role != UserRole::Administrator {
        warn!("Non-admin user attempted to reset a user password: {}", session.username);
        return Err("Only administrators can reset user passwords".to_string());
    }

    if session.user_id == user_id {
        return Err("Administrators cannot reset their own password this way".to_string());
    }

    drop(session_manager_guard);

    let data_key = session.data_key()?;

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;

    let target_user = set_password_and_grant_key(db, user_id, &new_password, &data_key, false)
        .map_err(|e| {
            error!("Failed to reset user password: {}", e);
            e
        })?;

    let audit_repo = SqliteAuditRepository::new(db.get_connection());
    let audit_event = password_audit_event(
        session.user_id,
        &session.username,
        &target_user,
        format!(
            "Administrator '{}' set a temporary password for '{}'; encryption key access re-granted",
            session.username, target_user.username
        ),
    );
    if let Err(e) = audit_repo.log_event(&audit_event) {
        error!("Failed to log audit event: {}", e);
    }
    drop(db_guard);

    // Old sessions were opened under the old password
    session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?
        .invalidate_user_sessions(user_id);

    info!("Password for user {} reset by admin: {}", target_user.username, session.username);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{LoginAttemptTracker, LoginResponse};
    use crate::commands::{create_admin_account, login};
    use crate::encryption::FileEncryption;
    use crate::keystore::KeyStoreRepository;
    use std::sync::Arc;
    use std::time::Duration;
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
    use tauri::Manager;

    type LoginAttemptTrackerState = Mutex<LoginAttemptTracker>;

    const ADMIN_PASSWORD: &str = "Admin-Pass-123";
    const ENGINEER_PASSWORD: &str = "Engineer-Pass-123";
    const TEMP_PASSWORD: &str = "Temp-Pass-456";

    fn setup_test_app() -> tauri::App<MockRuntime> {
        mock_builder()
            .manage(DatabaseState::new(Some(Database::new(":memory:".into()).unwrap())))
            .manage(SessionManagerState::new(SessionManager::new()))
            .manage(LoginAttemptTrackerState::new(LoginAttemptTracker::new()))
            .manage(RateLimiterState::new(RateLimiter::new(100, Duration::from_secs(60))))
            .build(mock_context(noop_assets()))
            .unwrap()
    }

    fn same_key(a: &DataKey, b: &DataKey) -> bool {
        let ciphertext = FileEncryption::new(a).encrypt(b"probe").unwrap();
        FileEncryption::new(b).decrypt(&ciphertext).is_ok()
    }

    fn session_key(app: &tauri::App<MockRuntime>, token: &str) -> Arc<DataKey> {
        app.state::<SessionManagerState>().lock().unwrap()
            .validate_session(token).unwrap().unwrap()
            .data_key().unwrap()
    }

    fn session_exists(app: &tauri::App<MockRuntime>, token: &str) -> bool {
        app.state::<SessionManagerState>().lock().unwrap()
            .validate_session(token).unwrap().is_some()
    }

    async fn login_as(app: &tauri::App<MockRuntime>, username: &str, password: &str) -> Result<LoginResponse, String> {
        login(username.to_string(), password.to_string(), app.state(), app.state(), app.state()).await
    }

    /// An admin plus an engineer; returns (admin login, engineer id).
    async fn setup_admin_and_engineer(app: &tauri::App<MockRuntime>) -> (LoginResponse, i64) {
        let admin = create_admin_account("admin".to_string(), ADMIN_PASSWORD.to_string(), app.state(), app.state())
            .await.unwrap();
        let engineer = create_engineer_user(
            admin.token.clone(), "engineer".to_string(), ENGINEER_PASSWORD.to_string(),
            app.state(), app.state(), app.state(),
        ).await.unwrap();
        (admin, engineer.id)
    }

    fn unlock(app: &tauri::App<MockRuntime>, user_id: i64, password: &str) -> anyhow::Result<Option<DataKey>> {
        let db_state = app.state::<DatabaseState>();
        let db_guard = db_state.lock().unwrap();
        SqliteKeyStoreRepository::new(db_guard.as_ref().unwrap().get_connection())
            .unlock_for_user(user_id, password)
    }

    #[tokio::test]
    async fn test_deactivation_revokes_key_and_ends_sessions() {
        let app = setup_test_app();
        let (admin, engineer_id) = setup_admin_and_engineer(&app).await;
        let engineer = login_as(&app, "engineer", ENGINEER_PASSWORD).await.unwrap();
        let engineer_second = login_as(&app, "engineer", ENGINEER_PASSWORD).await.unwrap();
        assert!(session_exists(&app, &engineer.token));

        deactivate_user(admin.token.clone(), engineer_id, app.state(), app.state()).await.unwrap();

        assert!(unlock(&app, engineer_id, ENGINEER_PASSWORD).unwrap().is_none());
        assert!(!session_exists(&app, &engineer.token));
        assert!(!session_exists(&app, &engineer_second.token));
        assert!(session_exists(&app, &admin.token));
        assert!(login_as(&app, "engineer", ENGINEER_PASSWORD).await.is_err());
    }

    #[tokio::test]
    async fn test_reactivation_with_new_password_yields_same_key() {
        let app = setup_test_app();
        let (admin, engineer_id) = setup_admin_and_engineer(&app).await;
        let admin_key = session_key(&app, &admin.token);
        deactivate_user(admin.token.clone(), engineer_id, app.state(), app.state()).await.unwrap();

        // Weak passwords are refused and nothing changes
        assert!(reactivate_user(admin.token.clone(), engineer_id, "weak".to_string(), app.state(), app.state())
            .await.is_err());

        reactivate_user(admin.token.clone(), engineer_id, TEMP_PASSWORD.to_string(), app.state(), app.state())
            .await.unwrap();

        assert!(login_as(&app, "engineer", ENGINEER_PASSWORD).await.is_err());
        let relogin = login_as(&app, "engineer", TEMP_PASSWORD).await.unwrap();
        assert!(same_key(&admin_key, &session_key(&app, &relogin.token)));
        let unlocked = unlock(&app, engineer_id, TEMP_PASSWORD).unwrap().unwrap();
        assert!(same_key(&admin_key, &unlocked));
    }

    #[tokio::test]
    async fn test_admin_reset_restores_access_and_ends_old_sessions() {
        let app = setup_test_app();
        let (admin, engineer_id) = setup_admin_and_engineer(&app).await;
        let admin_key = session_key(&app, &admin.token);
        let engineer = login_as(&app, "engineer", ENGINEER_PASSWORD).await.unwrap();

        admin_reset_user_password(admin.token.clone(), engineer_id, TEMP_PASSWORD.to_string(), app.state(), app.state())
            .await.unwrap();

        assert!(!session_exists(&app, &engineer.token));
        assert!(login_as(&app, "engineer", ENGINEER_PASSWORD).await.is_err());
        let relogin = login_as(&app, "engineer", TEMP_PASSWORD).await.unwrap();
        assert!(same_key(&admin_key, &session_key(&app, &relogin.token)));
    }

    #[tokio::test]
    async fn test_engineer_cannot_admin_reset_or_reactivate() {
        let app = setup_test_app();
        let (admin, engineer_id) = setup_admin_and_engineer(&app).await;
        let engineer = login_as(&app, "engineer", ENGINEER_PASSWORD).await.unwrap();

        let result = admin_reset_user_password(
            engineer.token.clone(), engineer_id, TEMP_PASSWORD.to_string(), app.state(), app.state(),
        ).await;
        assert_eq!(result, Err("Only administrators can reset user passwords".to_string()));

        let result = reactivate_user(
            engineer.token.clone(), engineer_id, TEMP_PASSWORD.to_string(), app.state(), app.state(),
        ).await;
        assert_eq!(result, Err("Only administrators can reactivate users".to_string()));

        let result = deactivate_user(engineer.token.clone(), 1, app.state(), app.state()).await;
        assert!(result.is_err());
        assert!(login_as(&app, "engineer", ENGINEER_PASSWORD).await.is_ok());
        assert!(session_exists(&app, &admin.token));
    }

    #[tokio::test]
    async fn test_admin_cannot_reset_own_password() {
        let app = setup_test_app();
        let (admin, _engineer_id) = setup_admin_and_engineer(&app).await;
        let admin_id = session_user_id(&app, &admin.token);

        let result = admin_reset_user_password(
            admin.token.clone(), admin_id, TEMP_PASSWORD.to_string(), app.state(), app.state(),
        ).await;

        assert_eq!(result, Err("Administrators cannot reset their own password this way".to_string()));
        assert!(login_as(&app, "admin", ADMIN_PASSWORD).await.is_ok());
    }

    #[tokio::test]
    async fn test_admin_reset_refuses_inactive_user_and_reactivate_refuses_active_user() {
        let app = setup_test_app();
        let (admin, engineer_id) = setup_admin_and_engineer(&app).await;

        assert!(reactivate_user(admin.token.clone(), engineer_id, TEMP_PASSWORD.to_string(), app.state(), app.state())
            .await.is_err());
        deactivate_user(admin.token.clone(), engineer_id, app.state(), app.state()).await.unwrap();
        assert!(admin_reset_user_password(admin.token.clone(), engineer_id, TEMP_PASSWORD.to_string(), app.state(), app.state())
            .await.is_err());
        // Still revoked
        assert!(unlock(&app, engineer_id, TEMP_PASSWORD).unwrap().is_none());
    }

    fn session_user_id(app: &tauri::App<MockRuntime>, token: &str) -> i64 {
        app.state::<SessionManagerState>().lock().unwrap()
            .validate_session(token).unwrap().unwrap().user_id
    }
}
