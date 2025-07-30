use crate::error_handling::{EnhancedError, ErrorDomain, ErrorSeverity, RecoveryStrategy};
use crate::error_handling::service_provider::{ServiceProvider, ServiceRegistry, FailoverExecutor};
use crate::error_handling::offline_providers::OfflineCacheManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Feature availability status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureAvailability {
    /// Feature is fully available
    Available,
    /// Feature is available but with reduced functionality
    Degraded,
    /// Feature is temporarily unavailable but may recover
    Unavailable,
    /// Feature is disabled and will not recover automatically
    Disabled,
}

impl std::fmt::Display for FeatureAvailability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeatureAvailability::Available => write!(f, "Available"),
            FeatureAvailability::Degraded => write!(f, "Degraded"),
            FeatureAvailability::Unavailable => write!(f, "Unavailable"),
            FeatureAvailability::Disabled => write!(f, "Disabled"),
        }
    }
}

/// Feature importance level for degradation decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FeatureImportance {
    /// Critical features must remain available (safety-related)
    Critical = 0,
    /// Important features should remain available if possible
    Important = 1,
    /// Standard features can be degraded
    Standard = 2,
    /// Optional features can be disabled
    Optional = 3,
}

impl std::fmt::Display for FeatureImportance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeatureImportance::Critical => write!(f, "Critical"),
            FeatureImportance::Important => write!(f, "Important"),
            FeatureImportance::Standard => write!(f, "Standard"),
            FeatureImportance::Optional => write!(f, "Optional"),
        }
    }
}

/// Feature definition for availability management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureDefinition {
    /// Feature identifier
    pub id: String,
    /// Human-readable feature name
    pub name: String,
    /// Feature description
    pub description: String,
    /// Feature importance level
    pub importance: FeatureImportance,
    /// Services this feature depends on
    pub service_dependencies: Vec<String>,
    /// Other features this feature depends on
    pub feature_dependencies: Vec<String>,
    /// Fallback mode configuration
    pub fallback_config: Option<FallbackConfig>,
    /// Whether feature can operate in offline mode
    pub offline_capable: bool,
}

/// Fallback configuration for degraded operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    /// Fallback mode description
    pub mode_description: String,
    /// Maximum cache age allowed for fallback (seconds)
    pub max_cache_age_seconds: u64,
    /// Reduced functionality description
    pub reduced_functionality: Vec<String>,
    /// User notification message for fallback mode
    pub user_notification: String,
}

/// Feature status tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureStatus {
    /// Feature definition
    pub feature: FeatureDefinition,
    /// Current availability status
    pub availability: FeatureAvailability,
    /// Last status check timestamp
    pub last_checked: chrono::DateTime<chrono::Utc>,
    /// Reason for current status
    pub status_reason: String,
    /// Available service providers for this feature
    pub available_services: Vec<String>,
    /// Currently active service for this feature
    pub active_service: Option<String>,
    /// Number of consecutive status checks
    pub check_count: u64,
    /// Performance metrics
    pub performance: FeaturePerformanceMetrics,
}

/// Performance metrics for feature availability checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturePerformanceMetrics {
    /// Average availability check time (microseconds)
    pub avg_check_time_us: f64,
    /// Number of samples
    pub sample_count: usize,
    /// Minimum check time
    pub min_check_time_us: u128,
    /// Maximum check time
    pub max_check_time_us: u128,
    /// Whether performance requirement is met (<10ms)
    pub meets_requirement: bool,
}

impl Default for FeaturePerformanceMetrics {
    fn default() -> Self {
        Self {
            avg_check_time_us: 0.0,
            sample_count: 0,
            min_check_time_us: 0,
            max_check_time_us: 0,
            meets_requirement: true,
        }
    }
}

/// Feature availability manager
pub struct FeatureAvailabilityManager {
    /// Registered features
    features: Arc<Mutex<HashMap<String, FeatureDefinition>>>,
    /// Feature status tracking
    status: Arc<Mutex<HashMap<String, FeatureStatus>>>,
    /// Service registry for health monitoring
    service_registry: Arc<ServiceRegistry>,
    /// Configuration
    config: FeatureAvailabilityConfig,
    /// Monitoring task handle
    monitoring_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

/// Configuration for feature availability management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureAvailabilityConfig {
    /// How often to check feature availability (milliseconds)
    pub check_interval_ms: u64,
    /// Timeout for availability checks (milliseconds)
    pub check_timeout_ms: u64,
    /// Enable automatic feature degradation
    pub auto_degradation: bool,
    /// Enable automatic feature recovery
    pub auto_recovery: bool,
    /// Minimum time between status changes (milliseconds)
    pub status_change_cooldown_ms: u64,
}

