use crate::error_handling::{EnhancedError, ErrorDomain, ErrorSeverity, RecoveryStrategy};
use crate::error_handling::graceful_degradation::{FeatureAvailability, FeatureImportance, SystemDegradationLevel};
use crate::error_handling::user_notifications::{NotificationType, NotificationPriority};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User preference for how degradation should be handled
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DegradationMode {
    /// Automatically degrade features when needed
    Automatic,
    /// Prompt user before degrading features
    Prompt,
    /// Never degrade, fail operations instead
    Never,
    /// Use custom rules per feature
    Custom,
}

impl std::fmt::Display for DegradationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DegradationMode::Automatic => write!(f, "Automatic"),
            DegradationMode::Prompt => write!(f, "Prompt"),
            DegradationMode::Never => write!(f, "Never"),
            DegradationMode::Custom => write!(f, "Custom"),
        }
    }
}

impl Default for DegradationMode {
    fn default() -> Self {
        DegradationMode::Automatic
    }
}

/// User preference for notification behavior during degradation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationNotificationPreferences {
    /// Enable notifications for feature degradation
    pub enable_degradation_notifications: bool,
    /// Enable notifications for feature recovery
    pub enable_recovery_notifications: bool,
    /// Enable system-wide degradation notifications
    pub enable_system_notifications: bool,
    /// Minimum notification priority to show
    pub min_notification_priority: NotificationPriority,
    /// Auto-dismiss timeout for info notifications (ms)
    pub info_auto_dismiss_ms: u64,
    /// Auto-dismiss timeout for warning notifications (ms, 0 = never)
    pub warning_auto_dismiss_ms: u64,
    /// Show degradation notifications in system tray
    pub show_in_system_tray: bool,
    /// Play sound for degradation notifications
    pub play_notification_sounds: bool,
    /// Notification types to suppress during critical operations
    pub suppress_during_critical: Vec<NotificationType>,
}

impl Default for DegradationNotificationPreferences {
    fn default() -> Self {
        Self {
            enable_degradation_notifications: true,
            enable_recovery_notifications: true,
            enable_system_notifications: true,
            min_notification_priority: NotificationPriority::Medium,
            info_auto_dismiss_ms: 5000,   // 5 seconds
            warning_auto_dismiss_ms: 0,   // Never auto-dismiss warnings
            show_in_system_tray: true,
            play_notification_sounds: true,
            suppress_during_critical: vec![NotificationType::Info],
        }
    }
}

/// Per-feature degradation behavior preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureDegradationPreference {
    /// Feature identifier
    pub feature_id: String,
    /// Degradation mode for this feature
    pub degradation_mode: DegradationMode,
    /// Allow offline mode for this feature
    pub allow_offline_mode: bool,
    /// Maximum cache age to accept (seconds)
    pub max_cache_age_seconds: u64,
    /// Notify user when this feature degrades
    pub notify_on_degradation: bool,
    /// Priority level for this feature (overrides default importance)
    pub custom_priority: Option<FeatureImportance>,
    /// Fallback behavior description
    pub fallback_description: String,
}

impl FeatureDegradationPreference {
    /// Create default preference for a feature
    pub fn new(feature_id: String) -> Self {
        Self {
            feature_id,
            degradation_mode: DegradationMode::Automatic,
            allow_offline_mode: true,
            max_cache_age_seconds: 24 * 60 * 60, // 24 hours
            notify_on_degradation: true,
            custom_priority: None,
            fallback_description: "Use cached data and limited functionality".to_string(),
        }
    }
    
    /// Create critical feature preference (never degrade)
    pub fn critical(feature_id: String) -> Self {
        Self {
            feature_id,
            degradation_mode: DegradationMode::Never,
            allow_offline_mode: false,
            max_cache_age_seconds: 60, // 1 minute for critical features
            notify_on_degradation: true,
            custom_priority: Some(FeatureImportance::Critical),
            fallback_description: "Feature cannot operate in degraded mode".to_string(),
        }
    }
    
    /// Create optional feature preference (auto-degrade freely)
    pub fn optional(feature_id: String) -> Self {
        Self {
            feature_id,
            degradation_mode: DegradationMode::Automatic,
            allow_offline_mode: true,
            max_cache_age_seconds: 7 * 24 * 60 * 60, // 7 days
            notify_on_degradation: false,
            custom_priority: Some(FeatureImportance::Optional),
            fallback_description: "Feature disabled in degraded mode".to_string(),
        }
    }
}

