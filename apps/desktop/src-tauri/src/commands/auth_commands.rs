use crate::auth::{SessionManager, LoginAttemptTracker, LoginResponse, verify_password};
use crate::users::{CreateUserRequest, UserRepository, SqliteUserRepository, UserRole, UserInfo};
use crate::audit::{AuditRepository, SqliteAuditRepository, create_user_created_event};
use crate::validation::{UsernameValidator, PasswordValidator, InputSanitizer, RateLimiter};
use crate::database::Database;
use std::sync::Mutex;
use tauri::State;
use tracing::{error, info, warn};

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;
type LoginAttemptTrackerState = Mutex<LoginAttemptTracker>;
type RateLimiterState = Mutex<RateLimiter>;

#[tauri::command]
pub async fn create_admin_account(
    username: String,
    password: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<LoginResponse, String> {
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let user_repo = SqliteUserRepository::new(db.get_connection());
            
            // Check if admin users already exist
            let has_admins = user_repo.has_admin_users()
                .map_err(|e| format!("Failed to check for admin users: {}", e))?;
            
            if has_admins {
                return Err("Admin account already exists".to_string());
            }

            let request = CreateUserRequest {
                username,
                password,
                role: UserRole::Administrator,
            };

            let user = user_repo.create_user(request)
                .map_err(|e| format!("Failed to create admin account: {}", e))?;

            let session_manager = session_manager.lock()
                .map_err(|_| "Failed to acquire session manager lock".to_string())?;
            let session = session_manager.create_session(&user)
                .map_err(|e| format!("Failed to create session: {}", e))?;

            info!("Created admin account for user: {}", user.username);

            Ok(LoginResponse {
                token: session.token,
                user: user.into(),
            })
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn login(
    username: String,
    password: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    attempt_tracker: State<'_, LoginAttemptTrackerState>,
) -> Result<LoginResponse, String> {
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            // Check if account is locked
            let tracker = attempt_tracker.lock()
                .map_err(|_| "Failed to acquire attempt tracker lock".to_string())?;
            if tracker.is_locked(&username).map_err(|e| e.to_string())? {
                warn!("Login attempt for locked account: {}", username);
                return Err("Account is temporarily locked due to too many failed attempts".to_string());
            }
            drop(tracker);

            let user_repo = SqliteUserRepository::new(db.get_connection());
            
            match user_repo.find_by_username(&username) {
                Ok(Some(user)) => {
                    match verify_password(&password, &user.password_hash) {
                        Ok(true) => {
                            // Successful login
                            let tracker = attempt_tracker.lock()
                                .map_err(|_| "Failed to acquire attempt tracker lock".to_string())?;
                            tracker.record_successful_attempt(&username)
                                .map_err(|e| e.to_string())?;
                            drop(tracker);

                            let session_manager = session_manager.lock()
                                .map_err(|_| "Failed to acquire session manager lock".to_string())?;
                            let session = session_manager.create_session(&user)
                                .map_err(|e| format!("Failed to create session: {}", e))?;

                            info!("Successful login for user: {}", username);

                            Ok(LoginResponse {
                                token: session.token,
                                user: user.into(),
                            })
                        }
                        Ok(false) => {
                            // Wrong password
                            let tracker = attempt_tracker.lock()
                                .map_err(|_| "Failed to acquire attempt tracker lock".to_string())?;
                            tracker.record_failed_attempt(&username)
                                .map_err(|e| e.to_string())?;
                            warn!("Invalid password for user: {}", username);
                            Err("Invalid credentials".to_string())
                        }
                        Err(e) => {
                            error!("Password verification error: {}", e);
                            Err("Authentication error".to_string())
                        }
                    }
                }
                Ok(None) => {
                    // User not found
                    let tracker = attempt_tracker.lock()
                        .map_err(|_| "Failed to acquire attempt tracker lock".to_string())?;
                    tracker.record_failed_attempt(&username)
                        .map_err(|e| e.to_string())?;
                    warn!("Login attempt for non-existent user: {}", username);
                    Err("Invalid credentials".to_string())
                }
                Err(e) => {
                    error!("Database error during login: {}", e);
                    Err("Authentication error".to_string())
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn logout(
    token: String,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    let session_manager = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    session_manager.invalidate_session(&token)
        .map_err(|e| format!("Failed to logout: {}", e))?;
    
    info!("User logged out");
    Ok(())
}

#[tauri::command]
pub async fn check_session(
    token: String,
    session_manager: State<'_, SessionManagerState>,
) -> Result<UserInfo, String> {
    let session_manager = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    match session_manager.validate_session(&token) {
        Ok(Some(session)) => {
            Ok(UserInfo {
                id: session.user_id,
                username: session.username,
                role: session.role,
                created_at: "".to_string(), // We don't store this in session
                is_active: true,
            })
        }
        Ok(None) => Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            Err("Session validation error".to_string())
        }
    }
}