impl Default for FeatureAvailabilityConfig {
    fn default() -> Self {
        Self {
            check_interval_ms: 15000, // 15 seconds
            check_timeout_ms: 5000,   // 5 seconds
            auto_degradation: true,
            auto_recovery: true,
            status_change_cooldown_ms: 30000, // 30 seconds
        }
    }
}

impl FeatureAvailabilityManager {
    /// Create a new feature availability manager
    pub fn new(service_registry: Arc<ServiceRegistry>, config: FeatureAvailabilityConfig) -> Self {
        Self {
            features: Arc::new(Mutex::new(HashMap::new())),
            status: Arc::new(Mutex::new(HashMap::new())),
            service_registry,
            config,
            monitoring_handle: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Create with default configuration
    pub fn with_default_config(service_registry: Arc<ServiceRegistry>) -> Self {
        Self::new(service_registry, FeatureAvailabilityConfig::default())
    }
    
    /// Register a feature for availability management
    pub fn register_feature(&self, feature: FeatureDefinition) -> Result<(), EnhancedError> {
        let feature_id = feature.id.clone();
        
        // Initialize feature status
        let status = FeatureStatus {
            feature: feature.clone(),
            availability: FeatureAvailability::Available, // Start optimistic
            last_checked: chrono::Utc::now(),
            status_reason: "Initial registration".to_string(),
            available_services: Vec::new(),
            active_service: None,
            check_count: 0,
            performance: FeaturePerformanceMetrics::default(),
        };
        
        {
            let mut features = self.features.lock().unwrap();
            let mut status_map = self.status.lock().unwrap();
            
            features.insert(feature_id.clone(), feature);
            status_map.insert(feature_id, status);
        }
        
        Ok(())
    }
    
    /// Check availability of a specific feature
    pub async fn check_feature_availability(&self, feature_id: &str) -> Result<FeatureAvailability, EnhancedError> {
        let start_time = Instant::now();
        
        let (feature, mut current_status) = {
            let features = self.features.lock().unwrap();
            let status_map = self.status.lock().unwrap();
            
            match (features.get(feature_id), status_map.get(feature_id)) {
                (Some(f), Some(s)) => (f.clone(), s.clone()),
                _ => return Err(EnhancedError::new(
                    ErrorSeverity::Medium,
                    ErrorDomain::System,
                    RecoveryStrategy::UserRecoverable,
                    format!("Feature '{}' not registered", feature_id),
                )),
            }
        };
        
        // Check service dependencies
        let mut available_services = Vec::new();
        let mut any_service_available = false;
        
        for service_name in &feature.service_dependencies {
            // Find service in registry
            let providers = self.service_registry.get_providers();
            if let Some(provider) = providers.iter().find(|p| p.name() == service_name) {
                if provider.is_available().await {
                    let health = provider.health_check().await;
                    if health != crate::error_handling::service_provider::ServiceHealth::Unhealthy {
                        available_services.push(service_name.clone());
                        any_service_available = true;
                    }
                }
            }
        }
        
        // Check feature dependencies
        let mut feature_deps_available = true;
        for dep_feature_id in &feature.feature_dependencies {
            let dep_availability = self.get_feature_status(dep_feature_id)
                .map(|s| s.availability)
                .unwrap_or(FeatureAvailability::Unavailable);
            
            if dep_availability == FeatureAvailability::Unavailable || 
               dep_availability == FeatureAvailability::Disabled {
                feature_deps_available = false;
                break;
            }
        }
        
        // Determine availability
        let new_availability = if !feature_deps_available {
            FeatureAvailability::Unavailable
        } else if !any_service_available {
            if feature.offline_capable {
                FeatureAvailability::Degraded
            } else {
                FeatureAvailability::Unavailable
            }
        } else if available_services.len() < feature.service_dependencies.len() {
            FeatureAvailability::Degraded
        } else {
            FeatureAvailability::Available
        };
        
        // Update status
        let check_duration = start_time.elapsed();
        current_status.availability = new_availability;
        current_status.last_checked = chrono::Utc::now();
        current_status.status_reason = self.generate_status_reason(&feature, &available_services, feature_deps_available);
        current_status.available_services = available_services;
        current_status.check_count += 1;
        
        // Update performance metrics
        self.update_performance_metrics(&mut current_status.performance, check_duration);
        
        // Store updated status
        {
            let mut status_map = self.status.lock().unwrap();
            status_map.insert(feature_id.to_string(), current_status);
        }
        
        Ok(new_availability)
    }
    
    /// Get current status of a feature
    pub fn get_feature_status(&self, feature_id: &str) -> Option<FeatureStatus> {
        let status_map = self.status.lock().unwrap();
        status_map.get(feature_id).cloned()
    }
    
    /// Get all feature statuses
    pub fn get_all_feature_statuses(&self) -> HashMap<String, FeatureStatus> {
        self.status.lock().unwrap().clone()
    }
    
    /// Get features by availability status
    pub fn get_features_by_availability(&self, availability: FeatureAvailability) -> Vec<FeatureStatus> {
        let status_map = self.status.lock().unwrap();
        status_map.values()
            .filter(|s| s.availability == availability)
            .cloned()
            .collect()
    }
    
    /// Get features by importance level
    pub fn get_features_by_importance(&self, importance: FeatureImportance) -> Vec<FeatureStatus> {
        let status_map = self.status.lock().unwrap();
        status_map.values()
            .filter(|s| s.feature.importance == importance)
            .cloned()
            .collect()
    }
    
    /// Manually set feature availability (for admin control)
    pub fn set_feature_availability(&self, feature_id: &str, availability: FeatureAvailability, reason: &str) -> Result<(), EnhancedError> {
        let mut status_map = self.status.lock().unwrap();
        
        if let Some(status) = status_map.get_mut(feature_id) {
            status.availability = availability;
            status.status_reason = reason.to_string();
            status.last_checked = chrono::Utc::now();
            Ok(())
        } else {
            Err(EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::System,
                RecoveryStrategy::UserRecoverable,
                format!("Feature '{}' not found", feature_id),
            ))
        }
    }
    
    /// Start monitoring feature availability
    pub async fn start_monitoring(&self) {
        let features = self.features.clone();
        let status = self.status.clone();
        let service_registry = self.service_registry.clone();
        let config = self.config.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_millis(config.check_interval_ms)
            );
            
            loop {
                interval.tick().await;
                
                // Check all registered features
                let feature_ids: Vec<String> = {
                    let features_map = features.lock().unwrap();
                    features_map.keys().cloned().collect()
                };
                
                for feature_id in feature_ids {
                    // Create a temporary manager instance for the async check
                    let temp_manager = FeatureAvailabilityManager {
                        features: features.clone(),
                        status: status.clone(),
                        service_registry: service_registry.clone(),
                        config: config.clone(),
                        monitoring_handle: Arc::new(Mutex::new(None)),
                    };
                    
                    // Perform availability check with timeout
                    let check_result = tokio::time::timeout(
                        Duration::from_millis(config.check_timeout_ms),
                        temp_manager.check_feature_availability(&feature_id)
                    ).await;
                    
                    if let Err(_timeout) = check_result {
                        // Handle timeout - mark as unknown/degraded
                        let mut status_map = status.lock().unwrap();
                        if let Some(feature_status) = status_map.get_mut(&feature_id) {
                            feature_status.availability = FeatureAvailability::Degraded;
                            feature_status.status_reason = "Availability check timeout".to_string();
                            feature_status.last_checked = chrono::Utc::now();
                        }
                    }
                }
            }
        });
        
        *self.monitoring_handle.lock().unwrap() = Some(handle);
    }
    
    /// Stop monitoring
    pub async fn stop_monitoring(&self) {
        if let Some(handle) = self.monitoring_handle.lock().unwrap().take() {
            handle.abort();
        }
    }
    
    /// Generate a descriptive status reason
    fn generate_status_reason(&self, feature: &FeatureDefinition, available_services: &[String], feature_deps_available: bool) -> String {
        if !feature_deps_available {
            "Feature dependencies unavailable".to_string()
        } else if available_services.is_empty() {
            if feature.offline_capable {
                "Operating in offline mode".to_string()
            } else {
                "No services available".to_string()
            }
        } else if available_services.len() < feature.service_dependencies.len() {
            format!("Degraded mode: {} of {} services available", 
                available_services.len(), 
                feature.service_dependencies.len())
        } else {
            "All services available".to_string()
        }
    }
    
    /// Update performance metrics for feature checks
    fn update_performance_metrics(&self, metrics: &mut FeaturePerformanceMetrics, check_duration: Duration) {
        let check_time_us = check_duration.as_micros();
        
        if metrics.sample_count == 0 {
            metrics.avg_check_time_us = check_time_us as f64;
            metrics.min_check_time_us = check_time_us;
            metrics.max_check_time_us = check_time_us;
        } else {
            // Update running average
            metrics.avg_check_time_us = 
                (metrics.avg_check_time_us * metrics.sample_count as f64 + check_time_us as f64) 
                / (metrics.sample_count + 1) as f64;
            
            metrics.min_check_time_us = metrics.min_check_time_us.min(check_time_us);
            metrics.max_check_time_us = metrics.max_check_time_us.max(check_time_us);
        }
        
        metrics.sample_count += 1;
        metrics.meets_requirement = metrics.avg_check_time_us < 10_000.0; // 10ms requirement in microseconds
        
        // Keep only recent samples to avoid overflow
        if metrics.sample_count > 1000 {
            metrics.sample_count = 500; // Reset to mid-range
        }
    }
}

