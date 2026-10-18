use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;
use tracing::info;
use crate::users::SqliteUserRepository;
use crate::keystore::SqliteKeyStoreRepository;
use crate::user_settings::SqliteUserSettingsRepository;
use crate::audit::{SqliteAuditRepository, AuditRepository};
use crate::assets::SqliteAssetRepository;
use crate::configurations::SqliteConfigurationRepository;
use crate::branches::SqliteBranchRepository;
use crate::firmware::SqliteFirmwareRepository;
use crate::firmware_analysis::{SqliteFirmwareAnalysisRepository, FirmwareAnalysisRepository};
use crate::vault::{SqliteVaultRepository, VaultRepository};
// Epic 5 imports
use crate::metadata::{SqliteMetadataRepository, SqliteMetadataSearchRepository};
use crate::bulk::{SqliteBulkImportRepository, operations::SqliteBulkOperationsRepository};

/// Tables whose rows hold data-key ciphertext (or, for `data_key_wraps`, a
/// wrapped data key) and so carry a `key_version` column.
pub const KEY_VERSIONED_TABLES: [&str; 5] = [
    "data_key_wraps",
    "configuration_versions",
    "vault_secrets",
    "standalone_credentials",
    "firmware_versions",
];

pub struct Database {
    conn: Connection,
}

// Type alias for compatibility with workflow module
pub type DatabaseManager = Database;
pub type DatabaseError = anyhow::Error;

impl Database {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        info!("Initializing database at: {:?}", db_path);
        
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;

        let db = Database { conn };
        db.initialize_schema()?;

