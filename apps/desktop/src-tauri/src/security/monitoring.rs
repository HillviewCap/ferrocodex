//! Security overview for the monitoring dashboard, computed from the audit
//! log and the current state of users, keys, vault grants and assets.
//!
//! Timestamps are the database's UTC "YYYY-MM-DD HH:MM:SS" strings, and
//! periods are applied with SQLite's `datetime('now', ...)` so they compare
//! directly with `audit_events.timestamp`.

use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::assets::SecurityClassification;
use crate::audit::audit_username;

/// How many security events the overview lists.
pub const RECENT_EVENTS_LIMIT: usize = 15;
/// How many security events an exported report holds at most.
pub const REPORT_EVENTS_LIMIT: usize = 5000;

const FAILED_LOGINS_WARNING_THRESHOLD: u64 = 5;
const DATA_KEY_AGE_INFO_DAYS: i64 = 365;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityOverview {
    pub period: String,
    pub generated_at: String,
    pub status: String,
    pub authentication: AuthenticationStats,
    pub access_control: AccessControlStats,
    pub users: UserStats,
    pub encryption: EncryptionStats,
    pub assets: AssetStats,
    pub findings: Vec<SecurityFinding>,
    pub recent_events: Vec<SecurityEventSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationStats {
    pub successful_logins: u64,
    pub failed_logins: u64,
    pub lockouts: u64,
    pub distinct_failed_usernames: u64,
    pub currently_locked_accounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessControlStats {
    pub vault_access_denied: u64,
    pub vault_exports: u64,
    pub secret_reads: u64,
    pub pending_permission_requests: u64,
    pub grants_expiring_soon: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStats {
    pub active_users: u64,
    pub active_administrators: u64,
    pub inactive_users: u64,
    pub users_pending_key_upgrade: u64,
    pub role_changes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptionStats {
    pub active_key_version: Option<i64>,
    pub active_key_created_at: Option<String>,
    pub active_key_age_days: Option<i64>,
    pub rotation_in_progress: bool,
    pub last_rotation_completed_at: Option<String>,
    pub recovery_key_last_replaced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetStats {
    pub total_assets: u64,
    pub classification_breakdown: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityFinding {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityEventSummary {
    pub id: i64,
    pub timestamp: String,
    pub event_code: String,
    pub category: String,
    pub severity: String,
    pub username: Option<String>,
    pub description: String,
}

/// What `export_security_report` writes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub overview: SecurityOverview,
    pub events: Vec<SecurityEventSummary>,
}

/// The security-relevant audit codes and their severity. Successful logins
/// (AUTH_001) and logouts (AUTH_003) are left out as noise.
const SECURITY_EVENTS: &[(&str, &str)] = &[
    ("USER_006", "critical"),
    ("SEC_001", "critical"),
    ("AUTH_002", "warning"),
    ("VAULT_003", "warning"),
    ("VAULT_014", "warning"),
    ("USER_005", "warning"),
    ("USER_002", "warning"),
    ("SEC_002", "warning"),
    ("SEC_004", "warning"),
    ("VAULT_010", "warning"),
    ("USER_001", "info"),
    ("USER_003", "info"),
    ("USER_004", "info"),
    ("USER_007", "info"),
    ("SEC_003", "info"),
    ("SEC_005", "info"),
    ("SEC_006", "info"),
    ("VAULT_001", "info"),
    ("VAULT_002", "info"),
    ("VAULT_013", "info"),
];

/// The SQLite `datetime('now', ...)` modifier for the start of a period.
pub fn parse_period(period: &str) -> Result<&'static str> {
    match period {
        "24h" => Ok("-1 day"),
        "7d" => Ok("-7 days"),
        "30d" => Ok("-30 days"),
        _ => Err(anyhow!("Unsupported period '{}'; use 24h, 7d or 30d", period)),
    }
}

fn period_label(period: &str) -> &'static str {
    match period {
        "24h" => "24 hours",
        "7d" => "7 days",
        _ => "30 days",
    }
}

fn event_severity(event_code: &str) -> Option<&'static str> {
    SECURITY_EVENTS
        .iter()
        .find(|(code, _)| *code == event_code)
        .map(|(_, severity)| *severity)
}

fn event_category(event_code: &str) -> &'static str {
    match event_code {
        // A security violation is about access, not key management
        "SEC_001" => "access",
        code if code.starts_with("AUTH_") => "authentication",
        code if code.starts_with("USER_") => "user",
        code if code.starts_with("SEC_") => "encryption",
        _ => "access",
    }
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "warning" => 1,
        _ => 2,
    }
}

fn count(conn: &Connection, sql: &str, params: impl rusqlite::Params) -> Result<u64> {
    let n: i64 = conn.query_row(sql, params, |row| row.get(0))?;
    Ok(n.max(0) as u64)
}

fn latest_event_timestamp(conn: &Connection, event_code: &str) -> Result<Option<String>> {
    Ok(conn.query_row(
        "SELECT MAX(timestamp) FROM audit_events WHERE event_code = ?1",
        [event_code],
        |row| row.get(0),
    )?)
}

/// Security-relevant audit events within the period, newest first.
pub fn security_events(conn: &Connection, period: &str, limit: usize) -> Result<Vec<SecurityEventSummary>> {
    let since = parse_period(period)?;
    // The codes are constants, so listing them inline is safe
    let codes = SECURITY_EVENTS
        .iter()
        .map(|(code, _)| format!("'{}'", code))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT id, timestamp, event_code, username, description
         FROM audit_events
         WHERE event_code IN ({}) AND timestamp >= datetime('now', ?1)
         ORDER BY timestamp DESC, id DESC
         LIMIT ?2",
        codes
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![since, limit as i64], |row| {
        let event_code: String = row.get(2)?;
        Ok(SecurityEventSummary {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            category: event_category(&event_code).to_string(),
            severity: event_severity(&event_code).unwrap_or("info").to_string(),
            event_code,
            username: row.get(3)?,
            description: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Builds the overview. `locked_accounts` comes from the in-memory login
/// attempt tracker, which the database knows nothing about.
pub fn compute_overview(conn: &Connection, period: &str, locked_accounts: Vec<String>) -> Result<SecurityOverview> {
    let since = parse_period(period)?;
    let generated_at: String = conn.query_row("SELECT datetime('now')", [], |row| row.get(0))?;

    let mut event_counts: HashMap<String, u64> = HashMap::new();
    {
        let mut stmt = conn.prepare(
            "SELECT event_code, COUNT(*) FROM audit_events
             WHERE timestamp >= datetime('now', ?1)
             GROUP BY event_code",
        )?;
        let rows = stmt.query_map([since], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
        for row in rows {
            let (code, n) = row?;
            event_counts.insert(code, n.max(0) as u64);
        }
    }
    let events = |code: &str| event_counts.get(code).copied().unwrap_or(0);

    // Locked names are as typed at login: clean them like the audit log does
    let mut currently_locked_accounts: Vec<String> =
        locked_accounts.iter().map(|name| audit_username(name)).collect();
    currently_locked_accounts.sort();
    currently_locked_accounts.dedup();
    let authentication = AuthenticationStats {
        successful_logins: events("AUTH_001"),
        failed_logins: events("AUTH_002"),
        lockouts: events("USER_006"),
        distinct_failed_usernames: count(
            conn,
            "SELECT COUNT(DISTINCT username) FROM audit_events
             WHERE event_code = 'AUTH_002' AND timestamp >= datetime('now', ?1)",
            [since],
        )?,
        currently_locked_accounts,
    };

    let access_control = AccessControlStats {
        vault_access_denied: events("VAULT_003"),
        vault_exports: events("VAULT_010"),
        secret_reads: events("VAULT_017"),
        pending_permission_requests: count(
            conn,
            "SELECT COUNT(*) FROM permission_requests WHERE status = 'pending'",
            [],
        )?,
        // expires_at may be stored as RFC 3339; datetime() normalizes it to UTC
        grants_expiring_soon: count(
            conn,
            "SELECT COUNT(*) FROM vault_permissions
             WHERE is_active = 1 AND expires_at IS NOT NULL
               AND datetime(expires_at) > datetime('now')
               AND datetime(expires_at) <= datetime('now', '+7 days')",
            [],
        )?,
    };

    let users = UserStats {
        active_users: count(conn, "SELECT COUNT(*) FROM users WHERE is_active = 1", [])?,
        active_administrators: count(
            conn,
            "SELECT COUNT(*) FROM users WHERE is_active = 1 AND role = 'Administrator'",
            [],
        )?,
        inactive_users: count(conn, "SELECT COUNT(*) FROM users WHERE is_active = 0", [])?,
        users_pending_key_upgrade: count(
            conn,
            "SELECT COUNT(*) FROM users u
             WHERE u.is_active = 1
               AND EXISTS (SELECT 1 FROM data_key_wraps w WHERE w.kind = 'user' AND w.user_id = u.id)
               AND NOT EXISTS (SELECT 1 FROM user_keypairs k WHERE k.user_id = u.id)",
            [],
        )?,
        role_changes: events("USER_005"),
    };

    let active_key: Option<(i64, Option<String>, Option<i64>)> = conn
        .query_row(
            "SELECT version, created_at,
                    CAST(julianday('now') - julianday(created_at) AS INTEGER)
             FROM data_keys WHERE status = 'active'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let encryption = EncryptionStats {
        active_key_version: active_key.as_ref().map(|(version, _, _)| *version),
        active_key_created_at: active_key.as_ref().and_then(|(_, created_at, _)| created_at.clone()),
        active_key_age_days: active_key.as_ref().and_then(|(_, _, age)| *age),
        rotation_in_progress: count(conn, "SELECT COUNT(*) FROM data_keys WHERE status = 'retiring'", [])? > 0,
        last_rotation_completed_at: latest_event_timestamp(conn, "SEC_005")?,
        recovery_key_last_replaced_at: latest_event_timestamp(conn, "SEC_002")?,
    };

    let mut classification_breakdown: HashMap<String, u64> = [
        SecurityClassification::Public,
        SecurityClassification::Internal,
        SecurityClassification::Confidential,
        SecurityClassification::Restricted,
        SecurityClassification::Secret,
    ]
    .iter()
    .map(|level| (level.as_str().to_string(), 0))
    .collect();
    {
        let mut stmt = conn.prepare(
            "SELECT security_classification, COUNT(*) FROM assets GROUP BY security_classification",
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
        for row in rows {
            let (level, n) = row?;
            if let Some(slot) = classification_breakdown.get_mut(&level) {
                *slot = n.max(0) as u64;
            }
        }
    }
    let assets = AssetStats {
        total_assets: count(conn, "SELECT COUNT(*) FROM assets", [])?,
        classification_breakdown,
    };

    let findings = findings(period, &authentication, &access_control, &users, &encryption);
    let status = if findings.iter().any(|f| f.severity == "critical") {
        "critical"
    } else if findings.iter().any(|f| f.severity == "warning") {
        "warning"
    } else {
        "ok"
    };

    Ok(SecurityOverview {
        period: period.to_string(),
        generated_at,
        status: status.to_string(),
        authentication,
        access_control,
        users,
        encryption,
        assets,
        findings,
        recent_events: security_events(conn, period, RECENT_EVENTS_LIMIT)?,
    })
}

/// The overview plus every security-relevant event in the period (capped).
pub fn build_report(conn: &Connection, period: &str, locked_accounts: Vec<String>) -> Result<SecurityReport> {
    Ok(SecurityReport {
        overview: compute_overview(conn, period, locked_accounts)?,
        events: security_events(conn, period, REPORT_EVENTS_LIMIT)?,
    })
}

fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{} {}", n, if n == 1 { one } else { many })
}

/// Applies the finding rules, sorted critical, warning, info.
fn findings(
    period: &str,
    authentication: &AuthenticationStats,
    access_control: &AccessControlStats,
    users: &UserStats,
    encryption: &EncryptionStats,
) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    let mut add = |id: &str, severity: &str, title: String, detail: String| {
        findings.push(SecurityFinding { id: id.to_string(), severity: severity.to_string(), title, detail });
    };
    let window = period_label(period);

    if encryption.rotation_in_progress {
        add(
            "rotation-in-progress",
            "critical",
            "Data key rotation unfinished".to_string(),
            "A data key rotation is in progress. Encrypted data is unavailable to every user until an administrator finishes it.".to_string(),
        );
    }
    if !authentication.currently_locked_accounts.is_empty() {
        add(
            "accounts-locked",
            "warning",
            "Accounts locked".to_string(),
            format!(
                "Temporarily locked after too many failed attempts: {}.",
                authentication.currently_locked_accounts.join(", ")
            ),
        );
    }
    if authentication.failed_logins >= FAILED_LOGINS_WARNING_THRESHOLD || authentication.lockouts > 0 {
        add(
            "failed-logins",
            "warning",
            "Repeated failed logins".to_string(),
            format!(
                "{} and {} in the last {}.",
                plural(authentication.failed_logins, "failed login", "failed logins"),
                plural(authentication.lockouts, "lockout", "lockouts"),
                window
            ),
        );
    }
    if users.active_administrators == 1 {
        add(
            "single-administrator",
            "warning",
            "Only one active administrator".to_string(),
            "Losing that account locks out administration of users, keys and vault permissions. Consider adding a second administrator.".to_string(),
        );
    }
    if users.users_pending_key_upgrade > 0 {
        add(
            "legacy-key-wraps",
            "warning",
            "Users on legacy key wraps".to_string(),
            format!(
                "{} still hold a password-wrapped copy of the data key. Each is moved to a per-user keypair at their next login.",
                plural(users.users_pending_key_upgrade, "active user", "active users")
            ),
        );
    }
    if access_control.vault_access_denied > 0 {
        add(
            "vault-access-denied",
            "warning",
            "Vault access denied".to_string(),
            format!(
                "{} denied in the last {}.",
                plural(access_control.vault_access_denied, "vault access attempt was", "vault access attempts were"),
                window
            ),
        );
    }
    if let Some(age) = encryption.active_key_age_days.filter(|age| *age >= DATA_KEY_AGE_INFO_DAYS) {
        add(
            "data-key-age",
            "info",
            "Data key is over a year old".to_string(),
            format!("The active data key is {} days old. Consider rotating it.", age),
        );
    }
    if access_control.pending_permission_requests > 0 {
        add(
            "pending-permission-requests",
            "info",
            "Pending vault permission requests".to_string(),
            format!(
                "{} awaiting review.",
                plural(access_control.pending_permission_requests, "request is", "requests are")
            ),
        );
    }
    if access_control.grants_expiring_soon > 0 {
        add(
            "grants-expiring",
            "info",
            "Vault grants expiring soon".to_string(),
            format!(
                "{} within 7 days.",
                plural(access_control.grants_expiring_soon, "grant expires", "grants expire")
            ),
        );
    }

    findings.sort_by_key(|f| severity_rank(&f.severity));
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    fn setup() -> Database {
        Database::new(":memory:".into()).unwrap()
    }

    fn add_user(conn: &Connection, username: &str, role: &str, active: bool) -> i64 {
        conn.execute(
            "INSERT INTO users (username, password_hash, role, is_active) VALUES (?1, 'x', ?2, ?3)",
            params![username, role, active],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    /// Logs an event `age` ago, as a SQLite modifier like "-2 hours".
    fn add_event(conn: &Connection, code: &str, username: Option<&str>, age: &str) -> i64 {
        conn.execute(
            "INSERT INTO audit_events (event_type, event_code, username, description, timestamp)
             VALUES (?1, ?1, ?2, 'test event', datetime('now', ?3))",
            params![code, username, age],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn add_data_key(conn: &Connection, version: i64, status: &str, age: &str) {
        conn.execute(
            "INSERT INTO data_keys (version, status, created_at) VALUES (?1, ?2, datetime('now', ?3))",
            params![version, status, age],
        )
        .unwrap();
    }

    fn overview(db: &Database, period: &str) -> SecurityOverview {
        compute_overview(db.get_connection(), period, Vec::new()).unwrap()
    }

    fn finding_ids(overview: &SecurityOverview) -> Vec<&str> {
        overview.findings.iter().map(|f| f.id.as_str()).collect()
    }

    #[test]
    fn test_parse_period() {
        assert_eq!(parse_period("24h").unwrap(), "-1 day");
        assert_eq!(parse_period("7d").unwrap(), "-7 days");
        assert_eq!(parse_period("30d").unwrap(), "-30 days");
        assert!(parse_period("1y").is_err());
        assert!(parse_period("").is_err());
        assert!(compute_overview(setup().get_connection(), "90d", Vec::new()).is_err());
    }

    #[test]
    fn test_period_filtering_excludes_old_events() {
        let db = setup();
        let conn = db.get_connection();
        add_event(conn, "AUTH_001", Some("a"), "-1 hours");
        add_event(conn, "AUTH_002", Some("a"), "-2 hours");
        add_event(conn, "AUTH_002", Some("b"), "-3 days");
        add_event(conn, "AUTH_002", Some("b"), "-10 days");
        add_event(conn, "AUTH_002", Some("c"), "-40 days");

        let day = overview(&db, "24h");
        assert_eq!(day.authentication.successful_logins, 1);
        assert_eq!(day.authentication.failed_logins, 1);
        assert_eq!(day.authentication.distinct_failed_usernames, 1);

        let week = overview(&db, "7d");
        assert_eq!(week.authentication.failed_logins, 2);
        assert_eq!(week.authentication.distinct_failed_usernames, 2);

        let month = overview(&db, "30d");
        assert_eq!(month.authentication.failed_logins, 3);
        assert_eq!(month.authentication.distinct_failed_usernames, 2);
        assert_eq!(month.period, "30d");
    }

    #[test]
    fn test_classification_breakdown_is_zero_filled_with_real_counts() {
        let db = setup();
        let conn = db.get_connection();
        let admin = add_user(conn, "admin", "Administrator", true);

        let empty = overview(&db, "24h");
        assert_eq!(empty.assets.total_assets, 0);
        assert_eq!(empty.assets.classification_breakdown.len(), 5);
        assert!(empty.assets.classification_breakdown.values().all(|n| *n == 0));

        for (name, level) in [("a", "SECRET"), ("b", "SECRET"), ("c", "PUBLIC"), ("d", "INTERNAL")] {
            conn.execute(
                "INSERT INTO assets (name, created_by, security_classification) VALUES (?1, ?2, ?3)",
                params![name, admin, level],
            )
            .unwrap();
        }
        let filled = overview(&db, "24h");
        let breakdown = &filled.assets.classification_breakdown;
        assert_eq!(filled.assets.total_assets, 4);
        assert_eq!(breakdown["SECRET"], 2);
        assert_eq!(breakdown["PUBLIC"], 1);
        assert_eq!(breakdown["INTERNAL"], 1);
        assert_eq!(breakdown["CONFIDENTIAL"], 0);
        assert_eq!(breakdown["RESTRICTED"], 0);
    }

    #[test]
    fn test_clean_install_with_two_admins_is_ok() {
        let db = setup();
        let conn = db.get_connection();
        add_user(conn, "admin", "Administrator", true);
        add_user(conn, "admin2", "Administrator", true);
        add_data_key(conn, 1, "active", "-1 days");

        let o = overview(&db, "24h");
        assert_eq!(o.status, "ok");
        assert!(o.findings.is_empty(), "{:?}", o.findings);
        assert_eq!(o.encryption.active_key_version, Some(1));
        assert_eq!(o.encryption.active_key_age_days, Some(1));
        assert!(o.encryption.active_key_created_at.is_some());
        assert!(!o.encryption.rotation_in_progress);
    }

    #[test]
    fn test_rotation_in_progress_is_critical() {
        let db = setup();
        let conn = db.get_connection();
        add_user(conn, "admin", "Administrator", true);
        add_data_key(conn, 1, "retiring", "-30 days");
        add_data_key(conn, 2, "active", "-1 hours");

        let o = overview(&db, "24h");
        assert!(o.encryption.rotation_in_progress);
        assert_eq!(o.encryption.active_key_version, Some(2));
        assert_eq!(o.status, "critical");
        // Sorted critical first, then the single-administrator warning
        assert_eq!(finding_ids(&o), vec!["rotation-in-progress", "single-administrator"]);
    }

    #[test]
    fn test_locked_accounts_finding_lists_names() {
        let db = setup();
        let conn = db.get_connection();
        add_user(conn, "admin", "Administrator", true);
        add_user(conn, "admin2", "Administrator", true);

        let o = compute_overview(conn, "24h", vec!["zed".to_string(), "bob".to_string()]).unwrap();
        assert_eq!(o.authentication.currently_locked_accounts, vec!["bob", "zed"]);
        assert_eq!(finding_ids(&o), vec!["accounts-locked"]);
        assert!(o.findings[0].detail.contains("bob, zed"));
        assert_eq!(o.status, "warning");
    }

    #[test]
    fn test_locked_account_names_are_sanitized_and_deduped() {
        let db = setup();
        let long = "p".repeat(100);
        let locked = vec![
            format!("{}a", long),
            format!("{}b", long),
            "eve\u{1b}[31m".to_string(),
            "eve[31m".to_string(),
            "line\nbreak".to_string(),
        ];

        let o = compute_overview(db.get_connection(), "24h", locked).unwrap();

        let names = &o.authentication.currently_locked_accounts;
        // The two long names are the same once cut to 64, as are the two eves
        assert_eq!(names, &vec!["eve[31m".to_string(), "linebreak".to_string(), "p".repeat(64)]);
        assert!(names.iter().all(|n| n.chars().count() <= 64 && !n.chars().any(|c| c.is_control())));
        let detail = &o.findings.iter().find(|f| f.id == "accounts-locked").unwrap().detail;
        assert!(!detail.chars().any(|c| c.is_control()));
        assert!(!detail.contains(&"p".repeat(65)));
    }

    #[test]
    fn test_failed_logins_finding_threshold_and_lockouts() {
        let db = setup();
        let conn = db.get_connection();
        add_user(conn, "admin", "Administrator", true);
        add_user(conn, "admin2", "Administrator", true);
        for _ in 0..4 {
            add_event(conn, "AUTH_002", Some("x"), "-1 hours");
        }
        assert!(!finding_ids(&overview(&db, "24h")).contains(&"failed-logins"));

        add_event(conn, "AUTH_002", Some("x"), "-1 hours");
        assert!(finding_ids(&overview(&db, "24h")).contains(&"failed-logins"));

        // A lockout alone is enough
        let db = setup();
        let conn = db.get_connection();
        add_event(conn, "USER_006", Some("x"), "-1 hours");
        let o = overview(&db, "24h");
        assert_eq!(o.authentication.lockouts, 1);
        assert!(finding_ids(&o).contains(&"failed-logins"));
    }

    #[test]
    fn test_single_administrator_finding() {
        let db = setup();
        let conn = db.get_connection();
        add_user(conn, "admin", "Administrator", true);
        add_user(conn, "former", "Administrator", false);
        add_user(conn, "eng", "Engineer", true);

        let o = overview(&db, "24h");
        assert_eq!(o.users.active_users, 2);
        assert_eq!(o.users.active_administrators, 1);
        assert_eq!(o.users.inactive_users, 1);
        assert_eq!(finding_ids(&o), vec!["single-administrator"]);

        add_user(conn, "admin2", "Administrator", true);
        assert!(overview(&db, "24h").findings.is_empty());
    }

    #[test]
    fn test_users_pending_key_upgrade() {
        let db = setup();
        let conn = db.get_connection();
        add_user(conn, "admin", "Administrator", true);
        add_user(conn, "admin2", "Administrator", true);
        let legacy = add_user(conn, "legacy", "Engineer", true);
        let upgraded = add_user(conn, "upgraded", "Engineer", true);
        let inactive = add_user(conn, "inactive", "Engineer", false);
        for user_id in [legacy, upgraded, inactive] {
            conn.execute(
                "INSERT INTO data_key_wraps
                     (kind, user_id, salt, wrapped_key, kdf_memory_kib, kdf_iterations, kdf_parallelism)
                 VALUES ('user', ?1, x'00', x'00', 1, 1, 1)",
                [user_id],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO user_keypairs
                 (user_id, public_key, salt, wrapped_private_key, kdf_memory_kib, kdf_iterations, kdf_parallelism)
             VALUES (?1, zeroblob(32), x'00', x'00', 1, 1, 1)",
            [upgraded],
        )
        .unwrap();

        let o = overview(&db, "24h");
        assert_eq!(o.users.users_pending_key_upgrade, 1);
        assert_eq!(finding_ids(&o), vec!["legacy-key-wraps"]);
        assert_eq!(o.status, "warning");
    }

    #[test]
    fn test_vault_access_denied_finding_and_access_counts() {
        let db = setup();
        let conn = db.get_connection();
        add_user(conn, "admin", "Administrator", true);
        add_user(conn, "admin2", "Administrator", true);
        add_event(conn, "VAULT_010", Some("admin"), "-1 hours");
        add_event(conn, "VAULT_017", Some("admin"), "-1 hours");
        add_event(conn, "VAULT_017", Some("admin"), "-1 hours");
        add_event(conn, "VAULT_003", Some("eng"), "-3 days");

        let day = overview(&db, "24h");
        assert_eq!(day.access_control.vault_exports, 1);
        assert_eq!(day.access_control.secret_reads, 2);
        assert_eq!(day.access_control.vault_access_denied, 0);
        assert!(day.findings.is_empty());

        let week = overview(&db, "7d");
        assert_eq!(week.access_control.vault_access_denied, 1);
        assert_eq!(finding_ids(&week), vec!["vault-access-denied"]);
    }

    #[test]
    fn test_info_findings_do_not_degrade_status() {
        let db = setup();
        let conn = db.get_connection();
        let admin = add_user(conn, "admin", "Administrator", true);
        add_user(conn, "admin2", "Administrator", true);
        let eng = add_user(conn, "eng", "Engineer", true);
        add_data_key(conn, 1, "active", "-400 days");
        conn.execute("INSERT INTO assets (id, name, created_by) VALUES (1, 'plc', ?1)", [admin]).unwrap();
        conn.execute(
            "INSERT INTO vault_entries (id, asset_id, name, created_by) VALUES (1, 1, 'v', ?1)",
            [admin],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO permission_requests (user_id, vault_id, requested_permission, requested_by)
             VALUES (?1, 1, 'read', ?1)",
            [eng],
        )
        .unwrap();
        // Expiring in 3 days (RFC 3339, as the app stores it), in 30 days,
        // already expired, and revoked
        let grants = [
            ("read", strftime_offset(conn, "+3 days"), 1),
            ("write", strftime_offset(conn, "+30 days"), 1),
            ("export", strftime_offset(conn, "-1 days"), 1),
            ("share", strftime_offset(conn, "+2 days"), 0),
        ];
        for (permission, expires_at, active) in grants {
            conn.execute(
                "INSERT INTO vault_permissions (user_id, vault_id, permission_type, granted_by, expires_at, is_active)
                 VALUES (?1, 1, ?2, ?3, ?4, ?5)",
                params![eng, permission, admin, expires_at, active],
            )
            .unwrap();
        }

        let o = overview(&db, "24h");
        assert_eq!(o.encryption.active_key_age_days, Some(400));
        assert_eq!(o.access_control.pending_permission_requests, 1);
        assert_eq!(o.access_control.grants_expiring_soon, 1);
        assert_eq!(finding_ids(&o), vec!["data-key-age", "pending-permission-requests", "grants-expiring"]);
        assert!(o.findings.iter().all(|f| f.severity == "info"));
        assert_eq!(o.status, "ok");
    }

    fn strftime_offset(conn: &Connection, offset: &str) -> String {
        conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?1)", [offset], |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn test_findings_sorted_by_severity() {
        let db = setup();
        let conn = db.get_connection();
        add_user(conn, "admin", "Administrator", true);
        add_data_key(conn, 1, "retiring", "-400 days");
        add_data_key(conn, 2, "active", "-400 days");
        add_event(conn, "VAULT_003", None, "-1 hours");

        let o = compute_overview(conn, "24h", vec!["x".to_string()]).unwrap();
        let ranks: Vec<u8> = o.findings.iter().map(|f| severity_rank(&f.severity)).collect();
        let mut sorted = ranks.clone();
        sorted.sort();
        assert_eq!(ranks, sorted);
        assert_eq!(o.findings.first().unwrap().id, "rotation-in-progress");
        assert_eq!(o.findings.last().unwrap().id, "data-key-age");
    }

    #[test]
    fn test_recent_events_exclude_noise_and_are_capped_newest_first() {
        let db = setup();
        let conn = db.get_connection();
        add_event(conn, "AUTH_001", Some("a"), "-1 minutes");
        add_event(conn, "AUTH_003", Some("a"), "-1 minutes");
        add_event(conn, "VAULT_017", Some("a"), "-1 minutes");
        let newest = add_event(conn, "USER_006", Some("a"), "-2 minutes");
        for i in 0..20 {
            add_event(conn, "AUTH_002", Some("a"), &format!("-{} minutes", 10 + i));
        }
        add_event(conn, "SEC_005", Some("admin"), "-2 days");

        let o = overview(&db, "24h");
        assert_eq!(o.recent_events.len(), RECENT_EVENTS_LIMIT);
        assert_eq!(o.recent_events[0].id, newest);
        assert_eq!(o.recent_events[0].severity, "critical");
        assert_eq!(o.recent_events[0].category, "user");
        assert!(o.recent_events.iter().all(|e| !["AUTH_001", "AUTH_003", "VAULT_017"].contains(&e.event_code.as_str())));
        let timestamps: Vec<&str> = o.recent_events.iter().map(|e| e.timestamp.as_str()).collect();
        let mut sorted = timestamps.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(timestamps, sorted);
        assert_eq!(o.recent_events[1].event_code, "AUTH_002");
        assert_eq!(o.recent_events[1].severity, "warning");
        assert_eq!(o.recent_events[1].category, "authentication");

        // The report holds every event in the period, and the older rotation in 7 days
        let report = build_report(conn, "7d", Vec::new()).unwrap();
        assert_eq!(report.events.len(), 22);
        let rotation = report.events.last().unwrap();
        assert_eq!(rotation.event_code, "SEC_005");
        assert_eq!(rotation.category, "encryption");
        assert_eq!(rotation.severity, "info");
        assert!(report.overview.encryption.last_rotation_completed_at.is_some());
    }

    #[test]
    fn test_event_categories() {
        assert_eq!(event_category("AUTH_002"), "authentication");
        assert_eq!(event_category("USER_005"), "user");
        assert_eq!(event_category("VAULT_003"), "access");
        assert_eq!(event_category("SEC_002"), "encryption");
        assert_eq!(event_category("SEC_001"), "access");
        assert_eq!(event_severity("AUTH_001"), None);
        assert_eq!(event_severity("SEC_001"), Some("critical"));
        assert_eq!(event_severity("SEC_006"), Some("info"));
        assert_eq!(event_category("SEC_006"), "encryption");
    }

    #[test]
    fn test_overview_serializes_camel_case() {
        let db = setup();
        let json = serde_json::to_value(overview(&db, "24h")).unwrap();
        assert!(json["generatedAt"].is_string());
        assert!(json["authentication"]["currentlyLockedAccounts"].is_array());
        assert!(json["accessControl"]["grantsExpiringSoon"].is_number());
        assert!(json["users"]["usersPendingKeyUpgrade"].is_number());
        assert!(json["encryption"]["activeKeyVersion"].is_null());
        assert!(json["assets"]["classificationBreakdown"]["SECRET"].is_number());
        assert!(json["recentEvents"].is_array());
    }
}
