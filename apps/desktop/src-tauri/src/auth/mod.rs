use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};
use uuid::Uuid;

use crate::encryption::DataKey;
use crate::users::{User, UserInfo, UserRole};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToken {
    pub token: String,
    pub user_id: i64,
    pub username: String,
    pub role: UserRole,
    pub expires_at: u64,
    /// The unlocked data key. Only set for sessions opened with a password,
    /// and never serialized.
    #[serde(skip)]
    pub data_key: Option<Arc<DataKey>>,
    /// Shared with the [`SessionManager`] and every other session. Set while
    /// a data-key rotation is in progress, which blocks [`SessionToken::data_key`].
    #[serde(skip)]
    rotation_gate: Arc<AtomicBool>,
}

/// The error every encrypted-data command returns while a rotation runs.
pub const KEY_ROTATION_IN_PROGRESS_ERROR: &str =
    "Encryption key rotation in progress; an administrator must finish it";

impl SessionToken {
    /// The unlocked data key, for reading or writing encrypted data. Errors
    /// while a data-key rotation is in progress, so nothing is read or written
    /// under a key that is being replaced.
    pub fn data_key(&self) -> Result<Arc<DataKey>, String> {
        if self.rotation_gate.load(Ordering::SeqCst) {
            return Err(KEY_ROTATION_IN_PROGRESS_ERROR.to_string());
        }
        self.rotation_key()
    }

    /// The unlocked data key regardless of the rotation gate. Only the
    /// rotation commands and Replace Recovery Key may use this.
    pub fn rotation_key(&self) -> Result<Arc<DataKey>, String> {
        self.data_key.clone().ok_or_else(|| {
            "Encrypted data is unavailable for this account. Ask an administrator to reset your password in User Management.".to_string()
        })
    }
}

// Type alias for compatibility
pub type Session = SessionToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
    /// Set only when this login created the data key. Shown to the user once.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_code: Option<String>,
    /// True while a data-key rotation is in progress: encrypted data is
    /// unavailable until an administrator finishes it.
    #[serde(default)]
    pub key_rotation_in_progress: bool,
}

// Type alias for service compatibility
pub type AuthService = SessionManager;
pub type AuthState = SessionManager;

