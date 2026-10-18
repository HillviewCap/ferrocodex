//! Data-key rotation commands (design chunk 5). The engine lives in
//! `keystore::rotation`; these commands add sessions, locking, and file IO.
//!
//! While a rotation is in progress every session's `data_key()` errors. These
//! commands read the key with `rotation_key()` instead.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};
use tracing::{error, info, warn};

use crate::auth::{verify_password, SessionToken};
use crate::firmware::{get_firmware_storage_dir, FirmwareFileStorage, FirmwareReencryptError};
use crate::firmware_analysis::AnalysisQueue;
use crate::keystore::rotation::{
    BatchResult, BeginKeyRotationResponse, FirmwareItem, KeyRotation, KeyRotationFinalized,
    KeyRotationProgress, KeyRotationStatus, RotationKeys, DEFAULT_BATCH_SIZE, FIRMWARE_TABLE,
};
use crate::keystore::{KeyStoreRepository, SqliteKeyStoreRepository};
use crate::users::{SqliteUserRepository, UserRepository, UserRole};
use crate::{DatabaseState, SessionManagerState};

fn valid_session(sessions: &SessionManagerState, token: &str) -> Result<SessionToken, String> {
    let manager = sessions.lock().map_err(|_| "Failed to acquire session manager lock".to_string())?;
    match manager.validate_session(token) {
        Ok(Some(session)) => Ok(session),
        Ok(None) => Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            Err("Session validation error".to_string())
        }
    }
}

fn admin_session(sessions: &SessionManagerState, token: &str) -> Result<SessionToken, String> {
    let session = valid_session(sessions, token)?;
    if session.role != UserRole::Administrator {
        warn!("Non-admin user attempted a data-key rotation command: {}", session.username);
        return Err("Only administrators can rotate the encryption key".to_string());
    }
    Ok(session)
}

/// Firmware-analysis jobs queued or running; each holds a copy of the key.
fn active_analysis_jobs(app: &AppHandle) -> usize {
    app.try_state::<Arc<AnalysisQueue>>().map(|queue| queue.active_jobs()).unwrap_or(0)
}

fn firmware_dir(app: &AppHandle) -> Result<PathBuf, String> {
    get_firmware_storage_dir(app).map_err(|e| format!("Failed to locate firmware storage: {}", e))
}

