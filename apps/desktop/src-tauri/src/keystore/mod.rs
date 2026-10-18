use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use tracing::info;

use crate::encryption::{
    DataKey, KdfParams, PublicKeyWrap, UserKeyPair, WrappedKey, generate_recovery_code,
    normalize_recovery_code, open_with_secret, seal_to_public_key,
};

/// Status values of a `data_keys` row. Only one row is `active` at a time.
/// A rotation marks the old key `retiring` until every ciphertext has been
/// re-encrypted, then `retired`.
pub const DATA_KEY_STATUS_ACTIVE: &str = "active";
pub const DATA_KEY_STATUS_RETIRING: &str = "retiring";
pub const DATA_KEY_STATUS_RETIRED: &str = "retired";

/// The result of unlocking a user's copy of the data key.
#[derive(Debug)]
pub struct UserUnlock {
    pub key: DataKey,
    /// True when this unlock moved the user from a legacy password wrap to a
    /// keypair, so the caller can audit it.
    pub upgraded: bool,
}

// The install-wide data key encrypts configuration files, firmware, and vault
// secrets. It is never stored in plaintext. Each user has an X25519 keypair
// whose private key is wrapped under their login password; the data key is
// sealed to each user's public key (`data_key_public_wraps`), so it can be
// re-sealed for every user without knowing their passwords. One more copy is
// wrapped under the recovery code shown at setup (`data_key_wraps`, kind
// 'recovery'). Users from before keypairs existed still have a copy wrapped
// directly under their password (`data_key_wraps`, kind 'user') until their
// next login converts it. The unwrapped key only lives in memory, in a session.
pub trait KeyStoreRepository {
    fn has_data_key(&self) -> Result<bool>;
    /// The version of the `active` data key, if one has been created.
    fn active_key_version(&self) -> Result<Option<u32>>;
    /// The version of the `retiring` data key: set while a rotation is in progress.
    fn retiring_key_version(&self) -> Result<Option<u32>>;
    /// Generates the data key, gives the first user a keypair holding it,
    /// wraps it under a new recovery code, and returns the key with the code
    /// to show once.
    fn create_data_key(&self, user_id: i64, password: &str) -> Result<(DataKey, String)>;
    /// Opens the user's copy of the active data key. A user who still has a
    /// legacy password wrap is moved to a keypair in the same transaction.
    /// Returns `None` when the user has no copy, and an error when the
    /// password does not open it.
    fn unlock_user(&self, user_id: i64, password: &str) -> Result<Option<UserUnlock>>;
    /// [`KeyStoreRepository::unlock_user`] without the upgrade flag.
    fn unlock_for_user(&self, user_id: i64, password: &str) -> Result<Option<DataKey>> {
        Ok(self.unlock_user(user_id, password)?.map(|unlocked| unlocked.key))
    }
    /// Gives the user a fresh keypair wrapped under `password` and seals `key`
    /// (which must be the active data key) to it. Replaces any keypair, public
    /// wraps, and legacy wrap the user already had.
    fn set_user_keys(&self, user_id: i64, password: &str, key: &DataKey) -> Result<()>;
    /// Deletes the user's keypair and every wrapped copy of the key they hold,
    /// so their password opens nothing.
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

