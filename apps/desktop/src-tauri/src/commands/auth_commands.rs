use crate::auth::{SessionManager, LoginAttemptTracker, LoginResponse, hash_password, verify_password};
use crate::encryption::DataKey;
use crate::keystore::{KeyStoreRepository, SqliteKeyStoreRepository};
use crate::users::{CreateUserRequest, User, UserRepository, SqliteUserRepository, UserRole, UserInfo};
use crate::audit::{AuditRepository, SqliteAuditRepository, AuditEventRequest, AuditEventType, create_user_created_event};
use crate::validation::{UsernameValidator, PasswordValidator, InputSanitizer, RateLimiter};
use crate::database::Database;
use std::sync::Mutex;
use tauri::State;
use tracing::{error, info, warn};

type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;
type LoginAttemptTrackerState = Mutex<LoginAttemptTracker>;
type RateLimiterState = Mutex<RateLimiter>;

#[tauri::command]
pub async fn create_admin_account(
    username: String,
    password: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<LoginResponse, String> {
    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    match db_guard.as_ref() {
        Some(db) => {
            let user_repo = SqliteUserRepository::new(db.get_connection());
            
            // Check if admin users already exist
            let has_admins = user_repo.has_admin_users()
                .map_err(|e| format!("Failed to check for admin users: {}", e))?;
            
            if has_admins {
                return Err("Admin account already exists".to_string());
            }

            let request = CreateUserRequest {
                username,
                password: password.clone(),
                role: UserRole::Administrator,
            };

            let tx = db.get_connection().unchecked_transaction()
                .map_err(|e| format!("Failed to start transaction: {}", e))?;
            let user = user_repo.create_user(request)
                .map_err(|e| format!("Failed to create admin account: {}", e))?;
            let (data_key, recovery_code) = SqliteKeyStoreRepository::new(db.get_connection())
                .create_data_key(user.id, &password)
                .map_err(|e| format!("Failed to create encryption key: {}", e))?;
            tx.commit()
                .map_err(|e| format!("Failed to create admin account: {}", e))?;
            // Never hold the DB lock while taking the session lock (lock order)
            drop(db_guard);

            let session_manager = session_manager.lock()
                .map_err(|_| "Failed to acquire session manager lock".to_string())?;
            let session = session_manager.create_session_with_key(&user, Some(data_key))
                .map_err(|e| format!("Failed to create session: {}", e))?;

            info!("Created admin account for user: {}", user.username);

            Ok(LoginResponse {
                token: session.token,
                user: user.into(),
                recovery_code: Some(recovery_code),
                key_rotation_in_progress: false,
            })
        }
        None => Err("Database not initialized".to_string()),
    }
}

#[tauri::command]
pub async fn login(
    username: String,
    password: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
    attempt_tracker: State<'_, LoginAttemptTrackerState>,
) -> Result<LoginResponse, String> {
    // Never hold the DB lock while taking the session lock: other commands
    // take them in the opposite order, and holding both here could deadlock
    // (ABBA). So the database work happens first, under the DB lock alone,
    // and the session is created afterwards under the session lock alone.
    //
    // The rotation state read from the database can go stale in between, if
    // a key rotation begins or finishes. The session manager's rotation epoch
    // detects that, and the database part is then repeated (a rotation that
    // began also changed the key this user opens).
    const ATTEMPTS: usize = 3;
    for _ in 0..ATTEMPTS {
        let epoch = session_manager.lock()
            .map_err(|_| "Failed to acquire session manager lock".to_string())?
            .rotation_epoch();

        let unlocked = {
            let db_guard = db_state.lock()
                .map_err(|_| "Failed to acquire database lock".to_string())?;
            let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;
            authenticate_and_unlock(db, &username, &password, &attempt_tracker)?
        };

        let session_manager = session_manager.lock()
            .map_err(|_| "Failed to acquire session manager lock".to_string())?;
        // A retiring key means a rotation is unfinished, possibly from before
        // a restart: gate encrypted data until an administrator finishes it
        if !session_manager.sync_rotation_gate(epoch, unlocked.key_rotation_in_progress) {
            info!("Encryption key state changed during login of {}; retrying", username);
            continue;
        }
        let session = session_manager.create_session_with_key(&unlocked.user, unlocked.data_key)
            .map_err(|e| format!("Failed to create session: {}", e))?;

        info!("Successful login for user: {}", username);
        return Ok(LoginResponse {
            token: session.token,
            user: unlocked.user.into(),
            recovery_code: unlocked.recovery_code,
            key_rotation_in_progress: unlocked.key_rotation_in_progress,
        });
    }
    Err("The encryption key changed while you were logging in; please try again".to_string())
}

/// What the database part of a login produced.
struct LoginUnlock {
    user: User,
    data_key: Option<DataKey>,
    recovery_code: Option<String>,
    key_rotation_in_progress: bool,
}

/// Checks the lockout and the password, records the attempt, and opens the
/// user's copy of the data key. Call under the DB lock (and not the session lock).
fn authenticate_and_unlock(
    db: &Database,
    username: &str,
    password: &str,
    attempt_tracker: &LoginAttemptTrackerState,
) -> Result<LoginUnlock, String> {
    // Check if account is locked
    let tracker = attempt_tracker.lock()
        .map_err(|_| "Failed to acquire attempt tracker lock".to_string())?;
    if tracker.is_locked(username).map_err(|e| e.to_string())? {
        warn!("Login attempt for locked account: {}", username);
        return Err("Account is temporarily locked due to too many failed attempts".to_string());
    }
    drop(tracker);

    let user_repo = SqliteUserRepository::new(db.get_connection());
    match user_repo.find_by_username(username) {
        Ok(Some(user)) => match verify_password(password, &user.password_hash) {
            Ok(true) => {
                // Successful login
                let tracker = attempt_tracker.lock()
                    .map_err(|_| "Failed to acquire attempt tracker lock".to_string())?;
                tracker.record_successful_attempt(username)
                    .map_err(|e| e.to_string())?;
                drop(tracker);

                let (data_key, recovery_code) = unlock_data_key(db, &user, password)?;
                let key_rotation_in_progress = SqliteKeyStoreRepository::new(db.get_connection())
                    .retiring_key_version()
                    .map_err(|e| format!("Failed to check encryption key state: {}", e))?
                    .is_some();
                Ok(LoginUnlock { user, data_key, recovery_code, key_rotation_in_progress })
            }
            Ok(false) => {
                // Wrong password
                let tracker = attempt_tracker.lock()
                    .map_err(|_| "Failed to acquire attempt tracker lock".to_string())?;
                tracker.record_failed_attempt(username)
                    .map_err(|e| e.to_string())?;
                warn!("Invalid password for user: {}", username);
                Err("Invalid credentials".to_string())
            }
            Err(e) => {
                error!("Password verification error: {}", e);
                Err("Authentication error".to_string())
            }
        },
        Ok(None) => {
            // User not found
            let tracker = attempt_tracker.lock()
                .map_err(|_| "Failed to acquire attempt tracker lock".to_string())?;
            tracker.record_failed_attempt(username)
                .map_err(|e| e.to_string())?;
            warn!("Login attempt for non-existent user: {}", username);
            Err("Invalid credentials".to_string())
        }
        Err(e) => {
            error!("Database error during login: {}", e);
            Err("Authentication error".to_string())
        }
    }
}

/// Unlocks the data key for a user who just proved their password. A user who
/// still has a legacy password wrap is moved to a keypair, which is audited.
/// An administrator logging in to a database created before the key store
/// existed creates the key, and gets the recovery code to record.
fn unlock_data_key(db: &Database, user: &User, password: &str) -> Result<(Option<DataKey>, Option<String>), String> {
    let key_store = SqliteKeyStoreRepository::new(db.get_connection());
    match key_store.unlock_user(user.id, password) {
        Ok(Some(unlocked)) => {
            if unlocked.upgraded {
                log_keys_upgraded(db, user);
            }
            Ok((Some(unlocked.key), None))
        }
        Ok(None) => {
            let has_data_key = key_store.has_data_key()
                .map_err(|e| format!("Failed to check encryption key: {}", e))?;
            if user.role == UserRole::Administrator && !has_data_key {
                let (data_key, recovery_code) = key_store.create_data_key(user.id, password)
                    .map_err(|e| format!("Failed to create encryption key: {}", e))?;
                info!("Created data key on first login of administrator {}", user.username);
                Ok((Some(data_key), Some(recovery_code)))
            } else {
                warn!("User {} has no copy of the data key; encrypted data is unavailable", user.username);
                Ok((None, None))
            }
        }
        Err(e) => {
            error!("Failed to unlock data key for user {}: {}", user.username, e);
            Err("Authentication error".to_string())
        }
    }
}

pub(crate) fn log_keys_upgraded(db: &Database, user: &User) {
    let audit_event = AuditEventRequest {
        event_type: AuditEventType::UserEncryptionKeysUpgraded,
        user_id: Some(user.id),
        username: Some(user.username.clone()),
        admin_user_id: None,
        admin_username: None,
        target_user_id: Some(user.id),
        target_username: Some(user.username.clone()),
        description: format!(
            "Encryption key access for '{}' moved from a password wrap to a per-user keypair",
            user.username
        ),
        metadata: None,
        ip_address: None,
        user_agent: None,
    };
    if let Err(e) = SqliteAuditRepository::new(db.get_connection()).log_event(&audit_event) {
        error!("Failed to log audit event: {}", e);
    }
    info!("Upgraded encryption keys for user: {}", user.username);
}

/// Sets a new password for an account using the recovery code, which also
/// gives the account a fresh copy of the data key. The recovery code is
/// replaced, and the new one is returned so it can be shown to the user.
#[tauri::command]
pub async fn reset_password_with_recovery_key(
    username: String,
    recovery_code: String,
    new_password: String,
    db_state: State<'_, DatabaseState>,
    attempt_tracker: State<'_, LoginAttemptTrackerState>,
) -> Result<String, String> {
    // Look the username up as typed, like login does: the setup screen allows mixed case
    if let Err(e) = PasswordValidator::validate(&new_password) {
        return Err(e.to_string());
    }

    let tracker = attempt_tracker.lock()
        .map_err(|_| "Failed to acquire attempt tracker lock".to_string())?;
    if tracker.is_locked(&username).map_err(|e| e.to_string())? {
        warn!("Recovery attempt for locked account: {}", username);
        return Err("Account is temporarily locked due to too many failed attempts".to_string());
    }
    drop(tracker);

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;

    let user_repo = SqliteUserRepository::new(db.get_connection());
    let key_store = SqliteKeyStoreRepository::new(db.get_connection());

    let user = user_repo.find_by_username(&username)
        .map_err(|e| format!("Failed to look up account: {}", e))?;
    let data_key = user.as_ref()
        .map(|_| key_store.unlock_with_recovery_code(&recovery_code));

    let (mut user, data_key) = match (user, data_key) {
        (Some(user), Some(Ok(data_key))) => (user, data_key),
        _ => {
            let tracker = attempt_tracker.lock()
                .map_err(|_| "Failed to acquire attempt tracker lock".to_string())?;
            tracker.record_failed_attempt(&username).map_err(|e| e.to_string())?;
            warn!("Failed password recovery attempt for user: {}", username);
            return Err("Invalid username or recovery key".to_string());
        }
    };

    user.password_hash = hash_password(&new_password)
        .map_err(|e| format!("Failed to hash password: {}", e))?;

    let tx = db.get_connection().unchecked_transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;
    user_repo.update_user(&user)
        .map_err(|e| format!("Failed to update password: {}", e))?;
    key_store.set_user_keys(user.id, &new_password, &data_key)
        .map_err(|e| format!("Failed to update encryption key: {}", e))?;
    let new_recovery_code = key_store.replace_recovery_code(&data_key)
        .map_err(|e| format!("Failed to replace recovery key: {}", e))?;
    tx.commit()
        .map_err(|e| format!("Failed to update password: {}", e))?;

    let tracker = attempt_tracker.lock()
        .map_err(|_| "Failed to acquire attempt tracker lock".to_string())?;
    tracker.record_successful_attempt(&username).map_err(|e| e.to_string())?;
    drop(tracker);

    let audit_repo = SqliteAuditRepository::new(db.get_connection());
    let audit_event = AuditEventRequest {
        event_type: AuditEventType::UserPasswordChanged,
        user_id: Some(user.id),
        username: Some(user.username.clone()),
        admin_user_id: None,
        admin_username: None,
        target_user_id: Some(user.id),
        target_username: Some(user.username.clone()),
        description: format!("Password for '{}' reset with the recovery key; recovery key replaced", user.username),
        metadata: None,
        ip_address: None,
        user_agent: None,
    };
    if let Err(e) = audit_repo.log_event(&audit_event) {
        error!("Failed to log audit event: {}", e);
    }

    info!("Password reset with recovery key for user: {}", user.username);
    Ok(new_recovery_code)
}

/// Replaces the recovery code for an administrator who still has their
/// password, e.g. after losing the old code. The old code stops working.
#[tauri::command]
pub async fn replace_recovery_key(
    token: String,
    db_state: State<'_, DatabaseState>,
    session_manager: State<'_, SessionManagerState>,
) -> Result<String, String> {
    let session_manager_guard = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager_guard.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager_guard);

    if session.role != UserRole::Administrator {
        warn!("Non-admin user attempted to replace the recovery key: {}", session.username);
        return Err("Only administrators can replace the recovery key".to_string());
    }
    // Allowed during a key rotation: the session key is then the new active
    // key, which is the one the recovery code must open
    let data_key = session.rotation_key()?;

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;

    let key_store = SqliteKeyStoreRepository::new(db.get_connection());
    let active_version = key_store.active_key_version()
        .map_err(|e| format!("Failed to check encryption key state: {}", e))?;
    if active_version != Some(data_key.version()) {
        return Err("Your session holds an outdated encryption key; log in again".to_string());
    }
    let recovery_code = key_store
        .replace_recovery_code(&data_key)
        .map_err(|e| format!("Failed to replace recovery key: {}", e))?;

    let audit_repo = SqliteAuditRepository::new(db.get_connection());
    let audit_event = AuditEventRequest {
        event_type: AuditEventType::RecoveryKeyReplaced,
        user_id: Some(session.user_id),
        username: Some(session.username.clone()),
        admin_user_id: Some(session.user_id),
        admin_username: Some(session.username.clone()),
        target_user_id: None,
        target_username: None,
        description: format!("Administrator '{}' replaced the recovery key", session.username),
        metadata: None,
        ip_address: None,
        user_agent: None,
    };
    if let Err(e) = audit_repo.log_event(&audit_event) {
        error!("Failed to log audit event: {}", e);
    }

    info!("Recovery key replaced by administrator: {}", session.username);
    Ok(recovery_code)
}

