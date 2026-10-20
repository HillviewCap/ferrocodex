use crate::security::{SecurityValidator, SecurityValidationResult, FileIntegrityResult};
use crate::security::monitoring::{self, SecurityOverview};
use crate::auth::{LoginAttemptTracker, SessionManager, SessionToken};
use crate::database::Database;
use crate::audit::{AuditEventRequest, AuditEventType, AuditRepository, SqliteAuditRepository};
use crate::users::UserRole;
use crate::validation::InputSanitizer;
use std::sync::Mutex;
use tauri::{command, State};
use tracing::{error, info};
use serde::{Serialize, Deserialize};

/// Security classification levels enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityClassificationLevel {
    #[serde(rename = "PUBLIC")]
    Public,
    #[serde(rename = "INTERNAL")]
    Internal,
    #[serde(rename = "CONFIDENTIAL")]
    Confidential,
    #[serde(rename = "RESTRICTED")]
    Restricted,
    #[serde(rename = "SECRET")]
    Secret,
}

/// Audit event filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFilter {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub event_type: Option<String>,
    pub user_id: Option<u32>,
    pub limit: Option<u32>,
}

/// Date range for reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start_date: String,
    pub end_date: String,
}

/// Audit event for security operations (frontend-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendAuditEvent {
    pub id: u32,
    pub timestamp: String,
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(rename = "userId")]
    pub user_id: Option<u32>,
    pub details: String,
    pub result: String,
}

// Type aliases for Tauri state
type DatabaseState = Mutex<Option<Database>>;
type SessionManagerState = Mutex<SessionManager>;
type LoginAttemptTrackerState = Mutex<LoginAttemptTracker>;

/// Validate asset name against security patterns
#[command]
pub async fn validate_asset_name(
    token: String,
    name: String,
    session_state: State<'_, SessionManagerState>,
    _db_state: State<'_, DatabaseState>,
) -> Result<SecurityValidationResult, String> {
    info!("Validating asset name: {}", name);

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let _session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    drop(session_manager);

    // Create security validator
    let validator = SecurityValidator::new();
    
    // Perform validation
    let result = validator.validate_asset_name(&name)
        .map_err(|e| format!("Security validation failed: {}", e))?;

    // TODO: Add audit logging for asset name validation

    info!("Asset name validation completed: {} (valid: {}, error: {:?}, message: {:?})", 
        name, result.is_valid, result.error_code, result.error_message);
    Ok(result)
}

/// Sanitize asset name to make it compliant
#[command]
pub async fn sanitize_asset_name(
    token: String,
    name: String,
    session_state: State<'_, SessionManagerState>,
    _db_state: State<'_, DatabaseState>,
) -> Result<String, String> {
    info!("Sanitizing asset name: {}", name);

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let _session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    drop(session_manager);

    // Create security validator
    let validator = SecurityValidator::new();
    
    // Sanitize name
    let sanitized = validator.sanitize_asset_name(&name)
        .map_err(|e| format!("Name sanitization failed: {}", e))?;

    // TODO: Add audit logging for asset name sanitization

    info!("Asset name sanitized: {} -> {}", name, sanitized);
    Ok(sanitized)
}

/// Check if asset name is available (pattern compliance only)
#[command]
pub async fn check_name_compliance(
    token: String,
    name: String,
    session_state: State<'_, SessionManagerState>,
) -> Result<bool, String> {
    info!("Checking name compliance: {}", name);

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let _session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    drop(session_manager);

    // Create security validator
    let validator = SecurityValidator::new();
    
    // Check compliance
    let is_compliant = match validator.validate_asset_name(&name) {
        Ok(result) => result.is_valid,
        Err(_) => false,
    };

    info!("Name compliance check completed: {} (compliant: {})", name, is_compliant);
    Ok(is_compliant)
}

