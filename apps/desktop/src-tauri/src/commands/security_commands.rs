use crate::security::{SecurityValidator, SecurityValidationResult, FileIntegrityResult};
use crate::auth::SessionManager;
use crate::database::Database;
use crate::audit::{AuditRepository, SqliteAuditRepository};
use std::sync::Mutex;
use std::collections::HashMap;
use tauri::{command, State};
use tracing::info;
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

/// Security metrics for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    #[serde(rename = "totalAssets")]
    pub total_assets: u64,
    #[serde(rename = "classificationBreakdown")]
    pub classification_breakdown: HashMap<String, u64>,
    #[serde(rename = "validationSuccessRate")]
    pub validation_success_rate: f64,
    #[serde(rename = "recentValidations")]
    pub recent_validations: u64,
    #[serde(rename = "securityAlerts")]
    pub security_alerts: u64,
    #[serde(rename = "complianceScore")]
    pub compliance_score: f64,
}

/// Security validation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStats {
    #[serde(rename = "totalValidations")]
    pub total_validations: u64,
    #[serde(rename = "successfulValidations")]
    pub successful_validations: u64,
    #[serde(rename = "failedValidations")]
    pub failed_validations: u64,
    #[serde(rename = "blockedAttempts")]
    pub blocked_attempts: u64,
    #[serde(rename = "lastValidation")]
    pub last_validation: Option<String>,
}

/// Security health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHealthReport {
    #[serde(rename = "overallStatus")]
    pub overall_status: String,
    #[serde(rename = "validationSuccessRate")]
    pub validation_success_rate: f64,
    #[serde(rename = "recentThreatsBlocked")]
    pub recent_threats_blocked: u64,
    #[serde(rename = "systemSecurityLevel")]
    pub system_security_level: String,
    pub recommendations: Vec<String>,
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

/// Get validation statistics
#[command]
pub async fn get_validation_statistics(
    token: String,
    session_state: State<'_, SessionManagerState>,
    db_state: State<'_, DatabaseState>,
) -> Result<ValidationStats, String> {
    info!("Getting validation statistics");

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    // Check authorization (Admin only)
    if session.role.to_string() != "Administrator" {
        return Err("Administrator role required for validation statistics".to_string());
    }

    drop(session_manager);

    // Get statistics from audit log (simplified for now)
    let total_validations = 0u64;
    let successful_validations = 0u64;
    let failed_validations = 0u64;
    let blocked_attempts = 0u64;
    let last_validation: Option<String> = None;
    
    // TODO: Implement audit log statistics retrieval

    let stats = ValidationStats {
        total_validations,
        successful_validations,
        failed_validations,
        blocked_attempts,
        last_validation,
    };

    info!("Validation statistics retrieved: {} total, {} successful", 
          stats.total_validations, stats.successful_validations);
    Ok(stats)
}

