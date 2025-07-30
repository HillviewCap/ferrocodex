use crate::error_handling::{EnhancedError, ErrorDomain, ErrorSeverity, RecoveryStrategy};
use crate::error_handling::graceful_degradation::{FeatureAvailability, FeatureStatus, SystemDegradationLevel, SystemDegradationStatus};
use crate::error_handling::service_provider::{ServiceHealth, ServiceHealthResult};
use crate::error_handling::user_notifications::{DegradationNotification, SystemDegradationNotification, NotificationType, NotificationPriority};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// UI status indicator types for different degradation states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UIStatusType {
    /// Success/operational status (green)
    Success,
    /// Warning/degraded status (yellow/orange)
    Warning,
    /// Error/unavailable status (red)
    Error,
    /// Info/loading status (blue)
    Info,
    /// Critical/severe status (dark red)
    Critical,
}

impl std::fmt::Display for UIStatusType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UIStatusType::Success => write!(f, "Success"),
            UIStatusType::Warning => write!(f, "Warning"),
            UIStatusType::Error => write!(f, "Error"),
            UIStatusType::Info => write!(f, "Info"),
            UIStatusType::Critical => write!(f, "Critical"),
        }
    }
}

impl From<FeatureAvailability> for UIStatusType {
    fn from(availability: FeatureAvailability) -> Self {
        match availability {
            FeatureAvailability::Available => UIStatusType::Success,
            FeatureAvailability::Degraded => UIStatusType::Warning,
            FeatureAvailability::Unavailable => UIStatusType::Error,
            FeatureAvailability::Disabled => UIStatusType::Critical,
        }
    }
}

impl From<SystemDegradationLevel> for UIStatusType {
    fn from(level: SystemDegradationLevel) -> Self {
        match level {
            SystemDegradationLevel::None => UIStatusType::Success,
            SystemDegradationLevel::Minor => UIStatusType::Info,
            SystemDegradationLevel::Major => UIStatusType::Warning,
            SystemDegradationLevel::Severe => UIStatusType::Critical,
        }
    }
}

impl From<ServiceHealth> for UIStatusType {
    fn from(health: ServiceHealth) -> Self {
        match health {
            ServiceHealth::Healthy => UIStatusType::Success,
            ServiceHealth::Degraded => UIStatusType::Warning,
            ServiceHealth::Unhealthy => UIStatusType::Error,
            ServiceHealth::Unknown => UIStatusType::Info,
        }
    }
}

impl From<NotificationType> for UIStatusType {
    fn from(notification_type: NotificationType) -> Self {
        match notification_type {
            NotificationType::Info => UIStatusType::Info,
            NotificationType::Warning => UIStatusType::Warning,
            NotificationType::Error => UIStatusType::Error,
            NotificationType::Critical => UIStatusType::Critical,
        }
    }
}

/// UI status indicator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIStatusIndicator {
    /// Unique identifier for the indicator
    pub id: String,
    /// Display label for the indicator
    pub label: String,
    /// Current status type
    pub status_type: UIStatusType,
    /// Status message
    pub message: String,
    /// Detailed description (optional)
    pub description: Option<String>,
    /// CSS color code for custom styling
    pub color: String,
    /// Icon name/identifier for UI frameworks
    pub icon: String,
    /// Whether the indicator is blinking/animated
    pub is_animated: bool,
    /// Priority level for display ordering
    pub priority: u32,
    /// Timestamp when status was last updated
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// Additional metadata for UI rendering
    pub metadata: HashMap<String, String>,
}

