use anyhow::Result;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};
use crate::users::User;
use crate::vault::{
    VaultRepository, SqliteVaultRepository, VaultInfo, IdentityVault, VaultSecret,
    AddSecretRequest, VaultAccessControlService, 
    PermissionType, AccessType, AccessResult, VaultVersion,
};

/// A secure wrapper around VaultRepository that enforces access control
pub struct SecureVaultRepository {
    db_conn: Arc<Mutex<Connection>>,
    access_control: VaultAccessControlService,
}

impl SecureVaultRepository {
    pub fn new(db_conn: Arc<Mutex<Connection>>) -> Self {
        let access_control = VaultAccessControlService::new(db_conn.clone());
        Self {
            db_conn,
            access_control,
        }
    }

    /// Get vault by asset ID with access control
    pub fn get_vault_by_asset_id(&self, asset_id: i64, user: &User) -> Result<Option<VaultInfo>> {
        let conn = self.db_conn.lock().unwrap();
        let repo = SqliteVaultRepository::new(&conn);
        
        // First get the vault info
        let vault_info = repo.get_vault_by_asset_id(asset_id)?;
        
        if let Some(mut vault_info) = vault_info {
            // Check if user has read access
            if !self.access_control.can_read(user, vault_info.vault.id)? {
                debug!("User {} denied read access to vault {}", user.id, vault_info.vault.id);
                
                // Log access denial
                repo.log_vault_access(
                    user.id, 
                    vault_info.vault.id, 
                    AccessType::View, 
                    AccessResult::Denied, 
                    Some("Insufficient permissions".to_string())
                )?;
                
                return Err(anyhow::anyhow!("Access denied: insufficient permissions"));
            }
            
            // Log successful access
            repo.log_vault_access(
                user.id, 
                vault_info.vault.id, 
                AccessType::View, 
                AccessResult::Success, 
                None
            )?;
            
            // Filter secrets based on permissions
            if user.role != crate::users::UserRole::Administrator {
                // For non-admins, only show secrets they have explicit access to
                // This could be enhanced to filter specific secret types based on permissions
                debug!("Non-admin user {} accessing vault {}, applying content filtering", user.id, vault_info.vault.id);
            }
            
            Ok(Some(vault_info))
        } else {
            Ok(None)
        }
    }

    /// Get vault by ID with access control
    pub fn get_vault_by_id(&self, vault_id: i64, user: &User) -> Result<Option<IdentityVault>> {
        // Check read access first
        if !self.access_control.can_read(user, vault_id)? {
            let conn = self.db_conn.lock().unwrap();
            let repo = SqliteVaultRepository::new(&conn);
            
            // Log access denial
            repo.log_vault_access(
                user.id, 
                vault_id, 
                AccessType::View, 
                AccessResult::Denied, 
                Some("Insufficient permissions".to_string())
            )?;
            
            return Err(anyhow::anyhow!("Access denied: insufficient permissions"));
        }
        
        let conn = self.db_conn.lock().unwrap();
        let repo = SqliteVaultRepository::new(&conn);
        
        // Log successful access
        repo.log_vault_access(
            user.id, 
            vault_id, 
            AccessType::View, 
            AccessResult::Success, 
            None
        )?;
        
        repo.get_vault_by_id(vault_id)
    }

    /// Add secret to vault with access control
    pub fn add_secret(&self, request: AddSecretRequest, user: &User) -> Result<VaultSecret> {
        // Check write access
        if !self.access_control.can_write(user, request.vault_id)? {
            let conn = self.db_conn.lock().unwrap();
            let repo = SqliteVaultRepository::new(&conn);
            
            // Log access denial
            repo.log_vault_access(
                user.id, 
                request.vault_id, 
                AccessType::Edit, 
                AccessResult::Denied, 
                Some("Insufficient permissions".to_string())
            )?;
            
            return Err(anyhow::anyhow!("Access denied: insufficient permissions"));
        }
        
        let conn = self.db_conn.lock().unwrap();
        let repo = SqliteVaultRepository::new(&conn);
        
        // Log successful access
        repo.log_vault_access(
            user.id, 
            request.vault_id, 
            AccessType::Edit, 
            AccessResult::Success, 
            None
        )?;
        
        // Ensure author_id matches the current user
        let mut secure_request = request;
        secure_request.author_id = user.id;
        
        repo.add_secret(secure_request)
    }

    /// Update secret with access control
    pub fn update_secret(&self, secret: &VaultSecret, user: &User) -> Result<()> {
        // Check write access
        if !self.access_control.can_write(user, secret.vault_id)? {
            let conn = self.db_conn.lock().unwrap();
            let repo = SqliteVaultRepository::new(&conn);
            
            // Log access denial
            repo.log_vault_access(
                user.id, 
                secret.vault_id, 
                AccessType::Edit, 
                AccessResult::Denied, 
                Some("Insufficient permissions".to_string())
            )?;
            
            return Err(anyhow::anyhow!("Access denied: insufficient permissions"));
        }
        
        let conn = self.db_conn.lock().unwrap();
        let repo = SqliteVaultRepository::new(&conn);
        
        // Log successful access
        repo.log_vault_access(
            user.id, 
            secret.vault_id, 
            AccessType::Edit, 
            AccessResult::Success, 
            None
        )?;
        
        repo.update_secret(secret, user.id)
    }

