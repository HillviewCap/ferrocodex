//! Data-key rotation (design chunk 5).
//!
//! Rotation replaces the install-wide data key K1 (version v) with a fresh K2
//! (version v+1) and re-encrypts every ciphertext under K2:
//!
//! 1. [`KeyRotation::begin`] makes K2 the active key, seals it to every active
//!    user's public key, deletes every copy of K1 (public wraps, legacy password
//!    wraps, the recovery wrap), and issues a new recovery code. K1 itself is
//!    kept only encrypted under K2 in `data_keys.wrapped_by_active`, so any
//!    administrator who can open K2 can resume an interrupted rotation
//!    ([`KeyRotation::rotation_keys`]).
//! 2. [`KeyRotation::process_db_batch`] re-encrypts a bounded batch of DB rows
//!    per call; firmware files go one per call through
//!    [`KeyRotation::next_firmware`], [`FirmwareFileStorage::reencrypt_firmware_file`]
//!    and [`KeyRotation::commit_firmware`]. Rows that don't decrypt under K1 are
//!    recorded in `key_rotation_failures` and skipped.
//! 3. [`KeyRotation::finalize`] marks K1 retired and deletes its wrapped copy.
//!
//! While a rotation is in progress the session gate (`SessionToken::data_key`)
//! blocks every encrypted-data command, so nothing is read or written under a
//! key that is being replaced.
//!
//! [`FirmwareFileStorage::reencrypt_firmware_file`]: crate::firmware::FirmwareFileStorage::reencrypt_firmware_file

use std::fmt;

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use rusqlite::types::Value;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::{
    KeyStoreRepository, SqliteKeyStoreRepository, DATA_KEY_STATUS_ACTIVE, DATA_KEY_STATUS_RETIRED,
    DATA_KEY_STATUS_RETIRING,
};
use crate::audit::{AuditEventRequest, AuditEventType, AuditRepository, SqliteAuditRepository};
use crate::encryption::{seal_to_public_key, AuthenticationFailed, DataKey, FileEncryption, KdfParams};
use crate::firmware::FirmwareRecord;

/// DB rows re-encrypted per `continue` call. Configuration files can be a few
/// MB each, so keep a batch small enough to hold the DB lock only briefly.
pub const DEFAULT_BATCH_SIZE: usize = 25;

/// `key_rotation_failures.table_name` for firmware files.
pub const FIRMWARE_TABLE: &str = "firmware_versions";

const RETIRING_KEY_AAD_PREFIX: &[u8] = b"ferrocodex-retiring-key-v1";

#[derive(Clone, Copy)]
enum Encoding {
    /// Raw `FileEncryption::encrypt` output in a BLOB column.
    Blob,
    /// `FileEncryption::encrypt_to_base64` output in a TEXT column.
    Base64,
}

/// A DB column that holds data-key ciphertext.
struct CipherColumn {
    table: &'static str,
    column: &'static str,
    encoding: Encoding,
}

/// Every DB column holding data-key ciphertext (design §0.6). Table and column
/// names are only ever taken from this list, never from input.
const DB_COLUMNS: [CipherColumn; 3] = [
    CipherColumn { table: "configuration_versions", column: "file_content", encoding: Encoding::Blob },
    CipherColumn { table: "vault_secrets", column: "encrypted_value", encoding: Encoding::Base64 },
    CipherColumn { table: "standalone_credentials", column: "encrypted_data", encoding: Encoding::Base64 },
];

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Rows a rotation could not decrypt under the retiring key
        CREATE TABLE IF NOT EXISTS key_rotation_failures (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            table_name TEXT NOT NULL,
            row_id INTEGER NOT NULL,
            from_version INTEGER NOT NULL,
            error TEXT NOT NULL,
            attempted_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(table_name, row_id, from_version)
        );

        -- Firmware files a rotation replaced. They are deleted only once the
        -- DB change pointing away from them is durable (after a WAL
        -- checkpoint at finalize): a non-durable commit can roll back at a
        -- power cut, and the row would then point at a deleted file.
        CREATE TABLE IF NOT EXISTS key_rotation_superseded_files (
            file_path TEXT PRIMARY KEY,
            firmware_id INTEGER NOT NULL,
            from_version INTEGER NOT NULL,
            recorded_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )?;
    Ok(())
}

fn retiring_key_aad(from_version: u32, to_version: u32) -> Vec<u8> {
    let mut aad = Vec::with_capacity(RETIRING_KEY_AAD_PREFIX.len() + 8);
    aad.extend_from_slice(RETIRING_KEY_AAD_PREFIX);
    aad.extend_from_slice(&from_version.to_be_bytes());
    aad.extend_from_slice(&to_version.to_be_bytes());
    aad
}

/// The retiring key and the active key of a rotation in progress.
#[derive(Debug, Clone)]
pub struct RotationKeys {
    pub old: DataKey,
    pub new: DataKey,
}

impl RotationKeys {
    pub fn from_version(&self) -> u32 {
        self.old.version()
    }

    pub fn to_version(&self) -> u32 {
        self.new.version()
    }
}

/// An active user, as listed in rotation status and responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserKeyAccess {
    pub id: i64,
    pub username: String,
    pub role: String,
}

/// Ciphertexts still under the retiring key, per table. Rows recorded as
/// failures are not counted here.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemainingCounts {
    pub configuration_versions: i64,
    pub vault_secrets: i64,
    pub standalone_credentials: i64,
    pub firmware_versions: i64,
    pub total: i64,
}

/// A row or firmware file that did not decrypt under the retiring key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRotationFailure {
    pub id: i64,
    pub table_name: String,
    pub row_id: i64,
    pub from_version: u32,
    pub error: String,
    pub attempted_at: String,
}

/// Administrator-only part of [`KeyRotationStatus`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRotationDetails {
    /// All zero when no rotation is in progress.
    pub remaining: RemainingCounts,
    /// Failures of the rotation in progress.
    pub failures: Vec<KeyRotationFailure>,
    /// Active users with no keypair: they lose access at the next rotation.
    pub users_without_keypair: Vec<UserKeyAccess>,
    /// Active users who cannot open the active key now (no wrap for it).
    /// An administrator password reset gives them access again.
    pub users_without_access: Vec<UserKeyAccess>,
    /// Firmware-analysis jobs queued or running. Rotation can't begin while
    /// any exist.
    pub active_analysis_jobs: usize,
}

/// Returned by `get_key_rotation_status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRotationStatus {
    pub in_progress: bool,
    pub active_version: Option<u32>,
    /// The key being replaced, while `in_progress`.
    pub retiring_version: Option<u32>,
    /// `None` unless the caller is an administrator.
    pub details: Option<KeyRotationDetails>,
}

