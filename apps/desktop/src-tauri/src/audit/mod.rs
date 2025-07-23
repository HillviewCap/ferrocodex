use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    UserCreated,
    UserDeactivated,
    UserReactivated,
    UserLoginSuccessful,
    UserLoginFailed,
    UserLogout,
    UserPasswordChanged,
    UserRoleChanged,
    UserAccountLocked,
    UserAccountUnlocked,
    UserSessionExpired,
    UserSessionInvalidated,
    SystemStartup,
    SystemShutdown,
    DatabaseOperation,
    SecurityViolation,
    FirmwareUpload,
    FirmwareDelete,
    FirmwareAnalysisStarted,
    FirmwareAnalysisCompleted,
    FirmwareAnalysisFailed,
}

impl fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditEventType::UserCreated => write!(f, "USER_001"),
            AuditEventType::UserDeactivated => write!(f, "USER_002"),
            AuditEventType::UserReactivated => write!(f, "USER_003"),
            AuditEventType::UserLoginSuccessful => write!(f, "AUTH_001"),
            AuditEventType::UserLoginFailed => write!(f, "AUTH_002"),
            AuditEventType::UserLogout => write!(f, "AUTH_003"),
            AuditEventType::UserPasswordChanged => write!(f, "USER_004"),
            AuditEventType::UserRoleChanged => write!(f, "USER_005"),
            AuditEventType::UserAccountLocked => write!(f, "USER_006"),
            AuditEventType::UserAccountUnlocked => write!(f, "USER_007"),
            AuditEventType::UserSessionExpired => write!(f, "AUTH_004"),
            AuditEventType::UserSessionInvalidated => write!(f, "AUTH_005"),
            AuditEventType::SystemStartup => write!(f, "SYS_001"),
            AuditEventType::SystemShutdown => write!(f, "SYS_002"),
            AuditEventType::DatabaseOperation => write!(f, "DB_001"),
            AuditEventType::SecurityViolation => write!(f, "SEC_001"),
            AuditEventType::FirmwareUpload => write!(f, "FW_001"),
            AuditEventType::FirmwareDelete => write!(f, "FW_002"),
            AuditEventType::FirmwareAnalysisStarted => write!(f, "FW_003"),
            AuditEventType::FirmwareAnalysisCompleted => write!(f, "FW_004"),
            AuditEventType::FirmwareAnalysisFailed => write!(f, "FW_005"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: i64,
    pub event_type: AuditEventType,
    pub event_code: String,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub admin_user_id: Option<i64>,
    pub admin_username: Option<String>,
    pub target_user_id: Option<i64>,
    pub target_username: Option<String>,
    pub description: String,
    pub metadata: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub timestamp: String,
}

pub trait AuditRepository {
    fn initialize_schema(&self) -> Result<()>;
    fn log_event(&self, event: &AuditEventRequest) -> Result<AuditEvent>;
    fn get_events(&self, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<AuditEvent>>;
    fn get_events_by_user(&self, user_id: i64) -> Result<Vec<AuditEvent>>;
    fn get_events_by_type(&self, event_type: &AuditEventType) -> Result<Vec<AuditEvent>>;
    fn cleanup_old_events(&self, days_to_keep: u32) -> Result<u64>;
}

#[derive(Debug, Clone)]
pub struct AuditEventRequest {
    pub event_type: AuditEventType,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub admin_user_id: Option<i64>,
    pub admin_username: Option<String>,
    pub target_user_id: Option<i64>,
    pub target_username: Option<String>,
    pub description: String,
    pub metadata: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

pub struct SqliteAuditRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteAuditRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    fn row_to_audit_event(row: &Row) -> rusqlite::Result<AuditEvent> {
        let event_type_str: String = row.get("event_type")?;
        let event_type = match event_type_str.as_str() {
            "USER_001" => AuditEventType::UserCreated,
            "USER_002" => AuditEventType::UserDeactivated,
            "USER_003" => AuditEventType::UserReactivated,
            "AUTH_001" => AuditEventType::UserLoginSuccessful,
            "AUTH_002" => AuditEventType::UserLoginFailed,
            "AUTH_003" => AuditEventType::UserLogout,
            "USER_004" => AuditEventType::UserPasswordChanged,
            "USER_005" => AuditEventType::UserRoleChanged,
            "USER_006" => AuditEventType::UserAccountLocked,
            "USER_007" => AuditEventType::UserAccountUnlocked,
            "AUTH_004" => AuditEventType::UserSessionExpired,
            "AUTH_005" => AuditEventType::UserSessionInvalidated,
            "SYS_001" => AuditEventType::SystemStartup,
            "SYS_002" => AuditEventType::SystemShutdown,
            "DB_001" => AuditEventType::DatabaseOperation,
            "SEC_001" => AuditEventType::SecurityViolation,
            _ => return Err(rusqlite::Error::InvalidColumnType(0, "event_type".to_string(), rusqlite::types::Type::Text)),
        };