/// Starts rotating the data key. Admin only; the administrator re-enters
/// their password. Returns the new recovery code, shown once.
#[tauri::command]
pub async fn begin_data_key_rotation(
    app: AppHandle,
    token: String,
    password: String,
    acknowledge_users_without_keys: bool,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<BeginKeyRotationResponse, String> {
    let jobs = active_analysis_jobs(&app);
    begin_rotation(&token, &password, acknowledge_users_without_keys, jobs, &db_state, &session_manager)
}

/// Re-encrypts one bounded batch: up to a batch of DB rows, or one firmware
/// file. Call repeatedly until `done`. The call that finds nothing left and no
/// failures also finalizes the rotation (`finalized`).
#[tauri::command]
pub async fn continue_data_key_rotation(
    app: AppHandle,
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<KeyRotationProgress, String> {
    let dir = firmware_dir(&app)?;
    continue_rotation(&token, &dir, ContinueOptions::default(), &db_state, &session_manager).await
}

/// Retires the old key once nothing remains under it. With failures recorded,
/// `discard_unreadable` must be true: those items stay unreadable for good.
#[tauri::command]
pub async fn finalize_data_key_rotation(
    app: AppHandle,
    token: String,
    discard_unreadable: bool,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<KeyRotationFinalized, String> {
    let dir = firmware_dir(&app)?;
    finalize_rotation(&token, &dir, discard_unreadable, &db_state, &session_manager).await
}

/// Rotation status for any signed-in user; `details` only for administrators.
#[tauri::command]
pub async fn get_key_rotation_status(
    app: AppHandle,
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<KeyRotationStatus, String> {
    let jobs = active_analysis_jobs(&app);
    rotation_status(&token, jobs, &db_state, &session_manager)
}

pub(crate) fn begin_rotation(
    token: &str,
    password: &str,
    acknowledge_users_without_keys: bool,
    active_analysis_jobs: usize,
    db_state: &DatabaseState,
    session_manager: &SessionManagerState,
) -> Result<BeginKeyRotationResponse, String> {
    let session = admin_session(session_manager, token)?;

    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;
    let conn = db.get_connection();

    // Re-authenticate: the password must match, and must open the
    // administrator's own copy of the active key
    let user = SqliteUserRepository::new(conn)
        .find_by_id(session.user_id)
        .map_err(|e| format!("Failed to look up account: {}", e))?
        .filter(|user| user.is_active)
        .ok_or_else(|| "Account not found".to_string())?;
    match verify_password(password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => {
            warn!("Wrong password when starting a key rotation: {}", user.username);
            return Err("Incorrect password".to_string());
        }
        Err(e) => {
            error!("Password verification error: {}", e);
            return Err("Authentication error".to_string());
        }
    }
    let unlocked = match SqliteKeyStoreRepository::new(conn).unlock_user(user.id, password) {
        Ok(Some(unlocked)) => unlocked,
        Ok(None) => return Err("Your account holds no copy of the active encryption key".to_string()),
        Err(e) => {
            error!("Failed to open the encryption key of {}: {}", user.username, e);
            return Err("Incorrect password".to_string());
        }
    };
    if unlocked.upgraded {
        super::auth_commands::log_keys_upgraded(db, &user);
    }

    let outcome = KeyRotation::new(conn)
        .begin(user.id, &user.username, &unlocked.key, active_analysis_jobs, acknowledge_users_without_keys)
        .map_err(|e| e.to_string())?;

    // Still under the DB lock, so a concurrent login sees the gate and the
    // retiring key together
    let manager = session_manager.lock().map_err(|_| "Failed to acquire session manager lock".to_string())?;
    manager.set_rotation_in_progress(true);
    manager.replace_session_key(token, outcome.new_key)
        .map_err(|e| format!("Failed to update session: {}", e))?;
    // Every other session holds the old key
    manager.invalidate_all_except(token);

    info!(
        "Data-key rotation started by {}: version {} -> {}",
        user.username, outcome.response.from_version, outcome.response.to_version
    );
    Ok(outcome.response)
}

/// Options for [`continue_rotation`]; the defaults are what the app uses.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ContinueOptions {
    pub batch_size: usize,
    /// Test hook: fail after the re-encrypted firmware file is written but
    /// before the DB points at it, as a crash there would.
    #[cfg(test)]
    pub fail_after_firmware_write: bool,
}

impl Default for ContinueOptions {
    fn default() -> Self {
        Self {
            batch_size: DEFAULT_BATCH_SIZE,
            #[cfg(test)]
            fail_after_firmware_write: false,
        }
    }
}

enum Step {
    Progress(KeyRotationProgress),
    Firmware(RotationKeys, FirmwareItem),
}

pub(crate) async fn continue_rotation(
    token: &str,
    firmware_dir: &Path,
    options: ContinueOptions,
    db_state: &DatabaseState,
    session_manager: &SessionManagerState,
) -> Result<KeyRotationProgress, String> {
    let session = admin_session(session_manager, token)?;
    let active_key = session.rotation_key()?;

    let step = {
        let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock".to_string())?;
        let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;
        let rotation = KeyRotation::new(db.get_connection()).with_batch_size(options.batch_size);
        let keys = rotation.rotation_keys(&active_key).map_err(|e| e.to_string())?;
        match rotation.process_db_batch(&keys).map_err(|e| format!("Failed to re-encrypt data: {}", e))? {
            Some(result) => Step::Progress(rotation.progress(&keys, result).map_err(|e| e.to_string())?),
            None => match rotation.next_firmware(keys.from_version()).map_err(|e| e.to_string())? {
                Some(item) => Step::Firmware(keys, item),
                // Nothing left to do
                None => Step::Progress(rotation.progress(&keys, BatchResult::default()).map_err(|e| e.to_string())?),
            },
        }
    };

    let mut progress = match step {
        Step::Progress(progress) => progress,
        Step::Firmware(keys, item) => rotate_firmware_file(&keys, item, firmware_dir, options, db_state).await?,
    };

    // Finished cleanly: finalize now. With failures, the administrator must
    // confirm discarding them through finalize_data_key_rotation.
    if progress.done && progress.failure_count == 0 {
        finalize_rotation_as(&session, firmware_dir, false, db_state, session_manager).await?;
        progress.finalized = true;
    }
    Ok(progress)
}

/// Re-encrypts one firmware file: write the new file outside the DB lock,
/// then point the row at it with a conditional update, then delete the old
/// file. A crash before the update leaves the row on the old (still valid)
/// file; a crash after it leaves the old file for the finalize sweep.
async fn rotate_firmware_file(
    keys: &RotationKeys,
    item: FirmwareItem,
    firmware_dir: &Path,
    options: ContinueOptions,
    db_state: &DatabaseState,
) -> Result<KeyRotationProgress, String> {
    let written = {
        let (dir, path, id) = (firmware_dir.to_path_buf(), item.file_path.clone(), item.id);
        let (old, new) = (keys.old.clone(), keys.new.clone());
        tokio::task::spawn_blocking(move || FirmwareFileStorage::reencrypt_firmware_file(&dir, id, &path, &old, &new))
            .await
            .map_err(|e| format!("Firmware re-encryption task failed: {}", e))?
    };

    #[cfg(test)]
    if options.fail_after_firmware_write && written.is_ok() {
        return Err("Injected failure after writing the firmware file".to_string());
    }
    #[cfg(not(test))]
    let _ = options;

    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;
    let rotation = KeyRotation::new(db.get_connection());

    let mut result = BatchResult::default();
    let mut superseded = None;
    match written {
        Ok(new_path) => {
            if rotation.commit_firmware(&item, &new_path, keys).map_err(|e| e.to_string())? {
                result.processed = 1;
                superseded = Some(item.file_path.clone());
            } else if rotation.firmware_path(item.id).map_err(|e| e.to_string())?.as_deref() != Some(new_path.as_str()) {
                // The row was deleted or changed meanwhile: drop the copy just written
                superseded = Some(new_path);
            }
        }
        Err(FirmwareReencryptError::Unreadable(e)) => {
            rotation.record_failure(FIRMWARE_TABLE, item.id, keys.from_version(), &e)
                .map_err(|e| e.to_string())?;
            result.failed = 1;
        }
        Err(FirmwareReencryptError::Write(e)) => {
            error!("Failed to write re-encrypted firmware {}: {}", item.id, e);
            return Err(format!("Failed to write re-encrypted firmware: {}", e));
        }
    }
    let progress = rotation.progress(keys, result).map_err(|e| e.to_string())?;
    drop(db_guard);

    if let Some(path) = superseded {
        if let Err(e) = FirmwareFileStorage::delete_firmware_file_in(firmware_dir, &path) {
            // The finalize sweep removes it
            warn!("Failed to delete superseded firmware file: {}", e);
        }
    }
    Ok(progress)
}

pub(crate) async fn finalize_rotation(
    token: &str,
    firmware_dir: &Path,
    discard_unreadable: bool,
    db_state: &DatabaseState,
    session_manager: &SessionManagerState,
) -> Result<KeyRotationFinalized, String> {
    let session = admin_session(session_manager, token)?;
    // Only an administrator holding the active key may finish a rotation
    session.rotation_key()?;
    finalize_rotation_as(&session, firmware_dir, discard_unreadable, db_state, session_manager).await
}

async fn finalize_rotation_as(
    session: &SessionToken,
    firmware_dir: &Path,
    discard_unreadable: bool,
    db_state: &DatabaseState,
    session_manager: &SessionManagerState,
) -> Result<KeyRotationFinalized, String> {
    let active_key = session.rotation_key()?;
    let (outcome, records) = {
        let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock".to_string())?;
        let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;
        let rotation = KeyRotation::new(db.get_connection());
        // Proves the session holds the active key that opens the retiring one
        rotation.rotation_keys(&active_key).map_err(|e| e.to_string())?;
        let outcome = rotation
            .finalize(session.user_id, &session.username, discard_unreadable)
            .map_err(|e| e.to_string())?;
        // Flush the deleted key copies and old ciphertext out of the WAL
        if let Err(e) = db.checkpoint_wal() {
            warn!("Failed to checkpoint the WAL after key rotation: {}", e);
        }
        let records = rotation.firmware_records().map_err(|e| e.to_string())?;
        session_manager
            .lock()
            .map_err(|_| "Failed to acquire session manager lock".to_string())?
            .set_rotation_in_progress(false);
        (outcome, records)
    };

    let dir = firmware_dir.to_path_buf();
    let orphaned_files_removed = match tokio::task::spawn_blocking(move || {
        FirmwareFileStorage::sweep_rotation_leftovers(&dir, &records)
    }).await {
        Ok(Ok(removed)) => removed,
        Ok(Err(e)) => {
            warn!("Firmware sweep after key rotation failed: {}", e);
            0
        }
        Err(e) => {
            warn!("Firmware sweep task failed: {}", e);
            0
        }
    };

    info!(
        "Data-key rotation finalized by {}: version {} retired, {} leftover firmware file(s) removed",
        session.username, outcome.retired_version, orphaned_files_removed
    );
    Ok(KeyRotationFinalized {
        retired_version: outcome.retired_version,
        active_version: outcome.active_version,
        discarded: outcome.discarded,
        orphaned_files_removed,
    })
}

pub(crate) fn rotation_status(
    token: &str,
    active_analysis_jobs: usize,
    db_state: &DatabaseState,
    session_manager: &SessionManagerState,
) -> Result<KeyRotationStatus, String> {
    let session = valid_session(session_manager, token)?;
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;
    KeyRotation::new(db.get_connection())
        .status(session.role == UserRole::Administrator, active_analysis_jobs)
        .map_err(|e| format!("Failed to read key rotation status: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{LoginAttemptTracker, LoginResponse, SessionManager, KEY_ROTATION_IN_PROGRESS_ERROR};
    use crate::commands::{admin_reset_user_password, create_admin_account, create_engineer_user, login};
    use crate::database::Database;
    use crate::encryption::{DataKey, FileEncryption};
    use crate::firmware::{CreateFirmwareRequest, FirmwareRepository, SqliteFirmwareRepository};
    use crate::validation::RateLimiter;
    use crate::{LoginAttemptTrackerState, RateLimiterState};
    use rusqlite::{params, Connection};
    use std::time::Duration;
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};

    const ADMIN_PASSWORD: &str = "Admin-Pass-123";
    const ENGINEER_PASSWORD: &str = "Engineer-Pass-123";
    const RESET_PASSWORD: &str = "Reset-Pass-456";

    fn setup_test_app() -> tauri::App<MockRuntime> {
        mock_builder()
            .manage(DatabaseState::new(Some(Database::new(":memory:".into()).unwrap())))
            .manage(SessionManagerState::new(SessionManager::new()))
            .manage(LoginAttemptTrackerState::new(LoginAttemptTracker::new()))
            .manage(RateLimiterState::new(RateLimiter::new(100, Duration::from_secs(60))))
            .build(mock_context(noop_assets()))
            .unwrap()
    }

    fn db(app: &tauri::App<MockRuntime>) -> &DatabaseState {
        app.state::<DatabaseState>().inner()
    }

    fn sessions(app: &tauri::App<MockRuntime>) -> &SessionManagerState {
        app.state::<SessionManagerState>().inner()
    }

    /// Simulates an app restart: every in-memory session is gone, the DB stays.
    fn restart(app: &tauri::App<MockRuntime>) {
        *sessions(app).lock().unwrap() = SessionManager::new();
    }

    fn with_conn<T>(app: &tauri::App<MockRuntime>, f: impl FnOnce(&Connection) -> T) -> T {
        let guard = db(app).lock().unwrap();
        f(guard.as_ref().unwrap().get_connection())
    }

    fn session(app: &tauri::App<MockRuntime>, token: &str) -> SessionToken {
        sessions(app).lock().unwrap().validate_session(token).unwrap().unwrap()
    }

    fn same_key(a: &DataKey, b: &DataKey) -> bool {
        let ciphertext = FileEncryption::new(a).encrypt(b"probe").unwrap();
        FileEncryption::new(b).decrypt(&ciphertext).is_ok()
    }

    async fn login_as(app: &tauri::App<MockRuntime>, username: &str, password: &str) -> Result<LoginResponse, String> {
        login(username.to_string(), password.to_string(), app.state(), app.state(), app.state()).await
    }

    async fn setup_admin(app: &tauri::App<MockRuntime>) -> LoginResponse {
        let setup = create_admin_account("admin".to_string(), ADMIN_PASSWORD.to_string(), app.state(), app.state())
            .await.unwrap();
        with_conn(app, |conn| {
            conn.execute("INSERT INTO assets (id, name, created_by) VALUES (1, 'plc', ?1)", [setup.user.id]).unwrap();
        });
        setup
    }

    async fn create_engineer(app: &tauri::App<MockRuntime>, admin_token: &str, name: &str) -> i64 {
        create_engineer_user(
            admin_token.to_string(), name.to_string(), ENGINEER_PASSWORD.to_string(),
            app.state(), app.state(), app.state(),
        ).await.unwrap().id
    }

    fn begin(app: &tauri::App<MockRuntime>, token: &str, acknowledge: bool) -> Result<BeginKeyRotationResponse, String> {
        begin_rotation(token, ADMIN_PASSWORD, acknowledge, 0, db(app), sessions(app))
    }

    fn batch_of_one() -> ContinueOptions {
        ContinueOptions { batch_size: 1, ..Default::default() }
    }

    async fn step(app: &tauri::App<MockRuntime>, token: &str, dir: &Path) -> Result<KeyRotationProgress, String> {
        continue_rotation(token, dir, batch_of_one(), db(app), sessions(app)).await
    }

    /// Calls continue until it reports done, returning every progress report.
    async fn run_until_done(app: &tauri::App<MockRuntime>, token: &str, dir: &Path) -> Vec<KeyRotationProgress> {
        let mut reports = Vec::new();
        for _ in 0..100 {
            let progress = step(app, token, dir).await.unwrap();
            let done = progress.done;
            reports.push(progress);
            if done {
                return reports;
            }
        }
        panic!("rotation did not finish");
    }

    /// Encrypted data of every kind, written under `key`.
    struct Seeded {
        configs: Vec<i64>,
        secret: i64,
        credential: i64,
        firmware: Vec<(i64, String)>,
    }

    fn seed(app: &tauri::App<MockRuntime>, author: i64, key: &DataKey, dir: &Path) -> Seeded {
        with_conn(app, |conn| {
            let enc = FileEncryption::new(key);
            let mut configs = Vec::new();
            for n in 0..3 {
                conn.execute(
                    "INSERT INTO configuration_versions (asset_id, version_number, file_name, file_content, file_size, content_hash, author, key_version)
                     VALUES (1, ?1, 'a.cfg', ?2, 1, 'h', ?3, ?4)",
                    params![format!("v{}", n), enc.encrypt(format!("config {}", n).as_bytes()).unwrap(), author, key.version()],
                ).unwrap();
                configs.push(conn.last_insert_rowid());
            }
            conn.execute(
                "INSERT INTO vault_entries (asset_id, name, description, created_by) VALUES (1, 'vault', '', ?1)",
                [author],
            ).unwrap();
            let vault = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO vault_secrets (vault_id, secret_type, label, encrypted_value, key_version) VALUES (?1, 'password', 'pw', ?2, ?3)",
                params![vault, enc.encrypt_to_base64(b"vault secret").unwrap(), key.version()],
            ).unwrap();
            let secret = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO standalone_credentials (name, credential_type, encrypted_data, created_by, key_version)
                 VALUES ('cred', 'password', ?1, ?2, ?3)",
                params![enc.encrypt_to_base64(b"credential").unwrap(), author, key.version()],
            ).unwrap();
            let credential = conn.last_insert_rowid();

            let repo = SqliteFirmwareRepository::new(conn);
            let mut firmware = Vec::new();
            for n in 0..2 {
                let record = repo.create_firmware(CreateFirmwareRequest {
                    asset_id: 1, vendor: None, model: None, version: format!("{}.0", n), notes: None,
                }, author, "pending".into(), "h".into(), 1, key.version()).unwrap();
                let (path, _, _) = FirmwareFileStorage::store_firmware_file_in(
                    dir, 1, record.id, format!("firmware {}", n).as_bytes(), key,
                ).unwrap();
                repo.update_firmware_file_path(record.id, path.clone()).unwrap();
                firmware.push((record.id, path));
            }
            Seeded { configs, secret, credential, firmware }
        })
    }

    /// Asserts every seeded item decrypts under `key` and none under `other`.
    fn assert_readable_only_with(app: &tauri::App<MockRuntime>, seeded: &Seeded, dir: &Path, key: &DataKey, other: &DataKey) {
        with_conn(app, |conn| {
            for (n, id) in seeded.configs.iter().enumerate() {
                let (content, version): (Vec<u8>, u32) = conn.query_row(
                    "SELECT file_content, key_version FROM configuration_versions WHERE id = ?1", [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                ).unwrap();
                assert_eq!(FileEncryption::new(key).decrypt(&content).unwrap(), format!("config {}", n).into_bytes());
                assert!(FileEncryption::new(other).decrypt(&content).is_err());
                assert_eq!(version, key.version());
            }
            for (table, column, id, expected) in [
                ("vault_secrets", "encrypted_value", seeded.secret, b"vault secret".as_slice()),
                ("standalone_credentials", "encrypted_data", seeded.credential, b"credential".as_slice()),
            ] {
                let (value, version): (String, u32) = conn.query_row(
                    &format!("SELECT {}, key_version FROM {} WHERE id = ?1", column, table), [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                ).unwrap();
                assert_eq!(FileEncryption::new(key).decrypt_from_base64(&value).unwrap(), expected);
                assert!(FileEncryption::new(other).decrypt_from_base64(&value).is_err());
                assert_eq!(version, key.version());
            }
            for (n, (id, _)) in seeded.firmware.iter().enumerate() {
                let (path, version): (String, u32) = conn.query_row(
                    "SELECT file_path, key_version FROM firmware_versions WHERE id = ?1", [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                ).unwrap();
                assert_eq!(
                    FirmwareFileStorage::read_firmware_file_in(dir, &path, key).unwrap(),
                    format!("firmware {}", n).into_bytes()
                );
                assert!(FirmwareFileStorage::read_firmware_file_in(dir, &path, other).is_err());
                assert_eq!(version, key.version());
            }
        });
    }

    fn files_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir.join("1")).unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        names
    }

    #[tokio::test]
    async fn test_full_rotation_with_batch_size_one() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        let seeded = seed(&app, setup.user.id, &k1, dir);
        // A stray file that is not a firmware leftover must survive the sweep
        std::fs::write(dir.join("1").join("readme.txt"), b"keep").unwrap();

        let response = begin(&app, &setup.token, false).unwrap();
        assert_eq!((response.from_version, response.to_version), (1, 2));
        assert!(response.users_without_access.is_empty());
        let k2 = session(&app, &setup.token).rotation_key().unwrap();
        assert_eq!(k2.version(), 2);
        assert!(!same_key(&k1, &k2));

        let reports = run_until_done(&app, &setup.token, dir).await;
        // 3 configs + 1 secret + 1 credential + 2 firmware files, one per
        // call; the call that re-encrypts the last item also finalizes
        assert_eq!(reports.len(), 7);
        assert!(reports.iter().all(|p| p.processed == 1 && p.failed == 0));
        assert_eq!(reports[0].remaining.total, 6);
        assert!(reports[..6].iter().all(|p| !p.done && !p.finalized));
        assert!(reports[6].done && reports[6].finalized);
        assert_eq!(reports[6].remaining.total, 0);

        assert_readable_only_with(&app, &seeded, dir, &k2, &k1);
        // The old firmware files are gone; only the re-encrypted ones and the stray file remain
        let expected: Vec<String> = {
            let mut names: Vec<String> = seeded.firmware.iter().map(|(id, _)| format!("{}.k2.enc", id)).collect();
            names.push("readme.txt".to_string());
            names.sort();
            names
        };
        assert_eq!(files_in(dir), expected);

        let status = rotation_status(&setup.token, 0, db(&app), sessions(&app)).unwrap();
        assert!(!status.in_progress);
        assert_eq!((status.active_version, status.retiring_version), (Some(2), None));
        assert!(!sessions(&app).lock().unwrap().rotation_in_progress());
        assert!(session(&app, &setup.token).data_key().is_ok());
        let (completed, retired): (i64, String) = with_conn(&app, |conn| {
            (
                conn.query_row("SELECT COUNT(*) FROM audit_events WHERE event_code = 'SEC_005'", [], |r| r.get(0)).unwrap(),
                conn.query_row("SELECT status FROM data_keys WHERE version = 1", [], |r| r.get(0)).unwrap(),
            )
        });
        assert_eq!((completed, retired.as_str()), (1, "retired"));
        // Nothing more to do
        assert!(step(&app, &setup.token, dir).await.is_err());
    }

    #[tokio::test]
    async fn test_resume_from_fresh_login_after_crash_between_batches() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        let seeded = seed(&app, setup.user.id, &k1, dir);

        begin(&app, &setup.token, false).unwrap();
        let k2 = session(&app, &setup.token).rotation_key().unwrap();
        for _ in 0..2 {
            step(&app, &setup.token, dir).await.unwrap();
        }

        restart(&app);
        assert!(step(&app, &setup.token, dir).await.is_err());

        // A fresh login finds the retiring key, sets the gate, and resumes
        let relogin = login_as(&app, "admin", ADMIN_PASSWORD).await.unwrap();
        assert!(relogin.key_rotation_in_progress);
        assert!(sessions(&app).lock().unwrap().rotation_in_progress());
        let resumed = session(&app, &relogin.token);
        assert_eq!(resumed.data_key().unwrap_err(), KEY_ROTATION_IN_PROGRESS_ERROR);
        assert!(same_key(&resumed.rotation_key().unwrap(), &k2));

        let reports = run_until_done(&app, &relogin.token, dir).await;
        assert!(reports.last().unwrap().finalized);
        assert_readable_only_with(&app, &seeded, dir, &k2, &k1);

        // Once finished, logins report no rotation and the gate is open
        let after = login_as(&app, "admin", ADMIN_PASSWORD).await.unwrap();
        assert!(!after.key_rotation_in_progress);
        assert!(session(&app, &after.token).data_key().is_ok());
    }

    #[tokio::test]
    async fn test_crash_between_firmware_write_and_db_update() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        let seeded = seed(&app, setup.user.id, &k1, dir);

        begin(&app, &setup.token, false).unwrap();
        let k2 = session(&app, &setup.token).rotation_key().unwrap();
        // Re-encrypt the DB rows, up to the first firmware file
        for _ in 0..5 {
            step(&app, &setup.token, dir).await.unwrap();
        }

        let (first_id, first_path) = seeded.firmware[0].clone();
        let crash = ContinueOptions { batch_size: 1, fail_after_firmware_write: true };
        assert!(continue_rotation(&setup.token, dir, crash, db(&app), sessions(&app)).await.is_err());

        // The new file exists but the row still points at the old, valid one
        let (path, version): (String, u32) = with_conn(&app, |conn| {
            conn.query_row("SELECT file_path, key_version FROM firmware_versions WHERE id = ?1", [first_id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
        });
        assert_eq!((path.as_str(), version), (first_path.as_str(), 1));
        assert!(dir.join("1").join(format!("{}.k2.enc", first_id)).exists());
        assert!(FirmwareFileStorage::read_firmware_file_in(dir, &first_path, &k1).is_ok());

        restart(&app);
        let relogin = login_as(&app, "admin", ADMIN_PASSWORD).await.unwrap();
        let reports = run_until_done(&app, &relogin.token, dir).await;
        assert!(reports.last().unwrap().finalized);
        assert_readable_only_with(&app, &seeded, dir, &k2, &k1);
        let expected: Vec<String> = seeded.firmware.iter().map(|(id, _)| format!("{}.k2.enc", id)).collect();
        assert_eq!(files_in(dir), expected);
    }

    #[tokio::test]
    async fn test_corrupt_row_is_recorded_and_finalize_needs_discard() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        let seeded = seed(&app, setup.user.id, &k1, dir);
        // One config and one firmware file that don't decrypt under K1
        let bad_config = seeded.configs[1];
        let (bad_firmware, bad_path) = seeded.firmware[1].clone();
        with_conn(&app, |conn| {
            let foreign = FileEncryption::new(&DataKey::generate()).encrypt(b"lost").unwrap();
            conn.execute("UPDATE configuration_versions SET file_content = ?1 WHERE id = ?2", params![foreign, bad_config]).unwrap();
        });
        std::fs::write(dir.join(&bad_path), b"not a ciphertext of this key").unwrap();

        begin(&app, &setup.token, false).unwrap();
        let reports = run_until_done(&app, &setup.token, dir).await;
        let last = reports.last().unwrap();
        assert!(last.done && !last.finalized);
        assert_eq!(last.failure_count, 2);
        assert_eq!(reports.iter().map(|p| p.failed).sum::<usize>(), 2);

        let status = rotation_status(&setup.token, 0, db(&app), sessions(&app)).unwrap();
        assert!(status.in_progress);
        let failures = status.details.unwrap().failures;
        let failed: Vec<(&str, i64)> = failures.iter().map(|f| (f.table_name.as_str(), f.row_id)).collect();
        assert_eq!(failed, vec![("configuration_versions", bad_config), ("firmware_versions", bad_firmware)]);

        // Further calls don't finalize, and finalize needs the discard flag
        assert!(!step(&app, &setup.token, dir).await.unwrap().finalized);
        let err = finalize_rotation(&setup.token, dir, false, db(&app), sessions(&app)).await.unwrap_err();
        assert!(err.contains("could not be decrypted"), "{}", err);
        assert!(sessions(&app).lock().unwrap().rotation_in_progress());

        let finalized = finalize_rotation(&setup.token, dir, true, db(&app), sessions(&app)).await.unwrap();
        assert_eq!((finalized.retired_version, finalized.active_version, finalized.discarded), (1, 2, 2));
        assert!(!sessions(&app).lock().unwrap().rotation_in_progress());
        // The unreadable firmware keeps its (old) file; the good one was rotated
        assert!(dir.join(&bad_path).exists());
        assert!(dir.join("1").join(format!("{}.k2.enc", seeded.firmware[0].0)).exists());
    }

    #[tokio::test]
    async fn test_engineer_is_gated_during_rotation_and_gets_k2_after() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        create_engineer(&app, &setup.token, "engineer").await;
        let before = login_as(&app, "engineer", ENGINEER_PASSWORD).await.unwrap();
        let k1 = session(&app, &before.token).data_key().unwrap();

        begin(&app, &setup.token, false).unwrap();
        let k2 = session(&app, &setup.token).rotation_key().unwrap();

        // The engineer's old session (holding K1) was ended
        assert!(sessions(&app).lock().unwrap().validate_session(&before.token).unwrap().is_none());
        // Logging in during the rotation opens K2, but encrypted data is gated
        let during = login_as(&app, "engineer", ENGINEER_PASSWORD).await.unwrap();
        assert!(during.key_rotation_in_progress);
        assert_eq!(session(&app, &during.token).data_key().unwrap_err(), KEY_ROTATION_IN_PROGRESS_ERROR);
        // Engineers can see that a rotation runs, but not the details, and can't drive it
        let status = rotation_status(&during.token, 0, db(&app), sessions(&app)).unwrap();
        assert!(status.in_progress && status.details.is_none());
        assert!(step(&app, &during.token, dir).await.is_err());
        assert!(finalize_rotation(&during.token, dir, true, db(&app), sessions(&app)).await.is_err());

        run_until_done(&app, &setup.token, dir).await;
        assert!(session(&app, &during.token).data_key().is_ok());

        let after = login_as(&app, "engineer", ENGINEER_PASSWORD).await.unwrap();
        assert!(!after.key_rotation_in_progress);
        let engineer_key = session(&app, &after.token).data_key().unwrap();
        assert!(same_key(&engineer_key, &k2));
        assert!(!same_key(&engineer_key, &k1));
        assert_eq!(engineer_key.version(), 2);
    }

    #[tokio::test]
    async fn test_user_without_keypair_loses_access_until_admin_reset() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let engineer = create_engineer(&app, &setup.token, "engineer").await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        // Make the engineer a pre-keypair user: only a legacy password wrap
        with_conn(&app, |conn| {
            let store = SqliteKeyStoreRepository::with_params(conn, crate::encryption::KdfParams::fast_for_tests());
            store.revoke_user(engineer).unwrap();
            store.store_legacy_user_wrap(engineer, ENGINEER_PASSWORD, &k1).unwrap();
        });

        let preflight = rotation_status(&setup.token, 0, db(&app), sessions(&app)).unwrap().details.unwrap();
        assert_eq!(preflight.users_without_keypair.iter().map(|u| u.id).collect::<Vec<_>>(), [engineer]);

        let err = begin(&app, &setup.token, false).unwrap_err();
        assert!(err.contains("engineer"), "{}", err);
        assert!(!sessions(&app).lock().unwrap().rotation_in_progress());
        let response = begin(&app, &setup.token, true).unwrap();
        assert_eq!(response.users_without_access.iter().map(|u| u.username.as_str()).collect::<Vec<_>>(), ["engineer"]);
        run_until_done(&app, &setup.token, dir).await;

        // Their password still logs in, but opens no key
        let locked_out = login_as(&app, "engineer", ENGINEER_PASSWORD).await.unwrap();
        assert!(session(&app, &locked_out.token).data_key().is_err());
        assert!(with_conn(&app, |conn| SqliteKeyStoreRepository::new(conn).unlock_for_user(engineer, ENGINEER_PASSWORD).unwrap().is_none()));
        let status = rotation_status(&setup.token, 0, db(&app), sessions(&app)).unwrap().details.unwrap();
        assert_eq!(status.users_without_access.iter().map(|u| u.id).collect::<Vec<_>>(), [engineer]);

        admin_reset_user_password(setup.token.clone(), engineer, RESET_PASSWORD.to_string(), app.state(), app.state())
            .await.unwrap();
        let restored = login_as(&app, "engineer", RESET_PASSWORD).await.unwrap();
        let key = session(&app, &restored.token).data_key().unwrap();
        assert!(same_key(&key, &session(&app, &setup.token).data_key().unwrap()));
        assert_eq!(key.version(), 2);
    }

    #[tokio::test]
    async fn test_engineer_cannot_begin() {
        let app = setup_test_app();
        let setup = setup_admin(&app).await;
        create_engineer(&app, &setup.token, "engineer").await;
        let engineer = login_as(&app, "engineer", ENGINEER_PASSWORD).await.unwrap();

        let err = begin_rotation(&engineer.token, ENGINEER_PASSWORD, true, 0, db(&app), sessions(&app)).unwrap_err();
        assert_eq!(err, "Only administrators can rotate the encryption key");
        assert!(!sessions(&app).lock().unwrap().rotation_in_progress());
        assert!(!rotation_status(&setup.token, 0, db(&app), sessions(&app)).unwrap().in_progress);
    }

    #[tokio::test]
    async fn test_begin_refused_while_analysis_jobs_are_active() {
        let app = setup_test_app();
        let setup = setup_admin(&app).await;

        let err = begin_rotation(&setup.token, ADMIN_PASSWORD, false, 1, db(&app), sessions(&app)).unwrap_err();
        assert!(err.contains("firmware analysis"), "{}", err);
        assert!(!sessions(&app).lock().unwrap().rotation_in_progress());
        assert!(session(&app, &setup.token).data_key().is_ok());
        assert_eq!(rotation_status(&setup.token, 1, db(&app), sessions(&app)).unwrap().details.unwrap().active_analysis_jobs, 1);
    }

    #[tokio::test]
    async fn test_begin_requires_the_admin_password_and_ends_other_sessions() {
        let app = setup_test_app();
        let setup = setup_admin(&app).await;
        let other_admin_session = login_as(&app, "admin", ADMIN_PASSWORD).await.unwrap();
        let old_code = setup.recovery_code.clone().unwrap();

        let err = begin_rotation(&setup.token, "Wrong-Pass-000", false, 0, db(&app), sessions(&app)).unwrap_err();
        assert_eq!(err, "Incorrect password");
        assert!(!sessions(&app).lock().unwrap().rotation_in_progress());

        let response = begin(&app, &setup.token, false).unwrap();
        assert!(sessions(&app).lock().unwrap().validate_session(&other_admin_session.token).unwrap().is_none());
        assert!(sessions(&app).lock().unwrap().validate_session(&setup.token).unwrap().is_some());
        // A second begin is refused while this one runs
        assert!(begin(&app, &setup.token, false).unwrap_err().contains("already in progress"));

        // The new recovery code opens K2; the old one opens nothing
        with_conn(&app, |conn| {
            let store = SqliteKeyStoreRepository::new(conn);
            assert!(store.unlock_with_recovery_code(&old_code).is_err());
            let recovered = store.unlock_with_recovery_code(&response.recovery_code).unwrap();
            assert_eq!(recovered.version(), 2);
        });
        // Replace Recovery Key works during the rotation, with K2
        let replaced = crate::commands::replace_recovery_key(setup.token.clone(), app.state(), app.state()).await.unwrap();
        with_conn(&app, |conn| {
            let recovered = SqliteKeyStoreRepository::new(conn).unlock_with_recovery_code(&replaced).unwrap();
            assert!(same_key(&recovered, &session(&app, &setup.token).rotation_key().unwrap()));
        });
    }
}
