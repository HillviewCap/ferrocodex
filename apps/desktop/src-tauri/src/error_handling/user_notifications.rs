use crate::error_handling::{EnhancedError, ErrorDomain, ErrorSeverity, RecoveryStrategy};
use crate::error_handling::graceful_degradation::{FeatureAvailability, FeatureStatus, SystemDegradationLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// User notification types for degradation scenarios
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType {
    /// Informational message about system status
    Info,
    /// Warning about degraded functionality
    Warning,
    /// Error notification about unavailable features
    Error,
    /// Critical alert about system-wide issues
    Critical,
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationType::Info => write!(f, "Info"),
            NotificationType::Warning => write!(f, "Warning"),
            NotificationType::Error => write!(f, "Error"),
            NotificationType::Critical => write!(f, "Critical"),
        }
    }
}

/// User notification priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NotificationPriority {
    /// Low priority - can be dismissed easily
    Low = 0,
    /// Medium priority - requires acknowledgment
    Medium = 1,
    /// High priority - requires immediate attention
    High = 2,
    /// Critical priority - blocks operation until addressed
    Critical = 3,
}

impl std::fmt::Display for NotificationPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationPriority::Low => write!(f, "Low"),
            NotificationPriority::Medium => write!(f, "Medium"),
            NotificationPriority::High => write!(f, "High"),
            NotificationPriority::Critical => write!(f, "Critical"),
        }
    }
}

/// User notification for feature degradation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationNotification {
    /// Unique notification ID
    pub id: String,
    /// Notification type
    pub notification_type: NotificationType,
    /// Priority level
    pub priority: NotificationPriority,
    /// Feature affected
    pub feature_id: String,
    /// Feature display name
    pub feature_name: String,
    /// Current availability status
    pub availability: FeatureAvailability,
    /// User-friendly title
    pub title: String,
    /// Detailed message
    pub message: String,
    /// Suggested actions for the user
    pub suggested_actions: Vec<String>,
    /// Whether this notification can be dismissed
    pub dismissible: bool,
    /// Auto-dismiss timeout (None for persistent notifications)
    pub auto_dismiss_ms: Option<u64>,
    /// Timestamp when notification was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Whether user has acknowledged this notification
    pub acknowledged: bool,
    /// Additional context data
    pub context: HashMap<String, String>,
}

impl DegradationNotification {
    /// Create a new degradation notification
    pub fn new(
        feature_id: String,
        feature_name: String,
        availability: FeatureAvailability,
        message: String,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let (notification_type, priority, title, dismissible, auto_dismiss_ms) = 
            Self::determine_notification_characteristics(&availability);
        
        Self {
            id,
            notification_type,
            priority,
            feature_id,
            feature_name,
            availability,
            title,
            message,
            suggested_actions: Self::generate_suggested_actions(&availability),
            dismissible,
            auto_dismiss_ms,
            created_at: chrono::Utc::now(),
            acknowledged: false,
            context: HashMap::new(),
        }
    }
    
    /// Determine notification characteristics based on availability
    fn determine_notification_characteristics(availability: &FeatureAvailability) -> (NotificationType, NotificationPriority, String, bool, Option<u64>) {
        match availability {
            FeatureAvailability::Available => (
                NotificationType::Info,
                NotificationPriority::Low,
                "Feature Restored".to_string(),
                true,
                Some(5000), // Auto-dismiss in 5 seconds
            ),
            FeatureAvailability::Degraded => (
                NotificationType::Warning,
                NotificationPriority::Medium,
                "Feature Running in Degraded Mode".to_string(),
                true,
                None, // Persistent until acknowledged
            ),
            FeatureAvailability::Unavailable => (
                NotificationType::Error,
                NotificationPriority::High,
                "Feature Temporarily Unavailable".to_string(),
                false,
                None, // Persistent until feature recovers
            ),
            FeatureAvailability::Disabled => (
                NotificationType::Critical,
                NotificationPriority::Critical,
                "Feature Disabled".to_string(),
                false,
                None, // Persistent until manually resolved
            ),
        }
    }
    