        Ok(AuditEvent {
            id: row.get("id")?,
            event_type,
            event_code: row.get("event_code")?,
            user_id: row.get("user_id")?,
            username: row.get("username")?,
            admin_user_id: row.get("admin_user_id")?,
            admin_username: row.get("admin_username")?,
            target_user_id: row.get("target_user_id")?,
            target_username: row.get("target_username")?,
            description: row.get("description")?,
            metadata: row.get("metadata")?,
            ip_address: row.get("ip_address")?,
            user_agent: row.get("user_agent")?,
            timestamp: row.get("timestamp")?,
        })
    }
}

impl<'a> AuditRepository for SqliteAuditRepository<'a> {
    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS audit_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                event_code TEXT NOT NULL,
                user_id INTEGER,
                username TEXT,
                admin_user_id INTEGER,
                admin_username TEXT,
                target_user_id INTEGER,
                target_username TEXT,
                description TEXT NOT NULL,
                metadata TEXT,
                ip_address TEXT,
                user_agent TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_audit_events_timestamp ON audit_events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_audit_events_user_id ON audit_events(user_id);
            CREATE INDEX IF NOT EXISTS idx_audit_events_event_type ON audit_events(event_type);
            CREATE INDEX IF NOT EXISTS idx_audit_events_admin_user_id ON audit_events(admin_user_id);
            CREATE INDEX IF NOT EXISTS idx_audit_events_target_user_id ON audit_events(target_user_id);
            "#,
        )?;
        Ok(())
    }

    fn log_event(&self, event: &AuditEventRequest) -> Result<AuditEvent> {
        let event_code = event.event_type.to_string();
        
        // Log to structured logger as well
        info!(
            event_code = %event_code,
            user_id = event.user_id,
            username = event.username.as_deref().unwrap_or("unknown"),
            admin_user_id = event.admin_user_id,
            admin_username = event.admin_username.as_deref().unwrap_or("unknown"),
            target_user_id = event.target_user_id,
            target_username = event.target_username.as_deref().unwrap_or("unknown"),
            description = %event.description,
            "Audit event logged"
        );

        let mut stmt = self.conn.prepare(
            "INSERT INTO audit_events (
                event_type, event_code, user_id, username, admin_user_id, admin_username,
                target_user_id, target_username, description, metadata, ip_address, user_agent
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) RETURNING *"
        )?;

        let audit_event = stmt.query_row(
            (
                &event_code,
                &event_code,
                &event.user_id,
                &event.username,
                &event.admin_user_id,
                &event.admin_username,
                &event.target_user_id,
                &event.target_username,
                &event.description,
                &event.metadata,
                &event.ip_address,
                &event.user_agent,
            ),
            Self::row_to_audit_event,
        )?;

        Ok(audit_event)
    }

    fn get_events(&self, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<AuditEvent>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let mut stmt = self.conn.prepare(
            "SELECT id, event_type, event_code, user_id, username, admin_user_id, admin_username,
                    target_user_id, target_username, description, metadata, ip_address, user_agent, timestamp
             FROM audit_events
             ORDER BY timestamp DESC
             LIMIT ?1 OFFSET ?2"
        )?;

        let event_iter = stmt.query_map([limit, offset], Self::row_to_audit_event)?;
        let mut events = Vec::new();

        for event in event_iter {
            events.push(event?);
        }

        Ok(events)
    }

    fn get_events_by_user(&self, user_id: i64) -> Result<Vec<AuditEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, event_type, event_code, user_id, username, admin_user_id, admin_username,
                    target_user_id, target_username, description, metadata, ip_address, user_agent, timestamp
             FROM audit_events
             WHERE user_id = ?1 OR admin_user_id = ?1 OR target_user_id = ?1
             ORDER BY timestamp DESC"
        )?;

        let event_iter = stmt.query_map([user_id], Self::row_to_audit_event)?;
        let mut events = Vec::new();

        for event in event_iter {
            events.push(event?);
        }

        Ok(events)
    }

    fn get_events_by_type(&self, event_type: &AuditEventType) -> Result<Vec<AuditEvent>> {
        let event_code = event_type.to_string();
        
        let mut stmt = self.conn.prepare(
            "SELECT id, event_type, event_code, user_id, username, admin_user_id, admin_username,
                    target_user_id, target_username, description, metadata, ip_address, user_agent, timestamp
             FROM audit_events
             WHERE event_type = ?1
             ORDER BY timestamp DESC"
        )?;

        let event_iter = stmt.query_map([event_code], Self::row_to_audit_event)?;
        let mut events = Vec::new();

        for event in event_iter {
            events.push(event?);
        }

        Ok(events)
    }

    fn cleanup_old_events(&self, days_to_keep: u32) -> Result<u64> {
        let rows_deleted = self.conn.execute(
            "DELETE FROM audit_events WHERE timestamp < datetime('now', '-' || ?1 || ' days')",
            [days_to_keep],
        )?;

        info!(
            days_to_keep = days_to_keep,
            rows_deleted = rows_deleted,
            "Old audit events cleaned up"
        );

        Ok(rows_deleted as u64)
    }
}