/// Generate compliant name suggestions
#[command]
pub async fn suggest_compliant_names(
    token: String,
    input: String,
    session_state: State<'_, SessionManagerState>,
) -> Result<Vec<String>, String> {
    info!("Generating compliant name suggestions for: {}", input);

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let _session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    drop(session_manager);

    // Create security validator
    let validator = SecurityValidator::new();
    
    // Get validation result with suggestions
    let result = validator.validate_asset_name(&input)
        .map_err(|e| format!("Validation failed: {}", e))?;

    let suggestions = if !result.is_valid {
        result.suggested_corrections
    } else {
        vec![input.clone()] // Already valid
    };

    info!("Generated {} suggestions for: {}", suggestions.len(), input);
    Ok(suggestions)
}

/// Validate file upload with comprehensive security checks
#[command]
pub async fn validate_file_upload(
    token: String,
    file_path: String,
    session_state: State<'_, SessionManagerState>,
    _db_state: State<'_, DatabaseState>,
) -> Result<FileIntegrityResult, String> {
    info!("Validating file upload: {}", file_path);

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let _session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    drop(session_manager);

    // Create security validator
    let validator = SecurityValidator::new();
    
    // Perform comprehensive validation
    let result = validator.validate_file_upload(&file_path)
        .map_err(|e| format!("File validation failed: {}", e))?;

    // TODO: Add audit logging for file upload validation

    info!("File upload validation completed: {} (passed: {})", file_path, result.security_scan_passed);
    Ok(result)
}

/// Calculate SHA-256 hash of file
#[command]
pub async fn calculate_file_hash(
    token: String,
    file_path: String,
    session_state: State<'_, SessionManagerState>,
    _db_state: State<'_, DatabaseState>,
) -> Result<String, String> {
    info!("Calculating file hash: {}", file_path);

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let _session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    drop(session_manager);

    // Create security validator
    let validator = SecurityValidator::new();
    
    // Calculate hash
    let hash = validator.calculate_file_hash(&file_path)
        .map_err(|e| format!("Hash calculation failed: {}", e))?;

    // TODO: Add audit logging for file hash calculation

    info!("File hash calculated: {} -> {}", file_path, hash);
    Ok(hash)
}

/// Sanitize filename for safe storage
#[command]
pub async fn sanitize_filename(
    token: String,
    filename: String,
    session_state: State<'_, SessionManagerState>,
) -> Result<String, String> {
    info!("Sanitizing filename: {}", filename);

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let _session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    drop(session_manager);

    // Create security validator
    let validator = SecurityValidator::new();
    
    // Sanitize filename using the validator's internal file security validator
    let file_validator = crate::security::FileSecurityValidator::new();
    let sanitized = file_validator.sanitize_filename(&filename)
        .map_err(|e| format!("Filename sanitization failed: {}", e))?;

    info!("Filename sanitized: {} -> {}", filename, sanitized);
    Ok(sanitized)
}

/// Verify file integrity against expected hash
#[command]
pub async fn verify_file_integrity(
    token: String,
    file_path: String,
    expected_hash: String,
    session_state: State<'_, SessionManagerState>,
    _db_state: State<'_, DatabaseState>,
) -> Result<bool, String> {
    info!("Verifying file integrity: {} (expected: {})", file_path, expected_hash);

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let _session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    drop(session_manager);

    // Create security validator
    let validator = SecurityValidator::new();
    
    // Verify integrity
    let is_valid = validator.verify_file_integrity(&file_path, &expected_hash)
        .map_err(|e| format!("File integrity verification failed: {}", e))?;

    // TODO: Add audit logging for file integrity verification

    info!("File integrity verification completed: {} (valid: {})", file_path, is_valid);
    Ok(is_valid)
}

