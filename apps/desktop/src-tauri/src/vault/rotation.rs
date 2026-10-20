use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};
use std::collections::HashMap;
use std::str::FromStr;
use zeroize::Zeroizing;
use tracing::{info, debug, warn};
use uuid::Uuid;
use rusqlite::{Connection, params};
use crate::audit::{AuditRepository, AuditEventRequest, AuditEventType};
use super::VaultRepository;
use crate::encryption::{DataKey, FileEncryption};

#[derive(Clone, Serialize, Deserialize)]
pub struct PasswordRotationRequest {
    pub secret_id: i64,
    pub new_password: String,
    pub rotation_reason: String,
    pub author_id: i64,
    pub batch_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationSchedule {
    pub schedule_id: i64,
    pub vault_id: i64,
    pub rotation_interval: i32, // days
    pub alert_days_before: i32,
    pub is_active: bool,
    pub created_at: String,
    pub created_by: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationBatch {
    pub batch_id: i64,
    pub batch_name: String,
    pub created_by: i64,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: BatchStatus,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchStatus::Pending => write!(f, "pending"),
            BatchStatus::InProgress => write!(f, "in_progress"),
            BatchStatus::Completed => write!(f, "completed"),
            BatchStatus::Failed => write!(f, "failed"),
            BatchStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for BatchStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(BatchStatus::Pending),
            "in_progress" => Ok(BatchStatus::InProgress),
            "completed" => Ok(BatchStatus::Completed),
            "failed" => Ok(BatchStatus::Failed),
            "cancelled" => Ok(BatchStatus::Cancelled),
            _ => Err(anyhow::anyhow!("Invalid batch status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordRotationHistory {
    pub rotation_id: i64,
    pub secret_id: i64,
    pub old_password_hash: String,
    pub rotation_reason: String,
    pub rotated_by: i64,
    pub rotated_at: String,
    pub batch_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationAlert {
    pub secret_id: i64,
    pub vault_id: i64,
    pub secret_label: String,
    pub asset_name: String,
    pub days_until_rotation: i32,
    pub next_rotation_due: String,
    pub last_rotated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRotationScheduleRequest {
    pub vault_id: i64,
    pub rotation_interval: i32,
    pub alert_days_before: i32,
    pub created_by: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRotationScheduleRequest {
    pub schedule_id: i64,
    pub rotation_interval: Option<i32>,
    pub alert_days_before: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRotationBatchRequest {
    pub batch_name: String,
    pub created_by: i64,
    pub notes: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BatchRotationItem {
    pub secret_id: i64,
    pub new_password: String,
    pub rotation_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRotationRequest {
    pub batch_id: i64,
    pub items: Vec<BatchRotationItem>,
    pub author_id: i64,
}

/// Number of previous passwords remembered per secret (matches update_password).
const PASSWORD_HISTORY_DEPTH: usize = 5;

// Password Rotation Service
pub struct PasswordRotationService<'a> {
    conn: &'a Connection,
    vault_repo: Box<dyn VaultRepository + 'a>,
    audit_repo: Box<dyn AuditRepository + 'a>,
}

impl<'a> PasswordRotationService<'a> {
    pub fn new(
        conn: &'a Connection,
        vault_repo: Box<dyn VaultRepository + 'a>,
        audit_repo: Box<dyn AuditRepository + 'a>,
    ) -> Self {
        Self { conn, vault_repo, audit_repo }
    }

    // Task 2.1 & 2.2: Guided workflow for individual password rotation
    pub fn rotate_password(&self, request: PasswordRotationRequest, key: &DataKey) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        self.rotate_password_in(&tx, request, key)?;
        tx.commit()?;
        Ok(())
    }

    /// Performs a rotation on `tx` without starting or committing a transaction, so it
    /// can run inside a caller-managed transaction or savepoint (SQLite cannot nest BEGIN).
    /// `tx` must be the same connection the repositories wrap. On error the caller is
    /// responsible for rolling back any partial writes.
    pub(crate) fn rotate_password_in(&self, tx: &Connection, request: PasswordRotationRequest, key: &DataKey) -> Result<()> {
        debug!("Rotating password for secret {}", request.secret_id);

        // Get the current secret
        let secret = self.vault_repo.get_secret_by_id(request.secret_id)?
            .ok_or_else(|| anyhow::anyhow!("Secret not found"))?;

        // Validate it's a password type
        if secret.secret_type.to_string() != "password" {
            return Err(anyhow::anyhow!("Can only rotate password type secrets"));
        }

        // Hash the new password
        let new_password_hash = bcrypt::hash(&request.new_password, super::password_services::HISTORY_BCRYPT_COST)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

        // Task 2.3: Archive the current password in rotation history
        let old_encrypted_value = &secret.encrypted_value;
        // Never fall back to storing the ciphertext itself: history must only hold hashes.
        let old_password_hash = bcrypt::hash(old_encrypted_value, 4)  // Lower cost for history
            .map_err(|e| anyhow::anyhow!("Failed to hash previous password for history: {}", e))?;

        tx.execute(
            "INSERT INTO password_rotation_history (secret_id, old_password_hash, rotation_reason, rotated_by, batch_id) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                request.secret_id,
                old_password_hash,
                request.rotation_reason,
                request.author_id,
                request.batch_id
            ],
        )?;

        // Task 2.4: Update the secret with new password
        let now = Utc::now();
        let rotation_interval = secret.rotation_interval_days.unwrap_or(90);
        let next_rotation_due = now + Duration::days(rotation_interval as i64);

        let encrypted_value = FileEncryption::new(key).encrypt_to_base64(request.new_password.as_bytes())?;

        tx.execute(
            "UPDATE vault_secrets
             SET encrypted_value = ?1,
                 key_version = ?5,
                 last_changed = ?2,
                 last_rotated = ?2,
                 next_rotation_due = ?3,
                 updated_at = ?2
             WHERE id = ?4",
            params![
                encrypted_value,
                now.to_rfc3339(),
                next_rotation_due.to_rfc3339(),
                request.secret_id,
                key.version()
            ],
        )?;

        // Add to password history for reuse prevention. The outgoing password is recorded
        // too (secrets created before rotation have no history entry), then history is
        // trimmed to the depth used by update_password.
        // Both the decrypted bytes and the string copy are wiped when they go out of scope.
        let old_plaintext: Option<Zeroizing<String>> = FileEncryption::new(key)
            .decrypt_from_base64(old_encrypted_value)
            .ok()
            .map(Zeroizing::new)
            .and_then(|bytes| std::str::from_utf8(&bytes).ok().map(|s| Zeroizing::new(s.to_owned())));
        if let Some(old_plaintext) = old_plaintext {
            let history = self.vault_repo.get_password_history(request.secret_id)?;
            let hashes: Vec<String> = history.into_iter().map(|h| h.password_hash).collect();
            if !super::password_services::PasswordReuseChecker::is_password_reused(old_plaintext.as_str(), &hashes) {
                let old_hash = bcrypt::hash(old_plaintext.as_str(), super::password_services::HISTORY_BCRYPT_COST)
                    .map_err(|e| anyhow::anyhow!("Failed to hash previous password for history: {}", e))?;
                self.vault_repo.add_password_history(request.secret_id, &old_hash)?;
            }
        }
        self.vault_repo.add_password_history(request.secret_id, &new_password_hash)?;
        self.vault_repo.cleanup_password_history(request.secret_id, PASSWORD_HISTORY_DEPTH)?;

        // Task 2.6: Audit logging
        let audit_event = AuditEventRequest {
            event_type: AuditEventType::VaultSecretRotated,
            user_id: Some(request.author_id),
            username: None,
            admin_user_id: None,
            admin_username: None,
            target_user_id: None,
            target_username: None,
            description: format!("Rotated password for secret '{}' in vault {}. Reason: {}", 
                secret.label, secret.vault_id, request.rotation_reason),
            metadata: Some(serde_json::json!({
                "secret_id": request.secret_id,
                "vault_id": secret.vault_id,
                "reason": request.rotation_reason
            }).to_string()),
            ip_address: None,
            user_agent: None,
            // request_id: Some(Uuid::new_v4().to_string()), // Temporarily disabled for deployment
        };
        self.audit_repo.log_event(&audit_event)?;

        // Add version history
        let mut changes = HashMap::new();
        changes.insert("action".to_string(), "password_rotated".to_string());
        changes.insert("reason".to_string(), request.rotation_reason.clone());
        changes.insert("batch_id".to_string(), request.batch_id.map_or("null".to_string(), |id| id.to_string()));
        
        self.vault_repo.add_version_history(
            secret.vault_id,
            super::ChangeType::SecretUpdated,
            request.author_id,
            &format!("Password rotated: {}", request.rotation_reason),
            changes,
        )?;

        info!("Successfully rotated password for secret {} by user {}", request.secret_id, request.author_id);
        Ok(())
    }

    // Task 2.5: Validation and rollback capabilities
    pub fn validate_rotation(&self, secret_id: i64, new_password: &str) -> Result<()> {
        // Check password complexity
        if new_password.len() < 12 {
            return Err(anyhow::anyhow!("Password must be at least 12 characters long"));
        }

        // Check password reuse against this secret's history (bcrypt hashes are salted, so verify).
        let hashes: Vec<String> = self.vault_repo.get_password_history(secret_id)?
            .into_iter().map(|h| h.password_hash).collect();
        if super::password_services::PasswordReuseChecker::is_password_reused(new_password, &hashes) {
            return Err(anyhow::anyhow!("Password has been used before. Please choose a different password."));
        }

        Ok(())
    }

    // Get rotation history for a secret
    pub fn get_rotation_history(&self, secret_id: i64) -> Result<Vec<PasswordRotationHistory>> {
        let mut stmt = self.conn.prepare(
            "SELECT rotation_id, secret_id, old_password_hash, rotation_reason, rotated_by, rotated_at, batch_id
             FROM password_rotation_history
             WHERE secret_id = ?1
             ORDER BY rotated_at DESC"
        )?;

        let history_iter = stmt.query_map([secret_id], |row| {
            Ok(PasswordRotationHistory {
                rotation_id: row.get("rotation_id")?,
                secret_id: row.get("secret_id")?,
                old_password_hash: row.get("old_password_hash")?,
                rotation_reason: row.get("rotation_reason")?,
                rotated_by: row.get("rotated_by")?,
                rotated_at: row.get("rotated_at")?,
                batch_id: row.get("batch_id")?,
            })
        })?;

        let mut history = Vec::new();
        for item in history_iter {
            history.push(item?);
        }

        Ok(history)
    }

    // Emergency rotation capability
    pub fn emergency_rotate_password(&self, request: PasswordRotationRequest, key: &DataKey) -> Result<()> {
        info!("Emergency password rotation initiated for secret {}", request.secret_id);
        
        // Skip normal validation for emergency rotation
        self.rotate_password(PasswordRotationRequest {
            rotation_reason: format!("EMERGENCY: {}", request.rotation_reason),
            ..request
        }, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use crate::database::Database;
    use crate::vault::SqliteVaultRepository;
    use crate::audit::SqliteAuditRepository;

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_batch_status_serialization() {
        assert_eq!(BatchStatus::Pending.to_string(), "pending");
        assert_eq!(BatchStatus::InProgress.to_string(), "in_progress");
        assert_eq!(BatchStatus::Completed.to_string(), "completed");
        assert_eq!(BatchStatus::Failed.to_string(), "failed");
        assert_eq!(BatchStatus::Cancelled.to_string(), "cancelled");

        assert_eq!(BatchStatus::from_str("pending").unwrap(), BatchStatus::Pending);
        assert_eq!(BatchStatus::from_str("in_progress").unwrap(), BatchStatus::InProgress);
        assert_eq!(BatchStatus::from_str("completed").unwrap(), BatchStatus::Completed);
        assert_eq!(BatchStatus::from_str("failed").unwrap(), BatchStatus::Failed);
        assert_eq!(BatchStatus::from_str("cancelled").unwrap(), BatchStatus::Cancelled);
    }

    use crate::vault::{AddSecretRequest, CreateVaultRequest, SecretType};

    /// In-memory DB with users, assets, vault and audit schemas, plus one vault.
    fn setup_batch_db() -> (Connection, i64) {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            CREATE TABLE users (
                id INTEGER PRIMARY KEY, username TEXT NOT NULL, password_hash TEXT NOT NULL,
                role TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP, is_active BOOLEAN DEFAULT 1
            );
            CREATE TABLE assets (
                id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, description TEXT,
                created_by INTEGER NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'u', 'h', 'Engineer');
            INSERT INTO assets (id, name, description, created_by) VALUES (1, 'A', 'D', 1);
            "#,
        ).unwrap();
        SqliteVaultRepository::new(&conn).initialize_schema().unwrap();
        SqliteAuditRepository::new(&conn).initialize_schema().unwrap();
        let vault = SqliteVaultRepository::new(&conn).create_vault(CreateVaultRequest {
            asset_id: 1, name: "Test Vault".into(), description: "desc".into(), created_by: 1,
        }).unwrap();
        (conn, vault.id)
    }

    fn add_secret(conn: &Connection, vault_id: i64, ty: SecretType, label: &str) -> i64 {
        SqliteVaultRepository::new(conn).add_secret(AddSecretRequest {
            vault_id, secret_type: ty, label: label.into(), username: None, value: "OldPassword-1234".into(), author_id: 1,
        }, crate::encryption::test_key()).unwrap().id
    }

    fn batch_service(conn: &Connection) -> BatchRotationService<'_> {
        BatchRotationService::new(
            conn,
            Box::new(SqliteVaultRepository::new(conn)),
            Box::new(SqliteAuditRepository::new(conn)),
        )
    }

    fn item(secret_id: i64, pw: &str) -> BatchRotationItem {
        BatchRotationItem { secret_id, new_password: pw.into(), rotation_reason: "scheduled".into() }
    }

    fn count(conn: &Connection, sql: &str, id: i64) -> i64 {
        conn.query_row(sql, [id], |r| r.get(0)).unwrap()
    }

    fn rotation_service(conn: &Connection) -> PasswordRotationService<'_> {
        PasswordRotationService::new(
            conn,
            Box::new(SqliteVaultRepository::new(conn)),
            Box::new(SqliteAuditRepository::new(conn)),
        )
    }

    fn rotate_to(svc: &PasswordRotationService, secret_id: i64, pw: &str) {
        svc.rotate_password(PasswordRotationRequest {
            secret_id, new_password: pw.into(), rotation_reason: "scheduled".into(),
            author_id: 1, batch_id: None,
        }, crate::encryption::test_key()).unwrap();
    }

    #[test]
    fn test_validate_rotation_rejects_reuse_of_previous_passwords() {
        let (conn, vault_id) = setup_batch_db();
        // Initial value is "OldPassword-1234" (A); rotate A -> B.
        let id = add_secret(&conn, vault_id, SecretType::Password, "pw");
        let svc = rotation_service(&conn);
        svc.validate_rotation(id, "NewPassword-bbbb-2").expect("fresh password is accepted");
        rotate_to(&svc, id, "NewPassword-bbbb-2");

        // B -> A (the original) and B -> B are both reuse.
        let err = svc.validate_rotation(id, "OldPassword-1234").unwrap_err();
        assert!(err.to_string().contains("used before"), "{}", err);
        assert!(svc.validate_rotation(id, "NewPassword-bbbb-2").is_err());
        // A brand-new password is still accepted.
        svc.validate_rotation(id, "BrandNew-Password-9").expect("new password accepted");
    }

    #[test]
    fn test_validate_rotation_skips_non_bcrypt_history_and_keeps_five() {
        let (conn, vault_id) = setup_batch_db();
        let id = add_secret(&conn, vault_id, SecretType::Password, "pw");
        let svc = rotation_service(&conn);
        SqliteVaultRepository::new(&conn).add_password_history(id, "redacted").unwrap();
        svc.validate_rotation(id, "BrandNew-Password-9").expect("redacted entries are skipped");

        for i in 0..7 {
            rotate_to(&svc, id, &format!("Rotated-Password-{}", i));
        }
        let n = count(&conn, "SELECT COUNT(*) FROM password_history WHERE secret_id = ?1", id);
        assert_eq!(n, 5);
        assert!(svc.validate_rotation(id, "Rotated-Password-6").is_err());
    }

    #[test]
    fn test_batch_rotation_rotates_all_items() {
        let (conn, vault_id) = setup_batch_db();
        let s1 = add_secret(&conn, vault_id, SecretType::Password, "one");
        let s2 = add_secret(&conn, vault_id, SecretType::Password, "two");
        let svc = batch_service(&conn);
        let batch = svc.create_batch(CreateRotationBatchRequest {
            batch_name: "b".into(), created_by: 1, notes: None,
        }).unwrap();
        let repo = SqliteVaultRepository::new(&conn);
        let before1 = repo.get_secret_by_id(s1).unwrap().unwrap().encrypted_value;

        svc.execute_batch_rotation(BatchRotationRequest {
            batch_id: batch.batch_id,
            items: vec![item(s1, "NewPassword-aaaa-1"), item(s2, "NewPassword-bbbb-2")],
            author_id: 1,
        }, crate::encryption::test_key()).expect("batch rotation should succeed for all items");

        for id in [s1, s2] {
            assert_eq!(count(&conn, "SELECT COUNT(*) FROM password_rotation_history WHERE secret_id = ?1", id), 1);
            // outgoing and new password are both remembered for reuse checks
            assert_eq!(count(&conn, "SELECT COUNT(*) FROM password_history WHERE secret_id = ?1", id), 2);
            assert_eq!(count(&conn, "SELECT COUNT(*) FROM vault_secrets WHERE id = ?1 AND last_rotated IS NOT NULL", id), 1);
        }
        for (id, pw) in [(s1, "NewPassword-aaaa-1"), (s2, "NewPassword-bbbb-2")] {
            let enc = repo.get_secret_by_id(id).unwrap().unwrap().encrypted_value;
            let plain = crate::encryption::FileEncryption::new(crate::encryption::test_key())
                .decrypt_from_base64(&enc).unwrap();
            assert_eq!(String::from_utf8(plain).unwrap(), pw);
        }
        assert_ne!(repo.get_secret_by_id(s1).unwrap().unwrap().encrypted_value, before1);
        let b = svc.get_batch(batch.batch_id).unwrap().unwrap();
        assert_eq!(b.status, BatchStatus::Completed);
        assert!(b.completed_at.is_some());
    }

    #[test]
    fn test_batch_rotation_partial_failure_keeps_successes_and_records_outcome() {
        let (conn, vault_id) = setup_batch_db();
        let good = add_secret(&conn, vault_id, SecretType::Password, "good");
        let ip = add_secret(&conn, vault_id, SecretType::IpAddress, "ip");
        let svc = batch_service(&conn);
        let batch = svc.create_batch(CreateRotationBatchRequest {
            batch_name: "b".into(), created_by: 1, notes: None,
        }).unwrap();

        let err = svc.execute_batch_rotation(BatchRotationRequest {
            batch_id: batch.batch_id,
            items: vec![item(ip, "NewPassword-cccc-3"), item(good, "NewPassword-dddd-4"), item(9999, "NewPassword-eeee-5")],
            author_id: 1,
        }, crate::encryption::test_key()).unwrap_err();
        assert!(err.to_string().contains("1 succeeded, 2 failed"), "{}", err);

        // Successful item persisted; failed items left no partial writes.
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM password_rotation_history WHERE secret_id = ?1", good), 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM password_rotation_history WHERE secret_id = ?1", ip), 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM password_history WHERE secret_id = ?1", ip), 0);
        // Batch record reflects the outcome and is persisted.
        let b = svc.get_batch(batch.batch_id).unwrap().unwrap();
        assert_eq!(b.status, BatchStatus::Completed);
        let notes = b.notes.unwrap();
        assert!(notes.contains("1 successes and 2 failures"), "{}", notes);
        assert!(conn.is_autocommit(), "no transaction should be left open");
    }

    #[test]
    fn test_batch_rotation_all_failures_marks_batch_failed() {
        let (conn, vault_id) = setup_batch_db();
        let ip = add_secret(&conn, vault_id, SecretType::IpAddress, "ip");
        let svc = batch_service(&conn);
        let batch = svc.create_batch(CreateRotationBatchRequest {
            batch_name: "b".into(), created_by: 1, notes: None,
        }).unwrap();
        assert!(svc.execute_batch_rotation(BatchRotationRequest {
            batch_id: batch.batch_id,
            items: vec![item(ip, "NewPassword-ffff-6"), item(9999, "short")],
            author_id: 1,
        }, crate::encryption::test_key()).is_err());
        let b = svc.get_batch(batch.batch_id).unwrap().unwrap();
        assert_eq!(b.status, BatchStatus::Failed);
        assert!(b.notes.unwrap().contains("0 successes and 2 failures"));
    }
}

// Task 3: Rotation Scheduling System
pub struct RotationScheduler<'a> {
    conn: &'a Connection,
}

impl<'a> RotationScheduler<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // Task 3.1: Create rotation schedule with configurable intervals
    pub fn create_rotation_schedule(&self, request: CreateRotationScheduleRequest) -> Result<RotationSchedule> {
        debug!("Creating rotation schedule for vault {}", request.vault_id);

        // Validate interval
        if request.rotation_interval <= 0 {
            return Err(anyhow::anyhow!("Rotation interval must be positive"));
        }

        if request.alert_days_before < 0 || request.alert_days_before >= request.rotation_interval {
            return Err(anyhow::anyhow!("Alert days must be between 0 and rotation interval"));
        }

        let now = Utc::now().to_rfc3339();
        
        let schedule_id = self.conn.execute(
            "INSERT INTO rotation_schedules (vault_id, rotation_interval, alert_days_before, is_active, created_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![
                request.vault_id,
                request.rotation_interval,
                request.alert_days_before,
                true,
                request.created_by,
                now
            ],
        )?;

        info!("Created rotation schedule {} for vault {}", schedule_id, request.vault_id);

        Ok(RotationSchedule {
            schedule_id: self.conn.last_insert_rowid(),
            vault_id: request.vault_id,
            rotation_interval: request.rotation_interval,
            alert_days_before: request.alert_days_before,
            is_active: true,
            created_at: now.clone(),
            created_by: request.created_by,
            updated_at: now,
        })
    }

    // Task 3.2: Calculate and update rotation due dates
    pub fn update_rotation_due_dates(&self, vault_id: i64) -> Result<()> {
        debug!("Updating rotation due dates for vault {}", vault_id);

        // Get the active schedule for this vault
        let schedule = self.get_active_schedule(vault_id)?
            .ok_or_else(|| anyhow::anyhow!("No active rotation schedule found for vault"))?;

        let rotation_interval_days = schedule.rotation_interval;
        
        // Update all password secrets in the vault
        self.conn.execute(
            "UPDATE vault_secrets 
             SET next_rotation_due = datetime(COALESCE(last_rotated, created_at), '+' || ?1 || ' days'),
                 rotation_interval_days = ?1
             WHERE vault_id = ?2 AND secret_type = 'password'",
            params![rotation_interval_days, vault_id],
        )?;

        Ok(())
    }

    // Task 3.3: Get rotation alerts
    pub fn get_rotation_alerts(&self, days_ahead: i32) -> Result<Vec<RotationAlert>> {
        let cutoff_date = (Utc::now() + Duration::days(days_ahead as i64)).to_rfc3339();
        
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.vault_id, s.label, s.next_rotation_due, s.last_rotated,
                    v.name as vault_name, a.name as asset_name,
                    julianday(s.next_rotation_due) - julianday('now') as days_until_rotation
             FROM vault_secrets s
             JOIN vault_entries v ON s.vault_id = v.id
             JOIN assets a ON v.asset_id = a.id
             WHERE s.secret_type = 'password' 
               AND s.next_rotation_due IS NOT NULL
               AND s.next_rotation_due <= ?1
             ORDER BY s.next_rotation_due ASC"
        )?;

        let alert_iter = stmt.query_map([cutoff_date], |row| {
            Ok(RotationAlert {
                secret_id: row.get("id")?,
                vault_id: row.get("vault_id")?,
                secret_label: row.get("label")?,
                asset_name: row.get("asset_name")?,
                days_until_rotation: row.get::<_, f64>("days_until_rotation")?.round() as i32,
                next_rotation_due: row.get("next_rotation_due")?,
                last_rotated: row.get("last_rotated")?,
            })
        })?;

        let mut alerts = Vec::new();
        for alert in alert_iter {
            alerts.push(alert?);
        }

        debug!("Found {} rotation alerts", alerts.len());
        Ok(alerts)
    }

    // Task 3.4: Update rotation schedule
    pub fn update_rotation_schedule(&self, request: UpdateRotationScheduleRequest) -> Result<()> {
        let mut query_parts = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(interval) = request.rotation_interval {
            if interval <= 0 {
                return Err(anyhow::anyhow!("Rotation interval must be positive"));
            }
            query_parts.push("rotation_interval = ?");
            params.push(Box::new(interval));
        }

        if let Some(alert_days) = request.alert_days_before {
            if alert_days < 0 {
                return Err(anyhow::anyhow!("Alert days must be non-negative"));
            }
            query_parts.push("alert_days_before = ?");
            params.push(Box::new(alert_days));
        }

        if let Some(is_active) = request.is_active {
            query_parts.push("is_active = ?");
            params.push(Box::new(is_active));
        }

        if query_parts.is_empty() {
            return Ok(());
        }

        query_parts.push("updated_at = ?");
        params.push(Box::new(Utc::now().to_rfc3339()));

        let query = format!(
            "UPDATE rotation_schedules SET {} WHERE schedule_id = ?",
            query_parts.join(", ")
        );
        params.push(Box::new(request.schedule_id));

        let rows_affected = self.conn.execute(&query, rusqlite::params_from_iter(params))?;
        
        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Schedule not found"));
        }

        Ok(())
    }