            -- Each user's X25519 keypair; the private key is wrapped under
            -- their login password with Argon2id.
            CREATE TABLE IF NOT EXISTS user_keypairs (
                user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
                public_key BLOB NOT NULL CHECK(length(public_key) = 32),
                salt BLOB NOT NULL,
                wrapped_private_key BLOB NOT NULL,
                kdf_memory_kib INTEGER NOT NULL,
                kdf_iterations INTEGER NOT NULL,
                kdf_parallelism INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- A data-key version sealed to one user's public key.
            CREATE TABLE IF NOT EXISTS data_key_public_wraps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                key_version INTEGER NOT NULL REFERENCES data_keys(version),
                recipient_public_key BLOB NOT NULL CHECK(length(recipient_public_key) = 32),
                ephemeral_public_key BLOB NOT NULL CHECK(length(ephemeral_public_key) = 32),
                ciphertext BLOB NOT NULL,
                created_by INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, key_version)
            );
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

    /// Loads the user's wrapped private key and stored public key.
    fn load_keypair(&self, user_id: i64) -> Result<Option<(WrappedKey, [u8; 32])>> {
        let row = self.conn.query_row(
            "SELECT salt, wrapped_private_key, kdf_memory_kib, kdf_iterations, kdf_parallelism, public_key
             FROM user_keypairs WHERE user_id = ?1",
            [user_id],
            |row| {
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
                    row.get::<_, Vec<u8>>(5)?,
                ))
            },
        ).optional()?;
        row.map(|(wrapped, public)| Ok((wrapped, to_public_key(&public)?))).transpose()
    }

    fn load_public_wrap(&self, user_id: i64, key_version: u32) -> Result<Option<PublicKeyWrap>> {
        let row = self.conn.query_row(
            "SELECT ephemeral_public_key, ciphertext FROM data_key_public_wraps
             WHERE user_id = ?1 AND key_version = ?2",
            params![user_id, key_version],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?)),
        ).optional()?;
        row.map(|(ephemeral, ciphertext)| {
            Ok(PublicKeyWrap { ephemeral_public: to_public_key(&ephemeral)?, ciphertext })
        }).transpose()
    }

    /// Makes sure `version` is the active data-key version, recording it as
    /// active when no data key has been recorded yet.
    fn ensure_active_version(&self, version: u32) -> Result<()> {
        match self.active_key_version()? {
            Some(active) if active == version => Ok(()),
            Some(active) => Err(anyhow::anyhow!(
                "Data key version {} is not the active version {}", version, active
            )),
            None => {
                self.conn.execute(
                    "INSERT INTO data_keys (version, status) VALUES (?1, ?2)",
                    params![version, DATA_KEY_STATUS_ACTIVE],
                )?;
                Ok(())
            }
        }
    }

    /// Stores a pre-keypair password wrap for a user, as installs made before
    /// keypairs existed have.
    #[cfg(test)]
    pub fn store_legacy_user_wrap(&self, user_id: i64, password: &str, key: &DataKey) -> Result<()> {
        let wrapped = WrappedKey::seal(key, password, self.params)?;
        self.store_wrap("user", Some(user_id), &wrapped, key.version())
    }
}