/// System-wide degradation behavior preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDegradationPreferences {
    /// Global degradation mode
    pub global_degradation_mode: DegradationMode,
    /// Enable automatic cache warming
    pub enable_cache_warming: bool,
    /// Enable proactive degradation to prevent failures
    pub enable_proactive_degradation: bool,
    /// System degradation threshold (0.0-1.0, percentage of features affected)
    pub system_degradation_threshold: f64,
    /// Enable offline mode when severely degraded
    pub enable_offline_mode: bool,
    /// Show degradation status in UI permanently
    pub show_degradation_status: bool,
    /// Log degradation events for analysis
    pub log_degradation_events: bool,
    /// Performance thresholds for triggering degradation
    pub performance_thresholds: PerformanceThresholds,
}

/// Performance thresholds for automatic degradation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// Response time threshold (ms) above which to consider degradation
    pub response_time_ms: u64,
    /// Error rate threshold (0.0-1.0) above which to consider degradation  
    pub error_rate_threshold: f64,
    /// Resource utilization threshold (0.0-1.0) above which to consider degradation
    pub resource_utilization_threshold: f64,
    /// Cache hit rate threshold (0.0-1.0) below which to consider degradation
    pub cache_hit_rate_threshold: f64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            response_time_ms: 5000,        // 5 seconds
            error_rate_threshold: 0.1,     // 10% error rate
            resource_utilization_threshold: 0.8, // 80% resource usage
            cache_hit_rate_threshold: 0.7, // 70% cache hit rate
        }
    }
}

impl Default for SystemDegradationPreferences {
    fn default() -> Self {
        Self {
            global_degradation_mode: DegradationMode::Automatic,
            enable_cache_warming: true,
            enable_proactive_degradation: true,
            system_degradation_threshold: 0.3, // 30% of features affected
            enable_offline_mode: true,
            show_degradation_status: true,
            log_degradation_events: true,
            performance_thresholds: PerformanceThresholds::default(),
        }
    }
}

/// Complete user degradation preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDegradationPreferences {
    /// User ID these preferences belong to
    pub user_id: i64,
    /// System-wide degradation preferences
    pub system_preferences: SystemDegradationPreferences,
    /// Notification preferences
    pub notification_preferences: DegradationNotificationPreferences,
    /// Per-feature degradation preferences
    pub feature_preferences: HashMap<String, FeatureDegradationPreference>,
    /// Preset name (if using a preset)
    pub preset_name: Option<String>,
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Preferences version for migration
    pub version: u32,
}

impl UserDegradationPreferences {
    /// Create new degradation preferences with defaults
    pub fn new(user_id: i64) -> Self {
        Self {
            user_id,
            system_preferences: SystemDegradationPreferences::default(),
            notification_preferences: DegradationNotificationPreferences::default(),
            feature_preferences: HashMap::new(),
            preset_name: None,
            updated_at: chrono::Utc::now(),
            version: 1,
        }
    }
    
    /// Create preferences with preset configuration
    pub fn with_preset(user_id: i64, preset: DegradationPreset) -> Self {
        let mut prefs = Self::new(user_id);
        prefs.apply_preset(preset);
        prefs
    }
    
    /// Apply a preset to these preferences
    pub fn apply_preset(&mut self, preset: DegradationPreset) {
        match preset {
            DegradationPreset::Conservative => {
                self.system_preferences.global_degradation_mode = DegradationMode::Prompt;
                self.system_preferences.enable_proactive_degradation = false;
                self.system_preferences.system_degradation_threshold = 0.1; // 10%
                self.notification_preferences.min_notification_priority = NotificationPriority::Low;
                self.notification_preferences.warning_auto_dismiss_ms = 0; // Never auto-dismiss
                self.preset_name = Some("conservative".to_string());
            }
            DegradationPreset::Aggressive => {
                self.system_preferences.global_degradation_mode = DegradationMode::Automatic;
                self.system_preferences.enable_proactive_degradation = true;
                self.system_preferences.system_degradation_threshold = 0.5; // 50%
                self.notification_preferences.min_notification_priority = NotificationPriority::High;
                self.notification_preferences.info_auto_dismiss_ms = 2000; // Quick dismiss
                self.preset_name = Some("aggressive".to_string());
            }
            DegradationPreset::Balanced => {
                // Use defaults (already applied in new())
                self.preset_name = Some("balanced".to_string());
            }
            DegradationPreset::HighAvailability => {
                self.system_preferences.global_degradation_mode = DegradationMode::Automatic;
                self.system_preferences.enable_cache_warming = true;
                self.system_preferences.enable_proactive_degradation = true;
                self.system_preferences.system_degradation_threshold = 0.7; // 70%
                self.system_preferences.enable_offline_mode = true;
                self.notification_preferences.enable_system_notifications = true;
                self.preset_name = Some("high_availability".to_string());
            }
        }
        self.updated_at = chrono::Utc::now();
    }
    
