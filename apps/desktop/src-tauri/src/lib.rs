mod database;
mod users;
mod auth;
mod audit;
mod validation;
mod assets;
mod configurations;
mod encryption;
mod branches;
mod firmware;
mod firmware_analysis;
mod recovery;
mod vault;

use database::Database;
use rusqlite::Connection;
use users::{CreateUserRequest, UserRepository, SqliteUserRepository, UserRole, UserInfo};
use auth::{SessionManager, LoginAttemptTracker, LoginResponse, verify_password};
use audit::{AuditRepository, SqliteAuditRepository, AuditEventRequest, AuditEventType, create_user_created_event, create_user_deactivated_event, create_user_reactivated_event, create_vault_access_granted_event, create_vault_access_revoked_event};
use validation::{UsernameValidator, PasswordValidator, InputSanitizer, RateLimiter};
use assets::{AssetRepository, SqliteAssetRepository, CreateAssetRequest, AssetInfo};
use configurations::{ConfigurationRepository, SqliteConfigurationRepository, CreateConfigurationRequest, ConfigurationVersionInfo, ConfigurationStatus, StatusChangeRecord, file_utils, FileMetadata};
use branches::{BranchRepository, SqliteBranchRepository, CreateBranchRequest, BranchInfo, CreateBranchVersionRequest, BranchVersionInfo};
use firmware::{FirmwareRepository, SqliteFirmwareRepository, CreateFirmwareRequest, FirmwareVersionInfo, FirmwareFileStorage, FirmwareStatus, FirmwareStatusHistory};
use firmware_analysis::{FirmwareAnalysisRepository, SqliteFirmwareAnalysisRepository, FirmwareAnalysisResult, AnalysisQueue, AnalysisJob};
use recovery::{RecoveryExporter, RecoveryExportRequest, RecoveryManifest, RecoveryImporter, RecoveryImportRequest};
use vault::{VaultRepository, SqliteVaultRepository, CreateVaultRequest, AddSecretRequest, VaultInfo, IdentityVault, GeneratePasswordRequest, UpdateCredentialPasswordRequest, PasswordStrength, PasswordHistory, PasswordGenerator, PasswordStrengthAnalyzer, CreateStandaloneCredentialRequest, UpdateStandaloneCredentialRequest, SearchCredentialsRequest, CreateCategoryRequest, StandaloneCredentialInfo, CategoryWithChildren, VaultAccessControlService, PermissionType, VaultAccessInfo, GrantVaultAccessRequest, VaultPermission, RevokeVaultAccessRequest, VaultAccessLog, CreatePermissionRequest, PermissionRequest, rotation::{PasswordRotationService, PasswordRotationRequest, RotationScheduler, RotationSchedule, RotationBatch, BatchRotationService, PasswordRotationHistory, RotationAlert, CreateRotationScheduleRequest, UpdateRotationScheduleRequest, CreateRotationBatchRequest, BatchRotationRequest}};
use encryption::FileEncryption;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, State, Manager};
use tracing::{error, info, warn};
use serde::{Serialize, Deserialize};

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;
type LoginAttemptTrackerState = Mutex<LoginAttemptTracker>;
type RateLimiterState = Mutex<RateLimiter>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DashboardStats {
    total_assets: i64,
    total_versions: i64,
    encryption_type: String,
}

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
    info!("Database path: {:?}", db_path);
    info!("Database exists before creation: {}", db_path.exists());
    
    match Database::new(db_path) {
        Ok(database) => {
            let health_check = database.health_check()
                .map_err(|e| format!("Database health check failed: {}", e))?;
            
            if health_check {
                let mut db_guard = db_state.lock()
                    .map_err(|_| "Failed to acquire database lock".to_string())?;
                *db_guard = Some(database);
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
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => db.health_check().map_err(|e| e.to_string()),
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn is_first_launch(db_state: State<'_, DatabaseState>) -> Result<bool, String> {
    info!("Checking if this is first launch");
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let user_repo = SqliteUserRepository::new(db.get_connection());
            let has_admins = user_repo.has_admin_users()
                .map_err(|e| format!("Failed to check for admin users: {}", e))?;
            let is_first = !has_admins;
            info!("Has admin users: {}, Is first launch: {}", has_admins, is_first);
            Ok(is_first)
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
async fn login(
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
async fn logout(
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
async fn check_session(
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
async fn deactivate_user(
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

// Asset Management Commands

#[tauri::command]
async fn create_asset(
    token: String,
    name: String,
    description: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<AssetInfo, String> {
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
    drop(session_manager_guard);

    // Validate inputs
    let name = InputSanitizer::sanitize_string(&name);
    let description = InputSanitizer::sanitize_string(&description);

    if name.trim().is_empty() {
        return Err("Asset name cannot be empty".to_string());
    }
    if name.len() < 2 {
        return Err("Asset name must be at least 2 characters long".to_string());
    }
    if name.len() > 100 {
        return Err("Asset name cannot exceed 100 characters".to_string());
    }
    if description.len() > 500 {
        return Err("Description cannot exceed 500 characters".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            let request = CreateAssetRequest {
                name,
                description,
                created_by: session.user_id,
            };

            match asset_repo.create_asset(request) {
                Ok(asset) => {
                    info!("Asset created by {}: {} (ID: {})", session.username, asset.name, asset.id);
                    Ok(asset.into())
                }
                Err(e) => {
                    error!("Failed to create asset: {}", e);
                    Err(format!("Failed to create asset: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_dashboard_assets(
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<AssetInfo>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            match asset_repo.get_assets_with_info() {
                Ok(assets) => {
                    info!("Dashboard assets accessed by: {}", session.username);
                    Ok(assets)
                }
                Err(e) => {
                    error!("Failed to get dashboard assets: {}", e);
                    Err(format!("Failed to get dashboard assets: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_dashboard_stats(
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<DashboardStats, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            
            // Get total assets count
            let total_assets: i64 = conn
                .query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))
                .map_err(|e| {
                    error!("Failed to count assets: {}", e);
                    format!("Failed to count assets: {}", e)
                })?;
            
            // Get total versions count across all assets
            let total_versions: i64 = conn
                .query_row("SELECT COUNT(*) FROM configuration_versions", [], |row| row.get(0))
                .map_err(|e| {
                    error!("Failed to count versions: {}", e);
                    format!("Failed to count versions: {}", e)
                })?;
            
            let stats = DashboardStats {
                total_assets,
                total_versions,
                encryption_type: "AES-256".to_string(),
            };
            
            info!("Dashboard stats accessed by: {}", session.username);
            Ok(stats)
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_asset_details(
    token: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<AssetInfo, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            match asset_repo.get_asset_by_id(asset_id) {
                Ok(Some(asset)) => {
                    info!("Asset details accessed by {}: {} (ID: {})", session.username, asset.name, asset.id);
                    Ok(asset.into())
                }
                Ok(None) => Err("Asset not found".to_string()),
                Err(e) => {
                    error!("Failed to get asset details: {}", e);
                    Err(format!("Failed to get asset details: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn import_configuration(
    token: String,
    asset_name: String,
    file_path: String,
    notes: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    rate_limiter: State<'_, RateLimiterState>,
) -> Result<AssetInfo, String> {
    // Check rate limiting
    let rate_limiter_guard = rate_limiter.lock()
        .map_err(|_| "Failed to acquire rate limiter lock".to_string())?;
    if let Err(e) = rate_limiter_guard.check_rate_limit(&format!("import_config_{}", token)) {
        return Err(e);
    }
    drop(rate_limiter_guard);

    // Validate session
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
    drop(session_manager_guard);

    // Validate inputs
    let asset_name = InputSanitizer::sanitize_string(&asset_name);
    let notes = InputSanitizer::sanitize_string(&notes);

    if asset_name.trim().is_empty() {
        return Err("Asset name cannot be empty".to_string());
    }
    if asset_name.len() < 2 {
        return Err("Asset name must be at least 2 characters long".to_string());
    }
    if asset_name.len() > 100 {
        return Err("Asset name cannot exceed 100 characters".to_string());
    }
    if notes.len() > 1000 {
        return Err("Notes cannot exceed 1000 characters".to_string());
    }

    // Check for malicious input
    if InputSanitizer::is_potentially_malicious(&asset_name) || InputSanitizer::is_potentially_malicious(&notes) {
        error!("Potentially malicious input detected in import_configuration");
        return Err("Invalid input detected. Please avoid using special characters or script-like patterns.".to_string());
    }

    // Read file content
    let file_content = match file_utils::read_file_content(&file_path) {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to read file {}: {}", file_path, e);
            return Err(format!("Failed to read file: {}", e));
        }
    };

    // Get file name from path
    let file_name = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Validate file type
    match file_utils::validate_file_type(&file_path) {
        Ok(false) => return Err("File type not supported".to_string()),
        Err(e) => {
            error!("File validation error: {}", e);
            return Err("File validation failed".to_string());
        }
        Ok(true) => {}
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            // Create asset
            let asset_request = CreateAssetRequest {
                name: asset_name,
                description: format!("Configuration asset - imported from {}", file_name),
                created_by: session.user_id,
            };

            let asset = match asset_repo.create_asset(asset_request) {
                Ok(asset) => asset,
                Err(e) => {
                    error!("Failed to create asset: {}", e);
                    return Err(format!("Failed to create asset: {}", e));
                }
            };

            // Store configuration
            let config_request = CreateConfigurationRequest {
                asset_id: asset.id,
                file_name,
                file_content,
                author: session.user_id,
                notes,
            };

            match config_repo.store_configuration(config_request) {
                Ok(_) => {
                    info!("Configuration imported by {}: {} (Asset ID: {})", session.username, asset.name, asset.id);
                    Ok(asset.into())
                }
                Err(e) => {
                    error!("Failed to store configuration: {}", e);
                    // Clean up asset if configuration storage failed
                    let _ = asset_repo.delete_asset(asset.id);
                    Err(format!("Failed to store configuration: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_configuration_versions(
    token: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<ConfigurationVersionInfo>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            match config_repo.get_configuration_versions(asset_id) {
                Ok(versions) => {
                    info!("Configuration versions accessed by {}: Asset ID {}", session.username, asset_id);
                    Ok(versions)
                }
                Err(e) => {
                    error!("Failed to get configuration versions: {}", e);
                    Err(format!("Failed to get configuration versions: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Branch Management Commands

#[tauri::command]
async fn create_branch(
    token: String,
    name: String,
    description: Option<String>,
    asset_id: i64,
    parent_version_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<BranchInfo, String> {
    // Validate session
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
    drop(session_manager_guard);

    // Validate inputs
    let name = InputSanitizer::sanitize_string(&name);
    let description = description.map(|d| InputSanitizer::sanitize_string(&d));

    if InputSanitizer::is_potentially_malicious(&name) {
        error!("Potentially malicious input detected in create_branch");
        return Err("Invalid input detected".to_string());
    }

    if let Some(ref desc) = description {
        if InputSanitizer::is_potentially_malicious(desc) {
            error!("Potentially malicious input detected in create_branch description");
            return Err("Invalid input detected".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            let request = CreateBranchRequest {
                name,
                description,
                asset_id,
                parent_version_id,
                created_by: session.user_id,
            };

            match branch_repo.create_branch(request) {
                Ok(branch) => {
                    info!("Branch created by {}: {} for asset {}", session.username, branch.name, asset_id);
                    // For a newly created branch, we need to fetch the full BranchInfo with proper metadata
                    match branch_repo.get_branch_by_id(branch.id) {
                        Ok(Some(branch_info)) => Ok(branch_info),
                        Ok(None) => {
                            error!("Created branch not found: {}", branch.id);
                            Err("Failed to retrieve created branch".to_string())
                        }
                        Err(e) => {
                            error!("Failed to retrieve created branch: {}", e);
                            Err(format!("Failed to retrieve created branch: {}", e))
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to create branch: {}", e);
                    Err(format!("Failed to create branch: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_branches(
    token: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<BranchInfo>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            match branch_repo.get_branches(asset_id) {
                Ok(branches) => {
                    info!("Branches accessed by {}: Asset ID {}", session.username, asset_id);
                    Ok(branches)
                }
                Err(e) => {
                    error!("Failed to get branches: {}", e);
                    Err(format!("Failed to get branches: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_branch_details(
    token: String,
    branch_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<BranchInfo, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            match branch_repo.get_branch_by_id(branch_id) {
                Ok(Some(branch)) => {
                    info!("Branch details accessed by {}: Branch ID {}", session.username, branch_id);
                    Ok(branch)
                }
                Ok(None) => Err("Branch not found".to_string()),
                Err(e) => {
                    error!("Failed to get branch details: {}", e);
                    Err(format!("Failed to get branch details: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Branch Version Management Commands

#[tauri::command]
async fn import_version_to_branch(
    token: String,
    branch_id: i64,
    file_path: String,
    notes: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<BranchVersionInfo, String> {
    // Validate session
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
    drop(session_manager_guard);

    // Validate inputs
    let notes = InputSanitizer::sanitize_string(&notes);
    
    if InputSanitizer::is_potentially_malicious(&notes) {
        error!("Potentially malicious input detected in import_version_to_branch");
        return Err("Invalid input detected. Please avoid using special characters or script-like patterns in your notes.".to_string());
    }

    if notes.len() > 1000 {
        return Err("Notes cannot exceed 1000 characters".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            let request = CreateBranchVersionRequest {
                branch_id,
                file_path,
                notes,
                author: session.user_id,
            };

            match branch_repo.import_version_to_branch(request) {
                Ok(_branch_version) => {
                    // Get the full branch version info
                    match branch_repo.get_branch_latest_version(branch_id) {
                        Ok(Some(version_info)) => {
                            info!("Version imported to branch by {}: Branch ID {}", session.username, branch_id);
                            Ok(version_info)
                        }
                        Ok(None) => Err("Failed to retrieve imported version info".to_string()),
                        Err(e) => {
                            error!("Failed to get branch version info: {}", e);
                            Err(format!("Failed to get branch version info: {}", e))
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to import version to branch: {}", e);
                    Err(format!("Failed to import version to branch: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_branch_versions(
    token: String,
    branch_id: i64,
    page: Option<i32>,
    limit: Option<i32>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<BranchVersionInfo>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            match branch_repo.get_branch_versions(branch_id, page, limit) {
                Ok(versions) => {
                    info!("Branch versions accessed by {}: Branch ID {}", session.username, branch_id);
                    Ok(versions)
                }
                Err(e) => {
                    error!("Failed to get branch versions: {}", e);
                    Err(format!("Failed to get branch versions: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_branch_latest_version(
    token: String,
    branch_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Option<BranchVersionInfo>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            match branch_repo.get_branch_latest_version(branch_id) {
                Ok(version) => {
                    info!("Branch latest version accessed by {}: Branch ID {}", session.username, branch_id);
                    Ok(version)
                }
                Err(e) => {
                    error!("Failed to get branch latest version: {}", e);
                    Err(format!("Failed to get branch latest version: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn compare_branch_versions(
    token: String,
    branch_id: i64,
    version1_id: i64,
    version2_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<String, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            
            match branch_repo.compare_branch_versions(branch_id, version1_id, version2_id) {
                Ok(diff_content) => {
                    // Convert bytes to string for frontend
                    match String::from_utf8(diff_content) {
                        Ok(diff_str) => {
                            info!("Branch versions compared by {}: Branch ID {}", session.username, branch_id);
                            Ok(diff_str)
                        }
                        Err(_) => Err("Failed to convert diff content to string".to_string()),
                    }
                }
                Err(e) => {
                    error!("Failed to compare branch versions: {}", e);
                    Err(format!("Failed to compare branch versions: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Configuration Status Management Commands

#[tauri::command]
async fn update_configuration_status(
    token: String,
    version_id: i64,
    new_status: String,
    change_reason: Option<String>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
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
    drop(session_manager_guard);

    // Parse and validate status
    let status = ConfigurationStatus::from_str(&new_status)
        .ok_or_else(|| format!("Invalid status: {}", new_status))?;

    // Prevent direct Golden status changes - must use promote_to_golden
    if status == ConfigurationStatus::Golden {
        return Err("Golden status can only be set through the promotion wizard. Please use 'Promote to Golden' option for Approved versions.".to_string());
    }

    // Validate inputs
    let change_reason = change_reason.map(|r| InputSanitizer::sanitize_string(&r));
    
    if let Some(ref reason) = change_reason {
        if InputSanitizer::is_potentially_malicious(reason) {
            error!("Potentially malicious input detected in update_configuration_status");
            return Err("Invalid input detected".to_string());
        }
        if reason.len() > 500 {
            return Err("Change reason cannot exceed 500 characters".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());

            // Check if user has permission for this status transition
            let available_transitions = config_repo.get_available_status_transitions(version_id, &session.role.to_string())
                .map_err(|e| format!("Failed to check permissions: {}", e))?;

            if !available_transitions.iter().any(|t| t.as_str() == status.as_str()) {
                return Err("You do not have permission to make this status change".to_string());
            }

            match config_repo.update_configuration_status(version_id, status, session.user_id, change_reason) {
                Ok(_) => {
                    info!("Configuration status updated by {}: Version ID {} to {}", session.username, version_id, new_status);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to update configuration status: {}", e);
                    Err(format!("Failed to update configuration status: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_configuration_status_history(
    token: String,
    version_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<StatusChangeRecord>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            match config_repo.get_configuration_status_history(version_id) {
                Ok(history) => {
                    info!("Configuration status history accessed by {}: Version ID {}", session.username, version_id);
                    Ok(history)
                }
                Err(e) => {
                    error!("Failed to get configuration status history: {}", e);
                    Err(format!("Failed to get configuration status history: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_available_status_transitions(
    token: String,
    version_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<String>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            match config_repo.get_available_status_transitions(version_id, &session.role.to_string()) {
                Ok(transitions) => {
                    let status_strings: Vec<String> = transitions.iter().map(|s| s.as_str().to_string()).collect();
                    info!("Available status transitions accessed by {}: Version ID {}", session.username, version_id);
                    Ok(status_strings)
                }
                Err(e) => {
                    error!("Failed to get available status transitions: {}", e);
                    Err(format!("Failed to get available status transitions: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Golden Promotion Commands

#[tauri::command]
async fn promote_to_golden(
    token: String,
    version_id: i64,
    promotion_reason: Option<String>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
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
    drop(session_manager_guard);

    // Only Engineers and Administrators can promote to Golden
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to promote to Golden: {}", session.username);
        return Err("Only Engineers and Administrators can promote versions to Golden".to_string());
    }

    // Validate inputs
    let promotion_reason = promotion_reason.map(|r| InputSanitizer::sanitize_string(&r));
    
    if let Some(ref reason) = promotion_reason {
        if InputSanitizer::is_potentially_malicious(reason) {
            error!("Potentially malicious input detected in promote_to_golden");
            return Err("Invalid input detected".to_string());
        }
        if reason.len() > 500 {
            return Err("Promotion reason cannot exceed 500 characters".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());

            // Check promotion eligibility
            let is_eligible = config_repo.get_promotion_eligibility(version_id)
                .map_err(|e| format!("Failed to check promotion eligibility: {}", e))?;

            if !is_eligible {
                return Err("Version is not eligible for Golden promotion. Only Approved versions can be promoted.".to_string());
            }

            match config_repo.promote_to_golden(version_id, session.user_id, promotion_reason) {
                Ok(_) => {
                    info!("Version promoted to Golden by {}: Version ID {}", session.username, version_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to promote to Golden: {}", e);
                    Err(format!("Failed to promote to Golden: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn promote_branch_to_silver(
    token: String,
    branch_id: i64,
    promotion_notes: Option<String>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<i64, String> {
    // Validate session
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
    drop(session_manager_guard);

    // Both Engineers and Administrators can promote branches to Silver
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to promote branch to Silver: {}", session.username);
        return Err("Insufficient permissions to promote branch to Silver".to_string());
    }

    // Validate inputs
    let promotion_notes = promotion_notes.map(|n| InputSanitizer::sanitize_string(&n));
    
    if let Some(ref notes) = promotion_notes {
        if InputSanitizer::is_potentially_malicious(notes) {
            error!("Potentially malicious input detected in promote_branch_to_silver");
            return Err("Invalid input detected".to_string());
        }
        if notes.len() > 1000 {
            return Err("Promotion notes cannot exceed 1000 characters".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let branch_repo = SqliteBranchRepository::new(db.get_connection());
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            // Get the latest version of the branch
            let latest_version = match branch_repo.get_branch_latest_version(branch_id) {
                Ok(Some(version)) => version,
                Ok(None) => return Err("Branch has no versions to promote".to_string()),
                Err(e) => {
                    error!("Failed to get branch latest version: {}", e);
                    return Err(format!("Failed to get branch latest version: {}", e));
                }
            };
            
            // Get branch details to get the asset ID
            let branch = match branch_repo.get_branch_by_id(branch_id) {
                Ok(Some(b)) => b,
                Ok(None) => return Err("Branch not found".to_string()),
                Err(e) => {
                    error!("Failed to get branch details: {}", e);
                    return Err(format!("Failed to get branch details: {}", e));
                }
            };
            
            // Get the configuration content
            let content = match config_repo.get_configuration_content(latest_version.version_id) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to get configuration content: {}", e);
                    return Err(format!("Failed to get configuration content: {}", e));
                }
            };
            
            // Create a new configuration version in the main line with Silver status
            let notes = format!(
                "Promoted from branch '{}' (version {}). {}",
                branch.name,
                latest_version.branch_version_number,
                promotion_notes.unwrap_or_default()
            );
            
            let config_request = CreateConfigurationRequest {
                asset_id: branch.asset_id,
                file_name: latest_version.file_name.clone(),
                file_content: content,
                author: session.user_id,
                notes,
            };
            
            // Store the new configuration
            let new_config = match config_repo.store_configuration(config_request) {
                Ok(config) => config,
                Err(e) => {
                    error!("Failed to create Silver configuration: {}", e);
                    return Err(format!("Failed to create Silver configuration: {}", e));
                }
            };
            
            // Update the status to Silver
            match config_repo.update_configuration_status(
                new_config.id,
                ConfigurationStatus::Silver,
                session.user_id,
                Some("Promoted from branch".to_string())
            ) {
                Ok(_) => {
                    info!("Branch promoted to Silver by {}: Branch {} -> Config ID {}", 
                          session.username, branch.name, new_config.id);
                    Ok(new_config.id)
                }
                Err(e) => {
                    error!("Failed to update status to Silver: {}", e);
                    Err(format!("Failed to update status to Silver: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_golden_version(
    token: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Option<ConfigurationVersionInfo>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            match config_repo.get_golden_version(asset_id) {
                Ok(golden_version) => {
                    info!("Golden version accessed by {}: Asset ID {}", session.username, asset_id);
                    Ok(golden_version)
                }
                Err(e) => {
                    error!("Failed to get Golden version: {}", e);
                    Err(format!("Failed to get Golden version: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_promotion_eligibility(
    token: String,
    version_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<bool, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            match config_repo.get_promotion_eligibility(version_id) {
                Ok(eligibility) => {
                    info!("Promotion eligibility checked by {}: Version ID {}", session.username, version_id);
                    Ok(eligibility)
                }
                Err(e) => {
                    error!("Failed to check promotion eligibility: {}", e);
                    Err(format!("Failed to check promotion eligibility: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn export_configuration_version(
    token: String,
    version_id: i64,
    export_path: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
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
    drop(session_manager_guard);

    // Validate export path
    let export_path = export_path.trim();
    
    if export_path.is_empty() {
        return Err("Export path cannot be empty".to_string());
    }
    
    // Use proper file path validation instead of generic malicious input check
    if let Err(e) = InputSanitizer::validate_file_path(&export_path) {
        error!("Invalid export path: {}", e);
        return Err(format!("Invalid export path: {}", e));
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            let start_time = std::time::Instant::now();
            
            match config_repo.export_configuration_version(version_id, &export_path) {
                Ok(_) => {
                    let duration = start_time.elapsed();
                    
                    // Log performance metrics
                    if duration.as_secs() >= 2 {
                        warn!("Export operation took {} seconds, exceeding 2-second requirement", duration.as_secs_f64());
                    } else {
                        info!("Export completed in {:.2} seconds", duration.as_secs_f64());
                    }
                    
                    info!("Configuration exported by {}: Version ID {} to {}", session.username, version_id, export_path);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to export configuration: {}", e);
                    Err(format!("Failed to export configuration: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_file_metadata(
    file_path: String,
) -> Result<FileMetadata, String> {
    // Validate file path
    let file_path = file_path.trim();
    
    if file_path.is_empty() {
        return Err("File path cannot be empty".to_string());
    }
    
    // Use proper file path validation
    if let Err(e) = InputSanitizer::validate_file_path(&file_path) {
        error!("Invalid file path: {}", e);
        return Err(format!("Invalid file path: {}", e));
    }

    match file_utils::get_file_metadata(&file_path) {
        Ok(metadata) => {
            info!("File metadata retrieved: {} ({} bytes)", metadata.name, metadata.size);
            Ok(metadata)
        }
        Err(e) => {
            error!("Failed to get file metadata: {}", e);
            Err(format!("Failed to get file metadata: {}", e))
        }
    }
}

#[tauri::command]
async fn archive_version(
    token: String,
    version_id: i64,
    archive_reason: Option<String>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
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
    drop(session_manager_guard);

    // Only Engineers and Administrators can archive versions
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to archive version: {}", session.username);
        return Err("Only Engineers and Administrators can archive versions".to_string());
    }

    // Validate inputs
    let archive_reason = archive_reason.map(|r| InputSanitizer::sanitize_string(&r));
    
    if let Some(ref reason) = archive_reason {
        if InputSanitizer::is_potentially_malicious(reason) {
            error!("Potentially malicious input detected in archive_version");
            return Err("Invalid input detected".to_string());
        }
        if reason.len() > 500 {
            return Err("Archive reason cannot exceed 500 characters".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());

            match config_repo.archive_version(version_id, session.user_id, archive_reason) {
                Ok(_) => {
                    info!("Version archived by {}: Version ID {}", session.username, version_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to archive version: {}", e);
                    Err(format!("Failed to archive version: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn restore_version(
    token: String,
    version_id: i64,
    restore_reason: Option<String>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
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
    drop(session_manager_guard);

    // Only Engineers and Administrators can restore versions
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to restore version: {}", session.username);
        return Err("Only Engineers and Administrators can restore versions".to_string());
    }

    // Validate inputs
    let restore_reason = restore_reason.map(|r| InputSanitizer::sanitize_string(&r));
    
    if let Some(ref reason) = restore_reason {
        if InputSanitizer::is_potentially_malicious(reason) {
            error!("Potentially malicious input detected in restore_version");
            return Err("Invalid input detected".to_string());
        }
        if reason.len() > 500 {
            return Err("Restore reason cannot exceed 500 characters".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());

            match config_repo.restore_version(version_id, session.user_id, restore_reason) {
                Ok(_) => {
                    info!("Version restored by {}: Version ID {}", session.username, version_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to restore version: {}", e);
                    Err(format!("Failed to restore version: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn link_firmware_to_configuration(
    token: String,
    config_id: i64,
    firmware_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
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
    drop(session_manager_guard);

    // Only Engineers and Administrators can link firmware
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to link firmware: {}", session.username);
        return Err("Only Engineers and Administrators can link firmware to configurations".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            let audit_repo = SqliteAuditRepository::new(db.get_connection());
            
            match config_repo.link_firmware_to_configuration(config_id, firmware_id) {
                Ok(_) => {
                    // Create audit event
                    let audit_event = audit::AuditEventRequest {
                        event_type: audit::AuditEventType::DatabaseOperation,
                        user_id: Some(session.user_id),
                        username: Some(session.username.clone()),
                        admin_user_id: None,
                        admin_username: None,
                        target_user_id: None,
                        target_username: None,
                        description: format!("Firmware {} linked to configuration {}", firmware_id, config_id),
                        metadata: Some(serde_json::json!({
                            "config_id": config_id,
                            "firmware_id": firmware_id,
                            "linked_by": session.username
                        }).to_string()),
                        ip_address: None,
                        user_agent: None,
                    };
                    
                    if let Err(e) = audit_repo.log_event(&audit_event) {
                        error!("Failed to log audit event: {}", e);
                    }
                    
                    info!("Firmware linked by {}: Config {} <-> Firmware {}", session.username, config_id, firmware_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to link firmware to configuration: {}", e);
                    Err(format!("Failed to link firmware to configuration: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn unlink_firmware_from_configuration(
    token: String,
    config_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
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
    drop(session_manager_guard);

    // Only Engineers and Administrators can unlink firmware
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to unlink firmware: {}", session.username);
        return Err("Only Engineers and Administrators can unlink firmware from configurations".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            let audit_repo = SqliteAuditRepository::new(db.get_connection());
            
            match config_repo.unlink_firmware_from_configuration(config_id) {
                Ok(_) => {
                    // Create audit event
                    let audit_event = audit::AuditEventRequest {
                        event_type: audit::AuditEventType::DatabaseOperation,
                        user_id: Some(session.user_id),
                        username: Some(session.username.clone()),
                        admin_user_id: None,
                        admin_username: None,
                        target_user_id: None,
                        target_username: None,
                        description: format!("Firmware unlinked from configuration {}", config_id),
                        metadata: Some(serde_json::json!({
                            "config_id": config_id,
                            "unlinked_by": session.username
                        }).to_string()),
                        ip_address: None,
                        user_agent: None,
                    };
                    
                    if let Err(e) = audit_repo.log_event(&audit_event) {
                        error!("Failed to log audit event: {}", e);
                    }
                    
                    info!("Firmware unlinked by {}: Config {}", session.username, config_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to unlink firmware from configuration: {}", e);
                    Err(format!("Failed to unlink firmware from configuration: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn upload_firmware(
    app: AppHandle,
    token: String,
    asset_id: i64,
    vendor: Option<String>,
    model: Option<String>,
    version: String,
    notes: Option<String>,
    file_path: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    audit_state: State<'_, DatabaseState>,
    rate_limiter: State<'_, RateLimiterState>,
) -> Result<FirmwareVersionInfo, String> {
    // Check rate limiting - limit firmware uploads per user
    {
        let rate_limiter_guard = rate_limiter.lock()
            .map_err(|_| "Failed to acquire rate limiter lock".to_string())?;
        if let Err(e) = rate_limiter_guard.check_rate_limit(&format!("firmware_upload_{}", token)) {
            return Err(e);
        }
    }

    // Validate session
    let session = {
        let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
        match session_manager_guard.validate_session(&token) {
            Ok(Some(session)) => session,
            Ok(None) => return Err("Invalid or expired session".to_string()),
            Err(e) => {
                error!("Session validation error: {}", e);
                return Err("Session validation error".to_string());
            }
        }
    };

    // Only Engineers and Administrators can upload firmware
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to upload firmware: {}", session.username);
        return Err("Only Engineers and Administrators can upload firmware".to_string());
    }

    // Validate file path first
    let file_path = file_path.trim();
    if file_path.is_empty() {
        return Err("File path cannot be empty".to_string());
    }
    
    if let Err(e) = InputSanitizer::validate_file_path(&file_path) {
        error!("Invalid file path: {}", e);
        return Err(format!("Invalid file path: {}", e));
    }

    // Validate inputs
    let version = InputSanitizer::sanitize_string(&version);
    let vendor = vendor.map(|v| InputSanitizer::sanitize_string(&v));
    let model = model.map(|m| InputSanitizer::sanitize_string(&m));
    let notes = notes.map(|n| InputSanitizer::sanitize_string(&n));
    
    if version.is_empty() {
        return Err("Version cannot be empty".to_string());
    }
    
    if let Some(ref v) = vendor {
        if v.len() > 100 {
            return Err("Vendor name cannot exceed 100 characters".to_string());
        }
    }
    
    if let Some(ref m) = model {
        if m.len() > 100 {
            return Err("Model name cannot exceed 100 characters".to_string());
        }
    }
    
    if let Some(ref n) = notes {
        if n.len() > 500 {
            return Err("Notes cannot exceed 500 characters".to_string());
        }
    }
    
    // Check for malicious input
    if InputSanitizer::is_potentially_malicious(&version) || 
       vendor.as_ref().map_or(false, |v| InputSanitizer::is_potentially_malicious(v)) ||
       model.as_ref().map_or(false, |m| InputSanitizer::is_potentially_malicious(m)) ||
       notes.as_ref().map_or(false, |n| InputSanitizer::is_potentially_malicious(n)) {
        error!("Potentially malicious input detected in upload_firmware");
        return Err("Invalid input detected. Please avoid using special characters or script-like patterns.".to_string());
    }
    
    // Validate file extension
    let allowed_extensions = vec![
        "bin", "hex", "img", "rom", "fw", "elf", "dfu", "upd", 
        "dat", "firmware", "update", "pkg", "ipk", "tar", "gz",
        "bz2", "xz", "zip", "rar", "7z", "cab", "iso", "dmg"
    ];
    
    let file_extension = std::path::Path::new(&file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_default();
    
    if file_extension.is_empty() {
        error!("Firmware file has no extension: {}", file_path);
        return Err("Firmware file must have a valid extension".to_string());
    }
    
    if !allowed_extensions.contains(&file_extension.as_str()) {
        error!("Invalid firmware file type: .{}", file_extension);
        return Err(format!(
            "File type .{} is not allowed. Allowed types: {}", 
            file_extension, 
            allowed_extensions.join(", ")
        ));
    }
    
    info!("Validated firmware file extension: .{}", file_extension);
    
    // Read file content
    let file_size = std::fs::metadata(&file_path)
        .map_err(|e| format!("Failed to get file metadata: {}", e))?
        .len();
    info!("Reading file content: {} bytes expected", file_size);
    let file_data = match file_utils::read_firmware_file_content(&file_path) {
        Ok(content) => {
            info!("Successfully read file content: {} bytes", content.len());
            content
        },
        Err(e) => {
            error!("Failed to read file {}: {}", file_path, e);
            return Err(format!("Failed to read file: {}", e));
        }
    };

    // Basic MIME type validation - check for common executable signatures
    if file_data.len() >= 2 {
        let header = &file_data[0..2];
        // Check for Windows executable (MZ header)
        if header == b"MZ" {
            error!("Detected Windows executable file signature");
            return Err("Executable files are not allowed as firmware".to_string());
        }
        // Check for ELF executable (common on Linux)
        if file_data.len() >= 4 && &file_data[0..4] == b"\x7FELF" {
            // ELF is actually allowed for firmware, so we'll permit this
            info!("Detected ELF format firmware file");
        }
    }

    // Store values needed for analysis before acquiring lock
    let firmware_result = {
        let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
        match db_guard.as_ref() {
            Some(db) => {
                let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
                
                // Generate a temporary firmware ID for file storage
            let temp_firmware_id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            
            // Store the firmware file
            info!("Starting firmware file storage process for {} bytes", file_data.len());
            let (file_path, file_hash, file_size) = match FirmwareFileStorage::store_firmware_file(
                &app,
                asset_id,
                temp_firmware_id,
                &file_data,
                session.user_id,
                &session.username,
            ) {
                Ok(result) => {
                    info!("Successfully stored firmware file: path={}, hash={}, size={}", result.0, result.1, result.2);
                    result
                },
                Err(e) => {
                    error!("Failed to store firmware file: {}", e);
                    return Err(format!("Failed to store firmware file: {}", e));
                }
            };
            
            // Create firmware record
            let request = CreateFirmwareRequest {
                asset_id,
                vendor,
                model,
                version,
                notes,
            };
            
            info!("Creating firmware database record");
            match firmware_repo.create_firmware(request, session.user_id, file_path.clone(), file_hash, file_size) {
                Ok(firmware) => {
                    info!("Successfully created firmware record with ID: {}", firmware.id);
                    // Update the file with the actual firmware ID
                    info!("Checking if file rename is needed");
                    let new_file_path = std::path::PathBuf::from(asset_id.to_string())
                        .join(format!("{}.enc", firmware.id))
                        .to_string_lossy()
                        .to_string();
                    info!("Comparing paths - old: '{}', new: '{}'", file_path, new_file_path);
                    if file_path != new_file_path {
                        // Rename the file if needed
                        let firmware_dir = firmware::get_firmware_storage_dir(&app).unwrap();
                        let old_path = firmware_dir.join(&file_path);
                        let new_path = firmware_dir.join(&new_file_path);
                        if let Err(e) = std::fs::rename(&old_path, &new_path) {
                            error!("Failed to rename firmware file: {}", e);
                            // Clean up on failure
                            let _ = firmware_repo.delete_firmware(firmware.id);
                            let _ = FirmwareFileStorage::delete_firmware_file(&app, &file_path);
                            return Err("Failed to finalize firmware storage".to_string());
                        }
                        info!("File renamed successfully from {:?} to {:?}", old_path, new_path);
                        
                        // Update the database with the new file path
                        if let Err(e) = firmware_repo.update_firmware_file_path(firmware.id, new_file_path.clone()) {
                            error!("Failed to update firmware file path in database: {}", e);
                            // Try to rename back on failure
                            let _ = std::fs::rename(&new_path, &old_path);
                            let _ = firmware_repo.delete_firmware(firmware.id);
                            return Err("Failed to update firmware file path".to_string());
                        }
                        info!("Database updated with new file path");
                    } else {
                        info!("File rename not needed, paths match");
                    }
                    
                    // Note: Audit logging moved outside database transaction to prevent deadlock
                    
                    info!("Creating FirmwareVersionInfo");
                    info!("Firmware uploaded by {}: v{} for asset {} (ID: {})", 
                         session.username, firmware.version, asset_id, firmware.id);
                    
                    // Convert to FirmwareVersionInfo
                    let firmware_info = FirmwareVersionInfo {
                        id: firmware.id,
                        asset_id: firmware.asset_id,
                        author_id: firmware.author_id,
                        author_username: session.username.clone(),
                        vendor: firmware.vendor,
                        model: firmware.model,
                        version: firmware.version,
                        notes: firmware.notes,
                        status: firmware.status,
                        file_path: new_file_path,
                        file_hash: firmware.file_hash,
                        file_size,
                        created_at: firmware.created_at,
                    };
                    
                    info!("Returning from database scope with firmware info");
                    Ok((firmware_info, firmware.id))
                }
                Err(e) => {
                    // Clean up file on database error
                    let _ = FirmwareFileStorage::delete_firmware_file(&app, &file_path);
                    error!("Failed to create firmware record: {}", e);
                    Err(format!("Failed to create firmware record: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
        }
    };
    
    // Handle the result and queue analysis if successful
    match firmware_result {
        Ok((firmware_info, firmware_id)) => {
            // Log audit event (moved here to avoid deadlock)
            info!("Logging audit event");
            if let Ok(audit_guard) = audit_state.lock() {
                if let Some(audit_db) = audit_guard.as_ref() {
                    let audit_repo = SqliteAuditRepository::new(audit_db.get_connection());
                    let audit_event = audit::AuditEventRequest {
                        event_type: audit::AuditEventType::FirmwareUpload,
                        user_id: Some(session.user_id),
                        username: Some(session.username.clone()),
                        admin_user_id: None,
                        admin_username: None,
                        target_user_id: None,
                        target_username: None,
                        description: format!("Uploaded firmware v{} for asset {}", firmware_info.version, asset_id),
                        metadata: Some(serde_json::json!({
                            "asset_id": asset_id,
                            "firmware_id": firmware_id,
                            "version": firmware_info.version,
                            "file_size": firmware_info.file_size,
                        }).to_string()),
                        ip_address: None,
                        user_agent: None,
                    };
                    if let Err(e) = audit_repo.log_event(&audit_event) {
                        error!("Failed to log audit event: {}", e);
                    } else {
                        info!("Audit event logged successfully");
                    }
                }
            } else {
                warn!("Could not acquire audit lock for logging");
            }
            
            // Queue firmware analysis
            let queue = get_or_create_analysis_queue(&app);
            
            let analysis_job = AnalysisJob {
                firmware_id,
                user_id: session.user_id,
                username: session.username.clone(),
            };
            
            info!("Attempting to queue firmware analysis for firmware ID: {}", firmware_id);
            if let Err(e) = queue.queue_analysis(analysis_job).await {
                warn!("Failed to queue firmware analysis: {}", e);
                // Don't fail the upload if analysis queueing fails
            } else {
                info!("Firmware analysis queued for firmware ID: {}", firmware_id);
            }
            
            info!("Upload process completed successfully, returning firmware info");
            Ok(firmware_info)
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
async fn get_firmware_list(
    token: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<FirmwareVersionInfo>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            
            match firmware_repo.get_firmware_by_asset(asset_id) {
                Ok(firmwares) => {
                    info!("Retrieved {} firmware versions for asset {} by {}", 
                         firmwares.len(), asset_id, session.username);
                    Ok(firmwares)
                }
                Err(e) => {
                    error!("Failed to get firmware list: {}", e);
                    Err(format!("Failed to get firmware list: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn delete_firmware(
    app: AppHandle,
    token: String,
    firmware_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    audit_state: State<'_, DatabaseState>,
) -> Result<(), String> {
    // Validate session
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
    drop(session_manager_guard);

    // Only Engineers and Administrators can delete firmware
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to delete firmware: {}", session.username);
        return Err("Only Engineers and Administrators can delete firmware".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            
            // Get firmware details before deletion for audit
            let firmware_info = match firmware_repo.get_firmware_by_id(firmware_id) {
                Ok(Some(fw)) => fw,
                Ok(None) => return Err("Firmware not found".to_string()),
                Err(e) => {
                    error!("Failed to get firmware info: {}", e);
                    return Err("Failed to get firmware info".to_string());
                }
            };
            
            // Delete firmware record and get file path
            match firmware_repo.delete_firmware(firmware_id) {
                Ok(Some(file_path)) => {
                    // Delete the actual file
                    if let Err(e) = FirmwareFileStorage::delete_firmware_file(&app, &file_path) {
                        error!("Failed to delete firmware file: {}", e);
                        // Continue anyway - the database record is already deleted
                    }
                    
                    // Log audit event
                    if let Ok(audit_guard) = audit_state.lock() {
                        if let Some(audit_db) = audit_guard.as_ref() {
                        let audit_repo = SqliteAuditRepository::new(audit_db.get_connection());
                        let audit_event = audit::AuditEventRequest {
                            event_type: audit::AuditEventType::FirmwareDelete,
                            user_id: Some(session.user_id),
                            username: Some(session.username.clone()),
                            admin_user_id: None,
                            admin_username: None,
                            target_user_id: None,
                            target_username: None,
                            description: format!("Deleted firmware v{} for asset {}", firmware_info.version, firmware_info.asset_id),
                            metadata: Some(serde_json::json!({
                                "asset_id": firmware_info.asset_id,
                                "firmware_id": firmware_id,
                                "version": firmware_info.version,
                            }).to_string()),
                            ip_address: None,
                            user_agent: None,
                        };
                        if let Err(e) = audit_repo.log_event(&audit_event) {
                            error!("Failed to log audit event: {}", e);
                        }
                    }
                    }
                    
                    info!("Firmware deleted by {}: ID {} (v{} for asset {})", 
                         session.username, firmware_id, firmware_info.version, firmware_info.asset_id);
                    Ok(())
                }
                Ok(None) => Err("Firmware not found".to_string()),
                Err(e) => {
                    error!("Failed to delete firmware: {}", e);
                    Err(format!("Failed to delete firmware: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

fn get_or_create_analysis_queue(app: &AppHandle) -> Arc<AnalysisQueue> {
    match app.try_state::<Arc<AnalysisQueue>>() {
        Some(queue) => queue.inner().clone(),
        None => {
            // Initialize the queue on first use
            let new_queue = Arc::new(AnalysisQueue::new(app.clone()));
            app.manage(new_queue.clone());
            new_queue
        }
    }
}

#[tauri::command]
async fn get_firmware_analysis(
    token: String,
    firmware_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Option<FirmwareAnalysisResult>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let analysis_repo = SqliteFirmwareAnalysisRepository::new(db.get_connection());
            
            match analysis_repo.get_analysis_by_firmware_id(firmware_id) {
                Ok(analysis) => {
                    info!("Firmware analysis retrieved by {}: Firmware ID {}", session.username, firmware_id);
                    Ok(analysis)
                }
                Err(e) => {
                    error!("Failed to get firmware analysis: {}", e);
                    Err(format!("Failed to get firmware analysis: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}


#[tauri::command]
async fn export_complete_recovery(
    app: AppHandle,
    token: String,
    asset_id: i64,
    config_version_id: i64,
    firmware_version_id: i64,
    export_directory: String,
    include_vault: Option<bool>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<RecoveryManifest, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            let audit_repo = SqliteAuditRepository::new(db.get_connection());
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            // Get asset name for proper filename generation
            let asset = match asset_repo.get_asset_by_id(asset_id) {
                Ok(Some(asset)) => asset,
                Ok(None) => return Err("Asset not found".to_string()),
                Err(e) => return Err(format!("Failed to get asset: {}", e)),
            };

            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            let exporter = RecoveryExporter::new(&config_repo, &firmware_repo, &vault_repo, &audit_repo);
            
            let request = RecoveryExportRequest {
                asset_id,
                config_version_id,
                firmware_version_id,
                export_directory,
                include_vault,
            };

            match exporter.export_complete_recovery(
                &app,
                request,
                session.user_id,
                &session.username,
                &session.role,
                &asset.name,
            ) {
                Ok(manifest) => Ok(manifest),
                Err(e) => Err(e.to_string()),
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_configurations_by_firmware(
    token: String,
    firmware_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<ConfigurationVersionInfo>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            
            match config_repo.get_configurations_by_firmware(firmware_id) {
                Ok(configs) => {
                    info!("Linked configurations accessed by {}: Firmware ID {}", session.username, firmware_id);
                    Ok(configs)
                }
                Err(e) => {
                    error!("Failed to get linked configurations: {}", e);
                    Err(format!("Failed to get linked configurations: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn retry_firmware_analysis(
    token: String,
    firmware_id: i64,
    session_manager: State<'_, SessionManagerState>,
    app: AppHandle,
) -> Result<(), String> {
    // Validate session
    let session = {
        let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
        match session_manager_guard.validate_session(&token) {
            Ok(Some(session)) => session,
            Ok(None) => return Err("Invalid or expired session".to_string()),
            Err(e) => {
                error!("Session validation error: {}", e);
                return Err("Session validation error".to_string());
            }
        }
    };

    // Only Engineers and Administrators can retry analysis
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to retry firmware analysis: {}", session.username);
        return Err("Only Engineers and Administrators can retry firmware analysis".to_string());
    }

    // Queue the analysis job
    let job = AnalysisJob {
        firmware_id,
        user_id: session.user_id,
        username: session.username.clone(),
    };

    // Get or create the analysis queue
    let queue = get_or_create_analysis_queue(&app);

    match queue.queue_analysis(job).await {
        Ok(_) => {
            info!("Firmware analysis retry queued by {}: Firmware ID {}", session.username, firmware_id);
            Ok(())
        }
        Err(e) => {
            error!("Failed to queue firmware analysis: {}", e);
            Err(format!("Failed to queue firmware analysis: {}", e))
        }
    }
}

#[tauri::command]
async fn update_firmware_status(
    token: String,
    firmware_id: i64,
    new_status: FirmwareStatus,
    reason: Option<String>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    audit_state: State<'_, DatabaseState>,
) -> Result<(), String> {
    // Validate session
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
    drop(session_manager_guard);

    // Only Engineers and Administrators can update firmware status
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to update firmware status: {}", session.username);
        return Err("Only Engineers and Administrators can update firmware status".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            
            match firmware_repo.update_firmware_status(firmware_id, new_status.clone(), session.user_id, reason.clone()) {
                Ok(_) => {
                    // Log audit event
                    if let Ok(audit_guard) = audit_state.lock() {
                        if let Some(audit_db) = audit_guard.as_ref() {
                            let audit_repo = SqliteAuditRepository::new(audit_db.get_connection());
                            let audit_event = audit::AuditEventRequest {
                                event_type: audit::AuditEventType::FirmwareStatusChange,
                                user_id: Some(session.user_id),
                                username: Some(session.username.clone()),
                                admin_user_id: None,
                                admin_username: None,
                                target_user_id: None,
                                target_username: None,
                                description: format!("Firmware {} status updated to {}", firmware_id, new_status),
                                metadata: Some(serde_json::json!({
                                    "firmware_id": firmware_id,
                                    "new_status": new_status.to_string(),
                                    "reason": reason,
                                    "changed_by": session.username
                                }).to_string()),
                                ip_address: None,
                                user_agent: None,
                            };
                            if let Err(e) = audit_repo.log_event(&audit_event) {
                                error!("Failed to log audit event: {}", e);
                            }
                        }
                    }
                    
                    info!("Firmware {} status updated to {} by {}", firmware_id, new_status, session.username);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to update firmware status: {}", e);
                    Err(format!("Failed to update firmware status: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_firmware_status_history(
    token: String,
    firmware_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<FirmwareStatusHistory>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            
            match firmware_repo.get_firmware_status_history(firmware_id) {
                Ok(history) => {
                    info!("Retrieved firmware status history for firmware {} by {}", firmware_id, session.username);
                    Ok(history)
                }
                Err(e) => {
                    error!("Failed to get firmware status history: {}", e);
                    Err(format!("Failed to get firmware status history: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_available_firmware_status_transitions(
    token: String,
    firmware_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<FirmwareStatus>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            
            match firmware_repo.get_available_firmware_status_transitions(firmware_id, &session.role.to_string()) {
                Ok(transitions) => {
                    info!("Retrieved available firmware status transitions for firmware {} by {}", firmware_id, session.username);
                    Ok(transitions)
                }
                Err(e) => {
                    error!("Failed to get available firmware status transitions: {}", e);
                    Err(format!("Failed to get available firmware status transitions: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn promote_firmware_to_golden(
    token: String,
    firmware_id: i64,
    reason: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    audit_state: State<'_, DatabaseState>,
) -> Result<(), String> {
    // Validate session
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
    drop(session_manager_guard);

    // Only Administrators can promote firmware to Golden
    if session.role != UserRole::Administrator {
        warn!("Non-administrator attempted to promote firmware to Golden: {}", session.username);
        return Err("Only Administrators can promote firmware to Golden status".to_string());
    }

    // Validate reason
    if reason.trim().is_empty() {
        return Err("Reason is required for Golden promotion".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            
            match firmware_repo.promote_firmware_to_golden(firmware_id, session.user_id, reason.clone()) {
                Ok(_) => {
                    // Log audit event
                    if let Ok(audit_guard) = audit_state.lock() {
                        if let Some(audit_db) = audit_guard.as_ref() {
                            let audit_repo = SqliteAuditRepository::new(audit_db.get_connection());
                            let audit_event = audit::AuditEventRequest {
                                event_type: audit::AuditEventType::FirmwareGoldenPromotion,
                                user_id: Some(session.user_id),
                                username: Some(session.username.clone()),
                                admin_user_id: None,
                                admin_username: None,
                                target_user_id: None,
                                target_username: None,
                                description: format!("Firmware {} promoted to Golden status", firmware_id),
                                metadata: Some(serde_json::json!({
                                    "firmware_id": firmware_id,
                                    "reason": reason,
                                    "promoted_by": session.username
                                }).to_string()),
                                ip_address: None,
                                user_agent: None,
                            };
                            if let Err(e) = audit_repo.log_event(&audit_event) {
                                error!("Failed to log audit event: {}", e);
                            }
                        }
                    }
                    
                    info!("Firmware {} promoted to Golden by {}", firmware_id, session.username);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to promote firmware to Golden: {}", e);
                    Err(format!("Failed to promote firmware to Golden: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn update_firmware_notes(
    token: String,
    firmware_id: i64,
    notes: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    audit_state: State<'_, DatabaseState>,
) -> Result<(), String> {
    // Validate session
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
    drop(session_manager_guard);

    // Only Engineers and Administrators can update firmware notes
    if session.role != UserRole::Engineer && session.role != UserRole::Administrator {
        warn!("User without sufficient permissions attempted to update firmware notes: {}", session.username);
        return Err("Only Engineers and Administrators can update firmware notes".to_string());
    }

    // Sanitize notes
    let sanitized_notes = InputSanitizer::sanitize_string(&notes);
    
    // Check for malicious input
    if InputSanitizer::is_potentially_malicious(&sanitized_notes) {
        error!("Potentially malicious input detected in firmware notes");
        return Err("Invalid input detected. Please avoid using special characters or script-like patterns.".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            
            match firmware_repo.update_firmware_notes(firmware_id, sanitized_notes.clone()) {
                Ok(_) => {
                    // Log audit event
                    if let Ok(audit_guard) = audit_state.lock() {
                        if let Some(audit_db) = audit_guard.as_ref() {
                            let audit_repo = SqliteAuditRepository::new(audit_db.get_connection());
                            let audit_event = audit::AuditEventRequest {
                                event_type: audit::AuditEventType::FirmwareNotesUpdate,
                                user_id: Some(session.user_id),
                                username: Some(session.username.clone()),
                                admin_user_id: None,
                                admin_username: None,
                                target_user_id: None,
                                target_username: None,
                                description: format!("Firmware {} notes updated", firmware_id),
                                metadata: Some(serde_json::json!({
                                    "firmware_id": firmware_id,
                                    "updated_by": session.username
                                }).to_string()),
                                ip_address: None,
                                user_agent: None,
                            };
                            if let Err(e) = audit_repo.log_event(&audit_event) {
                                error!("Failed to log audit event: {}", e);
                            }
                        }
                    }
                    
                    info!("Firmware {} notes updated by {}", firmware_id, session.username);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to update firmware notes: {}", e);
                    Err(format!("Failed to update firmware notes: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

// ========== VAULT COMMANDS ==========

#[tauri::command]
async fn create_identity_vault(
    token: String,
    vault_request: CreateVaultRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<vault::IdentityVault, String> {
    // Validate session
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
    drop(session_manager_guard);

    // Validate inputs
    let sanitized_name = InputSanitizer::sanitize_string(&vault_request.name);
    let sanitized_description = InputSanitizer::sanitize_string(&vault_request.description);
    
    if InputSanitizer::is_potentially_malicious(&sanitized_name) ||
       InputSanitizer::is_potentially_malicious(&sanitized_description) {
        error!("Potentially malicious input detected in create_identity_vault");
        return Err("Invalid input detected. Please avoid using special characters or script-like patterns.".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            let sanitized_request = CreateVaultRequest {
                asset_id: vault_request.asset_id,
                name: sanitized_name,
                description: sanitized_description,
                created_by: session.user_id,
            };

            match vault_repo.create_vault(sanitized_request) {
                Ok(vault) => {
                    info!("Identity vault created by {}: {}", session.username, vault.name);
                    Ok(vault)
                }
                Err(e) => {
                    error!("Failed to create identity vault: {}", e);
                    Err(format!("Failed to create identity vault: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn add_vault_secret(
    token: String,
    secret_request: AddSecretRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<vault::VaultSecret, String> {
    // Validate session
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
    drop(session_manager_guard);

    // Validate inputs
    let sanitized_label = InputSanitizer::sanitize_string(&secret_request.label);
    let sanitized_value = InputSanitizer::sanitize_string(&secret_request.value);
    
    if InputSanitizer::is_potentially_malicious(&sanitized_label) ||
       InputSanitizer::is_potentially_malicious(&sanitized_value) {
        error!("Potentially malicious input detected in add_vault_secret");
        return Err("Invalid input detected. Please avoid using special characters or script-like patterns.".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            let sanitized_request = AddSecretRequest {
                vault_id: secret_request.vault_id,
                secret_type: secret_request.secret_type,
                label: sanitized_label,
                value: sanitized_value,
                author_id: session.user_id,
            };

            match vault_repo.add_secret(sanitized_request) {
                Ok(secret) => {
                    info!("Secret added to vault by {}: {} ({})", 
                          session.username, secret.label, secret.secret_type.to_string());
                    Ok(secret)
                }
                Err(e) => {
                    error!("Failed to add secret to vault: {}", e);
                    Err(format!("Failed to add secret to vault: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_vault_by_asset_id(
    token: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Option<VaultInfo>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            match vault_repo.get_vault_by_asset_id(asset_id) {
                Ok(vault_info) => {
                    if vault_info.is_some() {
                        info!("Vault retrieved for asset {} by {}", asset_id, session.username);
                    }
                    Ok(vault_info)
                }
                Err(e) => {
                    error!("Failed to get vault for asset {}: {}", asset_id, e);
                    Err(format!("Failed to get vault: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_vault_history(
    token: String,
    vault_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<vault::VaultVersion>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            match vault_repo.get_vault_history(vault_id) {
                Ok(history) => {
                    info!("Vault history retrieved for vault {} by {}", vault_id, session.username);
                    Ok(history)
                }
                Err(e) => {
                    error!("Failed to get vault history for vault {}: {}", vault_id, e);
                    Err(format!("Failed to get vault history: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn import_vault_from_recovery(
    token: String,
    recovery_file_path: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<IdentityVault, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            // Read and parse the vault file from recovery package
            let vault_json = std::fs::read_to_string(&recovery_file_path)
                .map_err(|e| format!("Failed to read recovery vault file: {}", e))?;
            
            let vault_info: VaultInfo = serde_json::from_str(&vault_json)
                .map_err(|e| format!("Failed to parse vault data: {}", e))?;
            
            // Verify the asset_id matches
            if vault_info.vault.asset_id != asset_id {
                return Err("Vault asset ID does not match target asset".to_string());
            }
            
            // Import the vault
            match vault_repo.import_vault(&vault_info, session.user_id) {
                Ok(imported_vault) => {
                    info!("Vault imported from recovery by {}: Vault '{}' for Asset {}", 
                          session.username, imported_vault.name, asset_id);
                    Ok(imported_vault)
                },
                Err(e) => {
                    error!("Failed to import vault from recovery: {}", e);
                    Err(format!("Failed to import vault: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn decrypt_vault_secret(
    token: String,
    secret_id: i64,
    vault_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<String, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            match vault_repo.get_secret_by_id(secret_id) {
                Ok(Some(secret)) => {
                    // Decrypt the secret value
                    let encryption = FileEncryption::new(&format!("vault_{}_{}", vault_id, session.user_id));
                    use base64::{Engine as _, engine::general_purpose};
                    match general_purpose::STANDARD.decode(&secret.encrypted_value) {
                        Ok(encrypted_bytes) => {
                            match encryption.decrypt(&encrypted_bytes) {
                                Ok(decrypted_bytes) => {
                                    match String::from_utf8(decrypted_bytes) {
                                        Ok(decrypted_value) => {
                                            info!("Secret decrypted for user {}: {} ({})", 
                                                  session.username, secret.label, secret.secret_type.to_string());
                                            Ok(decrypted_value)
                                        }
                                        Err(e) => {
                                            error!("Failed to convert decrypted bytes to string: {}", e);
                                            Err("Failed to decrypt secret".to_string())
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to decrypt secret: {}", e);
                                    Err("Failed to decrypt secret".to_string())
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to decode base64 encrypted value: {}", e);
                            Err("Failed to decrypt secret".to_string())
                        }
                    }
                }
                Ok(None) => Err("Secret not found".to_string()),
                Err(e) => {
                    error!("Failed to get secret {}: {}", secret_id, e);
                    Err(format!("Failed to get secret: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn export_vault(
    token: String,
    vault_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<String, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            // Get the vault by ID
            match vault_repo.get_vault_by_id(vault_id) {
                Ok(Some(vault)) => {
                    // Get all secrets for this vault
                    match vault_repo.get_vault_secrets(vault_id) {
                        Ok(secrets) => {
                            // Construct VaultInfo for export
                            let vault_info = VaultInfo {
                                vault,
                                secrets: secrets.clone(),
                                secret_count: secrets.len(),
                            };
                            
                            // Serialize to JSON string for export
                            match serde_json::to_string_pretty(&vault_info) {
                                Ok(json_string) => {
                                    info!("Vault {} exported by {}: {} secrets", vault_id, session.username, vault_info.secret_count);
                                    Ok(json_string)
                                }
                                Err(e) => {
                                    error!("Failed to serialize vault data: {}", e);
                                    Err("Failed to serialize vault data".to_string())
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to get vault secrets for vault {}: {}", vault_id, e);
                            Err(format!("Failed to get vault secrets: {}", e))
                        }
                    }
                }
                Ok(None) => Err("Vault not found".to_string()),
                Err(e) => {
                    error!("Failed to get vault {}: {}", vault_id, e);
                    Err(format!("Failed to get vault: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn generate_secure_password(
    token: String,
    request: GeneratePasswordRequest,
    session_manager: State<'_, SessionManagerState>,
) -> Result<String, String> {
    // Validate session
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
    drop(session_manager_guard);

    match PasswordGenerator::generate(&request) {
        Ok(password) => {
            info!("Password generated by {}", session.username);
            Ok(password)
        }
        Err(e) => {
            error!("Failed to generate password: {}", e);
            Err(format!("Failed to generate password: {}", e))
        }
    }
}

#[tauri::command]
async fn validate_password_strength(
    token: String,
    password: String,
    session_manager: State<'_, SessionManagerState>,
) -> Result<PasswordStrength, String> {
    // Validate session
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
    drop(session_manager_guard);

    let strength = PasswordStrengthAnalyzer::analyze(&password);
    info!("Password strength analyzed by {} - Score: {}", session.username, strength.score);
    Ok(strength)
}

#[tauri::command]
async fn check_password_reuse(
    token: String,
    password: String,
    exclude_secret_id: Option<i64>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<bool, String> {
    // Validate session
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
    drop(session_manager_guard);

    // Hash the password for comparison
    let password_hash = match PasswordGenerator::hash_password(&password) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Failed to hash password for reuse check: {}", e);
            return Err("Failed to process password".to_string());
        }
    };

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            match vault_repo.check_password_reuse(&password_hash, exclude_secret_id) {
                Ok(is_reused) => {
                    info!("Password reuse check by {}: {}", session.username, is_reused);
                    Ok(is_reused)
                }
                Err(e) => {
                    error!("Failed to check password reuse: {}", e);
                    Err(format!("Failed to check password reuse: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_password_history(
    token: String,
    secret_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<PasswordHistory>, String> {
    // Validate session
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
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            match vault_repo.get_password_history(secret_id) {
                Ok(history) => {
                    info!("Password history retrieved by {} for secret {}", session.username, secret_id);
                    Ok(history)
                }
                Err(e) => {
                    error!("Failed to get password history: {}", e);
                    Err(format!("Failed to get password history: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn update_credential_password(
    token: String,
    request: UpdateCredentialPasswordRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
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
    drop(session_manager_guard);

    // Validate inputs
    let sanitized_password = InputSanitizer::sanitize_string(&request.new_password);
    if InputSanitizer::is_potentially_malicious(&sanitized_password) {
        error!("Potentially malicious input detected in update_credential_password");
        return Err("Invalid input detected".to_string());
    }

    // Analyze password strength
    let strength = PasswordStrengthAnalyzer::analyze(&request.new_password);
    
    // Hash password for storage
    let password_hash = match PasswordGenerator::hash_password(&request.new_password) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Failed to hash password: {}", e);
            return Err("Failed to process password".to_string());
        }
    };

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            // Check for password reuse
            match vault_repo.check_password_reuse(&password_hash, Some(request.secret_id)) {
                Ok(is_reused) => {
                    if is_reused {
                        return Err("Password has been used before on another credential".to_string());
                    }
                }
                Err(e) => {
                    error!("Failed to check password reuse: {}", e);
                    return Err("Failed to validate password uniqueness".to_string());
                }
            }
            
            match vault_repo.update_password(request, &password_hash, strength.score) {
                Ok(()) => {
                    info!("Password updated by {} for secret with strength score {}", session.username, strength.score);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to update password: {}", e);
                    Err(format!("Failed to update password: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Standalone credential commands for Story 4.3

#[tauri::command]
async fn create_standalone_credential(
    token: String,
    credential_request: CreateStandaloneCredentialRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<StandaloneCredentialInfo, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    // Sanitize input
    let sanitized_name = InputSanitizer::sanitize_string(&credential_request.name);
    let sanitized_description = InputSanitizer::sanitize_string(&credential_request.description);
    let sanitized_value = InputSanitizer::sanitize_string(&credential_request.value);
    let sanitized_tags = credential_request.tags.as_ref().map(|tags| 
        tags.iter().map(|tag| InputSanitizer::sanitize_string(tag)).collect()
    );

    // Check for malicious input
    if InputSanitizer::is_potentially_malicious(&sanitized_name) ||
       InputSanitizer::is_potentially_malicious(&sanitized_description) ||
       InputSanitizer::is_potentially_malicious(&sanitized_value) {
        error!("Potentially malicious input detected in create_standalone_credential");
        return Err("Invalid input detected. Please avoid using special characters or script-like patterns.".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            let sanitized_request = CreateStandaloneCredentialRequest {
                name: sanitized_name,
                description: sanitized_description,
                credential_type: credential_request.credential_type,
                category_id: credential_request.category_id,
                value: sanitized_value,
                tags: sanitized_tags,
                created_by: session.user_id,
            };

            match vault_repo.create_standalone_credential(sanitized_request) {
                Ok(credential) => {
                    info!("Standalone credential created by {}: {}", session.username, credential.name);
                    
                    // Fetch the full credential info
                    match vault_repo.get_standalone_credential(credential.id) {
                        Ok(Some(credential_info)) => Ok(credential_info),
                        Ok(None) => Err("Failed to retrieve created credential".to_string()),
                        Err(e) => {
                            error!("Failed to retrieve credential info: {}", e);
                            Err(format!("Failed to retrieve credential info: {}", e))
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to create standalone credential: {}", e);
                    Err(format!("Failed to create standalone credential: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn search_credentials(
    token: String,
    search_request: SearchCredentialsRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<vault::SearchCredentialsResponse, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let _session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            // Sanitize search query if provided
            let sanitized_request = SearchCredentialsRequest {
                query: search_request.query.map(|q| InputSanitizer::sanitize_string(&q)),
                credential_type: search_request.credential_type,
                category_id: search_request.category_id,
                tags: search_request.tags.map(|tags| 
                    tags.iter().map(|tag| InputSanitizer::sanitize_string(tag)).collect()
                ),
                created_after: search_request.created_after,
                created_before: search_request.created_before,
                limit: search_request.limit,
                offset: search_request.offset,
            };

            match vault_repo.search_standalone_credentials(sanitized_request) {
                Ok(response) => Ok(response),
                Err(e) => {
                    error!("Failed to search credentials: {}", e);
                    Err(format!("Failed to search credentials: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_credential_categories(
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<CategoryWithChildren>, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let _session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            match vault_repo.get_credential_categories() {
                Ok(categories) => Ok(categories),
                Err(e) => {
                    error!("Failed to get credential categories: {}", e);
                    Err(format!("Failed to get credential categories: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn manage_credential_categories(
    token: String,
    action: String,
    category_id: Option<i64>,
    category_request: Option<CreateCategoryRequest>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<String, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let _session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            match action.as_str() {
                "create" => {
                    if let Some(request) = category_request {
                        let sanitized_request = CreateCategoryRequest {
                            name: InputSanitizer::sanitize_string(&request.name),
                            description: request.description.map(|d| InputSanitizer::sanitize_string(&d)),
                            parent_category_id: request.parent_category_id,
                            color_code: request.color_code,
                            icon: request.icon,
                        };

                        match vault_repo.create_credential_category(sanitized_request) {
                            Ok(category) => Ok(format!("Created category: {}", category.name)),
                            Err(e) => Err(format!("Failed to create category: {}", e)),
                        }
                    } else {
                        Err("Category request data required for create action".to_string())
                    }
                }
                "update" => {
                    if let (Some(cat_id), Some(request)) = (category_id, category_request) {
                        let sanitized_request = CreateCategoryRequest {
                            name: InputSanitizer::sanitize_string(&request.name),
                            description: request.description.map(|d| InputSanitizer::sanitize_string(&d)),
                            parent_category_id: request.parent_category_id,
                            color_code: request.color_code,
                            icon: request.icon,
                        };

                        match vault_repo.update_credential_category(cat_id, sanitized_request) {
                            Ok(_) => Ok(format!("Updated category ID: {}", cat_id)),
                            Err(e) => Err(format!("Failed to update category: {}", e)),
                        }
                    } else {
                        Err("Category ID and request data required for update action".to_string())
                    }
                }
                "delete" => {
                    if let Some(cat_id) = category_id {
                        match vault_repo.delete_credential_category(cat_id) {
                            Ok(_) => Ok(format!("Deleted category ID: {}", cat_id)),
                            Err(e) => Err(format!("Failed to delete category: {}", e)),
                        }
                    } else {
                        Err("Category ID required for delete action".to_string())
                    }
                }
                _ => Err("Invalid action. Must be 'create', 'update', or 'delete'".to_string()),
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_credential_history(
    token: String,
    credential_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<vault::StandaloneCredentialHistory>, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let _session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            match vault_repo.get_standalone_credential_history(credential_id) {
                Ok(history) => {
                    info!("Retrieved history for credential {}", credential_id);
                    Ok(history)
                }
                Err(e) => {
                    error!("Failed to get credential history: {}", e);
                    Err(format!("Failed to get credential history: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn update_standalone_credential(
    token: String,
    update_request: UpdateStandaloneCredentialRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<String, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            let sanitized_request = UpdateStandaloneCredentialRequest {
                id: update_request.id,
                name: update_request.name.map(|n| InputSanitizer::sanitize_string(&n)),
                description: update_request.description.map(|d| InputSanitizer::sanitize_string(&d)),
                category_id: update_request.category_id,
                value: update_request.value.map(|v| InputSanitizer::sanitize_string(&v)),
                author_id: session.user_id,
            };

            match vault_repo.update_standalone_credential(sanitized_request) {
                Ok(_) => {
                    info!("Updated standalone credential {} by {}", update_request.id, session.username);
                    Ok("Credential updated successfully".to_string())
                }
                Err(e) => {
                    error!("Failed to update standalone credential: {}", e);
                    Err(format!("Failed to update credential: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn delete_standalone_credential(
    token: String,
    credential_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<String, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            match vault_repo.delete_standalone_credential(credential_id, session.user_id) {
                Ok(_) => {
                    info!("Deleted standalone credential {} by {}", credential_id, session.username);
                    Ok("Credential deleted successfully".to_string())
                }
                Err(e) => {
                    error!("Failed to delete standalone credential: {}", e);
                    Err(format!("Failed to delete credential: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn get_standalone_credential(
    token: String,
    credential_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<StandaloneCredentialInfo, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let _session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            match vault_repo.get_standalone_credential(credential_id) {
                Ok(Some(credential_info)) => Ok(credential_info),
                Ok(None) => Err("Credential not found".to_string()),
                Err(e) => {
                    error!("Failed to get standalone credential: {}", e);
                    Err(format!("Failed to get credential: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn decrypt_standalone_credential(
    token: String,
    credential_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<String, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            match vault_repo.get_standalone_credential(credential_id) {
                Ok(Some(credential_info)) => {
                    // Decrypt the credential value
                    let encryption = FileEncryption::new(&format!("standalone_{}_{}", credential_id, session.user_id));
                    use base64::{Engine as _, engine::general_purpose};
                    
                    let encrypted_bytes = general_purpose::STANDARD.decode(&credential_info.credential.encrypted_data)
                        .map_err(|e| format!("Failed to decode encrypted data: {}", e))?;
                    
                    let decrypted_bytes = encryption.decrypt(&encrypted_bytes)
                        .map_err(|e| format!("Failed to decrypt credential: {}", e))?;
                    
                    let decrypted_value = String::from_utf8(decrypted_bytes)
                        .map_err(|e| format!("Failed to convert decrypted data to string: {}", e))?;
                    
                    info!("Decrypted standalone credential {} for user {}", credential_id, session.username);
                    Ok(decrypted_value)
                }
                Ok(None) => Err("Credential not found".to_string()),
                Err(e) => {
                    error!("Failed to get standalone credential: {}", e);
                    Err(format!("Failed to get credential: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportOptions {
    include_vault: bool,
    vault_available: bool,
    vault_secret_count: usize,
}

#[tauri::command]
async fn get_export_options(
    token: String,
    asset_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<ExportOptions, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let _session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            // Check if vault exists for this asset
            let vault_info = vault_repo.get_vault_by_asset_id(asset_id)
                .map_err(|e| format!("Failed to check vault existence: {}", e))?;
            
            let (vault_available, vault_secret_count) = match vault_info {
                Some(info) => (true, info.secret_count),
                None => (false, 0),
            };
            
            Ok(ExportOptions {
                include_vault: vault_available, // Default to include if available
                vault_available,
                vault_secret_count,
            })
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BundlePreview {
    manifest: RecoveryManifest,
    estimated_size: i64,
}

#[tauri::command]
async fn preview_recovery_bundle(
    token: String,
    asset_id: i64,
    config_version_id: i64,
    firmware_version_id: i64,
    include_vault: Option<bool>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<BundlePreview, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            let asset_repo = SqliteAssetRepository::new(db.get_connection());
            
            // Get asset details
            let asset = match asset_repo.get_asset_by_id(asset_id) {
                Ok(Some(asset)) => asset,
                Ok(None) => return Err("Asset not found".to_string()),
                Err(e) => return Err(format!("Failed to get asset: {}", e)),
            };
            
            // Get configuration details
            let config = config_repo.get_configuration_by_id(config_version_id)
                .map_err(|e| format!("Failed to get configuration: {}", e))?
                .ok_or_else(|| "Configuration not found".to_string())?;
            
            // Get firmware details
            let firmware = firmware_repo.get_firmware_by_id(firmware_version_id)
                .map_err(|e| format!("Failed to get firmware: {}", e))?
                .ok_or_else(|| "Firmware not found".to_string())?;
            
            // Get vault info if requested
            let vault_info = if include_vault.unwrap_or(false) {
                vault_repo.get_vault_by_asset_id(asset_id)
                    .map_err(|e| format!("Failed to get vault info: {}", e))?
            } else {
                None
            };
            
            // Create preview manifest
            use recovery::{ConfigurationExportInfo, FirmwareExportInfo, VaultExportInfo};
            
            let mut estimated_size = config.file_size;
            estimated_size += 1024 * 1024; // Default 1MB for firmware size estimate
            
            let vault_export_info = vault_info.as_ref().map(|info| {
                // Estimate vault size
                let vault_size_estimate = 1024 + (info.secret_count as i64 * 512); // Rough estimate
                estimated_size += vault_size_estimate;
                
                VaultExportInfo {
                    vault_id: info.vault.id,
                    vault_name: info.vault.name.clone(),
                    filename: format!("{}_vault.json", RecoveryExporter::sanitize_filename(&asset.name)),
                    checksum: "preview".to_string(),
                    secret_count: info.secret_count,
                    file_size: vault_size_estimate,
                    encrypted: true,
                }
            });
            
            let manifest = RecoveryManifest {
                asset_id,
                export_date: chrono::Utc::now().to_rfc3339(),
                exported_by: session.username.clone(),
                configuration: ConfigurationExportInfo {
                    version_id: config.id,
                    version_number: config.version_number.clone(),
                    filename: format!("{}_config_v{}.{}", 
                        RecoveryExporter::sanitize_filename(&asset.name),
                        config.version_number,
                        std::path::Path::new(&config.file_name)
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("json")
                    ),
                    checksum: "preview".to_string(),
                    file_size: config.file_size,
                },
                firmware: FirmwareExportInfo {
                    version_id: firmware.id,
                    version: firmware.version.clone(),
                    filename: format!("{}_firmware_v{}.bin", 
                        RecoveryExporter::sanitize_filename(&asset.name),
                        firmware.version
                    ),
                    checksum: "preview".to_string(),
                    vendor: firmware.vendor.unwrap_or_else(|| "Unknown".to_string()),
                    model: firmware.model.unwrap_or_else(|| "Unknown".to_string()),
                    file_size: 1024 * 1024, // Default firmware size
                },
                vault: vault_export_info,
                compatibility_verified: config.firmware_version_id == Some(firmware_version_id),
            };
            
            estimated_size += 4096; // Add manifest file size
            
            Ok(BundlePreview {
                manifest,
                estimated_size,
            })
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn import_recovery_bundle(
    app: AppHandle,
    token: String,
    bundle_path: String,
    target_asset_id: i64,
    import_vault: bool,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<RecoveryManifest, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let config_repo = SqliteConfigurationRepository::new(db.get_connection());
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            let audit_repo = SqliteAuditRepository::new(db.get_connection());
            
            let importer = RecoveryImporter::new(&config_repo, &firmware_repo, &vault_repo, &audit_repo);
            
            let request = RecoveryImportRequest {
                bundle_path,
                target_asset_id,
                import_vault,
            };

            match importer.import_recovery_bundle(
                &app,
                request,
                session.user_id,
                &session.username,
                &session.role,
            ) {
                Ok(manifest) => Ok(manifest),
                Err(e) => Err(e.to_string()),
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
async fn validate_bundle_integrity(
    token: String,
    bundle_path: String,
    session_manager: State<'_, SessionManagerState>,
) -> Result<RecoveryManifest, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let _session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    match RecoveryImporter::validate_bundle_integrity(&bundle_path) {
        Ok(manifest) => Ok(manifest),
        Err(e) => Err(e.to_string()),
    }
}

// Vault permission commands for Story 4.5
#[tauri::command]
async fn check_vault_access(
    token: String,
    vault_id: i64,
    permission_type: PermissionType,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<VaultAccessInfo, String> {
    // Validate session and get current user
    let user = {
        let session_manager = session_manager.lock()
            .map_err(|_| "Failed to acquire session lock".to_string())?;
        session_manager.validate_session(&token)
            .map_err(|e| e.to_string())?
            .ok_or("Invalid session")?
    };

    info!("User {} checking access to vault {} with permission {:?}", 
          user.username, vault_id, permission_type);

    // Get database connection
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;

    // Get a proper database connection wrapped in Arc<Mutex<>>
    let db_conn = {
        let conn_path = db.get_connection().path()
            .ok_or("Failed to get database path")?;
        let new_conn = Connection::open(conn_path)
            .map_err(|e| format!("Failed to open database connection: {}", e))?;
        Arc::new(Mutex::new(new_conn))
    };
    let access_control = VaultAccessControlService::new(db_conn);
    
    // Get user details
    let user_repo = SqliteUserRepository::new(db.get_connection());
    let full_user = user_repo.find_by_id(user.user_id)
        .map_err(|e| e.to_string())?
        .ok_or("User not found")?;
    
    access_control.check_vault_access(&full_user, vault_id, permission_type)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn grant_vault_access(
    token: String,
    request: GrantVaultAccessRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<VaultPermission, String> {
    // Validate session and get current user
    let admin_user = {
        let session_manager = session_manager.lock()
            .map_err(|_| "Failed to acquire session lock".to_string())?;
        session_manager.validate_session(&token)
            .map_err(|e| e.to_string())?
            .ok_or("Invalid session")?
    };

    // Verify administrator role
    if admin_user.role != UserRole::Administrator {
        return Err("Only administrators can grant vault access".to_string());
    }

    info!("Administrator {} granting {} access to user {} for vault {}", 
          admin_user.username, request.permission_type.to_string(), 
          request.user_id, request.vault_id);

    // Get database connection
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;

    // Get target user details for audit
    let user_repo = SqliteUserRepository::new(db.get_connection());
    let target_user = user_repo.find_by_id(request.user_id)
        .map_err(|e| e.to_string())?
        .ok_or("Target user not found")?;

    // Grant the permission - Set the correct granted_by value
    let mut updated_request = request.clone();
    updated_request.granted_by = admin_user.user_id;
    
    let vault_repo = SqliteVaultRepository::new(db.get_connection());
    let permission = vault_repo.grant_vault_access(updated_request)
        .map_err(|e| e.to_string())?;

    // Log audit event
    let audit_repo = SqliteAuditRepository::new(db.get_connection());
    let audit_event = create_vault_access_granted_event(
        admin_user.user_id,
        &admin_user.username,
        request.user_id,
        &target_user.username,
        request.vault_id,
        &request.permission_type.to_string(),
    );
    
    audit_repo.log_event(&audit_event)
        .map_err(|e| e.to_string())?;

    Ok(permission)
}

#[tauri::command]
async fn revoke_vault_access(
    token: String,
    request: RevokeVaultAccessRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session and get current user
    let admin_user = {
        let session_manager = session_manager.lock()
            .map_err(|_| "Failed to acquire session lock".to_string())?;
        session_manager.validate_session(&token)
            .map_err(|e| e.to_string())?
            .ok_or("Invalid session")?
    };

    // Verify administrator role
    if admin_user.role != UserRole::Administrator {
        return Err("Only administrators can revoke vault access".to_string());
    }

    let permission_desc = request.permission_type
        .as_ref()
        .map(|p| p.to_string())
        .unwrap_or_else(|| "all".to_string());

    info!("Administrator {} revoking {} access from user {} for vault {}", 
          admin_user.username, permission_desc, request.user_id, request.vault_id);

    // Get database connection
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;

    // Get target user details for audit
    let user_repo = SqliteUserRepository::new(db.get_connection());
    let target_user = user_repo.find_by_id(request.user_id)
        .map_err(|e| e.to_string())?
        .ok_or("Target user not found")?;

    // Revoke the permission
    let vault_repo = SqliteVaultRepository::new(db.get_connection());
    vault_repo.revoke_vault_access(request.clone())
        .map_err(|e| e.to_string())?;

    // Log audit event
    let audit_repo = SqliteAuditRepository::new(db.get_connection());
    let permission_str = request.permission_type.as_ref().map(|p| p.to_string());
    let audit_event = create_vault_access_revoked_event(
        admin_user.user_id,
        &admin_user.username,
        request.user_id,
        &target_user.username,
        request.vault_id,
        permission_str.as_deref(),
    );
    
    audit_repo.log_event(&audit_event)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn get_user_vault_permissions(
    token: String,
    user_id: Option<i64>,
    vault_id: Option<i64>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<VaultPermission>, String> {
    // Validate session and get current user
    let current_user = {
        let session_manager = session_manager.lock()
            .map_err(|_| "Failed to acquire session lock".to_string())?;
        session_manager.validate_session(&token)
            .map_err(|e| e.to_string())?
    };

    let current_user = current_user.ok_or("Invalid session")?;

    // Determine which user's permissions to query
    let target_user_id = if let Some(uid) = user_id {
        // Only administrators can query other users' permissions
        if uid != current_user.user_id && current_user.role != UserRole::Administrator {
            return Err("Only administrators can view other users' permissions".to_string());
        }
        uid
    } else {
        // Default to current user
        current_user.user_id
    };

    info!("Getting vault permissions for user {}", target_user_id);

    // Get database connection
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;

    // Get permissions
    let vault_repo = SqliteVaultRepository::new(db.get_connection());
    vault_repo.get_user_vault_permissions(target_user_id, vault_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_vault_permissions(
    token: String,
    vault_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<VaultPermission>, String> {
    // Validate session and get current user
    let user = {
        let session_manager = session_manager.lock()
            .map_err(|_| "Failed to acquire session lock".to_string())?;
        session_manager.validate_session(&token)
            .map_err(|e| e.to_string())?
    };

    let user = user.ok_or("Invalid session")?;

    // Only administrators can view vault permissions
    if user.role != UserRole::Administrator {
        return Err("Only administrators can view vault permissions".to_string());
    }

    info!("Administrator {} viewing permissions for vault {}", user.username, vault_id);

    // Get database connection
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;

    // Get vault permissions
    let vault_repo = SqliteVaultRepository::new(db.get_connection());
    vault_repo.get_vault_permissions(vault_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_vault_access_log(
    token: String,
    vault_id: i64,
    limit: Option<i32>,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<VaultAccessLog>, String> {
    // Validate session and get current user
    let user = {
        let session_manager = session_manager.lock()
            .map_err(|_| "Failed to acquire session lock".to_string())?;
        session_manager.validate_session(&token)
            .map_err(|e| e.to_string())?
    };

    let user = user.ok_or("Invalid session")?;

    // Only administrators can view access logs
    if user.role != UserRole::Administrator {
        return Err("Only administrators can view vault access logs".to_string());
    }

    info!("Administrator {} viewing access log for vault {}", user.username, vault_id);

    // Get database connection
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;

    // Get access log
    let vault_repo = SqliteVaultRepository::new(db.get_connection());
    vault_repo.get_vault_access_log(vault_id, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_permission_request(
    token: String,
    request: CreatePermissionRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<PermissionRequest, String> {
    // Validate session and get current user
    let user = {
        let session_manager = session_manager.lock()
            .map_err(|_| "Failed to acquire session lock".to_string())?;
        session_manager.validate_session(&token)
            .map_err(|e| e.to_string())?
    };

    let user = user.ok_or("Invalid session")?;

    info!("User {} requesting {} permission for vault {}", 
          user.username, request.requested_permission.to_string(), request.vault_id);

    // Get database connection
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;

    // Create the permission request
    let vault_repo = SqliteVaultRepository::new(db.get_connection());
    let permission_request = vault_repo.create_permission_request(request.clone(), user.user_id)
        .map_err(|e| e.to_string())?;

    // Log audit event
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;
    
    let audit_repo = SqliteAuditRepository::new(db.get_connection());
    let audit_event = AuditEventRequest {
        event_type: AuditEventType::VaultPermissionRequested,
        user_id: Some(user.user_id),
        username: Some(user.username.clone()),
        admin_user_id: None,
        admin_username: None,
        target_user_id: None,
        target_username: None,
        description: format!("User '{}' requested {} permission for vault {}", 
                           user.username, request.requested_permission.to_string(), request.vault_id),
        metadata: Some(format!(r#"{{"vault_id": {}, "permission_type": "{}"}}"#, 
                             request.vault_id, request.requested_permission.to_string())),
        ip_address: None,
        user_agent: None,
    };
    
    audit_repo.log_event(&audit_event)
        .map_err(|e| e.to_string())?;

    Ok(permission_request)
}

// Task 5: Password Rotation Commands for Story 4.6

// Task 5.1: Rotate password command
#[tauri::command]
async fn rotate_password(
    token: String,
    request: PasswordRotationRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    // Check permissions - Engineers can rotate passwords
    if session.role != UserRole::Administrator && session.role != UserRole::Engineer {
        return Err("Insufficient permissions to rotate passwords".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            let vault_repo = Box::new(SqliteVaultRepository::new(conn));
            let audit_repo = Box::new(SqliteAuditRepository::new(conn));
            
            let rotation_service = PasswordRotationService::new(conn, vault_repo, audit_repo);
            
            // Validate rotation before executing
            rotation_service.validate_rotation(request.secret_id, &request.new_password)
                .map_err(|e| e.to_string())?;
            
            // Execute rotation with author_id set to session user
            let mut rotation_request = request;
            rotation_request.author_id = session.user_id;
            
            rotation_service.rotate_password(rotation_request)
                .map_err(|e| {
                    error!("Failed to rotate password: {}", e);
                    format!("Failed to rotate password: {}", e)
                })?;
            
            info!("Password rotated successfully by user {}", session.username);
            Ok(())
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Task 5.2: Get rotation schedule command
#[tauri::command]
async fn get_rotation_schedule(
    token: String,
    vault_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Option<RotationSchedule>, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            let scheduler = RotationScheduler::new(conn);
            
            scheduler.get_active_schedule(vault_id)
                .map_err(|e| format!("Failed to get rotation schedule: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Task 5.3: Create rotation batch command
#[tauri::command]
async fn create_rotation_batch(
    token: String,
    request: CreateRotationBatchRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<RotationBatch, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Check permissions
    if session.role != UserRole::Administrator && session.role != UserRole::Engineer {
        return Err("Insufficient permissions to create rotation batch".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            let vault_repo = Box::new(SqliteVaultRepository::new(conn));
            let audit_repo = Box::new(SqliteAuditRepository::new(conn));
            
            let batch_service = BatchRotationService::new(conn, vault_repo, audit_repo);
            
            // Set created_by to session user
            let mut batch_request = request;
            batch_request.created_by = session.user_id;
            
            batch_service.create_batch(batch_request)
                .map_err(|e| format!("Failed to create rotation batch: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Task 5.4: Get rotation history command
#[tauri::command]
async fn get_rotation_history(
    token: String,
    secret_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<PasswordRotationHistory>, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            let vault_repo = Box::new(SqliteVaultRepository::new(conn));
            let audit_repo = Box::new(SqliteAuditRepository::new(conn));
            
            let rotation_service = PasswordRotationService::new(conn, vault_repo, audit_repo);
            
            rotation_service.get_rotation_history(secret_id)
                .map_err(|e| format!("Failed to get rotation history: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Task 5.5: Update rotation policy command
#[tauri::command]
async fn update_rotation_policy(
    token: String,
    request: UpdateRotationScheduleRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Only administrators can update rotation policies
    if session.role != UserRole::Administrator {
        return Err("Only administrators can update rotation policies".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            let scheduler = RotationScheduler::new(conn);
            
            scheduler.update_rotation_schedule(request)
                .map_err(|e| format!("Failed to update rotation policy: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Task 5.6: Get rotation alerts command
#[tauri::command]
async fn get_rotation_alerts(
    token: String,
    days_ahead: i32,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<RotationAlert>, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            let scheduler = RotationScheduler::new(conn);
            
            scheduler.get_rotation_alerts(days_ahead)
                .map_err(|e| format!("Failed to get rotation alerts: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Additional rotation commands

// Execute batch rotation
#[tauri::command]
async fn execute_batch_rotation(
    token: String,
    request: BatchRotationRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Check permissions
    if session.role != UserRole::Administrator && session.role != UserRole::Engineer {
        return Err("Insufficient permissions to execute rotation batch".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            let vault_repo = Box::new(SqliteVaultRepository::new(conn));
            let audit_repo = Box::new(SqliteAuditRepository::new(conn));
            
            let batch_service = BatchRotationService::new(conn, vault_repo, audit_repo);
            
            // Set author_id to session user
            let mut batch_request = request;
            batch_request.author_id = session.user_id;
            
            batch_service.execute_batch_rotation(batch_request)
                .map_err(|e| format!("Failed to execute batch rotation: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Create rotation schedule
#[tauri::command]
async fn create_rotation_schedule(
    token: String,
    request: CreateRotationScheduleRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<RotationSchedule, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    // Only administrators can create rotation schedules
    if session.role != UserRole::Administrator {
        return Err("Only administrators can create rotation schedules".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            let scheduler = RotationScheduler::new(conn);
            
            // Set created_by to session user
            let mut schedule_request = request;
            schedule_request.created_by = session.user_id;
            
            scheduler.create_rotation_schedule(schedule_request)
                .map_err(|e| format!("Failed to create rotation schedule: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Get rotation compliance metrics
#[tauri::command]
async fn get_rotation_compliance_metrics(
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            let scheduler = RotationScheduler::new(conn);
            
            scheduler.get_rotation_compliance_metrics()
                .map_err(|e| format!("Failed to get compliance metrics: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Get batch rotation history
#[tauri::command]
async fn get_batch_rotation_history(
    token: String,
    limit: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<RotationBatch>, String> {
    // Validate session
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err("Session validation error".to_string()),
    };
    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            let vault_repo = Box::new(SqliteVaultRepository::new(conn));
            let audit_repo = Box::new(SqliteAuditRepository::new(conn));
            
            let batch_service = BatchRotationService::new(conn, vault_repo, audit_repo);
            
            batch_service.get_batch_history(limit)
                .map_err(|e| format!("Failed to get batch history: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
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
            reactivate_user,
            create_asset,
            get_dashboard_assets,
            get_dashboard_stats,
            get_asset_details,
            import_configuration,
            get_configuration_versions,
            update_configuration_status,
            get_configuration_status_history,
            get_available_status_transitions,
            promote_to_golden,
            promote_branch_to_silver,
            get_golden_version,
            get_promotion_eligibility,
            export_configuration_version,
            get_file_metadata,
            create_branch,
            get_branches,
            get_branch_details,
            import_version_to_branch,
            get_branch_versions,
            get_branch_latest_version,
            compare_branch_versions,
            archive_version,
            restore_version,
            link_firmware_to_configuration,
            unlink_firmware_from_configuration,
            get_configurations_by_firmware,
            export_complete_recovery,
            upload_firmware,
            get_firmware_list,
            delete_firmware,
            get_firmware_analysis,
            retry_firmware_analysis,
            update_firmware_status,
            get_firmware_status_history,
            get_available_firmware_status_transitions,
            promote_firmware_to_golden,
            update_firmware_notes,
            create_identity_vault,
            add_vault_secret,
            get_vault_by_asset_id,
            get_vault_history,
            decrypt_vault_secret,
            export_vault,
            import_vault_from_recovery,
            generate_secure_password,
            validate_password_strength,
            check_password_reuse,
            get_password_history,
            update_credential_password,
            // Standalone credential commands
            create_standalone_credential,
            search_credentials,
            get_credential_categories,
            manage_credential_categories,
            get_credential_history,
            update_standalone_credential,
            delete_standalone_credential,
            get_standalone_credential,
            decrypt_standalone_credential,
            // Recovery bundle commands
            get_export_options,
            preview_recovery_bundle,
            import_recovery_bundle,
            validate_bundle_integrity,
            // Vault permission commands for Story 4.5
            check_vault_access,
            grant_vault_access,
            revoke_vault_access,
            get_user_vault_permissions,
            get_vault_permissions,
            get_vault_access_log,
            create_permission_request,
            // Password rotation commands for Story 4.6
            rotate_password,
            get_rotation_schedule,
            create_rotation_batch,
            get_rotation_history,
            update_rotation_policy,
            get_rotation_alerts,
            execute_batch_rotation,
            create_rotation_schedule,
            get_rotation_compliance_metrics,
            get_batch_rotation_history
        ])
        .setup(|_app| {
            info!("Ferrocodex application starting up...");
            
            // Analysis queue will be initialized after database is ready
            // For now, we'll initialize it on first use
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
