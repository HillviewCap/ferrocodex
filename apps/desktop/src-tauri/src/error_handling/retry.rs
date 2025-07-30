use crate::error_handling::{EnhancedError, RecoveryStrategy};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

/// Retry strategy configuration for exponential backoff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStrategy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay before first retry (milliseconds)
    pub initial_delay_ms: u64,
    /// Maximum delay between retries (milliseconds)
    pub max_delay_ms: u64,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Jitter factor to prevent thundering herd (0.0 to 1.0)
    pub jitter_factor: f64,
    /// Maximum total retry duration (milliseconds)
    pub max_retry_duration_ms: Option<u64>,
    /// Enable/disable retry mechanism
    pub enabled: bool,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_retry_duration_ms: Some(30000), // 30 seconds
            enabled: true,
        }
    }
}

impl RetryStrategy {
    /// Create a new retry strategy with specified parameters
    pub fn new(
        max_attempts: u32,
        initial_delay_ms: u64,
        max_delay_ms: u64,
        backoff_multiplier: f64,
    ) -> Self {
        Self {
            max_attempts,
            initial_delay_ms,
            max_delay_ms,
            backoff_multiplier,
            jitter_factor: 0.1,
            max_retry_duration_ms: Some(30000),
            enabled: true,
        }
    }

    /// Create a conservative retry strategy for critical operations
    pub fn conservative() -> Self {
        Self {
            max_attempts: 2,
            initial_delay_ms: 200,
            max_delay_ms: 2000,
            backoff_multiplier: 1.5,
            jitter_factor: 0.05,
            max_retry_duration_ms: Some(10000), // 10 seconds
            enabled: true,
        }
    }

    /// Create an aggressive retry strategy for transient errors
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 50,
            max_delay_ms: 10000,
            backoff_multiplier: 2.5,
            jitter_factor: 0.2,
            max_retry_duration_ms: Some(60000), // 60 seconds
            enabled: true,
        }
    }

    /// Disable retry mechanism
    pub fn disabled() -> Self {
        Self {
            max_attempts: 0,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1.0,
            jitter_factor: 0.0,
            max_retry_duration_ms: None,
            enabled: false,
        }
    }

    /// Calculate delay for given attempt with exponential backoff and jitter
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 || !self.enabled {
            return Duration::from_millis(0);
        }

        let base_delay = self.initial_delay_ms as f64 
            * self.backoff_multiplier.powi((attempt - 1) as i32);
        
        let capped_delay = base_delay.min(self.max_delay_ms as f64);
        
        // Add jitter to prevent thundering herd
        let mut rng = thread_rng();
        let jitter = rng.gen_range(-self.jitter_factor..=self.jitter_factor);
        let jittered_delay = capped_delay * (1.0 + jitter);
        
        let final_delay = jittered_delay.max(0.0) as u64;
        Duration::from_millis(final_delay)
    }

    /// Validate strategy configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.max_attempts > 10 {
            return Err("Maximum attempts cannot exceed 10".to_string());
        }
        if self.initial_delay_ms > 10000 {
            return Err("Initial delay cannot exceed 10 seconds".to_string());
        }
        if self.max_delay_ms > 60000 {
            return Err("Maximum delay cannot exceed 60 seconds".to_string());
        }
        if self.backoff_multiplier < 1.0 || self.backoff_multiplier > 5.0 {
            return Err("Backoff multiplier must be between 1.0 and 5.0".to_string());
        }
        if self.jitter_factor < 0.0 || self.jitter_factor > 1.0 {
            return Err("Jitter factor must be between 0.0 and 1.0".to_string());
        }
        if let Some(max_duration) = self.max_retry_duration_ms {
            if max_duration > 300000 { // 5 minutes
                return Err("Maximum retry duration cannot exceed 5 minutes".to_string());
            }
        }
        Ok(())
    }
}

/// Information about a retry attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryAttempt {
    /// Attempt number (1-based)
    pub attempt: u32,
    /// Error that caused this retry
    pub error: EnhancedError,
    /// Timestamp of this attempt
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Delay before this attempt
    pub delay: Duration,
    /// Whether this was the final attempt
    pub is_final: bool,
}

/// Result of retry execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryResult<T> {
    /// Final result of the operation
    pub result: Result<T, EnhancedError>,
    /// All retry attempts made
    pub attempts: Vec<RetryAttempt>,
    /// Total execution time
    pub total_duration: Duration,
    /// Whether retry limit was exceeded
    pub retry_limit_exceeded: bool,
    /// Retry execution ID for tracking
    pub execution_id: Uuid,
}

