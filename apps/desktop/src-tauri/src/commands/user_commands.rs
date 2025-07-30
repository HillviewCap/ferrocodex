use crate::auth::{SessionManager};
use crate::users::{UserRepository, SqliteUserRepository, UserRole, UserInfo};
use crate::audit::{AuditRepository, SqliteAuditRepository, create_user_created_event, create_user_deactivated_event, create_user_reactivated_event};
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
    let initial_password = InputSanitizer::sanitize_string(&initial_password);

    // Check for malicious input
    if InputSanitizer::is_potentially_malicious(&username) || InputSanitizer::is_potentially_malicious(&initial_password) {
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

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let user_repo = SqliteUserRepository::new(db.get_connection());
            
            match user_repo.create_engineer_account(username, initial_password, session.user_id) {
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
    match db_guard.as_ref() {
        Some(db) => {
            let user_repo = SqliteUserRepository::new(db.get_connection());
            
            // Get the target user info before deactivation
            let target_user = match user_repo.find_by_id(user_id) {
                Ok(Some(user)) => user,
                Ok(None) => return Err("User not found".to_string()),
                Err(e) => return Err(format!("Failed to find user: {}", e)),
            };
            
            match user_repo.deactivate_user(user_id) {
                Ok(_) => {
                    // Log audit event
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

                    info!("User {} deactivated by admin: {}", target_user.username, session.username);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to deactivate user: {}", e);
                    Err(format!("Failed to deactivate user: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn reactivate_user(
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
        warn!("Non-admin user attempted to reactivate user: {}", session.username);
        return Err("Only administrators can reactivate users".to_string());
    }

    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let user_repo = SqliteUserRepository::new(db.get_connection());
            
            // Get the target user info before reactivation
            let target_user = match user_repo.find_by_id(user_id) {
                Ok(Some(user)) => user,
                Ok(None) => return Err("User not found".to_string()),
                Err(e) => return Err(format!("Failed to find user: {}", e)),
            };
            
            match user_repo.reactivate_user(user_id) {
                Ok(_) => {
                    // Log audit event
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
                Err(e) => {
                    error!("Failed to reactivate user: {}", e);
                    Err(format!("Failed to reactivate user: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}