/// Graceful degradation coordinator
pub struct GracefulDegradationCoordinator {
    /// Feature availability manager
    feature_manager: Arc<FeatureAvailabilityManager>,
    /// Service registry
    service_registry: Arc<ServiceRegistry>,
    /// Offline cache manager
    cache_manager: Arc<OfflineCacheManager>,
    /// Failover executor
    failover_executor: Arc<FailoverExecutor>,
    /// Degradation configuration
    config: DegradationConfig,
}

/// Configuration for graceful degradation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationConfig {
    /// Enable automatic degradation
    pub auto_degradation: bool,
    /// Enable automatic recovery
    pub auto_recovery: bool,
    /// Degradation decision timeout (milliseconds)
    pub degradation_timeout_ms: u64,
    /// Cache utilization threshold for degradation (0.0-1.0)
    pub cache_degradation_threshold: f64,
    /// Service failure threshold for degradation
    pub service_failure_threshold: u32,
}

impl Default for DegradationConfig {
    fn default() -> Self {
        Self {
            auto_degradation: true,
            auto_recovery: true,
            degradation_timeout_ms: 1000, // 1 second
            cache_degradation_threshold: 0.8, // 80%
            service_failure_threshold: 3,
        }
    }
}

impl GracefulDegradationCoordinator {
    /// Create a new graceful degradation coordinator
    pub fn new(
        feature_manager: Arc<FeatureAvailabilityManager>,
        service_registry: Arc<ServiceRegistry>,
        cache_manager: Arc<OfflineCacheManager>,
        failover_executor: Arc<FailoverExecutor>,
        config: DegradationConfig,
    ) -> Self {
        Self {
            feature_manager,
            service_registry,
            cache_manager,
            failover_executor,
            config,
        }
    }
    
