// Vault management commands
// Extracted from lib.rs.backup

use crate::auth::{Session, SessionManager, verify_password};
use crate::users::{UserRepository, SqliteUserRepository, UserRole, UserInfo};
use crate::audit::{AuditRepository, SqliteAuditRepository, AuditEventRequest, AuditEventType, create_vault_access_granted_event, create_vault_access_revoked_event, create_vault_access_denied_event};
use crate::validation::{InputSanitizer};
use crate::vault::{
    VaultRepository, SqliteVaultRepository, CreateVaultRequest, AddSecretRequest, VaultInfo, VaultSecret,
    IdentityVault, GeneratePasswordRequest, UpdateCredentialPasswordRequest, UpdateVaultSecretRequest, 
    DeleteVaultSecretRequest, PasswordStrength, PasswordHistory, PasswordGenerator, PasswordStrengthAnalyzer,
    CreateStandaloneCredentialRequest, UpdateStandaloneCredentialRequest, SearchCredentialsRequest, 
    CreateCategoryRequest, StandaloneCredentialInfo, CategoryWithChildren, VaultAccessControlService,
    password_services::PasswordReuseChecker, PermissionType, VaultAccessInfo, GrantVaultAccessRequest, VaultPermission, RevokeVaultAccessRequest,
    VaultAccessLog, CreatePermissionRequest, PermissionRequest, open_vault_export, seal_vault_export,
    rotation::{
        PasswordRotationService, PasswordRotationRequest, RotationScheduler, RotationSchedule,
        RotationBatch, BatchRotationService, PasswordRotationHistory, CreateRotationBatchRequest,
        UpdateRotationScheduleRequest, RotationAlert, CreateRotationScheduleRequest, BatchRotationRequest
    }
};
use crate::encryption::FileEncryption;
use crate::database::Database;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};
use tracing::{error, info, warn};
use zeroize::Zeroizing;
use serde::{Serialize, Deserialize};

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;

const ACCESS_DENIED: &str = "Access denied";

/// Whether the session user holds `permission` on a vault. Logs nothing, so
/// list commands can use it to filter results.
pub(crate) fn can_access_vault(
    conn: &Connection,
    session: &Session,
    vault_id: i64,
    permission: PermissionType,
) -> Result<bool, String> {
    VaultAccessControlService::new(conn)
        .evaluate_access(session.user_id, &session.role, vault_id, permission)
        .map(|access| access.has_access)
        .map_err(|e| {
            error!("Failed to check vault access: {}", e);
            "Failed to check vault access".to_string()
        })
}

/// Fails with "Access denied" unless the session user holds `permission` on the
/// vault. Non-administrator checks go to the vault access log, and denials are
/// also written to the audit trail.
pub(crate) fn require_vault_permission(
    conn: &Connection,
    session: &Session,
    vault_id: i64,
    permission: PermissionType,
) -> Result<(), String> {
    let access = VaultAccessControlService::new(conn)
        .check_access(session.user_id, &session.role, vault_id, permission)
        .map_err(|e| {
            error!("Failed to check vault access: {}", e);
            "Failed to check vault access".to_string()
        })?;
    if access.has_access {
        return Ok(());
    }

    warn!("User {} denied {} access to vault {}", session.username, permission.to_string(), vault_id);
    let event = create_vault_access_denied_event(
        session.user_id,
        &session.username,
        vault_id,
        &permission.to_string(),
        "no matching vault permission",
    );
    if let Err(e) = SqliteAuditRepository::new(conn).log_event(&event) {
        error!("Failed to record vault access denial: {}", e);
    }
    Err(ACCESS_DENIED.to_string())
}

/// Loads a secret and requires `permission` on the vault it belongs to.
fn require_secret_permission(
    conn: &Connection,
    session: &Session,
    secret_id: i64,
    permission: PermissionType,
) -> Result<VaultSecret, String> {
    let secret = SqliteVaultRepository::new(conn)
        .get_secret_by_id(secret_id)
        .map_err(|e| {
            error!("Failed to get secret {}: {}", secret_id, e);
            format!("Failed to get secret: {}", e)
        })?
        .ok_or("Secret not found")?;
    require_vault_permission(conn, session, secret.vault_id, permission)?;
    Ok(secret)
}