/// Retry execution engine
pub struct RetryExecutor {
    /// Retry strategy configuration
    strategy: RetryStrategy,
    /// Performance tracking
    execution_times: Vec<u128>, // microseconds
}

impl RetryExecutor {
    /// Create a new retry executor with given strategy
    pub fn new(strategy: RetryStrategy) -> Result<Self, String> {
        strategy.validate()?;
        Ok(Self {
            strategy,
            execution_times: Vec::new(),
        })
    }

    /// Create a retry executor with default strategy
    pub fn default_strategy() -> Self {
        Self {
            strategy: RetryStrategy::default(),
            execution_times: Vec::new(),
        }
    }

    /// Execute an operation with retry logic
    pub async fn execute<T, F, Fut>(&mut self, operation: F) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, EnhancedError>>,
        T: Clone,
    {
        let start_time = Instant::now();
        let execution_id = Uuid::new_v4();
        let mut attempts = Vec::new();
        let mut last_error = None;

        // If retry is disabled, execute once
        if !self.strategy.enabled || self.strategy.max_attempts == 0 {
            let result = operation().await;
            return RetryResult {
                result,
                attempts: Vec::new(),
                total_duration: start_time.elapsed(),
                retry_limit_exceeded: false,
                execution_id,
            };
        }

        // Attempt operation with retries
        for attempt_num in 1..=self.strategy.max_attempts {
            // Check if we've exceeded maximum retry duration
            if let Some(max_duration) = self.strategy.max_retry_duration_ms {
                if start_time.elapsed().as_millis() > max_duration as u128 {
                    break;
                }
            }

            // Calculate delay for this attempt (no delay for first attempt)
            let delay = if attempt_num == 1 {
                Duration::from_millis(0)
            } else {
                self.strategy.calculate_delay(attempt_num - 1)
            };

            // Wait before retry (except for first attempt)
            if delay.as_millis() > 0 {
                sleep(delay).await;
            }

            // Execute the operation
            let operation_start = Instant::now();
            let result = operation().await;
            let operation_duration = operation_start.elapsed();

            // Track performance
            self.execution_times.push(operation_duration.as_micros());
            
            // Keep only last 1000 measurements
            if self.execution_times.len() > 1000 {
                self.execution_times.remove(0);
            }

            match result {
                Ok(value) => {
                    // Success - record attempt and return
                    if attempts.len() > 0 {
                        // Only record attempts if we actually retried
                        let attempt = RetryAttempt {
                            attempt: attempt_num,
                            error: last_error.unwrap_or_else(|| {
                                EnhancedError::new(
                                    crate::error_handling::ErrorSeverity::Low,
                                    crate::error_handling::ErrorDomain::System,
                                    RecoveryStrategy::AutoRecoverable,
                                    "Unknown error".to_string(),
                                )
                            }),
                            timestamp: chrono::Utc::now(),
                            delay,
                            is_final: false,
                        };
                        attempts.push(attempt);
                    }

                    return RetryResult {
                        result: Ok(value),
                        attempts,
                        total_duration: start_time.elapsed(),
                        retry_limit_exceeded: false,
                        execution_id,
                    };
                }
                Err(error) => {
                    let is_final_attempt = attempt_num >= self.strategy.max_attempts;
                    let should_retry = !is_final_attempt && self.should_retry(&error);

                    // Record this attempt
                    let attempt = RetryAttempt {
                        attempt: attempt_num,
                        error: error.clone(),
                        timestamp: chrono::Utc::now(),
                        delay,
                        is_final: is_final_attempt,
                    };
                    attempts.push(attempt);

                    last_error = Some(error.clone());

                    // If this is the final attempt or we shouldn't retry, return error
                    if is_final_attempt || !should_retry {
                        return RetryResult {
                            result: Err(error),
                            attempts,
                            total_duration: start_time.elapsed(),
                            retry_limit_exceeded: is_final_attempt && should_retry,
                            execution_id,
                        };
                    }
                }
            }
        }

        // This should never be reached, but handle it just in case
        RetryResult {
            result: Err(last_error.unwrap_or_else(|| {
                EnhancedError::new(
                    crate::error_handling::ErrorSeverity::High,
                    crate::error_handling::ErrorDomain::System,
                    RecoveryStrategy::ManualRecoverable,
                    "Retry execution failed unexpectedly".to_string(),
                )
            })),
            attempts,
            total_duration: start_time.elapsed(),
            retry_limit_exceeded: true,
            execution_id,
        }
    }

