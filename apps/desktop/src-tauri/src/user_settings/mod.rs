use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error_handling::{RetryStrategy, CircuitBreakerConfig};

/// User preference for retry and recovery settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPreferences {
    /// Global retry configuration
    pub global_retry_strategy: RetryStrategy,
    /// Per-operation retry strategies
    pub operation_specific: HashMap<String, RetryStrategy>,
    /// Circuit breaker configurations by service
    pub circuit_breaker_configs: HashMap<String, CircuitBreakerConfig>,
    /// Enable/disable automatic recovery globally
    pub enable_automatic_recovery: bool,
    /// Show retry progress in UI
    pub show_retry_progress: bool,
    /// Show technical details in error messages
    pub show_technical_details: bool,
    /// Auto-fallback to manual recovery after failures
    pub auto_fallback_to_manual: bool,
    /// Notification duration for recovery messages (milliseconds)
    pub recovery_notification_duration_ms: u64,
}

impl Default for RetryPreferences {
    fn default() -> Self {
        Self {
            global_retry_strategy: RetryStrategy::default(),
            operation_specific: HashMap::new(),
            circuit_breaker_configs: HashMap::new(),
            enable_automatic_recovery: true,
            show_retry_progress: true,
            show_technical_details: false,
            auto_fallback_to_manual: true,
            recovery_notification_duration_ms: 5000,
        }
    }
}

impl RetryPreferences {
    /// Get retry strategy for a specific operation
    pub fn get_retry_strategy(&self, operation: &str) -> &RetryStrategy {
        self.operation_specific.get(operation)
            .unwrap_or(&self.global_retry_strategy)
    }

    /// Set retry strategy for a specific operation
    pub fn set_retry_strategy(&mut self, operation: String, strategy: RetryStrategy) {
        self.operation_specific.insert(operation, strategy);
    }

    /// Get circuit breaker config for a service
    pub fn get_circuit_breaker_config(&self, service: &str) -> CircuitBreakerConfig {
        self.circuit_breaker_configs.get(service)
            .cloned()
            .unwrap_or_default()
    }

    /// Set circuit breaker config for a service
    pub fn set_circuit_breaker_config(&mut self, service: String, config: CircuitBreakerConfig) {
        self.circuit_breaker_configs.insert(service, config);
    }

    /// Validate preferences
    pub fn validate(&self) -> Result<()> {
        // Validate global retry strategy
        self.global_retry_strategy.validate()
            .map_err(|e| anyhow::anyhow!("Invalid global retry strategy: {}", e))?;

        // Validate operation-specific strategies
        for (operation, strategy) in &self.operation_specific {
            strategy.validate()
                .map_err(|e| anyhow::anyhow!("Invalid retry strategy for operation '{}': {}", operation, e))?;
        }

        // Validate circuit breaker configs
        for (service, config) in &self.circuit_breaker_configs {
            config.validate()
                .map_err(|e| anyhow::anyhow!("Invalid circuit breaker config for service '{}': {}", service, e))?;
        }

        // Validate notification duration
        if self.recovery_notification_duration_ms > 60000 {
            return Err(anyhow::anyhow!("Recovery notification duration cannot exceed 60 seconds"));
        }

        Ok(())
    }

    /// Get preset configurations for different user types
    pub fn conservative_preset() -> Self {
        let mut prefs = Self::default();
        prefs.global_retry_strategy = RetryStrategy::conservative();
        prefs.enable_automatic_recovery = true;
        prefs.show_retry_progress = true;
        prefs.auto_fallback_to_manual = true;
        prefs
    }

    pub fn aggressive_preset() -> Self {
        let mut prefs = Self::default();
        prefs.global_retry_strategy = RetryStrategy::aggressive();
        prefs.enable_automatic_recovery = true;
        prefs.show_retry_progress = true;
        prefs.auto_fallback_to_manual = false; // Let it retry more
        prefs
    }

    pub fn minimal_preset() -> Self {
        let mut prefs = Self::default();
        prefs.global_retry_strategy = RetryStrategy::new(1, 500, 1000, 1.5);
        prefs.enable_automatic_recovery = true;
        prefs.show_retry_progress = false;
        prefs.show_technical_details = false;
        prefs.auto_fallback_to_manual = true;
        prefs
    }
}

/// Complete user settings including retry preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    /// User ID this settings belongs to
    pub user_id: i64,
    /// Retry and recovery preferences
    pub retry_preferences: RetryPreferences,
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Settings version for migration
    pub version: u32,
}

impl UserSettings {
    /// Create new user settings with defaults
    pub fn new(user_id: i64) -> Self {
        Self {
            user_id,
            retry_preferences: RetryPreferences::default(),
            updated_at: chrono::Utc::now(),
            version: 1,
        }
    }