    /// Generate suggested actions based on availability
    fn generate_suggested_actions(availability: &FeatureAvailability) -> Vec<String> {
        match availability {
            FeatureAvailability::Available => vec![
                "Feature is now fully operational".to_string(),
            ],
            FeatureAvailability::Degraded => vec![
                "Some functionality may be limited".to_string(),
                "Data may be served from cache".to_string(),
                "Consider retrying operations later".to_string(),
            ],
            FeatureAvailability::Unavailable => vec![
                "Please try again in a few moments".to_string(),
                "Check your network connection".to_string(),
                "Contact support if issue persists".to_string(),
            ],
            FeatureAvailability::Disabled => vec![
                "Contact administrator to re-enable".to_string(),
                "Check system configuration".to_string(),
                "Review error logs for details".to_string(),
            ],
        }
    }
    
    /// Add context information
    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.context.insert(key, value);
        self
    }
    
    /// Mark notification as acknowledged
    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }
    
    /// Check if notification should auto-dismiss
    pub fn should_auto_dismiss(&self) -> bool {
        if let Some(timeout_ms) = self.auto_dismiss_ms {
            let elapsed = chrono::Utc::now()
                .signed_duration_since(self.created_at)
                .num_milliseconds() as u64;
            elapsed >= timeout_ms
        } else {
            false
        }
    }
}

/// System-wide degradation notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDegradationNotification {
    /// Unique notification ID
    pub id: String,
    /// Notification type
    pub notification_type: NotificationType,
    /// Priority level
    pub priority: NotificationPriority,
    /// System degradation level
    pub degradation_level: SystemDegradationLevel,
    /// Title for the notification
    pub title: String,
    /// Detailed message
    pub message: String,
    /// Affected features summary
    pub affected_features_summary: String,
    /// Suggested actions
    pub suggested_actions: Vec<String>,
    /// Whether dismissible
    pub dismissible: bool,
    /// Timestamp created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Whether acknowledged
    pub acknowledged: bool,
}

impl SystemDegradationNotification {
    /// Create a new system degradation notification
    pub fn new(
        degradation_level: SystemDegradationLevel,
        affected_features_count: usize,
        critical_features_affected: usize,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let (notification_type, priority, title, message, dismissible) = 
            Self::determine_system_notification_characteristics(
                &degradation_level, 
                affected_features_count, 
                critical_features_affected
            );
        
        let affected_features_summary = if critical_features_affected > 0 {
            format!("{} critical features and {} total features affected", 
                critical_features_affected, affected_features_count)
        } else {
            format!("{} features affected", affected_features_count)
        };
        
        Self {
            id,
            notification_type,
            priority,
            degradation_level,
            title,
            message,
            affected_features_summary,
            suggested_actions: Self::generate_system_suggested_actions(&degradation_level),
            dismissible,
            created_at: chrono::Utc::now(),
            acknowledged: false,
        }
    }
    
    /// Determine system notification characteristics
    fn determine_system_notification_characteristics(
        level: &SystemDegradationLevel,
        affected_count: usize,
        critical_count: usize,
    ) -> (NotificationType, NotificationPriority, String, String, bool) {
        match level {
            SystemDegradationLevel::None => (
                NotificationType::Info,
                NotificationPriority::Low,
                "System Fully Operational".to_string(),
                "All features are working normally.".to_string(),
                true,
            ),
            SystemDegradationLevel::Minor => (
                NotificationType::Warning,
                NotificationPriority::Medium,
                "Minor System Degradation".to_string(),
                format!("Some optional features are experiencing issues. {} features affected.", affected_count),
                true,
            ),
            SystemDegradationLevel::Major => (
                NotificationType::Error,
                NotificationPriority::High,
                "Major System Degradation".to_string(),
                format!("Important system features are degraded. {} features affected.", affected_count),
                false,
            ),
            SystemDegradationLevel::Severe => (
                NotificationType::Critical,
                NotificationPriority::Critical,
                "Severe System Degradation".to_string(),
                format!("Critical system features are affected. {} critical features impacted.", critical_count),
                false,
            ),
        }
    }
    