    /// Determine if an error should trigger a retry
    fn should_retry(&self, error: &EnhancedError) -> bool {
        // Only retry auto-recoverable errors
        if error.recovery_strategy != RecoveryStrategy::AutoRecoverable {
            return false;
        }

        // Check for transient error indicators
        let message_lower = error.message.to_lowercase();
        
        // Network and connectivity errors are typically transient
        if message_lower.contains("timeout") ||
           message_lower.contains("connection") ||
           message_lower.contains("network") ||
           message_lower.contains("temporary") ||
           message_lower.contains("transient") ||
           message_lower.contains("retry") ||
           message_lower.contains("service unavailable") ||
           message_lower.contains("too many requests") ||
           message_lower.contains("rate limit") {
            return true;
        }

        // Database connection issues are often transient
        if error.domain == crate::error_handling::ErrorDomain::Data &&
           (message_lower.contains("connection") || 
            message_lower.contains("timeout") ||
            message_lower.contains("busy") ||
            message_lower.contains("locked")) {
            return true;
        }

        // System resource issues might be transient
        if error.domain == crate::error_handling::ErrorDomain::System &&
           (message_lower.contains("resource") ||
            message_lower.contains("memory") ||
            message_lower.contains("disk") ||
            message_lower.contains("file lock")) {
            return true;
        }

        // Low severity errors in general might be worth retrying
        if error.severity == crate::error_handling::ErrorSeverity::Low {
            return true;
        }

        false
    }

    /// Get the current retry strategy
    pub fn strategy(&self) -> &RetryStrategy {
        &self.strategy
    }

    /// Update the retry strategy
    pub fn set_strategy(&mut self, strategy: RetryStrategy) -> Result<(), String> {
        strategy.validate()?;
        self.strategy = strategy;
        Ok(())
    }

    /// Get average execution time in microseconds
    pub fn get_average_execution_time(&self) -> Option<f64> {
        if self.execution_times.is_empty() {
            None
        } else {
            let sum: u128 = self.execution_times.iter().sum();
            Some(sum as f64 / self.execution_times.len() as f64)
        }
    }

    /// Check if performance requirement (<10ms retry decisions) is being met
    pub fn meets_performance_requirement(&self) -> bool {
        match self.get_average_execution_time() {
            Some(avg_time) => avg_time < 10000.0, // 10ms in microseconds
            None => true, // No measurements yet, assume OK
        }
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> crate::error_handling::context::PerformanceStats {
        if self.execution_times.is_empty() {
            return crate::error_handling::context::PerformanceStats {
                sample_count: 0,
                average_time_us: 0.0,
                min_time_us: 0,
                max_time_us: 0,
                meets_requirement: true,
            };
        }

        let min_time = *self.execution_times.iter().min().unwrap();
        let max_time = *self.execution_times.iter().max().unwrap();
        let avg_time = self.get_average_execution_time().unwrap();

        crate::error_handling::context::PerformanceStats {
            sample_count: self.execution_times.len(),
            average_time_us: avg_time,
            min_time_us: min_time,
            max_time_us: max_time,
            meets_requirement: avg_time < 10000.0, // 10ms requirement
        }
    }
}

/// Utility functions for common retry scenarios
pub mod retry_utils {
    use super::*;

    /// Execute database operation with retry
    pub async fn retry_database_operation<T, F, Fut>(
        operation: F,
    ) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, EnhancedError>>,
        T: Clone,
    {
        let strategy = RetryStrategy {
            max_attempts: 3,
            initial_delay_ms: 200,
            max_delay_ms: 2000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_retry_duration_ms: Some(10000),
            enabled: true,
        };

        let mut executor = RetryExecutor::new(strategy).unwrap();
        executor.execute(operation).await
    }

    /// Execute network operation with retry
    pub async fn retry_network_operation<T, F, Fut>(
        operation: F,
    ) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, EnhancedError>>,
        T: Clone,
    {
        let strategy = RetryStrategy::aggressive();
        let mut executor = RetryExecutor::new(strategy).unwrap();
        executor.execute(operation).await
    }