impl UIStatusIndicator {
    /// Create a new UI status indicator
    pub fn new(id: String, label: String, status_type: UIStatusType, message: String) -> Self {
        let (color, icon) = Self::get_style_for_type(&status_type);
        
        Self {
            id,
            label,
            status_type,
            message,
            description: None,
            color,
            icon,
            is_animated: false,
            priority: Self::get_priority_for_type(&status_type),
            last_updated: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }
    
    /// Create indicator from feature status
    pub fn from_feature_status(feature_status: &FeatureStatus) -> Self {
        let status_type = UIStatusType::from(feature_status.availability);
        let message = Self::generate_feature_message(feature_status);
        
        let mut indicator = Self::new(
            format!("feature_{}", feature_status.feature.id),
            feature_status.feature.name.clone(),
            status_type,
            message,
        );
        
        indicator.description = Some(feature_status.status_reason.clone());
        indicator.add_metadata("feature_id".to_string(), feature_status.feature.id.clone());
        indicator.add_metadata("check_count".to_string(), feature_status.check_count.to_string());
        indicator.add_metadata("performance_ok".to_string(), feature_status.performance.meets_requirement.to_string());
        
        if feature_status.availability == FeatureAvailability::Degraded || 
           feature_status.availability == FeatureAvailability::Unavailable {
            indicator.is_animated = true;
        }
        
        indicator
    }
    
    /// Create indicator from system degradation status
    pub fn from_system_status(system_status: &SystemDegradationStatus) -> Self {
        let status_type = UIStatusType::from(system_status.level);
        let message = Self::generate_system_message(system_status);
        
        let mut indicator = Self::new(
            "system_status".to_string(),
            "System Status".to_string(),
            status_type,
            message,
        );
        
        indicator.add_metadata("total_features".to_string(), system_status.total_features.to_string());
        indicator.add_metadata("available_features".to_string(), system_status.available_features.to_string());
        indicator.add_metadata("degraded_features".to_string(), system_status.degraded_features.to_string());
        indicator.add_metadata("unavailable_features".to_string(), system_status.unavailable_features.to_string());
        indicator.add_metadata("critical_affected".to_string(), system_status.critical_features_affected.to_string());
        indicator.add_metadata("cache_utilization".to_string(), format!("{:.1}%", system_status.cache_utilization_percent));
        indicator.add_metadata("failover_count".to_string(), system_status.failover_count.to_string());
        
        if system_status.level != SystemDegradationLevel::None {
            indicator.is_animated = true;
        }
        
        indicator
    }
    
    /// Create indicator from service health
    pub fn from_service_health(service_name: &str, health_result: &ServiceHealthResult) -> Self {
        let status_type = UIStatusType::from(health_result.health);
        let message = Self::generate_service_message(health_result);
        
        let mut indicator = Self::new(
            format!("service_{}", service_name),
            format!("Service: {}", service_name),
            status_type,
            message,
        );
        
        indicator.add_metadata("service_name".to_string(), service_name.to_string());
        indicator.add_metadata("priority".to_string(), health_result.priority.to_string());
        indicator.add_metadata("check_duration_ms".to_string(), health_result.check_duration.as_millis().to_string());
        
        if let Some(ref context) = health_result.context {
            indicator.description = Some(context.clone());
        }
        
        if health_result.health == ServiceHealth::Unhealthy {
            indicator.is_animated = true;
        }
        
        indicator
    }
    
    /// Add metadata to the indicator
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// Update the status type and related styling
    pub fn update_status(&mut self, status_type: UIStatusType, message: String) {
        self.status_type = status_type;
        self.message = message;
        self.last_updated = chrono::Utc::now();
        
        let (color, icon) = Self::get_style_for_type(&status_type);
        self.color = color;
        self.icon = icon;
        self.priority = Self::get_priority_for_type(&status_type);
        
        // Update animation based on status
        self.is_animated = matches!(status_type, UIStatusType::Warning | UIStatusType::Error | UIStatusType::Critical);
    }
    
    /// Get CSS color and icon for status type
    fn get_style_for_type(status_type: &UIStatusType) -> (String, String) {
        match status_type {
            UIStatusType::Success => ("#52c41a".to_string(), "check-circle".to_string()),
            UIStatusType::Warning => ("#faad14".to_string(), "warning-circle".to_string()),
            UIStatusType::Error => ("#ff4d4f".to_string(), "close-circle".to_string()),
            UIStatusType::Info => ("#1890ff".to_string(), "info-circle".to_string()),
            UIStatusType::Critical => ("#a8071a".to_string(), "exclamation-circle".to_string()),
        }
    }
    
    /// Get priority for status type (higher number = higher priority)
    fn get_priority_for_type(status_type: &UIStatusType) -> u32 {
        match status_type {
            UIStatusType::Critical => 100,
            UIStatusType::Error => 80,
            UIStatusType::Warning => 60,
            UIStatusType::Info => 40,
            UIStatusType::Success => 20,
        }
    }
    
    /// Generate message for feature status
    fn generate_feature_message(feature_status: &FeatureStatus) -> String {
        match feature_status.availability {
            FeatureAvailability::Available => "Fully operational".to_string(),
            FeatureAvailability::Degraded => format!("Degraded - {}", feature_status.status_reason),
            FeatureAvailability::Unavailable => format!("Unavailable - {}", feature_status.status_reason),
            FeatureAvailability::Disabled => "Disabled".to_string(),
        }
    }
    
    /// Generate message for system status
    fn generate_system_message(system_status: &SystemDegradationStatus) -> String {
        match system_status.level {
            SystemDegradationLevel::None => "All systems operational".to_string(),
            SystemDegradationLevel::Minor => {
                format!("{} features affected", system_status.degraded_features + system_status.unavailable_features)
            }
            SystemDegradationLevel::Major => {
                format!("Major degradation - {} features affected", 
                    system_status.degraded_features + system_status.unavailable_features)
            }
            SystemDegradationLevel::Severe => {
                format!("Severe degradation - {} critical features affected", 
                    system_status.critical_features_affected)
            }
        }
    }
    
    /// Generate message for service health
    fn generate_service_message(health_result: &ServiceHealthResult) -> String {
        match health_result.health {
            ServiceHealth::Healthy => "Service healthy".to_string(),
            ServiceHealth::Degraded => "Service degraded".to_string(),
            ServiceHealth::Unhealthy => "Service unhealthy".to_string(),
            ServiceHealth::Unknown => "Service status unknown".to_string(),
        }
    }
}

/// Degraded mode banner configuration for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradedModeBanner {
    /// Banner ID
    pub id: String,
    /// Banner title
    pub title: String,
    /// Banner message
    pub message: String,
    /// Banner type (determines styling)
    pub banner_type: UIStatusType,
    /// Whether banner is dismissible
    pub dismissible: bool,
    /// Auto-hide timeout (ms, 0 = no auto-hide)
    pub auto_hide_ms: u64,
    /// Show close button
    pub show_close_button: bool,
    /// Banner action buttons
    pub actions: Vec<BannerAction>,
    /// Whether banner should be sticky (always visible)
    pub sticky: bool,
    /// Timestamp when banner was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Action button for degraded mode banner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BannerAction {
    /// Action ID
    pub id: String,
    /// Button text
    pub label: String,
    /// Button type (primary, secondary, etc.)
    pub button_type: String,
    /// Action to perform when clicked
    pub action: String,
    /// Additional parameters for the action
    pub parameters: HashMap<String, String>,
}