#[derive(Debug)]
pub struct SessionManager {
    sessions: Mutex<HashMap<String, SessionToken>>,
    session_duration: u64, // in seconds
    rotation_gate: Arc<AtomicBool>,
    /// Bumped every time begin or finalize sets or clears the gate. Login
    /// reads the database without holding the session lock, so it uses this
    /// to tell whether the state it read is still current.
    rotation_epoch: AtomicU64,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            session_duration: 24 * 60 * 60, // 24 hours
            rotation_gate: Arc::new(AtomicBool::new(false)),
            rotation_epoch: AtomicU64::new(0),
        }
    }

    /// Opens or closes the rotation gate for every session, current and
    /// future. For the rotation commands (begin and finalize), right after
    /// they commit; it also bumps the rotation epoch.
    pub fn set_rotation_in_progress(&self, in_progress: bool) {
        self.rotation_epoch.fetch_add(1, Ordering::SeqCst);
        let was = self.rotation_gate.swap(in_progress, Ordering::SeqCst);
        if was != in_progress {
            info!("Data-key rotation gate {}", if in_progress { "set" } else { "cleared" });
        }
    }

    /// The current rotation epoch; read before looking up the rotation state
    /// in the database, and pass it to [`SessionManager::sync_rotation_gate`].
    pub fn rotation_epoch(&self) -> u64 {
        self.rotation_epoch.load(Ordering::SeqCst)
    }

    /// For login: sets the gate to the rotation state read from the database
    /// after `epoch` was taken (a retiring key exists or not), so a restart
    /// in the middle of a rotation gates every session again.
    ///
    /// Login does not hold the session lock while it reads the database (that
    /// lock order could deadlock). If begin or finalize changed the gate since
    /// `epoch`, what login read may be stale (and so may the key it opened):
    /// nothing is changed and this returns false, and the caller must read the
    /// database again before creating a session.
    pub fn sync_rotation_gate(&self, epoch: u64, in_progress: bool) -> bool {
        if self.rotation_epoch.load(Ordering::SeqCst) != epoch {
            return false;
        }
        let was = self.rotation_gate.swap(in_progress, Ordering::SeqCst);
        if was != in_progress {
            info!("Data-key rotation gate {} at login", if in_progress { "set" } else { "cleared" });
        }
        true
    }

    pub fn rotation_in_progress(&self) -> bool {
        self.rotation_gate.load(Ordering::SeqCst)
    }

    pub fn create_session(&self, user: &User) -> Result<SessionToken> {
        self.create_session_with_key(user, None)
    }

    pub fn create_session_with_key(&self, user: &User, data_key: Option<DataKey>) -> Result<SessionToken> {
        let token = Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let session = SessionToken {
            token: token.clone(),
            user_id: user.id,
            username: user.username.clone(),
            role: user.role.clone(),
            expires_at: now + self.session_duration,
            data_key: data_key.map(Arc::new),
            rotation_gate: self.rotation_gate.clone(),
        };

        {
            let mut sessions = self.sessions.lock().unwrap();
            sessions.insert(token.clone(), session.clone());
        }

        info!("Created session for user: {} (ID: {})", user.username, user.id);
        Ok(session)
    }

    pub fn validate_session(&self, token: &str) -> Result<Option<SessionToken>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut sessions = self.sessions.lock().unwrap();
        
        if let Some(session) = sessions.get(token).cloned() {
            if session.expires_at > now {
                Ok(Some(session))
            } else {
                let username = session.username.clone();
                sessions.remove(token);
                warn!("Session expired for user: {}", username);
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn invalidate_session(&self, token: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.remove(token) {
            info!("Invalidated session for user: {}", session.username);
        }
        Ok(())
    }

    /// Ends every session belonging to `user_id`. Returns how many were removed.
    pub fn invalidate_user_sessions(&self, user_id: i64) -> usize {
        let mut sessions = self.sessions.lock().unwrap();
        let before = sessions.len();
        sessions.retain(|_, session| session.user_id != user_id);
        let removed = before - sessions.len();
        if removed > 0 {
            info!("Invalidated {} session(s) for user ID {}", removed, user_id);
        }
        removed
    }

    /// Ends every session except `keep_token`. Returns how many were removed.
    pub fn invalidate_all_except(&self, keep_token: &str) -> usize {
        let mut sessions = self.sessions.lock().unwrap();
        let before = sessions.len();
        sessions.retain(|token, _| token == keep_token);
        let removed = before - sessions.len();
        if removed > 0 {
            info!("Invalidated {} other session(s)", removed);
        }
        removed
    }

    /// Swaps the unlocked data key held by one session. Errors if there is no such session.
    pub fn replace_session_key(&self, token: &str, key: DataKey) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        match sessions.get_mut(token) {
            Some(session) => {
                session.data_key = Some(Arc::new(key));
                Ok(())
            }
            None => Err(anyhow::anyhow!("Session not found")),
        }
    }

    pub fn cleanup_expired_sessions(&self) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut sessions = self.sessions.lock().unwrap();
        let expired_tokens: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.expires_at <= now)
            .map(|(token, _)| token.clone())
            .collect();

        for token in expired_tokens {
            sessions.remove(&token);
        }

        Ok(())
    }

    /// Get current user ID from any active session (for backward compatibility)
    /// Note: This method assumes there's only one active session at a time
    pub fn get_current_user_id(&self) -> Option<i64> {
        let sessions = self.sessions.lock().ok()?;
        
        // Return the first valid session's user_id
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs();
            
        for session in sessions.values() {
            if session.expires_at > now {
                return Some(session.user_id);
            }
        }
        None
    }

    /// Get current user info from any active session
    pub fn get_current_user(&self) -> Option<UserInfo> {
        let sessions = self.sessions.lock().ok()?;
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs();
            
        for session in sessions.values() {
            if session.expires_at > now {
                return Some(UserInfo {
                    id: session.user_id,
                    username: session.username.clone(),
                    role: session.role.clone(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    is_active: true,
                });
            }
        }
        None
    }

    /// Check if there's any active session
    pub fn has_active_session(&self) -> bool {
        let sessions = self.sessions.lock().unwrap();
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        sessions.values().any(|session| session.expires_at > now)
    }

    /// Check if current user is admin
    pub fn is_admin(&self) -> bool {
        if let Some(user) = self.get_current_user() {
            matches!(user.role, UserRole::Administrator)
        } else {
            false
        }
    }

    /// Check if user has specific permission
    pub fn check_user_permission(&self, user_id: i64, permission: &str) -> Result<bool> {
        let sessions = self.sessions.lock().unwrap();
        
        // Find session for this user_id
        if let Some(session) = sessions.values().find(|s| s.user_id == user_id) {
            // Check if session is still valid
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
                
            if session.expires_at <= now {
                return Ok(false); // Session expired
            }
            
            // Basic permission check based on role
            match permission {
                "admin" => Ok(matches!(session.role, UserRole::Administrator)),
                "create_asset" | "modify_asset" | "delete_asset" => {
                    // Both admin and engineer can manage assets
                    Ok(matches!(session.role, UserRole::Administrator | UserRole::Engineer))
                }
                "read" | "view" => Ok(true), // All authenticated users can read
                _ => Ok(matches!(session.role, UserRole::Administrator)), // Default: admin only
            }
        } else {
            Ok(false) // No session found for user
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

// Password hashing functions
pub fn hash_password(password: &str) -> Result<String> {
    if password.len() < 8 {
        return Err(anyhow::anyhow!("Password must be at least 8 characters long"));
    }
    
    let hashed = hash(password, DEFAULT_COST)?;
    Ok(hashed)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let is_valid = verify(password, hash)?;
    Ok(is_valid)
}

// Rate limiting for login attempts
#[derive(Debug)]
pub struct LoginAttemptTracker {
    attempts: Mutex<HashMap<String, (u32, u64)>>, // username -> (count, last_attempt_time)
    max_attempts: u32,
    lockout_duration: u64, // in seconds
}

impl LoginAttemptTracker {
    pub fn new() -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            max_attempts: 5,
            lockout_duration: 15 * 60, // 15 minutes
        }
    }

    pub fn record_failed_attempt(&self, username: &str) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut attempts = self.attempts.lock().unwrap();
        let entry = attempts.entry(username.to_string()).or_insert((0, now));
        
        // Reset count if lockout period has passed
        if now - entry.1 > self.lockout_duration {
            entry.0 = 1;
        } else {
            entry.0 += 1;
        }
        entry.1 = now;

        if entry.0 >= self.max_attempts {
            warn!("Account locked due to too many failed attempts: {}", username);
        }

        Ok(())
    }

    pub fn record_successful_attempt(&self, username: &str) -> Result<()> {
        let mut attempts = self.attempts.lock().unwrap();
        attempts.remove(username);
        Ok(())
    }

    pub fn is_locked(&self, username: &str) -> Result<bool> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let attempts = self.attempts.lock().unwrap();
        if let Some((count, last_attempt)) = attempts.get(username) {
            if *count >= self.max_attempts && now - last_attempt < self.lockout_duration {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn cleanup_old_attempts(&self) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut attempts = self.attempts.lock().unwrap();
        attempts.retain(|_, (_, last_attempt)| now - *last_attempt < self.lockout_duration);
        Ok(())
    }
}

impl Default for LoginAttemptTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Account is locked due to too many failed attempts")]
    AccountLocked,
    
    #[error("Session has expired")]
    SessionExpired,
    
    #[error("Invalid session token")]
    InvalidSession,
    
    #[error("Password validation failed: {0}")]
    PasswordValidation(String),
    
    #[error("Database error: {0}")]
    Database(#[from] anyhow::Error),
}

#[cfg(test)]
pub mod tests {
    use super::*;

    static INIT: std::sync::Once = std::sync::Once::new();

    pub fn init_for_testing() {
        INIT.call_once(|| {
            // Initialize any global state needed for testing
        });
    }

    #[test]
    fn test_password_hashing() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();
        
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_password_length_validation() {
        let short_password = "1234567"; // 7 characters
        assert!(hash_password(short_password).is_err());

        let valid_password = "12345678"; // 8 characters
        assert!(hash_password(valid_password).is_ok());
    }

    #[test]
    fn test_session_creation_and_validation() {
        let session_manager = SessionManager::new();
        let user = User {
            id: 1,
            username: "testuser".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Administrator,
            created_at: "2023-01-01".to_string(),
            updated_at: "2023-01-01".to_string(),
            is_active: true,
        };

        let session = session_manager.create_session(&user).unwrap();
        assert_eq!(session.user_id, 1);
        assert_eq!(session.username, "testuser");
        assert_eq!(session.role, UserRole::Administrator);

        let validated = session_manager.validate_session(&session.token).unwrap();
        assert!(validated.is_some());
        assert_eq!(validated.unwrap().user_id, 1);
    }

    #[test]
    fn test_session_invalidation() {
        let session_manager = SessionManager::new();
        let user = User {
            id: 1,
            username: "testuser".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Administrator,
            created_at: "2023-01-01".to_string(),
            updated_at: "2023-01-01".to_string(),
            is_active: true,
        };

        let session = session_manager.create_session(&user).unwrap();
        session_manager.invalidate_session(&session.token).unwrap();

        let validated = session_manager.validate_session(&session.token).unwrap();
        assert!(validated.is_none());
    }

    fn test_user(id: i64, name: &str) -> User {
        User {
            id,
            username: name.to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Engineer,
            created_at: "2023-01-01".to_string(),
            updated_at: "2023-01-01".to_string(),
            is_active: true,
        }
    }

    #[test]
    fn test_invalidate_user_sessions() {
        let manager = SessionManager::new();
        let a1 = manager.create_session(&test_user(1, "a")).unwrap();
        let a2 = manager.create_session(&test_user(1, "a")).unwrap();
        let b = manager.create_session(&test_user(2, "b")).unwrap();

        assert_eq!(manager.invalidate_user_sessions(1), 2);
        assert!(manager.validate_session(&a1.token).unwrap().is_none());
        assert!(manager.validate_session(&a2.token).unwrap().is_none());
        assert!(manager.validate_session(&b.token).unwrap().is_some());
        assert_eq!(manager.invalidate_user_sessions(1), 0);
    }

    #[test]
    fn test_invalidate_all_except() {
        let manager = SessionManager::new();
        let keep = manager.create_session(&test_user(1, "a")).unwrap();
        let other1 = manager.create_session(&test_user(2, "b")).unwrap();
        let other2 = manager.create_session(&test_user(1, "a")).unwrap();

        assert_eq!(manager.invalidate_all_except(&keep.token), 2);
        assert!(manager.validate_session(&keep.token).unwrap().is_some());
        assert!(manager.validate_session(&other1.token).unwrap().is_none());
        assert!(manager.validate_session(&other2.token).unwrap().is_none());
    }

    #[test]
    fn test_replace_session_key() {
        use crate::encryption::FileEncryption;
        let manager = SessionManager::new();
        let k1 = DataKey::generate();
        let session = manager.create_session_with_key(&test_user(1, "a"), Some(k1)).unwrap();
        let old_key = session.data_key().unwrap();
        let ciphertext = FileEncryption::new(&old_key).encrypt(b"probe").unwrap();

        manager.replace_session_key(&session.token, DataKey::generate()).unwrap();
        let new_key = manager.validate_session(&session.token).unwrap().unwrap().data_key().unwrap();
        assert!(FileEncryption::new(&new_key).decrypt(&ciphertext).is_err());
        assert!(FileEncryption::new(&old_key).decrypt(&ciphertext).is_ok());

        assert!(manager.replace_session_key("missing", DataKey::generate()).is_err());
    }

    #[test]
    fn test_rotation_gate_blocks_data_key_for_every_session() {
        let manager = SessionManager::new();
        let before = manager.create_session_with_key(&test_user(1, "a"), Some(DataKey::generate())).unwrap();
        assert!(before.data_key().is_ok());

        manager.set_rotation_in_progress(true);
        assert!(manager.rotation_in_progress());
        // Sessions created before and after the gate was set, and copies
        // returned by validate_session, all share it
        let after = manager.create_session_with_key(&test_user(2, "b"), Some(DataKey::generate())).unwrap();
        let validated = manager.validate_session(&before.token).unwrap().unwrap();
        for session in [&before, &after, &validated] {
            assert_eq!(session.data_key().unwrap_err(), KEY_ROTATION_IN_PROGRESS_ERROR);
            assert!(session.rotation_key().is_ok());
        }

        manager.set_rotation_in_progress(false);
        assert!(before.data_key().is_ok());
        assert!(after.data_key().is_ok());
    }

    /// Login reads the rotation state without the session lock. A begin or
    /// finalize in between must win: login may not undo the gate it set.
    #[test]
    fn test_login_cannot_overwrite_a_gate_changed_meanwhile() {
        let manager = SessionManager::new();
        let epoch = manager.rotation_epoch();
        // begin commits and sets the gate after login read "no rotation"
        manager.set_rotation_in_progress(true);
        assert!(!manager.sync_rotation_gate(epoch, false));
        assert!(manager.rotation_in_progress());

        // A login that read the current state applies it
        let epoch = manager.rotation_epoch();
        assert!(manager.sync_rotation_gate(epoch, true));
        assert!(manager.rotation_in_progress());
        // finalize clears it after login read "rotation in progress"
        manager.set_rotation_in_progress(false);
        assert!(!manager.sync_rotation_gate(epoch, true));
        assert!(!manager.rotation_in_progress());
    }

    #[test]
    fn test_rotation_key_without_a_key_errors() {
        let manager = SessionManager::new();
        let session = manager.create_session(&test_user(1, "a")).unwrap();
        assert!(session.rotation_key().is_err());
        assert!(session.data_key().is_err());
    }

    #[test]
    fn test_login_attempt_tracking() {
        let tracker = LoginAttemptTracker::new();
        let username = "testuser";

        // Initially not locked
        assert!(!tracker.is_locked(username).unwrap());

        // Record failed attempts
        for _ in 0..4 {
            tracker.record_failed_attempt(username).unwrap();
            assert!(!tracker.is_locked(username).unwrap());
        }

        // 5th attempt should lock the account
        tracker.record_failed_attempt(username).unwrap();
        assert!(tracker.is_locked(username).unwrap());

        // Successful attempt should unlock
        tracker.record_successful_attempt(username).unwrap();
        assert!(!tracker.is_locked(username).unwrap());
    }
}