#[tauri::command]
pub async fn logout(
    token: String,
    session_manager: State<'_, SessionManagerState>,
) -> Result<(), String> {
    let session_manager = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    session_manager.invalidate_session(&token)
        .map_err(|e| format!("Failed to logout: {}", e))?;
    
    info!("User logged out");
    Ok(())
}

#[tauri::command]
pub async fn check_session(
    token: String,
    session_manager: State<'_, SessionManagerState>,
) -> Result<UserInfo, String> {
    let session_manager = session_manager.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    match session_manager.validate_session(&token) {
        Ok(Some(session)) => {
            Ok(UserInfo {
                id: session.user_id,
                username: session.username,
                role: session.role,
                created_at: "".to_string(), // We don't store this in session
                is_active: true,
            })
        }
        Ok(None) => Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            Err("Session validation error".to_string())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::create_engineer_user;
    use crate::encryption::FileEncryption;
    use std::sync::Arc;
    use std::time::Duration;
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
    use tauri::Manager;

    const ADMIN_PASSWORD: &str = "Admin-Pass-123";
    const ENGINEER_PASSWORD: &str = "Engineer-Pass-123";
    const NEW_PASSWORD: &str = "Reset-Pass-456";

    fn setup_test_app() -> tauri::App<MockRuntime> {
        mock_builder()
            .manage(DatabaseState::new(Some(Database::new(":memory:".into()).unwrap())))
            .manage(SessionManagerState::new(SessionManager::new()))
            .manage(LoginAttemptTrackerState::new(LoginAttemptTracker::new()))
            .manage(RateLimiterState::new(RateLimiter::new(100, Duration::from_secs(60))))
            .build(mock_context(noop_assets()))
            .unwrap()
    }

    fn session_key(app: &tauri::App<MockRuntime>, token: &str) -> Arc<DataKey> {
        app.state::<SessionManagerState>().lock().unwrap()
            .validate_session(token).unwrap().unwrap()
            .data_key().unwrap()
    }

    fn same_key(a: &DataKey, b: &DataKey) -> bool {
        let ciphertext = FileEncryption::new(a).encrypt(b"probe").unwrap();
        FileEncryption::new(b).decrypt(&ciphertext).is_ok()
    }

    async fn login_as(app: &tauri::App<MockRuntime>, username: &str, password: &str) -> Result<LoginResponse, String> {
        login(username.to_string(), password.to_string(), app.state(), app.state(), app.state()).await
    }

    async fn reset(app: &tauri::App<MockRuntime>, recovery_code: &str, new_password: &str) -> Result<String, String> {
        reset_password_with_recovery_key(
            "admin".to_string(), recovery_code.to_string(), new_password.to_string(), app.state(), app.state(),
        ).await
    }

    async fn setup_admin(app: &tauri::App<MockRuntime>) -> LoginResponse {
        create_admin_account("admin".to_string(), ADMIN_PASSWORD.to_string(), app.state(), app.state()).await.unwrap()
    }

    #[tokio::test]
    async fn test_setup_login_and_engineer_share_one_data_key() {
        let app = setup_test_app();

        let setup = setup_admin(&app).await;
        assert!(setup.recovery_code.is_some());
        let admin_key = session_key(&app, &setup.token);

        let admin_login = login_as(&app, "admin", ADMIN_PASSWORD).await.unwrap();
        assert!(admin_login.recovery_code.is_none());
        assert!(same_key(&admin_key, &session_key(&app, &admin_login.token)));

        create_engineer_user(
            setup.token.clone(), "engineer".to_string(), ENGINEER_PASSWORD.to_string(),
            app.state(), app.state(), app.state(),
        ).await.unwrap();
        let engineer_login = login_as(&app, "engineer", ENGINEER_PASSWORD).await.unwrap();
        assert!(same_key(&admin_key, &session_key(&app, &engineer_login.token)));
    }

    fn query_i64(app: &tauri::App<MockRuntime>, sql: &str, user_id: i64) -> i64 {
        let db_state = app.state::<DatabaseState>();
        let db_guard = db_state.lock().unwrap();
        db_guard.as_ref().unwrap().get_connection()
            .query_row(sql, [user_id], |row| row.get(0)).unwrap()
    }

    fn public_key(app: &tauri::App<MockRuntime>, user_id: i64) -> Vec<u8> {
        let db_state = app.state::<DatabaseState>();
        let db_guard = db_state.lock().unwrap();
        db_guard.as_ref().unwrap().get_connection()
            .query_row("SELECT public_key FROM user_keypairs WHERE user_id = ?1", [user_id], |row| row.get(0))
            .unwrap()
    }

    fn keypairs(app: &tauri::App<MockRuntime>, user_id: i64) -> i64 {
        query_i64(app, "SELECT COUNT(*) FROM user_keypairs WHERE user_id = ?1", user_id)
    }

    fn legacy_wraps(app: &tauri::App<MockRuntime>, user_id: i64) -> i64 {
        query_i64(app, "SELECT COUNT(*) FROM data_key_wraps WHERE user_id = ?1", user_id)
    }

    fn upgrade_events(app: &tauri::App<MockRuntime>, user_id: i64) -> i64 {
        query_i64(app, "SELECT COUNT(*) FROM audit_events WHERE event_code = 'SEC_003' AND user_id = ?1", user_id)
    }

    #[tokio::test]
    async fn test_setup_gives_admin_a_keypair_and_no_password_wrap() {
        let app = setup_test_app();
        let setup = setup_admin(&app).await;

        assert_eq!(keypairs(&app, setup.user.id), 1);
        assert_eq!(legacy_wraps(&app, setup.user.id), 0);
        assert_eq!(query_i64(&app, "SELECT COUNT(*) FROM data_key_wraps WHERE kind = 'recovery' AND ?1 > 0", 1), 1);
        assert_eq!(query_i64(&app, "SELECT COUNT(*) FROM data_key_public_wraps WHERE user_id = ?1", setup.user.id), 1);
        assert_eq!(upgrade_events(&app, setup.user.id), 0);
    }

    #[tokio::test]
    async fn test_legacy_user_login_moves_to_keypair_once() {
        let app = setup_test_app();
        let setup = setup_admin(&app).await;
        let admin_key = session_key(&app, &setup.token);
        let engineer = create_engineer_user(
            setup.token.clone(), "engineer".to_string(), ENGINEER_PASSWORD.to_string(),
            app.state(), app.state(), app.state(),
        ).await.unwrap();
        {
            // Turn the engineer into a pre-keypair user: only a password wrap
            let db_state = app.state::<DatabaseState>();
            let db_guard = db_state.lock().unwrap();
            let store = SqliteKeyStoreRepository::with_params(
                db_guard.as_ref().unwrap().get_connection(), crate::encryption::KdfParams::fast_for_tests(),
            );
            store.revoke_user(engineer.id).unwrap();
            store.store_legacy_user_wrap(engineer.id, ENGINEER_PASSWORD, &admin_key).unwrap();
        }
        assert_eq!(keypairs(&app, engineer.id), 0);
        assert_eq!(legacy_wraps(&app, engineer.id), 1);

        let first = login_as(&app, "engineer", ENGINEER_PASSWORD).await.unwrap();
        assert!(same_key(&admin_key, &session_key(&app, &first.token)));
        assert_eq!(keypairs(&app, engineer.id), 1);
        assert_eq!(legacy_wraps(&app, engineer.id), 0);
        assert_eq!(upgrade_events(&app, engineer.id), 1);
        let migrated_public_key = public_key(&app, engineer.id);

        let second = login_as(&app, "engineer", ENGINEER_PASSWORD).await.unwrap();
        assert!(same_key(&admin_key, &session_key(&app, &second.token)));
        assert_eq!(upgrade_events(&app, engineer.id), 1);
        assert_eq!(public_key(&app, engineer.id), migrated_public_key);
    }

    #[tokio::test]
    async fn test_recovery_reset_replaces_keypair() {
        let app = setup_test_app();
        let setup = setup_admin(&app).await;
        let admin_key = session_key(&app, &setup.token);
        let old_public_key = public_key(&app, setup.user.id);

        reset(&app, setup.recovery_code.as_deref().unwrap(), NEW_PASSWORD).await.unwrap();

        assert_ne!(public_key(&app, setup.user.id), old_public_key);
        assert_eq!(keypairs(&app, setup.user.id), 1);
        assert_eq!(legacy_wraps(&app, setup.user.id), 0);
        let relogin = login_as(&app, "admin", NEW_PASSWORD).await.unwrap();
        assert!(same_key(&admin_key, &session_key(&app, &relogin.token)));
    }

    #[tokio::test]
    async fn test_session_key_carries_active_version_into_new_rows() {
        use crate::configurations::{ConfigurationRepository, CreateConfigurationRequest, SqliteConfigurationRepository};

        let app = setup_test_app();
        let setup = setup_admin(&app).await;
        let login = login_as(&app, "admin", ADMIN_PASSWORD).await.unwrap();
        let key = session_key(&app, &login.token);

        let db_state = app.state::<DatabaseState>();
        let db_guard = db_state.lock().unwrap();
        let conn = db_guard.as_ref().unwrap().get_connection();
        let active: u32 = conn.query_row(
            "SELECT version FROM data_keys WHERE status = 'active'", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(key.version(), active);
        assert_eq!(session_key(&app, &setup.token).version(), active);

        conn.execute("INSERT INTO assets (id, name, created_by) VALUES (1, 'plc', ?1)", [setup.user.id]).unwrap();
        let config = SqliteConfigurationRepository::new(conn).store_configuration(CreateConfigurationRequest {
            asset_id: 1, file_name: "a.cfg".into(), file_content: b"config".to_vec(),
            author: setup.user.id, notes: String::new(),
        }, &key).unwrap();
        let row_version: u32 = conn.query_row(
            "SELECT key_version FROM configuration_versions WHERE id = ?1", [config.id], |r| r.get(0),
        ).unwrap();
        assert_eq!(row_version, active);
    }

    #[tokio::test]
    async fn test_recovery_key_resets_password_and_is_replaced() {
        let app = setup_test_app();
        let setup = setup_admin(&app).await;
        let admin_key = session_key(&app, &setup.token);
        let recovery_code = setup.recovery_code.unwrap();

        let new_code = reset(&app, &recovery_code, NEW_PASSWORD).await.unwrap();
        assert_ne!(new_code, recovery_code);

        assert!(login_as(&app, "admin", ADMIN_PASSWORD).await.is_err());
        let relogin = login_as(&app, "admin", NEW_PASSWORD).await.unwrap();
        assert!(same_key(&admin_key, &session_key(&app, &relogin.token)));

        // The used code stops working; the replacement works
        assert!(reset(&app, &recovery_code, "Another-Pass-789").await.is_err());
        assert!(reset(&app, &new_code, "Another-Pass-789").await.is_ok());
    }

    #[tokio::test]
    async fn test_wrong_recovery_key_changes_nothing() {
        let app = setup_test_app();
        setup_admin(&app).await;

        let result = reset(&app, "0000-0000-0000-0000-0000-0000-0000-0000-0000-0000", NEW_PASSWORD).await;

        assert_eq!(result, Err("Invalid username or recovery key".to_string()));
        assert!(login_as(&app, "admin", ADMIN_PASSWORD).await.is_ok());
    }

    #[tokio::test]
    async fn test_admin_login_creates_key_for_database_without_one() {
        let app = setup_test_app();
        {
            // An admin from before the key store existed: no wrapped key
            let db_state = app.state::<DatabaseState>();
            let db_guard = db_state.lock().unwrap();
            SqliteUserRepository::new(db_guard.as_ref().unwrap().get_connection())
                .create_user(CreateUserRequest {
                    username: "admin".to_string(),
                    password: ADMIN_PASSWORD.to_string(),
                    role: UserRole::Administrator,
                })
                .unwrap();
        }

        let first = login_as(&app, "admin", ADMIN_PASSWORD).await.unwrap();
        assert!(first.recovery_code.is_some());
        let key = session_key(&app, &first.token);

        let second = login_as(&app, "admin", ADMIN_PASSWORD).await.unwrap();
        assert!(second.recovery_code.is_none());
        assert!(same_key(&key, &session_key(&app, &second.token)));
    }

    #[tokio::test]
    async fn test_admin_can_replace_recovery_key() {
        let app = setup_test_app();
        let setup = setup_admin(&app).await;
        let old_code = setup.recovery_code.unwrap();

        let new_code = replace_recovery_key(setup.token.clone(), app.state(), app.state()).await.unwrap();

        assert!(reset(&app, &old_code, NEW_PASSWORD).await.is_err());
        assert!(reset(&app, &new_code, NEW_PASSWORD).await.is_ok());
    }

    #[tokio::test]
    async fn test_engineer_cannot_replace_recovery_key() {
        let app = setup_test_app();
        let setup = setup_admin(&app).await;
        create_engineer_user(
            setup.token.clone(), "engineer".to_string(), ENGINEER_PASSWORD.to_string(),
            app.state(), app.state(), app.state(),
        ).await.unwrap();
        let engineer = login_as(&app, "engineer", ENGINEER_PASSWORD).await.unwrap();

        let result = replace_recovery_key(engineer.token, app.state(), app.state()).await;

        assert_eq!(result, Err("Only administrators can replace the recovery key".to_string()));
    }

    #[tokio::test]
    async fn test_recovery_works_for_mixed_case_username() {
        let app = setup_test_app();
        let setup = create_admin_account("PlantAdmin".to_string(), ADMIN_PASSWORD.to_string(), app.state(), app.state())
            .await.unwrap();

        let result = reset_password_with_recovery_key(
            "PlantAdmin".to_string(), setup.recovery_code.unwrap(), NEW_PASSWORD.to_string(), app.state(), app.state(),
        ).await;

        assert!(result.is_ok(), "{:?}", result);
        assert!(login_as(&app, "PlantAdmin", NEW_PASSWORD).await.is_ok());
    }
}
