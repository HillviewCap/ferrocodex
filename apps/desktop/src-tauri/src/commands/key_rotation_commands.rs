//! Data-key rotation commands (design chunk 5). The engine lives in
//! `keystore::rotation`; these commands add sessions, locking, and file IO.
//!
//! While a rotation is in progress every session's `data_key()` errors. These
//! commands read the key with `rotation_key()` instead.
//!
//! Locking:
//! - Every rotation command (begin, continue, finalize, retry) holds the
//!   process-wide [`KeyRotationLockState`] for its whole run, so two calls (a
//!   double invoke, a retry, or two administrators) never work on the same
//!   firmware file at once.
//! - The DB lock and the session-manager lock are never held together: other
//!   commands take them in the opposite order, so nesting them could deadlock.
//!
//! Crash safety of firmware re-encryption (`PRAGMA synchronous = NORMAL` in
//! WAL mode, so a commit is not durable until the WAL is checkpointed or
//! synced). For one firmware row R at old version v, with stored file F:
//! 1. A crash while the new file is written leaves a uniquely named
//!    `<id>.k<v+1>.enc.<hex>.tmp`; R still points at F, which is intact. The
//!    finalize sweep removes the temporary file; the next `continue` redoes R.
//! 2. A crash after the rename, before the DB commit: `<id>.k<v+1>.enc` exists
//!    under K2, R still points at F. The next `continue` re-encrypts F again
//!    and atomically replaces the new file.
//! 3. A crash (or power cut) after the commit, before it is durable: R points
//!    at the new file, or rolls back to F. F is never deleted at this stage:
//!    it is only recorded in `key_rotation_superseded_files` (in the same
//!    transaction, so the record rolls back with the commit), so either way R
//!    points at a file that exists. A rolled-back R is case 2.
//! 4. If F is missing anyway (deleted by an older version right after the
//!    commit, which then rolled back), `continue` adopts `<id>.k<v+1>.enc`
//!    when it authenticates under K2 and commits R to point at it, instead of
//!    recording a failure.
//! 5. Finalize commits, then checkpoints the WAL. Superseded files and sweep
//!    leftovers are deleted only after a checkpoint that completed (not busy),
//!    when every commit above is durable. A crash before or during the
//!    deletions leaves files for the next finalize (a repeated finalize call
//!    retries the cleanup). A busy checkpoint keeps every file.
//! 6. The sweep never deletes `<id>.k<active>.enc` while R is still at an
//!    older version and that file authenticates under the active key: it is
//!    the only re-encrypted copy, which case 4 adopts.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};
use tracing::{error, info, warn};

use crate::auth::{verify_password, SessionManager, SessionToken};
use crate::database::Database;
use crate::encryption::DataKey;
use crate::firmware::{
    get_firmware_storage_dir, FirmwareFileStorage, FirmwareProbe, FirmwareReencryptError, FirmwareReencrypted,
};
use crate::firmware_analysis::AnalysisQueue;
use crate::keystore::rotation::{
    BatchResult, BeginKeyRotationResponse, FirmwareItem, KeyRotation, KeyRotationFinalized,
    KeyRotationProgress, KeyRotationStatus, RotationKeys, DEFAULT_BATCH_SIZE, FIRMWARE_TABLE,
};
use crate::keystore::{KeyStoreRepository, SqliteKeyStoreRepository};
use crate::users::{SqliteUserRepository, UserRepository, UserRole};
use crate::{DatabaseState, KeyRotationLockState, SessionManagerState};

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

/// The session manager, even if a panic poisoned its lock: the steps that
/// run right after a rotation commits must not be skipped.
fn session_manager_after_commit(sessions: &SessionManagerState) -> std::sync::MutexGuard<'_, SessionManager> {
    sessions.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Firmware-analysis jobs queued or running; each holds a copy of the key.
///
/// This is a preflight only: a job can be queued right after it is read
/// (time of check to time of use). Such a job fails closed: before using its
/// key it checks under the DB lock that the key is the active one and no
/// rotation is in progress (`keystore::ensure_key_is_current`), and the key
/// version triggers refuse any write under the old key.
fn active_analysis_jobs(app: &AppHandle) -> usize {
    app.try_state::<Arc<AnalysisQueue>>().map(|queue| queue.active_jobs()).unwrap_or(0)
}

fn firmware_dir(app: &AppHandle) -> Result<PathBuf, String> {
    get_firmware_storage_dir(app).map_err(|e| format!("Failed to locate firmware storage: {}", e))
}

fn with_db<T>(db_state: &DatabaseState, f: impl FnOnce(&Database) -> Result<T, String>) -> Result<T, String> {
    let db_guard = db_state.lock().map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;
    f(db)
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
    rotation_lock: State<'_, KeyRotationLockState>,
) -> Result<BeginKeyRotationResponse, String> {
    let jobs = active_analysis_jobs(&app);
    begin_rotation(&token, &password, acknowledge_users_without_keys, jobs, &db_state, &session_manager, &rotation_lock).await
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
    rotation_lock: State<'_, KeyRotationLockState>,
) -> Result<KeyRotationProgress, String> {
    let dir = firmware_dir(&app)?;
    continue_rotation(&token, &dir, ContinueOptions::default(), &db_state, &session_manager, &rotation_lock).await
}

/// Clears the recorded failures of the rotation in progress so the next
/// `continue` calls try those items again (after the cause was fixed, for
/// example a missing firmware file restored). Admin only. Returns how many
/// failures were cleared.
#[tauri::command]
pub async fn retry_key_rotation_failures(
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    rotation_lock: State<'_, KeyRotationLockState>,
) -> Result<usize, String> {
    retry_failures(&token, &db_state, &session_manager, &rotation_lock).await
}

