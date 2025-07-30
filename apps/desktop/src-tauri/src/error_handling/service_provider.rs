use crate::error_handling::{EnhancedError, ErrorDomain, ErrorSeverity, RecoveryStrategy};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Service health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceHealth {
    /// Service is healthy and fully operational
    Healthy,
    /// Service is degraded but partially functional
    Degraded,
    /// Service is unhealthy and not functional
    Unhealthy,
    /// Service health is unknown
    Unknown,
}

impl std::fmt::Display for ServiceHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceHealth::Healthy => write!(f, "Healthy"),
            ServiceHealth::Degraded => write!(f, "Degraded"),
            ServiceHealth::Unhealthy => write!(f, "Unhealthy"),
            ServiceHealth::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Service priority levels for fallback ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ServicePriority {
    /// Primary service - highest priority
    Primary = 0,
    /// Secondary service - backup to primary
    Secondary = 1,
    /// Cached service - uses cached data
    Cached = 2,
    /// Offline service - minimal functionality
    Offline = 3,
}

impl std::fmt::Display for ServicePriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServicePriority::Primary => write!(f, "Primary"),
            ServicePriority::Secondary => write!(f, "Secondary"),
            ServicePriority::Cached => write!(f, "Cached"),
            ServicePriority::Offline => write!(f, "Offline"),
        }
    }
}

/// Service provider trait for implementing fallback services
#[async_trait::async_trait]
pub trait ServiceProvider: Send + Sync {
    /// Get the service name/identifier
    fn name(&self) -> &str;
    
    /// Get the service priority level
    fn priority(&self) -> ServicePriority;
    
    /// Check if the service is available
    async fn is_available(&self) -> bool;
    
    /// Get the current health status of the service
    async fn health_check(&self) -> ServiceHealth;
    
    /// Execute a health check with timeout
    async fn health_check_with_timeout(&self, timeout: Duration) -> ServiceHealth {
        match tokio::time::timeout(timeout, self.health_check()).await {
            Ok(health) => health,
            Err(_) => ServiceHealth::Unknown,
        }
    }
    
    /// Warm up the service (e.g., establish connections, load cache)
    async fn warm_up(&self) -> Result<(), EnhancedError> {
        Ok(())
    }
    
    /// Shut down the service gracefully
    async fn shutdown(&self) -> Result<(), EnhancedError> {
        Ok(())
    }
    
    /// Get service-specific configuration
    fn get_configuration(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

/// Service health check result with timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealthResult {
    /// Service name
    pub service_name: String,
    /// Service priority
    pub priority: ServicePriority,
    /// Current health status
    pub health: ServiceHealth,
    /// Timestamp of the health check
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Duration of the health check
    pub check_duration: Duration,
    /// Additional context or error information
    pub context: Option<String>,
}

/// Service monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMonitoringConfig {
    /// How often to perform health checks (milliseconds)
    pub health_check_interval_ms: u64,
    /// Health check timeout (milliseconds)
    pub health_check_timeout_ms: u64,
    /// Number of consecutive failures before marking unhealthy
    pub failure_threshold: u32,
    /// Number of consecutive successes before marking healthy
    pub recovery_threshold: u32,
    /// Enable/disable automatic failover
    pub auto_failover_enabled: bool,
    /// Enable/disable automatic recovery detection
    pub auto_recovery_enabled: bool,
}

impl Default for ServiceMonitoringConfig {
    fn default() -> Self {
        Self {
            health_check_interval_ms: 30000, // 30 seconds
            health_check_timeout_ms: 5000,   // 5 seconds
            failure_threshold: 3,
            recovery_threshold: 2,
            auto_failover_enabled: true,
            auto_recovery_enabled: true,
        }
    }
}

