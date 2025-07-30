// Vault business logic handlers
// Business logic abstraction for vault operations

use crate::auth::SessionManager;
use crate::users::{UserRepository, SqliteUserRepository, UserRole};
use crate::audit::{AuditRepository, SqliteAuditRepository, AuditEventRequest, AuditEventType};
use crate::validation::InputSanitizer;
use crate::vault::{
    VaultRepository, SqliteVaultRepository, CreateVaultRequest, AddSecretRequest, VaultInfo,
    IdentityVault, GeneratePasswordRequest, UpdateCredentialPasswordRequest, UpdateVaultSecretRequest,
    DeleteVaultSecretRequest, PasswordStrength, PasswordHistory, PasswordGenerator, PasswordStrengthAnalyzer,
    CreateStandaloneCredentialRequest, UpdateStandaloneCredentialRequest, SearchCredentialsRequest,
    CreateCategoryRequest, StandaloneCredentialInfo, CategoryWithChildren,
    PermissionType, VaultAccessInfo, GrantVaultAccessRequest, VaultPermission, RevokeVaultAccessRequest,
    VaultAccessLog, CreatePermissionRequest, PermissionRequest,
    rotation::{
        PasswordRotationService, PasswordRotationRequest, RotationScheduler, RotationSchedule,
        RotationBatch, BatchRotationService, PasswordRotationHistory, CreateRotationBatchRequest,
        UpdateRotationScheduleRequest
    }
};
use crate::encryption::FileEncryption;
use crate::database::Database;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

pub struct VaultHandler;

impl VaultHandler {
    pub fn new() -> Self {
        Self
    }

    /// Validates session and returns user info, centralizing session validation logic
    pub fn validate_session_and_get_user(
        session_manager: &std::sync::MutexGuard<SessionManager>,
        token: &str,
    ) -> Result<crate::auth::Session, String> {
        match session_manager.validate_session(token) {
            Ok(Some(session)) => Ok(session),
            Ok(None) => Err("Invalid or expired session".to_string()),
            Err(e) => {
                error!("Session validation error: {}", e);
                Err("Session validation error".to_string())
            }
        }
    }

    /// Validates and sanitizes string inputs to prevent malicious content
    pub fn validate_and_sanitize_input(input: &str, field_name: &str) -> Result<String, String> {
        let sanitized = InputSanitizer::sanitize_string(input);
        if InputSanitizer::is_potentially_malicious(&sanitized) {
            error!("Potentially malicious input detected in {}", field_name);
            return Err("Invalid input detected. Please avoid using special characters or script-like patterns.".to_string());
        }
        Ok(sanitized)
    }

    /// Creates a vault repository instance
    pub fn create_vault_repository(db: &Database) -> SqliteVaultRepository {
        SqliteVaultRepository::new(db.get_connection())
    }

    /// Creates an audit repository instance
    pub fn create_audit_repository(db: &Database) -> SqliteAuditRepository {
        SqliteAuditRepository::new(db.get_connection())
    }

    /// Creates a user repository instance
    pub fn create_user_repository(db: &Database) -> SqliteUserRepository {
        SqliteUserRepository::new(db.get_connection())
    }

    /// Handles vault creation business logic
    pub fn handle_create_vault(
        vault_request: CreateVaultRequest,
        user_id: i64,
        username: &str,
        vault_repo: &SqliteVaultRepository,
    ) -> Result<IdentityVault, String> {
        // Validate inputs
        let sanitized_name = Self::validate_and_sanitize_input(&vault_request.name, "vault_name")?;
        let sanitized_description = Self::validate_and_sanitize_input(&vault_request.description, "vault_description")?;

        let sanitized_request = CreateVaultRequest {
            asset_id: vault_request.asset_id,
            name: sanitized_name,
            description: sanitized_description,
            created_by: user_id,
        };

        match vault_repo.create_vault(sanitized_request) {
            Ok(vault) => {
                info!("Identity vault created by {}: {}", username, vault.name);
                Ok(vault)
            }
            Err(e) => {
                error!("Failed to create identity vault: {}", e);
                Err(format!("Failed to create identity vault: {}", e))
            }
        }
    }

    /// Handles secret addition business logic
    pub fn handle_add_secret(
        secret_request: AddSecretRequest,
        user_id: i64,
        username: &str,
        vault_repo: &SqliteVaultRepository,
    ) -> Result<crate::vault::VaultSecret, String> {
        // Validate inputs
        let sanitized_label = Self::validate_and_sanitize_input(&secret_request.label, "secret_label")?;
        let sanitized_value = Self::validate_and_sanitize_input(&secret_request.value, "secret_value")?;

        let sanitized_request = AddSecretRequest {
            vault_id: secret_request.vault_id,
            secret_type: secret_request.secret_type,
            label: sanitized_label,
            value: sanitized_value,
            author_id: user_id,
        };

        match vault_repo.add_secret(sanitized_request) {
            Ok(secret) => {
                info!(
                    "Secret added to vault by {}: {} ({})",
                    username, secret.label, secret.secret_type.to_string()
                );
                Ok(secret)
            }
            Err(e) => {
                error!("Failed to add secret to vault: {}", e);
                Err(format!("Failed to add secret to vault: {}", e))
            }
        }
    }