/// Retires the old key once nothing remains under it. With failures recorded,
/// `discard_unreadable` must be true: those items stay unreadable for good.
/// Calling it again after the rotation finished is harmless (it retries the
/// cleanup of old firmware files).
#[tauri::command]
pub async fn finalize_data_key_rotation(
    app: AppHandle,
    token: String,
    discard_unreadable: bool,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    rotation_lock: State<'_, KeyRotationLockState>,
) -> Result<KeyRotationFinalized, String> {
    let dir = firmware_dir(&app)?;
    finalize_rotation(&token, &dir, discard_unreadable, &db_state, &session_manager, &rotation_lock).await
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

pub(crate) async fn begin_rotation(
    token: &str,
    password: &str,
    acknowledge_users_without_keys: bool,
    active_analysis_jobs: usize,
    db_state: &DatabaseState,
    session_manager: &SessionManagerState,
    rotation_lock: &KeyRotationLockState,
) -> Result<BeginKeyRotationResponse, String> {
    let _rotation = rotation_lock.lock().await;
    let session = admin_session(session_manager, token)?;
    begin_rotation_as(&session, password, acknowledge_users_without_keys, active_analysis_jobs, db_state, session_manager)
}

/// [`begin_rotation`] for a validated administrator session, with the
/// rotation lock held.
fn begin_rotation_as(
    session: &SessionToken,
    password: &str,
    acknowledge_users_without_keys: bool,
    active_analysis_jobs: usize,
    db_state: &DatabaseState,
    session_manager: &SessionManagerState,
) -> Result<BeginKeyRotationResponse, String> {
    let (outcome, username) = with_db(db_state, |db| {
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
        Ok((outcome, user.username))
    })?;

    // The rotation is committed: the old recovery code no longer works, so
    // from here on the new one is always returned. Problems with the session
    // are reported as warnings instead of failing the call.
    //
    // The DB lock is released before the session lock is taken (lock order).
    // A login in between is handled by the rotation epoch (see
    // `SessionManager::sync_rotation_gate`), and any session it creates with
    // the old key is ended below. Writes under the old key are refused by the
    // database itself from the commit on.
    let mut response = outcome.response;
    {
        let manager = session_manager_after_commit(session_manager);
        manager.set_rotation_in_progress(true);
        if let Err(e) = manager.replace_session_key(&session.token, outcome.new_key) {
            warn!("Could not switch {}'s session to the new data key: {}", username, e);
            response.warnings.push(
                "Your session could not be switched to the new encryption key. Store the new recovery key shown here, then log in again to continue the rotation.".to_string(),
            );
        }
        // Every other session holds the old key
        manager.invalidate_all_except(&session.token);
    }

    info!(
        "Data-key rotation started by {}: version {} -> {}",
        username, response.from_version, response.to_version
    );
    Ok(response)
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
    rotation_lock: &KeyRotationLockState,
) -> Result<KeyRotationProgress, String> {
    let _rotation = rotation_lock.lock().await;
    let session = admin_session(session_manager, token)?;
    let active_key = session.rotation_key()?;

    let step = with_db(db_state, |db| {
        let rotation = KeyRotation::new(db.get_connection()).with_batch_size(options.batch_size);
        let keys = rotation.rotation_keys(&active_key).map_err(|e| e.to_string())?;
        Ok(match rotation.process_db_batch(&keys).map_err(|e| format!("Failed to re-encrypt data: {}", e))? {
            Some(result) => Step::Progress(rotation.progress(&keys, result).map_err(|e| e.to_string())?),
            None => match rotation.next_firmware(keys.from_version()).map_err(|e| e.to_string())? {
                Some(item) => Step::Firmware(keys, item),
                // Nothing left to do
                None => Step::Progress(rotation.progress(&keys, BatchResult::default()).map_err(|e| e.to_string())?),
            },
        })
    })?;

    let mut progress = match step {
        Step::Progress(progress) => progress,
        Step::Firmware(keys, item) => rotate_firmware_file(&keys, item, firmware_dir, options, db_state).await?,
    };

    // Finished cleanly: finalize now. With failures, the administrator must
    // retry them or confirm discarding them through finalize_data_key_rotation.
    if progress.done && progress.failure_count == 0 {
        finalize_rotation_locked(&session, firmware_dir, false, db_state, session_manager).await?;
        progress.finalized = true;
    }
    Ok(progress)
}

/// Re-encrypts one firmware file: write the new file outside the DB lock,
/// then point the row at it with a conditional update that also records the
/// old file as superseded. The old file is deleted at finalize, once the
/// update is durable (see the module comment for every crash point).
async fn rotate_firmware_file(
    keys: &RotationKeys,
    item: FirmwareItem,
    firmware_dir: &Path,
    options: ContinueOptions,
    db_state: &DatabaseState,
) -> Result<KeyRotationProgress, String> {
    let reencrypted = {
        let (dir, path, id) = (firmware_dir.to_path_buf(), item.file_path.clone(), item.id);
        let (old, new) = (keys.old.clone(), keys.new.clone());
        tokio::task::spawn_blocking(move || FirmwareFileStorage::reencrypt_firmware_file(&dir, id, &path, &old, &new))
            .await
            .map_err(|e| format!("Firmware re-encryption task failed: {}", e))?
    };

    #[cfg(test)]
    if options.fail_after_firmware_write && reencrypted.is_ok() {
        return Err("Injected failure after writing the firmware file".to_string());
    }
    #[cfg(not(test))]
    let _ = options;

    let (progress, discard) = with_db(db_state, |db| {
        let rotation = KeyRotation::new(db.get_connection());
        let mut result = BatchResult::default();
        let mut discard = None;
        match reencrypted {
            Ok(FirmwareReencrypted::Written(new_path)) => {
                if rotation.commit_firmware(&item, &new_path, keys).map_err(|e| e.to_string())? {
                    result.processed = 1;
                } else if rotation.firmware_path(item.id).map_err(|e| e.to_string())?.as_deref() != Some(new_path.as_str()) {
                    // The row was deleted or changed meanwhile: the copy just
                    // written is referenced by nothing, committed or not
                    discard = Some(new_path);
                }
            }
            Ok(FirmwareReencrypted::Adopted(path)) => {
                info!("Adopted the already re-encrypted file of firmware {}", item.id);
                if rotation.commit_firmware(&item, &path, keys).map_err(|e| e.to_string())? {
                    result.processed = 1;
                }
            }
            Err(FirmwareReencryptError::Unreadable(e)) | Err(FirmwareReencryptError::Rejected(e)) => {
                rotation.record_failure(FIRMWARE_TABLE, item.id, keys.from_version(), &e)
                    .map_err(|e| e.to_string())?;
                result.failed = 1;
            }
            Err(FirmwareReencryptError::Io(e)) => {
                // Not a failure of the data: nothing was changed, so the
                // administrator fixes the cause and continues
                error!("Failed to re-encrypt firmware {}: {}", item.id, e);
                return Err(format!("Failed to re-encrypt firmware {}: {}. Fix the problem and continue the rotation.", item.id, e));
            }
        }
        Ok((rotation.progress(keys, result).map_err(|e| e.to_string())?, discard))
    })?;

    if let Some(path) = discard {
        if let Err(e) = FirmwareFileStorage::delete_superseded_file(firmware_dir, &path) {
            // The finalize sweep removes it
            warn!("Failed to delete an unused re-encrypted firmware file: {}", e);
        }
    }
    Ok(progress)
}

pub(crate) async fn retry_failures(
    token: &str,
    db_state: &DatabaseState,
    session_manager: &SessionManagerState,
    rotation_lock: &KeyRotationLockState,
) -> Result<usize, String> {
    let _rotation = rotation_lock.lock().await;
    let session = admin_session(session_manager, token)?;
    let active_key = session.rotation_key()?;
    let cleared = with_db(db_state, |db| {
        let rotation = KeyRotation::new(db.get_connection());
        let keys = rotation.rotation_keys(&active_key).map_err(|e| e.to_string())?;
        rotation.clear_failures(keys.from_version()).map_err(|e| format!("Failed to clear rotation failures: {}", e))
    })?;
    info!("{} cleared {} key rotation failure(s) to retry them", session.username, cleared);
    Ok(cleared)
}

pub(crate) async fn finalize_rotation(
    token: &str,
    firmware_dir: &Path,
    discard_unreadable: bool,
    db_state: &DatabaseState,
    session_manager: &SessionManagerState,
    rotation_lock: &KeyRotationLockState,
) -> Result<KeyRotationFinalized, String> {
    let _rotation = rotation_lock.lock().await;
    let session = admin_session(session_manager, token)?;
    // Only an administrator holding the active key may finish a rotation
    session.rotation_key()?;
    finalize_rotation_locked(&session, firmware_dir, discard_unreadable, db_state, session_manager).await
}

/// What finalize found before changing anything.
enum FinalizeState {
    /// No rotation in progress, and the session holds the active key.
    AlreadyFinalized { retired_version: u32, active_version: u32 },
    /// Failed firmware files to re-check before discarding.
    InProgress { keys: RotationKeys, failed_firmware: Vec<FirmwareItem> },
}

/// Finalizes with the rotation lock held.
async fn finalize_rotation_locked(
    session: &SessionToken,
    firmware_dir: &Path,
    discard_unreadable: bool,
    db_state: &DatabaseState,
    session_manager: &SessionManagerState,
) -> Result<KeyRotationFinalized, String> {
    let active_key = session.rotation_key()?;

    let state = with_db(db_state, |db| {
        let conn = db.get_connection();
        let rotation = KeyRotation::new(conn);
        let store = SqliteKeyStoreRepository::new(conn);
        if store.retiring_key_version().map_err(|e| e.to_string())?.is_none() {
            // A second finalize (a double invoke, or after the automatic one)
            let active_version = store.active_key_version().map_err(|e| e.to_string())?
                .ok_or_else(|| "No data key has been created".to_string())?;
            if active_key.version() != active_version {
                return Err(crate::keystore::STALE_KEY_ERROR.to_string());
            }
            let retired_version = rotation.latest_retired_version().map_err(|e| e.to_string())?
                .ok_or_else(|| "No key rotation is in progress".to_string())?;
            return Ok(FinalizeState::AlreadyFinalized { retired_version, active_version });
        }
        // Proves the session holds the active key that opens the retiring one
        let keys = rotation.rotation_keys(&active_key).map_err(|e| e.to_string())?;
        let mut failed_firmware = Vec::new();
        if discard_unreadable && rotation.failure_count(keys.from_version()).map_err(|e| e.to_string())? > 0 {
            // Discarding destroys the old key: re-check every failure first
            let readable = rotation.failed_rows_now_readable(&keys).map_err(|e| e.to_string())?;
            if !readable.is_empty() {
                return Err(format!(
                    "{} item(s) recorded as unreadable can be decrypted now. Retry the failed items instead of discarding them.",
                    readable.len()
                ));
            }
            failed_firmware = rotation.failed_firmware(keys.from_version()).map_err(|e| e.to_string())?;
        }
        Ok(FinalizeState::InProgress { keys, failed_firmware })
    })?;

    let keys = match state {
        FinalizeState::AlreadyFinalized { retired_version, active_version } => {
            let (orphaned_files_removed, warnings) = cleanup_after_rotation(firmware_dir, db_state, &active_key);
            return Ok(KeyRotationFinalized {
                retired_version,
                active_version,
                discarded: 0,
                orphaned_files_removed,
                already_finalized: true,
                warnings,
            });
        }
        FinalizeState::InProgress { keys, failed_firmware } => {
            if !failed_firmware.is_empty() {
                ensure_firmware_failures_unreadable(&keys, failed_firmware, firmware_dir).await?;
            }
            keys
        }
    };

    let outcome = with_db(db_state, |db| {
        KeyRotation::new(db.get_connection())
            .finalize(session.user_id, &session.username, discard_unreadable)
            .map_err(|e| e.to_string())
    })?;

    // Delete replaced and leftover files, once the WAL checkpoint has made
    // every commit durable
    let (orphaned_files_removed, warnings) = cleanup_after_rotation(firmware_dir, db_state, &keys.new);

    // DB lock released before the session lock (lock order)
    session_manager_after_commit(session_manager).set_rotation_in_progress(false);

    info!(
        "Data-key rotation finalized by {}: version {} retired, {} leftover firmware file(s) removed",
        session.username, outcome.retired_version, orphaned_files_removed
    );
    Ok(KeyRotationFinalized {
        retired_version: outcome.retired_version,
        active_version: outcome.active_version,
        discarded: outcome.discarded,
        orphaned_files_removed,
        already_finalized: false,
        warnings,
    })
}

/// Refuses to discard while any firmware file recorded as a failure is
/// readable now (under the old key, or already re-encrypted), or can't be
/// checked. The files are read outside the DB lock; the rotation lock keeps
/// other rotation commands out meanwhile.
async fn ensure_firmware_failures_unreadable(
    keys: &RotationKeys,
    failed: Vec<FirmwareItem>,
    firmware_dir: &Path,
) -> Result<(), String> {
    let (dir, old, new) = (firmware_dir.to_path_buf(), keys.old.clone(), keys.new.clone());
    let (readable, unchecked) = tokio::task::spawn_blocking(move || {
        let mut readable = 0usize;
        let mut unchecked = Vec::new();
        for item in failed {
            match FirmwareFileStorage::probe_firmware_file(&dir, item.id, &item.file_path, &old, &new) {
                Ok(FirmwareProbe::Unreadable) => {}
                Ok(FirmwareProbe::ReadableWithOldKey) | Ok(FirmwareProbe::AlreadyReencrypted) => readable += 1,
                Err(e) => unchecked.push(format!("firmware {}: {}", item.id, e)),
            }
        }
        (readable, unchecked)
    })
    .await
    .map_err(|e| format!("Firmware check task failed: {}", e))?;

    if readable > 0 {
        return Err(format!(
            "{} item(s) recorded as unreadable can be decrypted now. Retry the failed items instead of discarding them.",
            readable
        ));
    }
    if !unchecked.is_empty() {
        return Err(format!(
            "Some failed items could not be checked before discarding ({}). Fix the problem and try again.",
            unchecked.join("; ")
        ));
    }
    Ok(())
}

/// Checkpoints the WAL, then deletes firmware files the rotation replaced and
/// leftovers of interrupted steps. Nothing is deleted unless the checkpoint
/// completed: until then a commit pointing away from a file may not be
/// durable. Runs under the DB lock, so no firmware row changes meanwhile.
/// Returns how many files were deleted, and warnings for the response.
fn cleanup_after_rotation(firmware_dir: &Path, db_state: &DatabaseState, active: &DataKey) -> (usize, Vec<String>) {
    let mut warnings = Vec::new();
    let result = with_db(db_state, |db| {
        // Also flushes the deleted key copies and old ciphertext out of the WAL
        if let Err(e) = db.checkpoint_wal() {
            warn!("Kept old firmware files: the WAL checkpoint did not complete: {}", e);
            warnings.push(
                "Old firmware files were kept because the database could not be checkpointed. Finalize again later to remove them.".to_string(),
            );
            return Ok(0);
        }
        let rotation = KeyRotation::new(db.get_connection());
        let records = rotation.firmware_records().map_err(|e| e.to_string())?;
        let in_use: HashSet<&str> = records.iter().map(|r| r.file_path.as_str()).collect();

        let mut removed = 0;
        let mut forget = Vec::new();
        for path in rotation.superseded_files().map_err(|e| e.to_string())? {
            if in_use.contains(path.as_str()) {
                forget.push(path);
                continue;
            }
            match FirmwareFileStorage::delete_superseded_file(firmware_dir, &path) {
                Ok(()) => {
                    removed += 1;
                    forget.push(path);
                }
                Err(e) => warn!("Failed to delete a replaced firmware file: {}", e),
            }
        }
        rotation.forget_superseded(&forget).map_err(|e| e.to_string())?;

        match FirmwareFileStorage::sweep_rotation_leftovers(firmware_dir, &records, active) {
            Ok(swept) => removed += swept,
            Err(e) => warn!("Firmware sweep after key rotation failed: {}", e),
        }
        Ok(removed)
    });
    match result {
        Ok(removed) => (removed, warnings),
        Err(e) => {
            warn!("Firmware cleanup after key rotation failed: {}", e);
            warnings.push(format!("Old firmware files could not be cleaned up: {}", e));
            (0, warnings)
        }
    }
}

pub(crate) fn rotation_status(
    token: &str,
    active_analysis_jobs: usize,
    db_state: &DatabaseState,
    session_manager: &SessionManagerState,
) -> Result<KeyRotationStatus, String> {
    let session = valid_session(session_manager, token)?;
    with_db(db_state, |db| {
        KeyRotation::new(db.get_connection())
            .status(session.role == UserRole::Administrator, active_analysis_jobs)
            .map_err(|e| format!("Failed to read key rotation status: {}", e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{LoginAttemptTracker, LoginResponse, SessionManager, KEY_ROTATION_IN_PROGRESS_ERROR};
    use crate::commands::{admin_reset_user_password, create_admin_account, create_engineer_user, login};
    use crate::database::Database;
    use crate::encryption::{DataKey, FileEncryption};
    use crate::firmware::{CreateFirmwareRequest, FirmwareRepository, SqliteFirmwareRepository};
    use crate::keystore::rotation::KeyRotationFailure;
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
            .manage(KeyRotationLockState::default())
            .build(mock_context(noop_assets()))
            .unwrap()
    }

    fn db(app: &tauri::App<MockRuntime>) -> &DatabaseState {
        app.state::<DatabaseState>().inner()
    }

    fn sessions(app: &tauri::App<MockRuntime>) -> &SessionManagerState {
        app.state::<SessionManagerState>().inner()
    }

    fn lock(app: &tauri::App<MockRuntime>) -> &KeyRotationLockState {
        app.state::<KeyRotationLockState>().inner()
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

    async fn begin(app: &tauri::App<MockRuntime>, token: &str, acknowledge: bool) -> Result<BeginKeyRotationResponse, String> {
        begin_rotation(token, ADMIN_PASSWORD, acknowledge, 0, db(app), sessions(app), lock(app)).await
    }

    async fn finalize(app: &tauri::App<MockRuntime>, token: &str, dir: &Path, discard: bool) -> Result<KeyRotationFinalized, String> {
        finalize_rotation(token, dir, discard, db(app), sessions(app), lock(app)).await
    }

    async fn retry(app: &tauri::App<MockRuntime>, token: &str) -> Result<usize, String> {
        retry_failures(token, db(app), sessions(app), lock(app)).await
    }

    fn batch_of_one() -> ContinueOptions {
        ContinueOptions { batch_size: 1, ..Default::default() }
    }

    async fn step(app: &tauri::App<MockRuntime>, token: &str, dir: &Path) -> Result<KeyRotationProgress, String> {
        continue_rotation(token, dir, batch_of_one(), db(app), sessions(app), lock(app)).await
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
                    asset_id: 1, vendor: None, model: None, version: format!("{}.0", n), notes: None, original_filename: None,
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

        let response = begin(&app, &setup.token, false).await.unwrap();
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

    /// A second finalize after the automatic one (a double invoke from the
    /// UI) is harmless, and changes nothing.
    #[tokio::test]
    async fn test_finalize_after_auto_finalize_is_harmless() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        seed(&app, setup.user.id, &k1, dir);
        begin(&app, &setup.token, false).await.unwrap();
        assert!(run_until_done(&app, &setup.token, dir).await.last().unwrap().finalized);

        let again = finalize(&app, &setup.token, dir, false).await.unwrap();
        assert!(again.already_finalized);
        assert_eq!((again.retired_version, again.active_version, again.discarded), (1, 2, 0));
        assert!(!sessions(&app).lock().unwrap().rotation_in_progress());
        let completed: i64 = with_conn(&app, |conn| {
            conn.query_row("SELECT COUNT(*) FROM audit_events WHERE event_code = 'SEC_005'", [], |r| r.get(0)).unwrap()
        });
        assert_eq!(completed, 1);
    }

    #[tokio::test]
    async fn test_resume_from_fresh_login_after_crash_between_batches() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        let seeded = seed(&app, setup.user.id, &k1, dir);

        begin(&app, &setup.token, false).await.unwrap();
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

        begin(&app, &setup.token, false).await.unwrap();
        let k2 = session(&app, &setup.token).rotation_key().unwrap();
        // Re-encrypt the DB rows, up to the first firmware file
        for _ in 0..5 {
            step(&app, &setup.token, dir).await.unwrap();
        }

        let (first_id, first_path) = seeded.firmware[0].clone();
        let crash = ContinueOptions { batch_size: 1, fail_after_firmware_write: true };
        assert!(continue_rotation(&setup.token, dir, crash, db(&app), sessions(&app), lock(&app)).await.is_err());

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

        begin(&app, &setup.token, false).await.unwrap();
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
        let err = finalize(&app, &setup.token, dir, false).await.unwrap_err();
        assert!(err.contains("could not be decrypted"), "{}", err);
        assert!(sessions(&app).lock().unwrap().rotation_in_progress());

        assert!(failures.iter().all(|f| f.error.contains("could not be decrypted") || f.error.contains("Could not be decrypted")), "{:?}", failures);
        let finalized = finalize(&app, &setup.token, dir, true).await.unwrap();
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

        begin(&app, &setup.token, false).await.unwrap();
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
        assert!(finalize(&app, &during.token, dir, true).await.is_err());

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

        let err = begin(&app, &setup.token, false).await.unwrap_err();
        assert!(err.contains("engineer"), "{}", err);
        assert!(!sessions(&app).lock().unwrap().rotation_in_progress());
        let response = begin(&app, &setup.token, true).await.unwrap();
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

        let err = begin_rotation(&engineer.token, ENGINEER_PASSWORD, true, 0, db(&app), sessions(&app), lock(&app)).await.unwrap_err();
        assert_eq!(err, "Only administrators can rotate the encryption key");
        assert!(!sessions(&app).lock().unwrap().rotation_in_progress());
        assert!(!rotation_status(&setup.token, 0, db(&app), sessions(&app)).unwrap().in_progress);
    }

    #[tokio::test]
    async fn test_begin_refused_while_analysis_jobs_are_active() {
        let app = setup_test_app();
        let setup = setup_admin(&app).await;

        let err = begin_rotation(&setup.token, ADMIN_PASSWORD, false, 1, db(&app), sessions(&app), lock(&app)).await.unwrap_err();
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

        let err = begin_rotation(&setup.token, "Wrong-Pass-000", false, 0, db(&app), sessions(&app), lock(&app)).await.unwrap_err();
        assert_eq!(err, "Incorrect password");
        assert!(!sessions(&app).lock().unwrap().rotation_in_progress());

        let response = begin(&app, &setup.token, false).await.unwrap();
        assert!(sessions(&app).lock().unwrap().validate_session(&other_admin_session.token).unwrap().is_none());
        assert!(sessions(&app).lock().unwrap().validate_session(&setup.token).unwrap().is_some());
        // A second begin is refused while this one runs
        assert!(begin(&app, &setup.token, false).await.unwrap_err().contains("already in progress"));

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

    fn firmware_row(app: &tauri::App<MockRuntime>, id: i64) -> (String, u32) {
        with_conn(app, |conn| {
            conn.query_row("SELECT file_path, key_version FROM firmware_versions WHERE id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
        })
    }

    fn failures(app: &tauri::App<MockRuntime>, token: &str) -> Vec<KeyRotationFailure> {
        rotation_status(token, 0, db(app), sessions(app)).unwrap().details.unwrap().failures
    }

    /// Steps through the 5 seeded DB rows, up to the first firmware file.
    async fn rotate_db_rows(app: &tauri::App<MockRuntime>, token: &str, dir: &Path) {
        for _ in 0..5 {
            let progress = step(app, token, dir).await.unwrap();
            assert_eq!(progress.processed, 1);
        }
    }

    /// H1: recovery imports stored firmware as `<asset>/0.enc`. Rotation used
    /// to record such a readable file as unreadable ("name does not match"),
    /// and discarding then destroyed the only key that opened it.
    #[tokio::test]
    async fn test_firmware_with_a_file_name_not_matching_its_id_is_rotated() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        let imported = with_conn(&app, |conn| {
            let repo = SqliteFirmwareRepository::new(conn);
            let (path, hash, size) = FirmwareFileStorage::store_firmware_file_in(dir, 1, 0, b"imported image", &k1).unwrap();
            repo.create_firmware(CreateFirmwareRequest {
                asset_id: 1, vendor: None, model: None, version: "imported".into(), notes: None, original_filename: None,
            }, setup.user.id, path, hash, size, k1.version()).unwrap().id
        });

        begin(&app, &setup.token, false).await.unwrap();
        let k2 = session(&app, &setup.token).rotation_key().unwrap();
        let reports = run_until_done(&app, &setup.token, dir).await;
        assert!(reports.last().unwrap().finalized);
        assert!(reports.iter().all(|p| p.failed == 0));

        let (path, version) = firmware_row(&app, imported);
        assert_eq!(Path::new(&path), Path::new("1").join(format!("{}.k2.enc", imported)));
        assert_eq!(version, 2);
        assert_eq!(FirmwareFileStorage::read_firmware_file_in(dir, &path, &k2).unwrap(), b"imported image");
        // The replaced file is deleted at finalize
        assert_eq!(files_in(dir), vec![format!("{}.k2.enc", imported)]);
    }

    /// H1: a read error is not a decryption failure. It must not be recorded
    /// (and so later discarded); continue reports it and can be called again.
    #[cfg(unix)]
    #[tokio::test]
    async fn test_firmware_read_error_is_retryable_not_recorded() {
        use std::os::unix::fs::PermissionsExt;
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        let seeded = seed(&app, setup.user.id, &k1, dir);
        let locked = dir.join(&seeded.firmware[0].1);
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read(&locked).is_ok() {
            return; // running as root: permissions don't apply
        }

        begin(&app, &setup.token, false).await.unwrap();
        let k2 = session(&app, &setup.token).rotation_key().unwrap();
        rotate_db_rows(&app, &setup.token, dir).await;
        let err = step(&app, &setup.token, dir).await.unwrap_err();
        assert!(err.contains("Fix the problem"), "{}", err);
        assert!(failures(&app, &setup.token).is_empty());
        assert_eq!(firmware_row(&app, seeded.firmware[0].0).1, 1);

        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(run_until_done(&app, &setup.token, dir).await.last().unwrap().finalized);
        assert_readable_only_with(&app, &seeded, dir, &k2, &k1);
    }

    /// H1: failures can be retried once their cause is fixed.
    #[tokio::test]
    async fn test_retry_failed_items_after_fixing_them() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        let seeded = seed(&app, setup.user.id, &k1, dir);
        // One firmware file is missing (for example on a disconnected share)
        let missing = dir.join(&seeded.firmware[1].1);
        let saved = std::fs::read(&missing).unwrap();
        std::fs::remove_file(&missing).unwrap();

        begin(&app, &setup.token, false).await.unwrap();
        let k2 = session(&app, &setup.token).rotation_key().unwrap();
        let last = run_until_done(&app, &setup.token, dir).await.pop().unwrap();
        assert!(last.done && !last.finalized && last.failure_count == 1);
        assert!(failures(&app, &setup.token)[0].error.contains("missing"));

        // Retrying before fixing it just records the failure again
        assert_eq!(retry(&app, &setup.token).await.unwrap(), 1);
        let again = run_until_done(&app, &setup.token, dir).await.pop().unwrap();
        assert_eq!((again.failed, again.failure_count, again.finalized), (1, 1, false));

        std::fs::write(&missing, saved).unwrap();
        assert_eq!(retry(&app, &setup.token).await.unwrap(), 1);
        let fixed = run_until_done(&app, &setup.token, dir).await.pop().unwrap();
        assert!(fixed.finalized && fixed.failure_count == 0);
        assert_readable_only_with(&app, &seeded, dir, &k2, &k1);

        // Nothing to retry once the rotation has finished
        assert!(retry(&app, &setup.token).await.is_err());
    }

    /// H1: discarding destroys the old key, so it is refused while any item
    /// recorded as a failure can be decrypted now.
    #[tokio::test]
    async fn test_discard_is_refused_while_a_failed_item_is_readable() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        let seeded = seed(&app, setup.user.id, &k1, dir);
        let firmware_file = dir.join(&seeded.firmware[1].1);
        let good_firmware = std::fs::read(&firmware_file).unwrap();
        std::fs::write(&firmware_file, b"a temporarily corrupted copy of the firmware").unwrap();
        let bad_config = seeded.configs[0];
        let good_config: Vec<u8> = with_conn(&app, |conn| {
            let good: Vec<u8> = conn.query_row("SELECT file_content FROM configuration_versions WHERE id = ?1", [bad_config], |r| r.get(0)).unwrap();
            let foreign = FileEncryption::new(&DataKey::generate()).encrypt(b"lost").unwrap();
            conn.execute("UPDATE configuration_versions SET file_content = ?1 WHERE id = ?2", params![foreign, bad_config]).unwrap();
            good
        });

        begin(&app, &setup.token, false).await.unwrap();
        let last = run_until_done(&app, &setup.token, dir).await.pop().unwrap();
        assert_eq!(last.failure_count, 2);

        // The firmware file is restored: discard is refused
        std::fs::write(&firmware_file, &good_firmware).unwrap();
        let err = finalize(&app, &setup.token, dir, true).await.unwrap_err();
        assert!(err.contains("Retry"), "{}", err);
        // So is it for a DB row that decrypts again
        std::fs::write(&firmware_file, b"a temporarily corrupted copy of the firmware").unwrap();
        with_conn(&app, |conn| {
            // Bypass the write-time key check, as a restore from a backup would
            conn.execute_batch("DROP TRIGGER key_version_guard_update_configuration_versions;").unwrap();
            conn.execute("UPDATE configuration_versions SET file_content = ?1 WHERE id = ?2", params![good_config, bad_config]).unwrap();
            crate::keystore::install_key_version_guards(conn).unwrap();
        });
        let err = finalize(&app, &setup.token, dir, true).await.unwrap_err();
        assert!(err.contains("Retry"), "{}", err);

        // Nothing was discarded: the old key is still recoverable
        assert!(sessions(&app).lock().unwrap().rotation_in_progress());
        let wrapped: bool = with_conn(&app, |conn| {
            conn.query_row("SELECT wrapped_by_active IS NOT NULL FROM data_keys WHERE version = 1 AND status = 'retiring'", [], |r| r.get(0)).unwrap()
        });
        assert!(wrapped);

        // Once the config is corrupt again, both are unreadable and discard works
        with_conn(&app, |conn| {
            conn.execute_batch("DROP TRIGGER key_version_guard_update_configuration_versions;").unwrap();
            let foreign = FileEncryption::new(&DataKey::generate()).encrypt(b"lost").unwrap();
            conn.execute("UPDATE configuration_versions SET file_content = ?1 WHERE id = ?2", params![foreign, bad_config]).unwrap();
            crate::keystore::install_key_version_guards(conn).unwrap();
        });
        let finalized = finalize(&app, &setup.token, dir, true).await.unwrap();
        assert_eq!(finalized.discarded, 2);
    }

    /// M1: two continue calls at once (React StrictMode, a retry, or two
    /// administrators) must not work on the same firmware file.
    #[tokio::test]
    async fn test_concurrent_continue_calls_are_serialized() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        let seeded = seed(&app, setup.user.id, &k1, dir);
        begin(&app, &setup.token, false).await.unwrap();
        let k2 = session(&app, &setup.token).rotation_key().unwrap();
        rotate_db_rows(&app, &setup.token, dir).await;

        let (first, second) = tokio::join!(step(&app, &setup.token, dir), step(&app, &setup.token, dir));
        let (first, second) = (first.unwrap(), second.unwrap());
        // Each call re-encrypted its own file; the second also finalized
        assert_eq!((first.processed, second.processed), (1, 1));
        assert!(second.finalized);
        assert_readable_only_with(&app, &seeded, dir, &k2, &k1);
    }

    /// M2: with `synchronous = NORMAL`, a commit can roll back at a power
    /// cut. The old firmware file must still exist then, so it is only
    /// deleted at finalize.
    #[tokio::test]
    async fn test_old_firmware_file_survives_a_lost_commit() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        let seeded = seed(&app, setup.user.id, &k1, dir);
        begin(&app, &setup.token, false).await.unwrap();
        let k2 = session(&app, &setup.token).rotation_key().unwrap();
        rotate_db_rows(&app, &setup.token, dir).await;

        let (id, old_path) = seeded.firmware[0].clone();
        assert_eq!(step(&app, &setup.token, dir).await.unwrap().processed, 1);
        assert_eq!(firmware_row(&app, id).1, 2);
        // Committed, but the old file is kept until finalize
        assert!(dir.join(&old_path).exists());

        // Power cut: the commit was not durable and rolls back
        restart(&app);
        with_conn(&app, |conn| {
            conn.execute_batch("DROP TRIGGER key_version_guard_update_firmware_versions;").unwrap();
            conn.execute("UPDATE firmware_versions SET file_path = ?1, key_version = 1 WHERE id = ?2", params![old_path, id]).unwrap();
            conn.execute("DELETE FROM key_rotation_superseded_files", []).unwrap();
            crate::keystore::install_key_version_guards(conn).unwrap();
        });

        let relogin = login_as(&app, "admin", ADMIN_PASSWORD).await.unwrap();
        let reports = run_until_done(&app, &relogin.token, dir).await;
        assert!(reports.iter().all(|p| p.failed == 0));
        assert!(reports.last().unwrap().finalized);
        assert_readable_only_with(&app, &seeded, dir, &k2, &k1);
        let expected: Vec<String> = seeded.firmware.iter().map(|(id, _)| format!("{}.k2.enc", id)).collect();
        assert_eq!(files_in(dir), expected);
    }

    /// M2: if the old file is gone (older versions deleted it right after a
    /// commit that then rolled back), the re-encrypted copy is adopted.
    #[tokio::test]
    async fn test_missing_old_file_adopts_the_reencrypted_copy() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        let seeded = seed(&app, setup.user.id, &k1, dir);
        begin(&app, &setup.token, false).await.unwrap();
        let k2 = session(&app, &setup.token).rotation_key().unwrap();
        rotate_db_rows(&app, &setup.token, dir).await;

        // The new file was written, the DB update was lost, the old file deleted
        let (id, old_path) = seeded.firmware[0].clone();
        let crash = ContinueOptions { batch_size: 1, fail_after_firmware_write: true };
        assert!(continue_rotation(&setup.token, dir, crash, db(&app), sessions(&app), lock(&app)).await.is_err());
        std::fs::remove_file(dir.join(&old_path)).unwrap();

        let progress = step(&app, &setup.token, dir).await.unwrap();
        assert_eq!((progress.processed, progress.failed), (1, 0));
        let (path, version) = firmware_row(&app, id);
        assert_eq!((Path::new(&path).to_path_buf(), version), (Path::new("1").join(format!("{}.k2.enc", id)), 2));

        assert!(run_until_done(&app, &setup.token, dir).await.last().unwrap().finalized);
        assert_readable_only_with(&app, &seeded, dir, &k2, &k1);
    }

    /// L4/M2: when the WAL checkpoint is busy, commits may not be durable, so
    /// finalize keeps the old firmware files; a later finalize removes them.
    #[tokio::test]
    async fn test_finalize_keeps_old_files_while_the_checkpoint_is_busy() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let k1 = session(&app, &setup.token).data_key().unwrap();
        let seeded = seed(&app, setup.user.id, &k1, dir);
        db(&app).lock().unwrap().as_ref().unwrap().set_checkpoint_busy_for_tests(true);

        begin(&app, &setup.token, false).await.unwrap();
        assert!(run_until_done(&app, &setup.token, dir).await.last().unwrap().finalized);
        for (_, old_path) in &seeded.firmware {
            assert!(dir.join(old_path).exists(), "{} was deleted before a checkpoint", old_path);
        }
        let busy = finalize(&app, &setup.token, dir, false).await.unwrap();
        assert!(busy.already_finalized && busy.orphaned_files_removed == 0);
        assert!(!busy.warnings.is_empty());

        db(&app).lock().unwrap().as_ref().unwrap().set_checkpoint_busy_for_tests(false);
        let cleaned = finalize(&app, &setup.token, dir, false).await.unwrap();
        assert!(cleaned.already_finalized && cleaned.warnings.is_empty());
        assert_eq!(cleaned.orphaned_files_removed, 2);
        let expected: Vec<String> = seeded.firmware.iter().map(|(id, _)| format!("{}.k2.enc", id)).collect();
        assert_eq!(files_in(dir), expected);
    }

    /// L5: once begin has committed, the old recovery code is gone, so the new
    /// one must be returned even if switching the session to the new key fails.
    #[tokio::test]
    async fn test_begin_returns_the_recovery_code_when_the_session_cannot_be_switched() {
        let app = setup_test_app();
        let temp = tempfile::TempDir::new().unwrap();
        let dir = temp.path();
        let setup = setup_admin(&app).await;
        let admin = session(&app, &setup.token);
        // The session ends after it was validated (expiry, or another tab logging out)
        sessions(&app).lock().unwrap().invalidate_session(&setup.token).unwrap();

        let response = begin_rotation_as(&admin, ADMIN_PASSWORD, false, 0, db(&app), sessions(&app)).unwrap();
        assert_eq!(response.warnings.len(), 1, "{:?}", response.warnings);
        assert!(sessions(&app).lock().unwrap().rotation_in_progress());
        with_conn(&app, |conn| {
            let recovered = SqliteKeyStoreRepository::new(conn).unlock_with_recovery_code(&response.recovery_code).unwrap();
            assert_eq!(recovered.version(), 2);
        });

        // Logging in again resumes the rotation
        let relogin = login_as(&app, "admin", ADMIN_PASSWORD).await.unwrap();
        assert!(run_until_done(&app, &relogin.token, dir).await.last().unwrap().finalized);
    }

    /// L1: login must never hold the DB lock while it waits for the session
    /// lock; commands that take them in the other order would deadlock.
    #[test]
    fn test_login_never_holds_the_db_lock_while_waiting_for_the_session_lock() {
        use std::time::Instant;
        let app = setup_test_app();
        tauri::async_runtime::block_on(setup_admin(&app));
        let handle = app.handle().clone();

        let held = sessions(&app).lock().unwrap();
        let login_thread = std::thread::spawn(move || {
            tauri::async_runtime::block_on(login(
                "admin".to_string(), ADMIN_PASSWORD.to_string(), handle.state(), handle.state(), handle.state(),
            ))
        });

        // Login is now blocked on the session lock. The DB lock must not stay
        // taken meanwhile (password checks may hold it briefly).
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut locked_since: Option<Instant> = None;
        while Instant::now() < deadline {
            match db(&app).try_lock() {
                Ok(_) => locked_since = None,
                Err(_) => {
                    let since = *locked_since.get_or_insert_with(Instant::now);
                    assert!(
                        since.elapsed() < Duration::from_secs(3),
                        "login held the DB lock while waiting for the session lock"
                    );
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        drop(held);
        assert!(login_thread.join().unwrap().is_ok());
    }
}