// Helper function to create audit events
pub fn create_user_created_event(
    admin_user_id: i64,
    admin_username: &str,
    target_user_id: i64,
    target_username: &str,
) -> AuditEventRequest {
    AuditEventRequest {
        event_type: AuditEventType::UserCreated,
        user_id: Some(admin_user_id),
        username: Some(admin_username.to_string()),
        admin_user_id: Some(admin_user_id),
        admin_username: Some(admin_username.to_string()),
        target_user_id: Some(target_user_id),
        target_username: Some(target_username.to_string()),
        description: format!("Administrator '{}' created engineer account '{}'", admin_username, target_username),
        metadata: None,
        ip_address: None,
        user_agent: None,
    }
}

pub fn create_user_deactivated_event(
    admin_user_id: i64,
    admin_username: &str,
    target_user_id: i64,
    target_username: &str,
) -> AuditEventRequest {
    AuditEventRequest {
        event_type: AuditEventType::UserDeactivated,
        user_id: Some(admin_user_id),
        username: Some(admin_username.to_string()),
        admin_user_id: Some(admin_user_id),
        admin_username: Some(admin_username.to_string()),
        target_user_id: Some(target_user_id),
        target_username: Some(target_username.to_string()),
        description: format!("Administrator '{}' deactivated user '{}'", admin_username, target_username),
        metadata: None,
        ip_address: None,
        user_agent: None,
    }
}