    /// Handles secret decryption business logic
    pub fn handle_decrypt_secret(
        secret_id: i64,
        vault_id: i64,
        user_id: i64,
        username: &str,
        vault_repo: &SqliteVaultRepository,
    ) -> Result<String, String> {
        match vault_repo.get_secret_by_id(secret_id) {
            Ok(Some(secret)) => {
                // Decrypt the secret value
                let encryption = FileEncryption::new(&format!("vault_{}_{}", vault_id, user_id));
                use base64::{Engine as _, engine::general_purpose};
                
                match general_purpose::STANDARD.decode(&secret.encrypted_value) {
                    Ok(encrypted_bytes) => {
                        match encryption.decrypt(&encrypted_bytes) {
                            Ok(decrypted_bytes) => {
                                match String::from_utf8(decrypted_bytes) {
                                    Ok(decrypted_value) => {
                                        info!(
                                            "Secret decrypted for user {}: {} ({})",
                                            username, secret.label, secret.secret_type.to_string()
                                        );
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

    /// Handles standalone credential creation business logic
    pub fn handle_create_standalone_credential(
        credential_request: CreateStandaloneCredentialRequest,
        user_id: i64,
        username: &str,
        vault_repo: &SqliteVaultRepository,
    ) -> Result<StandaloneCredentialInfo, String> {
        // Sanitize input
        let sanitized_name = Self::validate_and_sanitize_input(&credential_request.name, "credential_name")?;
        let sanitized_description = Self::validate_and_sanitize_input(&credential_request.description, "credential_description")?;
        let sanitized_value = Self::validate_and_sanitize_input(&credential_request.value, "credential_value")?;
        
        let sanitized_tags = credential_request.tags.as_ref().map(|tags| {
            tags.iter()
                .filter_map(|tag| Self::validate_and_sanitize_input(tag, "credential_tag").ok())
                .collect()
        });

        let sanitized_request = CreateStandaloneCredentialRequest {
            name: sanitized_name,
            description: sanitized_description,
            credential_type: credential_request.credential_type,
            category_id: credential_request.category_id,
            value: sanitized_value,
            tags: sanitized_tags,
            created_by: user_id,
        };

        match vault_repo.create_standalone_credential(sanitized_request) {
            Ok(credential) => {
                info!("Standalone credential created by {}: {}", username, credential.name);
                
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

    /// Handles password strength validation
    pub fn handle_password_strength_validation(password: &str, username: &str) -> PasswordStrength {
        let strength = PasswordStrengthAnalyzer::analyze(password);
        info!("Password strength analyzed by {} - Score: {}", username, strength.score);
        strength
    }

    /// Handles password reuse checking
    pub fn handle_password_reuse_check(
        password: &str,
        exclude_secret_id: Option<i64>,
        username: &str,
        vault_repo: &SqliteVaultRepository,
    ) -> Result<bool, String> {
        // Hash the password for comparison
        let password_hash = match PasswordGenerator::hash_password(password) {
            Ok(hash) => hash,
            Err(e) => {
                error!("Failed to hash password for reuse check: {}", e);
                return Err("Failed to process password".to_string());
            }
        };

        match vault_repo.check_password_reuse(&password_hash, exclude_secret_id) {
            Ok(is_reused) => {
                info!("Password reuse check by {}: {}", username, is_reused);
                Ok(is_reused)
            }
            Err(e) => {
                error!("Failed to check password reuse: {}", e);
                Err(format!("Failed to check password reuse: {}", e))
            }
        }
    }

    /// Handles credential password updates with reuse checking
    pub fn handle_credential_password_update(
        request: UpdateCredentialPasswordRequest,
        username: &str,
        vault_repo: &SqliteVaultRepository,
    ) -> Result<(), String> {
        // Validate inputs
        let sanitized_password = Self::validate_and_sanitize_input(&request.new_password, "password")?;

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
                info!("Password updated by {} for secret with strength score {}", username, strength.score);
                Ok(())
            }
            Err(e) => {
                error!("Failed to update password: {}", e);
                Err(format!("Failed to update password: {}", e))
            }
        }
    }

    /// Handles search credential requests with input sanitization
    pub fn handle_search_credentials(
        search_request: SearchCredentialsRequest,
        vault_repo: &SqliteVaultRepository,
    ) -> Result<crate::vault::SearchCredentialsResponse, String> {
        // Sanitize search query if provided
        let sanitized_request = SearchCredentialsRequest {
            query: search_request.query.map(|q| InputSanitizer::sanitize_string(&q)),
            credential_type: search_request.credential_type,
            category_id: search_request.category_id,
            tags: search_request.tags.map(|tags| {
                tags.iter().map(|tag| InputSanitizer::sanitize_string(tag)).collect()
            }),
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

    /// Checks if user has required permissions for vault operations
    pub fn check_user_permissions(session: &crate::auth::Session, required_role: UserRole) -> Result<(), String> {
        if session.role != UserRole::Administrator && session.role != required_role {
            return Err("Insufficient permissions for this operation".to_string());
        }
        Ok(())
    }

    /// Handles vault export logic
    pub fn handle_vault_export(
        vault_id: i64,
        username: &str,
        vault_repo: &SqliteVaultRepository,
    ) -> Result<String, String> {
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
                                info!("Vault {} exported by {}: {} secrets", vault_id, username, vault_info.secret_count);
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
}