    /// Generate system-level suggested actions
    fn generate_system_suggested_actions(level: &SystemDegradationLevel) -> Vec<String> {
        match level {
            SystemDegradationLevel::None => vec![
                "Continue normal operations".to_string(),
            ],
            SystemDegradationLevel::Minor => vec![
                "Monitor system status".to_string(),
                "Non-essential features may be limited".to_string(),
                "Core functionality remains available".to_string(),
            ],
            SystemDegradationLevel::Major => vec![
                "Proceed with caution".to_string(),
                "Save work frequently".to_string(),
                "Consider postponing non-critical operations".to_string(),
                "Contact support if issues persist".to_string(),
            ],
            SystemDegradationLevel::Severe => vec![
                "Avoid critical operations".to_string(),
                "Save all work immediately".to_string(),
                "Contact support immediately".to_string(),
                "Consider switching to offline mode".to_string(),
            ],
        }
    }
    
    /// Mark as acknowledged
    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }
}

/// User notification manager for degradation scenarios
pub struct UserNotificationManager {
    /// Active feature notifications
    feature_notifications: Arc<Mutex<HashMap<String, DegradationNotification>>>,
    /// Active system notifications
    system_notifications: Arc<Mutex<Vec<SystemDegradationNotification>>>,
    /// Notification history
    notification_history: Arc<Mutex<Vec<NotificationHistoryEntry>>>,
    /// Configuration
    config: NotificationConfig,
}

/// Notification history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationHistoryEntry {
    /// Notification ID
    pub notification_id: String,
    /// Feature ID (if applicable)
    pub feature_id: Option<String>,
    /// Notification type
    pub notification_type: NotificationType,
    /// Priority
    pub priority: NotificationPriority,
    /// Title
    pub title: String,
    /// Message
    pub message: String,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Acknowledged timestamp
    pub acknowledged_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Auto-dismissed timestamp
    pub dismissed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Maximum number of active notifications
    pub max_active_notifications: usize,
    /// Maximum notification history size
    pub max_history_size: usize,
    /// Enable auto-dismiss for info notifications
    pub enable_auto_dismiss: bool,
    /// Default auto-dismiss timeout (milliseconds)
    pub default_auto_dismiss_ms: u64,
    /// Enable sound notifications
    pub enable_sound: bool,
    /// Enable desktop notifications
    pub enable_desktop_notifications: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            max_active_notifications: 10,
            max_history_size: 100,
            enable_auto_dismiss: true,
            default_auto_dismiss_ms: 10000, // 10 seconds
            enable_sound: true,
            enable_desktop_notifications: true,
        }
    }
}