    /// Create user settings with preset
    pub fn with_preset(user_id: i64, preset_name: &str) -> Result<Self> {
        let retry_preferences = match preset_name {
            "conservative" => RetryPreferences::conservative_preset(),
            "aggressive" => RetryPreferences::aggressive_preset(),
            "minimal" => RetryPreferences::minimal_preset(),
            _ => return Err(anyhow::anyhow!("Unknown preset: {}", preset_name)),
        };

        Ok(Self {
            user_id,
            retry_preferences,
            updated_at: chrono::Utc::now(),
            version: 1,
        })
    }

    /// Update settings and timestamp
    pub fn update(&mut self, retry_preferences: RetryPreferences) -> Result<()> {
        retry_preferences.validate()?;
        self.retry_preferences = retry_preferences;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// Validate all settings
    pub fn validate(&self) -> Result<()> {
        self.retry_preferences.validate()
    }
}

/// Repository for user settings persistence
pub trait UserSettingsRepository {
    fn create_settings(&self, settings: &UserSettings) -> Result<()>;
    fn get_settings(&self, user_id: i64) -> Result<Option<UserSettings>>;
    fn update_settings(&self, settings: &UserSettings) -> Result<()>;
    fn delete_settings(&self, user_id: i64) -> Result<()>;
    fn get_or_create_settings(&self, user_id: i64) -> Result<UserSettings>;
}

/// SQLite implementation of user settings repository
pub struct SqliteUserSettingsRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteUserSettingsRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS user_settings (
                user_id INTEGER PRIMARY KEY,
                retry_preferences TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                version INTEGER DEFAULT 1,
                FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_user_settings_updated_at ON user_settings(updated_at);
            "#,
        )?;
        Ok(())
    }

    fn row_to_settings(row: &Row) -> rusqlite::Result<UserSettings> {
        let retry_preferences_json: String = row.get("retry_preferences")?;
        let retry_preferences: RetryPreferences = serde_json::from_str(&retry_preferences_json)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("JSON parse error: {}", e))),
            ))?;

        let updated_at_str: String = row.get("updated_at")?;
        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("DateTime parse error: {}", e))),
            ))?
            .with_timezone(&chrono::Utc);

        Ok(UserSettings {
            user_id: row.get("user_id")?,
            retry_preferences,
            updated_at,
            version: row.get("version")?,
        })
    }
}

impl<'a> UserSettingsRepository for SqliteUserSettingsRepository<'a> {
    fn create_settings(&self, settings: &UserSettings) -> Result<()> {
        settings.validate()?;
        
        let retry_preferences_json = serde_json::to_string(&settings.retry_preferences)?;
        let updated_at_str = settings.updated_at.to_rfc3339();

        self.conn.execute(
            "INSERT INTO user_settings (user_id, retry_preferences, updated_at, version) VALUES (?1, ?2, ?3, ?4)",
            (
                &settings.user_id,
                &retry_preferences_json,
                &updated_at_str,
                &settings.version,
            ),
        )?;

        Ok(())
    }