    // Task 3.5: Get active schedule for a vault
    pub fn get_active_schedule(&self, vault_id: i64) -> Result<Option<RotationSchedule>> {
        let mut stmt = self.conn.prepare(
            "SELECT schedule_id, vault_id, rotation_interval, alert_days_before, is_active, 
                    created_at, created_by, updated_at
             FROM rotation_schedules
             WHERE vault_id = ?1 AND is_active = 1
             ORDER BY created_at DESC
             LIMIT 1"
        )?;

        let result = stmt.query_row([vault_id], |row| {
            Ok(RotationSchedule {
                schedule_id: row.get("schedule_id")?,
                vault_id: row.get("vault_id")?,
                rotation_interval: row.get("rotation_interval")?,
                alert_days_before: row.get("alert_days_before")?,
                is_active: row.get("is_active")?,
                created_at: row.get("created_at")?,
                created_by: row.get("created_by")?,
                updated_at: row.get("updated_at")?,
            })
        });

        match result {
            Ok(schedule) => Ok(Some(schedule)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // Task 3.6: Rotation compliance reporting
    pub fn get_rotation_compliance_metrics(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut metrics = HashMap::new();

        // Total passwords
        let total_passwords: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM vault_secrets WHERE secret_type = 'password'",
            [],
            |row| row.get(0),
        )?;
        metrics.insert("total_passwords".to_string(), serde_json::json!(total_passwords));

        // Overdue passwords
        let overdue_passwords: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM vault_secrets 
             WHERE secret_type = 'password' 
               AND next_rotation_due IS NOT NULL 
               AND next_rotation_due < datetime('now')",
            [],
            |row| row.get(0),
        )?;
        metrics.insert("overdue_passwords".to_string(), serde_json::json!(overdue_passwords));

        // Passwords due within 7 days
        let due_soon: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM vault_secrets 
             WHERE secret_type = 'password' 
               AND next_rotation_due IS NOT NULL 
               AND next_rotation_due BETWEEN datetime('now') AND datetime('now', '+7 days')",
            [],
            |row| row.get(0),
        )?;
        metrics.insert("due_within_7_days".to_string(), serde_json::json!(due_soon));

        // Average days since last rotation
        let avg_days_since_rotation: f64 = self.conn.query_row(
            "SELECT AVG(julianday('now') - julianday(last_rotated))
             FROM vault_secrets 
             WHERE secret_type = 'password' AND last_rotated IS NOT NULL",
            [],
            |row| row.get(0),
        ).unwrap_or(0.0);
        metrics.insert("avg_days_since_rotation".to_string(), serde_json::json!(avg_days_since_rotation.round()));

        // Compliance percentage
        let compliant = total_passwords - overdue_passwords;
        let compliance_percentage = if total_passwords > 0 {
            (compliant as f64 / total_passwords as f64 * 100.0).round()
        } else {
            100.0
        };
        metrics.insert("compliance_percentage".to_string(), serde_json::json!(compliance_percentage));

        Ok(metrics)
    }
}

