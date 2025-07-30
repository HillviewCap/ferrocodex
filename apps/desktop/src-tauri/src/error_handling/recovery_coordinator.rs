use crate::error_handling::{
    EnhancedError, 
    RetryExecutor, 
    RetryStrategy, 
    RetryResult,
    CircuitBreaker, 
    CircuitBreakerConfig,
    CircuitBreakerRegistry,
    ManualRecoveryGuide,
    ManualRecoveryAction,
    ManualRecoveryResult,
    RecoveryStrategy
};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

/// Recovery mode indicating which recovery mechanisms are active
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryMode {
    /// Only automatic recovery (retry + circuit breaker)
    Automatic,
    /// Automatic recovery with manual fallback
    AutoWithManualFallback,
    /// Manual recovery only
    ManualOnly,
    /// Recovery disabled
    Disabled,
}

/// Recovery attempt outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryOutcome {
    /// Operation succeeded through automatic recovery
    AutomaticSuccess {
        attempts: u32,
        total_duration: Duration,
    },
    /// Operation succeeded through manual intervention
    ManualSuccess {
        action_id: Uuid,
        duration: Duration,
    },
    /// All recovery attempts failed
    Failed {
        auto_attempts: u32,
        manual_attempts: u32,
        final_error: EnhancedError,
    },
    /// Recovery was cancelled by user
    Cancelled {
        at_stage: String,
    },
}

/// Comprehensive recovery coordinator that manages automatic and manual recovery
pub struct RecoveryCoordinator {
    /// Circuit breaker registry for managing service health
    circuit_breakers: CircuitBreakerRegistry,
    /// Manual recovery guide for generating user guidance
    manual_guide: Arc<Mutex<ManualRecoveryGuide>>,
    /// Default recovery mode
    default_mode: RecoveryMode,
    /// Performance tracking
    recovery_times: Vec<u128>, // microseconds
}

impl RecoveryCoordinator {
    /// Create a new recovery coordinator
    pub fn new(default_mode: RecoveryMode) -> Self {
        Self {
            circuit_breakers: CircuitBreakerRegistry::new(),
            manual_guide: Arc::new(Mutex::new(ManualRecoveryGuide::new())),
            default_mode,
            recovery_times: Vec::new(),
        }
    }

    /// Execute an operation with comprehensive recovery support
    pub async fn execute_with_recovery<T, F, Fut>(
        &mut self,
        operation_name: &str,
        service_name: &str,
        operation: F,
        retry_strategy: Option<RetryStrategy>,
        circuit_config: Option<CircuitBreakerConfig>,
        recovery_mode: Option<RecoveryMode>,
    ) -> RecoveryResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, EnhancedError>> + Send,
        T: Clone + Send,
    {
        let start_time = Instant::now();
        let recovery_mode = recovery_mode.unwrap_or(self.default_mode);
        let execution_id = Uuid::new_v4();

        match recovery_mode {
            RecoveryMode::Disabled => {
                let result = operation().await;
                match result {
                    Ok(value) => RecoveryResult {
                        outcome: RecoveryOutcome::AutomaticSuccess {
                            attempts: 1,
                            total_duration: start_time.elapsed(),
                        },
                        execution_id,
                        total_duration: start_time.elapsed(),
                        value: Some(value),
                    },
                    Err(error) => RecoveryResult {
                        outcome: RecoveryOutcome::Failed {
                            auto_attempts: 1,
                            manual_attempts: 0,
                            final_error: error,
                        },
                        execution_id,
                        total_duration: start_time.elapsed(),
                        value: None,
                    },
                }
            }
            RecoveryMode::ManualOnly => {
                // Execute once, then provide manual guidance if it fails
                let result = operation().await;
                match result {
                    Ok(value) => RecoveryResult {
                        outcome: RecoveryOutcome::AutomaticSuccess {
                            attempts: 1,
                            total_duration: start_time.elapsed(),
                        },
                        execution_id,
                        total_duration: start_time.elapsed(),
                        value: Some(value),
                    },
                    Err(error) => {
                        let manual_actions = self.generate_manual_recovery_actions(&error);
                        RecoveryResult {
                            outcome: RecoveryOutcome::Failed {
                                auto_attempts: 1,
                                manual_attempts: 0,
                                final_error: error,
                            },
                            execution_id,
                            total_duration: start_time.elapsed(),
                            value: None,
                        }
                    }
                }
            }
            RecoveryMode::Automatic | RecoveryMode::AutoWithManualFallback => {
                self.execute_automatic_recovery(
                    operation_name,
                    service_name,
                    operation,
                    retry_strategy,
                    circuit_config,
                    recovery_mode == RecoveryMode::AutoWithManualFallback,
                    execution_id,
                    start_time,
                ).await
            }
        }
    }