        info!("Database initialized successfully");
        Ok(db)
    }

    /// Opens an existing connection as if it were the app database, running
    /// the schema initializers and migrations on it.
    #[cfg(test)]
    pub fn from_connection(conn: Connection) -> Result<Self> {
        let db = Database { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    fn initialize_schema(&self) -> Result<()> {
        info!("Setting up database schema");
        
        self.conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA secure_delete = ON;
            PRAGMA mmap_size = 268435456; -- 256MB
            
            CREATE TABLE IF NOT EXISTS app_metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            INSERT OR IGNORE INTO app_metadata (key, value) VALUES 
                ('db_version', '1.0'),
                ('created_at', datetime('now'));
            "#,
        )?;

        // Initialize user schema
        let user_repo = SqliteUserRepository::new(&self.conn);
        user_repo.initialize_schema()?;

        // Initialize the wrapped data key schema
        let key_store = SqliteKeyStoreRepository::new(&self.conn);
        key_store.initialize_schema()?;

        // Initialize user settings schema
        let user_settings_repo = SqliteUserSettingsRepository::new(&self.conn);
        user_settings_repo.initialize_schema()?;

        // Initialize audit schema
        let audit_repo = SqliteAuditRepository::new(&self.conn);
        audit_repo.initialize_schema()?;

        // Initialize asset schema
        let asset_repo = SqliteAssetRepository::new(&self.conn);
        asset_repo.initialize_schema()?;

        // Initialize configuration schema
        let config_repo = SqliteConfigurationRepository::new(&self.conn);
        config_repo.initialize_schema()?;

        // Initialize branches schema
        let branch_repo = SqliteBranchRepository::new(&self.conn);
        branch_repo.initialize_schema()?;

        // Initialize firmware schema
        let firmware_repo = SqliteFirmwareRepository::new(&self.conn);
        firmware_repo.initialize_schema()?;

        // Initialize firmware analysis schema
        let firmware_analysis_repo = SqliteFirmwareAnalysisRepository::new(&self.conn);
        firmware_analysis_repo.initialize_schema()?;

        // Initialize vault schema
        let vault_repo = SqliteVaultRepository::new(&self.conn);
        vault_repo.initialize_schema()?;

        // Epic 5 - Initialize metadata schema
        let metadata_repo = SqliteMetadataRepository::new(&self.conn);
        metadata_repo.initialize_schema()?;
        metadata_repo.populate_system_templates()?;
        metadata_repo.create_default_schema()?;

        // Epic 5 - Initialize metadata search schema
        let metadata_search_repo = SqliteMetadataSearchRepository::new(&self.conn);
        metadata_search_repo.initialize_search_schema()?;

        // Epic 5 - Initialize bulk import schema
        let bulk_repo = SqliteBulkImportRepository::new(&self.conn);
        bulk_repo.initialize_schema()?;

        // Epic 5 - Initialize bulk operations schema
        let bulk_ops_repo = SqliteBulkOperationsRepository::new(&self.conn);
        bulk_ops_repo.initialize_schema()?;

        // Run data migrations
        self.run_data_migrations()?;

        info!("Database schema initialized");
        Ok(())
    }

    pub fn health_check(&self) -> Result<bool> {
        let mut stmt = self.conn.prepare("SELECT 1")?;
        let result: i32 = stmt.query_row([], |row| row.get(0))?;
        Ok(result == 1)
    }

    pub fn get_metadata(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM app_metadata WHERE key = ?1")?;
        let result = stmt.query_row([key], |row| row.get(0));
        
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_metadata(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO app_metadata (key, value, updated_at) VALUES (?1, ?2, datetime('now'))",
            [key, value],
        )?;
        Ok(())
    }

    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }

    fn run_data_migrations(&self) -> Result<()> {
        info!("Running data migrations");
        
        // Each migration is an independent guarded block: never return early, or
        // migrations appended later would not run on installs that already applied
        // an earlier one.

        // Fix firmware records with invalid metadata
        let migration_key = "firmware_metadata_migration_20250125";
        if self.get_metadata(migration_key)?.is_none() {
            info!("Fixing firmware metadata issues");

            self.conn.execute_batch(r#"
                -- Update firmware records with NULL or invalid file_size
                UPDATE firmware_versions
                SET file_size = 0
                WHERE file_size IS NULL OR file_size < 0;

                -- Update firmware records with NULL or empty file_hash
                UPDATE firmware_versions
                SET file_hash = 'unknown'
                WHERE file_hash IS NULL OR file_hash = '';

                -- Update firmware records with invalid created_at timestamps
                UPDATE firmware_versions
                SET created_at = datetime('now')
                WHERE created_at IS NULL OR created_at = '' OR datetime(created_at) IS NULL;

                -- Ensure all firmware records have proper status
                UPDATE firmware_versions
                SET status = 'Draft'
                WHERE status IS NULL OR status = '' OR status NOT IN ('Draft', 'Approved', 'Golden', 'Archived');
            "#)?;

            self.set_metadata(migration_key, "applied")?;
            info!("Firmware metadata migration completed");
        }

        // Story 4.6 - Password rotation migration
        let rotation_migration_key = "password_rotation_migration_20250126";
        if let Ok(None) = self.get_metadata(rotation_migration_key) {
            info!("Applying password rotation migration");
            
            // Check if columns already exist before adding them
            let column_check: Result<i32, rusqlite::Error> = self.conn.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('vault_secrets') WHERE name = 'last_rotated'",
                [],
                |row| row.get(0),
            );
            
            if let Ok(0) = column_check {
                // Add rotation columns to existing vault_secrets table
                self.conn.execute_batch(r#"
                    -- Add rotation management columns to vault_secrets
                    ALTER TABLE vault_secrets ADD COLUMN last_rotated DATETIME;
                    ALTER TABLE vault_secrets ADD COLUMN rotation_interval_days INTEGER;
                    ALTER TABLE vault_secrets ADD COLUMN next_rotation_due DATETIME;
                    ALTER TABLE vault_secrets ADD COLUMN rotation_policy_id INTEGER;
                "#)?;
                
                info!("Added rotation columns to vault_secrets table");
            }
            
            // Mark migration as complete
            self.set_metadata(rotation_migration_key, "applied")?;
            info!("Password rotation migration completed");
        }
        
        // Epic 5 - Metadata archive columns migration
        let metadata_archive_migration_key = "metadata_archive_columns_20250206";
        if let Ok(None) = self.get_metadata(metadata_archive_migration_key) {
            info!("Applying metadata archive columns migration");
            
            // Check if is_archived column already exists
            let column_check: Result<i32, rusqlite::Error> = self.conn.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('asset_metadata_schemas') WHERE name = 'is_archived'",
                [],
                |row| row.get(0),
            );
            
            if let Ok(0) = column_check {
                // Add archive columns to asset_metadata_schemas table
                self.conn.execute_batch(r#"
                    -- Add archive management columns to asset_metadata_schemas
                    ALTER TABLE asset_metadata_schemas ADD COLUMN is_archived BOOLEAN DEFAULT 0;
                    ALTER TABLE asset_metadata_schemas ADD COLUMN archived_at DATETIME;
                    ALTER TABLE asset_metadata_schemas ADD COLUMN archive_reason TEXT;
                    
                    -- Create index for the new is_archived column
                    CREATE INDEX IF NOT EXISTS idx_metadata_schemas_archived ON asset_metadata_schemas(is_archived);
                "#)?;
                
                info!("Added archive columns and index to asset_metadata_schemas table");
            } else {
                info!("Archive columns already exist in asset_metadata_schemas table");
            }
            
            // Mark migration as complete
            self.set_metadata(metadata_archive_migration_key, "applied")?;
            info!("Metadata archive columns migration completed");
        }
        
        // Password rotation history must never hold data-key ciphertext. An earlier
        // bcrypt-failure fallback stored the raw ciphertext there.
        let redact_key = "redact_rotation_history_20261019";
        if self.get_metadata(redact_key)?.is_none() {
            self.conn.execute(
                "UPDATE password_rotation_history SET old_password_hash = 'redacted' WHERE old_password_hash NOT LIKE '$2%'",
                [],
            )?;
            self.set_metadata(redact_key, "applied")?;
            info!("Redacted non-bcrypt rotation history entries");
        }

        // A deactivated user must not keep a wrapped copy of the data key.
        let revoke_key = "revoke_inactive_user_wraps_20261019";
        if self.get_metadata(revoke_key)?.is_none() {
            self.conn.execute(
                "DELETE FROM data_key_wraps WHERE user_id IN (SELECT id FROM users WHERE is_active = 0)",
                [],
            )?;
            self.set_metadata(revoke_key, "applied")?;
            info!("Revoked data key wraps of inactive users");
        }

        // Every ciphertext and key wrap records the data-key version it was
        // written under, so a rotation can find what still uses an old key.
        let key_version_key = "key_version_columns_20261019";
        if self.get_metadata(key_version_key)?.is_none() {
            let tx = self.conn.unchecked_transaction()?;
            for table in KEY_VERSIONED_TABLES {
                let has_column: bool = self.conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = 'key_version')",
                    [table],
                    |row| row.get(0),
                )?;
                if !has_column {
                    // Table names come from the constant list above, never from input
                    self.conn.execute_batch(&format!(
                        "ALTER TABLE {} ADD COLUMN key_version INTEGER NOT NULL DEFAULT 1;",
                        table
                    ))?;
                    info!("Added key_version column to {}", table);
                }
            }
            // An install that already has a data key has been using version 1
            self.conn.execute(
                "INSERT INTO data_keys (version, status)
                 SELECT 1, 'active'
                 WHERE EXISTS (SELECT 1 FROM data_key_wraps)
                   AND NOT EXISTS (SELECT 1 FROM data_keys)",
                [],
            )?;
            self.set_metadata(key_version_key, "applied")?;
            tx.commit()?;
            info!("Key version columns migration completed");
        }

        Ok(())
    }

    /// Truncates the WAL so deleted pages (for example revoked key wraps) don't
    /// linger in the -wal file.
    pub fn checkpoint_wal(&self) -> Result<()> {
        self.conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_database_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();
        
        let db = Database::new(db_path).unwrap();
        assert!(db.health_check().unwrap());
    }

    #[test]
    fn test_metadata_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();
        
        let db = Database::new(db_path).unwrap();
        
        // Test setting and getting metadata
        db.set_metadata("test_key", "test_value").unwrap();
        let value = db.get_metadata("test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));
        
        // Test getting non-existent key
        let empty_value = db.get_metadata("non_existent").unwrap();
        assert_eq!(empty_value, None);
    }

    #[test]
    fn test_later_migration_runs_when_first_already_applied() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();

        let db = Database::new(db_path).unwrap();
        assert!(db.get_metadata("firmware_metadata_migration_20250125").unwrap().is_some());

        // Simulate an existing install: first migration applied, newer ones not yet.
        db.conn.execute(
            "DELETE FROM app_metadata WHERE key IN ('redact_rotation_history_20261019', 'revoke_inactive_user_wraps_20261019')",
            [],
        ).unwrap();
        db.conn.execute(
            "INSERT INTO users (username, password_hash, role, is_active) VALUES ('gone', 'x', 'Engineer', 0)",
            [],
        ).unwrap();
        let uid = db.conn.last_insert_rowid();
        db.conn.execute(
            "INSERT INTO data_key_wraps (kind, user_id, salt, wrapped_key, kdf_memory_kib, kdf_iterations, kdf_parallelism)
             VALUES ('user', ?1, x'00', x'00', 8, 1, 1)",
            [uid],
        ).unwrap();

        db.run_data_migrations().unwrap();

        assert!(db.get_metadata("revoke_inactive_user_wraps_20261019").unwrap().is_some());
        let wraps: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM data_key_wraps WHERE user_id = ?1", [uid], |r| r.get(0)).unwrap();
        assert_eq!(wraps, 0);
    }

    /// The five key-versioned tables as they were created before key versioning.
    const PRE_KEY_VERSION_SCHEMA: &str = r#"
        -- Seed rows reference users, assets and vaults this fixture does not create
        PRAGMA foreign_keys = OFF;

        CREATE TABLE data_key_wraps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL CHECK(kind IN ('user', 'recovery')),
            user_id INTEGER UNIQUE,
            salt BLOB NOT NULL,
            wrapped_key BLOB NOT NULL,
            kdf_memory_kib INTEGER NOT NULL,
            kdf_iterations INTEGER NOT NULL,
            kdf_parallelism INTEGER NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            CHECK((kind = 'user') = (user_id IS NOT NULL))
        );
        CREATE UNIQUE INDEX idx_data_key_wraps_recovery ON data_key_wraps(kind) WHERE kind = 'recovery';

        CREATE TABLE configuration_versions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            asset_id INTEGER NOT NULL,
            version_number TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_content BLOB NOT NULL,
            file_size INTEGER NOT NULL,
            content_hash TEXT NOT NULL,
            author INTEGER NOT NULL,
            notes TEXT,
            status TEXT DEFAULT 'Draft' CHECK(status IN ('Draft', 'Silver', 'Approved', 'Golden', 'Archived')),
            status_changed_by INTEGER REFERENCES users(id),
            status_changed_at DATETIME,
            firmware_version_id INTEGER REFERENCES firmware_versions(id) ON DELETE SET NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
            FOREIGN KEY (author) REFERENCES users(id),
            UNIQUE(asset_id, version_number)
        );

        CREATE TABLE firmware_versions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            asset_id INTEGER NOT NULL,
            author_id INTEGER NOT NULL,
            vendor TEXT,
            model TEXT,
            version TEXT NOT NULL,
            notes TEXT,
            status TEXT NOT NULL CHECK(status IN ('Draft', 'Approved', 'Golden', 'Archived')),
            file_path TEXT NOT NULL,
            file_hash TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            status_changed_at DATETIME,
            status_changed_by INTEGER,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
            FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE RESTRICT,
            FOREIGN KEY (status_changed_by) REFERENCES users(id) ON DELETE RESTRICT
        );

        CREATE TABLE vault_secrets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            vault_id INTEGER NOT NULL,
            secret_type TEXT NOT NULL CHECK(secret_type IN ('password', 'ip_address', 'vpn_key', 'license_file')),
            label TEXT NOT NULL,
            encrypted_value TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            strength_score INTEGER,
            last_changed DATETIME,
            generation_method TEXT,
            policy_version INTEGER,
            last_rotated DATETIME,
            rotation_interval_days INTEGER,
            next_rotation_due DATETIME,
            rotation_policy_id INTEGER,
            FOREIGN KEY (vault_id) REFERENCES vault_entries(id) ON DELETE CASCADE,
            UNIQUE(vault_id, label)
        );

        CREATE TABLE standalone_credentials (
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

        INSERT INTO data_key_wraps (kind, user_id, salt, wrapped_key, kdf_memory_kib, kdf_iterations, kdf_parallelism)
            VALUES ('user', 1, x'00', x'00', 8, 1, 1), ('recovery', NULL, x'00', x'00', 8, 1, 1);
        INSERT INTO configuration_versions (asset_id, version_number, file_name, file_content, file_size, content_hash, author)
            VALUES (1, 'v1', 'a.cfg', x'00', 1, 'h', 1);
        INSERT INTO firmware_versions (asset_id, author_id, version, status, file_path, file_hash, file_size)
            VALUES (1, 1, '1.0', 'Draft', '1/1.enc', 'h', 1);
        INSERT INTO vault_secrets (vault_id, secret_type, label, encrypted_value) VALUES (1, 'password', 'pw', 'x');
        INSERT INTO standalone_credentials (name, credential_type, encrypted_data, created_by)
            VALUES ('c', 'password', 'x', 1);
    "#;

    fn has_key_version_column(conn: &Connection, table: &str) -> bool {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = 'key_version')",
            [table],
            |r| r.get(0),
        ).unwrap()
    }

    #[test]
    fn test_key_version_migration_upgrades_pre_versioning_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(PRE_KEY_VERSION_SCHEMA).unwrap();
        for table in KEY_VERSIONED_TABLES {
            assert!(!has_key_version_column(&conn, table));
        }

        // Opening the old database runs every schema initializer, then the migrations
        let db = Database::from_connection(conn).unwrap();

        assert_eq!(db.get_metadata("key_version_columns_20261019").unwrap().as_deref(), Some("applied"));
        for table in KEY_VERSIONED_TABLES {
            assert!(has_key_version_column(&db.conn, table), "{} has no key_version", table);
            let unversioned: i64 = db.conn.query_row(
                &format!("SELECT COUNT(*) FROM {} WHERE key_version IS NOT 1", table), [], |r| r.get(0),
            ).unwrap();
            assert_eq!(unversioned, 0, "{} has rows not at version 1", table);
        }
        let data_keys: Vec<(u32, String)> = db.conn
            .prepare("SELECT version, status FROM data_keys").unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
            .collect::<Result<_, _>>().unwrap();
        assert_eq!(data_keys, vec![(1, "active".to_string())]);

        // Running again is a no-op, even with the marker gone
        db.conn.execute("DELETE FROM app_metadata WHERE key = 'key_version_columns_20261019'", []).unwrap();
        db.run_data_migrations().unwrap();
        let rows: i64 = db.conn.query_row("SELECT COUNT(*) FROM data_keys", [], |r| r.get(0)).unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn test_fresh_install_has_key_version_columns_and_no_data_key_yet() {
        let temp_file = NamedTempFile::new().unwrap();
        let db = Database::new(temp_file.path().to_path_buf()).unwrap();

        for table in KEY_VERSIONED_TABLES {
            assert!(has_key_version_column(&db.conn, table));
        }
        assert!(db.get_metadata("key_version_columns_20261019").unwrap().is_some());
        let rows: i64 = db.conn.query_row("SELECT COUNT(*) FROM data_keys", [], |r| r.get(0)).unwrap();
        assert_eq!(rows, 0);
    }

    #[test]
    fn test_ciphertext_writers_record_key_version() {
        use crate::configurations::{ConfigurationRepository, CreateConfigurationRequest};
        use crate::encryption::DataKey;
        use crate::firmware::{CreateFirmwareRequest, FirmwareRepository};
        use crate::vault::rotation::{PasswordRotationRequest, PasswordRotationService};
        use crate::vault::{
            AddSecretRequest, CreateStandaloneCredentialRequest, CreateVaultRequest, SecretType,
            UpdateCredentialPasswordRequest, UpdateStandaloneCredentialRequest, UpdateVaultSecretRequest,
        };

        let temp_file = NamedTempFile::new().unwrap();
        let db = Database::new(temp_file.path().to_path_buf()).unwrap();
        let conn = db.get_connection();
        conn.execute_batch(
            "INSERT INTO users (id, username, password_hash, role) VALUES (1, 'admin', 'x', 'Administrator');
             INSERT INTO assets (id, name, created_by) VALUES (1, 'plc', 1);",
        ).unwrap();
        let version_of = |table: &str, id: i64| -> u32 {
            conn.query_row(&format!("SELECT key_version FROM {} WHERE id = ?1", table), [id], |r| r.get(0)).unwrap()
        };
        let set_version = |table: &str, id: i64, v: u32| {
            conn.execute(&format!("UPDATE {} SET key_version = ?1 WHERE id = ?2", table), [v as i64, id]).unwrap();
        };

        let key = DataKey::generate_with_version(3);

        // Configurations: stored under the key; duplicates keep the source's version
        let configs = SqliteConfigurationRepository::new(conn);
        let config = configs.store_configuration(CreateConfigurationRequest {
            asset_id: 1, file_name: "a.cfg".into(), file_content: b"config".to_vec(), author: 1, notes: String::new(),
        }, &key).unwrap();
        assert_eq!(version_of("configuration_versions", config.id), 3);
        set_version("configuration_versions", config.id, 2);
        let copy = configs.duplicate_configuration_version(config.id, 1, "copy").unwrap();
        assert_eq!(version_of("configuration_versions", copy.id), 2);

        // Firmware
        let firmware = SqliteFirmwareRepository::new(conn).create_firmware(CreateFirmwareRequest {
            asset_id: 1, vendor: None, model: None, version: "1.0".into(), notes: None,
        }, 1, "1/1.enc".into(), "h".into(), 1, key.version()).unwrap();
        assert_eq!(version_of("firmware_versions", firmware.id), 3);

        // Vault secrets, through every path that writes encrypted_value
        let vaults = SqliteVaultRepository::new(conn);
        let vault = vaults.create_vault(CreateVaultRequest {
            asset_id: 1, name: "plc vault".into(), description: String::new(), created_by: 1,
        }).unwrap();
        let secret = vaults.add_secret(AddSecretRequest {
            vault_id: vault.id, secret_type: SecretType::Password, label: "pw".into(),
            value: "Initial-Pass-1".into(), author_id: 1,
        }, &key).unwrap();
        assert_eq!(version_of("vault_secrets", secret.id), 3);
        assert_eq!(secret.key_version, 3);
        set_version("vault_secrets", secret.id, 2);
        assert_eq!(vaults.get_secret_by_id(secret.id).unwrap().unwrap().key_version, 2);
        assert_eq!(vaults.get_vault_secrets(vault.id).unwrap()[0].key_version, 2);

        let update =|label: Option<&str>, value: Option<&str>| UpdateVaultSecretRequest {
            secret_id: secret.id, label: label.map(String::from), value: value.map(String::from), author_id: 1,
        };
        for (label, value) in [(Some("pw2"), Some("Value-Two-2")), (None, Some("Value-Three-3"))] {
            set_version("vault_secrets", secret.id, 1);
            vaults.update_vault_secret(update(label, value), &key).unwrap();
            assert_eq!(version_of("vault_secrets", secret.id), 3);
        }
        // A label-only change leaves the ciphertext, and so its version, alone
        set_version("vault_secrets", secret.id, 1);
        vaults.update_vault_secret(update(Some("pw3"), None), &key).unwrap();
        assert_eq!(version_of("vault_secrets", secret.id), 1);

        vaults.update_password(UpdateCredentialPasswordRequest {
            secret_id: secret.id, new_password: "Value-Four-4".into(), author_id: 1,
        }, "$2b$04$hash", 80, &key).unwrap();
        assert_eq!(version_of("vault_secrets", secret.id), 3);

        set_version("vault_secrets", secret.id, 1);
        let stored = vaults.get_secret_by_id(secret.id).unwrap().unwrap();
        vaults.update_secret(&stored, 1, 5).unwrap();
        assert_eq!(version_of("vault_secrets", secret.id), 5);

        set_version("vault_secrets", secret.id, 1);
        PasswordRotationService::new(conn, Box::new(SqliteVaultRepository::new(conn)), Box::new(SqliteAuditRepository::new(conn)))
            .rotate_password(PasswordRotationRequest {
                secret_id: secret.id, new_password: "Rotated-Pass-5!".into(),
                rotation_reason: "test".into(), author_id: 1, batch_id: None,
            }, &key).unwrap();
        assert_eq!(version_of("vault_secrets", secret.id), 3);

        let mut info = vaults.get_vault_by_asset_id(1).unwrap().unwrap();
        info.vault.name = "imported".into();
        let imported = vaults.import_vault(&info, 1, 4).unwrap();
        let imported_version: u32 = conn.query_row(
            "SELECT key_version FROM vault_secrets WHERE vault_id = ?1", [imported.id], |r| r.get(0),
        ).unwrap();
        assert_eq!(imported_version, 4);

        // Standalone credentials
        let credential = vaults.create_standalone_credential(CreateStandaloneCredentialRequest {
            name: "plc login".into(), description: String::new(), credential_type: SecretType::Password,
            category_id: None, value: "Cred-Value-1".into(), tags: None, created_by: 1,
        }, &key).unwrap();
        assert_eq!(version_of("standalone_credentials", credential.id), 3);

        set_version("standalone_credentials", credential.id, 1);
        vaults.update_standalone_credential(UpdateStandaloneCredentialRequest {
            id: credential.id, name: Some("plc login 2".into()), description: None, category_id: None,
            value: Some("Cred-Value-2".into()), author_id: 1,
        }, &key).unwrap();
        assert_eq!(version_of("standalone_credentials", credential.id), 3);

        set_version("standalone_credentials", credential.id, 1);
        vaults.update_standalone_credential(UpdateStandaloneCredentialRequest {
            id: credential.id, name: Some("plc login 3".into()), description: None, category_id: None,
            value: None, author_id: 1,
        }, &key).unwrap();
        assert_eq!(version_of("standalone_credentials", credential.id), 1);
    }

    #[test]
    fn test_secure_delete_and_checkpoint() {
        let temp_file = NamedTempFile::new().unwrap();
        let db = Database::new(temp_file.path().to_path_buf()).unwrap();
        let secure: i64 = db.conn.query_row("PRAGMA secure_delete", [], |r| r.get(0)).unwrap();
        assert_eq!(secure, 1);
        db.checkpoint_wal().unwrap();
    }
}