    fn get_settings(&self, user_id: i64) -> Result<Option<UserSettings>> {
        let mut stmt = self.conn.prepare(
            "SELECT user_id, retry_preferences, updated_at, version FROM user_settings WHERE user_id = ?1"
        )?;

        let result = stmt.query_row([user_id], Self::row_to_settings);

        match result {
            Ok(settings) => Ok(Some(settings)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn update_settings(&self, settings: &UserSettings) -> Result<()> {
        settings.validate()?;
        
        let retry_preferences_json = serde_json::to_string(&settings.retry_preferences)?;
        let updated_at_str = settings.updated_at.to_rfc3339();

        let rows_affected = self.conn.execute(
            "UPDATE user_settings SET retry_preferences = ?1, updated_at = ?2, version = ?3 WHERE user_id = ?4",
            (
                &retry_preferences_json,
                &updated_at_str,
                &settings.version,
                &settings.user_id,
            ),
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Settings not found for user ID: {}", settings.user_id));
        }

        Ok(())
    }

    fn delete_settings(&self, user_id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM user_settings WHERE user_id = ?1",
            [user_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Settings not found for user ID: {}", user_id));
        }

        Ok(())
    }

    fn get_or_create_settings(&self, user_id: i64) -> Result<UserSettings> {
        if let Some(settings) = self.get_settings(user_id)? {
            Ok(settings)
        } else {
            let settings = UserSettings::new(user_id);
            self.create_settings(&settings)?;
            Ok(settings)
        }
    }
}

/// Utility functions for common settings operations
pub mod settings_utils {
    use super::*;

    /// Get operation-specific retry configurations
    pub fn get_default_operation_configs() -> HashMap<String, RetryStrategy> {
        let mut configs = HashMap::new();
        
        // Database operations - conservative retries
        configs.insert("database_query".to_string(), RetryStrategy::conservative());
        configs.insert("database_transaction".to_string(), RetryStrategy::conservative());
        
        // Network operations - aggressive retries
        configs.insert("network_request".to_string(), RetryStrategy::aggressive());
        configs.insert("file_download".to_string(), RetryStrategy::aggressive());
        
        // Authentication - minimal retries
        configs.insert("user_login".to_string(), RetryStrategy::new(2, 1000, 5000, 2.0));
        configs.insert("token_refresh".to_string(), RetryStrategy::new(2, 500, 2000, 1.5));
        
        // Asset operations - balanced approach
        configs.insert("asset_read".to_string(), RetryStrategy::default());
        configs.insert("asset_write".to_string(), RetryStrategy::conservative());
        configs.insert("asset_deploy".to_string(), RetryStrategy::conservative());
        
        configs
    }

    /// Get default circuit breaker configurations for services
    pub fn get_default_circuit_breaker_configs() -> HashMap<String, CircuitBreakerConfig> {
        let mut configs = HashMap::new();
        
        // Database service - conservative
        configs.insert("database".to_string(), CircuitBreakerConfig::conservative());
        
        // External services - balanced
        configs.insert("external_api".to_string(), CircuitBreakerConfig::default());
        
        // File system operations - aggressive (recover quickly)
        configs.insert("file_system".to_string(), CircuitBreakerConfig::aggressive());
        
        // Network services - balanced with quick recovery
        configs.insert("network_service".to_string(), CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_ms: 5000,
            half_open_max_calls: 3,
            sliding_window_size: 10,
            enabled: true,
        });
        
        configs
    }

    /// Apply role-based defaults to user settings
    pub fn apply_role_defaults(settings: &mut UserSettings, role: &crate::users::UserRole) {
        match role {
            crate::users::UserRole::Administrator => {
                // Administrators get more aggressive retry settings
                settings.retry_preferences = RetryPreferences::aggressive_preset();
                settings.retry_preferences.show_technical_details = true;
                settings.retry_preferences.recovery_notification_duration_ms = 8000;
            }
            crate::users::UserRole::Engineer => {
                // Engineers get balanced settings with technical details
                settings.retry_preferences = RetryPreferences::default();
                settings.retry_preferences.show_technical_details = true;
                settings.retry_preferences.show_retry_progress = true;
            }
        }
    }

    /// Migrate settings from older versions
    pub fn migrate_settings(settings: &mut UserSettings) -> Result<bool> {
        let mut migrated = false;
        
        // Future migration logic would go here
        match settings.version {
            1 => {
                // Current version, no migration needed
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported settings version: {}", settings.version));
            }
        }
        
        Ok(migrated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use rusqlite::Connection;

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        
        // Initialize users table first (required for foreign key)
        conn.execute(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, username TEXT, password_hash TEXT, role TEXT, created_at TEXT, updated_at TEXT, is_active BOOLEAN)",
            [],
        ).unwrap();
        
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at, updated_at, is_active) VALUES (1, 'test', 'hash', 'Administrator', '2023-01-01T00:00:00Z', '2023-01-01T00:00:00Z', 1)",
            [],
        ).unwrap();
        
        let repo = SqliteUserSettingsRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_retry_preferences_defaults() {
        let prefs = RetryPreferences::default();
        assert!(prefs.enable_automatic_recovery);
        assert!(prefs.show_retry_progress);
        assert!(!prefs.show_technical_details);
        assert_eq!(prefs.recovery_notification_duration_ms, 5000);
    }

    #[test]
    fn test_retry_preferences_validation() {
        let mut prefs = RetryPreferences::default();
        assert!(prefs.validate().is_ok());
        
        // Invalid notification duration
        prefs.recovery_notification_duration_ms = 70000;
        assert!(prefs.validate().is_err());
    }

    #[test]
    fn test_retry_preferences_operation_specific() {
        let mut prefs = RetryPreferences::default();
        
        // Initially uses global strategy
        assert_eq!(prefs.get_retry_strategy("test_op").max_attempts, 3);
        
        // Set operation-specific strategy
        let custom_strategy = RetryStrategy::conservative();
        prefs.set_retry_strategy("test_op".to_string(), custom_strategy.clone());
        
        assert_eq!(prefs.get_retry_strategy("test_op").max_attempts, custom_strategy.max_attempts);
        assert_eq!(prefs.get_retry_strategy("other_op").max_attempts, 3); // Still uses global
    }

    #[test]
    fn test_user_settings_creation() {
        let settings = UserSettings::new(1);
        assert_eq!(settings.user_id, 1);
        assert_eq!(settings.version, 1);
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_user_settings_presets() {
        let conservative = UserSettings::with_preset(1, "conservative").unwrap();
        assert_eq!(conservative.retry_preferences.global_retry_strategy.max_attempts, 2);
        
        let aggressive = UserSettings::with_preset(1, "aggressive").unwrap();
        assert_eq!(aggressive.retry_preferences.global_retry_strategy.max_attempts, 5);
        
        let minimal = UserSettings::with_preset(1, "minimal").unwrap();
        assert_eq!(minimal.retry_preferences.global_retry_strategy.max_attempts, 1);
        
        assert!(UserSettings::with_preset(1, "invalid").is_err());
    }

    #[test]
    fn test_user_settings_update() {
        let mut settings = UserSettings::new(1);
        let original_time = settings.updated_at;
        
        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let new_prefs = RetryPreferences::aggressive_preset();
        settings.update(new_prefs).unwrap();
        
        assert!(settings.updated_at > original_time);
        assert_eq!(settings.retry_preferences.global_retry_strategy.max_attempts, 5);
    }

    #[test]
    fn test_settings_repository_create_and_get() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserSettingsRepository::new(&conn);
        
        let settings = UserSettings::new(1);
        repo.create_settings(&settings).unwrap();
        
        let retrieved = repo.get_settings(1).unwrap().unwrap();
        assert_eq!(retrieved.user_id, settings.user_id);
        assert_eq!(retrieved.version, settings.version);
    }

    #[test]
    fn test_settings_repository_update() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserSettingsRepository::new(&conn);
        
        let mut settings = UserSettings::new(1);
        repo.create_settings(&settings).unwrap();
        
        // Update settings
        settings.retry_preferences = RetryPreferences::conservative_preset();
        repo.update_settings(&settings).unwrap();
        
        let retrieved = repo.get_settings(1).unwrap().unwrap();
        assert_eq!(retrieved.retry_preferences.global_retry_strategy.max_attempts, 2);
    }

