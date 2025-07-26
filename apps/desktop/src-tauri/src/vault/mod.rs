use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::encryption::FileEncryption;
use tracing::{info, debug};

pub mod password_services;
pub use password_services::{PasswordGenerator, PasswordStrengthAnalyzer, PasswordReuseChecker};

#[cfg(test)]
mod password_performance_tests;

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