/// Returned by `continue_data_key_rotation`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRotationProgress {
    pub from_version: u32,
    pub to_version: u32,
    /// Rows or files re-encrypted by this call.
    pub processed: usize,
    /// Rows or files this call could not decrypt (added to the failures).
    pub failed: usize,
    /// What is still to be re-encrypted after this call.
    pub remaining: RemainingCounts,
    /// Failures recorded so far in this rotation.
    pub failure_count: i64,
    /// Nothing remains to re-encrypt.
    pub done: bool,
    /// This call finalized the rotation (only when done with no failures).
    pub finalized: bool,
}

/// Returned by `begin_data_key_rotation`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeginKeyRotationResponse {
    /// The new recovery code. Shown once; the old code no longer works.
    pub recovery_code: String,
    pub from_version: u32,
    pub to_version: u32,
    /// Active users who had no keypair and so did not get the new key. An
    /// administrator password reset gives them access again.
    pub users_without_access: Vec<UserKeyAccess>,
    /// Problems after the rotation was committed that did not stop it, for
    /// example the session could not be switched to the new key (log in
    /// again). The recovery code above is valid either way.
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl fmt::Debug for BeginKeyRotationResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BeginKeyRotationResponse")
            .field("recovery_code", &"<redacted>")
            .field("from_version", &self.from_version)
            .field("to_version", &self.to_version)
            .field("users_without_access", &self.users_without_access)
            .field("warnings", &self.warnings)
            .finish()
    }
}

/// Returned by `finalize_data_key_rotation`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRotationFinalized {
    pub retired_version: u32,
    pub active_version: u32,
    /// Unreadable rows and files left behind (still under the retired key).
    pub discarded: i64,
    /// Replaced and leftover firmware files deleted after the rotation.
    pub orphaned_files_removed: usize,
    /// The rotation had already been finalized (for example automatically by
    /// the last `continue`); this call only retried the file cleanup.
    #[serde(default)]
    pub already_finalized: bool,
    /// Problems that did not stop finalizing, for example old firmware files
    /// kept because the database could not be checkpointed yet.
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// What [`KeyRotation::begin`] produced: the new key for the admin's session
/// and the response for the frontend.
pub struct BeginOutcome {
    pub new_key: DataKey,
    pub response: BeginKeyRotationResponse,
}

/// What one DB batch did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BatchResult {
    pub processed: usize,
    pub failed: usize,
}

/// A firmware file still under the retiring key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareItem {
    pub id: i64,
    pub file_path: String,
}

/// What [`KeyRotation::finalize`] did in the DB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalizeOutcome {
    pub retired_version: u32,
    pub active_version: u32,
    pub discarded: i64,
}

const NOT_DECRYPTABLE: &str = "Could not be decrypted with the old key";

fn reencrypt_bytes(bytes: &mut [u8], keys: &RotationKeys) -> std::result::Result<(), String> {
    FileEncryption::reencrypt_in_place(&keys.old, &keys.new, bytes).map_err(|e| {
        if e.is::<AuthenticationFailed>() {
            NOT_DECRYPTABLE.to_string()
        } else {
            e.to_string()
        }
    })
}

/// The raw ciphertext bytes of a stored value, if it has the expected shape.
fn stored_bytes(value: Value, encoding: Encoding) -> std::result::Result<Vec<u8>, String> {
    match (encoding, value) {
        (Encoding::Blob, Value::Blob(bytes)) => Ok(bytes),
        (Encoding::Base64, Value::Text(text)) => general_purpose::STANDARD
            .decode(text.as_bytes())
            .map_err(|_| "Stored value is not valid base64".to_string()),
        _ => Err("Stored value has an unexpected type".to_string()),
    }
}

/// Why re-encrypting one value failed. The message never contains data.
fn reencrypt_value(value: Value, encoding: Encoding, keys: &RotationKeys) -> std::result::Result<Value, String> {
    let mut bytes = stored_bytes(value, encoding)?;
    reencrypt_bytes(&mut bytes, keys)?;
    Ok(match encoding {
        Encoding::Blob => Value::Blob(bytes),
        Encoding::Base64 => Value::Text(general_purpose::STANDARD.encode(bytes)),
    })
}

fn commit(tx: Option<rusqlite::Transaction<'_>>) -> Result<()> {
    if let Some(tx) = tx {
        tx.commit()?;
    }
    Ok(())
}

pub struct KeyRotation<'a> {
    conn: &'a Connection,
    params: KdfParams,
    batch_size: usize,
}