    #[test]
    fn test_settings_repository_get_or_create() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserSettingsRepository::new(&conn);
        
        // Should create new settings
        let settings1 = repo.get_or_create_settings(1).unwrap();
        assert_eq!(settings1.user_id, 1);
        
        // Should return existing settings
        let settings2 = repo.get_or_create_settings(1).unwrap();
        assert_eq!(settings1.user_id, settings2.user_id);
    }

    #[test]
    fn test_settings_repository_delete() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteUserSettingsRepository::new(&conn);
        
        let settings = UserSettings::new(1);
        repo.create_settings(&settings).unwrap();
        
        repo.delete_settings(1).unwrap();
        assert!(repo.get_settings(1).unwrap().is_none());
        
        // Deleting non-existent settings should error
        assert!(repo.delete_settings(999).is_err());
    }

    #[test]
    fn test_default_operation_configs() {
        let configs = settings_utils::get_default_operation_configs();
        
        assert!(configs.contains_key("database_query"));
        assert!(configs.contains_key("network_request"));
        assert!(configs.contains_key("user_login"));
        assert!(configs.contains_key("asset_read"));
        
        // Database operations should be conservative
        assert_eq!(configs["database_query"].max_attempts, 2);
        
        // Network operations should be aggressive
        assert_eq!(configs["network_request"].max_attempts, 5);
    }

    #[test]
    fn test_default_circuit_breaker_configs() {
        let configs = settings_utils::get_default_circuit_breaker_configs();
        
        assert!(configs.contains_key("database"));
        assert!(configs.contains_key("external_api"));
        assert!(configs.contains_key("file_system"));
        assert!(configs.contains_key("network_service"));
        
        // Database should be conservative
        assert_eq!(configs["database"].failure_threshold, 3);
        
        // File system should be aggressive
        assert_eq!(configs["file_system"].failure_threshold, 10);
    }

    #[test]
    fn test_role_based_defaults() {
        let mut admin_settings = UserSettings::new(1);
        settings_utils::apply_role_defaults(&mut admin_settings, &crate::users::UserRole::Administrator);
        
        assert!(admin_settings.retry_preferences.show_technical_details);
        assert_eq!(admin_settings.retry_preferences.global_retry_strategy.max_attempts, 5);
        
        let mut engineer_settings = UserSettings::new(2);
        settings_utils::apply_role_defaults(&mut engineer_settings, &crate::users::UserRole::Engineer);
        
        assert!(engineer_settings.retry_preferences.show_technical_details);
        assert!(engineer_settings.retry_preferences.show_retry_progress);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = UserSettings::new(1);
        
        // Test JSON serialization/deserialization
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: UserSettings = serde_json::from_str(&json).unwrap();
        
        assert_eq!(settings.user_id, deserialized.user_id);
        assert_eq!(settings.version, deserialized.version);
    }
}