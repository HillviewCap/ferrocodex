use anyhow::Result;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};
use crate::users::{UserRole, User};
use crate::vault::{
    VaultRepository, SqliteVaultRepository, CheckVaultAccessRequest, VaultAccessInfo,
    VaultPermission, PermissionType, AccessType, AccessResult,
};

/// Service for managing vault access control and permissions
pub struct VaultAccessControlService<'a> {
    db_conn: &'a Connection,
}

impl<'a> VaultAccessControlService<'a> {
    pub fn new(db_conn: &'a Connection) -> Self {
        Self { db_conn }
    }

    /// Check if a user has access to a vault with a specific permission
    pub fn check_vault_access(&self, user: &User, vault_id: i64, permission_type: PermissionType) -> Result<VaultAccessInfo> {
        debug!("Checking vault access for user {} on vault {} with permission {:?}", 
               user.id, vault_id, permission_type);

        // Administrators always have full access
        if user.role == UserRole::Administrator {
            info!("Administrator {} has full access to vault {}", user.username, vault_id);
            return Ok(VaultAccessInfo {
                has_access: true,
                permissions: vec![],
                is_administrator: true,
            });
        }

        // For engineers, check specific permissions
        let conn = self.db_conn;
        let repo = SqliteVaultRepository::new(&conn);
        
        let request = CheckVaultAccessRequest {
            user_id: user.id,
            vault_id,
            permission_type,
        };

        let access_info = repo.check_vault_access(request)?;
        
        // Log the access attempt
        let result = if access_info.has_access {
            AccessResult::Success
        } else {
            AccessResult::Denied
        };
        
        let access_type = match permission_type {
            PermissionType::Read => AccessType::View,
            PermissionType::Write => AccessType::Edit,
            PermissionType::Export => AccessType::Export,
            PermissionType::Share => AccessType::Share,
        };
        
        repo.log_vault_access(user.id, vault_id, access_type, result, None)?;
        
        debug!("Access check result for user {}: has_access={}", user.id, access_info.has_access);
        Ok(access_info)
    }

    /// Check if a user can read from a vault
    pub fn can_read(&self, user: &User, vault_id: i64) -> Result<bool> {
        let access_info = self.check_vault_access(user, vault_id, PermissionType::Read)?;
        Ok(access_info.has_access)
    }

    /// Check if a user can write to a vault
    pub fn can_write(&self, user: &User, vault_id: i64) -> Result<bool> {
        let access_info = self.check_vault_access(user, vault_id, PermissionType::Write)?;
        Ok(access_info.has_access)
    }

    /// Check if a user can export from a vault
    pub fn can_export(&self, user: &User, vault_id: i64) -> Result<bool> {
        let access_info = self.check_vault_access(user, vault_id, PermissionType::Export)?;
        Ok(access_info.has_access)
    }

    /// Check if a user can share a vault
    pub fn can_share(&self, user: &User, vault_id: i64) -> Result<bool> {
        let access_info = self.check_vault_access(user, vault_id, PermissionType::Share)?;
        Ok(access_info.has_access)
    }

    /// Get all permissions for a user on a specific vault
    pub fn get_user_vault_permissions(&self, user_id: i64, vault_id: Option<i64>) -> Result<Vec<VaultPermission>> {
        let conn = self.db_conn;
        let repo = SqliteVaultRepository::new(&conn);
        repo.get_user_vault_permissions(user_id, vault_id)
    }

    /// Get all permissions for a vault
    pub fn get_vault_permissions(&self, vault_id: i64) -> Result<Vec<VaultPermission>> {
        let conn = self.db_conn;
        let repo = SqliteVaultRepository::new(&conn);
        repo.get_vault_permissions(vault_id)
    }

    /// Check and expire any permissions that have passed their expiry date
    pub fn expire_permissions(&self) -> Result<u64> {
        let conn = self.db_conn;
        let repo = SqliteVaultRepository::new(&conn);
        let expired_count = repo.expire_permissions()?;
        
        if expired_count > 0 {
            info!("Expired {} vault permissions", expired_count);
        }
        
        Ok(expired_count)
    }

    /// Check permission inheritance based on role
    pub fn check_permission_inheritance(&self, user: &User, _permission_type: PermissionType) -> bool {
        match user.role {
            UserRole::Administrator => true, // Administrators have all permissions
            UserRole::Engineer => {
                // Engineers only have permissions that are explicitly granted
                false
            }
        }
    }

    /// Validate permission request
    pub fn validate_permission_request(&self, requestor: &User, target_user_id: i64, permission_type: PermissionType) -> Result<()> {
        // Only administrators can grant permissions
        if requestor.role != UserRole::Administrator {
            return Err(anyhow::anyhow!("Only administrators can grant vault permissions"));
        }

        // Cannot grant share permission to non-administrators
        if permission_type == PermissionType::Share {
            warn!("Attempt to grant share permission to user {}", target_user_id);
            // Additional check could be added here to verify target user role
        }

        Ok(())
    }