    /// Create with default configuration
    pub fn with_default_config(
        feature_manager: Arc<FeatureAvailabilityManager>,
        service_registry: Arc<ServiceRegistry>,
        cache_manager: Arc<OfflineCacheManager>,
        failover_executor: Arc<FailoverExecutor>,
    ) -> Self {
        Self::new(feature_manager, service_registry, cache_manager, failover_executor, DegradationConfig::default())
    }
    
    /// Execute operation with graceful degradation
    pub async fn execute_with_degradation<T, F, Fut>(&self, 
        feature_id: &str, 
        operation: F
    ) -> Result<T, EnhancedError>
    where
        F: Fn(Arc<dyn ServiceProvider>) -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<Output = Result<T, EnhancedError>> + Send,
        T: Send,
    {
        // Check feature availability first
        let availability = self.feature_manager.check_feature_availability(feature_id).await?;
        
        match availability {
            FeatureAvailability::Available => {
                // Try normal operation with failover
                self.failover_executor.execute(operation).await
            }
            FeatureAvailability::Degraded => {
                // Try degraded operation (may use cache or limited functionality)
                match self.try_degraded_operation(feature_id, operation).await {
                    Ok(result) => Ok(result),
                    Err(_) => {
                        // Degraded operation failed, update feature status
                        self.feature_manager.set_feature_availability(
                            feature_id, 
                            FeatureAvailability::Unavailable, 
                            "Degraded operation failed"
                        )?;
                        Err(EnhancedError::new(
                            ErrorSeverity::High,
                            ErrorDomain::System,
                            RecoveryStrategy::ManualRecoverable,
                            format!("Feature '{}' degraded operation failed", feature_id),
                        ))
                    }
                }
            }
            FeatureAvailability::Unavailable => {
                // Try offline operation if supported
                if let Some(feature_status) = self.feature_manager.get_feature_status(feature_id) {
                    if feature_status.feature.offline_capable {
                        self.try_offline_operation(feature_id).await
                    } else {
                        Err(EnhancedError::new(
                            ErrorSeverity::High,
                            ErrorDomain::System,
                            RecoveryStrategy::ManualRecoverable,
                            format!("Feature '{}' is unavailable and not offline-capable", feature_id),
                        ))
                    }
                } else {
                    Err(EnhancedError::new(
                        ErrorSeverity::High,
                        ErrorDomain::System,
                        RecoveryStrategy::UserRecoverable,
                        format!("Feature '{}' not found", feature_id),
                    ))
                }
            }
            FeatureAvailability::Disabled => {
                Err(EnhancedError::new(
                    ErrorSeverity::High,
                    ErrorDomain::System,
                    RecoveryStrategy::UserRecoverable,
                    format!("Feature '{}' is disabled", feature_id),
                ))
            }
        }
    }
    