impl<'a> KeyRotation<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn, params: KdfParams::default(), batch_size: DEFAULT_BATCH_SIZE }
    }

    /// Uses `params` for the recovery-code wrap written by [`KeyRotation::begin`].
    #[cfg(test)]
    pub fn with_params(conn: &'a Connection, params: KdfParams) -> Self {
        Self { conn, params, batch_size: DEFAULT_BATCH_SIZE }
    }

    /// DB rows per [`KeyRotation::process_db_batch`] call (at least 1).
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size.max(1);
        self
    }

    fn key_store(&self) -> SqliteKeyStoreRepository<'a> {
        SqliteKeyStoreRepository::with_params(self.conn, self.params)
    }

    /// Starts a transaction, or returns `None` to join the caller's if one is
    /// open (the work then commits or rolls back with it).
    fn begin_tx(&self) -> Result<Option<rusqlite::Transaction<'a>>> {
        if self.conn.is_autocommit() {
            Ok(Some(self.conn.unchecked_transaction()?))
        } else {
            Ok(None)
        }
    }

    #[cfg(test)]
    pub fn retiring_version(&self) -> Result<Option<u32>> {
        self.key_store().retiring_key_version()
    }

    /// The keys of the rotation in progress, from the active key `active`
    /// (an administrator's session key). The retiring key is recovered from
    /// its copy encrypted under the active key, which is how an interrupted
    /// rotation resumes after a restart or from another administrator.
    pub fn rotation_keys(&self, active: &DataKey) -> Result<RotationKeys> {
        let active_version = self.key_store().active_key_version()?
            .ok_or_else(|| anyhow!("No data key has been created"))?;
        if active.version() != active_version {
            return Err(anyhow!("Your session holds an outdated encryption key; log in again"));
        }
        let (from_version, wrapped): (u32, Option<Vec<u8>>) = self.conn.query_row(
            "SELECT version, wrapped_by_active FROM data_keys WHERE status = ?1",
            [DATA_KEY_STATUS_RETIRING],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?
            .ok_or_else(|| anyhow!("No key rotation is in progress"))?;
        let wrapped = wrapped.ok_or_else(|| anyhow!("The retiring key is missing"))?;
        let old = DataKey::open_under(&wrapped, active, &retiring_key_aad(from_version, active_version), from_version)
            .map_err(|_| anyhow!("The retiring key does not open with your session key"))?;
        Ok(RotationKeys { old, new: active.clone() })
    }

    /// Active users matching `condition` (fixed SQL, never input).
    fn list_users(&self, condition: &str, params: &[&dyn rusqlite::ToSql]) -> Result<Vec<UserKeyAccess>> {
        let sql = format!(
            "SELECT u.id, u.username, u.role FROM users u WHERE u.is_active = 1 AND ({}) ORDER BY u.username",
            condition
        );
        Ok(self.conn
            .prepare(&sql)?
            .query_map(params, |row| Ok(UserKeyAccess { id: row.get(0)?, username: row.get(1)?, role: row.get(2)? }))?
            .collect::<rusqlite::Result<_>>()?)
    }

    /// Active users without a keypair. A rotation can't seal the new key to
    /// them, so they lose access until an administrator resets their password.
    pub fn users_without_keypair(&self) -> Result<Vec<UserKeyAccess>> {
        self.list_users("NOT EXISTS (SELECT 1 FROM user_keypairs k WHERE k.user_id = u.id)", &[])
    }

    /// Active users who hold no copy of the active key: no public wrap for it
    /// and no legacy password wrap of it.
    pub fn users_without_access(&self) -> Result<Vec<UserKeyAccess>> {
        let active = self.key_store().active_key_version()?;
        self.list_users(
            "NOT EXISTS (SELECT 1 FROM data_key_public_wraps w WHERE w.user_id = u.id AND w.key_version = ?1)
             AND NOT EXISTS (SELECT 1 FROM data_key_wraps w WHERE w.kind = 'user' AND w.user_id = u.id AND w.key_version = ?1)",
            &[&active],
        )
    }

    /// Starts a rotation away from `current`, the active key that the
    /// administrator just re-opened with their password.
    ///
    /// Refuses while a rotation is in progress, while firmware-analysis jobs
    /// (which hold a copy of the key) are queued or running, and, unless
    /// `acknowledge_users_without_keys`, while some active user has no keypair.
    /// Everything else happens in one transaction.
    pub fn begin(
        &self,
        admin_id: i64,
        admin_username: &str,
        current: &DataKey,
        active_analysis_jobs: usize,
        acknowledge_users_without_keys: bool,
    ) -> Result<BeginOutcome> {
        let store = self.key_store();
        if store.retiring_key_version()?.is_some() {
            return Err(anyhow!("A key rotation is already in progress; continue or finalize it"));
        }
        if active_analysis_jobs > 0 {
            return Err(anyhow!(
                "{} firmware analysis job(s) are queued or running; wait for them to finish before rotating the encryption key",
                active_analysis_jobs
            ));
        }
        let from_version = store.active_key_version()?
            .ok_or_else(|| anyhow!("No data key has been created"))?;
        if current.version() != from_version {
            return Err(anyhow!("Your session holds an outdated encryption key; log in again"));
        }
        let users_without_access = self.users_without_keypair()?;
        if !users_without_access.is_empty() && !acknowledge_users_without_keys {
            let names: Vec<&str> = users_without_access.iter().map(|u| u.username.as_str()).collect();
            return Err(anyhow!(
                "{} active user(s) have no encryption keypair and will lose access to encrypted data until an administrator resets their password: {}. Acknowledge this to continue.",
                users_without_access.len(),
                names.join(", ")
            ));
        }

        let max_version: u32 = self.conn.query_row("SELECT MAX(version) FROM data_keys", [], |row| row.get(0))?;
        let to_version = max_version
            .checked_add(1)
            .ok_or_else(|| anyhow!("Key version overflow"))?;
        let new_key = DataKey::generate_with_version(to_version);
        let wrapped_old = current.seal_under(&new_key, &retiring_key_aad(from_version, to_version))?;

        let recipients: Vec<(i64, Vec<u8>)> = self.conn
            .prepare(
                "SELECT k.user_id, k.public_key FROM user_keypairs k
                 JOIN users u ON u.id = k.user_id WHERE u.is_active = 1 ORDER BY k.user_id",
            )?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;

        let tx = self.begin_tx()?;

        // 1. K2 becomes active; K1 is kept only encrypted under K2
        let retired = self.conn.execute(
            "UPDATE data_keys SET status = ?1, wrapped_by_active = ?2 WHERE version = ?3 AND status = ?4",
            params![DATA_KEY_STATUS_RETIRING, wrapped_old, from_version, DATA_KEY_STATUS_ACTIVE],
        )?;
        if retired != 1 {
            return Err(anyhow!("The active data key changed; try again"));
        }
        self.conn.execute(
            "INSERT INTO data_keys (version, status, created_by) VALUES (?1, ?2, ?3)",
            params![to_version, DATA_KEY_STATUS_ACTIVE, admin_id],
        )?;

        // 2. Seal K2 to every active user with a keypair
        for (user_id, public_key) in &recipients {
            let public_key: [u8; 32] = public_key.as_slice().try_into()
                .map_err(|_| anyhow!("Invalid public key length"))?;
            let wrap = seal_to_public_key(&new_key, &public_key, *user_id, to_version)?;
            self.conn.execute(
                "INSERT INTO data_key_public_wraps (user_id, key_version, recipient_public_key, ephemeral_public_key, ciphertext, created_by)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![user_id, to_version, public_key.as_slice(), wrap.ephemeral_public.as_slice(), wrap.ciphertext, admin_id],
            )?;
        }

        // 3. Delete every other copy of K1: older public wraps, legacy password
        // wraps (an unconverted user's wrap at an old version would also break
        // their login), and inactive users' keypairs
        self.conn.execute("DELETE FROM data_key_public_wraps WHERE key_version <> ?1", [to_version])?;
        self.conn.execute("DELETE FROM data_key_wraps WHERE kind = 'user'", [])?;
        self.conn.execute(
            "DELETE FROM user_keypairs WHERE user_id IN (SELECT id FROM users WHERE is_active = 0)",
            [],
        )?;

        // 4. The recovery code now opens K2
        let recovery_code = store.replace_recovery_code(&new_key)?;

        // 5. Audit
        SqliteAuditRepository::new(self.conn).log_event(&AuditEventRequest {
            event_type: AuditEventType::DataKeyRotationStarted,
            user_id: Some(admin_id),
            username: Some(admin_username.to_string()),
            admin_user_id: Some(admin_id),
            admin_username: Some(admin_username.to_string()),
            target_user_id: None,
            target_username: None,
            description: format!(
                "Administrator '{}' started rotating the data encryption key from version {} to {}; the recovery key was replaced",
                admin_username, from_version, to_version
            ),
            metadata: Some(serde_json::json!({
                "from_version": from_version,
                "to_version": to_version,
                "users_resealed": recipients.len(),
                "users_without_access": users_without_access.iter().map(|u| u.id).collect::<Vec<_>>(),
            }).to_string()),
            ip_address: None,
            user_agent: None,
        })?;

        commit(tx)?;
        info!("Data-key rotation started: version {} -> {}", from_version, to_version);

        Ok(BeginOutcome {
            new_key,
            response: BeginKeyRotationResponse { recovery_code, from_version, to_version, users_without_access, warnings: Vec::new() },
        })
    }

    fn record_failure_row(&self, table: &str, row_id: i64, from_version: u32, error: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO key_rotation_failures (table_name, row_id, from_version, error) VALUES (?1, ?2, ?3, ?4)",
            params![table, row_id, from_version, error],
        )?;
        Ok(())
    }

    /// Records that a row or file did not decrypt under the retiring key.
    pub fn record_failure(&self, table: &str, row_id: i64, from_version: u32, error: &str) -> Result<()> {
        self.record_failure_row(table, row_id, from_version, error)?;
        info!("Key rotation could not re-encrypt {} row {}", table, row_id);
        Ok(())
    }

    /// Re-encrypts up to the batch size of DB rows from the first table that
    /// still has rows under the retiring key, in one transaction. Updates are
    /// conditional on the row still being at the old version, so re-running a
    /// batch after a crash is harmless. Returns `None` when no DB rows remain.
    pub fn process_db_batch(&self, keys: &RotationKeys) -> Result<Option<BatchResult>> {
        let from = keys.from_version();
        let to = keys.to_version();
        for column in &DB_COLUMNS {
            let select = format!(
                "SELECT id, {col} FROM {table}
                 WHERE key_version = ?1
                   AND id NOT IN (SELECT row_id FROM key_rotation_failures WHERE table_name = ?2 AND from_version = ?1)
                 ORDER BY id LIMIT ?3",
                col = column.column,
                table = column.table,
            );
            let rows: Vec<(i64, Value)> = self.conn
                .prepare(&select)?
                .query_map(params![from, column.table, self.batch_size as i64], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            if rows.is_empty() {
                continue;
            }

            let update = format!(
                "UPDATE {table} SET {col} = ?1, key_version = ?2 WHERE id = ?3 AND key_version = ?4",
                col = column.column,
                table = column.table,
            );
            let mut result = BatchResult::default();
            let tx = self.begin_tx()?;
            for (id, value) in rows {
                match reencrypt_value(value, column.encoding, keys) {
                    Ok(new_value) => {
                        result.processed += self.conn.execute(&update, params![new_value, to, id, from])?;
                    }
                    Err(error) => {
                        self.record_failure_row(column.table, id, from, &error)?;
                        result.failed += 1;
                    }
                }
            }
            commit(tx)?;
            if result.failed > 0 {
                info!("Key rotation could not re-encrypt {} row(s) of {}", result.failed, column.table);
            }
            return Ok(Some(result));
        }
        Ok(None)
    }

    /// The next firmware file still under the retiring key, skipping failures.
    pub fn next_firmware(&self, from_version: u32) -> Result<Option<FirmwareItem>> {
        Ok(self.conn.query_row(
            "SELECT id, file_path FROM firmware_versions
             WHERE key_version = ?1
               AND id NOT IN (SELECT row_id FROM key_rotation_failures WHERE table_name = ?2 AND from_version = ?1)
             ORDER BY id LIMIT 1",
            params![from_version, FIRMWARE_TABLE],
            |row| Ok(FirmwareItem { id: row.get(0)?, file_path: row.get(1)? }),
        ).optional()?)
    }

    /// Points a firmware row at its re-encrypted file, only if the row is
    /// still the one that was read (same version and path). Returns whether
    /// it was updated. In the same transaction, records the old file as
    /// superseded so finalize deletes it once this change is durable.
    pub fn commit_firmware(&self, item: &FirmwareItem, new_path: &str, keys: &RotationKeys) -> Result<bool> {
        let tx = self.begin_tx()?;
        let updated = self.conn.execute(
            "UPDATE firmware_versions SET file_path = ?1, key_version = ?2
             WHERE id = ?3 AND key_version = ?4 AND file_path = ?5",
            params![new_path, keys.to_version(), item.id, keys.from_version(), item.file_path],
        )? == 1;
        if updated && item.file_path != new_path {
            self.conn.execute(
                "INSERT OR IGNORE INTO key_rotation_superseded_files (file_path, firmware_id, from_version) VALUES (?1, ?2, ?3)",
                params![item.file_path, item.id, keys.from_version()],
            )?;
        }
        commit(tx)?;
        Ok(updated)
    }

    /// Firmware files replaced by a rotation and not deleted yet.
    pub fn superseded_files(&self) -> Result<Vec<String>> {
        Ok(self.conn
            .prepare("SELECT file_path FROM key_rotation_superseded_files ORDER BY file_path")?
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }

    /// Stops tracking superseded files (deleted, or found to be in use).
    pub fn forget_superseded(&self, paths: &[String]) -> Result<()> {
        let tx = self.begin_tx()?;
        for path in paths {
            self.conn.execute("DELETE FROM key_rotation_superseded_files WHERE file_path = ?1", [path])?;
        }
        commit(tx)
    }

    /// Deletes the recorded failures of the rotation in progress, so the
    /// next batches try those rows and files again. Returns how many.
    pub fn clear_failures(&self, from_version: u32) -> Result<usize> {
        Ok(self.conn.execute("DELETE FROM key_rotation_failures WHERE from_version = ?1", [from_version])?)
    }

    /// Failed DB rows that do decrypt under the retiring key now (for
    /// example restored from a backup since). Discarding them would lose
    /// them, so finalize refuses while any exist.
    pub fn failed_rows_now_readable(&self, keys: &RotationKeys) -> Result<Vec<(String, i64)>> {
        let from = keys.from_version();
        let mut readable = Vec::new();
        for column in &DB_COLUMNS {
            let select = format!(
                "SELECT t.id, t.{col} FROM {table} t
                 JOIN key_rotation_failures f ON f.row_id = t.id AND f.table_name = ?1 AND f.from_version = ?2
                 WHERE t.key_version = ?2",
                col = column.column,
                table = column.table,
            );
            let rows: Vec<(i64, Value)> = self.conn
                .prepare(&select)?
                .query_map(params![column.table, from], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            for (id, value) in rows {
                if let Ok(mut bytes) = stored_bytes(value, column.encoding) {
                    if FileEncryption::authenticates_in_place(&keys.old, &mut bytes)
                        || FileEncryption::authenticates_in_place(&keys.new, &mut bytes)
                    {
                        readable.push((column.table.to_string(), id));
                    }
                }
            }
        }
        Ok(readable)
    }

    /// Firmware files recorded as failures that are still at the retiring version.
    pub fn failed_firmware(&self, from_version: u32) -> Result<Vec<FirmwareItem>> {
        Ok(self.conn
            .prepare(
                "SELECT v.id, v.file_path FROM firmware_versions v
                 JOIN key_rotation_failures f ON f.row_id = v.id AND f.table_name = ?1 AND f.from_version = ?2
                 WHERE v.key_version = ?2 ORDER BY v.id",
            )?
            .query_map(params![FIRMWARE_TABLE, from_version], |row| Ok(FirmwareItem { id: row.get(0)?, file_path: row.get(1)? }))?
            .collect::<rusqlite::Result<_>>()?)
    }

    /// The most recently retired key version, if any.
    pub fn latest_retired_version(&self) -> Result<Option<u32>> {
        Ok(self.conn.query_row(
            "SELECT MAX(version) FROM data_keys WHERE status = ?1",
            [DATA_KEY_STATUS_RETIRED],
            |row| row.get(0),
        )?)
    }

    /// The stored path of a firmware row, if the row exists.
    pub fn firmware_path(&self, firmware_id: i64) -> Result<Option<String>> {
        Ok(self.conn.query_row(
            "SELECT file_path FROM firmware_versions WHERE id = ?1",
            [firmware_id],
            |row| row.get(0),
        ).optional()?)
    }

    /// Every firmware record's id, stored path, and key version, for the
    /// finalize sweep.
    pub fn firmware_records(&self) -> Result<Vec<FirmwareRecord>> {
        Ok(self.conn
            .prepare("SELECT id, file_path, key_version FROM firmware_versions")?
            .query_map([], |row| Ok(FirmwareRecord { id: row.get(0)?, file_path: row.get(1)?, key_version: row.get(2)? }))?
            .collect::<rusqlite::Result<_>>()?)
    }

    /// Ciphertexts still under `from_version`, excluding recorded failures.
    pub fn remaining(&self, from_version: u32) -> Result<RemainingCounts> {
        let count = |table: &str| -> Result<i64> {
            Ok(self.conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM {table} WHERE key_version = ?1
                       AND id NOT IN (SELECT row_id FROM key_rotation_failures WHERE table_name = ?2 AND from_version = ?1)"
                ),
                params![from_version, table],
                |row| row.get(0),
            )?)
        };
        let mut counts = RemainingCounts {
            configuration_versions: count(DB_COLUMNS[0].table)?,
            vault_secrets: count(DB_COLUMNS[1].table)?,
            standalone_credentials: count(DB_COLUMNS[2].table)?,
            firmware_versions: count(FIRMWARE_TABLE)?,
            total: 0,
        };
        counts.total = counts.configuration_versions + counts.vault_secrets
            + counts.standalone_credentials + counts.firmware_versions;
        Ok(counts)
    }

    pub fn failures(&self, from_version: u32) -> Result<Vec<KeyRotationFailure>> {
        Ok(self.conn
            .prepare(
                "SELECT id, table_name, row_id, from_version, error, COALESCE(attempted_at, '')
                 FROM key_rotation_failures WHERE from_version = ?1 ORDER BY id",
            )?
            .query_map([from_version], |row| {
                Ok(KeyRotationFailure {
                    id: row.get(0)?,
                    table_name: row.get(1)?,
                    row_id: row.get(2)?,
                    from_version: row.get(3)?,
                    error: row.get(4)?,
                    attempted_at: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn failure_count(&self, from_version: u32) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM key_rotation_failures WHERE from_version = ?1",
            [from_version],
            |row| row.get(0),
        )?)
    }

    pub fn progress(&self, keys: &RotationKeys, result: BatchResult) -> Result<KeyRotationProgress> {
        let remaining = self.remaining(keys.from_version())?;
        Ok(KeyRotationProgress {
            from_version: keys.from_version(),
            to_version: keys.to_version(),
            processed: result.processed,
            failed: result.failed,
            done: remaining.total == 0,
            remaining,
            failure_count: self.failure_count(keys.from_version())?,
            finalized: false,
        })
    }

    /// Retires the old key once nothing remains under it. Refuses while rows
    /// remain, and while failures exist unless `discard_unreadable`: those
    /// rows and files stay as they are, unreadable for good.
    pub fn finalize(&self, admin_id: i64, admin_username: &str, discard_unreadable: bool) -> Result<FinalizeOutcome> {
        let store = self.key_store();
        let from_version = store.retiring_key_version()?
            .ok_or_else(|| anyhow!("No key rotation is in progress"))?;
        let active_version = store.active_key_version()?
            .ok_or_else(|| anyhow!("No data key has been created"))?;

        let remaining = self.remaining(from_version)?;
        if remaining.total > 0 {
            return Err(anyhow!(
                "{} item(s) still need to be re-encrypted; continue the rotation first",
                remaining.total
            ));
        }
        let discarded = self.failure_count(from_version)?;
        if discarded > 0 && !discard_unreadable {
            return Err(anyhow!(
                "{} item(s) could not be decrypted with the old key and were recorded as failures. If you have fixed the cause (for example restored a missing file), retry the failed items. Finalizing discards them: they stay unreadable for good. Confirm to discard them.",
                discarded
            ));
        }

        let tx = self.begin_tx()?;
        self.conn.execute(
            "UPDATE data_keys SET status = ?1, wrapped_by_active = NULL, retired_at = CURRENT_TIMESTAMP
             WHERE version = ?2 AND status = ?3",
            params![DATA_KEY_STATUS_RETIRED, from_version, DATA_KEY_STATUS_RETIRING],
        )?;
        self.conn.execute("DELETE FROM key_rotation_failures WHERE from_version = ?1", [from_version])?;
        SqliteAuditRepository::new(self.conn).log_event(&AuditEventRequest {
            event_type: AuditEventType::DataKeyRotationCompleted,
            user_id: Some(admin_id),
            username: Some(admin_username.to_string()),
            admin_user_id: Some(admin_id),
            admin_username: Some(admin_username.to_string()),
            target_user_id: None,
            target_username: None,
            description: format!(
                "Administrator '{}' finished rotating the data encryption key; version {} retired, version {} active",
                admin_username, from_version, active_version
            ),
            metadata: Some(serde_json::json!({
                "retired_version": from_version,
                "active_version": active_version,
                "discarded_unreadable": discarded,
            }).to_string()),
            ip_address: None,
            user_agent: None,
        })?;
        commit(tx)?;
        info!("Data-key rotation finalized: version {} retired, {} unreadable item(s) discarded", from_version, discarded);

        Ok(FinalizeOutcome { retired_version: from_version, active_version, discarded })
    }

    /// Rotation status. `details` only for administrators.
    pub fn status(&self, is_admin: bool, active_analysis_jobs: usize) -> Result<KeyRotationStatus> {
        let store = self.key_store();
        let active_version = store.active_key_version()?;
        let retiring_version = store.retiring_key_version()?;
        let details = if is_admin {
            let (remaining, failures) = match retiring_version {
                Some(from) => (self.remaining(from)?, self.failures(from)?),
                None => (RemainingCounts::default(), Vec::new()),
            };
            Some(KeyRotationDetails {
                remaining,
                failures,
                users_without_keypair: self.users_without_keypair()?,
                users_without_access: self.users_without_access()?,
                active_analysis_jobs,
            })
        } else {
            None
        };
        Ok(KeyRotationStatus { in_progress: retiring_version.is_some(), active_version, retiring_version, details })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::encryption::INITIAL_KEY_VERSION;
    use crate::users::{CreateUserRequest, SqliteUserRepository, UserRepository, UserRole};

    const ADMIN_PASSWORD: &str = "admin-password";

    struct Fixture {
        db: Database,
        admin: i64,
        k1: DataKey,
        recovery_code: String,
    }

    fn params() -> KdfParams {
        KdfParams::fast_for_tests()
    }

    fn store(conn: &Connection) -> SqliteKeyStoreRepository<'_> {
        SqliteKeyStoreRepository::with_params(conn, params())
    }

    fn rotation(conn: &Connection) -> KeyRotation<'_> {
        KeyRotation::with_params(conn, params()).with_batch_size(1)
    }

    fn create_user(conn: &Connection, name: &str, role: UserRole) -> i64 {
        SqliteUserRepository::new(conn).create_user(CreateUserRequest {
            username: name.to_string(),
            password: "password123!".to_string(),
            role,
        }).unwrap().id
    }

    fn setup() -> Fixture {
        let db = Database::from_connection(Connection::open_in_memory().unwrap()).unwrap();
        let conn = db.get_connection();
        let admin = create_user(conn, "admin", UserRole::Administrator);
        let (k1, recovery_code) = store(conn).create_data_key(admin, ADMIN_PASSWORD).unwrap();
        conn.execute("INSERT INTO assets (id, name, created_by) VALUES (1, 'plc', ?1)", [admin]).unwrap();
        Fixture { db, admin, k1, recovery_code }
    }

    fn same_key(a: &DataKey, b: &DataKey) -> bool {
        let ciphertext = FileEncryption::new(a).encrypt(b"probe").unwrap();
        FileEncryption::new(b).decrypt(&ciphertext).is_ok()
    }

    fn insert_config(conn: &Connection, author: i64, n: usize, key: &DataKey) -> i64 {
        let content = FileEncryption::new(key).encrypt(format!("config {}", n).as_bytes()).unwrap();
        conn.execute(
            "INSERT INTO configuration_versions (asset_id, version_number, file_name, file_content, file_size, content_hash, author, key_version)
             VALUES (1, ?1, 'a.cfg', ?2, 1, 'h', ?3, ?4)",
            params![format!("v{}", n), content, author, key.version()],
        ).unwrap();
        conn.last_insert_rowid()
    }

    fn insert_credential(conn: &Connection, author: i64, n: usize, key: &DataKey) -> i64 {
        let value = FileEncryption::new(key).encrypt_to_base64(format!("secret {}", n).as_bytes()).unwrap();
        conn.execute(
            "INSERT INTO standalone_credentials (name, credential_type, encrypted_data, created_by, key_version)
             VALUES (?1, 'password', ?2, ?3, ?4)",
            params![format!("cred {}", n), value, author, key.version()],
        ).unwrap();
        conn.last_insert_rowid()
    }

    fn config_plaintext(conn: &Connection, id: i64, key: &DataKey) -> Option<Vec<u8>> {
        let content: Vec<u8> = conn.query_row(
            "SELECT file_content FROM configuration_versions WHERE id = ?1", [id], |r| r.get(0),
        ).unwrap();
        FileEncryption::new(key).decrypt(&content).ok()
    }

    fn version_of(conn: &Connection, table: &str, id: i64) -> u32 {
        conn.query_row(&format!("SELECT key_version FROM {} WHERE id = ?1", table), [id], |r| r.get(0)).unwrap()
    }

    #[test]
    fn test_begin_switches_keys_and_reseals_to_active_users() {
        let f = setup();
        let conn = f.db.get_connection();
        let engineer = create_user(conn, "engineer", UserRole::Engineer);
        let legacy = create_user(conn, "legacy", UserRole::Engineer);
        let gone = create_user(conn, "gone", UserRole::Engineer);
        store(conn).set_user_keys(engineer, "engineer-password", &f.k1).unwrap();
        store(conn).store_legacy_user_wrap(legacy, "legacy-password", &f.k1).unwrap();
        store(conn).set_user_keys(gone, "gone-password", &f.k1).unwrap();
        conn.execute("UPDATE users SET is_active = 0 WHERE id = ?1", [gone]).unwrap();

        // The legacy user has no keypair, so begin needs the acknowledgement
        let err = rotation(conn).begin(f.admin, "admin", &f.k1, 0, false).err().unwrap();
        assert!(err.to_string().contains("legacy"), "{}", err);
        let outcome = rotation(conn).begin(f.admin, "admin", &f.k1, 0, true).unwrap();
        let k2 = outcome.new_key;

        assert_eq!(k2.version(), INITIAL_KEY_VERSION + 1);
        assert!(!same_key(&f.k1, &k2));
        assert_eq!(outcome.response.from_version, 1);
        assert_eq!(outcome.response.to_version, 2);
        assert_eq!(outcome.response.users_without_access.iter().map(|u| u.username.as_str()).collect::<Vec<_>>(), ["legacy"]);

        let statuses: Vec<(u32, String, bool)> = conn
            .prepare("SELECT version, status, wrapped_by_active IS NOT NULL FROM data_keys ORDER BY version").unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap()
            .collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(statuses, vec![(1, "retiring".into(), true), (2, "active".into(), false)]);

        // Only v2 public wraps remain, for active users with keypairs
        let wraps: Vec<(i64, u32)> = conn
            .prepare("SELECT user_id, key_version FROM data_key_public_wraps ORDER BY user_id").unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
            .collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(wraps, vec![(f.admin, 2), (engineer, 2)]);
        let legacy_wraps: i64 = conn.query_row("SELECT COUNT(*) FROM data_key_wraps WHERE kind = 'user'", [], |r| r.get(0)).unwrap();
        assert_eq!(legacy_wraps, 0);
        let gone_keypairs: i64 = conn.query_row("SELECT COUNT(*) FROM user_keypairs WHERE user_id = ?1", [gone], |r| r.get(0)).unwrap();
        assert_eq!(gone_keypairs, 0);

        // Users open K2; the legacy user gets nothing; the recovery code moved to K2
        let s = store(conn);
        assert!(same_key(&k2, &s.unlock_for_user(f.admin, ADMIN_PASSWORD).unwrap().unwrap()));
        let engineer_key = s.unlock_for_user(engineer, "engineer-password").unwrap().unwrap();
        assert!(same_key(&k2, &engineer_key));
        assert_eq!(engineer_key.version(), 2);
        assert!(s.unlock_for_user(legacy, "legacy-password").unwrap().is_none());
        assert!(s.unlock_with_recovery_code(&f.recovery_code).is_err());
        let recovered = s.unlock_with_recovery_code(&outcome.response.recovery_code).unwrap();
        assert!(same_key(&k2, &recovered));
        assert_eq!(recovered.version(), 2);

        let started: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_events WHERE event_code = 'SEC_004'", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(started, 1);
        // The recovery code is never printed
        assert!(!format!("{:?}", outcome.response).contains(&outcome.response.recovery_code));
    }

    #[test]
    fn test_retiring_key_is_recovered_from_the_active_key() {
        let f = setup();
        let conn = f.db.get_connection();
        let k2 = rotation(conn).begin(f.admin, "admin", &f.k1, 0, false).unwrap().new_key;

        let keys = rotation(conn).rotation_keys(&k2).unwrap();
        assert!(same_key(&keys.old, &f.k1));
        assert_eq!(keys.from_version(), 1);
        assert_eq!(keys.to_version(), 2);
        // Only the active key opens it
        assert!(rotation(conn).rotation_keys(&f.k1).is_err());
        assert!(rotation(conn).rotation_keys(&DataKey::generate_with_version(2)).is_err());
    }

    #[test]
    fn test_begin_refusals() {
        let f = setup();
        let conn = f.db.get_connection();

        let err = rotation(conn).begin(f.admin, "admin", &f.k1, 2, false).err().unwrap();
        assert!(err.to_string().contains("firmware analysis"), "{}", err);
        let err = rotation(conn).begin(f.admin, "admin", &DataKey::generate_with_version(7), 0, false).err().unwrap();
        assert!(err.to_string().contains("outdated"), "{}", err);
        // Nothing changed
        assert_eq!(store(conn).active_key_version().unwrap(), Some(1));
        assert!(rotation(conn).retiring_version().unwrap().is_none());

        let k2 = rotation(conn).begin(f.admin, "admin", &f.k1, 0, false).unwrap().new_key;
        let err = rotation(conn).begin(f.admin, "admin", &k2, 0, false).err().unwrap();
        assert!(err.to_string().contains("already in progress"), "{}", err);
    }

    #[test]
    fn test_db_batches_reencrypt_and_record_failures() {
        let f = setup();
        let conn = f.db.get_connection();
        let configs: Vec<i64> = (0..3).map(|n| insert_config(conn, f.admin, n, &f.k1)).collect();
        let cred = insert_credential(conn, f.admin, 0, &f.k1);
        // A row that does not decrypt under K1
        let corrupt = insert_config(conn, f.admin, 9, &DataKey::generate());

        let k2 = rotation(conn).begin(f.admin, "admin", &f.k1, 0, false).unwrap().new_key;
        let keys = rotation(conn).rotation_keys(&k2).unwrap();
        assert_eq!(rotation(conn).remaining(1).unwrap().total, 5);

        let mut results = Vec::new();
        while let Some(result) = rotation(conn).process_db_batch(&keys).unwrap() {
            assert_eq!(result.processed + result.failed, 1);
            results.push(result);
        }
        assert_eq!(results.len(), 5);
        assert_eq!(results.iter().map(|r| r.failed).sum::<usize>(), 1);

        for (n, id) in configs.iter().enumerate() {
            assert_eq!(config_plaintext(conn, *id, &k2).unwrap(), format!("config {}", n).into_bytes());
            assert!(config_plaintext(conn, *id, &f.k1).is_none());
            assert_eq!(version_of(conn, "configuration_versions", *id), 2);
        }
        let value: String = conn.query_row("SELECT encrypted_data FROM standalone_credentials WHERE id = ?1", [cred], |r| r.get(0)).unwrap();
        assert_eq!(FileEncryption::new(&k2).decrypt_from_base64(&value).unwrap(), b"secret 0");
        assert_eq!(version_of(conn, "configuration_versions", corrupt), 1);

        let failures = rotation(conn).failures(1).unwrap();
        assert_eq!(failures.len(), 1);
        assert_eq!((failures[0].table_name.as_str(), failures[0].row_id), ("configuration_versions", corrupt));
        assert_eq!(rotation(conn).remaining(1).unwrap().total, 0);

        // Finalize needs the discard confirmation, then retires K1
        let err = rotation(conn).finalize(f.admin, "admin", false).err().unwrap();
        assert!(err.to_string().contains("could not be decrypted"), "{}", err);
        let outcome = rotation(conn).finalize(f.admin, "admin", true).unwrap();
        assert_eq!(outcome, FinalizeOutcome { retired_version: 1, active_version: 2, discarded: 1 });
        let (status, wrapped): (String, Option<Vec<u8>>) = conn.query_row(
            "SELECT status, wrapped_by_active FROM data_keys WHERE version = 1", [], |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!((status.as_str(), wrapped), ("retired", None));
        assert_eq!(rotation(conn).failure_count(1).unwrap(), 0);
        assert!(rotation(conn).retiring_version().unwrap().is_none());
        assert!(rotation(conn).rotation_keys(&k2).is_err());
        let completed: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_events WHERE event_code = 'SEC_005'", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(completed, 1);
    }

    #[test]
    fn test_finalize_refuses_while_rows_remain() {
        let f = setup();
        let conn = f.db.get_connection();
        insert_config(conn, f.admin, 0, &f.k1);
        assert!(rotation(conn).finalize(f.admin, "admin", true).is_err());

        rotation(conn).begin(f.admin, "admin", &f.k1, 0, false).unwrap();
        let err = rotation(conn).finalize(f.admin, "admin", true).err().unwrap();
        assert!(err.to_string().contains("still need"), "{}", err);
    }

    #[test]
    fn test_rerunning_a_batch_is_idempotent() {
        let f = setup();
        let conn = f.db.get_connection();
        let id = insert_config(conn, f.admin, 0, &f.k1);
        let k2 = rotation(conn).begin(f.admin, "admin", &f.k1, 0, false).unwrap().new_key;
        let keys = rotation(conn).rotation_keys(&k2).unwrap();

        // A batch whose transaction never committed (a crash) changes nothing
        {
            let tx = conn.unchecked_transaction().unwrap();
            rotation(conn).process_db_batch(&keys).unwrap();
            drop(tx);
        }
        assert_eq!(version_of(conn, "configuration_versions", id), 1);
        assert!(config_plaintext(conn, id, &f.k1).is_some());

        assert_eq!(rotation(conn).process_db_batch(&keys).unwrap(), Some(BatchResult { processed: 1, failed: 0 }));
        assert_eq!(rotation(conn).process_db_batch(&keys).unwrap(), None);
        assert_eq!(config_plaintext(conn, id, &k2).unwrap(), b"config 0");
    }

    #[test]
    fn test_status_details_are_admin_only() {
        let f = setup();
        let conn = f.db.get_connection();
        let engineer = create_user(conn, "engineer", UserRole::Engineer);
        insert_config(conn, f.admin, 0, &f.k1);

        let idle = rotation(conn).status(true, 3).unwrap();
        assert!(!idle.in_progress);
        assert_eq!(idle.active_version, Some(1));
        let details = idle.details.unwrap();
        assert_eq!(details.active_analysis_jobs, 3);
        assert_eq!(details.remaining.total, 0);
        assert_eq!(details.users_without_keypair.iter().map(|u| u.id).collect::<Vec<_>>(), [engineer]);
        assert_eq!(details.users_without_access.iter().map(|u| u.id).collect::<Vec<_>>(), [engineer]);

        rotation(conn).begin(f.admin, "admin", &f.k1, 0, true).unwrap();
        let running = rotation(conn).status(true, 0).unwrap();
        assert!(running.in_progress);
        assert_eq!((running.active_version, running.retiring_version), (Some(2), Some(1)));
        assert_eq!(running.details.unwrap().remaining.configuration_versions, 1);

        let engineer_view = rotation(conn).status(false, 0).unwrap();
        assert!(engineer_view.in_progress);
        assert!(engineer_view.details.is_none());
    }

    /// The threat model in docs/data-key-rotation-design.md §1: a copy taken
    /// before rotation stays readable with the old key, but that key opens
    /// nothing in the current database, nor does the old recovery code.
    #[test]
    fn test_old_database_copy_after_rotation() {
        let f = setup();
        let conn = f.db.get_connection();
        let config = insert_config(conn, f.admin, 0, &f.k1);
        let cred = insert_credential(conn, f.admin, 0, &f.k1);

        // An attacker's copy, taken before the rotation
        let copy_file = tempfile::NamedTempFile::new().unwrap();
        conn.backup(rusqlite::MAIN_DB, copy_file.path(), None).unwrap();
        let copy = Connection::open(copy_file.path()).unwrap();

        let k2 = rotation(conn).begin(f.admin, "admin", &f.k1, 0, false).unwrap().new_key;
        let keys = rotation(conn).rotation_keys(&k2).unwrap();
        while rotation(conn).process_db_batch(&keys).unwrap().is_some() {}
        rotation(conn).finalize(f.admin, "admin", false).unwrap();

        // The old password still opens K1 from the copy
        let old_key = store(&copy).unlock_for_user(f.admin, ADMIN_PASSWORD).unwrap().unwrap();
        assert!(same_key(&old_key, &f.k1));
        assert!(store(&copy).unlock_with_recovery_code(&f.recovery_code).is_ok());

        // K1 decrypts no current row
        let contents: Vec<Vec<u8>> = conn.prepare("SELECT file_content FROM configuration_versions").unwrap()
            .query_map([], |r| r.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        let values: Vec<String> = conn.prepare("SELECT encrypted_data FROM standalone_credentials").unwrap()
            .query_map([], |r| r.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        assert!(!contents.is_empty() && !values.is_empty());
        assert!(contents.iter().all(|c| FileEncryption::new(&old_key).decrypt(c).is_err()));
        assert!(values.iter().all(|v| FileEncryption::new(&old_key).decrypt_from_base64(v).is_err()));
        assert_eq!(config_plaintext(conn, config, &k2).unwrap(), b"config 0");
        let value: String = conn.query_row("SELECT encrypted_data FROM standalone_credentials WHERE id = ?1", [cred], |r| r.get(0)).unwrap();
        assert!(FileEncryption::new(&k2).decrypt_from_base64(&value).is_ok());

        // Nothing in the current DB holds K1 any more
        let wrapped: i64 = conn.query_row(
            "SELECT COUNT(*) FROM data_keys WHERE wrapped_by_active IS NOT NULL", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(wrapped, 0);
        // The old recovery code fails on the current DB; the old password opens K2 only
        assert!(store(conn).unlock_with_recovery_code(&f.recovery_code).is_err());
        let current = store(conn).unlock_for_user(f.admin, ADMIN_PASSWORD).unwrap().unwrap();
        assert!(same_key(&current, &k2));
    }

    fn store_config(conn: &Connection, author: i64, key: &DataKey) -> Result<i64> {
        use crate::configurations::{ConfigurationRepository, CreateConfigurationRequest, SqliteConfigurationRepository};
        Ok(SqliteConfigurationRepository::new(conn).store_configuration(CreateConfigurationRequest {
            asset_id: 1, file_name: "a.cfg".into(), file_content: b"config".to_vec(), author, notes: String::new(),
        }, key)?.id)
    }

    /// M3: a command can fetch the key before a rotation begins and write
    /// after it began (or even after it finished). Such data would be under a
    /// key nobody holds any more, and no rotation would ever pick it up. Every
    /// write is checked against the active key when it happens.
    #[test]
    fn test_writes_under_a_replaced_key_are_refused() {
        let f = setup();
        let conn = f.db.get_connection();
        let cred = insert_credential(conn, f.admin, 0, &f.k1);
        assert!(store_config(conn, f.admin, &f.k1).is_ok());

        let k2 = rotation(conn).begin(f.admin, "admin", &f.k1, 0, false).unwrap().new_key;

        // While the rotation runs: the repositories refuse, even with K2
        let err = store_config(conn, f.admin, &f.k1).unwrap_err();
        assert_eq!(err.to_string(), crate::auth::KEY_ROTATION_IN_PROGRESS_ERROR);
        assert!(store_config(conn, f.admin, &k2).is_err());
        assert!(crate::keystore::ensure_key_is_current(conn, &k2).is_err());

        // Writers that bypass the repositories are stopped by the database
        let stale = FileEncryption::new(&f.k1).encrypt_to_base64(b"late").unwrap();
        let err = conn.execute(
            "INSERT INTO standalone_credentials (name, credential_type, encrypted_data, created_by, key_version)
             VALUES ('late', 'password', ?1, ?2, 1)",
            params![stale, f.admin],
        ).unwrap_err();
        assert!(err.to_string().contains("outdated encryption key"), "{}", err);
        assert!(conn.execute(
            "UPDATE standalone_credentials SET encrypted_data = ?1 WHERE id = ?2", params![stale, cred],
        ).is_err());
        // Other columns of a row still at the old version can change
        conn.execute("UPDATE standalone_credentials SET name = 'renamed' WHERE id = ?1", [cred]).unwrap();

        let keys = rotation(conn).rotation_keys(&k2).unwrap();
        while rotation(conn).process_db_batch(&keys).unwrap().is_some() {}
        rotation(conn).finalize(f.admin, "admin", false).unwrap();

        // After the rotation, K1 stays refused and K2 works
        let err = store_config(conn, f.admin, &f.k1).unwrap_err();
        assert_eq!(err.to_string(), crate::keystore::STALE_KEY_ERROR);
        let id = store_config(conn, f.admin, &k2).unwrap();
        assert_eq!(version_of(conn, "configuration_versions", id), 2);
        let late = conn.execute(
            "INSERT INTO vault_entries (asset_id, name, description, created_by) VALUES (1, 'v', '', ?1)", [f.admin],
        ).and_then(|_| conn.execute(
            "INSERT INTO vault_secrets (vault_id, secret_type, label, encrypted_value, key_version) VALUES (?1, 'password', 'pw', ?2, 1)",
            params![conn.last_insert_rowid(), stale],
        ));
        assert!(late.is_err());
    }
}