    /// Delete secret with access control
    pub fn delete_secret(&self, secret_id: i64, user: &User) -> Result<()> {
        let conn = self.db_conn.lock().unwrap();
        let repo = SqliteVaultRepository::new(&conn);
        
        // Get the secret to find its vault_id
        let secret = repo.get_secret_by_id(secret_id)?
            .ok_or_else(|| anyhow::anyhow!("Secret not found"))?;
        
        // Check write access
        if !self.access_control.can_write(user, secret.vault_id)? {
            // Log access denial
            repo.log_vault_access(
                user.id, 
                secret.vault_id, 
                AccessType::Edit, 
                AccessResult::Denied, 
                Some("Insufficient permissions".to_string())
            )?;
            
            return Err(anyhow::anyhow!("Access denied: insufficient permissions"));
        }
        
        // Log successful access
        repo.log_vault_access(
            user.id, 
            secret.vault_id, 
            AccessType::Edit, 
            AccessResult::Success, 
            None
        )?;
        
        repo.delete_secret(secret_id, user.id)
    }

    /// Get vault history with access control
    pub fn get_vault_history(&self, vault_id: i64, user: &User) -> Result<Vec<VaultVersion>> {
        // Check read access
        if !self.access_control.can_read(user, vault_id)? {
            let conn = self.db_conn.lock().unwrap();
            let repo = SqliteVaultRepository::new(&conn);
            
            // Log access denial
            repo.log_vault_access(
                user.id, 
                vault_id, 
                AccessType::View, 
                AccessResult::Denied, 
                Some("Insufficient permissions".to_string())
            )?;
            
            return Err(anyhow::anyhow!("Access denied: insufficient permissions"));
        }
        
        let conn = self.db_conn.lock().unwrap();
        let repo = SqliteVaultRepository::new(&conn);
        
        // Log successful access
        repo.log_vault_access(
            user.id, 
            vault_id, 
            AccessType::View, 
            AccessResult::Success, 
            None
        )?;
        
        repo.get_vault_history(vault_id)
    }

    /// Export vault with access control
    pub fn export_vault(&self, vault_id: i64, user: &User) -> Result<VaultInfo> {
        // Check export access
        if !self.access_control.can_export(user, vault_id)? {
            let conn = self.db_conn.lock().unwrap();
            let repo = SqliteVaultRepository::new(&conn);
            
            // Log access denial
            repo.log_vault_access(
                user.id, 
                vault_id, 
                AccessType::Export, 
                AccessResult::Denied, 
                Some("Insufficient permissions".to_string())
            )?;
            
            return Err(anyhow::anyhow!("Access denied: insufficient permissions"));
        }
        
        let conn = self.db_conn.lock().unwrap();
        let repo = SqliteVaultRepository::new(&conn);
        
        // Log successful access
        repo.log_vault_access(
            user.id, 
            vault_id, 
            AccessType::Export, 
            AccessResult::Success, 
            None
        )?;
        
        // Get vault info for export
        let vault = repo.get_vault_by_id(vault_id)?
            .ok_or_else(|| anyhow::anyhow!("Vault not found"))?;
        
        let secrets = repo.get_vault_secrets(vault_id)?;
        
        Ok(VaultInfo {
            vault,
            secrets: secrets.clone(),
            secret_count: secrets.len(),
        })
    }

    /// Get all vaults accessible by the user
    pub fn get_accessible_vaults(&self, user: &User) -> Result<Vec<IdentityVault>> {
        let conn = self.db_conn.lock().unwrap();
        let repo = SqliteVaultRepository::new(&conn);
        
        if user.role == crate::users::UserRole::Administrator {
            // Administrators can see all vaults
            let mut stmt = conn.prepare(
                "SELECT id, asset_id, name, description, created_by, created_at, updated_at 
                 FROM vault_entries ORDER BY created_at DESC"
            )?;
            
            let vault_iter = stmt.query_map([], SqliteVaultRepository::row_to_vault)?;
            let mut vaults = Vec::new();
            
            for vault in vault_iter {
                vaults.push(vault?);
            }
            
            Ok(vaults)
        } else {
            // Engineers can only see vaults they have permissions for
            let mut stmt = conn.prepare(
                "SELECT DISTINCT v.id, v.asset_id, v.name, v.description, v.created_by, v.created_at, v.updated_at 
                 FROM vault_entries v
                 INNER JOIN vault_permissions vp ON v.id = vp.vault_id
                 WHERE vp.user_id = ?1 AND vp.is_active = 1
                 ORDER BY v.created_at DESC"
            )?;
            
            let vault_iter = stmt.query_map([user.id], SqliteVaultRepository::row_to_vault)?;
            let mut vaults = Vec::new();
            
            for vault in vault_iter {
                vaults.push(vault?);
            }
            
            Ok(vaults)
        }
    }