impl UserNotificationManager {
    /// Create a new user notification manager
    pub fn new(config: NotificationConfig) -> Self {
        Self {
            feature_notifications: Arc::new(Mutex::new(HashMap::new())),
            system_notifications: Arc::new(Mutex::new(Vec::new())),
            notification_history: Arc::new(Mutex::new(Vec::new())),
            config,
        }
    }
    
    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(NotificationConfig::default())
    }
    
    /// Show feature degradation notification
    pub fn notify_feature_degradation(&self, feature_status: &FeatureStatus) -> Result<String, EnhancedError> {
        let notification = DegradationNotification::new(
            feature_status.feature.id.clone(),
            feature_status.feature.name.clone(),
            feature_status.availability,
            self.generate_feature_message(feature_status),
        );
        
        let notification_id = notification.id.clone();
        
        // Add to active notifications
        {
            let mut feature_notifications = self.feature_notifications.lock().unwrap();
            feature_notifications.insert(feature_status.feature.id.clone(), notification.clone());
            
            // Limit active notifications
            if feature_notifications.len() > self.config.max_active_notifications {
                // Remove oldest notifications
                let mut notifications: Vec<_> = feature_notifications.values().cloned().collect();
                notifications.sort_by_key(|n| n.created_at);
                
                for old_notification in notifications.iter().take(notifications.len() - self.config.max_active_notifications) {
                    feature_notifications.remove(&old_notification.feature_id);
                }
            }
        }
        
        // Add to history
        self.add_to_history(&notification);
        
        Ok(notification_id)
    }
    
    /// Show system degradation notification
    pub fn notify_system_degradation(
        &self,
        degradation_level: SystemDegradationLevel,
        affected_features_count: usize,
        critical_features_affected: usize,
    ) -> Result<String, EnhancedError> {
        let notification = SystemDegradationNotification::new(
            degradation_level,
            affected_features_count,
            critical_features_affected,
        );
        
        let notification_id = notification.id.clone();
        
        // Add to active system notifications
        {
            let mut system_notifications = self.system_notifications.lock().unwrap();
            system_notifications.push(notification.clone());
            
            // Limit active notifications
            if system_notifications.len() > self.config.max_active_notifications {
                system_notifications.remove(0);
            }
        }
        
        // Add to history
        self.add_system_to_history(&notification);
        
        Ok(notification_id)
    }
    
    /// Get all active feature notifications
    pub fn get_active_feature_notifications(&self) -> HashMap<String, DegradationNotification> {
        self.feature_notifications.lock().unwrap().clone()
    }
    
    /// Get all active system notifications
    pub fn get_active_system_notifications(&self) -> Vec<SystemDegradationNotification> {
        self.system_notifications.lock().unwrap().clone()
    }
    
    /// Acknowledge a feature notification
    pub fn acknowledge_feature_notification(&self, feature_id: &str) -> Result<(), EnhancedError> {
        let mut feature_notifications = self.feature_notifications.lock().unwrap();
        
        if let Some(notification) = feature_notifications.get_mut(feature_id) {
            notification.acknowledge();
            
            // Remove if dismissible
            if notification.dismissible {
                let notification_clone = notification.clone();
                feature_notifications.remove(feature_id);
                self.mark_as_acknowledged_in_history(&notification_clone.id);
            }
            
            Ok(())
        } else {
            Err(EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::System,
                RecoveryStrategy::UserRecoverable,
                format!("Feature notification not found: {}", feature_id),
            ))
        }
    }
    
    /// Acknowledge a system notification
    pub fn acknowledge_system_notification(&self, notification_id: &str) -> Result<(), EnhancedError> {
        let mut system_notifications = self.system_notifications.lock().unwrap();
        
        if let Some(notification) = system_notifications.iter_mut().find(|n| n.id == notification_id) {
            notification.acknowledge();
            
            if notification.dismissible {
                let notification_id = notification.id.clone();
                system_notifications.retain(|n| n.id != notification_id);
                self.mark_as_acknowledged_in_history(&notification_id);
            }
            
            Ok(())
        } else {
            Err(EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::System,
                RecoveryStrategy::UserRecoverable,
                format!("System notification not found: {}", notification_id),
            ))
        }
    }
    
    /// Clear feature notification when feature recovers
    pub fn clear_feature_notification(&self, feature_id: &str) -> Result<(), EnhancedError> {
        let mut feature_notifications = self.feature_notifications.lock().unwrap();
        
        if let Some(notification) = feature_notifications.remove(feature_id) {
            self.mark_as_dismissed_in_history(&notification.id);
            Ok(())
        } else {
            // Not an error if notification doesn't exist
            Ok(())
        }
    }
    
    /// Clean up auto-dismissible notifications
    pub fn cleanup_auto_dismissible(&self) -> usize {
        let mut cleaned_count = 0;
        
        // Clean up feature notifications
        {
            let mut feature_notifications = self.feature_notifications.lock().unwrap();
            let to_remove: Vec<String> = feature_notifications
                .iter()
                .filter(|(_, notification)| notification.should_auto_dismiss())
                .map(|(feature_id, _)| feature_id.clone())
                .collect();
            
            for feature_id in to_remove {
                if let Some(notification) = feature_notifications.remove(&feature_id) {
                    self.mark_as_dismissed_in_history(&notification.id);
                    cleaned_count += 1;
                }
            }
        }
        
        cleaned_count
    }
    
    /// Get notification history
    pub fn get_notification_history(&self, limit: Option<usize>) -> Vec<NotificationHistoryEntry> {
        let history = self.notification_history.lock().unwrap();
        let limit = limit.unwrap_or(history.len());
        
        history.iter()
            .rev() // Most recent first
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Clear all notifications
    pub fn clear_all_notifications(&self) {
        {
            let mut feature_notifications = self.feature_notifications.lock().unwrap();
            for (_, notification) in feature_notifications.drain() {
                self.mark_as_dismissed_in_history(&notification.id);
            }
        }
        
        {
            let mut system_notifications = self.system_notifications.lock().unwrap();
            for notification in system_notifications.drain(..) {
                self.mark_as_dismissed_in_history(&notification.id);
            }
        }
    }
    
    /// Generate feature-specific message
    fn generate_feature_message(&self, feature_status: &FeatureStatus) -> String {
        match feature_status.availability {
            FeatureAvailability::Available => {
                format!("The {} feature is now fully operational.", feature_status.feature.name)
            }
            FeatureAvailability::Degraded => {
                let reason = &feature_status.status_reason;
                format!("The {} feature is running in degraded mode. {}", 
                    feature_status.feature.name, reason)
            }
            FeatureAvailability::Unavailable => {
                let reason = &feature_status.status_reason;
                format!("The {} feature is currently unavailable. {}", 
                    feature_status.feature.name, reason)
            }
            FeatureAvailability::Disabled => {
                format!("The {} feature has been disabled.", feature_status.feature.name)
            }
        }
    }
    
    /// Add notification to history
    fn add_to_history(&self, notification: &DegradationNotification) {
        let entry = NotificationHistoryEntry {
            notification_id: notification.id.clone(),
            feature_id: Some(notification.feature_id.clone()),
            notification_type: notification.notification_type,
            priority: notification.priority,
            title: notification.title.clone(),
            message: notification.message.clone(),
            created_at: notification.created_at,
            acknowledged_at: None,
            dismissed_at: None,
        };
        
        let mut history = self.notification_history.lock().unwrap();
        history.push(entry);
        
        // Limit history size
        if history.len() > self.config.max_history_size {
            history.remove(0);
        }
    }
    
    /// Add system notification to history
    fn add_system_to_history(&self, notification: &SystemDegradationNotification) {
        let entry = NotificationHistoryEntry {
            notification_id: notification.id.clone(),
            feature_id: None,
            notification_type: notification.notification_type,
            priority: notification.priority,
            title: notification.title.clone(),
            message: notification.message.clone(),
            created_at: notification.created_at,
            acknowledged_at: None,
            dismissed_at: None,
        };
        
        let mut history = self.notification_history.lock().unwrap();
        history.push(entry);
        
        // Limit history size
        if history.len() > self.config.max_history_size {
            history.remove(0);
        }
    }
    
    /// Mark notification as acknowledged in history
    fn mark_as_acknowledged_in_history(&self, notification_id: &str) {
        let mut history = self.notification_history.lock().unwrap();
        
        if let Some(entry) = history.iter_mut().find(|e| e.notification_id == notification_id) {
            entry.acknowledged_at = Some(chrono::Utc::now());
        }
    }
    
    /// Mark notification as dismissed in history
    fn mark_as_dismissed_in_history(&self, notification_id: &str) {
        let mut history = self.notification_history.lock().unwrap();
        
        if let Some(entry) = history.iter_mut().find(|e| e.notification_id == notification_id) {
            entry.dismissed_at = Some(chrono::Utc::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_handling::graceful_degradation::{FeatureDefinition, FeatureImportance, FeaturePerformanceMetrics};
    
    fn create_test_feature_status(availability: FeatureAvailability) -> FeatureStatus {
        FeatureStatus {
            feature: FeatureDefinition {
                id: "test_feature".to_string(),
                name: "Test Feature".to_string(),
                description: "A test feature".to_string(),
                importance: FeatureImportance::Standard,
                service_dependencies: vec![],
                feature_dependencies: vec![],
                fallback_config: None,
                offline_capable: false,
            },
            availability,
            last_checked: chrono::Utc::now(),
            status_reason: "Test reason".to_string(),
            available_services: vec![],
            active_service: None,
            check_count: 1,
            performance: FeaturePerformanceMetrics::default(),
        }
    }
    
    #[test]
    fn test_degradation_notification_creation() {
        let notification = DegradationNotification::new(
            "test_feature".to_string(),
            "Test Feature".to_string(),
            FeatureAvailability::Degraded,
            "Feature is degraded".to_string(),
        );
        
        assert_eq!(notification.feature_id, "test_feature");
        assert_eq!(notification.feature_name, "Test Feature");
        assert_eq!(notification.availability, FeatureAvailability::Degraded);
        assert_eq!(notification.notification_type, NotificationType::Warning);
        assert_eq!(notification.priority, NotificationPriority::Medium);
        assert!(!notification.acknowledged);
    }
    
    #[test]
    fn test_system_degradation_notification() {
        let notification = SystemDegradationNotification::new(
            SystemDegradationLevel::Major,
            5,
            1,
        );
        
        assert_eq!(notification.degradation_level, SystemDegradationLevel::Major);
        assert_eq!(notification.notification_type, NotificationType::Error);
        assert_eq!(notification.priority, NotificationPriority::High);
        assert!(!notification.dismissible);
        assert!(!notification.acknowledged);
    }
    
    #[test]
    fn test_notification_manager_feature_notification() {
        let manager = UserNotificationManager::default();
        let feature_status = create_test_feature_status(FeatureAvailability::Degraded);
        
        let notification_id = manager.notify_feature_degradation(&feature_status).unwrap();
        assert!(!notification_id.is_empty());
        
        let active_notifications = manager.get_active_feature_notifications();
        assert_eq!(active_notifications.len(), 1);
        assert!(active_notifications.contains_key("test_feature"));
    }
    
    #[test]
    fn test_notification_acknowledgment() {
        let manager = UserNotificationManager::default();
        let feature_status = create_test_feature_status(FeatureAvailability::Degraded);
        
        manager.notify_feature_degradation(&feature_status).unwrap();
        
        let result = manager.acknowledge_feature_notification("test_feature");
        assert!(result.is_ok());
        
        // Degraded notifications are dismissible after acknowledgment
        let active_notifications = manager.get_active_feature_notifications();
        assert_eq!(active_notifications.len(), 0);
    }
    
    #[test]
    fn test_auto_dismiss_functionality() {
        let notification = DegradationNotification::new(
            "test_feature".to_string(),
            "Test Feature".to_string(),
            FeatureAvailability::Available, // Available notifications auto-dismiss
            "Feature restored".to_string(),
        );
        
        // Should not auto-dismiss immediately
        assert!(!notification.should_auto_dismiss());
        
        // Verify auto-dismiss timeout is set for Available status
        assert!(notification.auto_dismiss_ms.is_some());
        assert_eq!(notification.auto_dismiss_ms.unwrap(), 5000);
    }
    
    #[test]
    fn test_notification_priority_ordering() {
        assert!(NotificationPriority::Critical > NotificationPriority::High);
        assert!(NotificationPriority::High > NotificationPriority::Medium);
        assert!(NotificationPriority::Medium > NotificationPriority::Low);
    }
    
    #[test]
    fn test_notification_history() {
        let manager = UserNotificationManager::default();
        let feature_status = create_test_feature_status(FeatureAvailability::Unavailable);
        
        manager.notify_feature_degradation(&feature_status).unwrap();
        
        let history = manager.get_notification_history(Some(10));
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].feature_id, Some("test_feature".to_string()));
        assert!(history[0].acknowledged_at.is_none());
    }
}