    /// Execute automatic recovery with optional manual fallback
    async fn execute_automatic_recovery<T, F, Fut>(
        &mut self,
        operation_name: &str,
        service_name: &str,
        operation: F,
        retry_strategy: Option<RetryStrategy>,
        circuit_config: Option<CircuitBreakerConfig>,
        allow_manual_fallback: bool,
        execution_id: Uuid,
        start_time: Instant,
    ) -> RecoveryResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, EnhancedError>> + Send,
        T: Clone + Send,
    {
        // Get or create circuit breaker for this service
        let circuit_breaker = self.circuit_breakers.get_or_create(service_name, circuit_config);

        // Create retry executor with provided strategy
        let strategy = retry_strategy.unwrap_or_default();
        let mut retry_executor = match RetryExecutor::new(strategy) {
            Ok(executor) => executor,
            Err(e) => {
                let error = EnhancedError::new(
                    crate::error_handling::ErrorSeverity::High,
                    crate::error_handling::ErrorDomain::System,
                    RecoveryStrategy::ManualRecoverable,
                    format!("Failed to create retry executor: {}", e),
                );
                return RecoveryResult {
                    outcome: RecoveryOutcome::Failed {
                        auto_attempts: 0,
                        manual_attempts: 0,
                        final_error: error,
                    },
                    execution_id,
                    total_duration: start_time.elapsed(),
                    value: None,
                };
            }
        };

        // Execute with circuit breaker and retry
        let circuit_wrapped_operation = || async {
            circuit_breaker.execute(|| operation()).await
        };

        let retry_result = retry_executor.execute(circuit_wrapped_operation).await;

        // Track performance
        let recovery_time = start_time.elapsed().as_micros();
        self.recovery_times.push(recovery_time);
        if self.recovery_times.len() > 1000 {
            self.recovery_times.remove(0);
        }

        match retry_result.result {
            Ok(value) => RecoveryResult {
                outcome: RecoveryOutcome::AutomaticSuccess {
                    attempts: retry_result.attempts.len() as u32 + 1,
                    total_duration: retry_result.total_duration,
                },
                execution_id,
                total_duration: start_time.elapsed(),
                value: Some(value),
            },
            Err(final_error) => {
                if allow_manual_fallback {
                    // Generate manual recovery actions as fallback
                    let manual_actions = self.generate_manual_recovery_actions(&final_error);
                    
                    RecoveryResult {
                        outcome: RecoveryOutcome::Failed {
                            auto_attempts: retry_result.attempts.len() as u32 + 1,
                            manual_attempts: 0,
                            final_error,
                        },
                        execution_id,
                        total_duration: start_time.elapsed(),
                        value: None,
                    }
                } else {
                    RecoveryResult {
                        outcome: RecoveryOutcome::Failed {
                            auto_attempts: retry_result.attempts.len() as u32 + 1,
                            manual_attempts: 0,
                            final_error,
                        },
                        execution_id,
                        total_duration: start_time.elapsed(),
                        value: None,
                    }
                }
            }
        }
    }

    /// Generate manual recovery actions for an error
    pub fn generate_manual_recovery_actions(&self, error: &EnhancedError) -> Vec<ManualRecoveryAction> {
        let mut guide = self.manual_guide.lock().unwrap();
        guide.generate_recovery_actions(error)
    }