    /// Try degraded operation (placeholder - would be implemented per feature)
    async fn try_degraded_operation<T, F, Fut>(&self, 
        _feature_id: &str, 
        operation: F
    ) -> Result<T, EnhancedError>
    where
        F: Fn(Arc<dyn ServiceProvider>) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T, EnhancedError>> + Send,
        T: Send,
    {
        // For now, try with available services but with shorter timeout
        let providers = self.service_registry.get_providers();
        for provider in providers {
            if provider.is_available().await {
                match tokio::time::timeout(
                    Duration::from_millis(self.config.degradation_timeout_ms),
                    operation(provider)
                ).await {
                    Ok(result) => return result,
                    Err(_) => continue, // Timeout, try next provider
                }
            }
        }
        
        Err(EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::System,
            RecoveryStrategy::AutoRecoverable,
            "All degraded operations failed".to_string(),
        ))
    }
    
    /// Try offline operation using cached data
    async fn try_offline_operation<T>(&self, feature_id: &str) -> Result<T, EnhancedError> {
        // This would be implemented per feature type
        // For now, return an appropriate error
        Err(EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::System,
            RecoveryStrategy::ManualRecoverable,
            format!("Offline operation not implemented for feature '{}'", feature_id),
        ))
    }
    
    /// Get overall system degradation status
    pub fn get_system_degradation_status(&self) -> SystemDegradationStatus {
        let all_statuses = self.feature_manager.get_all_feature_statuses();
        let cache_stats = self.cache_manager.get_stats();
        let failover_stats = self.failover_executor.get_stats();
        
        let total_features = all_statuses.len();
        let available_features = all_statuses.values()
            .filter(|s| s.availability == FeatureAvailability::Available)
            .count();
        let degraded_features = all_statuses.values()
            .filter(|s| s.availability == FeatureAvailability::Degraded)
            .count();
        let unavailable_features = all_statuses.values()
            .filter(|s| s.availability == FeatureAvailability::Unavailable)
            .count();
        
        let critical_features_affected = all_statuses.values()
            .filter(|s| s.feature.importance == FeatureImportance::Critical && 
                      s.availability != FeatureAvailability::Available)
            .count();
        
        let overall_health = if critical_features_affected > 0 {
            SystemDegradationLevel::Severe
        } else if unavailable_features > total_features / 2 {
            SystemDegradationLevel::Major
        } else if degraded_features > 0 || unavailable_features > 0 {
            SystemDegradationLevel::Minor
        } else {
            SystemDegradationLevel::None
        };
        
        SystemDegradationStatus {
            level: overall_health,
            total_features,
            available_features,
            degraded_features,
            unavailable_features,
            critical_features_affected,
            cache_utilization_percent: cache_stats.utilization_percent,
            failover_count: failover_stats.failover_count,
            last_updated: chrono::Utc::now(),
        }
    }
}