    /// Get degradation preference for a specific feature
    pub fn get_feature_preference(&self, feature_id: &str) -> FeatureDegradationPreference {
        self.feature_preferences.get(feature_id)
            .cloned()
            .unwrap_or_else(|| FeatureDegradationPreference::new(feature_id.to_string()))
    }
    
    /// Set degradation preference for a specific feature
    pub fn set_feature_preference(&mut self, preference: FeatureDegradationPreference) {
        let feature_id = preference.feature_id.clone();
        self.feature_preferences.insert(feature_id, preference);
        self.updated_at = chrono::Utc::now();
    }
    
    /// Remove feature preference (will use defaults)
    pub fn remove_feature_preference(&mut self, feature_id: &str) -> bool {
        let removed = self.feature_preferences.remove(feature_id).is_some();
        if removed {
            self.updated_at = chrono::Utc::now();
        }
        removed
    }
    
    /// Check if degradation should be allowed for a feature
    pub fn should_allow_degradation(&self, feature_id: &str, current_availability: FeatureAvailability) -> bool {
        let feature_pref = self.get_feature_preference(feature_id);
        
        match feature_pref.degradation_mode {
            DegradationMode::Never => false,
            DegradationMode::Automatic => true,
            DegradationMode::Prompt => {
                // This would typically trigger a user prompt in the UI
                // For now, allow degradation but this could be enhanced
                true
            }
            DegradationMode::Custom => {
                // Custom logic based on feature-specific rules
                self.evaluate_custom_degradation_rules(feature_id, current_availability)
            }
        }
    }
    
    /// Check if offline mode should be allowed for a feature  
    pub fn should_allow_offline_mode(&self, feature_id: &str) -> bool {
        let feature_pref = self.get_feature_preference(feature_id);
        feature_pref.allow_offline_mode && self.system_preferences.enable_offline_mode
    }
    
    /// Get maximum acceptable cache age for a feature
    pub fn get_max_cache_age(&self, feature_id: &str) -> u64 {
        let feature_pref = self.get_feature_preference(feature_id);
        feature_pref.max_cache_age_seconds
    }
    
    /// Check if user should be notified about feature degradation
    pub fn should_notify_degradation(&self, feature_id: &str, notification_type: NotificationType, notification_priority: NotificationPriority) -> bool {
        if !self.notification_preferences.enable_degradation_notifications {
            return false;
        }
        
        if notification_priority < self.notification_preferences.min_notification_priority {
            return false;
        }
        
        if self.notification_preferences.suppress_during_critical.contains(&notification_type) {
            // Would check if currently in critical operation mode
            // For now, just check system degradation level
            // This is a placeholder for more sophisticated logic
            return true;
        }
        
        let feature_pref = self.get_feature_preference(feature_id);
        feature_pref.notify_on_degradation
    }
    
    /// Validate preferences configuration
    pub fn validate(&self) -> Result<(), EnhancedError> {
        // Validate system degradation threshold
        if self.system_preferences.system_degradation_threshold < 0.0 || 
           self.system_preferences.system_degradation_threshold > 1.0 {
            return Err(EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::System,
                RecoveryStrategy::UserRecoverable,
                "System degradation threshold must be between 0.0 and 1.0".to_string(),
            ));
        }
        