    /// Record the result of a manual recovery attempt
    pub fn record_manual_recovery_result(&self, result: ManualRecoveryResult) {
        let mut guide = self.manual_guide.lock().unwrap();
        guide.record_recovery_result(result);
    }

    /// Get circuit breaker metrics for a service
    pub fn get_circuit_breaker_metrics(&self, service_name: &str) -> Option<crate::error_handling::CircuitBreakerMetrics> {
        let breakers = self.circuit_breakers.get_all();
        breakers.iter()
            .find(|cb| cb.service_name() == service_name)
            .map(|cb| cb.metrics())
    }

    /// Get all circuit breaker metrics
    pub fn get_all_circuit_breaker_metrics(&self) -> Vec<(String, crate::error_handling::CircuitBreakerMetrics)> {
        self.circuit_breakers.get_all_metrics()
    }

    /// Reset circuit breaker for a service
    pub fn reset_circuit_breaker(&self, service_name: &str) -> bool {
        let breakers = self.circuit_breakers.get_all();
        if let Some(breaker) = breakers.iter().find(|cb| cb.service_name() == service_name) {
            breaker.reset();
            true
        } else {
            false
        }
    }

    /// Reset all circuit breakers
    pub fn reset_all_circuit_breakers(&self) {
        self.circuit_breakers.reset_all();
    }

    /// Get performance statistics for the recovery coordinator
    pub fn get_performance_stats(&self) -> crate::error_handling::context::PerformanceStats {
        if self.recovery_times.is_empty() {
            return crate::error_handling::context::PerformanceStats {
                sample_count: 0,
                average_time_us: 0.0,
                min_time_us: 0,
                max_time_us: 0,
                meets_requirement: true,
            };
        }

        let min_time = *self.recovery_times.iter().min().unwrap();
        let max_time = *self.recovery_times.iter().max().unwrap();
        let sum: u128 = self.recovery_times.iter().sum();
        let avg_time = sum as f64 / self.recovery_times.len() as f64;

        crate::error_handling::context::PerformanceStats {
            sample_count: self.recovery_times.len(),
            average_time_us: avg_time,
            min_time_us: min_time,
            max_time_us: max_time,
            meets_requirement: avg_time < 50000.0, // 50ms requirement for overall recovery coordination
        }
    }

    /// Update default recovery mode
    pub fn set_default_recovery_mode(&mut self, mode: RecoveryMode) {
        self.default_mode = mode;
    }

    /// Get default recovery mode
    pub fn get_default_recovery_mode(&self) -> RecoveryMode {
        self.default_mode
    }
}

impl Default for RecoveryCoordinator {
    fn default() -> Self {
        Self::new(RecoveryMode::AutoWithManualFallback)
    }
}

/// Result of a coordinated recovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryResult<T> {
    /// The outcome of the recovery attempt
    pub outcome: RecoveryOutcome,
    /// Unique identifier for this recovery execution
    pub execution_id: Uuid,
    /// Total time taken for the recovery process
    pub total_duration: Duration,
    /// Successful result value if recovery succeeded
    pub value: Option<T>,
}

impl<T> RecoveryResult<T> {
    /// Check if the recovery was successful
    pub fn is_success(&self) -> bool {
        matches!(
            self.outcome,
            RecoveryOutcome::AutomaticSuccess { .. } | RecoveryOutcome::ManualSuccess { .. }
        )
    }

    /// Check if the recovery failed
    pub fn is_failure(&self) -> bool {
        matches!(self.outcome, RecoveryOutcome::Failed { .. })
    }

    /// Check if the recovery was cancelled
    pub fn is_cancelled(&self) -> bool {
        matches!(self.outcome, RecoveryOutcome::Cancelled { .. })
    }

    /// Get the final error if recovery failed
    pub fn get_error(&self) -> Option<&EnhancedError> {
        match &self.outcome {
            RecoveryOutcome::Failed { final_error, .. } => Some(final_error),
            _ => None,
        }
    }