/// System-wide degradation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemDegradationLevel {
    /// No degradation - all systems operational
    None,
    /// Minor degradation - some optional features affected
    Minor,
    /// Major degradation - important features affected
    Major,
    /// Severe degradation - critical features affected
    Severe,
}

/// Overall system degradation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDegradationStatus {
    /// Overall degradation level
    pub level: SystemDegradationLevel,
    /// Total number of features
    pub total_features: usize,
    /// Number of available features
    pub available_features: usize,
    /// Number of degraded features
    pub degraded_features: usize,
    /// Number of unavailable features
    pub unavailable_features: usize,
    /// Number of critical features affected
    pub critical_features_affected: usize,
    /// Cache utilization percentage
    pub cache_utilization_percent: f64,
    /// Number of failovers performed
    pub failover_count: u64,
    /// Last status update
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_handling::service_provider::{ServiceRegistry, ServiceMonitoringConfig};
    use crate::error_handling::offline_providers::OfflineCacheConfig;
    
    #[tokio::test]
    async fn test_feature_definition_and_registration() {
        let service_registry = Arc::new(ServiceRegistry::new(ServiceMonitoringConfig::default()));
        let manager = FeatureAvailabilityManager::with_default_config(service_registry);
        
        let feature = FeatureDefinition {
            id: "test_feature".to_string(),
            name: "Test Feature".to_string(),
            description: "A test feature".to_string(),
            importance: FeatureImportance::Standard,
            service_dependencies: vec!["test_service".to_string()],
            feature_dependencies: vec![],
            fallback_config: None,
            offline_capable: true,
        };
        
        let result = manager.register_feature(feature);
        assert!(result.is_ok());
        
        let status = manager.get_feature_status("test_feature");
        assert!(status.is_some());
        assert_eq!(status.unwrap().feature.id, "test_feature");
    }
    
    #[tokio::test]
    async fn test_feature_availability_check() {
        let service_registry = Arc::new(ServiceRegistry::new(ServiceMonitoringConfig::default()));
        let manager = FeatureAvailabilityManager::with_default_config(service_registry);
        
        let feature = FeatureDefinition {
            id: "test_feature".to_string(),
            name: "Test Feature".to_string(),
            description: "A test feature".to_string(),
            importance: FeatureImportance::Standard,
            service_dependencies: vec![], // No dependencies
            feature_dependencies: vec![],
            fallback_config: None,
            offline_capable: false,
        };
        
        manager.register_feature(feature).unwrap();
        
        let availability = manager.check_feature_availability("test_feature").await;
        assert!(availability.is_ok());
        // Should be available since there are no dependencies
        assert_eq!(availability.unwrap(), FeatureAvailability::Available);
    }
    
    #[test]
    fn test_feature_importance_ordering() {
        assert!(FeatureImportance::Critical < FeatureImportance::Important);
        assert!(FeatureImportance::Important < FeatureImportance::Standard);
        assert!(FeatureImportance::Standard < FeatureImportance::Optional);
    }
    
    #[test]
    fn test_feature_availability_display() {
        assert_eq!(format!("{}", FeatureAvailability::Available), "Available");
        assert_eq!(format!("{}", FeatureAvailability::Degraded), "Degraded");
        assert_eq!(format!("{}", FeatureAvailability::Unavailable), "Unavailable");
        assert_eq!(format!("{}", FeatureAvailability::Disabled), "Disabled");
    }
    
    #[test]
    fn test_graceful_degradation_coordinator_creation() {
        let service_registry = Arc::new(ServiceRegistry::default());
        let feature_manager = Arc::new(FeatureAvailabilityManager::with_default_config(service_registry.clone()));
        let cache_manager = Arc::new(OfflineCacheManager::new(OfflineCacheConfig::default()));
        let failover_executor = Arc::new(crate::error_handling::service_provider::FailoverExecutor::new(service_registry.clone()));
        
        let coordinator = GracefulDegradationCoordinator::with_default_config(
            feature_manager,
            service_registry,
            cache_manager,
            failover_executor,
        );
        
        // Just verify it can be created
        assert!(coordinator.config.auto_degradation);
        assert!(coordinator.config.auto_recovery);
    }
}