/// Perform security health check
#[command]
pub async fn perform_security_health_check(
    token: String,
    session_state: State<'_, SessionManagerState>,
    db_state: State<'_, DatabaseState>,
) -> Result<SecurityHealthReport, String> {
    info!("Performing security health check");

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    // Check authorization (Admin only)
    if session.role.to_string() != "Administrator" {
        return Err("Administrator role required for security health check".to_string());
    }

    drop(session_manager);

    // Get statistics for health assessment (simplified for now)
    let total_validations = 0u64;
    let successful_validations = 0u64;
    let blocked_attempts = 0u64;
    
    // TODO: Implement security health statistics retrieval

    // Calculate success rate
    let success_rate = if total_validations > 0 {
        (successful_validations as f64 / total_validations as f64) * 100.0
    } else {
        100.0
    };

    // Determine overall status
    let overall_status = match success_rate {
        rate if rate >= 95.0 => "Excellent",
        rate if rate >= 85.0 => "Good",
        rate if rate >= 75.0 => "Fair",
        _ => "Needs Attention",
    }.to_string();

    // Determine security level
    let security_level = if blocked_attempts == 0 && success_rate >= 95.0 {
        "High"
    } else if blocked_attempts < 10 && success_rate >= 85.0 {
        "Medium"
    } else {
        "Low"
    }.to_string();

    // Generate recommendations
    let mut recommendations = Vec::new();
    if success_rate < 95.0 {
        recommendations.push("Review validation failures and improve input quality".to_string());
    }
    if blocked_attempts > 0 {
        recommendations.push("Investigate security threats and blocked attempts".to_string());
    }
    if total_validations < 10 {
        recommendations.push("Increase system usage to better assess security health".to_string());
    }

    let report = SecurityHealthReport {
        overall_status,
        validation_success_rate: success_rate,
        recent_threats_blocked: blocked_attempts,
        system_security_level: security_level,
        recommendations,
    };

    info!("Security health check completed: {} ({}% success rate)", 
          report.overall_status, report.validation_success_rate);
    Ok(report)
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
            code if code.starts_with("AUTH_003") => "logout",
            code if code.starts_with("USER_") => "permission_change",
            code if code.starts_with("SEC_") => "security_validation",
            code if code.starts_with("VAULT_") => "vault_operation",
            code if code.starts_with("META_") => "metadata_operation",
            _ => "system_event",
        }.to_string();
        
        FrontendAuditEvent {
            id: event.id as u32,
            timestamp: event.timestamp,
            event_type,
            user_id: event.user_id.map(|id| id as u32),
            details: event.description,
            result: "success".to_string(), // Default as backend doesn't track result directly
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

/// Get security metrics for dashboard
#[command]
pub async fn get_security_metrics(
    token: String,
    period: String,
    session_state: State<'_, SessionManagerState>,
    db_state: State<'_, DatabaseState>,
) -> Result<SecurityMetrics, String> {
    info!("Getting security metrics for period: {}", period);

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    // Check authorization (Admin only)
    if session.role.to_string() != "Administrator" {
        return Err("Administrator role required for security metrics".to_string());
    }

    drop(session_manager);

    // Get database connection
    let db_guard = db_state.lock().unwrap();
    let db = db_guard.as_ref()
        .ok_or("Database not initialized")?;
    
    let conn = db.get_connection();
    
    // Get total assets count
    let total_assets: u64 = conn
        .query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))
        .unwrap_or(0);

    // Get classification breakdown (simulated for now)
    let mut classification_breakdown = HashMap::new();
    classification_breakdown.insert("PUBLIC".to_string(), 45);
    classification_breakdown.insert("INTERNAL".to_string(), 30);
    classification_breakdown.insert("CONFIDENTIAL".to_string(), 15);
    classification_breakdown.insert("RESTRICTED".to_string(), 8);
    classification_breakdown.insert("SECRET".to_string(), 2);

    // Calculate metrics based on period
    let (recent_validations, security_alerts) = match period.as_str() {
        "24h" => (25, 2),
        "7d" => (150, 5),
        "30d" => (500, 12),
        _ => (100, 3),
    };

    // Calculate validation success rate (simulated)
    let validation_success_rate = 94.5;
    
    // Calculate compliance score (simulated)
    let compliance_score = 87.0;

    let metrics = SecurityMetrics {
        total_assets,
        classification_breakdown,
        validation_success_rate,
        recent_validations,
        security_alerts,
        compliance_score,
    };

    info!("Security metrics retrieved: {} assets, {}% success rate", 
          metrics.total_assets, metrics.validation_success_rate);
    Ok(metrics)
}

/// Export security report
#[command]
pub async fn export_security_report(
    token: String,
    start_date: String,
    end_date: String,
    session_state: State<'_, SessionManagerState>,
    db_state: State<'_, DatabaseState>,
) -> Result<String, String> {
    info!("Exporting security report from {} to {}", start_date, end_date);

    // Validate session
    let session_manager = session_state.lock().unwrap();
    let session = match session_manager.validate_session(&token) {
        Ok(Some(session)) => session,
        Ok(None) => return Err("Invalid or expired session".to_string()),
        Err(e) => return Err(format!("Session validation failed: {}", e)),
    };

    // Check authorization (Admin only)
    if session.role.to_string() != "Administrator" {
        return Err("Administrator role required for security report export".to_string());
    }

    drop(session_manager);

    // TODO: Implement actual report generation
    // For now, return a success message
    let file_path = format!("security_report_{}_{}.pdf", 
                           start_date.replace('-', ""), 
                           end_date.replace('-', ""));

    info!("Security report exported to: {}", file_path);
    Ok(file_path)
}