/// Get audit events with filtering
#[command]
pub async fn get_audit_events(
    token: String,
    filter: Option<AuditFilter>,
    session_state: State<'_, SessionManagerState>,
    db_state: State<'_, DatabaseState>,
) -> Result<Vec<FrontendAuditEvent>, String> {
    info!("Getting audit events with filter: {:?}", filter);

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    // Check authorization (Admin only)
    if session.role.to_string() != "Administrator" {
        return Err("Administrator role required for audit log access".to_string());
    }

    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;
    
    let conn = db.get_connection();
    let audit_repo = SqliteAuditRepository::new(conn);

    // Get events with filtering
    let limit = filter.as_ref().and_then(|f| f.limit).map(|l| l as usize);
    let offset = None; // Can be extended to support pagination
    
    let events = audit_repo.get_events(limit, offset)
        .map_err(|e| format!("Failed to retrieve audit events: {}", e))?;

    // Map backend events to frontend-compatible events
    let frontend_events: Vec<FrontendAuditEvent> = events.into_iter().map(|event| {
        // Map event code to more readable event type
        let event_type = match event.event_code.as_str() {
            code if code.starts_with("AUTH_001") => "login",
            code if code.starts_with("AUTH_002") => "login",
            code if code.starts_with("AUTH_003") => "logout",
            code if code.starts_with("USER_") => "permission_change",
            code if code.starts_with("SEC_") => "security_validation",
            code if code.starts_with("VAULT_") => "vault_operation",
            code if code.starts_with("META_") => "metadata_operation",
            _ => "system_event",
        }.to_string();

        // The audit log has no result column; these codes record a refusal or failure
        let result = match event.event_code.as_str() {
            "AUTH_002" | "USER_006" | "VAULT_003" | "VAULT_014" | "FW_005" => "failure",
            _ => "success",
        }.to_string();
        
        FrontendAuditEvent {
            id: event.id as u32,
            timestamp: event.timestamp,
            event_type,
            user_id: event.user_id.map(|id| id as u32),
            details: event.description,
            result,
        }
    }).collect();

    // Apply additional filters if needed
    let filtered_events = if let Some(filter) = filter {
        frontend_events.into_iter().filter(|event| {
            // Filter by event type if specified
            if let Some(ref filter_type) = filter.event_type {
                if event.event_type != *filter_type {
                    return false;
                }
            }
            
            // Filter by user ID if specified
            if let Some(user_id) = filter.user_id {
                if event.user_id != Some(user_id) {
                    return false;
                }
            }
            
            // TODO: Add date range filtering when start_date and end_date are provided
            
            true
        }).collect()
    } else {
        frontend_events
    };

    info!("Retrieved {} audit events", filtered_events.len());
    Ok(filtered_events)
}

/// Export audit log in specified format
#[command]
pub async fn export_audit_log(
    token: String,
    start_date: String,
    end_date: String,
    format: String,
    filters: Option<AuditFilter>,
    session_state: State<'_, SessionManagerState>,
    db_state: State<'_, DatabaseState>,
) -> Result<String, String> {
    info!("Exporting audit log in {} format from {} to {}", format, start_date, end_date);

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    // Check authorization (Admin only)
    if session.role.to_string() != "Administrator" {
        return Err("Administrator role required for audit log export".to_string());
    }

    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;
    
    let conn = db.get_connection();
    let audit_repo = SqliteAuditRepository::new(conn);

    // Get all events (can be optimized with date filtering in SQL)
    let events = audit_repo.get_events(None, None)
        .map_err(|e| format!("Failed to retrieve audit events: {}", e))?;

    // Filter by date range
    let filtered_events: Vec<_> = events.into_iter().filter(|event| {
        // Simple date comparison (can be improved)
        event.timestamp >= start_date && event.timestamp <= end_date
    }).collect();

    // Count before formatting
    let event_count = filtered_events.len();
    
    // Format output based on requested format
    let output = match format.as_str() {
        "csv" => {
            let mut csv_output = String::from("ID,Timestamp,Event Type,User ID,Description,Result\n");
            for event in &filtered_events {
                csv_output.push_str(&format!(
                    "{},{},{},{:?},\"{}\",{}\n",
                    event.id,
                    event.timestamp,
                    event.event_code,
                    event.user_id.unwrap_or(0),
                    event.description.replace("\"", "\"\""),
                    "success" // Default result as the audit module doesn't store result directly
                ));
            }
            csv_output
        },
        "json" => {
            serde_json::to_string_pretty(&filtered_events)
                .map_err(|e| format!("Failed to serialize to JSON: {}", e))?
        },
        "pdf" => {
            // PDF export would require additional dependencies
            return Err("PDF export not yet implemented".to_string());
        },
        _ => return Err(format!("Unsupported export format: {}", format)),
    };

    // TODO: Actually save to file and return file path
    info!("Exported {} audit events in {} format", event_count, format);
    Ok(output)
}

