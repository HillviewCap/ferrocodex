// Auth business logic handlers
// This module contains the core business logic for authentication operations
// separated from the Tauri command layer for better testability and modularity

use crate::auth::{SessionManager, LoginAttemptTracker, LoginResponse, verify_password};
use crate::users::{CreateUserRequest, UserRepository, SqliteUserRepository, UserRole, User};
use crate::database::Database;
use tracing::{error, info, warn};

pub struct AuthHandler;

impl AuthHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn create_admin_account_handler(
        &self,
        username: String,
        password: String,
        db: &Database,
        session_manager: &mut SessionManager,
    ) -> Result<LoginResponse, String> {
        let user_repo = SqliteUserRepository::new(db.get_connection());
        
        // Check if admin users already exist
        let has_admins = user_repo.has_admin_users()
            .map_err(|e| format!("Failed to check for admin users: {}", e))?;
        
        if has_admins {
            return Err("Admin account already exists".to_string());
        }

        let request = CreateUserRequest {
            username: username.clone(),
            password,
            role: UserRole::Administrator,
        };

        let user = user_repo.create_user(request)
            .map_err(|e| format!("Failed to create admin account: {}", e))?;

        let session = session_manager.create_session(&user)
            .map_err(|e| format!("Failed to create session: {}", e))?;

        info!("Created admin account for user: {}", username);

        Ok(LoginResponse {
            token: session.token,
            user: user.into(),
        })
    }

    pub fn login_handler(
        &self,
        username: String,
        password: String,
        db: &Database,
        session_manager: &mut SessionManager,
        attempt_tracker: &mut LoginAttemptTracker,
    ) -> Result<LoginResponse, String> {
        // Check if account is locked
        if attempt_tracker.is_locked(&username).map_err(|e| e.to_string())? {
            warn!("Login attempt for locked account: {}", username);
            return Err("Account is temporarily locked due to too many failed attempts".to_string());
        }

        let user_repo = SqliteUserRepository::new(db.get_connection());
        
        match user_repo.find_by_username(&username) {
            Ok(Some(user)) => {
                match verify_password(&password, &user.password_hash) {
                    Ok(true) => {
                        // Successful login
                        attempt_tracker.record_successful_attempt(&username)
                            .map_err(|e| e.to_string())?;

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
                        attempt_tracker.record_failed_attempt(&username)
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
                attempt_tracker.record_failed_attempt(&username)
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

    pub fn logout_handler(
        &self,
        token: String,
        session_manager: &mut SessionManager,
    ) -> Result<(), String> {
        session_manager.invalidate_session(&token)
            .map_err(|e| format!("Failed to logout: {}", e))?;
        
        info!("User logged out");
        Ok(())
    }

    pub fn validate_session_handler(
        &self,
        token: String,
        session_manager: &SessionManager,
    ) -> Result<Option<crate::auth::SessionToken>, String> {
        session_manager.validate_session(&token)
            .map_err(|e| {
                error!("Session validation error: {}", e);
                "Session validation error".to_string()
            })
    }
}