// Task 4: Batch Rotation Workflow
pub struct BatchRotationService<'a> {
    conn: &'a Connection,
    rotation_service: PasswordRotationService<'a>,
}

impl<'a> BatchRotationService<'a> {
    pub fn new(
        conn: &'a Connection,
        vault_repo: Box<dyn VaultRepository + 'a>,
        audit_repo: Box<dyn AuditRepository + 'a>,
    ) -> Self {
        let rotation_service = PasswordRotationService::new(conn, vault_repo, audit_repo);
        Self { conn, rotation_service }
    }

    // Task 4.1: Create batch for coordinated operations
    pub fn create_batch(&self, request: CreateRotationBatchRequest) -> Result<RotationBatch> {
        debug!("Creating rotation batch: {}", request.batch_name);

        let now = Utc::now().to_rfc3339();
        
        self.conn.execute(
            "INSERT INTO rotation_batches (batch_name, created_by, started_at, status, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                request.batch_name,
                request.created_by,
                now,
                BatchStatus::Pending.to_string(),
                request.notes
            ],
        )?;

        let batch_id = self.conn.last_insert_rowid();
        
        info!("Created rotation batch {} by user {}", batch_id, request.created_by);

        Ok(RotationBatch {
            batch_id,
            batch_name: request.batch_name,
            created_by: request.created_by,
            started_at: now,
            completed_at: None,
            status: BatchStatus::Pending,
            notes: request.notes,
        })
    }

    // Task 4.2 & 4.3: Execute batch rotation with progress tracking
    pub fn execute_batch_rotation(&self, request: BatchRotationRequest, key: &DataKey) -> Result<()> {
        info!("Executing batch rotation {} with {} items", request.batch_id, request.items.len());

        // Semantics: items are independent. The outer transaction holds the whole batch, but
        // each item runs in its own SAVEPOINT, so a failing item is rolled back completely
        // while other items still succeed. The transaction is always committed at the end so
        // successful rotations and the batch outcome (status/notes) are persisted; partial
        // failure is reported to the caller as an Err after that commit.
        let tx = self.conn.unchecked_transaction()?;

        // Update batch status to in_progress
        tx.execute(
            "UPDATE rotation_batches SET status = ?1 WHERE batch_id = ?2",
            params![BatchStatus::InProgress.to_string(), request.batch_id],
        )?;

        let mut successful_count = 0;
        let mut failed_items = Vec::new();

        // Task 4.4: Process each item with transactional integrity
        for item in &request.items {
            // Task 4.6: Pre-flight validation
            match self.rotation_service.validate_rotation(item.secret_id, &item.new_password) {
                Ok(_) => {
                    tx.execute_batch("SAVEPOINT batch_item")?;
                    let result = self.rotation_service.rotate_password_in(&tx, PasswordRotationRequest {
                        secret_id: item.secret_id,
                        new_password: item.new_password.clone(),
                        rotation_reason: item.rotation_reason.clone(),
                        author_id: request.author_id,
                        batch_id: Some(request.batch_id),
                    }, key);
                    match result {
                        Ok(_) => {
                            tx.execute_batch("RELEASE SAVEPOINT batch_item")?;
                            successful_count += 1
                        }
                        Err(e) => {
                            tx.execute_batch("ROLLBACK TO SAVEPOINT batch_item; RELEASE SAVEPOINT batch_item")?;
                            warn!("Failed to rotate password for secret {}: {}", item.secret_id, e);
                            failed_items.push((item.secret_id, e.to_string()));
                        }
                    }
                }
                Err(e) => {
                    warn!("Validation failed for secret {}: {}", item.secret_id, e);
                    failed_items.push((item.secret_id, e.to_string()));
                }
            }
        }

        // Update batch status based on results
        let final_status = if failed_items.is_empty() {
            BatchStatus::Completed
        } else if successful_count == 0 {
            BatchStatus::Failed
        } else {
            BatchStatus::Completed // Partial success
        };

        let completed_at = Utc::now().to_rfc3339();
        let notes = if !failed_items.is_empty() {
            let failed_summary = failed_items.iter()
                .map(|(id, err)| format!("Secret {}: {}", id, err))
                .collect::<Vec<_>>()
                .join("; ");
            format!("Completed with {} successes and {} failures. Failures: {}", 
                    successful_count, failed_items.len(), failed_summary)
        } else {
            format!("Successfully rotated {} passwords", successful_count)
        };

        tx.execute(
            "UPDATE rotation_batches 
             SET status = ?1, completed_at = ?2, notes = COALESCE(notes || ' | ' || ?3, ?3)
             WHERE batch_id = ?4",
            params![final_status.to_string(), completed_at, notes, request.batch_id],
        )?;

        // Always commit: keeps successful rotations and the recorded batch outcome.
        tx.commit()?;
        if failed_items.is_empty() {
            info!("Batch rotation {} completed successfully", request.batch_id);
        } else {
            return Err(anyhow::anyhow!(
                "Batch rotation partially failed. {} succeeded, {} failed", 
                successful_count, failed_items.len()
            ));
        }

        Ok(())
    }

    // Task 4.5: Get batch rotation templates
    pub fn get_rotation_templates(&self) -> Vec<(String, String)> {
        vec![
            ("all_plc_credentials".to_string(), "Rotate all PLC passwords in an asset".to_string()),
            ("vendor_default_passwords".to_string(), "Rotate all vendor default passwords".to_string()),
            ("expired_passwords".to_string(), "Rotate all overdue passwords".to_string()),
            ("security_incident_response".to_string(), "Emergency rotation for security incident".to_string()),
            ("quarterly_rotation".to_string(), "Scheduled quarterly password rotation".to_string()),
        ]
    }

    // Get batch details
    pub fn get_batch(&self, batch_id: i64) -> Result<Option<RotationBatch>> {
        let mut stmt = self.conn.prepare(
            "SELECT batch_id, batch_name, created_by, started_at, completed_at, status, notes
             FROM rotation_batches WHERE batch_id = ?1"
        )?;

        let result = stmt.query_row([batch_id], |row| {
            let status_str: String = row.get("status")?;
            let status = BatchStatus::from_str(&status_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(0, "status".to_string(), rusqlite::types::Type::Text))?;

            Ok(RotationBatch {
                batch_id: row.get("batch_id")?,
                batch_name: row.get("batch_name")?,
                created_by: row.get("created_by")?,
                started_at: row.get("started_at")?,
                completed_at: row.get("completed_at")?,
                status,
                notes: row.get("notes")?,
            })
        });

        match result {
            Ok(batch) => Ok(Some(batch)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // Get batch history
    pub fn get_batch_history(&self, limit: i64) -> Result<Vec<RotationBatch>> {
        let mut stmt = self.conn.prepare(
            "SELECT batch_id, batch_name, created_by, started_at, completed_at, status, notes
             FROM rotation_batches
             ORDER BY started_at DESC
             LIMIT ?1"
        )?;

        let batch_iter = stmt.query_map([limit], |row| {
            let status_str: String = row.get("status")?;
            let status = BatchStatus::from_str(&status_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(0, "status".to_string(), rusqlite::types::Type::Text))?;

            Ok(RotationBatch {
                batch_id: row.get("batch_id")?,
                batch_name: row.get("batch_name")?,
                created_by: row.get("created_by")?,
                started_at: row.get("started_at")?,
                completed_at: row.get("completed_at")?,
                status,
                notes: row.get("notes")?,
            })
        })?;

        let mut batches = Vec::new();
        for batch in batch_iter {
            batches.push(batch?);
        }

        Ok(batches)
    }

    // Cancel a batch
    pub fn cancel_batch(&self, batch_id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE rotation_batches 
             SET status = ?1, completed_at = ?2, notes = COALESCE(notes || ' | Cancelled by user', 'Cancelled by user')
             WHERE batch_id = ?3 AND status IN ('pending', 'in_progress')",
            params![BatchStatus::Cancelled.to_string(), Utc::now().to_rfc3339(), batch_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Batch not found or already completed"));
        }

        info!("Cancelled batch rotation {}", batch_id);
        Ok(())
    }
}

/// Manual Debug impl: secret fields are redacted so the struct can be safely logged.
impl std::fmt::Debug for PasswordRotationRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasswordRotationRequest")
            .field("secret_id", &self.secret_id)
            .field("new_password", &"[REDACTED]")
            .field("rotation_reason", &self.rotation_reason)
            .field("author_id", &self.author_id)
            .field("batch_id", &self.batch_id)
            .finish()
    }
}

/// Manual Debug impl: secret fields are redacted so the struct can be safely logged.
impl std::fmt::Debug for BatchRotationItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchRotationItem")
            .field("secret_id", &self.secret_id)
            .field("new_password", &"[REDACTED]")
            .field("rotation_reason", &self.rotation_reason)
            .finish()
    }
}

#[cfg(test)]
mod redaction_tests {
    use super::*;

    #[test]
    fn debug_output_redacts_new_password() {
        let secret = "rotated-s3cret";
        let r = format!("{:?}", PasswordRotationRequest {
            secret_id: 1, new_password: secret.into(), rotation_reason: "r".into(),
            author_id: 1, batch_id: None,
        });
        assert!(!r.contains(secret));
        let b = format!("{:?}", BatchRotationItem {
            secret_id: 1, new_password: secret.into(), rotation_reason: "r".into(),
        });
        assert!(!b.contains(secret));
    }
}
