use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use tracing::info;

use crate::encryption::{DataKey, KdfParams, WrappedKey, generate_recovery_code, normalize_recovery_code};

/// Status values of a `data_keys` row. Only one row is `active` at a time;
/// `retiring` and `retired` are used by rotation (design chunk 5).
pub const DATA_KEY_STATUS_ACTIVE: &str = "active";

// The install-wide data key encrypts configuration files, firmware, and vault
// secrets. It is never stored in plaintext: each user holds a copy wrapped
// under their login password, and one more copy is wrapped under the recovery
// code shown at setup. The unwrapped key only lives in memory, in a session.
pub trait KeyStoreRepository {
    fn has_data_key(&self) -> Result<bool>;
    /// Generates the data key, wraps it for the first user and under a new
    /// recovery code, and returns the key with the code to show once.
    fn create_data_key(&self, user_id: i64, password: &str) -> Result<(DataKey, String)>;
    /// Returns `None` when the user has no wrapped copy of the key.
    fn unlock_for_user(&self, user_id: i64, password: &str) -> Result<Option<DataKey>>;
    /// Wraps the key for a user, replacing any copy they already hold.
    fn grant_user(&self, user_id: i64, password: &str, key: &DataKey) -> Result<()>;
    /// Deletes the user's wrapped copy of the key, so their password opens nothing.
    fn revoke_user(&self, user_id: i64) -> Result<()>;
    fn unlock_with_recovery_code(&self, recovery_code: &str) -> Result<DataKey>;
    /// Wraps the key under a new recovery code and returns the code.
    fn replace_recovery_code(&self, key: &DataKey) -> Result<String>;
}

pub struct SqliteKeyStoreRepository<'a> {
    conn: &'a Connection,
    params: KdfParams,
}

impl<'a> SqliteKeyStoreRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn, params: KdfParams::default() }
    }

    #[cfg(test)]
    pub fn with_params(conn: &'a Connection, params: KdfParams) -> Self {
        Self { conn, params }
    }

    pub fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS data_key_wraps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL CHECK(kind IN ('user', 'recovery')),
                user_id INTEGER UNIQUE,
                salt BLOB NOT NULL,
                wrapped_key BLOB NOT NULL,
                kdf_memory_kib INTEGER NOT NULL,
                kdf_iterations INTEGER NOT NULL,
                kdf_parallelism INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                key_version INTEGER NOT NULL DEFAULT 1,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                CHECK((kind = 'user') = (user_id IS NOT NULL))
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_data_key_wraps_recovery
                ON data_key_wraps(kind) WHERE kind = 'recovery';

            -- One row per data-key version. The key itself is never stored
            -- here in plaintext; `wrapped_by_active` holds a retiring key
            -- encrypted under the active one while a rotation runs.
            CREATE TABLE IF NOT EXISTS data_keys (
                version INTEGER PRIMARY KEY,
                status TEXT NOT NULL CHECK(status IN ('active', 'retiring', 'retired')),
                wrapped_by_active BLOB,
                created_by INTEGER REFERENCES users(id),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                retired_at DATETIME
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_data_keys_active
                ON data_keys(status) WHERE status = 'active';
            "#,
        )?;
        Ok(())
    }

    /// Runs `f` in a transaction, or inside the caller's if one is open.
    fn in_transaction<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        if !self.conn.is_autocommit() {
            return f();
        }
        let tx = self.conn.unchecked_transaction()?;
        let result = f()?;
        tx.commit()?;
        Ok(result)
    }

    fn store_wrap(&self, kind: &str, user_id: Option<i64>, wrapped: &WrappedKey, key_version: u32) -> Result<()> {
        // Delete then insert: SQLite UNIQUE constraints treat NULL user_ids as
        // distinct, so an upsert can't target the single recovery row.
        self.in_transaction(|| {
            match user_id {
                Some(id) => self.conn.execute("DELETE FROM data_key_wraps WHERE user_id = ?1", [id])?,
                None => self.conn.execute("DELETE FROM data_key_wraps WHERE kind = ?1", [kind])?,
            };
            self.conn.execute(
                "INSERT INTO data_key_wraps (kind, user_id, salt, wrapped_key, kdf_memory_kib, kdf_iterations, kdf_parallelism, key_version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    kind,
                    user_id,
                    wrapped.salt,
                    wrapped.ciphertext,
                    wrapped.params.memory_kib,
                    wrapped.params.iterations,
                    wrapped.params.parallelism,
                    key_version,
                ],
            )?;
            Ok(())
        })
    }

    /// Loads one wrap and the data-key version it holds.
    fn load_wrap(&self, condition: &str, param: &dyn rusqlite::ToSql) -> Result<Option<(WrappedKey, u32)>> {
        let query = format!(
            "SELECT salt, wrapped_key, kdf_memory_kib, kdf_iterations, kdf_parallelism, key_version
             FROM data_key_wraps WHERE {}",
            condition
        );
        let wrapped = self.conn.query_row(&query, [param], |row| {
            Ok((
                WrappedKey {
                    salt: row.get(0)?,
                    ciphertext: row.get(1)?,
                    params: KdfParams {
                        memory_kib: row.get(2)?,
                        iterations: row.get(3)?,
                        parallelism: row.get(4)?,
                    },
                },
                row.get(5)?,
            ))
        }).optional()?;
        Ok(wrapped)
    }

    /// Opens a wrap and labels the key with the version stored in its row.
    fn open_wrap(wrapped: &WrappedKey, key_version: u32, secret: &str) -> Result<DataKey> {
        Ok(wrapped.open(secret)?.with_version(key_version))
    }
}

