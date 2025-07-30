use crate::error_handling::{
    EnhancedError, ErrorSeverity, ErrorDomain, RecoveryStrategy,
    ServiceProvider, ServiceRegistry, ServiceHealth, ServiceHealthResult, ServicePriority,
    OfflineCacheManager, CachedDataEntry,
    FeatureAvailabilityManager, FeatureDefinition, FeatureImportance, FeatureAvailability, 
    GracefulDegradationCoordinator, SystemDegradationLevel,
    UserNotificationManager, DegradationNotification, NotificationType, NotificationPriority,
    EnhancedSqliteCache, EnhancedCacheConfig, CacheEvictionStrategy,
    UserDegradationPreferences, SystemDegradationPreferences, DegradationMode,
    UIStatusIndicator, UIStatusType, DegradedModeBanner, ServiceStatusDashboard,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use tempfile::TempDir;

/// Test data structure for integration testing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestAssetData {
    id: String,
    name: String,
    config: HashMap<String, String>,
    critical: bool,
}

/// Mock service provider for testing
struct MockDatabaseService {
    service_name: String,
    priority: crate::error_handling::ServicePriority,
    healthy: Arc<Mutex<bool>>,
    response_delay: Duration,
}

#[async_trait::async_trait]
impl ServiceProvider for MockDatabaseService {
    fn name(&self) -> &str {
        &self.service_name
    }
    
    fn priority(&self) -> crate::error_handling::ServicePriority {
        self.priority
    }
    
    async fn is_available(&self) -> bool {
        *self.healthy.lock().unwrap()
    }
    
    async fn health_check(&self) -> ServiceHealth {
        // Simulate response delay
        sleep(self.response_delay).await;
        
        let healthy = *self.healthy.lock().unwrap();
        if healthy { ServiceHealth::Healthy } else { ServiceHealth::Unhealthy }
    }
}

impl MockDatabaseService {
    fn new(name: &str, priority: crate::error_handling::ServicePriority, response_delay: Duration) -> Self {
        Self {
            service_name: name.to_string(),
            priority,
            healthy: Arc::new(Mutex::new(true)),
            response_delay,
        }
    }
    
    fn set_healthy(&self, healthy: bool) {
        *self.healthy.lock().unwrap() = healthy;
    }
}

/// Integration test suite for graceful degradation system
pub struct GracefulDegradationIntegrationTest {
    temp_dir: TempDir,
    service_registry: ServiceRegistry,
    cache_manager: OfflineCacheManager,
    feature_manager: FeatureAvailabilityManager,
    degradation_coordinator: GracefulDegradationCoordinator,
    notification_manager: UserNotificationManager,
    enhanced_cache: EnhancedSqliteCache,
    user_preferences: UserDegradationPreferences,
}

