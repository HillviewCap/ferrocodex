use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};
use uuid::Uuid;

use crate::users::{User, UserInfo, UserRole};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToken {
    pub token: String,
    pub user_id: i64,
    pub username: String,
    pub role: UserRole,
    pub expires_at: u64,
}

// Type alias for compatibility
pub type Session = SessionToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

// Type alias for service compatibility
pub type AuthService = SessionManager;
pub type AuthState = SessionManager;

#[derive(Debug)]
pub struct SessionManager {
    sessions: Mutex<HashMap<String, SessionToken>>,
    session_duration: u64, // in seconds
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            session_duration: 24 * 60 * 60, // 24 hours
        }
    }

    pub fn create_session(&self, user: &User) -> Result<SessionToken> {
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