/// Validates the session and requires an administrator. Takes the session
/// lock only, and releases it before returning.
fn require_admin(session_state: &SessionManagerState, token: &str, action: &str) -> Result<SessionToken, String> {
    let session_manager = session_state.lock()
        .map_err(|_| "Failed to acquire session manager lock".to_string())?;
    let session = match session_manager.validate_session(token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => {
            error!("Session validation error: {}", e);
            return Err("Session validation error".to_string());
        }
    };
    drop(session_manager);

    if session.role != UserRole::Administrator {
        return Err(format!("Administrator role required for {}", action));
    }
    Ok(session)
}

fn locked_usernames(attempt_tracker: &LoginAttemptTrackerState) -> Result<Vec<String>, String> {
    Ok(attempt_tracker.lock()
        .map_err(|_| "Failed to acquire attempt tracker lock".to_string())?
        .locked_usernames())
}

/// Security overview for the monitoring dashboard over the last 24h, 7d or 30d
#[command]
pub async fn get_security_overview(
    token: String,
    period: String,
    session_state: State<'_, SessionManagerState>,
    db_state: State<'_, DatabaseState>,
    attempt_tracker: State<'_, LoginAttemptTrackerState>,
) -> Result<SecurityOverview, String> {
    require_admin(&session_state, &token, "the security overview")?;
    monitoring::parse_period(&period).map_err(|e| e.to_string())?;
    let locked = locked_usernames(&attempt_tracker)?;

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;

    monitoring::compute_overview(db.get_connection(), &period, locked).map_err(|e| {
        error!("Failed to compute security overview: {}", e);
        format!("Failed to compute security overview: {}", e)
    })
}