    /// Get the number of automatic attempts made
    pub fn get_auto_attempts(&self) -> u32 {
        match &self.outcome {
            RecoveryOutcome::AutomaticSuccess { attempts, .. } => *attempts,
            RecoveryOutcome::Failed { auto_attempts, .. } => *auto_attempts,
            _ => 0,
        }
    }
}

/// Utility functions for common recovery patterns
pub mod recovery_patterns {
    use super::*;

    /// Execute a database operation with standard recovery
    pub async fn execute_database_operation<T, F, Fut>(
        coordinator: &mut RecoveryCoordinator,
        operation_name: &str,
        operation: F,
    ) -> RecoveryResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, EnhancedError>> + Send,
        T: Clone + Send,
    {
        let retry_strategy = RetryStrategy::conservative();
        let circuit_config = CircuitBreakerConfig::conservative();
        
        coordinator.execute_with_recovery(
            operation_name,
            "database",
            operation,
            Some(retry_strategy),
            Some(circuit_config),
            Some(RecoveryMode::AutoWithManualFallback),
        ).await
    }

    /// Execute a network operation with aggressive retry
    pub async fn execute_network_operation<T, F, Fut>(
        coordinator: &mut RecoveryCoordinator,
        operation_name: &str,
        operation: F,
    ) -> RecoveryResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, EnhancedError>> + Send,
        T: Clone + Send,
    {
        let retry_strategy = RetryStrategy::aggressive();
        let circuit_config = CircuitBreakerConfig::aggressive();
        
        coordinator.execute_with_recovery(
            operation_name,
            "network",
            operation,
            Some(retry_strategy),
            Some(circuit_config),
            Some(RecoveryMode::AutoWithManualFallback),
        ).await
    }