        // Validate performance thresholds
        let perf = &self.system_preferences.performance_thresholds;
        if perf.error_rate_threshold < 0.0 || perf.error_rate_threshold > 1.0 {
            return Err(EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::System,
                RecoveryStrategy::UserRecoverable,
                "Error rate threshold must be between 0.0 and 1.0".to_string(),
            ));
        }
        
        if perf.resource_utilization_threshold < 0.0 || perf.resource_utilization_threshold > 1.0 {
            return Err(EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::System,
                RecoveryStrategy::UserRecoverable,
                "Resource utilization threshold must be between 0.0 and 1.0".to_string(),
            ));
        }
        
        if perf.cache_hit_rate_threshold < 0.0 || perf.cache_hit_rate_threshold > 1.0 {
            return Err(EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::System,
                RecoveryStrategy::UserRecoverable,
                "Cache hit rate threshold must be between 0.0 and 1.0".to_string(),
            ));
        }
        
        // Validate notification timeouts
        if self.notification_preferences.info_auto_dismiss_ms > 60000 {
            return Err(EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::System,
                RecoveryStrategy::UserRecoverable,
                "Info auto-dismiss timeout cannot exceed 60 seconds".to_string(),
            ));
        }
        
        if self.notification_preferences.warning_auto_dismiss_ms > 300000 && 
           self.notification_preferences.warning_auto_dismiss_ms != 0 {
            return Err(EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::System,
                RecoveryStrategy::UserRecoverable,
                "Warning auto-dismiss timeout cannot exceed 5 minutes (or use 0 for never)".to_string(),
            ));
        }
        
        // Validate feature preferences
        for (feature_id, preference) in &self.feature_preferences {
            if preference.max_cache_age_seconds > 30 * 24 * 60 * 60 { // 30 days
                return Err(EnhancedError::new(
                    ErrorSeverity::Medium,
                    ErrorDomain::System,
                    RecoveryStrategy::UserRecoverable,
                    format!("Cache age for feature '{}' cannot exceed 30 days", feature_id),
                ));
            }
        }
        
        Ok(())
    }
    
    /// Evaluate custom degradation rules for a feature
    fn evaluate_custom_degradation_rules(&self, _feature_id: &str, _current_availability: FeatureAvailability) -> bool {
        // Placeholder for custom rule evaluation
        // This would be implemented based on specific business logic
        true
    }
}

/// Preset configurations for different user types/scenarios
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DegradationPreset {
    /// Conservative approach - minimal degradation, user confirmation
    Conservative,
    /// Aggressive approach - auto-degrade readily to maintain availability
    Aggressive,
    /// Balanced approach - reasonable defaults
    Balanced,
    /// High availability approach - maximize uptime
    HighAvailability,
}

impl std::fmt::Display for DegradationPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DegradationPreset::Conservative => write!(f, "Conservative"),
            DegradationPreset::Aggressive => write!(f, "Aggressive"),
            DegradationPreset::Balanced => write!(f, "Balanced"),
            DegradationPreset::HighAvailability => write!(f, "High Availability"),
        }
    }
}

impl DegradationPreset {
    /// Get all available presets
    pub fn all_presets() -> Vec<DegradationPreset> {
        vec![
            DegradationPreset::Conservative,
            DegradationPreset::Aggressive,
            DegradationPreset::Balanced,
            DegradationPreset::HighAvailability,
        ]
    }
    
    /// Get preset description
    pub fn description(&self) -> &'static str {
        match self {
            DegradationPreset::Conservative => "Minimal degradation with user confirmation",
            DegradationPreset::Aggressive => "Auto-degrade readily to maintain availability",
            DegradationPreset::Balanced => "Reasonable defaults for most users",
            DegradationPreset::HighAvailability => "Maximize uptime with extensive fallbacks",
        }
    }
}

/// Repository trait for degradation preferences persistence
pub trait DegradationPreferencesRepository {
    fn create_preferences(&self, preferences: &UserDegradationPreferences) -> Result<(), EnhancedError>;
    fn get_preferences(&self, user_id: i64) -> Result<Option<UserDegradationPreferences>, EnhancedError>;
    fn update_preferences(&self, preferences: &UserDegradationPreferences) -> Result<(), EnhancedError>;
    fn delete_preferences(&self, user_id: i64) -> Result<(), EnhancedError>;
    fn get_or_create_preferences(&self, user_id: i64) -> Result<UserDegradationPreferences, EnhancedError>;
}