pub fn create_user_reactivated_event(
    admin_user_id: i64,
    admin_username: &str,
    target_user_id: i64,
    target_username: &str,
) -> AuditEventRequest {
    AuditEventRequest {
        event_type: AuditEventType::UserReactivated,
        user_id: Some(admin_user_id),
        username: Some(admin_username.to_string()),
        admin_user_id: Some(admin_user_id),
        admin_username: Some(admin_username.to_string()),
        target_user_id: Some(target_user_id),
        target_username: Some(target_username.to_string()),
        description: format!("Administrator '{}' reactivated user '{}'", admin_username, target_username),
        metadata: None,
        ip_address: None,
        user_agent: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use rusqlite::Connection;

    fn setup_test_audit_db() -> (NamedTempFile, Connection) {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        
        let audit_repo = SqliteAuditRepository::new(&conn);
        audit_repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_audit_event_creation() {
        let (_temp_file, conn) = setup_test_audit_db();
        let audit_repo = SqliteAuditRepository::new(&conn);

        let event_request = AuditEventRequest {
            event_type: AuditEventType::UserCreated,
            user_id: Some(1),
            username: Some("admin".to_string()),
            admin_user_id: Some(1),
            admin_username: Some("admin".to_string()),
            target_user_id: Some(2),
            target_username: Some("engineer".to_string()),
            description: "Test user creation".to_string(),
            metadata: None,
            ip_address: None,
            user_agent: None,
        };

        let event = audit_repo.log_event(&event_request).unwrap();
        
        assert!(event.id > 0);
        assert_eq!(event.event_code, "USER_001");
        assert_eq!(event.user_id, Some(1));
        assert_eq!(event.username, Some("admin".to_string()));
        assert_eq!(event.description, "Test user creation");
    }

    #[test]
    fn test_get_events() {
        let (_temp_file, conn) = setup_test_audit_db();
        let audit_repo = SqliteAuditRepository::new(&conn);

        // Create test events
        let event1 = AuditEventRequest {
            event_type: AuditEventType::UserCreated,
            user_id: Some(1),
            username: Some("admin".to_string()),
            admin_user_id: Some(1),
            admin_username: Some("admin".to_string()),
            target_user_id: Some(2),
            target_username: Some("engineer".to_string()),
            description: "Test user creation 1".to_string(),
            metadata: None,
            ip_address: None,
            user_agent: None,
        };

        let event2 = AuditEventRequest {
            event_type: AuditEventType::UserDeactivated,
            user_id: Some(1),
            username: Some("admin".to_string()),
            admin_user_id: Some(1),
            admin_username: Some("admin".to_string()),
            target_user_id: Some(2),
            target_username: Some("engineer".to_string()),
            description: "Test user deactivation".to_string(),
            metadata: None,
            ip_address: None,
            user_agent: None,
        };

        audit_repo.log_event(&event1).unwrap();
        audit_repo.log_event(&event2).unwrap();

        let events = audit_repo.get_events(None, None).unwrap();
        assert_eq!(events.len(), 2);
        
        // Events should be in reverse chronological order
        assert_eq!(events[0].description, "Test user deactivation");
        assert_eq!(events[1].description, "Test user creation 1");
    }

    #[test]
    fn test_get_events_by_user() {
        let (_temp_file, conn) = setup_test_audit_db();
        let audit_repo = SqliteAuditRepository::new(&conn);

        let event = AuditEventRequest {
            event_type: AuditEventType::UserCreated,
            user_id: Some(1),
            username: Some("admin".to_string()),
            admin_user_id: Some(1),
            admin_username: Some("admin".to_string()),
            target_user_id: Some(2),
            target_username: Some("engineer".to_string()),
            description: "Test user creation".to_string(),
            metadata: None,
            ip_address: None,
            user_agent: None,
        };

        audit_repo.log_event(&event).unwrap();

        let events = audit_repo.get_events_by_user(1).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].user_id, Some(1));
    }

    #[test]
    fn test_audit_event_type_display() {
        assert_eq!(AuditEventType::UserCreated.to_string(), "USER_001");
        assert_eq!(AuditEventType::UserDeactivated.to_string(), "USER_002");
        assert_eq!(AuditEventType::UserReactivated.to_string(), "USER_003");
    }

    #[test]
    fn test_helper_functions() {
        let event = create_user_created_event(1, "admin", 2, "engineer");
        
        assert!(matches!(event.event_type, AuditEventType::UserCreated));
        assert_eq!(event.admin_user_id, Some(1));
        assert_eq!(event.admin_username, Some("admin".to_string()));
        assert_eq!(event.target_user_id, Some(2));
        assert_eq!(event.target_username, Some("engineer".to_string()));
        assert!(event.description.contains("admin"));
        assert!(event.description.contains("engineer"));
    }
}