#[tauri::command]
pub async fn create_identity_vault(
    token: String,
    vault_request: CreateVaultRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<IdentityVault, String> {
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
pub async fn add_vault_secret(
    token: String,
    secret_request: AddSecretRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<crate::vault::VaultSecret, String> {
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

    let data_key = session.data_key()?;

    // Validate inputs
    let sanitized_label = InputSanitizer::sanitize_string(&secret_request.label);
    // Secret values are opaque: preserved byte-for-byte, not run through the heuristic.
    let sanitized_value = secret_request.value.clone();
    InputSanitizer::validate_secret_value(&sanitized_value)?;
    
    if InputSanitizer::is_potentially_malicious(&sanitized_label) {
        error!("Potentially malicious input detected in add_vault_secret");
        return Err("Invalid input detected. Please avoid using special characters or script-like patterns.".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            require_vault_permission(db.get_connection(), &session, secret_request.vault_id, PermissionType::Write)?;
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            let sanitized_request = AddSecretRequest {
                vault_id: secret_request.vault_id,
                secret_type: secret_request.secret_type,
                label: sanitized_label,
                value: sanitized_value,
                author_id: session.user_id,
            };

            match vault_repo.add_secret(sanitized_request, &data_key) {
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
pub async fn get_vault_by_asset_id(
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
                Ok(Some(mut vault_info)) => {
                    // Without Read, show that the vault exists (so access can be
                    // requested) but not its secrets
                    if !can_access_vault(db.get_connection(), &session, vault_info.vault.id, PermissionType::Read)? {
                        vault_info.secrets.clear();
                        vault_info.secret_count = 0;
                    }
                    info!("Vault retrieved for asset {} by {}", asset_id, session.username);
                    Ok(Some(vault_info))
                }
                Ok(None) => Ok(None),
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
pub async fn get_vault_history(
    token: String,
    vault_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<crate::vault::VaultVersion>, String> {
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
            require_vault_permission(db.get_connection(), &session, vault_id, PermissionType::Read)?;
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
pub async fn import_vault_from_recovery(
    token: String,
    recovery_file_path: String,
    asset_id: i64,
    passphrase: String,
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
    drop(session_manager_guard);

    let data_key = session.data_key()?;

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            // Read and parse the vault file from recovery package
            let vault_json = std::fs::read_to_string(&recovery_file_path)
                .map_err(|e| format!("Failed to read recovery vault file: {}", e))?;
            
            let vault_info = open_vault_export(&vault_json, &passphrase, &data_key)
                .map_err(|e| format!("Failed to open vault export: {}", e))?;
            
            // Verify the asset_id matches
            if vault_info.vault.asset_id != asset_id {
                return Err("Vault asset ID does not match target asset".to_string());
            }
            
            // Import the vault
            // open_vault_export re-encrypted the secrets under the session key
            match vault_repo.import_vault(&vault_info, session.user_id, data_key.version()) {
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
pub async fn decrypt_vault_secret(
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

    let data_key = session.data_key()?;

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            // Check the vault the secret actually belongs to, not just the one passed in
            let secret = require_secret_permission(db.get_connection(), &session, secret_id, PermissionType::Read)?;
            if secret.vault_id != vault_id {
                warn!("User {} requested secret {} through vault {}, but it belongs to vault {}",
                      session.username, secret_id, vault_id, secret.vault_id);
                return Err("Secret not found".to_string());
            }

            // Decrypt the secret value
            let decrypted = FileEncryption::new(&data_key)
                .decrypt_from_base64(&secret.encrypted_value)
                .map(Zeroizing::new)
                .and_then(|bytes| Ok(std::str::from_utf8(&bytes)?.to_owned()));
            match decrypted {
                Ok(decrypted_value) => {
                    info!("Secret decrypted for user {}: {} ({})",
                          session.username, secret.label, secret.secret_type.to_string());
                    Ok(decrypted_value)
                }
                Err(e) => {
                    error!("Failed to decrypt secret: {}", e);
                    Err("Failed to decrypt secret".to_string())
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn export_vault(
    token: String,
    vault_id: i64,
    passphrase: String,
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

    let data_key = session.data_key()?;

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            require_vault_permission(db.get_connection(), &session, vault_id, PermissionType::Export)?;
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
                            
                            // Re-encrypt the secrets under the export passphrase
                            match seal_vault_export(&vault_info, &data_key, &passphrase) {
                                Ok(json_string) => {
                                    info!("Vault {} exported by {}: {} secrets", vault_id, session.username, vault_info.secret_count);
                                    Ok(json_string)
                                }
                                Err(e) => {
                                    error!("Failed to export vault data: {}", e);
                                    Err(format!("Failed to export vault: {}", e))
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
pub async fn generate_secure_password(
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
pub async fn validate_password_strength(
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

/// Checks whether `password` matches anything in the password history of one secret.
///
/// This is deliberately scoped to a single secret the caller holds Write on. A global
/// reuse check would be an oracle for other vaults' current passwords.
#[tauri::command]
pub async fn check_password_reuse(
    token: String,
    password: String,
    secret_id: i64,
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

    // Fetch the hashes under the DB lock; verify (slow bcrypt) after releasing it.
    let hashes: Vec<String> = {
        let db_guard = db_state.lock()
            .map_err(|_| "Failed to acquire database lock".to_string())?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        secret_reuse_hashes(db.get_connection(), &session, secret_id)?
    };

    let is_reused = PasswordReuseChecker::is_password_reused(&password, &hashes);
    info!("Password reuse check by {} for secret {}: {}", session.username, secret_id, is_reused);
    Ok(is_reused)
}

/// Authorizes (Write on the secret's vault) and returns the hashes of one secret's own history.
fn secret_reuse_hashes(
    conn: &Connection,
    session: &Session,
    secret_id: i64,
) -> Result<Vec<String>, String> {
    require_secret_permission(conn, session, secret_id, PermissionType::Write)?;
    Ok(SqliteVaultRepository::new(conn)
        .get_password_history(secret_id)
        .map_err(|e| {
            error!("Failed to load password history: {}", e);
            format!("Failed to check password reuse: {}", e)
        })?
        .into_iter()
        .map(|h| h.password_hash)
        .collect())
}

#[tauri::command]
pub async fn get_password_history(
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
            require_secret_permission(db.get_connection(), &session, secret_id, PermissionType::Read)?;
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
pub async fn update_credential_password(
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

    let data_key = session.data_key()?;

    // Validate inputs
    // Passwords are opaque: preserved byte-for-byte, not run through the heuristic.
    InputSanitizer::validate_secret_value(&request.new_password)?;

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
            require_secret_permission(db.get_connection(), &session, request.secret_id, PermissionType::Write)?;
            let request = UpdateCredentialPasswordRequest { author_id: session.user_id, ..request };
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            match vault_repo.update_password(request, &password_hash, strength.score, &data_key) {
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

#[tauri::command]
pub async fn update_vault_secret(
    token: String,
    request: UpdateVaultSecretRequest,
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

    let data_key = session.data_key()?;

    // Validate inputs
    if let Some(ref label) = request.label {
        let sanitized_label = InputSanitizer::sanitize_string(label);
        if InputSanitizer::is_potentially_malicious(&sanitized_label) {
            error!("Potentially malicious input detected in update_vault_secret");
            return Err("Invalid input detected".to_string());
        }
    }
    
    if let Some(ref value) = request.value {
        // Secret values are opaque: preserved byte-for-byte, not run through the heuristic.
        InputSanitizer::validate_secret_value(value)?;
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            require_secret_permission(db.get_connection(), &session, request.secret_id, PermissionType::Write)?;
            let request = UpdateVaultSecretRequest { author_id: session.user_id, ..request };
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            match vault_repo.update_vault_secret(request, &data_key) {
                Ok(()) => {
                    info!("Vault secret updated by {}", session.username);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to update vault secret: {}", e);
                    Err(format!("Failed to update secret: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn delete_vault_secret(
    token: String,
    request: DeleteVaultSecretRequest,
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

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            require_secret_permission(db.get_connection(), &session, request.secret_id, PermissionType::Write)?;
            let request = DeleteVaultSecretRequest { author_id: session.user_id, ..request };
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            match vault_repo.delete_vault_secret(request) {
                Ok(()) => {
                    info!("Vault secret deleted by {}", session.username);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to delete vault secret: {}", e);
                    Err(format!("Failed to delete secret: {}", e))
                }
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Standalone credential commands for Story 4.3
//
// Authorization model (deliberate, see PR #187): standalone credentials are shared
// across the organization. They have no vault and no per-credential grants, so any
// authenticated user may list, search, view the history of, and decrypt any of them.
// Only the creator or an Administrator may update or delete a credential. Credential
// categories have no owner, so updating or deleting one is Administrator-only, while
// any user may create one. Every decryption is written to the audit trail
// (`VaultSecretAccessed`).

#[tauri::command]
pub async fn create_standalone_credential(
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
    drop(session_manager_guard);

    let data_key = session.data_key()?;

    // Sanitize input
    let sanitized_name = InputSanitizer::sanitize_string(&credential_request.name);
    let sanitized_description = InputSanitizer::sanitize_string(&credential_request.description);
    // Secret values are opaque: preserved byte-for-byte, not run through the heuristic.
    let sanitized_value = credential_request.value.clone();
    InputSanitizer::validate_secret_value(&sanitized_value)?;
    let sanitized_tags = credential_request.tags.as_ref().map(|tags| 
        tags.iter().map(|tag| InputSanitizer::sanitize_string(tag)).collect()
    );

    // Check for malicious input
    if InputSanitizer::is_potentially_malicious(&sanitized_name) ||
       InputSanitizer::is_potentially_malicious(&sanitized_description) {
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

            match vault_repo.create_standalone_credential(sanitized_request, &data_key) {
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
pub async fn search_credentials(
    token: String,
    search_request: SearchCredentialsRequest,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<crate::vault::SearchCredentialsResponse, String> {
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
pub async fn get_credential_categories(
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
    drop(session_manager_guard);

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
pub async fn manage_credential_categories(
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
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    // Categories have no owner: anyone may create one, only administrators may change or remove one
    if matches!(action.as_str(), "update" | "delete") && session.role != UserRole::Administrator {
        warn!("User {} denied '{}' on credential category", session.username, action);
        return Err("Only administrators can update or delete credential categories".to_string());
    }

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
pub async fn get_credential_history(
    token: String,
    credential_id: i64,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<Vec<crate::vault::StandaloneCredentialHistory>, String> {
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
pub async fn update_standalone_credential(
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

    drop(session_manager_guard);

    let data_key = session.data_key()?;

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;
    let vault_repo = SqliteVaultRepository::new(db.get_connection());

    let credential = vault_repo.get_standalone_credential(update_request.id)
        .map_err(|e| format!("Failed to load credential: {}", e))?
        .ok_or("Credential not found")?
        .credential;

    // Only administrators or the credential's creator may edit it
    if session.role != UserRole::Administrator && credential.created_by != session.user_id {
        warn!("User {} denied updating standalone credential {} owned by user {}",
              session.username, update_request.id, credential.created_by);
        return Err("Only administrators or the credential's creator can edit it".to_string());
    }

    if let Some(ref v) = update_request.value {
        InputSanitizer::validate_secret_value(v)?;
    }

    let sanitized_request = UpdateStandaloneCredentialRequest {
        id: update_request.id,
        name: update_request.name.map(|n| InputSanitizer::sanitize_string(&n)),
        description: update_request.description.map(|d| InputSanitizer::sanitize_string(&d)),
        category_id: update_request.category_id,
        value: update_request.value.clone(),
        author_id: session.user_id,
    };

    match vault_repo.update_standalone_credential(sanitized_request, &data_key) {
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

#[tauri::command]
pub async fn delete_standalone_credential(
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

    drop(session_manager_guard);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;
    let conn = db.get_connection();
    let vault_repo = SqliteVaultRepository::new(conn);

    let credential = vault_repo.get_standalone_credential(credential_id)
        .map_err(|e| format!("Failed to load credential: {}", e))?
        .ok_or("Credential not found")?
        .credential;

    // Only administrators or the credential's creator may delete it
    if session.role != UserRole::Administrator && credential.created_by != session.user_id {
        warn!("User {} denied deleting standalone credential {} owned by user {}",
              session.username, credential_id, credential.created_by);
        return Err("Only administrators or the credential's creator can delete it".to_string());
    }

    // The credential's history rows cascade-delete with it, so the audit log is the
    // durable record. Delete and audit atomically so neither happens without the other.
    let tx = conn.unchecked_transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    vault_repo.delete_standalone_credential(credential_id, session.user_id)
        .map_err(|e| {
            error!("Failed to delete standalone credential: {}", e);
            format!("Failed to delete credential: {}", e)
        })?;

    let audit_repo = SqliteAuditRepository::new(conn);
    let audit_event = AuditEventRequest {
        event_type: AuditEventType::VaultSecretDeleted,
        user_id: Some(session.user_id),
        username: Some(session.username.clone()),
        admin_user_id: None,
        admin_username: None,
        target_user_id: Some(credential.created_by),
        target_username: None,
        description: format!("Deleted standalone credential '{}'", credential.name),
        metadata: Some(serde_json::json!({
            "standalone_credential_id": credential_id,
            "name": credential.name,
            "credential_type": credential.credential_type.to_string(),
            "created_by": credential.created_by,
        }).to_string()),
        ip_address: None,
        user_agent: None,
    };
    audit_repo.log_event(&audit_event)
        .map_err(|e| format!("Failed to record audit event: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit credential deletion: {}", e))?;

    info!("Deleted standalone credential {} by {}", credential_id, session.username);
    Ok("Credential deleted successfully".to_string())
}

#[tauri::command]
pub async fn get_standalone_credential(
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
    drop(session_manager_guard);

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
pub async fn decrypt_standalone_credential(
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
    drop(session_manager_guard);

    let data_key = session.data_key()?;

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());

            match vault_repo.get_standalone_credential(credential_id) {
                Ok(Some(credential_info)) => {
                    // Decrypt the credential value
                    // The plaintext bytes are wiped on drop, including on the error path.
                    // Only the returned String (handed to the IPC layer) outlives this scope.
                    let decrypted_bytes = Zeroizing::new(
                        FileEncryption::new(&data_key)
                            .decrypt_from_base64(&credential_info.credential.encrypted_data)
                            .map_err(|e| format!("Failed to decrypt credential: {}", e))?,
                    );

                    let decrypted_value = std::str::from_utf8(&decrypted_bytes)
                        .map_err(|e| format!("Failed to convert decrypted data to string: {}", e))?
                        .to_owned();
                    
                    let audit_event = AuditEventRequest {
                        event_type: AuditEventType::VaultSecretAccessed,
                        user_id: Some(session.user_id),
                        username: Some(session.username.clone()),
                        admin_user_id: None,
                        admin_username: None,
                        target_user_id: Some(credential_info.credential.created_by),
                        target_username: None,
                        description: format!("Decrypted standalone credential '{}'", credential_info.credential.name),
                        metadata: Some(serde_json::json!({
                            "standalone_credential_id": credential_id,
                            "name": credential_info.credential.name,
                            "credential_type": credential_info.credential.credential_type.to_string(),
                            "created_by": credential_info.credential.created_by,
                        }).to_string()),
                        ip_address: None,
                        user_agent: None,
                    };
                    SqliteAuditRepository::new(db.get_connection()).log_event(&audit_event)
                        .map_err(|e| {
                            error!("Failed to record credential access audit event: {}", e);
                            "Failed to record audit event".to_string()
                        })?;

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

// Password Rotation Commands for Story 4.6

#[tauri::command]
pub async fn rotate_password(
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

    let data_key = session.data_key()?;

    // Check permissions - Engineers can rotate passwords
    if session.role != UserRole::Administrator && session.role != UserRole::Engineer {
        return Err("Insufficient permissions to rotate passwords".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            require_secret_permission(conn, &session, request.secret_id, PermissionType::Write)?;
            let vault_repo = Box::new(SqliteVaultRepository::new(conn));
            let audit_repo = Box::new(SqliteAuditRepository::new(conn));

            let rotation_service = PasswordRotationService::new(conn, vault_repo, audit_repo);

            // Validate rotation before executing
            rotation_service.validate_rotation(request.secret_id, &request.new_password)
                .map_err(|e| e.to_string())?;
            
            // Execute rotation with author_id set to session user
            let mut rotation_request = request;
            rotation_request.author_id = session.user_id;
            
            rotation_service.rotate_password(rotation_request, &data_key)
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

#[tauri::command]
pub async fn get_rotation_schedule(
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
            require_vault_permission(conn, &session, vault_id, PermissionType::Read)?;
            let scheduler = RotationScheduler::new(conn);

            scheduler.get_active_schedule(vault_id)
                .map_err(|e| format!("Failed to get rotation schedule: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn create_rotation_batch(
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

#[tauri::command]
pub async fn get_rotation_history(
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
            require_secret_permission(conn, &session, secret_id, PermissionType::Read)?;
            let vault_repo = Box::new(SqliteVaultRepository::new(conn));
            let audit_repo = Box::new(SqliteAuditRepository::new(conn));

            let rotation_service = PasswordRotationService::new(conn, vault_repo, audit_repo);

            rotation_service.get_rotation_history(secret_id)
                .map_err(|e| format!("Failed to get rotation history: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

// Vault permission commands for Story 4.5

#[tauri::command]
pub async fn check_vault_access(
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

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;

    let access_control = VaultAccessControlService::new(db.get_connection());
    let user_repo = SqliteUserRepository::new(db.get_connection());
    let full_user = user_repo.find_by_id(user.user_id)
        .map_err(|e| e.to_string())?
        .ok_or("User not found")?;
    
    access_control.check_vault_access(&full_user, vault_id, permission_type)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn grant_vault_access(
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
    if admin_user.role != crate::users::UserRole::Administrator {
        return Err("Only administrators can grant vault access".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;

    // Verify target user exists
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
pub async fn revoke_vault_access(
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
    if admin_user.role != crate::users::UserRole::Administrator {
        return Err("Only administrators can revoke vault access".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;

    // Verify target user exists
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
pub async fn get_user_vault_permissions(
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

    // Determine target user - default to current user if not specified
    let target_user_id = user_id.unwrap_or(current_user.user_id);

    // Only administrators can view other users' permissions
    if target_user_id != current_user.user_id && current_user.role != crate::users::UserRole::Administrator {
        return Err("Insufficient permissions to view other users' vault permissions".to_string());
    }

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
pub async fn get_vault_permissions(
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
    if user.role != crate::users::UserRole::Administrator {
        return Err("Only administrators can view vault permissions".to_string());
    }

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
pub async fn get_vault_access_log(
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
    if user.role != crate::users::UserRole::Administrator {
        return Err("Only administrators can view vault access logs".to_string());
    }

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
pub async fn create_permission_request(
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

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;

    // Create the permission request
    let vault_repo = SqliteVaultRepository::new(db.get_connection());
    let permission_request = vault_repo.create_permission_request(request.clone(), user.user_id)
        .map_err(|e| e.to_string())?;

    // Log audit event
    let audit_repo = SqliteAuditRepository::new(db.get_connection());
    let audit_event = crate::audit::AuditEventRequest {
        event_type: crate::audit::AuditEventType::VaultPermissionRequested,
        user_id: Some(user.user_id),
        username: Some(user.username.clone()),
        admin_user_id: None,
        admin_username: None,
        target_user_id: None,
        target_username: None,
        description: format!("Permission requested for vault {}", request.vault_id),
        metadata: Some(serde_json::json!({
            "vault_id": request.vault_id,
            "permission_type": request.requested_permission.to_string()
        }).to_string()),
        ip_address: None,
        user_agent: None,
    };
    
    audit_repo.log_event(&audit_event)
        .map_err(|e| e.to_string())?;

    Ok(permission_request)
}

// Additional rotation commands

#[tauri::command]
pub async fn get_rotation_alerts(
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

            let alerts = scheduler.get_rotation_alerts(days_ahead)
                .map_err(|e| format!("Failed to get rotation alerts: {}", e))?;

            // Only show alerts for vaults the user can read
            let mut readable = std::collections::HashMap::new();
            let mut visible = Vec::with_capacity(alerts.len());
            for alert in alerts {
                let can_read = match readable.get(&alert.vault_id) {
                    Some(&can_read) => can_read,
                    None => {
                        let can_read = can_access_vault(conn, &session, alert.vault_id, PermissionType::Read)?;
                        readable.insert(alert.vault_id, can_read);
                        can_read
                    }
                };
                if can_read {
                    visible.push(alert);
                }
            }
            Ok(visible)
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn execute_batch_rotation(
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

    let data_key = session.data_key()?;

    // Check permissions
    if session.role != crate::users::UserRole::Administrator && session.role != crate::users::UserRole::Engineer {
        return Err("Insufficient permissions to execute batch rotation".to_string());
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let conn = db.get_connection();
            // Every secret in the batch must be writable before any is rotated
            for item in &request.items {
                require_secret_permission(conn, &session, item.secret_id, PermissionType::Write)?;
            }
            let vault_repo = Box::new(SqliteVaultRepository::new(conn));
            let audit_repo = Box::new(SqliteAuditRepository::new(conn));

            let batch_service = BatchRotationService::new(conn, vault_repo, audit_repo);

            // Set author_id to session user
            let mut batch_request = request;
            batch_request.author_id = session.user_id;
            
            batch_service.execute_batch_rotation(batch_request, &data_key)
                .map_err(|e| format!("Failed to execute batch rotation: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn create_rotation_schedule(
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
    if session.role != crate::users::UserRole::Administrator {
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

#[tauri::command]
pub async fn get_rotation_compliance_metrics(
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

#[tauri::command]
pub async fn get_batch_rotation_history(
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
                .map_err(|e| format!("Failed to get batch rotation history: {}", e))
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn update_rotation_policy(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::test_key;
    use crate::vault::{CreateStandaloneCredentialRequest, SecretType};
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
    use tauri::Manager;

    const ADMIN_ID: i64 = 1;
    const OWNER_ID: i64 = 2;
    const OTHER_ID: i64 = 3;

    /// Mock app with an admin, two engineers, and a credential owned by OWNER_ID.
    /// Returns the app, the credential id, and session tokens for (admin, owner, other).
    fn setup_test_app() -> (tauri::App<MockRuntime>, i64, [String; 3]) {
        let db = Database::new(":memory:".into()).unwrap();
        let conn = db.get_connection();
        conn.execute_batch(
            r#"
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'admin', 'hash', 'Administrator');
            INSERT INTO users (id, username, password_hash, role) VALUES (2, 'owner', 'hash', 'Engineer');
            INSERT INTO users (id, username, password_hash, role) VALUES (3, 'other', 'hash', 'Engineer');
            "#,
        ).unwrap();

        let credential = SqliteVaultRepository::new(conn)
            .create_standalone_credential(CreateStandaloneCredentialRequest {
                name: "PLC Login".to_string(),
                description: "".to_string(),
                credential_type: SecretType::Password,
                category_id: None,
                value: "secret".to_string(),
                tags: None,
                created_by: OWNER_ID,
            }, test_key())
            .unwrap();

        let session_manager = SessionManager::new();
        let user_repo = SqliteUserRepository::new(conn);
        let tokens = [ADMIN_ID, OWNER_ID, OTHER_ID].map(|id| {
            let user = user_repo.find_by_id(id).unwrap().unwrap();
            session_manager.create_session_with_key(&user, Some(test_key().clone())).unwrap().token
        });

        let app = mock_builder()
            .manage(DatabaseState::new(Some(db)))
            .manage(SessionManagerState::new(session_manager))
            .build(mock_context(noop_assets()))
            .unwrap();

        (app, credential.id, tokens)
    }

    fn credential_exists(app: &tauri::App<MockRuntime>, credential_id: i64) -> bool {
        let db_state = app.state::<DatabaseState>();
        let db_guard = db_state.lock().unwrap();
        SqliteVaultRepository::new(db_guard.as_ref().unwrap().get_connection())
            .get_standalone_credential(credential_id)
            .unwrap()
            .is_some()
    }

    fn deletion_audit_events(app: &tauri::App<MockRuntime>) -> Vec<crate::audit::AuditEvent> {
        let db_state = app.state::<DatabaseState>();
        let db_guard = db_state.lock().unwrap();
        SqliteAuditRepository::new(db_guard.as_ref().unwrap().get_connection())
            .get_events_by_type(&AuditEventType::VaultSecretDeleted)
            .unwrap()
    }

    fn credential_name(app: &tauri::App<MockRuntime>, credential_id: i64) -> String {
        let db_state = app.state::<DatabaseState>();
        let db_guard = db_state.lock().unwrap();
        SqliteVaultRepository::new(db_guard.as_ref().unwrap().get_connection())
            .get_standalone_credential(credential_id)
            .unwrap()
            .unwrap()
            .credential
            .name
    }

    fn rename_request(credential_id: i64) -> UpdateStandaloneCredentialRequest {
        UpdateStandaloneCredentialRequest {
            id: credential_id,
            name: Some("Renamed".to_string()),
            description: None,
            category_id: None,
            value: None,
            // Ignored: the command sets the author from the session
            author_id: 0,
        }
    }

    #[tokio::test]
    async fn test_owner_can_delete_credential_and_it_is_audited() {
        let (app, credential_id, [_, owner_token, _]) = setup_test_app();

        let result = delete_standalone_credential(owner_token, credential_id, app.state(), app.state()).await;

        assert!(result.is_ok(), "{:?}", result);
        assert!(!credential_exists(&app, credential_id));
        let events = deletion_audit_events(&app);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].user_id, Some(OWNER_ID));
        assert!(events[0].description.contains("PLC Login"));
    }

    #[tokio::test]
    async fn test_admin_can_delete_another_users_credential() {
        let (app, credential_id, [admin_token, _, _]) = setup_test_app();

        let result = delete_standalone_credential(admin_token, credential_id, app.state(), app.state()).await;

        assert!(result.is_ok(), "{:?}", result);
        assert!(!credential_exists(&app, credential_id));
        assert_eq!(deletion_audit_events(&app).len(), 1);
    }

    #[tokio::test]
    async fn test_non_owner_engineer_cannot_delete_credential() {
        let (app, credential_id, [_, _, other_token]) = setup_test_app();

        let result = delete_standalone_credential(other_token, credential_id, app.state(), app.state()).await;

        assert!(result.is_err());
        assert!(credential_exists(&app, credential_id));
        assert!(deletion_audit_events(&app).is_empty());
    }

    #[tokio::test]
    async fn test_owner_can_update_credential() {
        let (app, credential_id, [_, owner_token, _]) = setup_test_app();

        let result = update_standalone_credential(owner_token, rename_request(credential_id), app.state(), app.state()).await;

        assert!(result.is_ok(), "{:?}", result);
        assert_eq!(credential_name(&app, credential_id), "Renamed");
    }

    #[tokio::test]
    async fn test_admin_can_update_another_users_credential() {
        let (app, credential_id, [admin_token, _, _]) = setup_test_app();

        let result = update_standalone_credential(admin_token, rename_request(credential_id), app.state(), app.state()).await;

        assert!(result.is_ok(), "{:?}", result);
        assert_eq!(credential_name(&app, credential_id), "Renamed");
    }

    #[tokio::test]
    async fn test_non_owner_engineer_cannot_update_credential() {
        let (app, credential_id, [_, _, other_token]) = setup_test_app();

        let result = update_standalone_credential(other_token, rename_request(credential_id), app.state(), app.state()).await;

        assert!(result.is_err());
        assert_eq!(credential_name(&app, credential_id), "PLC Login");
    }

    #[tokio::test]
    async fn test_updating_missing_credential_fails() {
        let (app, credential_id, [admin_token, _, _]) = setup_test_app();

        let result = update_standalone_credential(admin_token, rename_request(credential_id + 1), app.state(), app.state()).await;

        assert_eq!(result, Err("Credential not found".to_string()));
    }

    #[tokio::test]
    async fn test_credential_decrypts_for_users_other_than_its_creator() {
        let (app, credential_id, [admin_token, owner_token, _]) = setup_test_app();

        for token in [owner_token, admin_token] {
            let result = decrypt_standalone_credential(token, credential_id, app.state(), app.state()).await;
            assert_eq!(result, Ok("secret".to_string()));
        }
    }

    fn access_audit_events(app: &tauri::App<MockRuntime>) -> Vec<crate::audit::AuditEvent> {
        let db_state = app.state::<DatabaseState>();
        let db_guard = db_state.lock().unwrap();
        SqliteAuditRepository::new(db_guard.as_ref().unwrap().get_connection())
            .get_events_by_type(&AuditEventType::VaultSecretAccessed)
            .unwrap()
    }

    #[tokio::test]
    async fn test_decrypting_standalone_credential_is_audited() {
        let (app, credential_id, [admin_token, _, other_token]) = setup_test_app();

        decrypt_standalone_credential(other_token, credential_id, app.state(), app.state()).await.unwrap();
        decrypt_standalone_credential(admin_token, credential_id, app.state(), app.state()).await.unwrap();

        let events = access_audit_events(&app);
        assert_eq!(events.len(), 2);
        let users: Vec<_> = events.iter().map(|e| e.user_id).collect();
        assert!(users.contains(&Some(OTHER_ID)) && users.contains(&Some(ADMIN_ID)));
        assert!(events[0].description.contains("PLC Login"));
        // The secret itself never reaches the audit trail
        assert!(events.iter().all(|e| !e.description.contains("secret\"") && !e.metadata.clone().unwrap_or_default().contains("\"secret\"")));
    }

    #[tokio::test]
    async fn test_failed_decrypt_is_not_audited() {
        let (app, credential_id, [_, owner_token, _]) = setup_test_app();

        let result = decrypt_standalone_credential(owner_token, credential_id + 1, app.state(), app.state()).await;

        assert_eq!(result, Err("Credential not found".to_string()));
        assert!(access_audit_events(&app).is_empty());
    }

    fn category_request(name: &str) -> Option<CreateCategoryRequest> {
        Some(CreateCategoryRequest {
            name: name.to_string(),
            description: None,
            parent_category_id: None,
            color_code: None,
            icon: None,
        })
    }

    #[tokio::test]
    async fn test_any_user_can_create_but_only_admin_can_change_categories() {
        let (app, _, [admin_token, _, other_token]) = setup_test_app();

        let created = manage_credential_categories(
            other_token.clone(), "create".into(), None, category_request("Vendors"), app.state(), app.state(),
        ).await;
        assert!(created.is_ok(), "{:?}", created);
        let category_id = {
            let categories = get_credential_categories(other_token.clone(), app.state(), app.state()).await.unwrap();
            categories.iter().find(|c| c.category.name == "Vendors").unwrap().category.id
        };

        let denied_update = manage_credential_categories(
            other_token.clone(), "update".into(), Some(category_id), category_request("Renamed"), app.state(), app.state(),
        ).await;
        assert!(denied_update.unwrap_err().contains("Only administrators"));
        let denied_delete = manage_credential_categories(
            other_token, "delete".into(), Some(category_id), None, app.state(), app.state(),
        ).await;
        assert!(denied_delete.unwrap_err().contains("Only administrators"));

        let updated = manage_credential_categories(
            admin_token.clone(), "update".into(), Some(category_id), category_request("Renamed"), app.state(), app.state(),
        ).await;
        assert!(updated.is_ok(), "{:?}", updated);
        let deleted = manage_credential_categories(
            admin_token, "delete".into(), Some(category_id), None, app.state(), app.state(),
        ).await;
        assert!(deleted.is_ok(), "{:?}", deleted);
    }

    /// Runs `$call` on another thread while this one holds the DB lock, so the command
    /// blocks waiting for the DB. A command that still holds the session-manager guard at
    /// that point would invert the lock order used elsewhere (DB first, then session).
    macro_rules! assert_session_lock_released_while_waiting_for_db {
        ($app:expr, |$db:ident, $sm:ident| $call:expr) => {{
            let db_state = $app.state::<DatabaseState>();
            let sm_state = $app.state::<SessionManagerState>();
            let db_guard = db_state.lock().unwrap();
            std::thread::scope(|scope| {
                let (thread_db, thread_sm) = (db_state.clone(), sm_state.clone());
                let handle = scope.spawn(move || {
                    let $db = thread_db;
                    let $sm = thread_sm;
                    let _ = tauri::async_runtime::block_on($call);
                });
                std::thread::sleep(std::time::Duration::from_millis(300));
                let session_lock_free = sm_state.try_lock().is_ok();
                drop(db_guard);
                handle.join().unwrap();
                assert!(session_lock_free, "session manager lock is held while waiting for the database lock");
            });
        }};
    }

    #[test]
    fn test_standalone_read_commands_do_not_hold_session_lock_while_taking_db_lock() {
        let (app, credential_id, [admin_token, _, _]) = setup_test_app();
        let admin_token = &admin_token;

        assert_session_lock_released_while_waiting_for_db!(app, |db, sm| search_credentials(
            admin_token.clone(),
            SearchCredentialsRequest {
                query: None, credential_type: None, category_id: None, tags: None,
                created_after: None, created_before: None, limit: None, offset: None,
            },
            db, sm,
        ));
        assert_session_lock_released_while_waiting_for_db!(app, |db, sm| get_credential_categories(admin_token.clone(), db, sm));
        assert_session_lock_released_while_waiting_for_db!(app, |db, sm| manage_credential_categories(
            admin_token.clone(), "create".into(), None, category_request("Locks"), db, sm,
        ));
        assert_session_lock_released_while_waiting_for_db!(app, |db, sm| get_credential_history(admin_token.clone(), credential_id, db, sm));
        assert_session_lock_released_while_waiting_for_db!(app, |db, sm| get_standalone_credential(admin_token.clone(), credential_id, db, sm));
    }

    #[tokio::test]
    async fn test_session_without_data_key_cannot_decrypt() {
        let (app, credential_id, _) = setup_test_app();
        let keyless_token = {
            let db_state = app.state::<DatabaseState>();
            let db_guard = db_state.lock().unwrap();
            let user = SqliteUserRepository::new(db_guard.as_ref().unwrap().get_connection())
                .find_by_id(OWNER_ID).unwrap().unwrap();
            app.state::<SessionManagerState>().lock().unwrap().create_session(&user).unwrap().token
        };

        let result = decrypt_standalone_credential(keyless_token, credential_id, app.state(), app.state()).await;

        assert!(result.unwrap_err().contains("Encrypted data is unavailable"));
    }

    /// Server-side vault permission enforcement
    mod vault_permissions {
        use super::*;
        use crate::vault::{AccessResult, GrantVaultAccessRequest};

        const GRANTEE_ID: i64 = 4;

        struct VaultFixture {
            app: tauri::App<MockRuntime>,
            /// Vault created by the administrator
            admin_vault: i64,
            admin_secret: i64,
            /// Vault created by OWNER_ID, an engineer
            owner_vault: i64,
            owner_secret: i64,
            admin: String,
            owner: String,
            other: String,
            grantee: String,
        }

        fn setup() -> VaultFixture {
            let db = Database::new(":memory:".into()).unwrap();
            let conn = db.get_connection();
            conn.execute_batch(
                r#"
                INSERT INTO users (id, username, password_hash, role) VALUES (1, 'admin', 'hash', 'Administrator');
                INSERT INTO users (id, username, password_hash, role) VALUES (2, 'owner', 'hash', 'Engineer');
                INSERT INTO users (id, username, password_hash, role) VALUES (3, 'other', 'hash', 'Engineer');
                INSERT INTO users (id, username, password_hash, role) VALUES (4, 'grantee', 'hash', 'Engineer');
                INSERT INTO assets (id, name, created_by) VALUES (1, 'PLC-1', 1);
                INSERT INTO assets (id, name, created_by) VALUES (2, 'PLC-2', 2);
                "#,
            ).unwrap();

            let repo = SqliteVaultRepository::new(conn);
            let add_vault = |asset_id: i64, created_by: i64, value: &str| {
                let vault = repo.create_vault(CreateVaultRequest {
                    asset_id,
                    name: format!("Vault {}", asset_id),
                    description: String::new(),
                    created_by,
                }).unwrap();
                let secret = repo.add_secret(AddSecretRequest {
                    vault_id: vault.id,
                    secret_type: SecretType::Password,
                    label: "admin login".to_string(),
                    value: value.to_string(),
                    author_id: created_by,
                }, test_key()).unwrap();
                (vault.id, secret.id)
            };
            let (admin_vault, admin_secret) = add_vault(1, ADMIN_ID, "admin-vault-pw");
            let (owner_vault, owner_secret) = add_vault(2, OWNER_ID, "owner-vault-pw");

            let session_manager = SessionManager::new();
            let user_repo = SqliteUserRepository::new(conn);
            let [admin, owner, other, grantee] = [ADMIN_ID, OWNER_ID, OTHER_ID, GRANTEE_ID].map(|id| {
                let user = user_repo.find_by_id(id).unwrap().unwrap();
                session_manager.create_session_with_key(&user, Some(test_key().clone())).unwrap().token
            });

            let app = mock_builder()
                .manage(DatabaseState::new(Some(db)))
                .manage(SessionManagerState::new(session_manager))
                .build(mock_context(noop_assets()))
                .unwrap();

            VaultFixture { app, admin_vault, admin_secret, owner_vault, owner_secret, admin, owner, other, grantee }
        }

        impl VaultFixture {
            fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> T) -> T {
                let db_state = self.app.state::<DatabaseState>();
                let db_guard = db_state.lock().unwrap();
                f(db_guard.as_ref().unwrap().get_connection())
            }

            fn grant(&self, user_id: i64, vault_id: i64, permission_type: PermissionType, expires_at: Option<String>) {
                self.with_conn(|conn| {
                    SqliteVaultRepository::new(conn).grant_vault_access(GrantVaultAccessRequest {
                        user_id,
                        vault_id,
                        permission_type,
                        granted_by: ADMIN_ID,
                        expires_at,
                    }).unwrap();
                });
            }

            fn denial_events(&self) -> Vec<crate::audit::AuditEvent> {
                self.with_conn(|conn| {
                    SqliteAuditRepository::new(conn)
                        .get_events_by_type(&AuditEventType::VaultAccessDenied)
                        .unwrap()
                })
            }

            fn secret_count(&self, vault_id: i64) -> usize {
                self.with_conn(|conn| SqliteVaultRepository::new(conn).get_vault_secrets(vault_id).unwrap().len())
            }

            async fn decrypt(&self, token: &str, secret_id: i64, vault_id: i64) -> Result<String, String> {
                decrypt_vault_secret(token.to_string(), secret_id, vault_id, self.app.state(), self.app.state()).await
            }

            async fn export(&self, token: &str, vault_id: i64) -> Result<String, String> {
                export_vault(token.to_string(), vault_id, "export passphrase".to_string(), self.app.state(), self.app.state()).await
            }

            async fn update_label(&self, token: &str, secret_id: i64) -> Result<(), String> {
                update_vault_secret(token.to_string(), UpdateVaultSecretRequest {
                    secret_id,
                    label: Some("renamed".to_string()),
                    value: None,
                    author_id: ADMIN_ID,
                }, self.app.state(), self.app.state()).await
            }

            async fn delete(&self, token: &str, secret_id: i64) -> Result<(), String> {
                delete_vault_secret(token.to_string(), DeleteVaultSecretRequest {
                    secret_id,
                    author_id: ADMIN_ID,
                }, self.app.state(), self.app.state()).await
            }
        }

        fn assert_denied<T: std::fmt::Debug>(result: Result<T, String>) {
            assert_eq!(result.unwrap_err(), ACCESS_DENIED);
        }

        #[tokio::test]
        async fn test_engineer_without_grant_cannot_decrypt_and_denial_is_recorded() {
            let f = setup();

            assert_denied(f.decrypt(&f.other, f.admin_secret, f.admin_vault).await);

            let events = f.denial_events();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].user_id, Some(OTHER_ID));
            let log = f.with_conn(|conn| SqliteVaultRepository::new(conn).get_user_access_log(OTHER_ID, None).unwrap());
            assert_eq!(log.len(), 1);
            assert_eq!(log[0].result, AccessResult::Denied);
        }

        #[tokio::test]
        async fn test_engineer_without_grant_cannot_export() {
            let f = setup();

            assert_denied(f.export(&f.other, f.admin_vault).await);
            assert_eq!(f.denial_events().len(), 1);
        }

        #[tokio::test]
        async fn test_engineer_with_read_grant_can_decrypt_but_not_export() {
            let f = setup();
            f.grant(GRANTEE_ID, f.admin_vault, PermissionType::Read, None);

            assert_eq!(f.decrypt(&f.grantee, f.admin_secret, f.admin_vault).await, Ok("admin-vault-pw".to_string()));
            assert_denied(f.export(&f.grantee, f.admin_vault).await);
        }

        #[tokio::test]
        async fn test_engineer_with_export_grant_can_export() {
            let f = setup();
            f.grant(GRANTEE_ID, f.admin_vault, PermissionType::Export, None);

            assert!(f.export(&f.grantee, f.admin_vault).await.is_ok());
        }

        #[tokio::test]
        async fn test_expired_grant_is_denied() {
            let f = setup();
            let expired = (chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339();
            f.grant(GRANTEE_ID, f.admin_vault, PermissionType::Read, Some(expired));

            assert_denied(f.decrypt(&f.grantee, f.admin_secret, f.admin_vault).await);
        }

        #[tokio::test]
        async fn test_admin_can_decrypt_and_export_any_vault() {
            let f = setup();

            assert_eq!(f.decrypt(&f.admin, f.owner_secret, f.owner_vault).await, Ok("owner-vault-pw".to_string()));
            assert!(f.export(&f.admin, f.owner_vault).await.is_ok());
            assert!(f.denial_events().is_empty());
        }

        #[tokio::test]
        async fn test_creator_can_read_write_and_export_own_vault() {
            let f = setup();

            assert_eq!(f.decrypt(&f.owner, f.owner_secret, f.owner_vault).await, Ok("owner-vault-pw".to_string()));
            let added = add_vault_secret(f.owner.clone(), AddSecretRequest {
                vault_id: f.owner_vault,
                secret_type: SecretType::IpAddress,
                label: "plc ip".to_string(),
                value: "10.0.0.5".to_string(),
                author_id: 0,
            }, f.app.state(), f.app.state()).await;
            assert!(added.is_ok(), "{:?}", added);
            assert!(f.export(&f.owner, f.owner_vault).await.is_ok());
            // Creating one vault gives no access to another
            assert_denied(f.decrypt(&f.owner, f.admin_secret, f.admin_vault).await);
            assert_denied(f.export(&f.owner, f.admin_vault).await);
        }

        #[tokio::test]
        async fn test_secret_from_another_vault_is_rejected() {
            let f = setup();
            f.grant(GRANTEE_ID, f.admin_vault, PermissionType::Read, None);

            // Readable vault id paired with a secret from an unreadable vault
            assert_denied(f.decrypt(&f.grantee, f.owner_secret, f.admin_vault).await);
            // Even with access to both vaults, the pair must match
            assert_eq!(
                f.decrypt(&f.admin, f.owner_secret, f.admin_vault).await,
                Err("Secret not found".to_string())
            );
        }

        #[tokio::test]
        async fn test_vault_lookup_hides_secrets_without_read() {
            let f = setup();

            let hidden = get_vault_by_asset_id(f.other.clone(), 1, f.app.state(), f.app.state()).await.unwrap().unwrap();
            assert_eq!(hidden.vault.id, f.admin_vault);
            assert!(hidden.secrets.is_empty());
            assert_eq!(hidden.secret_count, 0);

            let visible = get_vault_by_asset_id(f.admin.clone(), 1, f.app.state(), f.app.state()).await.unwrap().unwrap();
            assert_eq!(visible.secret_count, 1);
            // Filtering is silent
            assert!(f.denial_events().is_empty());
        }

        #[tokio::test]
        async fn test_modifying_secrets_requires_write() {
            let f = setup();
            f.grant(GRANTEE_ID, f.admin_vault, PermissionType::Read, None);

            assert_denied(f.update_label(&f.grantee, f.admin_secret).await);
            assert_denied(f.delete(&f.grantee, f.admin_secret).await);
            assert_eq!(f.secret_count(f.admin_vault), 1);

            f.grant(GRANTEE_ID, f.admin_vault, PermissionType::Write, None);
            assert!(f.update_label(&f.grantee, f.admin_secret).await.is_ok());
            assert!(f.delete(&f.grantee, f.admin_secret).await.is_ok());
            assert_eq!(f.secret_count(f.admin_vault), 0);
        }

        #[tokio::test]
        async fn test_password_history_requires_read() {
            let f = setup();

            assert_denied(get_password_history(f.other.clone(), f.admin_secret, f.app.state(), f.app.state()).await);
            assert!(get_password_history(f.admin.clone(), f.admin_secret, f.app.state(), f.app.state()).await.is_ok());
        }

        async fn set_password(f: &VaultFixture, token: &str, secret_id: i64, pw: &str) {
            update_credential_password(token.to_string(), UpdateCredentialPasswordRequest {
                secret_id,
                new_password: pw.to_string(),
                author_id: 0,
            }, f.app.state(), f.app.state()).await.unwrap();
        }

        async fn reuse(f: &VaultFixture, token: &str, pw: &str, secret_id: i64) -> Result<bool, String> {
            check_password_reuse(token.to_string(), pw.to_string(), secret_id, f.app.state(), f.app.state()).await
        }

        #[tokio::test]
        async fn test_reuse_check_requires_write_on_the_secrets_vault() {
            let f = setup();
            set_password(&f, &f.admin, f.admin_secret, "Target-Default-Pw-1!").await;

            assert_denied(reuse(&f, &f.other, "Target-Default-Pw-1!", f.admin_secret).await);
            // Read alone is not enough
            f.grant(GRANTEE_ID, f.admin_vault, PermissionType::Read, None);
            assert_denied(reuse(&f, &f.grantee, "Target-Default-Pw-1!", f.admin_secret).await);
            assert_eq!(f.denial_events().len(), 2);
        }

        #[tokio::test]
        async fn test_reuse_check_is_not_a_cross_vault_oracle() {
            let f = setup();
            set_password(&f, &f.admin, f.admin_secret, "Target-Default-Pw-1!").await;
            set_password(&f, &f.owner, f.owner_secret, "Owner-Own-Pw-2222!").await;

            // The owner can only test against their own secret: the other vault's current
            // password never matches, and the other vault's secret id is denied outright.
            assert_eq!(reuse(&f, &f.owner, "Target-Default-Pw-1!", f.owner_secret).await, Ok(false));
            assert_denied(reuse(&f, &f.owner, "Target-Default-Pw-1!", f.admin_secret).await);
        }

        #[tokio::test]
        async fn test_reuse_check_detects_own_history() {
            let f = setup();
            set_password(&f, &f.owner, f.owner_secret, "First-Password-111!").await;
            set_password(&f, &f.owner, f.owner_secret, "Second-Password-222!").await;

            assert_eq!(reuse(&f, &f.owner, "First-Password-111!", f.owner_secret).await, Ok(true));
            assert_eq!(reuse(&f, &f.owner, "Second-Password-222!", f.owner_secret).await, Ok(true));
            assert_eq!(reuse(&f, &f.owner, "Never-Used-Password-3!", f.owner_secret).await, Ok(false));
        }

        #[tokio::test]
        async fn test_changing_password_is_not_blocked_by_other_secrets_passwords() {
            let f = setup();
            set_password(&f, &f.admin, f.admin_secret, "Shared-Password-123!").await;
            // Previously rejected with "used before on another credential"; that global
            // check was an oracle and is gone.
            set_password(&f, &f.owner, f.owner_secret, "Shared-Password-123!").await;
        }

        #[tokio::test]
        async fn test_rotation_alerts_are_filtered_to_readable_vaults() {
            let f = setup();
            f.with_conn(|conn| {
                conn.execute("UPDATE vault_secrets SET next_rotation_due = datetime('now', '-1 day')", []).unwrap();
            });

            let alerts = get_rotation_alerts(f.owner.clone(), 30, f.app.state(), f.app.state()).await.unwrap();
            assert_eq!(alerts.iter().map(|a| a.vault_id).collect::<Vec<_>>(), vec![f.owner_vault]);

            let alerts = get_rotation_alerts(f.admin.clone(), 30, f.app.state(), f.app.state()).await.unwrap();
            assert_eq!(alerts.len(), 2);
        }
    }
}