/// Service registry for managing fallback service providers
pub struct ServiceRegistry {
    /// Registered service providers by priority
    providers: Arc<Mutex<Vec<Arc<dyn ServiceProvider>>>>,
    /// Service health results
    health_results: Arc<Mutex<HashMap<String, ServiceHealthResult>>>,
    /// Monitoring configuration
    config: ServiceMonitoringConfig,
    /// Currently active service
    active_service: Arc<Mutex<Option<Arc<dyn ServiceProvider>>>>,
    /// Service monitoring task handle
    monitoring_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new(config: ServiceMonitoringConfig) -> Self {
        Self {
            providers: Arc::new(Mutex::new(Vec::new())),
            health_results: Arc::new(Mutex::new(HashMap::new())),
            config,
            active_service: Arc::new(Mutex::new(None)),
            monitoring_handle: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Create a service registry with default configuration
    pub fn default() -> Self {
        Self::new(ServiceMonitoringConfig::default())
    }
    
    /// Register a service provider
    pub fn register_provider(&self, provider: Arc<dyn ServiceProvider>) {
        let mut providers = self.providers.lock().unwrap();
        providers.push(provider);
        
        // Sort by priority (Primary = 0, Secondary = 1, etc.)
        providers.sort_by_key(|p| p.priority());
    }
    
    /// Get all registered providers sorted by priority
    pub fn get_providers(&self) -> Vec<Arc<dyn ServiceProvider>> {
        self.providers.lock().unwrap().clone()
    }
    
    /// Get the currently active service
    pub fn get_active_service(&self) -> Option<Arc<dyn ServiceProvider>> {
        self.active_service.lock().unwrap().clone()
    }
    
    /// Set the active service
    pub fn set_active_service(&self, service: Option<Arc<dyn ServiceProvider>>) {
        *self.active_service.lock().unwrap() = service;
    }
    
    /// Get the best available service based on health
    pub async fn get_best_available_service(&self) -> Option<Arc<dyn ServiceProvider>> {
        let providers = self.get_providers();
        
        for provider in providers {
            if provider.is_available().await {
                let health = provider.health_check_with_timeout(
                    Duration::from_millis(self.config.health_check_timeout_ms)
                ).await;
                
                if health == ServiceHealth::Healthy || health == ServiceHealth::Degraded {
                    return Some(provider);
                }
            }
        }
        
        None
    }
    
    /// Perform health check on all services
    pub async fn check_all_services(&self) -> Vec<ServiceHealthResult> {
        let providers = self.get_providers();
        let mut results = Vec::new();
        
        for provider in providers {
            let start_time = Instant::now();
            let health = provider.health_check_with_timeout(
                Duration::from_millis(self.config.health_check_timeout_ms)
            ).await;
            let check_duration = start_time.elapsed();
            
            let result = ServiceHealthResult {
                service_name: provider.name().to_string(),
                priority: provider.priority(),
                health,
                timestamp: chrono::Utc::now(),
                check_duration,
                context: None,
            };
            
            results.push(result.clone());
            
            // Update stored results
            self.health_results.lock().unwrap().insert(
                provider.name().to_string(),
                result,
            );
        }
        
        results
    }
    
    /// Get health results for all services
    pub fn get_health_results(&self) -> HashMap<String, ServiceHealthResult> {
        self.health_results.lock().unwrap().clone()
    }
    
    /// Start service monitoring
    pub async fn start_monitoring(&self) {
        let providers = self.providers.clone();
        let health_results = self.health_results.clone();
        let active_service = self.active_service.clone();
        let config = self.config.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_millis(config.health_check_interval_ms)
            );
            
            loop {
                interval.tick().await;
                
                // Check health of all services
                let providers_list = providers.lock().unwrap().clone();
                let mut service_healths = Vec::new();
                
                for provider in &providers_list {
                    let start_time = Instant::now();
                    let health = provider.health_check_with_timeout(
                        Duration::from_millis(config.health_check_timeout_ms)
                    ).await;
                    let check_duration = start_time.elapsed();
                    
                    let result = ServiceHealthResult {
                        service_name: provider.name().to_string(),
                        priority: provider.priority(),
                        health,
                        timestamp: chrono::Utc::now(),
                        check_duration,
                        context: None,
                    };
                    
                    service_healths.push((provider.clone(), result.clone()));
                    
                    // Update stored results
                    health_results.lock().unwrap().insert(
                        provider.name().to_string(),
                        result,
                    );
                }
                
                // Automatic failover if enabled
                if config.auto_failover_enabled {
                    let current_active = active_service.lock().unwrap().clone();
                    
                    // Check if current active service is still healthy
                    let current_healthy = if let Some(ref active) = current_active {
                        service_healths.iter()
                            .find(|(p, _)| p.name() == active.name())
                            .map(|(_, result)| result.health == ServiceHealth::Healthy || result.health == ServiceHealth::Degraded)
                            .unwrap_or(false)
                    } else {
                        false
                    };
                    
                    // If current service is unhealthy or no active service, find best alternative
                    if !current_healthy {
                        for (provider, result) in &service_healths {
                            if result.health == ServiceHealth::Healthy || result.health == ServiceHealth::Degraded {
                                *active_service.lock().unwrap() = Some(provider.clone());
                                break;
                            }
                        }
                    }
                    
                    // Automatic recovery - try to use higher priority service if available
                    if config.auto_recovery_enabled {
                        if let Some(ref current) = active_service.lock().unwrap().clone() {
                            // Look for a higher priority healthy service
                            for (provider, result) in &service_healths {
                                if (result.health == ServiceHealth::Healthy) &&
                                   (provider.priority() < current.priority()) {
                                    *active_service.lock().unwrap() = Some(provider.clone());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });
        
        *self.monitoring_handle.lock().unwrap() = Some(handle);
    }
    
    /// Stop service monitoring
    pub async fn stop_monitoring(&self) {
        if let Some(handle) = self.monitoring_handle.lock().unwrap().take() {
            handle.abort();
        }
    }
    
    /// Warm up all services
    pub async fn warm_up_all(&self) -> Vec<Result<(), EnhancedError>> {
        let providers = self.get_providers();
        let mut results = Vec::new();
        
        for provider in providers {
            let result = provider.warm_up().await;
            results.push(result);
        }
        
        results
    }
    
    /// Shutdown all services
    pub async fn shutdown_all(&self) -> Vec<Result<(), EnhancedError>> {
        // Stop monitoring first
        self.stop_monitoring().await;
        
        let providers = self.get_providers();
        let mut results = Vec::new();
        
        for provider in providers {
            let result = provider.shutdown().await;
            results.push(result);
        }
        
        results
    }
}

/// Failover executor that handles service provider switching
pub struct FailoverExecutor {
    /// Service registry
    registry: Arc<ServiceRegistry>,
    /// Execution statistics
    execution_stats: Arc<Mutex<FailoverStats>>,
}

/// Failover execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverStats {
    /// Total operations executed
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Number of failovers performed
    pub failover_count: u64,
    /// Average execution time (microseconds)
    pub average_execution_time_us: f64,
    /// Service usage statistics (service name -> usage count)
    pub service_usage: HashMap<String, u64>,
}

impl Default for FailoverStats {
    fn default() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            failover_count: 0,
            average_execution_time_us: 0.0,
            service_usage: HashMap::new(),
        }
    }
}

impl FailoverExecutor {
    /// Create a new failover executor
    pub fn new(registry: Arc<ServiceRegistry>) -> Self {
        Self {
            registry,
            execution_stats: Arc::new(Mutex::new(FailoverStats::default())),
        }
    }
    
    /// Execute an operation with automatic failover
    pub async fn execute<T, F, Fut>(&self, operation: F) -> Result<T, EnhancedError>
    where
        F: Fn(Arc<dyn ServiceProvider>) -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, EnhancedError>> + Send,
        T: Send,
    {
        let start_time = Instant::now();
        let operation_id = Uuid::new_v4();
        
        // Get all providers sorted by priority
        let providers = self.registry.get_providers();
        let mut last_error = None;
        let mut services_tried = Vec::new();
        
        // Try each service in priority order
        for provider in providers {
            // Check if service is available
            if !provider.is_available().await {
                continue;
            }
            
            // Check service health with timeout
            let health = provider.health_check_with_timeout(
                Duration::from_millis(1000) // 1 second timeout for health check
            ).await;
            
            // Skip unhealthy services unless it's the last resort
            if health == ServiceHealth::Unhealthy {
                continue;
            }
            
            services_tried.push(provider.name().to_string());
            
            // Update service usage statistics
            {
                let mut stats = self.execution_stats.lock().unwrap();
                *stats.service_usage.entry(provider.name().to_string()).or_insert(0) += 1;
            }
            
            // Attempt the operation
            match operation(provider.clone()).await {
                Ok(result) => {
                    // Success - update statistics and return
                    let execution_time = start_time.elapsed();
                    self.update_success_stats(execution_time, &services_tried);
                    
                    // Set as active service if not already
                    self.registry.set_active_service(Some(provider));
                    
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error);
                    
                    // Record failover if we're not on the first service
                    if services_tried.len() > 1 {
                        let mut stats = self.execution_stats.lock().unwrap();
                        stats.failover_count += 1;
                    }
                    
                    // Continue to next service
                    continue;
                }
            }
        }
        
        // All services failed
        let execution_time = start_time.elapsed();
        self.update_failure_stats(execution_time, &services_tried);
        
        // Return the last error with enhanced context
        let final_error = last_error.unwrap_or_else(|| {
            EnhancedError::new(
                ErrorSeverity::High,
                ErrorDomain::System,
                RecoveryStrategy::ManualRecoverable,
                "No service providers available".to_string(),
            )
        });
        
        // Add context about services tried
        let mut enhanced_error = final_error;
        enhanced_error.add_context("operation_id", operation_id.to_string());
        enhanced_error.add_context("services_tried", services_tried.join(", "));
        enhanced_error.add_context("execution_time_ms", execution_time.as_millis().to_string());
        
        Err(enhanced_error)
    }
    
    /// Update statistics for successful operations
    fn update_success_stats(&self, execution_time: Duration, services_tried: &[String]) {
        let mut stats = self.execution_stats.lock().unwrap();
        stats.total_operations += 1;
        stats.successful_operations += 1;
        
        // Update average execution time
        let new_time_us = execution_time.as_micros() as f64;
        if stats.total_operations == 1 {
            stats.average_execution_time_us = new_time_us;
        } else {
            stats.average_execution_time_us = 
                (stats.average_execution_time_us * (stats.total_operations - 1) as f64 + new_time_us) 
                / stats.total_operations as f64;
        }
    }
    
    /// Update statistics for failed operations
    fn update_failure_stats(&self, execution_time: Duration, services_tried: &[String]) {
        let mut stats = self.execution_stats.lock().unwrap();
        stats.total_operations += 1;
        stats.failed_operations += 1;
        
        // Update average execution time
        let new_time_us = execution_time.as_micros() as f64;
        if stats.total_operations == 1 {
            stats.average_execution_time_us = new_time_us;
        } else {
            stats.average_execution_time_us = 
                (stats.average_execution_time_us * (stats.total_operations - 1) as f64 + new_time_us) 
                / stats.total_operations as f64;
        }
    }
    
    /// Get current failover statistics
    pub fn get_stats(&self) -> FailoverStats {
        self.execution_stats.lock().unwrap().clone()
    }
    
    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.execution_stats.lock().unwrap() = FailoverStats::default();
    }
    