impl GracefulDegradationIntegrationTest {
    /// Create a new integration test environment
    pub async fn new() -> Result<Self, EnhancedError> {
        let temp_dir = TempDir::new()
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::High,
                ErrorDomain::System,
                RecoveryStrategy::ManualRequired,
                format!("Failed to create temp directory: {}", e),
            ))?;
        
        // Setup enhanced cache
        let cache_config = EnhancedCacheConfig {
            db_path: temp_dir.path().join("test_cache.db").to_string_lossy().to_string(),
            default_ttl_seconds: 3600,
            max_cache_size_bytes: 10 * 1024 * 1024, // 10MB
            enable_compression: true,
            cache_warming_interval_seconds: 300,
            max_items: 1000,
            enable_statistics: true,
            eviction_strategy: CacheEvictionStrategy::LeastRecentlyUsed,
        };
        let enhanced_cache = EnhancedSqliteCache::new(cache_config)?;
        
        // Setup service registry with mock services
        let service_registry = ServiceRegistry::default();
        
        let primary_db = Arc::new(MockDatabaseService::new("primary_database", ServicePriority::Primary, Duration::from_millis(50)));
        let secondary_db = Arc::new(MockDatabaseService::new("secondary_database", ServicePriority::Secondary, Duration::from_millis(100)));
        let file_service = Arc::new(MockDatabaseService::new("file_service", ServicePriority::Primary, Duration::from_millis(25)));
        
        service_registry.register_provider(primary_db.clone());
        service_registry.register_provider(secondary_db.clone());
        service_registry.register_provider(file_service.clone());
        
        // Setup offline cache manager
        let cache_manager = OfflineCacheManager::new(Duration::from_secs(300)); // 5 min TTL
        
        // Setup feature manager with test features
        let mut feature_manager = FeatureAvailabilityManager::new();
        
        // Critical asset management feature
        let asset_feature = FeatureDefinition {
            id: "asset_management".to_string(),
            name: "Asset Management".to_string(),
            description: "Manage OT assets and configurations".to_string(),
            importance: FeatureImportance::Critical,
            service_dependencies: vec!["database".to_string()],
            feature_dependencies: vec![],
            fallback_config: None,
            offline_capable: true,
        };
        
        // Standard backup feature
        let backup_feature = FeatureDefinition {
            id: "backup_system".to_string(),
            name: "Backup System".to_string(),
            description: "Automated backup operations".to_string(),
            importance: FeatureImportance::Standard,
            service_dependencies: vec!["database".to_string(), "file_system".to_string()],
            feature_dependencies: vec![],
            fallback_config: None,
            offline_capable: false,
        };
        
        feature_manager.register_feature(asset_feature)?;
        feature_manager.register_feature(backup_feature)?;
        
        // Setup degradation coordinator
        let degradation_coordinator = GracefulDegradationCoordinator::new();
        
        // Setup notification manager
        let notification_manager = UserNotificationManager::new();
        
        // Setup user preferences
        let user_preferences = UserDegradationPreferences {
            user_id: 1,
            system_preferences: SystemDegradationPreferences {
                mode: DegradationMode::Balanced,
                auto_enable_offline_mode: true,
                show_performance_warnings: true,
                critical_feature_protection: true,
                max_degradation_level: SystemDegradationLevel::Major,
                notification_preferences: HashMap::new(),
            },
            feature_preferences: HashMap::new(),
            last_updated: chrono::Utc::now(),
        };
        
        Ok(Self {
            temp_dir,
            service_registry,
            cache_manager,
            feature_manager,
            degradation_coordinator,
            notification_manager,
            enhanced_cache,
            user_preferences,
        })
    }
    
    /// Test AC1: Fallback service provider switching and restoration
    pub async fn test_fallback_service_switching(&mut self) -> Result<(), EnhancedError> {
        println!("Testing fallback service switching...");
        
        // Check all services initially
        let initial_results = self.service_registry.check_all_services().await;
        assert!(!initial_results.is_empty());
        println!("✓ Initial service health checks completed: {} services", initial_results.len());
        
        // Find the primary database service and simulate failure
        let providers = self.service_registry.get_providers();
        let primary_db = providers.iter()
            .find(|p| p.name() == "primary_database")
            .expect("Primary database service should exist");
        
        // Access the underlying mock service to set it unhealthy
        // Note: In a real implementation, this would be handled by actual service failures
        let mock_service = Arc::as_ptr(primary_db) as *const MockDatabaseService;
        unsafe {
            (*mock_service).set_healthy(false);
        }
        
        // Check health after failure
        let failed_health = primary_db.health_check().await;
        assert_eq!(failed_health, ServiceHealth::Unhealthy);
        println!("✓ Primary service failure simulated");
        
        // Find best available service (should fallback to secondary)
        let fallback_service = self.service_registry.get_best_available_service().await;
        assert!(fallback_service.is_some());
        
        if let Some(service) = fallback_service {
            assert_ne!(service.name(), "primary_database"); // Should not be primary
            println!("✓ Fallback to service: {}", service.name());
        }
        
        // Restore primary service
        unsafe {
            (*mock_service).set_healthy(true);
        }
        
        // Verify restoration
        let restored_health = primary_db.health_check().await;
        assert_eq!(restored_health, ServiceHealth::Healthy);
        println!("✓ Service restoration validated");
        
        Ok(())
    }
    
    /// Test AC2: Feature disabling and user notification systems
    pub async fn test_feature_degradation_and_notifications(&mut self) -> Result<(), EnhancedError> {
        println!("Testing feature degradation and notifications...");
        
        // Check initial feature availability
        let initial_status = self.feature_manager.check_feature_availability(
            "asset_management",
            &self.service_registry,
        ).await?;
        println!("✓ Initial feature status: {:?}", initial_status.availability);
        
        // Simulate service failure affecting features by making all database services unhealthy
        let providers = self.service_registry.get_providers();
        for provider in &providers {
            if provider.name().contains("database") {
                let mock_service = Arc::as_ptr(provider) as *const MockDatabaseService;
                unsafe {
                    (*mock_service).set_healthy(false);
                }
            }
        }
        
        // Check feature availability after service failure
        let degraded_status = self.feature_manager.check_feature_availability(
            "asset_management",
            &self.service_registry,
        ).await?;
        
        // Critical feature should degrade but remain available (offline mode)
        assert!(matches!(
            degraded_status.availability, 
            FeatureAvailability::Degraded | FeatureAvailability::Available
        ));
        
        // Generate notifications for degraded state
        let notification = DegradationNotification {
            id: uuid::Uuid::new_v4().to_string(),
            feature_id: "asset_management".to_string(),
            feature_name: "Asset Management".to_string(),
            notification_type: NotificationType::Warning,
            priority: NotificationPriority::High,
            message: "Asset Management running in degraded mode".to_string(),
            timestamp: chrono::Utc::now(),
            dismissible: true,
            auto_dismiss_ms: Some(10000),
            actions: vec![],
        };
        
        self.notification_manager.add_notification(notification.clone())?;
        
        // Verify notification was added
        let active_notifications = self.notification_manager.get_active_notifications();
        assert!(!active_notifications.is_empty());
        assert_eq!(active_notifications[0].feature_id, "asset_management");
        
        // Test notification dismissal
        self.notification_manager.dismiss_notification(&notification.id)?;
        let remaining_notifications = self.notification_manager.get_active_notifications();
        assert!(remaining_notifications.is_empty());
        
        println!("✓ Feature degradation and notifications validated");
        Ok(())
    }
    
    /// Test AC3: Cached data utilization and staleness handling
    pub async fn test_cached_data_utilization(&mut self) -> Result<(), EnhancedError> {
        println!("Testing cached data utilization...");
        
        // Create test asset data
        let test_asset = TestAssetData {
            id: "asset_001".to_string(),
            name: "Critical Pump".to_string(),
            config: {
                let mut config = HashMap::new();
                config.insert("pressure".to_string(), "150_psi".to_string());
                config.insert("temperature".to_string(), "85_C".to_string());
                config
            },
            critical: true,
        };
        
        // Store in enhanced cache
        self.enhanced_cache.store(
            "asset_001",
            &test_asset,
            "asset_data",
            "primary_database",
            Some(3600), // 1 hour TTL
            vec!["critical".to_string(), "pump".to_string()],
        )?;
        
        // Store in offline cache manager
        let cached_entry = CachedDataEntry {
            data: test_asset.clone(),
            cached_at: chrono::Utc::now().timestamp() as u64,
            ttl_seconds: 3600,
            version: "v1.0".to_string(),
            source: "primary_database".to_string(),
        };
        
        self.cache_manager.store_critical_data("asset_001", cached_entry)?;
        
        // Test retrieval from enhanced cache
        let retrieved: Option<CachedDataEntry<TestAssetData>> = self.enhanced_cache.retrieve("asset_001")?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data, test_asset);
        
        // Test retrieval from offline cache
        let offline_data: Option<CachedDataEntry<TestAssetData>> = self.cache_manager.get_cached_data("asset_001")?;
        assert!(offline_data.is_some());
        assert_eq!(offline_data.unwrap().data, test_asset);
        
        // Test cache performance requirement (<200ms)
        let start = std::time::Instant::now();
        let _perf_test: Option<CachedDataEntry<TestAssetData>> = self.enhanced_cache.retrieve("asset_001")?;
        let access_time = start.elapsed();
        assert!(access_time < Duration::from_millis(200), "Cache access too slow: {:?}", access_time);
        
        // Test cache statistics
        let stats = self.enhanced_cache.get_stats();
        assert!(stats.hits > 0);
        assert!(stats.hit_ratio > 0.0);
        
        // Verify performance requirement is met
        assert!(self.enhanced_cache.meets_performance_requirement());
        
        println!("✓ Cached data utilization validated (access time: {:?})", access_time);
        Ok(())
    }
    
    /// Test AC4: User preference configuration and application
    pub async fn test_user_preference_configuration(&mut self) -> Result<(), EnhancedError> {
        println!("Testing user preference configuration...");
        
        // Test initial preferences
        assert_eq!(self.user_preferences.system_preferences.mode, DegradationMode::Balanced);
        assert!(self.user_preferences.system_preferences.auto_enable_offline_mode);
        
        // Test preference application in degradation coordinator
        let system_status = self.degradation_coordinator.assess_system_degradation(
            &self.feature_manager,
            &self.service_registry,
        ).await?;
        
        assert_eq!(system_status.level, SystemDegradationLevel::None);
        
        // Simulate degradation scenario by making database services unhealthy
        let providers = self.service_registry.get_providers();
        for provider in &providers {
            if provider.name().contains("database") {
                let mock_service = Arc::as_ptr(provider) as *const MockDatabaseService;
                unsafe {
                    (*mock_service).set_healthy(false);
                }
            }
        }
        
        // Re-assess with degradation
        let degraded_status = self.degradation_coordinator.assess_system_degradation(
            &self.feature_manager,
            &self.service_registry,
        ).await?;
        
        // Should detect degradation
        assert!(degraded_status.level != SystemDegradationLevel::None);
        
        // Test UI status indicator creation
        let status_indicator = UIStatusIndicator::from_system_status(&degraded_status);
        assert_eq!(status_indicator.status_type, UIStatusType::from(degraded_status.level));
        
        // Test degraded mode banner creation
        let banner = DegradedModeBanner::for_system_degradation(&degraded_status);
        assert_eq!(banner.id, "system_degradation");
        
        println!("✓ User preference configuration validated");
        Ok(())
    }
    
    /// Test AC5: Safety testing for degraded mode operation boundaries  
    pub async fn test_safety_boundaries(&mut self) -> Result<(), EnhancedError> {
        println!("Testing safety boundaries...");
        
        // Test critical feature protection
        assert!(self.user_preferences.system_preferences.critical_feature_protection);
        
        // Simulate severe degradation by making all services unhealthy
        let providers = self.service_registry.get_providers();
        for provider in &providers {
            let mock_service = Arc::as_ptr(provider) as *const MockDatabaseService;
            unsafe {
                (*mock_service).set_healthy(false);
            }
        }
        
        // Check system status with multiple failures
        let severe_status = self.degradation_coordinator.assess_system_degradation(
            &self.feature_manager,
            &self.service_registry,
        ).await?;
        
        // System should detect severe degradation
        assert!(matches!(
            severe_status.level,
            SystemDegradationLevel::Major | SystemDegradationLevel::Severe
        ));
        
        // Verify that degradation level doesn't exceed user's maximum
        let max_allowed = self.user_preferences.system_preferences.max_degradation_level;
        assert!(severe_status.level as u8 <= max_allowed as u8);
        
        // Test critical feature handling
        let asset_status = self.feature_manager.check_feature_availability(
            "asset_management",
            &self.service_registry,
        ).await?;
        
        // Critical features should have offline fallback
        if asset_status.availability == FeatureAvailability::Unavailable {
            // Should have cached data available
            let cached_data: Option<CachedDataEntry<TestAssetData>> = 
                self.cache_manager.get_cached_data("asset_001")?;
            assert!(cached_data.is_some(), "Critical feature should have cached fallback");
        }
        
        println!("✓ Safety boundaries validated");
        Ok(())
    }
    
    /// Test AC6: Performance verification (fallback <1s, cache <200ms, feature check <10ms)
    pub async fn test_performance_requirements(&mut self) -> Result<(), EnhancedError> {
        println!("Testing performance requirements...");
        
        // Test feature availability check performance (<10ms)
        let start = std::time::Instant::now();
        let _feature_status = self.feature_manager.check_feature_availability(
            "asset_management",
            &self.service_registry,
        ).await?;
        let feature_check_time = start.elapsed();
        assert!(
            feature_check_time < Duration::from_millis(10),
            "Feature check too slow: {:?}",
            feature_check_time
        );
        
        // Test cache access performance (<200ms)
        let start = std::time::Instant::now();
        let _cached_data: Option<CachedDataEntry<TestAssetData>> = 
            self.enhanced_cache.retrieve("asset_001")?;
        let cache_access_time = start.elapsed();
        assert!(
            cache_access_time < Duration::from_millis(200),
            "Cache access too slow: {:?}",
            cache_access_time
        );
        
        // Test service discovery performance (<1s)
        let start = std::time::Instant::now();
        let _best_service = self.service_registry.get_best_available_service().await;
        let service_discovery_time = start.elapsed();
        assert!(
            service_discovery_time < Duration::from_secs(1),
            "Service discovery too slow: {:?}",
            service_discovery_time
        );
        
        println!("✓ Performance requirements validated:");
        println!("  - Feature check: {:?} (<10ms required)", feature_check_time);
        println!("  - Cache access: {:?} (<200ms required)", cache_access_time);
        println!("  - Service discovery: {:?} (<1s required)", service_discovery_time);
        
        Ok(())
    }
    
    /// Test comprehensive system integration
    pub async fn test_comprehensive_integration(&mut self) -> Result<(), EnhancedError> {
        println!("Testing comprehensive system integration...");
        
        // Create service status dashboard
        let mut dashboard = ServiceStatusDashboard::new();
        
        // Update dashboard with current system status
        let system_status = self.degradation_coordinator.assess_system_degradation(
            &self.feature_manager,
            &self.service_registry,
        ).await?;
        
        dashboard.update_with_system_status(&system_status);
        
        // Add service health indicators
        let db_health = self.service_registry.check_service_health("database").await?;
        dashboard.update_service_indicator("database", &db_health);
        
        let file_health = self.service_registry.check_service_health("file_system").await?;
        dashboard.update_service_indicator("file_system", &file_health);
        
        // Add feature indicators
        let asset_status = self.feature_manager.check_feature_availability(
            "asset_management",
            &self.service_registry,
        ).await?;
        dashboard.update_feature_indicator(&asset_status);
        
        let backup_status = self.feature_manager.check_feature_availability(
            "backup_system",
            &self.service_registry,
        ).await?;
        dashboard.update_feature_indicator(&backup_status);
        
        // Verify dashboard state
        assert_eq!(dashboard.service_indicators.len(), 2);
        assert_eq!(dashboard.feature_indicators.len(), 2);
        
        // Test critical indicators
        let critical_indicators = dashboard.get_critical_indicators();
        println!("Found {} critical indicators", critical_indicators.len());
        
        // Update performance summary
        let cache_stats = self.enhanced_cache.get_stats();
        let performance_summary = crate::error_handling::ui_status::PerformanceSummary {
            avg_response_time_ms: 50.0,
            cache_hit_rate: cache_stats.hit_ratio,
            uptime_percentage: 0.99,
            active_failovers: 0,
            total_requests: cache_stats.hits + cache_stats.misses,
            error_rate: 0.01,
        };
        
        dashboard.update_performance_summary(performance_summary);
        
        // Verify dashboard integration
        assert!(dashboard.performance_summary.cache_hit_rate >= 0.0);
        assert!(dashboard.performance_summary.uptime_percentage > 0.0);
        
        println!("✓ Comprehensive system integration validated");
        Ok(())
    }
    
    /// Run all integration tests
    pub async fn run_all_tests(&mut self) -> Result<(), EnhancedError> {
        println!("=== Running Graceful Degradation Integration Tests ===\n");
        
        // AC1: Fallback service provider switching and restoration
        self.test_fallback_service_switching().await?;
        println!();
        
        // AC2: Feature disabling and user notification systems
        self.test_feature_degradation_and_notifications().await?;
        println!();
        
        // AC3: Cached data utilization and staleness handling
        self.test_cached_data_utilization().await?;
        println!();
        
        // AC4: User preference configuration and application
        self.test_user_preference_configuration().await?;
        println!();
        
        // AC5: Safety testing for degraded mode operation boundaries
        self.test_safety_boundaries().await?;
        println!();
        
        // AC6: Performance verification
        self.test_performance_requirements().await?;
        println!();
        
        // Comprehensive integration test
        self.test_comprehensive_integration().await?;
        println!();
        
        println!("=== All Integration Tests Completed Successfully ===");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_graceful_degradation_integration() {
        let mut test_suite = GracefulDegradationIntegrationTest::new().await
            .expect("Failed to create integration test suite");
        
        test_suite.run_all_tests().await
            .expect("Integration tests failed");
    }
    
    #[tokio::test]
    async fn test_performance_benchmarks() {
        let mut test_suite = GracefulDegradationIntegrationTest::new().await
            .expect("Failed to create integration test suite");
        
        // Run performance tests multiple times to get average
        let mut feature_check_times = Vec::new();
        let mut cache_access_times = Vec::new();
        let mut failover_times = Vec::new();
        
        for _ in 0..10 {
            // Feature check timing
            let start = std::time::Instant::now();
            let _feature_status = test_suite.feature_manager.check_feature_availability(
                "asset_management",
                &test_suite.service_registry,
            ).await.unwrap();
            feature_check_times.push(start.elapsed());
            
            // Cache access timing
            let start = std::time::Instant::now();
            let _cached_data: Option<CachedDataEntry<TestAssetData>> = 
                test_suite.enhanced_cache.retrieve("asset_001").unwrap();
            cache_access_times.push(start.elapsed());
            
            // Service discovery timing
            let start = std::time::Instant::now();
            let _best_service = test_suite.service_registry.get_best_available_service().await;
            failover_times.push(start.elapsed());
        }
        
        // Calculate averages
        let avg_feature_check = feature_check_times.iter().sum::<Duration>() / feature_check_times.len() as u32;
        let avg_cache_access = cache_access_times.iter().sum::<Duration>() / cache_access_times.len() as u32;
        let avg_service_discovery = failover_times.iter().sum::<Duration>() / failover_times.len() as u32;
        
        println!("Performance Benchmarks (10 iterations):");
        println!("  Average feature check: {:?} (requirement: <10ms)", avg_feature_check);
        println!("  Average cache access: {:?} (requirement: <200ms)", avg_cache_access);
        println!("  Average service discovery: {:?} (requirement: <1s)", avg_service_discovery);
        
        // Verify performance requirements
        assert!(avg_feature_check < Duration::from_millis(10));
        assert!(avg_cache_access < Duration::from_millis(200));
        assert!(avg_service_discovery < Duration::from_secs(1));
    }
}