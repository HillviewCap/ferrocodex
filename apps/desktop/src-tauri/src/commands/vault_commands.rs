// Vault management commands
// Extracted from lib.rs.backup

use crate::auth::{SessionManager, verify_password};
use crate::users::{UserRepository, SqliteUserRepository, UserRole, UserInfo};
use crate::audit::{AuditRepository, SqliteAuditRepository, AuditEventRequest, AuditEventType, create_vault_access_granted_event, create_vault_access_revoked_event};
use crate::validation::{InputSanitizer};
use crate::vault::{
    VaultRepository, SqliteVaultRepository, CreateVaultRequest, AddSecretRequest, VaultInfo, 
    IdentityVault, GeneratePasswordRequest, UpdateCredentialPasswordRequest, UpdateVaultSecretRequest, 
    DeleteVaultSecretRequest, PasswordStrength, PasswordHistory, PasswordGenerator, PasswordStrengthAnalyzer,
    CreateStandaloneCredentialRequest, UpdateStandaloneCredentialRequest, SearchCredentialsRequest, 
    CreateCategoryRequest, StandaloneCredentialInfo, CategoryWithChildren, VaultAccessControlService,
    PermissionType, VaultAccessInfo, GrantVaultAccessRequest, VaultPermission, RevokeVaultAccessRequest,
    VaultAccessLog, CreatePermissionRequest, PermissionRequest,
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
use serde::{Serialize, Deserialize};

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;

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
pub async fn export_vault(
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

#[tauri::command]
pub async fn check_password_reuse(
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

    // Validate inputs
    if let Some(ref label) = request.label {
        let sanitized_label = InputSanitizer::sanitize_string(label);
        if InputSanitizer::is_potentially_malicious(&sanitized_label) {
            error!("Potentially malicious input detected in update_vault_secret");
            return Err("Invalid input detected".to_string());
        }
    }
    
    if let Some(ref value) = request.value {
        let sanitized_value = InputSanitizer::sanitize_string(value);
        if InputSanitizer::is_potentially_malicious(&sanitized_value) {
            error!("Potentially malicious input detected in update_vault_secret");
            return Err("Invalid input detected".to_string());
        }
    }

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let vault_repo = SqliteVaultRepository::new(db.get_connection());
            
            match vault_repo.update_vault_secret(request) {
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
            
            scheduler.get_rotation_alerts(days_ahead)
                .map_err(|e| format!("Failed to get rotation alerts: {}", e))
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

    // Check permissions
    if session.role != crate::users::UserRole::Administrator && session.role != crate::users::UserRole::Engineer {
        return Err("Insufficient permissions to execute batch rotation".to_string());
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