fn to_public_key(bytes: &[u8]) -> Result<[u8; 32]> {
    bytes.try_into().map_err(|_| anyhow::anyhow!("Invalid public key length"))
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

    fn active_key_version(&self) -> Result<Option<u32>> {
        Ok(self.conn.query_row(
            "SELECT version FROM data_keys WHERE status = ?1",
            [DATA_KEY_STATUS_ACTIVE],
            |row| row.get(0),
        ).optional()?)
    }

    fn retiring_key_version(&self) -> Result<Option<u32>> {
        Ok(self.conn.query_row(
            "SELECT version FROM data_keys WHERE status = ?1",
            [DATA_KEY_STATUS_RETIRING],
            |row| row.get(0),
        ).optional()?)
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
            self.set_user_keys(user_id, password, &key)?;
            self.replace_recovery_code(&key)
        })?;

        info!("Created data key for user ID {}", user_id);
        Ok((key, recovery_code))
    }

    fn unlock_user(&self, user_id: i64, password: &str) -> Result<Option<UserUnlock>> {
        if let Some((wrapped_secret, public_key)) = self.load_keypair(user_id)? {
            let keypair = UserKeyPair::open_secret(&wrapped_secret, password)?;
            if keypair.public_key() != &public_key {
                return Err(anyhow::anyhow!("Stored public key does not match the private key"));
            }
            let Some(version) = self.active_key_version()? else {
                return Ok(None);
            };
            let Some(wrap) = self.load_public_wrap(user_id, version)? else {
                return Ok(None);
            };
            let key = open_with_secret(&wrap, &keypair, user_id, version)?;
            return Ok(Some(UserUnlock { key, upgraded: false }));
        }

        // A user from before keypairs: open the legacy wrap, then replace it
        // with a keypair, atomically
        self.in_transaction(|| {
            let Some((wrapped, key_version)) = self.load_wrap("kind = 'user' AND user_id = ?1", &user_id)? else {
                return Ok(None);
            };
            let key = Self::open_wrap(&wrapped, key_version, password)?;
            self.set_user_keys(user_id, password, &key)?;
            info!("Moved user ID {} from a password wrap to a keypair", user_id);
            Ok(Some(UserUnlock { key, upgraded: true }))
        })
    }

    fn set_user_keys(&self, user_id: i64, password: &str, key: &DataKey) -> Result<()> {
        let keypair = UserKeyPair::generate();
        let wrapped_secret = keypair.seal_secret(password, self.params)?;
        let wrap = seal_to_public_key(key, keypair.public_key(), user_id, key.version())?;

        self.in_transaction(|| {
            self.ensure_active_version(key.version())?;
            self.conn.execute("DELETE FROM data_key_public_wraps WHERE user_id = ?1", [user_id])?;
            self.conn.execute("DELETE FROM user_keypairs WHERE user_id = ?1", [user_id])?;
            self.conn.execute("DELETE FROM data_key_wraps WHERE user_id = ?1", [user_id])?;
            self.conn.execute(
                "INSERT INTO user_keypairs (user_id, public_key, salt, wrapped_private_key, kdf_memory_kib, kdf_iterations, kdf_parallelism)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    user_id,
                    keypair.public_key().as_slice(),
                    wrapped_secret.salt,
                    wrapped_secret.ciphertext,
                    wrapped_secret.params.memory_kib,
                    wrapped_secret.params.iterations,
                    wrapped_secret.params.parallelism,
                ],
            )?;
            self.conn.execute(
                "INSERT INTO data_key_public_wraps (user_id, key_version, recipient_public_key, ephemeral_public_key, ciphertext)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    user_id,
                    key.version(),
                    keypair.public_key().as_slice(),
                    wrap.ephemeral_public.as_slice(),
                    wrap.ciphertext,
                ],
            )?;
            Ok(())
        })
    }

    fn revoke_user(&self, user_id: i64) -> Result<()> {
        self.in_transaction(|| {
            self.conn.execute("DELETE FROM data_key_public_wraps WHERE user_id = ?1", [user_id])?;
            self.conn.execute("DELETE FROM user_keypairs WHERE user_id = ?1", [user_id])?;
            self.conn.execute("DELETE FROM data_key_wraps WHERE user_id = ?1", [user_id])?;
            Ok(())
        })
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

    fn count(conn: &Connection, sql: &str, user_id: i64) -> i64 {
        conn.query_row(sql, [user_id], |row| row.get(0)).unwrap()
    }

    fn legacy_wraps(conn: &Connection, user_id: i64) -> i64 {
        count(conn, "SELECT COUNT(*) FROM data_key_wraps WHERE user_id = ?1", user_id)
    }

    fn keypairs(conn: &Connection, user_id: i64) -> i64 {
        count(conn, "SELECT COUNT(*) FROM user_keypairs WHERE user_id = ?1", user_id)
    }

    fn public_wraps(conn: &Connection, user_id: i64) -> i64 {
        count(conn, "SELECT COUNT(*) FROM data_key_public_wraps WHERE user_id = ?1", user_id)
    }

    fn public_key(conn: &Connection, user_id: i64) -> Vec<u8> {
        conn.query_row("SELECT public_key FROM user_keypairs WHERE user_id = ?1", [user_id], |row| row.get(0)).unwrap()
    }

    fn wrap_version(conn: &Connection, condition: &str) -> u32 {
        conn.query_row(
            &format!("SELECT key_version FROM data_key_wraps WHERE {}", condition), [], |row| row.get(0),
        ).unwrap()
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
    fn test_setup_creates_keypair_and_no_legacy_user_wrap() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        let (key, code) = store.create_data_key(admin, "admin-password").unwrap();

        assert_eq!(keypairs(&conn, admin), 1);
        assert_eq!(public_wraps(&conn, admin), 1);
        assert_eq!(legacy_wraps(&conn, admin), 0);
        let user_wraps: i64 = conn.query_row(
            "SELECT COUNT(*) FROM data_key_wraps WHERE kind = 'user'", [], |row| row.get(0),
        ).unwrap();
        assert_eq!(user_wraps, 0);

        // The recovery code still lives in data_key_wraps
        assert!(same_key(&key, &store.unlock_with_recovery_code(&code).unwrap()));
        let unlocked = store.unlock_user(admin, "admin-password").unwrap().unwrap();
        assert!(!unlocked.upgraded);
        assert!(same_key(&key, &unlocked.key));
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
    fn test_user_with_keys_gets_same_key() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let engineer = create_user(&conn, "engineer");
        let store = store(&conn);

        let (key, _code) = store.create_data_key(admin, "admin-password").unwrap();
        store.set_user_keys(engineer, "engineer-password", &key).unwrap();

        let unlocked = store.unlock_for_user(engineer, "engineer-password").unwrap().unwrap();
        assert!(same_key(&key, &unlocked));
        assert_eq!(unlocked.version(), key.version());
    }

    #[test]
    fn test_legacy_user_is_moved_to_keypair_on_unlock() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let engineer = create_user(&conn, "engineer");
        let store = store(&conn);

        let (key, _code) = store.create_data_key(admin, "admin-password").unwrap();
        store.store_legacy_user_wrap(engineer, "engineer-password", &key).unwrap();
        assert_eq!(legacy_wraps(&conn, engineer), 1);
        assert_eq!(keypairs(&conn, engineer), 0);

        let first = store.unlock_user(engineer, "engineer-password").unwrap().unwrap();
        assert!(first.upgraded);
        assert!(same_key(&key, &first.key));
        assert_eq!(legacy_wraps(&conn, engineer), 0);
        assert_eq!(keypairs(&conn, engineer), 1);
        assert_eq!(public_wraps(&conn, engineer), 1);
        let migrated_public_key = public_key(&conn, engineer);

        // A second unlock uses the keypair and does not migrate again
        let second = store.unlock_user(engineer, "engineer-password").unwrap().unwrap();
        assert!(!second.upgraded);
        assert!(same_key(&key, &second.key));
        assert_eq!(public_key(&conn, engineer), migrated_public_key);
        assert!(store.unlock_user(engineer, "wrong-password").is_err());
    }

    #[test]
    fn test_legacy_unlock_with_wrong_password_changes_nothing() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let engineer = create_user(&conn, "engineer");
        let store = store(&conn);

        let (key, _code) = store.create_data_key(admin, "admin-password").unwrap();
        store.store_legacy_user_wrap(engineer, "engineer-password", &key).unwrap();

        assert!(store.unlock_user(engineer, "wrong-password").is_err());
        assert_eq!(legacy_wraps(&conn, engineer), 1);
        assert_eq!(keypairs(&conn, engineer), 0);
    }

    #[test]
    fn test_legacy_migration_keeps_the_row_version() {
        let conn = setup();
        let engineer = create_user(&conn, "engineer");
        let store = store(&conn);

        // An install whose active key is version 4, with a legacy wrap of it
        conn.execute("INSERT INTO data_keys (version, status) VALUES (4, 'active')", []).unwrap();
        let key = DataKey::generate_with_version(4);
        store.store_legacy_user_wrap(engineer, "engineer-password", &key).unwrap();

        let unlocked = store.unlock_user(engineer, "engineer-password").unwrap().unwrap();
        assert!(unlocked.upgraded);
        assert_eq!(unlocked.key.version(), 4);
        let wrap_version: u32 = conn.query_row(
            "SELECT key_version FROM data_key_public_wraps WHERE user_id = ?1", [engineer], |row| row.get(0),
        ).unwrap();
        assert_eq!(wrap_version, 4);
        assert_eq!(store.unlock_for_user(engineer, "engineer-password").unwrap().unwrap().version(), 4);
    }

    #[test]
    fn test_set_user_keys_replaces_keypair() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        let (key, _code) = store.create_data_key(admin, "old-password").unwrap();
        let old_public_key = public_key(&conn, admin);
        store.set_user_keys(admin, "new-password", &key).unwrap();

        assert_ne!(public_key(&conn, admin), old_public_key);
        assert_eq!(keypairs(&conn, admin), 1);
        assert_eq!(public_wraps(&conn, admin), 1);
        assert!(store.unlock_for_user(admin, "old-password").is_err());
        assert!(same_key(&key, &store.unlock_for_user(admin, "new-password").unwrap().unwrap()));
    }

    #[test]
    fn test_set_user_keys_refuses_a_key_that_is_not_active() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let engineer = create_user(&conn, "engineer");
        let store = store(&conn);

        store.create_data_key(admin, "admin-password").unwrap();
        let other = DataKey::generate_with_version(2);
        assert!(store.set_user_keys(engineer, "engineer-password", &other).is_err());
        assert_eq!(keypairs(&conn, engineer), 0);
    }

    #[test]
    fn test_revoked_user_unlocks_nothing_and_can_be_regranted() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let engineer = create_user(&conn, "engineer");
        let legacy = create_user(&conn, "legacy");
        let store = store(&conn);

        let (key, _code) = store.create_data_key(admin, "admin-password").unwrap();
        store.set_user_keys(engineer, "engineer-password", &key).unwrap();
        store.store_legacy_user_wrap(legacy, "legacy-password", &key).unwrap();
        store.revoke_user(engineer).unwrap();
        store.revoke_user(legacy).unwrap();

        for user in [engineer, legacy] {
            assert_eq!(keypairs(&conn, user), 0);
            assert_eq!(public_wraps(&conn, user), 0);
            assert_eq!(legacy_wraps(&conn, user), 0);
        }
        assert!(store.unlock_for_user(engineer, "engineer-password").unwrap().is_none());
        assert!(store.unlock_for_user(legacy, "legacy-password").unwrap().is_none());
        // Other users and the recovery code are untouched
        assert!(store.unlock_for_user(admin, "admin-password").unwrap().is_some());

        store.set_user_keys(engineer, "new-password", &key).unwrap();
        let unlocked = store.unlock_for_user(engineer, "new-password").unwrap().unwrap();
        assert!(same_key(&key, &unlocked));
    }

    #[test]
    fn test_public_wrap_for_another_user_id_does_not_open() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let engineer = create_user(&conn, "engineer");
        let store = store(&conn);

        store.create_data_key(admin, "admin-password").unwrap();
        // Give the engineer copies of the admin's keypair and wrap: the wrap
        // is bound to the admin's user ID, so it must not open as the engineer
        conn.execute_batch(&format!(
            "INSERT INTO user_keypairs (user_id, public_key, salt, wrapped_private_key, kdf_memory_kib, kdf_iterations, kdf_parallelism)
                 SELECT {engineer}, public_key, salt, wrapped_private_key, kdf_memory_kib, kdf_iterations, kdf_parallelism
                 FROM user_keypairs WHERE user_id = {admin};
             INSERT INTO data_key_public_wraps (user_id, key_version, recipient_public_key, ephemeral_public_key, ciphertext)
                 SELECT {engineer}, key_version, recipient_public_key, ephemeral_public_key, ciphertext
                 FROM data_key_public_wraps WHERE user_id = {admin};"
        )).unwrap();

        assert!(store.unlock_for_user(engineer, "admin-password").is_err());
        assert!(store.unlock_for_user(admin, "admin-password").unwrap().is_some());
    }

    #[test]
    fn test_keypair_without_wrap_for_active_version_unlocks_nothing() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        store.create_data_key(admin, "admin-password").unwrap();
        conn.execute("DELETE FROM data_key_public_wraps", []).unwrap();

        assert!(store.unlock_for_user(admin, "admin-password").unwrap().is_none());
        assert!(store.unlock_for_user(admin, "wrong-password").is_err());
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
        // The admin's keypair is untouched
        assert!(store.unlock_for_user(admin, "admin-password").unwrap().is_some());
    }

    #[test]
    fn test_create_data_key_records_active_version() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        let (key, _code) = store.create_data_key(admin, "admin-password").unwrap();
        assert_eq!(key.version(), crate::encryption::INITIAL_KEY_VERSION);
        assert_eq!(store.active_key_version().unwrap(), Some(key.version()));

        let (version, status, created_by): (u32, String, i64) = conn.query_row(
            "SELECT version, status, created_by FROM data_keys", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap();
        assert_eq!((version, status.as_str(), created_by), (key.version(), "active", admin));
        assert_eq!(wrap_version(&conn, "kind = 'recovery'"), key.version());
    }

    #[test]
    fn test_recovery_key_takes_version_from_row() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        let (_key, code) = store.create_data_key(admin, "admin-password").unwrap();
        // Pretend the recovery wrap holds a later key version
        conn.execute("UPDATE data_key_wraps SET key_version = 4 WHERE kind = 'recovery'", []).unwrap();

        assert_eq!(store.unlock_with_recovery_code(&code).unwrap().version(), 4);
    }

    #[test]
    fn test_recovery_wrap_stores_the_key_version() {
        let conn = setup();
        let admin = create_user(&conn, "admin");
        let store = store(&conn);

        store.create_data_key(admin, "admin-password").unwrap();
        let later = DataKey::generate_with_version(3);
        let code = store.replace_recovery_code(&later).unwrap();

        assert_eq!(wrap_version(&conn, "kind = 'recovery'"), 3);
        assert_eq!(store.unlock_with_recovery_code(&code).unwrap().version(), 3);
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
        assert_eq!(keypairs(&conn, admin), 0);
    }
}