/// SQLite implementation of degradation preferences repository
pub struct SqliteDegradationPreferencesRepository {
    connection: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
}

impl SqliteDegradationPreferencesRepository {
    /// Create a new repository with database connection
    pub fn new(connection: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>) -> Self {
        Self { connection }
    }
    
    /// Initialize database schema for degradation preferences
    pub fn initialize_schema(&self) -> Result<(), EnhancedError> {
        let conn = self.connection.lock().unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS degradation_preferences (
                user_id INTEGER PRIMARY KEY,
                system_preferences TEXT NOT NULL,
                notification_preferences TEXT NOT NULL,
                feature_preferences TEXT NOT NULL,
                preset_name TEXT,
                updated_at TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to create degradation_preferences table: {}", e),
        ))?;
        
        Ok(())
    }
}

impl DegradationPreferencesRepository for SqliteDegradationPreferencesRepository {
    fn create_preferences(&self, preferences: &UserDegradationPreferences) -> Result<(), EnhancedError> {
        let conn = self.connection.lock().unwrap();
        
        let system_json = serde_json::to_string(&preferences.system_preferences)
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to serialize system preferences: {}", e),
            ))?;
        
        let notification_json = serde_json::to_string(&preferences.notification_preferences)
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to serialize notification preferences: {}", e),
            ))?;
        
        let feature_json = serde_json::to_string(&preferences.feature_preferences)
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to serialize feature preferences: {}", e),
            ))?;
        
        conn.execute(
            "INSERT INTO degradation_preferences 
             (user_id, system_preferences, notification_preferences, feature_preferences, preset_name, updated_at, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                preferences.user_id,
                system_json,
                notification_json,
                feature_json,
                preferences.preset_name,
                preferences.updated_at.to_rfc3339(),
                preferences.version,
            ],
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to create degradation preferences: {}", e),
        ))?;
        
        Ok(())
    }
    
    fn get_preferences(&self, user_id: i64) -> Result<Option<UserDegradationPreferences>, EnhancedError> {
        let conn = self.connection.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT system_preferences, notification_preferences, feature_preferences, preset_name, updated_at, version
             FROM degradation_preferences WHERE user_id = ?1"
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to prepare preferences query: {}", e),
        ))?;
        
        let result = stmt.query_row(
            rusqlite::params![user_id],
            |row| {
                let system_json: String = row.get(0)?;
                let notification_json: String = row.get(1)?;
                let feature_json: String = row.get(2)?;
                let preset_name: Option<String> = row.get(3)?;
                let updated_at_str: String = row.get(4)?;
                let version: u32 = row.get(5)?;
                
                Ok((system_json, notification_json, feature_json, preset_name, updated_at_str, version))
            }
        );
        
        match result {
            Ok((system_json, notification_json, feature_json, preset_name, updated_at_str, version)) => {
                let system_preferences: SystemDegradationPreferences = serde_json::from_str(&system_json)
                    .map_err(|e| EnhancedError::new(
                        ErrorSeverity::Medium,
                        ErrorDomain::Data,
                        RecoveryStrategy::AutoRecoverable,
                        format!("Failed to deserialize system preferences: {}", e),
                    ))?;
                
                let notification_preferences: DegradationNotificationPreferences = serde_json::from_str(&notification_json)
                    .map_err(|e| EnhancedError::new(
                        ErrorSeverity::Medium,
                        ErrorDomain::Data,
                        RecoveryStrategy::AutoRecoverable,
                        format!("Failed to deserialize notification preferences: {}", e),
                    ))?;
                
                let feature_preferences: HashMap<String, FeatureDegradationPreference> = serde_json::from_str(&feature_json)
                    .map_err(|e| EnhancedError::new(
                        ErrorSeverity::Medium,
                        ErrorDomain::Data,
                        RecoveryStrategy::AutoRecoverable,
                        format!("Failed to deserialize feature preferences: {}", e),
                    ))?;
                
                let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                    .map_err(|e| EnhancedError::new(
                        ErrorSeverity::Medium,
                        ErrorDomain::Data,
                        RecoveryStrategy::AutoRecoverable,
                        format!("Failed to parse updated_at timestamp: {}", e),
                    ))?.with_timezone(&chrono::Utc);
                
                Ok(Some(UserDegradationPreferences {
                    user_id,
                    system_preferences,
                    notification_preferences,
                    feature_preferences,
                    preset_name,
                    updated_at,
                    version,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to get degradation preferences: {}", e),
            )),
        }
    }
    
    fn update_preferences(&self, preferences: &UserDegradationPreferences) -> Result<(), EnhancedError> {
        let conn = self.connection.lock().unwrap();
        
        let system_json = serde_json::to_string(&preferences.system_preferences)
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to serialize system preferences: {}", e),
            ))?;
        
        let notification_json = serde_json::to_string(&preferences.notification_preferences)
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to serialize notification preferences: {}", e),
            ))?;
        
        let feature_json = serde_json::to_string(&preferences.feature_preferences)
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to serialize feature preferences: {}", e),
            ))?;
        
        let updated_count = conn.execute(
            "UPDATE degradation_preferences 
             SET system_preferences = ?2, notification_preferences = ?3, feature_preferences = ?4, 
                 preset_name = ?5, updated_at = ?6, version = ?7
             WHERE user_id = ?1",
            rusqlite::params![
                preferences.user_id,
                system_json,
                notification_json,
                feature_json,
                preferences.preset_name,
                preferences.updated_at.to_rfc3339(),
                preferences.version,
            ],
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to update degradation preferences: {}", e),
        ))?;
        
        if updated_count == 0 {
            return Err(EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::UserRecoverable,
                format!("No degradation preferences found for user_id: {}", preferences.user_id),
            ));
        }
        
        Ok(())
    }
    
    fn delete_preferences(&self, user_id: i64) -> Result<(), EnhancedError> {
        let conn = self.connection.lock().unwrap();
        
        conn.execute(
            "DELETE FROM degradation_preferences WHERE user_id = ?1",
            rusqlite::params![user_id],
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to delete degradation preferences: {}", e),
        ))?;
        
        Ok(())
    }
    
    fn get_or_create_preferences(&self, user_id: i64) -> Result<UserDegradationPreferences, EnhancedError> {
        match self.get_preferences(user_id)? {
            Some(preferences) => Ok(preferences),
            None => {
                let preferences = UserDegradationPreferences::new(user_id);
                self.create_preferences(&preferences)?;
                Ok(preferences)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::sync::Arc;
    
    fn create_test_repository() -> (SqliteDegradationPreferencesRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_degradation_prefs.db");
        
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let conn = Arc::new(std::sync::Mutex::new(conn));
        
        let repo = SqliteDegradationPreferencesRepository::new(conn);
        repo.initialize_schema().unwrap();
        
        (repo, temp_dir)
    }
    
    #[test]
    fn test_degradation_preferences_creation() {
        let prefs = UserDegradationPreferences::new(1);
        
        assert_eq!(prefs.user_id, 1);
        assert_eq!(prefs.system_preferences.global_degradation_mode, DegradationMode::Automatic);
        assert!(prefs.notification_preferences.enable_degradation_notifications);
        assert_eq!(prefs.feature_preferences.len(), 0);
        assert!(prefs.validate().is_ok());
    }
    
    #[test]
    fn test_degradation_presets() {
        let mut prefs = UserDegradationPreferences::new(1);
        
        prefs.apply_preset(DegradationPreset::Conservative);
        assert_eq!(prefs.system_preferences.global_degradation_mode, DegradationMode::Prompt);
        assert_eq!(prefs.preset_name, Some("conservative".to_string()));
        
        prefs.apply_preset(DegradationPreset::Aggressive);
        assert_eq!(prefs.system_preferences.global_degradation_mode, DegradationMode::Automatic);
        assert_eq!(prefs.preset_name, Some("aggressive".to_string()));
    }
    
    #[test]
    fn test_feature_preference_operations() {
        let mut prefs = UserDegradationPreferences::new(1);
        
        // Test getting default preference
        let default_pref = prefs.get_feature_preference("test_feature");
        assert_eq!(default_pref.feature_id, "test_feature");
        assert_eq!(default_pref.degradation_mode, DegradationMode::Automatic);
        
        // Test setting custom preference
        let custom_pref = FeatureDegradationPreference::critical("test_feature".to_string());
        prefs.set_feature_preference(custom_pref.clone());
        
        let retrieved_pref = prefs.get_feature_preference("test_feature");
        assert_eq!(retrieved_pref.degradation_mode, DegradationMode::Never);
        assert_eq!(retrieved_pref.custom_priority, Some(FeatureImportance::Critical));
        
        // Test removing preference
        let removed = prefs.remove_feature_preference("test_feature");
        assert!(removed);
        
        let back_to_default = prefs.get_feature_preference("test_feature");
        assert_eq!(back_to_default.degradation_mode, DegradationMode::Automatic);
    }
    
    #[test]
    fn test_should_allow_degradation() {
        let mut prefs = UserDegradationPreferences::new(1);
        
        // Test automatic mode
        assert!(prefs.should_allow_degradation("auto_feature", FeatureAvailability::Available));
        
        // Test never mode
        let never_pref = FeatureDegradationPreference::critical("never_feature".to_string());
        prefs.set_feature_preference(never_pref);
        assert!(!prefs.should_allow_degradation("never_feature", FeatureAvailability::Available));
        
        // Test prompt mode (currently allows)
        let mut prompt_pref = FeatureDegradationPreference::new("prompt_feature".to_string());
        prompt_pref.degradation_mode = DegradationMode::Prompt;
        prefs.set_feature_preference(prompt_pref);
        assert!(prefs.should_allow_degradation("prompt_feature", FeatureAvailability::Available));
    }
    
    #[test]
    fn test_validation() {
        let mut prefs = UserDegradationPreferences::new(1);
        
        // Valid preferences should pass
        assert!(prefs.validate().is_ok());
        
        // Invalid system degradation threshold
        prefs.system_preferences.system_degradation_threshold = 1.5;
        assert!(prefs.validate().is_err());
        
        prefs.system_preferences.system_degradation_threshold = 0.5; // Fix it
        
        // Invalid error rate threshold
        prefs.system_preferences.performance_thresholds.error_rate_threshold = -0.1;
        assert!(prefs.validate().is_err());
        
        prefs.system_preferences.performance_thresholds.error_rate_threshold = 0.1; // Fix it
        
        // Invalid notification timeout
        prefs.notification_preferences.info_auto_dismiss_ms = 70000; // 70 seconds
        assert!(prefs.validate().is_err());
    }
    
    #[test]
    fn test_repository_operations() {
        let (repo, _temp_dir) = create_test_repository();
        
        let prefs = UserDegradationPreferences::new(1);
        
        // Test create
        let create_result = repo.create_preferences(&prefs);
        assert!(create_result.is_ok());
        
        // Test get
        let retrieved = repo.get_preferences(1).unwrap();
        assert!(retrieved.is_some());
        let retrieved_prefs = retrieved.unwrap();
        assert_eq!(retrieved_prefs.user_id, 1);
        assert_eq!(retrieved_prefs.version, 1);
        
        // Test update
        let mut updated_prefs = retrieved_prefs;
        updated_prefs.system_preferences.global_degradation_mode = DegradationMode::Prompt;
        updated_prefs.version = 2;
        updated_prefs.updated_at = chrono::Utc::now();
        
        let update_result = repo.update_preferences(&updated_prefs);
        assert!(update_result.is_ok());
        
        // Verify update
        let re_retrieved = repo.get_preferences(1).unwrap().unwrap();
        assert_eq!(re_retrieved.system_preferences.global_degradation_mode, DegradationMode::Prompt);
        assert_eq!(re_retrieved.version, 2);
        
        // Test get_or_create for existing user
        let existing = repo.get_or_create_preferences(1).unwrap();
        assert_eq!(existing.user_id, 1);
        
        // Test get_or_create for new user
        let new_user = repo.get_or_create_preferences(999).unwrap();
        assert_eq!(new_user.user_id, 999);
        
        // Test delete
        let delete_result = repo.delete_preferences(1);
        assert!(delete_result.is_ok());
        
        // Verify deletion
        let deleted_check = repo.get_preferences(1).unwrap();
        assert!(deleted_check.is_none());
    }
    
    #[test]
    fn test_preset_descriptions() {
        let presets = DegradationPreset::all_presets();
        assert_eq!(presets.len(), 4);
        
        for preset in presets {
            let description = preset.description();
            assert!(!description.is_empty());
            
            let display = format!("{}", preset);
            assert!(!display.is_empty());
        }
    }
}