/// Writes the security overview and every security event in the period as
/// JSON to a path the user chose, and returns that path
#[command]
pub async fn export_security_report(
    token: String,
    period: String,
    file_path: String,
    session_state: State<'_, SessionManagerState>,
    db_state: State<'_, DatabaseState>,
    attempt_tracker: State<'_, LoginAttemptTrackerState>,
) -> Result<String, String> {
    let session = require_admin(&session_state, &token, "security report export")?;
    monitoring::parse_period(&period).map_err(|e| e.to_string())?;

    let file_path = file_path.trim().to_string();
    if file_path.is_empty() {
        return Err("Export path cannot be empty".to_string());
    }
    if let Err(e) = InputSanitizer::validate_file_path(&file_path) {
        error!("Invalid export path: {}", e);
        return Err(format!("Invalid export path: {}", e));
    }
    let is_json = std::path::Path::new(&file_path)
        .extension()
        .map_or(false, |ext| ext.eq_ignore_ascii_case("json"));
    if !is_json {
        return Err("Invalid export path: the security report must be saved as a .json file".to_string());
    }
    // Overwriting a file the user chose in the save dialog is fine; anything
    // else at that path (a directory, a symlink, a device) is refused
    match std::fs::symlink_metadata(&file_path) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err("Invalid export path: it exists and is not a regular file".to_string());
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Failed to check export path: {}", e)),
    }
    let locked = locked_usernames(&attempt_tracker)?;

    let db_guard = db_state.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    let db = db_guard.as_ref().ok_or_else(|| "Database not initialized".to_string())?;

    let report = monitoring::build_report(db.get_connection(), &period, locked).map_err(|e| {
        error!("Failed to build security report: {}", e);
        format!("Failed to build security report: {}", e)
    })?;
    let json = serde_json::to_string_pretty(&report)
        .map_err(|e| format!("Failed to serialize security report: {}", e))?;
    std::fs::write(&file_path, json).map_err(|e| {
        error!("Failed to write security report to {}: {}", file_path, e);
        format!("Failed to write security report: {}", e)
    })?;

    let audit_event = AuditEventRequest {
        event_type: AuditEventType::SecurityReportExported,
        user_id: Some(session.user_id),
        username: Some(session.username.clone()),
        admin_user_id: Some(session.user_id),
        admin_username: Some(session.username.clone()),
        target_user_id: None,
        target_username: None,
        description: format!(
            "Administrator '{}' exported the security report for the last {} ({} events) to {}",
            session.username, period, report.events.len(), file_path
        ),
        metadata: Some(serde_json::json!({
            "period": period,
            "events": report.events.len(),
            "file_path": file_path,
        }).to_string()),
        ip_address: None,
        user_agent: None,
    };
    if let Err(e) = SqliteAuditRepository::new(db.get_connection()).log_event(&audit_event) {
        error!("Failed to log audit event: {}", e);
    }

    info!("Security report for {} exported by {} to {}", period, session.username, file_path);
    Ok(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{create_admin_account, create_engineer_user, login};
    use crate::validation::RateLimiter;
    use std::time::Duration;
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
    use tauri::Manager;

    type RateLimiterState = Mutex<RateLimiter>;

    fn setup_test_app() -> tauri::App<MockRuntime> {
        mock_builder()
            .manage(DatabaseState::new(Some(Database::new(":memory:".into()).unwrap())))
            .manage(SessionManagerState::new(SessionManager::new()))
            .manage(LoginAttemptTrackerState::new(LoginAttemptTracker::new()))
            .manage(RateLimiterState::new(RateLimiter::new(100, Duration::from_secs(60))))
            .build(mock_context(noop_assets()))
            .unwrap()
    }

    async fn admin_token(app: &tauri::App<MockRuntime>) -> String {
        create_admin_account("admin".to_string(), "Admin-Pass-123".to_string(), app.state(), app.state())
            .await.unwrap().token
    }

    #[tokio::test]
    async fn test_overview_reflects_failed_logins_and_lockout() {
        let app = setup_test_app();
        let token = admin_token(&app).await;
        for _ in 0..5 {
            let _ = login("admin".to_string(), "Wrong-Pass-999".to_string(), app.state(), app.state(), app.state()).await;
        }

        let overview = get_security_overview(token, "24h".to_string(), app.state(), app.state(), app.state())
            .await.unwrap();

        assert_eq!(overview.authentication.failed_logins, 5);
        assert_eq!(overview.authentication.lockouts, 1);
        assert_eq!(overview.authentication.currently_locked_accounts, vec!["admin".to_string()]);
        assert_eq!(overview.users.active_administrators, 1);
        assert_eq!(overview.status, "warning");
        let ids: Vec<&str> = overview.findings.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"accounts-locked") && ids.contains(&"failed-logins"), "{:?}", ids);
        assert_eq!(overview.recent_events[0].event_code, "USER_006");
    }

    #[tokio::test]
    async fn test_overview_rejects_bad_period_and_engineers() {
        let app = setup_test_app();
        let token = admin_token(&app).await;
        assert!(get_security_overview(token.clone(), "1y".to_string(), app.state(), app.state(), app.state())
            .await.is_err());

        create_engineer_user(
            token, "engineer".to_string(), "Engineer-Pass-123".to_string(), app.state(), app.state(), app.state(),
        ).await.unwrap();
        let engineer = login(
            "engineer".to_string(), "Engineer-Pass-123".to_string(), app.state(), app.state(), app.state(),
        ).await.unwrap();
        let result = get_security_overview(engineer.token.clone(), "24h".to_string(), app.state(), app.state(), app.state()).await;
        assert_eq!(result.err().as_deref(), Some("Administrator role required for the security overview"));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.json").to_string_lossy().to_string();
        assert!(export_security_report(engineer.token, "24h".to_string(), path.clone(), app.state(), app.state(), app.state())
            .await.is_err());
        assert!(!std::path::Path::new(&path).exists());
    }

    #[tokio::test]
    async fn test_export_writes_report_and_audits_it() {
        let app = setup_test_app();
        let token = admin_token(&app).await;
        let _ = login("ghost".to_string(), "Wrong-Pass-999".to_string(), app.state(), app.state(), app.state()).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("security-report-7d.json").to_string_lossy().to_string();

        let written = export_security_report(token, "7d".to_string(), path.clone(), app.state(), app.state(), app.state())
            .await.unwrap();

        assert_eq!(written, path);
        let report: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(report["overview"]["period"], "7d");
        assert_eq!(report["overview"]["authentication"]["failedLogins"], 1);
        assert_eq!(report["events"][0]["eventCode"], "AUTH_002");

        let db_state = app.state::<DatabaseState>();
        let db_guard = db_state.lock().unwrap();
        let exports: i64 = db_guard.as_ref().unwrap().get_connection()
            .query_row("SELECT COUNT(*) FROM audit_events WHERE event_code = 'SEC_006'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(exports, 1);
    }

    async fn export_to(app: &tauri::App<MockRuntime>, token: &str, path: &std::path::Path) -> Result<String, String> {
        export_security_report(
            token.to_string(), "24h".to_string(), path.to_string_lossy().to_string(), app.state(), app.state(), app.state(),
        ).await
    }

    #[tokio::test]
    async fn test_export_requires_json_extension() {
        let app = setup_test_app();
        let token = admin_token(&app).await;
        let dir = tempfile::tempdir().unwrap();

        for name in ["report.txt", "report", "report.json.exe"] {
            let path = dir.path().join(name);
            let result = export_to(&app, &token, &path).await;
            assert!(result.unwrap_err().contains(".json"), "{}", name);
            assert!(!path.exists());
        }
        assert!(export_to(&app, &token, &dir.path().join("REPORT.JSON")).await.is_ok());
    }

    #[tokio::test]
    async fn test_export_overwrites_regular_file_but_refuses_directory() {
        let app = setup_test_app();
        let token = admin_token(&app).await;
        let dir = tempfile::tempdir().unwrap();

        let existing = dir.path().join("old.json");
        std::fs::write(&existing, "old").unwrap();
        assert!(export_to(&app, &token, &existing).await.is_ok());
        assert_ne!(std::fs::read_to_string(&existing).unwrap(), "old");

        let directory = dir.path().join("folder.json");
        std::fs::create_dir(&directory).unwrap();
        let result = export_to(&app, &token, &directory).await;
        assert!(result.unwrap_err().contains("not a regular file"));
        assert!(directory.is_dir());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_export_refuses_symlink() {
        let app = setup_test_app();
        let token = admin_token(&app).await;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        std::fs::write(&target, "keep").unwrap();
        let link = dir.path().join("link.json");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let result = export_to(&app, &token, &link).await;

        assert!(result.unwrap_err().contains("not a regular file"));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "keep");
    }

    #[tokio::test]
    async fn test_export_rejects_traversal_path() {
        let app = setup_test_app();
        let token = admin_token(&app).await;
        let result = export_security_report(
            token, "24h".to_string(), "../../report.json".to_string(), app.state(), app.state(), app.state(),
        ).await;
        assert!(result.unwrap_err().starts_with("Invalid export path"));
    }
}