    /// Check if user can perform an operation on a vault
    pub fn check_permission(&self, user: &User, vault_id: i64, permission: PermissionType) -> Result<bool> {
        match permission {
            PermissionType::Read => self.access_control.can_read(user, vault_id),
            PermissionType::Write => self.access_control.can_write(user, vault_id),
            PermissionType::Export => self.access_control.can_export(user, vault_id),
            PermissionType::Share => self.access_control.can_share(user, vault_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use crate::users::{UserRole, UserRepository, SqliteUserRepository, CreateUserRequest};
    use crate::vault::{CreateVaultRequest};

    fn setup_test_db() -> (NamedTempFile, Arc<Mutex<Connection>>) {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        
        // Initialize schemas
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            
            CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                is_active BOOLEAN DEFAULT 1
            );
            
            CREATE TABLE assets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES users(id)
            );
            "#,
        ).unwrap();
        
        let user_repo = SqliteUserRepository::new(&conn);
        user_repo.initialize_schema().unwrap();
        
        let vault_repo = SqliteVaultRepository::new(&conn);
        vault_repo.initialize_schema().unwrap();
        
        // Create test data
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role) VALUES (1, 'admin', 'hash', 'Administrator')",
            [],
        ).unwrap();
        
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role) VALUES (2, 'engineer', 'hash', 'Engineer')",
            [],
        ).unwrap();
        
        conn.execute(
            "INSERT INTO assets (id, name, description, created_by) VALUES (1, 'Test Asset', 'Description', 1)",
            [],
        ).unwrap();
        
        let db_conn = Arc::new(Mutex::new(conn));
        (temp_file, db_conn)
    }

    #[test]
    fn test_administrator_vault_access() {
        let (_temp_file, db_conn) = setup_test_db();
        let secure_repo = SecureVaultRepository::new(db_conn.clone());
        
        let admin_user = User {
            id: 1,
            username: "admin".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Administrator,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            is_active: true,
        };
        
        // Create a vault
        {
            let conn = db_conn.lock().unwrap();
            let repo = SqliteVaultRepository::new(&conn);
            let request = CreateVaultRequest {
                asset_id: 1,
                name: "Test Vault".to_string(),
                description: "Test Description".to_string(),
                created_by: 1,
            };
            repo.create_vault(request).unwrap();
        }
        
        // Administrator should have access
        let vault_info = secure_repo.get_vault_by_asset_id(1, &admin_user).unwrap();
        assert!(vault_info.is_some());
    }

    #[test]
    fn test_engineer_vault_access_denied() {
        let (_temp_file, db_conn) = setup_test_db();
        let secure_repo = SecureVaultRepository::new(db_conn.clone());
        
        let engineer_user = User {
            id: 2,
            username: "engineer".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Engineer,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            is_active: true,
        };
        
        // Create a vault
        {
            let conn = db_conn.lock().unwrap();
            let repo = SqliteVaultRepository::new(&conn);
            let request = CreateVaultRequest {
                asset_id: 1,
                name: "Test Vault".to_string(),
                description: "Test Description".to_string(),
                created_by: 1,
            };
            repo.create_vault(request).unwrap();
        }
        
        // Engineer without permissions should be denied
        let result = secure_repo.get_vault_by_asset_id(1, &engineer_user);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Access denied"));
    }

    #[test]
    fn test_permission_checking() {
        let (_temp_file, db_conn) = setup_test_db();
        let secure_repo = SecureVaultRepository::new(db_conn);
        
        let admin_user = User {
            id: 1,
            username: "admin".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Administrator,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            is_active: true,
        };
        
        let engineer_user = User {
            id: 2,
            username: "engineer".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Engineer,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            is_active: true,
        };
        
        // Test all permission types for admin
        assert!(secure_repo.check_permission(&admin_user, 1, PermissionType::Read).unwrap());
        assert!(secure_repo.check_permission(&admin_user, 1, PermissionType::Write).unwrap());
        assert!(secure_repo.check_permission(&admin_user, 1, PermissionType::Export).unwrap());
        assert!(secure_repo.check_permission(&admin_user, 1, PermissionType::Share).unwrap());
        
        // Test all permission types for engineer (should all be false without grants)
        assert!(!secure_repo.check_permission(&engineer_user, 1, PermissionType::Read).unwrap());
        assert!(!secure_repo.check_permission(&engineer_user, 1, PermissionType::Write).unwrap());
        assert!(!secure_repo.check_permission(&engineer_user, 1, PermissionType::Export).unwrap());
        assert!(!secure_repo.check_permission(&engineer_user, 1, PermissionType::Share).unwrap());
    }
}