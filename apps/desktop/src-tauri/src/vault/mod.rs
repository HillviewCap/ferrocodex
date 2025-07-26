use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::encryption::FileEncryption;
use tracing::{info, debug};
use chrono;

pub mod password_services;
pub use password_services::{PasswordGenerator, PasswordStrengthAnalyzer, PasswordReuseChecker};

#[cfg(test)]
mod password_performance_tests;

#[cfg(test)]
mod standalone_tests;

// Vault data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityVault {
    pub id: i64,
    pub asset_id: i64,
    pub name: String,
    pub description: String,
    pub created_by: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSecret {
    pub id: i64,
    pub vault_id: i64,
    pub secret_type: SecretType,
    pub label: String,
    pub encrypted_value: String,
    pub created_at: String,
    pub updated_at: String,
    // Password management fields
    pub strength_score: Option<i32>,
    pub last_changed: Option<String>,
    pub generation_method: Option<String>,
    pub policy_version: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultVersion {
    pub id: i64,
    pub vault_id: i64,
    pub change_type: ChangeType,
    pub author: i64,
    pub timestamp: String,
    pub notes: String,
    pub changes_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordHistory {
    pub id: i64,
    pub secret_id: i64,
    pub password_hash: String,
    pub created_at: String,
    pub retired_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicy {
    pub id: i64,
    pub min_length: i32,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_numbers: bool,
    pub require_special: bool,
    pub max_age_days: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordStrength {
    pub score: i32,
    pub entropy: f64,
    pub has_uppercase: bool,
    pub has_lowercase: bool,
    pub has_numbers: bool,
    pub has_special: bool,
    pub length: usize,
    pub feedback: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretType {
    Password,
    IpAddress,
    VpnKey,
    LicenseFile,
}

impl SecretType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "password" => Ok(SecretType::Password),
            "ip_address" => Ok(SecretType::IpAddress),
            "vpn_key" => Ok(SecretType::VpnKey),
            "license_file" => Ok(SecretType::LicenseFile),
            _ => Err(anyhow::anyhow!("Invalid secret type: {}", s)),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            SecretType::Password => "password".to_string(),
            SecretType::IpAddress => "ip_address".to_string(),
            SecretType::VpnKey => "vpn_key".to_string(),
            SecretType::LicenseFile => "license_file".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    VaultCreated,
    SecretAdded,
    SecretUpdated,
    SecretDeleted,
    VaultUpdated,
}

impl ChangeType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "vault_created" => Ok(ChangeType::VaultCreated),
            "secret_added" => Ok(ChangeType::SecretAdded),
            "secret_updated" => Ok(ChangeType::SecretUpdated),
            "secret_deleted" => Ok(ChangeType::SecretDeleted),
            "vault_updated" => Ok(ChangeType::VaultUpdated),
            _ => Err(anyhow::anyhow!("Invalid change type: {}", s)),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            ChangeType::VaultCreated => "vault_created".to_string(),
            ChangeType::SecretAdded => "secret_added".to_string(),
            ChangeType::SecretUpdated => "secret_updated".to_string(),
            ChangeType::SecretDeleted => "secret_deleted".to_string(),
            ChangeType::VaultUpdated => "vault_updated".to_string(),
        }
    }
}

// Request/Response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVaultRequest {
    pub asset_id: i64,
    pub name: String,
    pub description: String,
    pub created_by: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddSecretRequest {
    pub vault_id: i64,
    pub secret_type: SecretType,
    pub label: String,
    pub value: String,
    pub author_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    pub vault: IdentityVault,
    pub secrets: Vec<VaultSecret>,
    pub secret_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratePasswordRequest {
    pub length: usize,
    pub include_uppercase: bool,
    pub include_lowercase: bool,
    pub include_numbers: bool,
    pub include_special: bool,
    pub exclude_ambiguous: bool,
}

impl Default for GeneratePasswordRequest {
    fn default() -> Self {
        Self {
            length: 16,
            include_uppercase: true,
            include_lowercase: true,
            include_numbers: true,
            include_special: true,
            exclude_ambiguous: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCredentialPasswordRequest {
    pub secret_id: i64,
    pub new_password: String,
    pub author_id: i64,
}

// Standalone credential structures for Story 4.3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandaloneCredential {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub credential_type: SecretType,
    pub category_id: Option<i64>,
    pub encrypted_data: String,
    pub created_by: i64,
    pub created_at: String,
    pub updated_at: String,
    pub last_accessed: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialCategory {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub parent_category_id: Option<i64>,
    pub color_code: Option<String>,
    pub icon: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialTag {
    pub id: i64,
    pub standalone_credential_id: i64,
    pub tag_name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandaloneCredentialHistory {
    pub id: i64,
    pub credential_id: i64,
    pub change_type: StandaloneChangeType,
    pub author: i64,
    pub timestamp: String,
    pub notes: Option<String>,
    pub changes_json: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StandaloneChangeType {
    Created,
    Updated,
    Accessed,
    Deleted,
}

impl StandaloneChangeType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "created" => Ok(StandaloneChangeType::Created),
            "updated" => Ok(StandaloneChangeType::Updated),
            "accessed" => Ok(StandaloneChangeType::Accessed),
            "deleted" => Ok(StandaloneChangeType::Deleted),
            _ => Err(anyhow::anyhow!("Invalid standalone change type: {}", s)),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            StandaloneChangeType::Created => "created".to_string(),
            StandaloneChangeType::Updated => "updated".to_string(),
            StandaloneChangeType::Accessed => "accessed".to_string(),
            StandaloneChangeType::Deleted => "deleted".to_string(),
        }
    }
}

// Request/Response types for standalone credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStandaloneCredentialRequest {
    pub name: String,
    pub description: String,
    pub credential_type: SecretType,
    pub category_id: Option<i64>,
    pub value: String,
    pub tags: Option<Vec<String>>,
    pub created_by: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStandaloneCredentialRequest {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub value: Option<String>,
    pub author_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandaloneCredentialInfo {
    pub credential: StandaloneCredential,
    pub category: Option<CredentialCategory>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCredentialsRequest {
    pub query: Option<String>,
    pub credential_type: Option<SecretType>,
    pub category_id: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCredentialsResponse {
    pub credentials: Vec<StandaloneCredentialInfo>,
    pub total_count: i64,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_category_id: Option<i64>,
    pub color_code: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryWithChildren {
    pub category: CredentialCategory,
    pub children: Vec<CategoryWithChildren>,
    pub credential_count: i64,
}

// Repository trait
pub trait VaultRepository {
    fn create_vault(&self, request: CreateVaultRequest) -> Result<IdentityVault>;
    fn get_vault_by_id(&self, vault_id: i64) -> Result<Option<IdentityVault>>;
    fn get_vault_by_asset_id(&self, asset_id: i64) -> Result<Option<VaultInfo>>;
    fn update_vault(&self, vault: &IdentityVault) -> Result<()>;
    fn delete_vault(&self, vault_id: i64) -> Result<()>;
    
    fn add_secret(&self, request: AddSecretRequest) -> Result<VaultSecret>;
    fn get_vault_secrets(&self, vault_id: i64) -> Result<Vec<VaultSecret>>;
    fn get_secret_by_id(&self, secret_id: i64) -> Result<Option<VaultSecret>>;
    fn update_secret(&self, secret: &VaultSecret, author_id: i64) -> Result<()>;
    fn delete_secret(&self, secret_id: i64, author_id: i64) -> Result<()>;
    
    fn add_version_history(&self, vault_id: i64, change_type: ChangeType, author: i64, notes: &str, changes: HashMap<String, String>) -> Result<()>;
    fn get_vault_history(&self, vault_id: i64) -> Result<Vec<VaultVersion>>;
    
    fn import_vault(&self, vault_info: &VaultInfo, author_id: i64) -> Result<IdentityVault>;
    fn initialize_schema(&self) -> Result<()>;
    
    // Password management methods
    fn add_password_history(&self, secret_id: i64, password_hash: &str) -> Result<()>;
    fn get_password_history(&self, secret_id: i64) -> Result<Vec<PasswordHistory>>;
    fn check_password_reuse(&self, password_hash: &str, exclude_secret_id: Option<i64>) -> Result<bool>;
    fn update_password(&self, request: UpdateCredentialPasswordRequest, password_hash: &str, strength_score: i32) -> Result<()>;
    fn get_default_password_policy(&self) -> Result<PasswordPolicy>;
    fn cleanup_password_history(&self, secret_id: i64, keep_count: usize) -> Result<()>;
    
    // Standalone credential methods for Story 4.3
    fn create_standalone_credential(&self, request: CreateStandaloneCredentialRequest) -> Result<StandaloneCredential>;
    fn get_standalone_credential(&self, credential_id: i64) -> Result<Option<StandaloneCredentialInfo>>;
    fn update_standalone_credential(&self, request: UpdateStandaloneCredentialRequest) -> Result<()>;
    fn delete_standalone_credential(&self, credential_id: i64, author_id: i64) -> Result<()>;
    fn search_standalone_credentials(&self, request: SearchCredentialsRequest) -> Result<SearchCredentialsResponse>;
    fn get_standalone_credential_history(&self, credential_id: i64) -> Result<Vec<StandaloneCredentialHistory>>;
    fn update_credential_last_accessed(&self, credential_id: i64) -> Result<()>;
    
    // Category management methods
    fn create_credential_category(&self, request: CreateCategoryRequest) -> Result<CredentialCategory>;
    fn get_credential_categories(&self) -> Result<Vec<CategoryWithChildren>>;
    fn update_credential_category(&self, category_id: i64, request: CreateCategoryRequest) -> Result<()>;
    fn delete_credential_category(&self, category_id: i64) -> Result<()>;
    fn get_category_by_id(&self, category_id: i64) -> Result<Option<CredentialCategory>>;
    
    // Tag management methods
    fn add_credential_tags(&self, credential_id: i64, tags: &[String]) -> Result<()>;
    fn remove_credential_tag(&self, credential_id: i64, tag_name: &str) -> Result<()>;
    fn get_all_tags(&self) -> Result<Vec<String>>;
}

// SQLite implementation
pub struct SqliteVaultRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteVaultRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    fn row_to_vault(row: &Row) -> rusqlite::Result<IdentityVault> {
        Ok(IdentityVault {
            id: row.get("id")?,
            asset_id: row.get("asset_id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    fn row_to_secret(row: &Row) -> rusqlite::Result<VaultSecret> {
        let secret_type_str: String = row.get("secret_type")?;
        let secret_type = SecretType::from_str(&secret_type_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "secret_type".to_string(), rusqlite::types::Type::Text))?;

        Ok(VaultSecret {
            id: row.get("id")?,
            vault_id: row.get("vault_id")?,
            secret_type,
            label: row.get("label")?,
            encrypted_value: row.get("encrypted_value")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            strength_score: row.get("strength_score").ok(),
            last_changed: row.get("last_changed").ok(),
            generation_method: row.get("generation_method").ok(),
            policy_version: row.get("policy_version").ok(),
        })
    }

    fn row_to_password_history(row: &Row) -> rusqlite::Result<PasswordHistory> {
        Ok(PasswordHistory {
            id: row.get("id")?,
            secret_id: row.get("secret_id")?,
            password_hash: row.get("password_hash")?,
            created_at: row.get("created_at")?,
            retired_at: row.get("retired_at").ok(),
        })
    }

    fn row_to_version(row: &Row) -> rusqlite::Result<VaultVersion> {
        let change_type_str: String = row.get("change_type")?;
        let change_type = ChangeType::from_str(&change_type_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "change_type".to_string(), rusqlite::types::Type::Text))?;

        Ok(VaultVersion {
            id: row.get("id")?,
            vault_id: row.get("vault_id")?,
            change_type,
            author: row.get("author")?,
            timestamp: row.get("timestamp")?,
            notes: row.get("notes")?,
            changes_json: row.get("changes_json")?,
        })
    }

    fn row_to_standalone_credential(row: &Row) -> rusqlite::Result<StandaloneCredential> {
        let credential_type_str: String = row.get("credential_type")?;
        let credential_type = SecretType::from_str(&credential_type_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "credential_type".to_string(), rusqlite::types::Type::Text))?;

        Ok(StandaloneCredential {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            credential_type,
            category_id: row.get("category_id").ok(),
            encrypted_data: row.get("encrypted_data")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            last_accessed: row.get("last_accessed").ok(),
        })
    }

    fn row_to_category(row: &Row) -> rusqlite::Result<CredentialCategory> {
        Ok(CredentialCategory {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description").ok(),
            parent_category_id: row.get("parent_category_id").ok(),
            color_code: row.get("color_code").ok(),
            icon: row.get("icon").ok(),
            created_at: row.get("created_at")?,
        })
    }

    fn row_to_standalone_history(row: &Row) -> rusqlite::Result<StandaloneCredentialHistory> {
        let change_type_str: String = row.get("change_type")?;
        let change_type = StandaloneChangeType::from_str(&change_type_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "change_type".to_string(), rusqlite::types::Type::Text))?;

        Ok(StandaloneCredentialHistory {
            id: row.get("id")?,
            credential_id: row.get("credential_id")?,
            change_type,
            author: row.get("author")?,
            timestamp: row.get("timestamp")?,
            notes: row.get("notes").ok(),
            changes_json: row.get("changes_json").ok(),
        })
    }
}

impl<'a> VaultRepository for SqliteVaultRepository<'a> {
    fn initialize_schema(&self) -> Result<()> {
        info!("Initializing vault database schema");
        
        self.conn.execute_batch(
            r#"
            -- Vault entries table
            CREATE TABLE IF NOT EXISTS vault_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                asset_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
                FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE RESTRICT,
                UNIQUE(asset_id, name)
            );

            -- Vault secrets table
            CREATE TABLE IF NOT EXISTS vault_secrets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                vault_id INTEGER NOT NULL,
                secret_type TEXT NOT NULL CHECK(secret_type IN ('password', 'ip_address', 'vpn_key', 'license_file')),
                label TEXT NOT NULL,
                encrypted_value TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                -- Password management fields
                strength_score INTEGER,
                last_changed DATETIME,
                generation_method TEXT,
                policy_version INTEGER,
                FOREIGN KEY (vault_id) REFERENCES vault_entries(id) ON DELETE CASCADE,
                UNIQUE(vault_id, label)
            );

            -- Vault version history table
            CREATE TABLE IF NOT EXISTS vault_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                vault_id INTEGER NOT NULL,
                change_type TEXT NOT NULL CHECK(change_type IN ('vault_created', 'secret_added', 'secret_updated', 'secret_deleted', 'vault_updated')),
                author INTEGER NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                notes TEXT,
                changes_json TEXT,
                FOREIGN KEY (vault_id) REFERENCES vault_entries(id) ON DELETE CASCADE,
                FOREIGN KEY (author) REFERENCES users(id) ON DELETE RESTRICT
            );

            -- Password history table for tracking password changes and preventing reuse
            CREATE TABLE IF NOT EXISTS password_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                secret_id INTEGER NOT NULL,
                password_hash TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                retired_at DATETIME,
                FOREIGN KEY (secret_id) REFERENCES vault_secrets(id) ON DELETE CASCADE
            );

            -- Password policy configuration table
            CREATE TABLE IF NOT EXISTS password_policies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                min_length INTEGER NOT NULL DEFAULT 12,
                require_uppercase BOOLEAN NOT NULL DEFAULT 1,
                require_lowercase BOOLEAN NOT NULL DEFAULT 1,
                require_numbers BOOLEAN NOT NULL DEFAULT 1,
                require_special BOOLEAN NOT NULL DEFAULT 1,
                max_age_days INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- Indexes for performance
            CREATE INDEX IF NOT EXISTS idx_vault_entries_asset_id ON vault_entries(asset_id);
            CREATE INDEX IF NOT EXISTS idx_vault_entries_created_by ON vault_entries(created_by);
            CREATE INDEX IF NOT EXISTS idx_vault_entries_created_at ON vault_entries(created_at);
            
            CREATE INDEX IF NOT EXISTS idx_vault_secrets_vault_id ON vault_secrets(vault_id);
            CREATE INDEX IF NOT EXISTS idx_vault_secrets_secret_type ON vault_secrets(secret_type);
            CREATE INDEX IF NOT EXISTS idx_vault_secrets_created_at ON vault_secrets(created_at);
            CREATE INDEX IF NOT EXISTS idx_vault_secrets_strength_score ON vault_secrets(strength_score);
            
            CREATE INDEX IF NOT EXISTS idx_vault_versions_vault_id ON vault_versions(vault_id);
            CREATE INDEX IF NOT EXISTS idx_vault_versions_timestamp ON vault_versions(timestamp);
            CREATE INDEX IF NOT EXISTS idx_vault_versions_author ON vault_versions(author);
            
            CREATE INDEX IF NOT EXISTS idx_password_history_secret_id ON password_history(secret_id);
            CREATE INDEX IF NOT EXISTS idx_password_history_password_hash ON password_history(password_hash);
            CREATE INDEX IF NOT EXISTS idx_password_history_created_at ON password_history(created_at);

            -- Insert default password policy if none exists
            INSERT OR IGNORE INTO password_policies (id, min_length, require_uppercase, require_lowercase, require_numbers, require_special)
            VALUES (1, 12, 1, 1, 1, 1);

            -- Standalone credentials tables for Story 4.3
            CREATE TABLE IF NOT EXISTS standalone_credentials (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                credential_type TEXT NOT NULL CHECK(credential_type IN ('password', 'ip_address', 'vpn_key', 'license_file')),
                category_id INTEGER,
                encrypted_data TEXT NOT NULL,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_accessed DATETIME,
                FOREIGN KEY (category_id) REFERENCES credential_categories(id) ON DELETE SET NULL,
                FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE RESTRICT
            );

            -- Credential categories table for hierarchical organization
            CREATE TABLE IF NOT EXISTS credential_categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                parent_category_id INTEGER,
                color_code TEXT,
                icon TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (parent_category_id) REFERENCES credential_categories(id) ON DELETE CASCADE,
                UNIQUE(name, parent_category_id)
            );

            -- Credential tags for flexible organization
            CREATE TABLE IF NOT EXISTS credential_tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                standalone_credential_id INTEGER NOT NULL,
                tag_name TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (standalone_credential_id) REFERENCES standalone_credentials(id) ON DELETE CASCADE,
                UNIQUE(standalone_credential_id, tag_name)
            );

            -- Standalone credential history table
            CREATE TABLE IF NOT EXISTS standalone_credential_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                credential_id INTEGER NOT NULL,
                change_type TEXT NOT NULL CHECK(change_type IN ('created', 'updated', 'accessed', 'deleted')),
                author INTEGER NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                notes TEXT,
                changes_json TEXT,
                FOREIGN KEY (credential_id) REFERENCES standalone_credentials(id) ON DELETE CASCADE,
                FOREIGN KEY (author) REFERENCES users(id) ON DELETE RESTRICT
            );

            -- Indexes for standalone credentials performance
            CREATE INDEX IF NOT EXISTS idx_standalone_credentials_name ON standalone_credentials(name);
            CREATE INDEX IF NOT EXISTS idx_standalone_credentials_category_id ON standalone_credentials(category_id);
            CREATE INDEX IF NOT EXISTS idx_standalone_credentials_credential_type ON standalone_credentials(credential_type);
            CREATE INDEX IF NOT EXISTS idx_standalone_credentials_created_at ON standalone_credentials(created_at);
            CREATE INDEX IF NOT EXISTS idx_standalone_credentials_last_accessed ON standalone_credentials(last_accessed);
            CREATE INDEX IF NOT EXISTS idx_standalone_credentials_created_by ON standalone_credentials(created_by);
            
            CREATE INDEX IF NOT EXISTS idx_credential_categories_parent_id ON credential_categories(parent_category_id);
            CREATE INDEX IF NOT EXISTS idx_credential_categories_name ON credential_categories(name);
            
            CREATE INDEX IF NOT EXISTS idx_credential_tags_credential_id ON credential_tags(standalone_credential_id);
            CREATE INDEX IF NOT EXISTS idx_credential_tags_tag_name ON credential_tags(tag_name);
            
            CREATE INDEX IF NOT EXISTS idx_standalone_credential_history_credential_id ON standalone_credential_history(credential_id);
            CREATE INDEX IF NOT EXISTS idx_standalone_credential_history_timestamp ON standalone_credential_history(timestamp);
            CREATE INDEX IF NOT EXISTS idx_standalone_credential_history_author ON standalone_credential_history(author);

            -- Insert predefined credential categories
            INSERT OR IGNORE INTO credential_categories (id, name, description, parent_category_id, color_code, icon)
            VALUES 
                (1, 'Jump Hosts', 'SSH jump servers and bastion hosts', NULL, '#4CAF50', 'server'),
                (2, 'Databases', 'Database server credentials', NULL, '#2196F3', 'database'),
                (3, 'Network Equipment', 'Switches, routers, and firewalls', NULL, '#FF9800', 'network'),
                (4, 'Applications', 'Application and service credentials', NULL, '#9C27B0', 'apps'),
                (5, 'Cloud Services', 'Cloud platform credentials', NULL, '#00BCD4', 'cloud');
            "#,
        )?;

        info!("Vault database schema initialized successfully");
        Ok(())
    }

    fn create_vault(&self, request: CreateVaultRequest) -> Result<IdentityVault> {
        // Validate vault name
        if request.name.trim().is_empty() {
            return Err(anyhow::anyhow!("Vault name cannot be empty"));
        }
        if request.name.len() < 2 {
            return Err(anyhow::anyhow!("Vault name must be at least 2 characters long"));
        }
        if request.name.len() > 100 {
            return Err(anyhow::anyhow!("Vault name cannot exceed 100 characters"));
        }

        debug!("Creating vault '{}' for asset {}", request.name, request.asset_id);

        let mut stmt = self.conn.prepare(
            "INSERT INTO vault_entries (asset_id, name, description, created_by) 
             VALUES (?1, ?2, ?3, ?4) 
             RETURNING id, asset_id, name, description, created_by, created_at, updated_at"
        )?;

        let vault = stmt.query_row(
            (&request.asset_id, &request.name, &request.description, &request.created_by),
            Self::row_to_vault,
        )?;

        // Add version history for vault creation
        let mut changes = HashMap::new();
        changes.insert("name".to_string(), request.name.clone());
        changes.insert("description".to_string(), request.description.clone());
        
        self.add_version_history(
            vault.id,
            ChangeType::VaultCreated,
            request.created_by,
            &format!("Created vault '{}'", request.name),
            changes,
        )?;

        info!("Created vault '{}' with ID {} for asset {}", vault.name, vault.id, vault.asset_id);
        Ok(vault)
    }

    fn get_vault_by_id(&self, vault_id: i64) -> Result<Option<IdentityVault>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, asset_id, name, description, created_by, created_at, updated_at 
             FROM vault_entries WHERE id = ?1"
        )?;

        let result = stmt.query_row([vault_id], Self::row_to_vault);
        
        match result {
            Ok(vault) => Ok(Some(vault)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn get_vault_by_asset_id(&self, asset_id: i64) -> Result<Option<VaultInfo>> {
        // First get the vault
        let mut vault_stmt = self.conn.prepare(
            "SELECT id, asset_id, name, description, created_by, created_at, updated_at 
             FROM vault_entries WHERE asset_id = ?1"
        )?;

        let vault_result = vault_stmt.query_row([asset_id], Self::row_to_vault);
        
        let vault = match vault_result {
            Ok(vault) => vault,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // Then get all secrets for this vault
        let secrets = self.get_vault_secrets(vault.id)?;
        let secret_count = secrets.len();

        Ok(Some(VaultInfo {
            vault,
            secrets,
            secret_count,
        }))
    }

    fn update_vault(&self, vault: &IdentityVault) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE vault_entries 
             SET name = ?1, description = ?2, updated_at = CURRENT_TIMESTAMP 
             WHERE id = ?3",
            (&vault.name, &vault.description, &vault.id),
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Vault not found"));
        }

        info!("Updated vault '{}' (ID: {})", vault.name, vault.id);
        Ok(())
    }

    fn delete_vault(&self, vault_id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM vault_entries WHERE id = ?1",
            [vault_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Vault not found"));
        }

        info!("Deleted vault with ID: {}", vault_id);
        Ok(())
    }

    fn add_secret(&self, request: AddSecretRequest) -> Result<VaultSecret> {
        // Validate secret data
        if request.label.trim().is_empty() {
            return Err(anyhow::anyhow!("Secret label cannot be empty"));
        }
        if request.value.trim().is_empty() {
            return Err(anyhow::anyhow!("Secret value cannot be empty"));
        }

        // Encrypt the secret value
        let encryption = FileEncryption::new(&format!("vault_{}_{}", request.vault_id, request.author_id));
        let encrypted_value = encryption.encrypt(request.value.as_bytes())?;
        use base64::{Engine as _, engine::general_purpose};
        let encrypted_value_base64 = general_purpose::STANDARD.encode(encrypted_value);

        debug!("Adding {} secret '{}' to vault {}", 
               request.secret_type.to_string(), request.label, request.vault_id);

        let mut stmt = self.conn.prepare(
            "INSERT INTO vault_secrets (vault_id, secret_type, label, encrypted_value) 
             VALUES (?1, ?2, ?3, ?4) 
             RETURNING id, vault_id, secret_type, label, encrypted_value, created_at, updated_at"
        )?;

        let secret = stmt.query_row(
            (&request.vault_id, &request.secret_type.to_string(), &request.label, &encrypted_value_base64),
            Self::row_to_secret,
        )?;

        // Add version history for secret addition
        let mut changes = HashMap::new();
        changes.insert("label".to_string(), request.label.clone());
        changes.insert("secret_type".to_string(), request.secret_type.to_string());
        
        self.add_version_history(
            request.vault_id,
            ChangeType::SecretAdded,
            request.author_id,
            &format!("Added {} secret '{}'", request.secret_type.to_string(), request.label),
            changes,
        )?;

        info!("Added {} secret '{}' to vault {}", 
              request.secret_type.to_string(), request.label, request.vault_id);
        Ok(secret)
    }

    fn get_vault_secrets(&self, vault_id: i64) -> Result<Vec<VaultSecret>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, vault_id, secret_type, label, encrypted_value, created_at, updated_at 
             FROM vault_secrets WHERE vault_id = ?1 ORDER BY created_at ASC"
        )?;

        let secret_iter = stmt.query_map([vault_id], Self::row_to_secret)?;
        let mut secrets = Vec::new();

        for secret in secret_iter {
            secrets.push(secret?);
        }

        Ok(secrets)
    }

    fn get_secret_by_id(&self, secret_id: i64) -> Result<Option<VaultSecret>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, vault_id, secret_type, label, encrypted_value, created_at, updated_at 
             FROM vault_secrets WHERE id = ?1"
        )?;

        let result = stmt.query_row([secret_id], Self::row_to_secret);
        
        match result {
            Ok(secret) => Ok(Some(secret)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn update_secret(&self, secret: &VaultSecret, author_id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE vault_secrets 
             SET label = ?1, encrypted_value = ?2, updated_at = CURRENT_TIMESTAMP 
             WHERE id = ?3",
            (&secret.label, &secret.encrypted_value, &secret.id),
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Secret not found"));
        }

        // Add version history for secret update
        let mut changes = HashMap::new();
        changes.insert("label".to_string(), secret.label.clone());
        
        self.add_version_history(
            secret.vault_id,
            ChangeType::SecretUpdated,
            author_id,
            &format!("Updated {} secret '{}'", secret.secret_type.to_string(), secret.label),
            changes,
        )?;

        info!("Updated secret '{}' (ID: {})", secret.label, secret.id);
        Ok(())
    }

    fn delete_secret(&self, secret_id: i64, author_id: i64) -> Result<()> {
        // Get secret details before deletion for audit trail
        let secret = self.get_secret_by_id(secret_id)?
            .ok_or_else(|| anyhow::anyhow!("Secret not found"))?;

        let rows_affected = self.conn.execute(
            "DELETE FROM vault_secrets WHERE id = ?1",
            [secret_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Secret not found"));
        }

        // Add version history for secret deletion
        let mut changes = HashMap::new();
        changes.insert("label".to_string(), secret.label.clone());
        changes.insert("secret_type".to_string(), secret.secret_type.to_string());
        
        self.add_version_history(
            secret.vault_id,
            ChangeType::SecretDeleted,
            author_id,
            &format!("Deleted {} secret '{}'", secret.secret_type.to_string(), secret.label),
            changes,
        )?;

        info!("Deleted secret '{}' (ID: {})", secret.label, secret_id);
        Ok(())
    }

    fn add_version_history(&self, vault_id: i64, change_type: ChangeType, author: i64, notes: &str, changes: HashMap<String, String>) -> Result<()> {
        let changes_json = serde_json::to_string(&changes)?;
        
        self.conn.execute(
            "INSERT INTO vault_versions (vault_id, change_type, author, notes, changes_json) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (&vault_id, &change_type.to_string(), &author, &notes, &changes_json),
        )?;

        debug!("Added version history for vault {}: {}", vault_id, notes);
        Ok(())
    }

    fn get_vault_history(&self, vault_id: i64) -> Result<Vec<VaultVersion>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, vault_id, change_type, author, timestamp, notes, changes_json 
             FROM vault_versions WHERE vault_id = ?1 ORDER BY timestamp DESC"
        )?;

        let version_iter = stmt.query_map([vault_id], Self::row_to_version)?;
        let mut versions = Vec::new();

        for version in version_iter {
            versions.push(version?);
        }

        Ok(versions)
    }

    fn import_vault(&self, vault_info: &VaultInfo, author_id: i64) -> Result<IdentityVault> {
        debug!("Importing vault '{}' for asset {}", vault_info.vault.name, vault_info.vault.asset_id);

        // Create the vault
        let create_request = CreateVaultRequest {
            asset_id: vault_info.vault.asset_id,
            name: vault_info.vault.name.clone(),
            description: vault_info.vault.description.clone(),
            created_by: author_id,
        };

        let imported_vault = self.create_vault(create_request)?;

        // Import all secrets (they are already encrypted from export)
        for secret in &vault_info.secrets {
            let secret_request = AddSecretRequest {
                vault_id: imported_vault.id,
                secret_type: secret.secret_type,
                label: secret.label.clone(),
                value: "imported_encrypted_value".to_string(), // Placeholder - we'll set encrypted value directly
                author_id,
            };

            // First create the secret with placeholder value
            let mut imported_secret = self.add_secret(secret_request)?;
            
            // Then update with the original encrypted value
            imported_secret.encrypted_value = secret.encrypted_value.clone();
            self.update_secret(&imported_secret, author_id)?;
        }

        // Add version history for import
        let mut changes = HashMap::new();
        changes.insert("imported_secrets".to_string(), vault_info.secret_count.to_string());
        changes.insert("original_vault_id".to_string(), vault_info.vault.id.to_string());
        
        self.add_version_history(
            imported_vault.id,
            ChangeType::VaultCreated,
            author_id,
            &format!("Imported vault '{}' with {} secrets from recovery package", vault_info.vault.name, vault_info.secret_count),
            changes,
        )?;

        info!("Imported vault '{}' with {} secrets", imported_vault.name, vault_info.secret_count);
        Ok(imported_vault)
    }

    // Password management method implementations
    fn add_password_history(&self, secret_id: i64, password_hash: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO password_history (secret_id, password_hash) VALUES (?1, ?2)",
            (secret_id, password_hash),
        )?;

        debug!("Added password history entry for secret {}", secret_id);
        Ok(())
    }

    fn get_password_history(&self, secret_id: i64) -> Result<Vec<PasswordHistory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, secret_id, password_hash, created_at, retired_at 
             FROM password_history WHERE secret_id = ?1 ORDER BY created_at DESC"
        )?;

        let history_iter = stmt.query_map([secret_id], Self::row_to_password_history)?;
        let mut history = Vec::new();

        for entry in history_iter {
            history.push(entry?);
        }

        Ok(history)
    }

    fn check_password_reuse(&self, password_hash: &str, exclude_secret_id: Option<i64>) -> Result<bool> {
        let query = if let Some(_exclude_id) = exclude_secret_id {
            "SELECT COUNT(*) FROM password_history ph 
             JOIN vault_secrets vs ON ph.secret_id = vs.id 
             WHERE ph.password_hash = ?1 AND vs.id != ?2 AND ph.retired_at IS NULL"
        } else {
            "SELECT COUNT(*) FROM password_history ph 
             JOIN vault_secrets vs ON ph.secret_id = vs.id 
             WHERE ph.password_hash = ?1 AND ph.retired_at IS NULL"
        };

        let mut stmt = self.conn.prepare(query)?;
        let count: i64 = if let Some(exclude_id) = exclude_secret_id {
            stmt.query_row((password_hash, exclude_id), |row| row.get(0))?
        } else {
            stmt.query_row([password_hash], |row| row.get(0))?
        };

        Ok(count > 0)
    }

    fn update_password(&self, request: UpdateCredentialPasswordRequest, password_hash: &str, strength_score: i32) -> Result<()> {
        // First, retire the old password in history if it exists
        self.conn.execute(
            "UPDATE password_history SET retired_at = CURRENT_TIMESTAMP 
             WHERE secret_id = ?1 AND retired_at IS NULL",
            [request.secret_id],
        )?;

        // Add new password to history
        self.add_password_history(request.secret_id, password_hash)?;

        // Encrypt the new password value
        let encryption = FileEncryption::new(&format!("vault_{}_{}", request.secret_id, request.author_id));
        let encrypted_value = encryption.encrypt(request.new_password.as_bytes())?;
        use base64::{Engine as _, engine::general_purpose};
        let encrypted_value_base64 = general_purpose::STANDARD.encode(encrypted_value);

        // Update the secret with new password and metadata
        self.conn.execute(
            "UPDATE vault_secrets 
             SET encrypted_value = ?1, updated_at = CURRENT_TIMESTAMP, 
                 strength_score = ?2, last_changed = CURRENT_TIMESTAMP,
                 generation_method = 'manual', policy_version = 1
             WHERE id = ?3",
            (&encrypted_value_base64, strength_score, request.secret_id),
        )?;

        // Add version history
        let mut changes = HashMap::new();
        changes.insert("password_updated".to_string(), "true".to_string());
        changes.insert("strength_score".to_string(), strength_score.to_string());

        // Get the vault_id for version history
        let vault_id: i64 = self.conn.query_row(
            "SELECT vault_id FROM vault_secrets WHERE id = ?1",
            [request.secret_id],
            |row| row.get(0),
        )?;

        self.add_version_history(
            vault_id,
            ChangeType::SecretUpdated,
            request.author_id,
            "Password updated with strength validation",
            changes,
        )?;

        // Clean up old password history (keep last 5)
        self.cleanup_password_history(request.secret_id, 5)?;

        info!("Updated password for secret {} with strength score {}", request.secret_id, strength_score);
        Ok(())
    }

    fn get_default_password_policy(&self) -> Result<PasswordPolicy> {
        let mut stmt = self.conn.prepare(
            "SELECT id, min_length, require_uppercase, require_lowercase, require_numbers, require_special, max_age_days, created_at, updated_at 
             FROM password_policies WHERE id = 1"
        )?;

        let policy = stmt.query_row([], |row| {
            Ok(PasswordPolicy {
                id: row.get("id")?,
                min_length: row.get("min_length")?,
                require_uppercase: row.get("require_uppercase")?,
                require_lowercase: row.get("require_lowercase")?,
                require_numbers: row.get("require_numbers")?,
                require_special: row.get("require_special")?,
                max_age_days: row.get("max_age_days").ok(),
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?;

        Ok(policy)
    }

    fn cleanup_password_history(&self, secret_id: i64, keep_count: usize) -> Result<()> {
        self.conn.execute(
            "DELETE FROM password_history 
             WHERE secret_id = ?1 AND id NOT IN (
                 SELECT id FROM password_history 
                 WHERE secret_id = ?1 
                 ORDER BY created_at DESC 
                 LIMIT ?2
             )",
            (secret_id, keep_count),
        )?;

        debug!("Cleaned up password history for secret {}, keeping {} entries", secret_id, keep_count);
        Ok(())
    }

    // Standalone credential implementations for Story 4.3
    fn create_standalone_credential(&self, request: CreateStandaloneCredentialRequest) -> Result<StandaloneCredential> {
        // Validate input
        if request.name.trim().is_empty() {
            return Err(anyhow::anyhow!("Credential name cannot be empty"));
        }
        if request.name.len() < 2 {
            return Err(anyhow::anyhow!("Credential name must be at least 2 characters long"));
        }
        if request.name.len() > 100 {
            return Err(anyhow::anyhow!("Credential name cannot exceed 100 characters"));
        }
        if request.value.trim().is_empty() {
            return Err(anyhow::anyhow!("Credential value cannot be empty"));
        }

        // Encrypt the credential value
        let encryption = FileEncryption::new(&format!("standalone_{}_{}", request.created_by, chrono::Utc::now().timestamp()));
        let encrypted_data = encryption.encrypt(request.value.as_bytes())?;
        use base64::{Engine as _, engine::general_purpose};
        let encrypted_data_base64 = general_purpose::STANDARD.encode(encrypted_data);

        debug!("Creating standalone credential '{}' of type {}", request.name, request.credential_type.to_string());

        let mut stmt = self.conn.prepare(
            "INSERT INTO standalone_credentials (name, description, credential_type, category_id, encrypted_data, created_by) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) 
             RETURNING id, name, description, credential_type, category_id, encrypted_data, created_by, created_at, updated_at, last_accessed"
        )?;

        let credential = stmt.query_row(
            (&request.name, &request.description, &request.credential_type.to_string(), 
             &request.category_id, &encrypted_data_base64, &request.created_by),
            Self::row_to_standalone_credential,
        )?;

        // Add tags if provided
        if let Some(tags) = &request.tags {
            self.add_credential_tags(credential.id, tags)?;
        }

        // Add history entry
        let mut changes = HashMap::new();
        changes.insert("name".to_string(), request.name.clone());
        changes.insert("credential_type".to_string(), request.credential_type.to_string());
        if let Some(category_id) = request.category_id {
            changes.insert("category_id".to_string(), category_id.to_string());
        }
        
        self.add_standalone_history(
            credential.id,
            StandaloneChangeType::Created,
            request.created_by,
            &format!("Created standalone credential '{}'", request.name),
            changes,
        )?;

        info!("Created standalone credential '{}' with ID {}", credential.name, credential.id);
        Ok(credential)
    }

    fn get_standalone_credential(&self, credential_id: i64) -> Result<Option<StandaloneCredentialInfo>> {
        // Get the credential
        let mut cred_stmt = self.conn.prepare(
            "SELECT id, name, description, credential_type, category_id, encrypted_data, created_by, created_at, updated_at, last_accessed 
             FROM standalone_credentials WHERE id = ?1"
        )?;

        let cred_result = cred_stmt.query_row([credential_id], Self::row_to_standalone_credential);
        
        let credential = match cred_result {
            Ok(cred) => cred,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // Get the category if it exists
        let category = if let Some(cat_id) = credential.category_id {
            self.get_category_by_id(cat_id)?
        } else {
            None
        };

        // Get tags
        let tags = self.get_credential_tags(credential_id)?;

        // Update last accessed
        self.update_credential_last_accessed(credential_id)?;

        Ok(Some(StandaloneCredentialInfo {
            credential,
            category,
            tags,
        }))
    }

    fn update_standalone_credential(&self, request: UpdateStandaloneCredentialRequest) -> Result<()> {
        let mut updates = Vec::new();
        let mut changes = HashMap::new();

        // Prepare the base query components
        if request.name.is_some() {
            updates.push("name = ?");
            changes.insert("name".to_string(), request.name.as_ref().unwrap().clone());
        }

        if request.description.is_some() {
            updates.push("description = ?");
            changes.insert("description".to_string(), request.description.as_ref().unwrap().clone());
        }

        if request.category_id.is_some() {
            updates.push("category_id = ?");
            changes.insert("category_id".to_string(), request.category_id.unwrap().to_string());
        }

        let encrypted_data_base64 = if let Some(ref value) = request.value {
            // Encrypt the new value
            let encryption = FileEncryption::new(&format!("standalone_{}_{}", request.id, request.author_id));
            let encrypted_data = encryption.encrypt(value.as_bytes())?;
            use base64::{Engine as _, engine::general_purpose};
            let encrypted_base64 = general_purpose::STANDARD.encode(encrypted_data);
            
            updates.push("encrypted_data = ?");
            changes.insert("value_updated".to_string(), "true".to_string());
            Some(encrypted_base64)
        } else {
            None
        };

        if updates.is_empty() {
            return Err(anyhow::anyhow!("No fields to update"));
        }

        updates.push("updated_at = CURRENT_TIMESTAMP");

        let query = format!(
            "UPDATE standalone_credentials SET {} WHERE id = ?",
            updates.join(", ")
        );

        // Build params based on what's being updated
        let rows_affected = match (&request.name, &request.description, &request.category_id, &encrypted_data_base64) {
            (Some(name), Some(desc), Some(cat_id), Some(enc_data)) => {
                self.conn.execute(&query, (name, desc, cat_id, enc_data, request.id))?
            },
            (Some(name), Some(desc), Some(cat_id), None) => {
                self.conn.execute(&query, (name, desc, cat_id, request.id))?
            },
            (Some(name), Some(desc), None, Some(enc_data)) => {
                self.conn.execute(&query, (name, desc, enc_data, request.id))?
            },
            (Some(name), Some(desc), None, None) => {
                self.conn.execute(&query, (name, desc, request.id))?
            },
            (Some(name), None, Some(cat_id), Some(enc_data)) => {
                self.conn.execute(&query, (name, cat_id, enc_data, request.id))?
            },
            (Some(name), None, Some(cat_id), None) => {
                self.conn.execute(&query, (name, cat_id, request.id))?
            },
            (Some(name), None, None, Some(enc_data)) => {
                self.conn.execute(&query, (name, enc_data, request.id))?
            },
            (Some(name), None, None, None) => {
                self.conn.execute(&query, (name, request.id))?
            },
            (None, Some(desc), Some(cat_id), Some(enc_data)) => {
                self.conn.execute(&query, (desc, cat_id, enc_data, request.id))?
            },
            (None, Some(desc), Some(cat_id), None) => {
                self.conn.execute(&query, (desc, cat_id, request.id))?
            },
            (None, Some(desc), None, Some(enc_data)) => {
                self.conn.execute(&query, (desc, enc_data, request.id))?
            },
            (None, Some(desc), None, None) => {
                self.conn.execute(&query, (desc, request.id))?
            },
            (None, None, Some(cat_id), Some(enc_data)) => {
                self.conn.execute(&query, (cat_id, enc_data, request.id))?
            },
            (None, None, Some(cat_id), None) => {
                self.conn.execute(&query, (cat_id, request.id))?
            },
            (None, None, None, Some(enc_data)) => {
                self.conn.execute(&query, (enc_data, request.id))?
            },
            (None, None, None, None) => {
                return Err(anyhow::anyhow!("No fields to update"));
            }
        };

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Credential not found"));
        }

        // Add history entry
        self.add_standalone_history(
            request.id,
            StandaloneChangeType::Updated,
            request.author_id,
            "Updated standalone credential",
            changes,
        )?;

        info!("Updated standalone credential with ID {}", request.id);
        Ok(())
    }

    fn delete_standalone_credential(&self, credential_id: i64, author_id: i64) -> Result<()> {
        // Get credential details before deletion for audit trail
        let credential_info = self.get_standalone_credential(credential_id)?
            .ok_or_else(|| anyhow::anyhow!("Credential not found"))?;

        // Add history entry before deletion
        let mut changes = HashMap::new();
        changes.insert("name".to_string(), credential_info.credential.name.clone());
        changes.insert("credential_type".to_string(), credential_info.credential.credential_type.to_string());
        
        self.add_standalone_history(
            credential_id,
            StandaloneChangeType::Deleted,
            author_id,
            &format!("Deleted standalone credential '{}'", credential_info.credential.name),
            changes,
        )?;

        let rows_affected = self.conn.execute(
            "DELETE FROM standalone_credentials WHERE id = ?1",
            [credential_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Credential not found"));
        }

        info!("Deleted standalone credential '{}' (ID: {})", credential_info.credential.name, credential_id);
        Ok(())
    }

    fn search_standalone_credentials(&self, request: SearchCredentialsRequest) -> Result<SearchCredentialsResponse> {
        let mut query = String::from(
            "SELECT DISTINCT sc.id, sc.name, sc.description, sc.credential_type, sc.category_id, 
                    sc.encrypted_data, sc.created_by, sc.created_at, sc.updated_at, sc.last_accessed 
             FROM standalone_credentials sc"
        );
        
        let mut joins = Vec::new();
        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // Add tag join if searching by tags
        if let Some(ref tags) = request.tags {
            if !tags.is_empty() {
                joins.push("LEFT JOIN credential_tags ct ON sc.id = ct.standalone_credential_id");
            }
        }

        // Build WHERE conditions
        let mut param_index = 1;
        if let Some(ref query_str) = request.query {
            conditions.push(format!("(sc.name LIKE ?{} OR sc.description LIKE ?{})", param_index, param_index));
            params.push(Box::new(format!("%{}%", query_str)));
            param_index += 1;
        }

        if let Some(ref cred_type) = request.credential_type {
            conditions.push(format!("sc.credential_type = ?{}", param_index));
            params.push(Box::new(cred_type.to_string()));
            param_index += 1;
        }

        if let Some(category_id) = request.category_id {
            conditions.push(format!("sc.category_id = ?{}", param_index));
            params.push(Box::new(category_id));
            param_index += 1;
        }

        if let Some(ref tags) = request.tags {
            if !tags.is_empty() {
                let tag_placeholders = tags.iter().enumerate()
                    .map(|(i, _)| format!("?{}", param_index + i))
                    .collect::<Vec<_>>()
                    .join(", ");
                conditions.push(format!("ct.tag_name IN ({})", tag_placeholders));
                for tag in tags {
                    params.push(Box::new(tag.clone()));
                }
                param_index += tags.len();
            }
        }

        if let Some(ref created_after) = request.created_after {
            conditions.push(format!("sc.created_at >= ?{}", param_index));
            params.push(Box::new(created_after.clone()));
            param_index += 1;
        }

        if let Some(ref created_before) = request.created_before {
            conditions.push(format!("sc.created_at <= ?{}", param_index));
            params.push(Box::new(created_before.clone()));
            // param_index += 1; // Not needed as it's the last parameter
        }

        // Combine joins and conditions
        if !joins.is_empty() {
            query.push_str(" ");
            query.push_str(&joins.join(" "));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        // Count total results
        let count_query = format!("SELECT COUNT(DISTINCT sc.id) FROM standalone_credentials sc {} {}", 
            if !joins.is_empty() { joins.join(" ") } else { String::new() },
            if !conditions.is_empty() { format!("WHERE {}", conditions.join(" AND ")) } else { String::new() }
        );

        let total_count: i64 = self.conn.query_row(&count_query, rusqlite::params_from_iter(params.iter()), |row| row.get(0))?;

        // Add ordering and pagination
        query.push_str(" ORDER BY sc.created_at DESC");
        
        let limit = request.limit.unwrap_or(50).min(100);
        let offset = request.offset.unwrap_or(0);
        query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        // Execute search query
        let mut stmt = self.conn.prepare(&query)?;
        let cred_iter = stmt.query_map(rusqlite::params_from_iter(params.iter()), Self::row_to_standalone_credential)?;
        
        let mut credentials = Vec::new();
        for cred_result in cred_iter {
            let cred = cred_result?;
            
            // Get category info
            let category = if let Some(cat_id) = cred.category_id {
                self.get_category_by_id(cat_id)?
            } else {
                None
            };
            
            // Get tags
            let tags = self.get_credential_tags(cred.id)?;
            
            credentials.push(StandaloneCredentialInfo {
                credential: cred,
                category,
                tags,
            });
        }

        let page = (offset / limit) + 1;

        Ok(SearchCredentialsResponse {
            credentials,
            total_count,
            page: page as i32,
            page_size: limit,
        })
    }

    fn get_standalone_credential_history(&self, credential_id: i64) -> Result<Vec<StandaloneCredentialHistory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, credential_id, change_type, author, timestamp, notes, changes_json 
             FROM standalone_credential_history WHERE credential_id = ?1 ORDER BY timestamp DESC"
        )?;

        let history_iter = stmt.query_map([credential_id], Self::row_to_standalone_history)?;
        let mut history = Vec::new();

        for entry in history_iter {
            history.push(entry?);
        }

        Ok(history)
    }

    fn update_credential_last_accessed(&self, credential_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE standalone_credentials SET last_accessed = CURRENT_TIMESTAMP WHERE id = ?1",
            [credential_id],
        )?;

        debug!("Updated last accessed time for credential {}", credential_id);
        Ok(())
    }

    // Category management implementations
    fn create_credential_category(&self, request: CreateCategoryRequest) -> Result<CredentialCategory> {
        if request.name.trim().is_empty() {
            return Err(anyhow::anyhow!("Category name cannot be empty"));
        }

        debug!("Creating credential category '{}'", request.name);

        let mut stmt = self.conn.prepare(
            "INSERT INTO credential_categories (name, description, parent_category_id, color_code, icon) 
             VALUES (?1, ?2, ?3, ?4, ?5) 
             RETURNING id, name, description, parent_category_id, color_code, icon, created_at"
        )?;

        let category = stmt.query_row(
            (&request.name, &request.description, &request.parent_category_id, 
             &request.color_code, &request.icon),
            Self::row_to_category,
        )?;

        info!("Created credential category '{}' with ID {}", category.name, category.id);
        Ok(category)
    }

    fn get_credential_categories(&self) -> Result<Vec<CategoryWithChildren>> {
        // Get all categories
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, parent_category_id, color_code, icon, created_at 
             FROM credential_categories ORDER BY name"
        )?;

        let cat_iter = stmt.query_map([], Self::row_to_category)?;
        let mut all_categories = Vec::new();

        for cat in cat_iter {
            all_categories.push(cat?);
        }

        // Build hierarchical structure
        let mut root_categories = Vec::new();
        let mut category_map: HashMap<i64, Vec<CredentialCategory>> = HashMap::new();

        // Group categories by parent
        for cat in &all_categories {
            if let Some(parent_id) = cat.parent_category_id {
                category_map.entry(parent_id).or_insert_with(Vec::new).push(cat.clone());
            } else {
                root_categories.push(cat.clone());
            }
        }

        // Build tree structure
        let mut result = Vec::new();
        for root_cat in root_categories {
            let children = self.build_category_tree(&root_cat, &category_map)?;
            let credential_count = self.get_category_credential_count(root_cat.id)?;
            
            result.push(CategoryWithChildren {
                category: root_cat,
                children,
                credential_count,
            });
        }

        Ok(result)
    }

    fn update_credential_category(&self, category_id: i64, request: CreateCategoryRequest) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE credential_categories 
             SET name = ?1, description = ?2, parent_category_id = ?3, color_code = ?4, icon = ?5 
             WHERE id = ?6",
            (&request.name, &request.description, &request.parent_category_id, 
             &request.color_code, &request.icon, &category_id),
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Category not found"));
        }

        info!("Updated credential category with ID {}", category_id);
        Ok(())
    }

    fn delete_credential_category(&self, category_id: i64) -> Result<()> {
        // Check if category has credentials
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM standalone_credentials WHERE category_id = ?1",
            [category_id],
            |row| row.get(0),
        )?;

        if count > 0 {
            return Err(anyhow::anyhow!("Cannot delete category with associated credentials"));
        }

        let rows_affected = self.conn.execute(
            "DELETE FROM credential_categories WHERE id = ?1",
            [category_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Category not found"));
        }

        info!("Deleted credential category with ID {}", category_id);
        Ok(())
    }

    fn get_category_by_id(&self, category_id: i64) -> Result<Option<CredentialCategory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, parent_category_id, color_code, icon, created_at 
             FROM credential_categories WHERE id = ?1"
        )?;

        let result = stmt.query_row([category_id], Self::row_to_category);
        
        match result {
            Ok(category) => Ok(Some(category)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // Tag management implementations
    fn add_credential_tags(&self, credential_id: i64, tags: &[String]) -> Result<()> {
        for tag in tags {
            if !tag.trim().is_empty() {
                self.conn.execute(
                    "INSERT OR IGNORE INTO credential_tags (standalone_credential_id, tag_name) VALUES (?1, ?2)",
                    (credential_id, tag.trim()),
                )?;
            }
        }

        debug!("Added {} tags to credential {}", tags.len(), credential_id);
        Ok(())
    }

    fn remove_credential_tag(&self, credential_id: i64, tag_name: &str) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM credential_tags WHERE standalone_credential_id = ?1 AND tag_name = ?2",
            (credential_id, tag_name),
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Tag not found"));
        }

        debug!("Removed tag '{}' from credential {}", tag_name, credential_id);
        Ok(())
    }

    fn get_all_tags(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT tag_name FROM credential_tags ORDER BY tag_name"
        )?;

        let tag_iter = stmt.query_map([], |row| row.get(0))?;
        let mut tags = Vec::new();

        for tag in tag_iter {
            tags.push(tag?);
        }

        Ok(tags)
    }
}