impl DegradedModeBanner {
    /// Create a new degraded mode banner
    pub fn new(id: String, title: String, message: String, banner_type: UIStatusType) -> Self {
        Self {
            id,
            title,
            message,
            banner_type,
            dismissible: true,
            auto_hide_ms: 0,
            show_close_button: true,
            actions: Vec::new(),
            sticky: false,
            created_at: chrono::Utc::now(),
        }
    }
    
    /// Create banner for system degradation
    pub fn for_system_degradation(system_status: &SystemDegradationStatus) -> Self {
        let (title, message, banner_type) = match system_status.level {
            SystemDegradationLevel::None => {
                return Self::for_system_recovery();
            }
            SystemDegradationLevel::Minor => (
                "Minor System Degradation".to_string(),
                format!("Some features may be limited. {} features affected.", 
                    system_status.degraded_features + system_status.unavailable_features),
                UIStatusType::Info,
            ),
            SystemDegradationLevel::Major => (
                "System Degradation".to_string(),
                format!("Important features are degraded. {} features affected. Consider saving your work.", 
                    system_status.degraded_features + system_status.unavailable_features),
                UIStatusType::Warning,
            ),
            SystemDegradationLevel::Severe => (
                "Severe System Degradation".to_string(),
                format!("Critical system features are affected. {} critical features impacted. Save work immediately.", 
                    system_status.critical_features_affected),
                UIStatusType::Critical,
            ),
        };
        
        let mut banner = Self::new(
            "system_degradation".to_string(),
            title,
            message,
            banner_type,
        );
        
        // Configure banner based on severity
        match system_status.level {
            SystemDegradationLevel::Minor => {
                banner.dismissible = true;
                banner.auto_hide_ms = 10000; // 10 seconds
            }
            SystemDegradationLevel::Major => {
                banner.dismissible = true;
                banner.sticky = false;
                banner.add_action("view_status", "View Status", "primary", "show_status_dashboard");
                banner.add_action("dismiss", "Dismiss", "secondary", "dismiss_banner");
            }
            SystemDegradationLevel::Severe => {
                banner.dismissible = false;
                banner.sticky = true;
                banner.add_action("safe_mode", "Safe Mode", "primary", "enable_safe_mode");
                banner.add_action("view_details", "View Details", "secondary", "show_detailed_status");
            }
            SystemDegradationLevel::None => unreachable!(),
        }
        
        banner
    }
    