    /// Execute system operation with conservative retry
    pub async fn retry_system_operation<T, F, Fut>(
        operation: F,
    ) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, EnhancedError>>,
        T: Clone,
    {
        let strategy = RetryStrategy::conservative();
        let mut executor = RetryExecutor::new(strategy).unwrap();
        executor.execute(operation).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_handling::{ErrorSeverity, ErrorDomain};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_retry_strategy_default() {
        let strategy = RetryStrategy::default();
        assert_eq!(strategy.max_attempts, 3);
        assert_eq!(strategy.initial_delay_ms, 100);
        assert_eq!(strategy.max_delay_ms, 5000);
        assert_eq!(strategy.backoff_multiplier, 2.0);
        assert_eq!(strategy.jitter_factor, 0.1);
        assert!(strategy.enabled);
    }

    #[test]
    fn test_retry_strategy_validation() {
        let mut strategy = RetryStrategy::default();
        
        // Valid strategy should pass
        assert!(strategy.validate().is_ok());
        
        // Invalid max attempts
        strategy.max_attempts = 15;
        assert!(strategy.validate().is_err());
        
        // Invalid backoff multiplier
        strategy = RetryStrategy::default();
        strategy.backoff_multiplier = 0.5;
        assert!(strategy.validate().is_err());
        
        // Invalid jitter factor
        strategy = RetryStrategy::default();
        strategy.jitter_factor = 1.5;
        assert!(strategy.validate().is_err());
    }

    #[test]
    fn test_retry_strategy_delay_calculation() {
        let strategy = RetryStrategy::new(5, 100, 5000, 2.0);
        
        // First attempt should have no delay
        let delay0 = strategy.calculate_delay(0);
        assert_eq!(delay0.as_millis(), 0);
        
        // Subsequent delays should increase exponentially (with jitter)
        let delay1 = strategy.calculate_delay(1);
        let delay2 = strategy.calculate_delay(2);
        let delay3 = strategy.calculate_delay(3);
        
        // Base delays should be around 100, 200, 400 ms (with jitter)
        assert!(delay1.as_millis() >= 80 && delay1.as_millis() <= 120); // 100ms ± 20%
        assert!(delay2.as_millis() >= 160 && delay2.as_millis() <= 240); // 200ms ± 20%
        assert!(delay3.as_millis() >= 320 && delay3.as_millis() <= 480); // 400ms ± 20%
    }

    #[test]
    fn test_retry_strategy_presets() {
        let conservative = RetryStrategy::conservative();
        assert_eq!(conservative.max_attempts, 2);
        assert_eq!(conservative.backoff_multiplier, 1.5);
        
        let aggressive = RetryStrategy::aggressive();
        assert_eq!(aggressive.max_attempts, 5);
        assert_eq!(aggressive.backoff_multiplier, 2.5);
        
        let disabled = RetryStrategy::disabled();
        assert!(!disabled.enabled);
        assert_eq!(disabled.max_attempts, 0);
    }

    #[tokio::test]
    async fn test_retry_executor_success_on_first_attempt() {
        let strategy = RetryStrategy::default();
        let mut executor = RetryExecutor::new(strategy).unwrap();
        
        let result = executor.execute(|| async {
            Ok::<i32, EnhancedError>(42)
        }).await;
        
        assert!(result.result.is_ok());
        assert_eq!(result.result.unwrap(), 42);
        assert_eq!(result.attempts.len(), 0); // No retries needed
        assert!(!result.retry_limit_exceeded);
    }

    #[tokio::test]
    async fn test_retry_executor_success_after_retries() {
        let strategy = RetryStrategy::new(3, 10, 100, 2.0); // Fast retries for testing
        let mut executor = RetryExecutor::new(strategy).unwrap();
        
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();
        
        let result: RetryResult<i32> = executor.execute(move || {
            let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if count < 2 {
                    // Fail first two attempts
                    Err(EnhancedError::new(
                        ErrorSeverity::Low,
                        ErrorDomain::System,
                        RecoveryStrategy::AutoRecoverable,
                        "Transient network timeout".to_string(),
                    ))
                } else {
                    // Succeed on third attempt
                    Ok(42)
                }
            }
        }).await;
        
        assert!(result.result.is_ok());
        assert_eq!(result.result.unwrap(), 42);
        assert_eq!(result.attempts.len(), 2); // Two failed attempts before success
        assert!(!result.retry_limit_exceeded);
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3); // Three total calls
    }

    #[tokio::test]
    async fn test_retry_executor_failure_after_max_attempts() {
        let strategy = RetryStrategy::new(2, 10, 100, 2.0); // Fast retries for testing
        let mut executor = RetryExecutor::new(strategy).unwrap();
        
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();
        
        let result: RetryResult<i32> = executor.execute(move || {
            let _count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                // Always fail
                Err(EnhancedError::new(
                    ErrorSeverity::Medium,
                    ErrorDomain::System,
                    RecoveryStrategy::AutoRecoverable,
                    "Persistent timeout error".to_string(),
                ))
            }
        }).await;
        
