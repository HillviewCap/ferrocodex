use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use tracing::info;

use crate::encryption::{DataKey, KdfParams, WrappedKey, generate_recovery_code, normalize_recovery_code};

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
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                CHECK((kind = 'user') = (user_id IS NOT NULL))
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_data_key_wraps_recovery
                ON data_key_wraps(kind) WHERE kind = 'recovery';
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

    fn store_wrap(&self, kind: &str, user_id: Option<i64>, wrapped: &WrappedKey) -> Result<()> {
        // Delete then insert: SQLite UNIQUE constraints treat NULL user_ids as
        // distinct, so an upsert can't target the single recovery row.
        self.in_transaction(|| {
            match user_id {
                Some(id) => self.conn.execute("DELETE FROM data_key_wraps WHERE user_id = ?1", [id])?,
                None => self.conn.execute("DELETE FROM data_key_wraps WHERE kind = ?1", [kind])?,
            };
            self.conn.execute(
                "INSERT INTO data_key_wraps (kind, user_id, salt, wrapped_key, kdf_memory_kib, kdf_iterations, kdf_parallelism)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    kind,
                    user_id,
                    wrapped.salt,
                    wrapped.ciphertext,
                    wrapped.params.memory_kib,
                    wrapped.params.iterations,
                    wrapped.params.parallelism,
                ],
            )?;
            Ok(())
        })
    }

    fn load_wrap(&self, condition: &str, param: &dyn rusqlite::ToSql) -> Result<Option<WrappedKey>> {
        let query = format!(
            "SELECT salt, wrapped_key, kdf_memory_kib, kdf_iterations, kdf_parallelism
             FROM data_key_wraps WHERE {}",
            condition
        );
        let wrapped = self.conn.query_row(&query, [param], |row| {
            Ok(WrappedKey {
                salt: row.get(0)?,
                ciphertext: row.get(1)?,
                params: KdfParams {
                    memory_kib: row.get(2)?,
                    iterations: row.get(3)?,
                    parallelism: row.get(4)?,
                },
            })
        }).optional()?;
        Ok(wrapped)
    }
}

impl<'a> KeyStoreRepository for SqliteKeyStoreRepository<'a> {
    fn has_data_key(&self) -> Result<bool> {
        let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM data_key_wraps", [], |row| row.get(0))?;
        Ok(count > 0)
    }

    fn create_data_key(&self, user_id: i64, password: &str) -> Result<(DataKey, String)> {
        if self.has_data_key()? {
            return Err(anyhow::anyhow!("A data key already exists"));
        }

        let key = DataKey::generate();
        let recovery_code = self.in_transaction(|| {
            self.grant_user(user_id, password, &key)?;
            self.replace_recovery_code(&key)
        })?;

        info!("Created data key for user ID {}", user_id);
        Ok((key, recovery_code))
    }

    fn unlock_for_user(&self, user_id: i64, password: &str) -> Result<Option<DataKey>> {
        match self.load_wrap("user_id = ?1", &user_id)? {
            Some(wrapped) => Ok(Some(wrapped.open(password)?)),
            None => Ok(None),
        }
    }

    fn grant_user(&self, user_id: i64, password: &str, key: &DataKey) -> Result<()> {
        let wrapped = WrappedKey::seal(key, password, self.params)?;
        self.store_wrap("user", Some(user_id), &wrapped)
    }

    fn unlock_with_recovery_code(&self, recovery_code: &str) -> Result<DataKey> {
        let wrapped = self.load_wrap("kind = ?1", &"recovery")?
            .ok_or_else(|| anyhow::anyhow!("No recovery key has been set up"))?;
        wrapped.open(&normalize_recovery_code(recovery_code))
            .map_err(|_| anyhow::anyhow!("Invalid recovery key"))
    }

    fn replace_recovery_code(&self, key: &DataKey) -> Result<String> {
        let recovery_code = generate_recovery_code();
        let wrapped = WrappedKey::seal(key, &normalize_recovery_code(&recovery_code), self.params)?;
        self.store_wrap("recovery", None, &wrapped)?;
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