    /// Execute a critical operation with manual fallback only
    pub async fn execute_critical_operation<T, F, Fut>(
        coordinator: &mut RecoveryCoordinator,
        operation_name: &str,
        operation: F,
    ) -> RecoveryResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, EnhancedError>> + Send,
        T: Clone + Send,
    {
        coordinator.execute_with_recovery(
            operation_name,
            "critical",
            operation,
            None,
            None,
            Some(RecoveryMode::ManualOnly),
        ).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_handling::{ErrorSeverity, ErrorDomain};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_recovery_coordinator_success() {
        let mut coordinator = RecoveryCoordinator::new(RecoveryMode::Automatic);
        
        let result = coordinator.execute_with_recovery(
            "test_operation",
            "test_service",
            || async { Ok::<i32, EnhancedError>(42) },
            None,
            None,
            None,
        ).await;
        
        assert!(result.is_success());
        assert_eq!(result.value, Some(42));
        assert_eq!(result.get_auto_attempts(), 1);
    }

    #[tokio::test]
    async fn test_recovery_coordinator_automatic_recovery() {
        let mut coordinator = RecoveryCoordinator::new(RecoveryMode::Automatic);
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();
        
        let result = coordinator.execute_with_recovery(
            "test_operation",
            "test_service",
            move || {
                let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
                async move {
                    if count < 2 {
                        Err(EnhancedError::new(
                            ErrorSeverity::Low,
                            ErrorDomain::System,
                            RecoveryStrategy::AutoRecoverable,
                            "Transient error".to_string(),
                        ))
                    } else {
                        Ok(42)
                    }
                }
            },
            Some(RetryStrategy::default()),
            None,
            None,
        ).await;
        
        assert!(result.is_success());
        assert_eq!(result.value, Some(42));
        assert!(result.get_auto_attempts() > 1);
    }

    #[tokio::test]
    async fn test_recovery_coordinator_manual_fallback() {
        let mut coordinator = RecoveryCoordinator::new(RecoveryMode::AutoWithManualFallback);
        
        let result: RecoveryResult<i32> = coordinator.execute_with_recovery(
            "test_operation",
            "test_service",
            || async {
                Err(EnhancedError::new(
                    ErrorSeverity::High,
                    ErrorDomain::Auth,
                    RecoveryStrategy::UserRecoverable,
                    "Authentication failed".to_string(),
                ))
            },
            Some(RetryStrategy::new(1, 100, 1000, 2.0)), // Only 1 retry
            None,
            None,
        ).await;
        
        assert!(result.is_failure());
        assert!(result.get_auto_attempts() > 0);
        
        // Should be able to generate manual recovery actions
        let error = result.get_error().unwrap();
        let manual_actions = coordinator.generate_manual_recovery_actions(error);
        assert!(!manual_actions.is_empty());
    }

    #[tokio::test]
    async fn test_recovery_coordinator_disabled_mode() {
        let mut coordinator = RecoveryCoordinator::new(RecoveryMode::Disabled);
        
        let result: RecoveryResult<i32> = coordinator.execute_with_recovery(
            "test_operation",
            "test_service",
            || async {
                Err(EnhancedError::new(
                    ErrorSeverity::Medium,
                    ErrorDomain::System,
                    RecoveryStrategy::AutoRecoverable,
                    "Some error".to_string(),
                ))
            },
            None,
            None,
            None,
        ).await;
        
        assert!(result.is_failure());
        assert_eq!(result.get_auto_attempts(), 1); // Only one attempt when disabled
    }

    #[tokio::test]
    async fn test_circuit_breaker_integration() {
        let mut coordinator = RecoveryCoordinator::new(RecoveryMode::Automatic);
        
        // Cause multiple failures to open circuit breaker
        for _ in 0..5 {
            let _: RecoveryResult<i32> = coordinator.execute_with_recovery(
                "failing_operation",
                "failing_service",
                || async {
                    Err(EnhancedError::new(
                        ErrorSeverity::High,
                        ErrorDomain::System,
                        RecoveryStrategy::AutoRecoverable,
                        "Service error".to_string(),
                    ))
                },
                Some(RetryStrategy::new(1, 10, 100, 2.0)),
                Some(CircuitBreakerConfig::new(2, 1, 100)),
                None,
            ).await;
        }
        
        // Check circuit breaker metrics
        let metrics = coordinator.get_circuit_breaker_metrics("failing_service");
        assert!(metrics.is_some());
        
        let all_metrics = coordinator.get_all_circuit_breaker_metrics();
        assert!(!all_metrics.is_empty());
    }

    #[tokio::test]
    async fn test_manual_recovery_result_recording() {
        let coordinator = RecoveryCoordinator::new(RecoveryMode::ManualOnly);
        
        let result = ManualRecoveryResult {
            action_id: Uuid::new_v4(),
            success: true,
            user_feedback: Some("This worked".to_string()),
            duration: Duration::from_secs(30),
            resolved_error: true,
            notes: None,
        };
        
        // Should not panic
        coordinator.record_manual_recovery_result(result);
    }

    #[tokio::test]
    async fn test_recovery_patterns() {
        let mut coordinator = RecoveryCoordinator::default();
        
        // Test database pattern
        let db_result = recovery_patterns::execute_database_operation(
            &mut coordinator,
            "test_db_op",
            || async { Ok::<String, EnhancedError>("success".to_string()) },
        ).await;
        
        assert!(db_result.is_success());
        
        // Test network pattern
        let net_result = recovery_patterns::execute_network_operation(
            &mut coordinator,
            "test_net_op",
            || async { Ok::<String, EnhancedError>("success".to_string()) },
        ).await;
        
        assert!(net_result.is_success());
    }

    #[test]
    fn test_performance_tracking() {
        let mut coordinator = RecoveryCoordinator::default();
        
        // Simulate some recovery times
        coordinator.recovery_times.push(1000); // 1ms
        coordinator.recovery_times.push(2000); // 2ms
        coordinator.recovery_times.push(3000); // 3ms
        
        let stats = coordinator.get_performance_stats();
        assert_eq!(stats.sample_count, 3);
        assert_eq!(stats.min_time_us, 1000);
        assert_eq!(stats.max_time_us, 3000);
        assert!((stats.average_time_us - 2000.0).abs() < 0.1);
        assert!(stats.meets_requirement); // All under 50ms
    }
}