    /// Create banner for system recovery
    pub fn for_system_recovery() -> Self {
        let mut banner = Self::new(
            "system_recovery".to_string(),
            "System Restored".to_string(),
            "All systems are now operational.".to_string(),
            UIStatusType::Success,
        );
        
        banner.dismissible = true;
        banner.auto_hide_ms = 5000; // 5 seconds
        banner.add_action("dismiss", "Dismiss", "primary", "dismiss_banner");
        
        banner
    }
    
    /// Create banner for feature degradation
    pub fn for_feature_degradation(feature_status: &FeatureStatus) -> Self {
        let (title, message, banner_type) = match feature_status.availability {
            FeatureAvailability::Available => {
                return Self::for_feature_recovery(&feature_status.feature.name);
            }
            FeatureAvailability::Degraded => (
                format!("{} Degraded", feature_status.feature.name),
                format!("The {} feature is running in degraded mode. {}", 
                    feature_status.feature.name, feature_status.status_reason),
                UIStatusType::Warning,
            ),
            FeatureAvailability::Unavailable => (
                format!("{} Unavailable", feature_status.feature.name),
                format!("The {} feature is currently unavailable. {}", 
                    feature_status.feature.name, feature_status.status_reason),
                UIStatusType::Error,
            ),
            FeatureAvailability::Disabled => (
                format!("{} Disabled", feature_status.feature.name),
                format!("The {} feature has been disabled.", feature_status.feature.name),
                UIStatusType::Critical,
            ),
        };
        
        let mut banner = Self::new(
            format!("feature_{}", feature_status.feature.id),
            title,
            message,
            banner_type,
        );
        
        // Add feature-specific actions
        match feature_status.availability {
            FeatureAvailability::Degraded => {
                banner.add_action("continue", "Continue", "primary", "dismiss_banner");
                banner.add_action("details", "Details", "secondary", "show_feature_details");
            }
            FeatureAvailability::Unavailable => {
                banner.add_action("retry", "Retry", "primary", "retry_feature");
                banner.add_action("offline", "Use Offline", "secondary", "enable_offline_mode");
            }
            FeatureAvailability::Disabled => {
                banner.dismissible = false;
                banner.add_action("contact", "Contact Admin", "primary", "contact_administrator");
            }
            FeatureAvailability::Available => unreachable!(),
        }
        
        banner
    }
    
    /// Create banner for feature recovery
    pub fn for_feature_recovery(feature_name: &str) -> Self {
        let mut banner = Self::new(
            format!("feature_recovery_{}", feature_name.to_lowercase()),
            format!("{} Restored", feature_name),
            format!("The {} feature is now fully operational.", feature_name),
            UIStatusType::Success,
        );
        
        banner.dismissible = true;
        banner.auto_hide_ms = 3000; // 3 seconds
        
        banner
    }
    
    /// Add an action button to the banner
    pub fn add_action(&mut self, id: &str, label: &str, button_type: &str, action: &str) {
        let action_button = BannerAction {
            id: id.to_string(),
            label: label.to_string(),
            button_type: button_type.to_string(),
            action: action.to_string(),
            parameters: HashMap::new(),
        };
        self.actions.push(action_button);
    }
}

/// Service status dashboard data for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatusDashboard {
    /// Dashboard title
    pub title: String,
    /// Overall system health indicator
    pub system_health: UIStatusIndicator,
    /// Individual service indicators
    pub service_indicators: Vec<UIStatusIndicator>,
    /// Feature indicators
    pub feature_indicators: Vec<UIStatusIndicator>,
    /// Current active notifications
    pub active_notifications: Vec<UIStatusIndicator>,
    /// Performance metrics summary
    pub performance_summary: PerformanceSummary,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Performance metrics summary for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    /// Average response time (ms)
    pub avg_response_time_ms: f64,
    /// Cache hit rate (0.0-1.0)
    pub cache_hit_rate: f64,
    /// System uptime percentage (0.0-1.0)
    pub uptime_percentage: f64,
    /// Number of active failovers
    pub active_failovers: u64,
    /// Total requests processed
    pub total_requests: u64,
    /// Error rate (0.0-1.0)
    pub error_rate: f64,
}

impl Default for PerformanceSummary {
    fn default() -> Self {
        Self {
            avg_response_time_ms: 0.0,
            cache_hit_rate: 0.0,
            uptime_percentage: 1.0,
            active_failovers: 0,
            total_requests: 0,
            error_rate: 0.0,
        }
    }
}