// Helper methods
impl<'a> SqliteVaultRepository<'a> {
    fn add_standalone_history(&self, credential_id: i64, change_type: StandaloneChangeType, author: i64, notes: &str, changes: HashMap<String, String>) -> Result<()> {
        let changes_json = serde_json::to_string(&changes)?;
        
        self.conn.execute(
            "INSERT INTO standalone_credential_history (credential_id, change_type, author, notes, changes_json) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (&credential_id, &change_type.to_string(), &author, &notes, &changes_json),
        )?;

        debug!("Added history entry for standalone credential {}: {}", credential_id, notes);
        Ok(())
    }

    fn get_credential_tags(&self, credential_id: i64) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT tag_name FROM credential_tags WHERE standalone_credential_id = ?1 ORDER BY tag_name"
        )?;

        let tag_iter = stmt.query_map([credential_id], |row| row.get(0))?;
        let mut tags = Vec::new();

        for tag in tag_iter {
            tags.push(tag?);
        }

        Ok(tags)
    }

    fn build_category_tree(&self, parent: &CredentialCategory, category_map: &HashMap<i64, Vec<CredentialCategory>>) -> Result<Vec<CategoryWithChildren>> {
        let mut children = Vec::new();

        if let Some(child_categories) = category_map.get(&parent.id) {
            for child_cat in child_categories {
                let sub_children = self.build_category_tree(child_cat, category_map)?;
                let credential_count = self.get_category_credential_count(child_cat.id)?;
                
                children.push(CategoryWithChildren {
                    category: child_cat.clone(),
                    children: sub_children,
                    credential_count,
                });
            }
        }

        Ok(children)
    }

    fn get_category_credential_count(&self, category_id: i64) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM standalone_credentials WHERE category_id = ?1",
            [category_id],
            |row| row.get(0),
        )?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use rusqlite::Connection;

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        
        // Create required tables for foreign key constraints
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
            
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'testuser', 'hash', 'Engineer');
            INSERT INTO assets (id, name, description, created_by) VALUES (1, 'Test Asset', 'Test Description', 1);
            "#,
        ).unwrap();
        
        let repo = SqliteVaultRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_vault_creation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);

        let request = CreateVaultRequest {
            asset_id: 1,
            name: "PLC Identity Vault".to_string(),
            description: "Main identity vault for PLC authentication".to_string(),
            created_by: 1,
        };

        let vault = repo.create_vault(request).unwrap();
        assert_eq!(vault.name, "PLC Identity Vault");
        assert_eq!(vault.asset_id, 1);
        assert_eq!(vault.created_by, 1);
        assert!(!vault.created_at.is_empty());

        // Verify version history was created
        let history = repo.get_vault_history(vault.id).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].change_type, ChangeType::VaultCreated);
    }

    #[test]
    fn test_add_secret() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);

        // Create a vault first
        let vault_request = CreateVaultRequest {
            asset_id: 1,
            name: "Test Vault".to_string(),
            description: "Test Description".to_string(),
            created_by: 1,
        };
        let vault = repo.create_vault(vault_request).unwrap();

        // Add a small delay to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Add a secret
        let secret_request = AddSecretRequest {
            vault_id: vault.id,
            secret_type: SecretType::Password,
            label: "Admin Password".to_string(),
            value: "super_secret_password".to_string(),
            author_id: 1,
        };

        let secret = repo.add_secret(secret_request).unwrap();
        assert_eq!(secret.vault_id, vault.id);
        assert_eq!(secret.secret_type, SecretType::Password);
        assert_eq!(secret.label, "Admin Password");
        assert!(!secret.encrypted_value.is_empty());

        // Verify version history was created
        let history = repo.get_vault_history(vault.id).unwrap();
        assert_eq!(history.len(), 2); // Vault creation + secret addition
        
        // Verify both events are recorded (flexible ordering due to timestamp precision)
        let has_vault_created = history.iter().any(|h| h.change_type == ChangeType::VaultCreated);
        let has_secret_added = history.iter().any(|h| h.change_type == ChangeType::SecretAdded);
        assert!(has_vault_created, "History should contain VaultCreated");
        assert!(has_secret_added, "History should contain SecretAdded");
    }

    #[test]
    fn test_get_vault_by_asset_id() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);

        // Create a vault with secrets
        let vault_request = CreateVaultRequest {
            asset_id: 1,
            name: "Test Vault".to_string(),
            description: "Test Description".to_string(),
            created_by: 1,
        };
        let vault = repo.create_vault(vault_request).unwrap();

        // Add multiple secrets
        let secret1_request = AddSecretRequest {
            vault_id: vault.id,
            secret_type: SecretType::Password,
            label: "Admin Password".to_string(),
            value: "password123".to_string(),
            author_id: 1,
        };
        repo.add_secret(secret1_request).unwrap();

        let secret2_request = AddSecretRequest {
            vault_id: vault.id,
            secret_type: SecretType::IpAddress,
            label: "PLC IP".to_string(),
            value: "192.168.1.100".to_string(),
            author_id: 1,
        };
        repo.add_secret(secret2_request).unwrap();

        // Get vault info
        let vault_info = repo.get_vault_by_asset_id(1).unwrap().unwrap();
        assert_eq!(vault_info.vault.name, "Test Vault");
        assert_eq!(vault_info.secret_count, 2);
        assert_eq!(vault_info.secrets.len(), 2);
    }

    #[test]
    fn test_secret_type_conversion() {
        assert_eq!(SecretType::from_str("password").unwrap(), SecretType::Password);
        assert_eq!(SecretType::from_str("ip_address").unwrap(), SecretType::IpAddress);
        assert_eq!(SecretType::from_str("vpn_key").unwrap(), SecretType::VpnKey);
        assert_eq!(SecretType::from_str("license_file").unwrap(), SecretType::LicenseFile);
        assert!(SecretType::from_str("invalid").is_err());

        assert_eq!(SecretType::Password.to_string(), "password");
        assert_eq!(SecretType::IpAddress.to_string(), "ip_address");
        assert_eq!(SecretType::VpnKey.to_string(), "vpn_key");
        assert_eq!(SecretType::LicenseFile.to_string(), "license_file");
    }

    #[test]
    fn test_vault_validation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteVaultRepository::new(&conn);

        // Test empty name
        let request = CreateVaultRequest {
            asset_id: 1,
            name: "".to_string(),
            description: "Test".to_string(),
            created_by: 1,
        };
        assert!(repo.create_vault(request).is_err());

        // Test short name
        let request = CreateVaultRequest {
            asset_id: 1,
            name: "A".to_string(),
            description: "Test".to_string(),
            created_by: 1,
        };
        assert!(repo.create_vault(request).is_err());

        // Test long name
        let request = CreateVaultRequest {
            asset_id: 1,
            name: "A".repeat(101),
            description: "Test".to_string(),
            created_by: 1,
        };
        assert!(repo.create_vault(request).is_err());
    }
}