    /// Check if a permission is still valid (not expired)
    pub fn is_permission_valid(&self, permission: &VaultPermission) -> bool {
        if !permission.is_active {
            return false;
        }

        if let Some(expires_at) = &permission.expires_at {
            // Parse the expiry date and check if it's in the past
            if let Ok(expiry_time) = chrono::DateTime::parse_from_rfc3339(expires_at) {
                let now = chrono::Utc::now();
                return expiry_time > now;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use crate::users::{UserRepository, SqliteUserRepository, CreateUserRequest};

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
        
        let db_conn = Arc::new(Mutex::new(conn));
        (temp_file, db_conn)
    }

    #[test]
    fn test_administrator_access() {
        let (_temp_file, db_conn) = setup_test_db();
        let conn_guard = db_conn.lock().unwrap();
        let service = VaultAccessControlService::new(&*conn_guard);
        
        let admin_user = User {
            id: 1,
            username: "admin".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Administrator,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            is_active: true,
        };
        
        // Test all permission types for administrator
        assert!(service.can_read(&admin_user, 1).unwrap());
        assert!(service.can_write(&admin_user, 1).unwrap());
        assert!(service.can_export(&admin_user, 1).unwrap());
        assert!(service.can_share(&admin_user, 1).unwrap());
    }

    #[test]
    fn test_permission_inheritance() {
        let (_temp_file, db_conn) = setup_test_db();
        let conn_guard = db_conn.lock().unwrap();
        let service = VaultAccessControlService::new(&*conn_guard);
        
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
        
        // Admin should have all permissions through inheritance
        assert!(service.check_permission_inheritance(&admin_user, PermissionType::Read));
        assert!(service.check_permission_inheritance(&admin_user, PermissionType::Write));
        assert!(service.check_permission_inheritance(&admin_user, PermissionType::Export));
        assert!(service.check_permission_inheritance(&admin_user, PermissionType::Share));
        
        // Engineer should have no permissions through inheritance
        assert!(!service.check_permission_inheritance(&engineer_user, PermissionType::Read));
        assert!(!service.check_permission_inheritance(&engineer_user, PermissionType::Write));
        assert!(!service.check_permission_inheritance(&engineer_user, PermissionType::Export));
        assert!(!service.check_permission_inheritance(&engineer_user, PermissionType::Share));
    }

    #[test]
    fn test_permission_validation() {
        let (_temp_file, db_conn) = setup_test_db();
        let conn_guard = db_conn.lock().unwrap();
        let service = VaultAccessControlService::new(&*conn_guard);
        
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
        
        // Admin should be able to grant permissions
        assert!(service.validate_permission_request(&admin_user, 2, PermissionType::Read).is_ok());
        
        // Engineer should not be able to grant permissions
        assert!(service.validate_permission_request(&engineer_user, 1, PermissionType::Read).is_err());
    }

    #[test]
    fn test_permission_expiry() {
        let (_temp_file, db_conn) = setup_test_db();
        let conn_guard = db_conn.lock().unwrap();
        let service = VaultAccessControlService::new(&*conn_guard);
        
        // Test active permission without expiry
        let perm1 = VaultPermission {
            permission_id: 1,
            user_id: 1,
            vault_id: 1,
            permission_type: PermissionType::Read,
            granted_by: 1,
            granted_at: chrono::Utc::now().to_rfc3339(),
            expires_at: None,
            is_active: true,
        };
        assert!(service.is_permission_valid(&perm1));
        
        // Test active permission with future expiry
        let future_expiry = chrono::Utc::now() + chrono::Duration::days(1);
        let perm2 = VaultPermission {
            permission_id: 2,
            user_id: 1,
            vault_id: 1,
            permission_type: PermissionType::Read,
            granted_by: 1,
            granted_at: chrono::Utc::now().to_rfc3339(),
            expires_at: Some(future_expiry.to_rfc3339()),
            is_active: true,
        };
        assert!(service.is_permission_valid(&perm2));
        
        // Test active permission with past expiry
        let past_expiry = chrono::Utc::now() - chrono::Duration::days(1);
        let perm3 = VaultPermission {
            permission_id: 3,
            user_id: 1,
            vault_id: 1,
            permission_type: PermissionType::Read,
            granted_by: 1,
            granted_at: chrono::Utc::now().to_rfc3339(),
            expires_at: Some(past_expiry.to_rfc3339()),
            is_active: true,
        };
        assert!(!service.is_permission_valid(&perm3));
        
        // Test inactive permission
        let perm4 = VaultPermission {
            permission_id: 4,
            user_id: 1,
            vault_id: 1,
            permission_type: PermissionType::Read,
            granted_by: 1,
            granted_at: chrono::Utc::now().to_rfc3339(),
            expires_at: None,
            is_active: false,
        };
        assert!(!service.is_permission_valid(&perm4));
    }
}