impl ServiceStatusDashboard {
    /// Create a new service status dashboard
    pub fn new() -> Self {
        Self {
            title: "System Status Dashboard".to_string(),
            system_health: UIStatusIndicator::new(
                "system_health".to_string(),
                "System Health".to_string(),
                UIStatusType::Success,
                "All systems operational".to_string(),
            ),
            service_indicators: Vec::new(),
            feature_indicators: Vec::new(),
            active_notifications: Vec::new(),
            performance_summary: PerformanceSummary::default(),
            last_updated: chrono::Utc::now(),
        }
    }
    
    /// Update dashboard with current system status
    pub fn update_with_system_status(&mut self, system_status: &SystemDegradationStatus) {
        self.system_health = UIStatusIndicator::from_system_status(system_status);
        self.last_updated = chrono::Utc::now();
    }
    
    /// Add or update service indicator
    pub fn update_service_indicator(&mut self, service_name: &str, health_result: &ServiceHealthResult) {
        let indicator = UIStatusIndicator::from_service_health(service_name, health_result);
        
        // Replace existing indicator or add new one
        if let Some(existing) = self.service_indicators.iter_mut().find(|i| i.id == indicator.id) {
            *existing = indicator;
        } else {
            self.service_indicators.push(indicator);
        }
        
        // Sort by priority (highest first)
        self.service_indicators.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.last_updated = chrono::Utc::now();
    }
    
    /// Add or update feature indicator
    pub fn update_feature_indicator(&mut self, feature_status: &FeatureStatus) {
        let indicator = UIStatusIndicator::from_feature_status(feature_status);
        
        // Replace existing indicator or add new one
        if let Some(existing) = self.feature_indicators.iter_mut().find(|i| i.id == indicator.id) {
            *existing = indicator;
        } else {
            self.feature_indicators.push(indicator);
        }
        
        // Sort by priority (highest first)
        self.feature_indicators.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.last_updated = chrono::Utc::now();
    }
    
    /// Add notification indicator
    pub fn add_notification_indicator(&mut self, notification: &DegradationNotification) {
        let status_type = UIStatusType::from(notification.notification_type);
        let mut indicator = UIStatusIndicator::new(
            notification.id.clone(),
            notification.feature_name.clone(),
            status_type,
            notification.message.clone(),
        );
        
        indicator.add_metadata("feature_id".to_string(), notification.feature_id.clone());
        indicator.add_metadata("priority".to_string(), notification.priority.to_string());
        indicator.add_metadata("dismissible".to_string(), notification.dismissible.to_string());
        
        if let Some(auto_dismiss_ms) = notification.auto_dismiss_ms {
            indicator.add_metadata("auto_dismiss_ms".to_string(), auto_dismiss_ms.to_string());
        }
        
        self.active_notifications.push(indicator);
        self.active_notifications.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.last_updated = chrono::Utc::now();
    }
    
    /// Remove notification indicator
    pub fn remove_notification_indicator(&mut self, notification_id: &str) {
        self.active_notifications.retain(|n| n.id != notification_id);
        self.last_updated = chrono::Utc::now();
    }
    
    /// Update performance summary
    pub fn update_performance_summary(&mut self, summary: PerformanceSummary) {
        self.performance_summary = summary;
        self.last_updated = chrono::Utc::now();
    }
    
    /// Get critical indicators that need immediate attention
    pub fn get_critical_indicators(&self) -> Vec<&UIStatusIndicator> {
        let mut critical = Vec::new();
        
        if self.system_health.status_type == UIStatusType::Critical {
            critical.push(&self.system_health);
        }
        
        critical.extend(
            self.service_indicators.iter()
                .filter(|i| i.status_type == UIStatusType::Critical || i.status_type == UIStatusType::Error)
        );
        
        critical.extend(
            self.feature_indicators.iter()
                .filter(|i| i.status_type == UIStatusType::Critical || i.status_type == UIStatusType::Error)
        );
        
        critical.extend(
            self.active_notifications.iter()
                .filter(|i| i.status_type == UIStatusType::Critical || i.status_type == UIStatusType::Error)
        );
        
        critical
    }
}