impl<'a> KeyStoreRepository for SqliteKeyStoreRepository<'a> {
    fn has_data_key(&self) -> Result<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM data_key_wraps) OR EXISTS(SELECT 1 FROM data_keys)",
            [],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    fn create_data_key(&self, user_id: i64, password: &str) -> Result<(DataKey, String)> {
        if self.has_data_key()? {
            return Err(anyhow::anyhow!("A data key already exists"));
        }

        let key = DataKey::generate();
        let recovery_code = self.in_transaction(|| {
            self.conn.execute(
                "INSERT INTO data_keys (version, status, created_by) VALUES (?1, ?2, ?3)",
                params![key.version(), DATA_KEY_STATUS_ACTIVE, user_id],
            )?;
            self.grant_user(user_id, password, &key)?;
            self.replace_recovery_code(&key)
        })?;

        info!("Created data key for user ID {}", user_id);
        Ok((key, recovery_code))
    }

    fn unlock_for_user(&self, user_id: i64, password: &str) -> Result<Option<DataKey>> {
        match self.load_wrap("user_id = ?1", &user_id)? {
            Some((wrapped, key_version)) => Ok(Some(Self::open_wrap(&wrapped, key_version, password)?)),
            None => Ok(None),
        }
    }

    fn grant_user(&self, user_id: i64, password: &str, key: &DataKey) -> Result<()> {
        let wrapped = WrappedKey::seal(key, password, self.params)?;
        self.store_wrap("user", Some(user_id), &wrapped, key.version())
    }

    fn revoke_user(&self, user_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM data_key_wraps WHERE user_id = ?1", [user_id])?;
        Ok(())
    }

    fn unlock_with_recovery_code(&self, recovery_code: &str) -> Result<DataKey> {
        let (wrapped, key_version) = self.load_wrap("kind = ?1", &"recovery")?
            .ok_or_else(|| anyhow::anyhow!("No recovery key has been set up"))?;
        Self::open_wrap(&wrapped, key_version, &normalize_recovery_code(recovery_code))
            .map_err(|_| anyhow::anyhow!("Invalid recovery key"))
    }

    fn replace_recovery_code(&self, key: &DataKey) -> Result<String> {
        let recovery_code = generate_recovery_code();
        let wrapped = WrappedKey::seal(key, &normalize_recovery_code(&recovery_code), self.params)?;
        self.store_wrap("recovery", None, &wrapped, key.version())?;
        Ok(recovery_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::users::{CreateUserRequest, SqliteUserRepository, UserRepository, UserRole};
    use crate::encryption::FileEncryption;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        SqliteUserRepository::new(&conn).initialize_schema().unwrap();
        SqliteKeyStoreRepository::new(&conn).initialize_schema().unwrap();
        conn
    }

    fn create_user(conn: &Connection, username: &str) -> i64 {
        SqliteUserRepository::new(conn).create_user(CreateUserRequest {
            username: username.to_string(),
            password: "password123!".to_string(),
            role: UserRole::Administrator,
        }).unwrap().id
    }

    fn store(conn: &Connection) -> SqliteKeyStoreRepository<'_> {
        SqliteKeyStoreRepository::with_params(conn, KdfParams::fast_for_tests())
    }

    fn same_key(a: &DataKey, b: &DataKey) -> bool {
        let ciphertext = FileEncryption::new(a).encrypt(b"probe").unwrap();
        FileEncryption::new(b).decrypt(&ciphertext).is_ok()
    }

    #[test]
    fn test_create_and_unlock_data_key() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        assert!(!store.has_data_key().unwrap());
        let (key, _code) = store.create_data_key(admin, "admin-password").unwrap();
        assert!(store.has_data_key().unwrap());

        let unlocked = store.unlock_for_user(admin, "admin-password").unwrap().unwrap();
        assert!(same_key(&key, &unlocked));
        assert!(store.unlock_for_user(admin, "wrong-password").is_err());
    }

    #[test]
    fn test_create_data_key_only_once() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        store.create_data_key(admin, "admin-password").unwrap();
        assert!(store.create_data_key(admin, "admin-password").is_err());
    }

    #[test]
    fn test_user_without_wrap_unlocks_nothing() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let engineer = create_user(&conn, "engineer");
        let store = store(&conn);

        store.create_data_key(admin, "admin-password").unwrap();
        assert!(store.unlock_for_user(engineer, "anything").unwrap().is_none());
    }

    #[test]
    fn test_granted_user_gets_same_key() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let engineer = create_user(&conn, "engineer");
        let store = store(&conn);

        let (key, _code) = store.create_data_key(admin, "admin-password").unwrap();
        store.grant_user(engineer, "engineer-password", &key).unwrap();

        let unlocked = store.unlock_for_user(engineer, "engineer-password").unwrap().unwrap();
        assert!(same_key(&key, &unlocked));
    }

    #[test]
    fn test_revoked_user_unlocks_nothing_and_can_be_regranted() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let engineer = create_user(&conn, "engineer");
        let store = store(&conn);

        let (key, _code) = store.create_data_key(admin, "admin-password").unwrap();
        store.grant_user(engineer, "engineer-password", &key).unwrap();
        store.revoke_user(engineer).unwrap();

        assert!(store.unlock_for_user(engineer, "engineer-password").unwrap().is_none());
        // Other users and the recovery code are untouched
        assert!(store.unlock_for_user(admin, "admin-password").unwrap().is_some());

        store.grant_user(engineer, "new-password", &key).unwrap();
        let unlocked = store.unlock_for_user(engineer, "new-password").unwrap().unwrap();
        assert!(same_key(&key, &unlocked));
    }

    #[test]
    fn test_regrant_replaces_old_password() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        let (key, _code) = store.create_data_key(admin, "old-password").unwrap();
        store.grant_user(admin, "new-password", &key).unwrap();

        assert!(store.unlock_for_user(admin, "old-password").is_err());
        assert!(store.unlock_for_user(admin, "new-password").unwrap().is_some());
    }

    #[test]
    fn test_recovery_code_unlocks_key() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        let (key, code) = store.create_data_key(admin, "admin-password").unwrap();

        let recovered = store.unlock_with_recovery_code(&code).unwrap();
        assert!(same_key(&key, &recovered));

        // Typed without dashes and in lowercase still works
        let typed = code.replace('-', "").to_lowercase();
        assert!(store.unlock_with_recovery_code(&typed).is_ok());

        assert!(store.unlock_with_recovery_code("0000-0000").is_err());
    }

    #[test]
    fn test_replacing_recovery_code_revokes_old_one() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        let (key, old_code) = store.create_data_key(admin, "admin-password").unwrap();
        let new_code = store.replace_recovery_code(&key).unwrap();

        assert_ne!(old_code, new_code);
        assert!(store.unlock_with_recovery_code(&old_code).is_err());
        assert!(store.unlock_with_recovery_code(&new_code).is_ok());

        let recovery_rows: i64 = conn.query_row(
            "SELECT COUNT(*) FROM data_key_wraps WHERE kind = 'recovery'", [], |row| row.get(0),
        ).unwrap();
        assert_eq!(recovery_rows, 1);
    }

    fn wrap_version(conn: &Connection, condition: &str) -> u32 {
        conn.query_row(
            &format!("SELECT key_version FROM data_key_wraps WHERE {}", condition), [], |row| row.get(0),
        ).unwrap()
    }

    #[test]
    fn test_create_data_key_records_active_version() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        let (key, _code) = store.create_data_key(admin, "admin-password").unwrap();
        assert_eq!(key.version(), crate::encryption::INITIAL_KEY_VERSION);

        let (version, status, created_by): (u32, String, i64) = conn.query_row(
            "SELECT version, status, created_by FROM data_keys", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap();
        assert_eq!((version, status.as_str(), created_by), (key.version(), "active", admin));
        assert_eq!(wrap_version(&conn, "kind = 'recovery'"), key.version());
    }

    #[test]
    fn test_unwrapped_keys_take_version_from_row() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        let (_key, code) = store.create_data_key(admin, "admin-password").unwrap();
        // Pretend the wraps hold a later key version
        conn.execute("UPDATE data_key_wraps SET key_version = 4", []).unwrap();

        assert_eq!(store.unlock_for_user(admin, "admin-password").unwrap().unwrap().version(), 4);
        assert_eq!(store.unlock_with_recovery_code(&code).unwrap().version(), 4);
    }

    #[test]
    fn test_wraps_store_the_key_version() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let engineer = create_user(&conn, "engineer");
        let store = store(&conn);

        store.create_data_key(admin, "admin-password").unwrap();
        let later = DataKey::generate_with_version(3);
        store.grant_user(engineer, "engineer-password", &later).unwrap();
        store.replace_recovery_code(&later).unwrap();

        assert_eq!(wrap_version(&conn, &format!("user_id = {}", engineer)), 3);
        assert_eq!(wrap_version(&conn, "kind = 'recovery'"), 3);
        assert_eq!(store.unlock_for_user(engineer, "engineer-password").unwrap().unwrap().version(), 3);
    }

    #[test]
    fn test_key_creation_joins_callers_transaction() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        let tx = conn.unchecked_transaction().unwrap();
        store.create_data_key(admin, "admin-password").unwrap();
        drop(tx); // roll back

        assert!(!store.has_data_key().unwrap());
    }
}
