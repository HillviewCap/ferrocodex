mod database;
mod users;
mod auth;
mod audit;
mod validation;

use database::Database;
use users::{CreateUserRequest, UserRepository, SqliteUserRepository, UserRole, UserInfo};
use auth::{SessionManager, LoginAttemptTracker, LoginResponse, verify_password};
use audit::{AuditRepository, SqliteAuditRepository, create_user_created_event, create_user_deactivated_event, create_user_reactivated_event};
use validation::{UsernameValidator, PasswordValidator, InputSanitizer, RateLimiter};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};
use tracing::{error, info, warn};

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;
type LoginAttemptTrackerState = Mutex<LoginAttemptTracker>;
type RateLimiterState = Mutex<RateLimiter>;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn initialize_database(app: AppHandle, db_state: State<'_, DatabaseState>) -> Result<bool, String> {
    info!("Initializing database...");
    
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    let db_path = app_data_dir.join("ferrocodex.db");
    
    match Database::new(db_path) {
        Ok(database) => {
            let health_check = database.health_check()
                .map_err(|e| format!("Database health check failed: {}", e))?;
            
            if health_check {
                *db_state.lock().unwrap() = Some(database);
                info!("Database initialized successfully");
                Ok(true)
            } else {
                error!("Database health check failed");
                Err("Database health check failed".to_string())
            }
        }
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            Err(format!("Failed to initialize database: {}", e))
        }
    }
}

#[tauri::command]
async fn database_health_check(db_state: State<'_, DatabaseState>) -> Result<bool, String> {
    let db_guard = db_state.lock().unwrap();
    match db_guard.as_ref() {
        Some(db) => db.health_check().map_err(|e| e.to_string()),
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn is_first_launch(db_state: State<'_, DatabaseState>) -> Result<bool, String> {
    let db_guard = db_state.lock().unwrap();
    match db_guard.as_ref() {
        Some(db) => {
            let user_repo = SqliteUserRepository::new(db.get_connection());
            let has_admins = user_repo.has_admin_users()
                .map_err(|e| format!("Failed to check for admin users: {}", e))?;
            Ok(!has_admins)
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn create_admin_account(
    username: String,
    password: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<LoginResponse, String> {
    let db_guard = db_state.lock().unwrap();
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

            let session_manager = session_manager.lock().unwrap();
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
async fn login(
    username: String,
    password: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    attempt_tracker: State<'_, LoginAttemptTrackerState>,
) -> Result<LoginResponse, String> {
    let db_guard = db_state.lock().unwrap();
    match db_guard.as_ref() {
        Some(db) => {
            // Check if account is locked
            let tracker = attempt_tracker.lock().unwrap();
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
                            let tracker = attempt_tracker.lock().unwrap();
                            tracker.record_successful_attempt(&username)
                                .map_err(|e| e.to_string())?;
                            drop(tracker);

                            let session_manager = session_manager.lock().unwrap();
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
                            let tracker = attempt_tracker.lock().unwrap();
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
                    let tracker = attempt_tracker.lock().unwrap();
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
async fn logout(
    token: String,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    let session_manager = session_manager.lock().unwrap();
    session_manager.invalidate_session(&token)
        .map_err(|e| format!("Failed to logout: {}", e))?;
    
    info!("User logged out");
    Ok(())
}

#[tauri::command]
async fn check_session(
    token: String,
    session_manager: State<'_, SessionManagerState>,
) -> Result<UserInfo, String> {
    let session_manager = session_manager.lock().unwrap();
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

#[tauri::command]
async fn create_engineer_user(
    token: String,
    username: String,
    initial_password: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    rate_limiter: State<'_, RateLimiterState>,
) -> Result<UserInfo, String> {
    // Check rate limiting
    let rate_limiter_guard = rate_limiter.lock().unwrap();
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
    let session_manager_guard = session_manager.lock().unwrap();
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

    let db_guard = db_state.lock().unwrap();
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

                    info!("Engineer account created by admin {}: {}", session.username, user.username);
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
async fn list_users(
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<UserInfo>, String> {
    // Validate session and get user info
    let session_manager_guard = session_manager.lock().unwrap();
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

    let db_guard = db_state.lock().unwrap();
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
async fn deactivate_user(
    token: String,
    user_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session and get user info
    let session_manager_guard = session_manager.lock().unwrap();
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

    let db_guard = db_state.lock().unwrap();
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

                    info!("User {} deactivated by admin: {}", user_id, session.username);
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
async fn reactivate_user(
    token: String,
    user_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session and get user info
    let session_manager_guard = session_manager.lock().unwrap();
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

    let db_guard = db_state.lock().unwrap();
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

                    info!("User {} reactivated by admin: {}", user_id, session.username);
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(DatabaseState::default())
        .manage(SessionManagerState::default())
        .manage(LoginAttemptTrackerState::default())
        .manage(RateLimiterState::new(RateLimiter::new(10, Duration::from_secs(60))))
        .invoke_handler(tauri::generate_handler![
            greet,
            initialize_database,
            database_health_check,
            is_first_launch,
            create_admin_account,
            login,
            logout,
            check_session,
            create_engineer_user,
            list_users,
            deactivate_user,
            reactivate_user
        ])
        .setup(|_app| {
            info!("Ferrocodex application starting up...");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