        assert!(result.result.is_err());
        assert_eq!(result.attempts.len(), 2); // Two failed attempts
        assert!(result.retry_limit_exceeded);
        assert_eq!(attempt_count.load(Ordering::SeqCst), 2); // Two total calls
    }

    #[tokio::test]
    async fn test_retry_executor_non_retryable_error() {
        let strategy = RetryStrategy::default();
        let mut executor = RetryExecutor::new(strategy).unwrap();
        
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();
        
        let result: RetryResult<i32> = executor.execute(move || {
            let _count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                // Non-retryable error
                Err(EnhancedError::new(
                    ErrorSeverity::High,
                    ErrorDomain::Auth,
                    RecoveryStrategy::UserRecoverable,
                    "Authentication failed".to_string(),
                ))
            }
        }).await;
        
        assert!(result.result.is_err());
        assert_eq!(result.attempts.len(), 1); // Only one attempt
        assert!(!result.retry_limit_exceeded); // Didn't exceed because we didn't retry
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1); // Only one call
    }

    #[tokio::test]
    async fn test_retry_executor_disabled_strategy() {
        let strategy = RetryStrategy::disabled();
        let mut executor = RetryExecutor::new(strategy).unwrap();
        
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();
        
        let result: RetryResult<i32> = executor.execute(move || {
            let _count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                // Always fail (but retries are disabled)
                Err(EnhancedError::new(
                    ErrorSeverity::Low,
                    ErrorDomain::System,
                    RecoveryStrategy::AutoRecoverable,
                    "Some error".to_string(),
                ))
            }
        }).await;
        
        assert!(result.result.is_err());
        assert_eq!(result.attempts.len(), 0); // No attempts recorded when disabled
        assert!(!result.retry_limit_exceeded);
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1); // Only one call
    }

    #[test]
    fn test_should_retry_logic() {
        let strategy = RetryStrategy::default();
        let executor = RetryExecutor::new(strategy).unwrap();
        
        // Should retry transient errors
        let transient_error = EnhancedError::new(
            ErrorSeverity::Low,
            ErrorDomain::System,
            RecoveryStrategy::AutoRecoverable,
            "Network timeout occurred".to_string(),
        );
        assert!(executor.should_retry(&transient_error));
        
        // Should not retry auth errors
        let auth_error = EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::Auth,
            RecoveryStrategy::UserRecoverable,
            "Invalid credentials".to_string(),
        );
        assert!(!executor.should_retry(&auth_error));
        
        // Should not retry critical errors
        let critical_error = EnhancedError::new(
            ErrorSeverity::Critical,
            ErrorDomain::System,
            RecoveryStrategy::ManualRecoverable,
            "System failure".to_string(),
        );
        assert!(!executor.should_retry(&critical_error));
        
        // Should retry database connection errors
        let db_error = EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            "Database connection timeout".to_string(),
        );
        assert!(executor.should_retry(&db_error));
    }

    #[test]
    fn test_performance_tracking() {
        let strategy = RetryStrategy::default();
        let mut executor = RetryExecutor::new(strategy).unwrap();
        
        // Initially no stats
        let stats = executor.get_performance_stats();
        assert_eq!(stats.sample_count, 0);
        assert!(stats.meets_requirement);
        
        // Simulate some execution times
        executor.execution_times.push(5000); // 5ms
        executor.execution_times.push(8000); // 8ms
        executor.execution_times.push(3000); // 3ms
        
        let stats = executor.get_performance_stats();
        assert_eq!(stats.sample_count, 3);
        assert_eq!(stats.min_time_us, 3000);
        assert_eq!(stats.max_time_us, 8000);
        assert!((stats.average_time_us - 5333.333).abs() < 0.1); // Average should be ~5.33ms
        assert!(stats.meets_requirement); // All under 10ms
        
        // Add a slow execution
        executor.execution_times.push(15000); // 15ms
        let stats = executor.get_performance_stats();
        assert!(!stats.meets_requirement); // Now exceeds 10ms average
    }

    #[tokio::test]
    async fn test_retry_utils_database_operation() {
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();
        
        let result = retry_utils::retry_database_operation(move || {
            let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if count < 1 {
                    Err(EnhancedError::new(
                        ErrorSeverity::Medium,
                        ErrorDomain::Data,
                        RecoveryStrategy::AutoRecoverable,
                        "Database connection timeout".to_string(),
                    ))
                } else {
                    Ok("success".to_string())
                }
            }
        }).await;
        
        assert!(result.result.is_ok());
        assert_eq!(result.result.unwrap(), "success");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 2);
    }
}