impl Default for ServiceStatusDashboard {
    fn default() -> Self {
        Self::new()
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
    fn test_ui_status_indicator_creation() {
        let indicator = UIStatusIndicator::new(
            "test_id".to_string(),
            "Test Label".to_string(),
            UIStatusType::Warning,
            "Test message".to_string(),
        );
        
        assert_eq!(indicator.id, "test_id");
        assert_eq!(indicator.label, "Test Label");
        assert_eq!(indicator.status_type, UIStatusType::Warning);
        assert_eq!(indicator.message, "Test message");
        assert_eq!(indicator.color, "#faad14");
        assert_eq!(indicator.icon, "warning-circle");
        assert_eq!(indicator.priority, 60);
    }
    
    #[test]
    fn test_indicator_from_feature_status() {
        let feature_status = create_test_feature_status(FeatureAvailability::Degraded);
        let indicator = UIStatusIndicator::from_feature_status(&feature_status);
        
        assert_eq!(indicator.id, "feature_test_feature");
        assert_eq!(indicator.label, "Test Feature");
        assert_eq!(indicator.status_type, UIStatusType::Warning);
        assert!(indicator.is_animated);
        
        let feature_id = indicator.metadata.get("feature_id").unwrap();
        assert_eq!(feature_id, "test_feature");
    }
    
    #[test]
    fn test_status_type_conversions() {
        assert_eq!(UIStatusType::from(FeatureAvailability::Available), UIStatusType::Success);
        assert_eq!(UIStatusType::from(FeatureAvailability::Degraded), UIStatusType::Warning);
        assert_eq!(UIStatusType::from(FeatureAvailability::Unavailable), UIStatusType::Error);
        assert_eq!(UIStatusType::from(FeatureAvailability::Disabled), UIStatusType::Critical);
        
        assert_eq!(UIStatusType::from(SystemDegradationLevel::None), UIStatusType::Success);
        assert_eq!(UIStatusType::from(SystemDegradationLevel::Minor), UIStatusType::Info);
        assert_eq!(UIStatusType::from(SystemDegradationLevel::Major), UIStatusType::Warning);
        assert_eq!(UIStatusType::from(SystemDegradationLevel::Severe), UIStatusType::Critical);
    }
    
    #[test]
    fn test_degraded_mode_banner_creation() {
        let system_status = SystemDegradationStatus {
            level: SystemDegradationLevel::Major,
            total_features: 10,
            available_features: 6,
            degraded_features: 3,
            unavailable_features: 1,
            critical_features_affected: 0,
            cache_utilization_percent: 75.0,
            failover_count: 2,
            last_updated: chrono::Utc::now(),
        };
        
        let banner = DegradedModeBanner::for_system_degradation(&system_status);
        
        assert_eq!(banner.id, "system_degradation");
        assert_eq!(banner.title, "System Degradation");
        assert_eq!(banner.banner_type, UIStatusType::Warning);
        assert!(banner.dismissible);
        assert!(!banner.sticky);
        assert_eq!(banner.actions.len(), 2);
    }
    
    #[test]
    fn test_service_status_dashboard() {
        let mut dashboard = ServiceStatusDashboard::new();
        
        // Test initial state
        assert_eq!(dashboard.system_health.status_type, UIStatusType::Success);
        assert_eq!(dashboard.service_indicators.len(), 0);
        assert_eq!(dashboard.feature_indicators.len(), 0);
        
        // Test adding feature indicator
        let feature_status = create_test_feature_status(FeatureAvailability::Degraded);
        dashboard.update_feature_indicator(&feature_status);
        
        assert_eq!(dashboard.feature_indicators.len(), 1);
        assert_eq!(dashboard.feature_indicators[0].status_type, UIStatusType::Warning);
        
        // Test getting critical indicators
        let critical = dashboard.get_critical_indicators();
        assert_eq!(critical.len(), 0); // Warning is not critical
        
        // Add a critical feature
        let critical_feature = create_test_feature_status(FeatureAvailability::Disabled);
        dashboard.update_feature_indicator(&critical_feature);
        
        let critical = dashboard.get_critical_indicators();
        assert_eq!(critical.len(), 1); // Now we have one critical
    }
    
    #[test]
    fn test_performance_summary() {
        let summary = PerformanceSummary {
            avg_response_time_ms: 150.0,
            cache_hit_rate: 0.85,
            uptime_percentage: 0.99,
            active_failovers: 1,
            total_requests: 10000,
            error_rate: 0.02,
        };
        
        assert_eq!(summary.avg_response_time_ms, 150.0);
        assert_eq!(summary.cache_hit_rate, 0.85);
        assert_eq!(summary.uptime_percentage, 0.99);
        assert_eq!(summary.active_failovers, 1);
        assert_eq!(summary.total_requests, 10000);
        assert_eq!(summary.error_rate, 0.02);
    }
}