    /// Check if failover performance meets requirements (<1 second)
    pub fn meets_performance_requirement(&self) -> bool {
        let stats = self.execution_stats.lock().unwrap();
        stats.average_execution_time_us < 1_000_000.0 // 1 second in microseconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    
    /// Mock service provider for testing
    struct MockServiceProvider {
        name: String,
        priority: ServicePriority,
        is_available: AtomicBool,
        health: Arc<Mutex<ServiceHealth>>,
        call_count: AtomicU32,
    }
    
    impl MockServiceProvider {
        fn new(name: &str, priority: ServicePriority, is_available: bool, health: ServiceHealth) -> Self {
            Self {
                name: name.to_string(),
                priority,
                is_available: AtomicBool::new(is_available),
                health: Arc::new(Mutex::new(health)),
                call_count: AtomicU32::new(0),
            }
        }
        
        fn set_available(&self, available: bool) {
            self.is_available.store(available, Ordering::SeqCst);
        }
        
        fn set_health(&self, health: ServiceHealth) {
            *self.health.lock().unwrap() = health;
        }
        
        fn get_call_count(&self) -> u32 {
            self.call_count.load(Ordering::SeqCst)
        }
    }
    
    #[async_trait::async_trait]
    impl ServiceProvider for MockServiceProvider {
        fn name(&self) -> &str {
            &self.name
        }
        
        fn priority(&self) -> ServicePriority {
            self.priority
        }
        
        async fn is_available(&self) -> bool {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            self.is_available.load(Ordering::SeqCst)
        }
        
        async fn health_check(&self) -> ServiceHealth {
            *self.health.lock().unwrap()
        }
    }
    
    #[tokio::test]
    async fn test_service_registry_registration() {
        let registry = ServiceRegistry::default();
        
        let primary = Arc::new(MockServiceProvider::new(
            "primary", ServicePriority::Primary, true, ServiceHealth::Healthy
        ));
        let secondary = Arc::new(MockServiceProvider::new(
            "secondary", ServicePriority::Secondary, true, ServiceHealth::Healthy
        ));
        
        registry.register_provider(secondary.clone());
        registry.register_provider(primary.clone());
        
        let providers = registry.get_providers();
        assert_eq!(providers.len(), 2);
        // Should be sorted by priority (Primary first)
        assert_eq!(providers[0].name(), "primary");
        assert_eq!(providers[1].name(), "secondary");
    }
    
    #[tokio::test]
    async fn test_get_best_available_service() {
        let registry = ServiceRegistry::default();
        
        let primary = Arc::new(MockServiceProvider::new(
            "primary", ServicePriority::Primary, false, ServiceHealth::Unhealthy
        ));
        let secondary = Arc::new(MockServiceProvider::new(
            "secondary", ServicePriority::Secondary, true, ServiceHealth::Healthy
        ));
        
        registry.register_provider(primary);
        registry.register_provider(secondary);
        
        let best = registry.get_best_available_service().await;
        assert!(best.is_some());
        assert_eq!(best.unwrap().name(), "secondary");
    }
    
    #[tokio::test]
    async fn test_failover_executor_success() {
        let registry = Arc::new(ServiceRegistry::default());
        
        let primary = Arc::new(MockServiceProvider::new(
            "primary", ServicePriority::Primary, true, ServiceHealth::Healthy
        ));
        registry.register_provider(primary);
        
        let executor = FailoverExecutor::new(registry);
        
        let result = executor.execute(|_provider| async {
            Ok::<String, EnhancedError>("success".to_string())
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        
        let stats = executor.get_stats();
        assert_eq!(stats.total_operations, 1);
        assert_eq!(stats.successful_operations, 1);
        assert!(executor.meets_performance_requirement());
    }
    
    #[tokio::test]
    async fn test_failover_executor_fallback() {
        let registry = Arc::new(ServiceRegistry::default());
        
        let primary = Arc::new(MockServiceProvider::new(
            "primary", ServicePriority::Primary, true, ServiceHealth::Healthy
        ));
        let secondary = Arc::new(MockServiceProvider::new(
            "secondary", ServicePriority::Secondary, true, ServiceHealth::Healthy
        ));
        
        registry.register_provider(primary);
        registry.register_provider(secondary);
        
        let executor = FailoverExecutor::new(registry);
        
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();
        
        let result = executor.execute(move |provider| {
            let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if provider.name() == "primary" {
                    // Primary fails
                    Err(EnhancedError::new(
                        ErrorSeverity::Medium,
                        ErrorDomain::System,
                        RecoveryStrategy::AutoRecoverable,
                        "Primary service failure".to_string(),
                    ))
                } else {
                    // Secondary succeeds
                    Ok::<String, EnhancedError>("fallback_success".to_string())
                }
            }
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "fallback_success");
        
        let stats = executor.get_stats();
        assert_eq!(stats.total_operations, 1);
        assert_eq!(stats.successful_operations, 1);
        assert_eq!(stats.failover_count, 1);
        assert_eq!(call_count.load(Ordering::SeqCst), 2); // Called both services
    }
    
    #[tokio::test]
    async fn test_service_health_check() {
        let registry = ServiceRegistry::default();
        
        let service = Arc::new(MockServiceProvider::new(
            "test", ServicePriority::Primary, true, ServiceHealth::Degraded
        ));
        registry.register_provider(service);
        
        let results = registry.check_all_services().await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].service_name, "test");
        assert_eq!(results[0].health, ServiceHealth::Degraded);
        assert_eq!(results[0].priority, ServicePriority::Primary);
    }
}