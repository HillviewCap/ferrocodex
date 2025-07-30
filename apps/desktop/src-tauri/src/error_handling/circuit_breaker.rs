use crate::error_handling::{EnhancedError, RecoveryStrategy};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Circuit is closed - operations flow through normally
    Closed,
    /// Circuit is open - operations fail fast without execution
    Open,
    /// Circuit is half-open - testing if service has recovered
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "Half-Open"),
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of failures required to open the circuit
    pub failure_threshold: u32,
    /// Number of successes required to close circuit from half-open
    pub success_threshold: u32,
    /// Time to wait before transitioning from open to half-open (milliseconds)
    pub timeout_ms: u64,
    /// Maximum number of calls allowed in half-open state
    pub half_open_max_calls: u32,
    /// Sliding window size for failure counting
    pub sliding_window_size: u32,
    /// Enable/disable circuit breaker
    pub enabled: bool,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_ms: 10000, // 10 seconds
            half_open_max_calls: 5,
            sliding_window_size: 10,
            enabled: true,
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new circuit breaker configuration
    pub fn new(
        failure_threshold: u32,
        success_threshold: u32,
        timeout_ms: u64,
    ) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout_ms,
            half_open_max_calls: 5,
            sliding_window_size: 10,
            enabled: true,
        }
    }

    /// Create a conservative configuration for critical operations
    pub fn conservative() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 2,
            timeout_ms: 30000, // 30 seconds
            half_open_max_calls: 3,
            sliding_window_size: 5,
            enabled: true,
        }
    }

    /// Create an aggressive configuration for resilient operations
    pub fn aggressive() -> Self {
        Self {
            failure_threshold: 10,
            success_threshold: 5,
            timeout_ms: 5000, // 5 seconds
            half_open_max_calls: 10,
            sliding_window_size: 20,
            enabled: true,
        }
    }

    /// Disable circuit breaker
    pub fn disabled() -> Self {
        Self {
            failure_threshold: 0,
            success_threshold: 0,
            timeout_ms: 0,
            half_open_max_calls: 0,
            sliding_window_size: 0,
            enabled: false,
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled {
            if self.failure_threshold == 0 {
                return Err("Failure threshold must be greater than 0 when enabled".to_string());
            }
            if self.success_threshold == 0 {
                return Err("Success threshold must be greater than 0 when enabled".to_string());
            }
            if self.failure_threshold > 100 {
                return Err("Failure threshold cannot exceed 100".to_string());
            }
            if self.success_threshold > 50 {
                return Err("Success threshold cannot exceed 50".to_string());
            }
            if self.timeout_ms > 300000 { // 5 minutes
                return Err("Timeout cannot exceed 5 minutes".to_string());
            }
            if self.half_open_max_calls > 100 {
                return Err("Half-open max calls cannot exceed 100".to_string());
            }
            if self.sliding_window_size > 1000 {
                return Err("Sliding window size cannot exceed 1000".to_string());
            }
        }
        Ok(())
    }
}

/// Call outcome for circuit breaker tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CallOutcome {
    Success,
    Failure,
}

/// Circuit breaker metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerMetrics {
    /// Current state of the circuit
    pub state: CircuitState,
    /// Total number of calls made
    pub total_calls: u64,
    /// Number of successful calls
    pub successful_calls: u64,
    /// Number of failed calls
    pub failed_calls: u64,
    /// Number of calls rejected due to open circuit
    pub rejected_calls: u64,
    /// Current failure rate (0.0 to 1.0)
    pub failure_rate: f64,
    /// Time when circuit was last opened
    pub last_opened_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Time when circuit was last closed
    pub last_closed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Number of state transitions
    pub state_transitions: u64,
}

impl Default for CircuitBreakerMetrics {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            total_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            rejected_calls: 0,
            failure_rate: 0.0,
            last_opened_at: None,
            last_closed_at: None,
            state_transitions: 0,
        }
    }
}

/// Internal state of the circuit breaker
#[derive(Debug)]
struct CircuitBreakerState {
    /// Current circuit state
    state: CircuitState,
    /// Configuration
    config: CircuitBreakerConfig,
    /// Sliding window of recent call outcomes
    call_outcomes: Vec<(CallOutcome, Instant)>,
    /// Consecutive successes in half-open state
    consecutive_successes: u32,
    /// Number of calls made in half-open state
    half_open_calls: u32,
    /// Time when circuit was opened
    opened_at: Option<Instant>,
    /// Metrics
    metrics: CircuitBreakerMetrics,
    /// Performance tracking
    state_check_times: Vec<u128>, // microseconds
}

impl CircuitBreakerState {
    fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: CircuitState::Closed,
            config,
            call_outcomes: Vec::new(),
            consecutive_successes: 0,
            half_open_calls: 0,
            opened_at: None,
            metrics: CircuitBreakerMetrics::default(),
            state_check_times: Vec::new(),
        }
    }

    /// Check if the circuit should transition to half-open
    fn should_attempt_reset(&self) -> bool {
        if self.state != CircuitState::Open {
            return false;
        }

        if let Some(opened_at) = self.opened_at {
            let elapsed = opened_at.elapsed();
            elapsed.as_millis() >= self.config.timeout_ms as u128
        } else {
            false
        }
    }

    /// Transition circuit state
    fn transition_to(&mut self, new_state: CircuitState) {
        if self.state != new_state {
            self.state = new_state;
            self.metrics.state = new_state;
            self.metrics.state_transitions += 1;

            match new_state {
                CircuitState::Open => {
                    self.opened_at = Some(Instant::now());
                    self.metrics.last_opened_at = Some(chrono::Utc::now());
                }
                CircuitState::Closed => {
                    self.consecutive_successes = 0;
                    self.half_open_calls = 0;
                    self.metrics.last_closed_at = Some(chrono::Utc::now());
                }
                CircuitState::HalfOpen => {
                    self.consecutive_successes = 0;
                    self.half_open_calls = 0;
                }
            }
        }
    }

    /// Update call outcome and metrics
    fn record_call_outcome(&mut self, outcome: CallOutcome) {
        let now = Instant::now();
        
        // Add to sliding window
        self.call_outcomes.push((outcome, now));
        
        // Remove old outcomes outside sliding window
        let window_size = self.config.sliding_window_size as usize;
        if self.call_outcomes.len() > window_size {
            self.call_outcomes.drain(0..self.call_outcomes.len() - window_size);
        }
        
        // Update metrics
        self.metrics.total_calls += 1;
        match outcome {
            CallOutcome::Success => {
                self.metrics.successful_calls += 1;
                if self.state == CircuitState::HalfOpen {
                    self.consecutive_successes += 1;
                }
            }
            CallOutcome::Failure => {
                self.metrics.failed_calls += 1;
                if self.state == CircuitState::HalfOpen {
                    self.consecutive_successes = 0;
                }
            }
        }
        
        // Update failure rate
        let total_in_window = self.call_outcomes.len();
        if total_in_window > 0 {
            let failures_in_window = self.call_outcomes.iter()
                .filter(|(outcome, _)| *outcome == CallOutcome::Failure)
                .count();
            self.metrics.failure_rate = failures_in_window as f64 / total_in_window as f64;
        }
        
        // Update half-open call count
        if self.state == CircuitState::HalfOpen {
            self.half_open_calls += 1;
        }
    }

    /// Check if circuit should open based on failure threshold
    fn should_open(&self) -> bool {
        if self.state == CircuitState::Open || !self.config.enabled {
            return false;
        }

        let failures_in_window = self.call_outcomes.iter()
            .filter(|(outcome, _)| *outcome == CallOutcome::Failure)
            .count();

        failures_in_window >= self.config.failure_threshold as usize
    }

    /// Check if circuit should close from half-open
    fn should_close(&self) -> bool {
        self.state == CircuitState::HalfOpen && 
        self.consecutive_successes >= self.config.success_threshold
    }

    /// Check if half-open state should return to open
    fn should_reopen(&self) -> bool {
        self.state == CircuitState::HalfOpen && (
            self.half_open_calls >= self.config.half_open_max_calls ||
            self.consecutive_successes == 0
        )
    }

    /// Track state check performance
    fn track_state_check_time(&mut self, duration: u128) {
        self.state_check_times.push(duration);
        if self.state_check_times.len() > 1000 {
            self.state_check_times.remove(0);
        }
    }
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    /// Internal state protected by mutex
    state: Arc<Mutex<CircuitBreakerState>>,
    /// Unique identifier
    id: Uuid,
    /// Service name for identification
    service_name: String,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(service_name: String, config: CircuitBreakerConfig) -> Result<Self, String> {
        config.validate()?;
        
        Ok(Self {
            state: Arc::new(Mutex::new(CircuitBreakerState::new(config))),
            id: Uuid::new_v4(),
            service_name,
        })
    }

    /// Create a circuit breaker with default configuration
    pub fn default_config(service_name: String) -> Self {
        let config = CircuitBreakerConfig::default();
        Self {
            state: Arc::new(Mutex::new(CircuitBreakerState::new(config))),
            id: Uuid::new_v4(),
            service_name,
        }
    }

    /// Execute an operation through the circuit breaker
    pub async fn execute<T, F, Fut>(&self, operation: F) -> Result<T, EnhancedError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, EnhancedError>>,
    {
        let start_time = Instant::now();
        
        // Check circuit state and determine if we should execute
        let should_execute = {
            let mut state = self.state.lock().unwrap();
            
            // Track state check performance
            let check_start = Instant::now();
            
            // Check if we should transition from open to half-open
            if state.should_attempt_reset() {
                state.transition_to(CircuitState::HalfOpen);
            }
            
            let should_execute = match state.state {
                CircuitState::Closed => true,
                CircuitState::HalfOpen => {
                    state.half_open_calls < state.config.half_open_max_calls
                }
                CircuitState::Open => false,
            };
            
            // Record state check time
            state.track_state_check_time(check_start.elapsed().as_micros());
            
            if !should_execute {
                state.metrics.rejected_calls += 1;
            }
            
            should_execute
        };

        // If circuit is open, fail fast
        if !should_execute {
            return Err(EnhancedError::new(
                crate::error_handling::ErrorSeverity::Medium,
                crate::error_handling::ErrorDomain::System,
                RecoveryStrategy::AutoRecoverable,
                format!("Circuit breaker is open for service: {}", self.service_name),
            )
            .with_component("circuit_breaker".to_string())
            .with_operation("execute".to_string()));
        }

        // Execute the operation
        let result = operation().await;
        
        // Record outcome and update state
        {
            let mut state = self.state.lock().unwrap();
            
            match &result {
                Ok(_) => {
                    state.record_call_outcome(CallOutcome::Success);
                    
                    // Check if we should close from half-open
                    if state.should_close() {
                        state.transition_to(CircuitState::Closed);
                    }
                }
                Err(error) => {
                    // Only count as failure if it's a service-level error
                    if self.should_count_as_failure(error) {
                        state.record_call_outcome(CallOutcome::Failure);
                        
                        // Check if we should open the circuit
                        if state.should_open() {
                            state.transition_to(CircuitState::Open);
                        } else if state.should_reopen() {
                            state.transition_to(CircuitState::Open);
                        }
                    } else {
                        // Don't count client errors as failures
                        state.record_call_outcome(CallOutcome::Success);
                    }
                }
            }
        }
        
        result
    }

    /// Get current circuit state
    pub fn state(&self) -> CircuitState {
        let state = self.state.lock().unwrap();
        state.state
    }

    /// Get circuit breaker metrics
    pub fn metrics(&self) -> CircuitBreakerMetrics {
        let state = self.state.lock().unwrap();
        state.metrics.clone()
    }

    /// Get circuit breaker configuration
    pub fn config(&self) -> CircuitBreakerConfig {
        let state = self.state.lock().unwrap();
        state.config.clone()
    }

    /// Update circuit breaker configuration
    pub fn set_config(&self, config: CircuitBreakerConfig) -> Result<(), String> {
        config.validate()?;
        let mut state = self.state.lock().unwrap();
        state.config = config;
        Ok(())
    }

    /// Manually open the circuit (for testing or maintenance)
    pub fn open(&self) {
        let mut state = self.state.lock().unwrap();
        state.transition_to(CircuitState::Open);
    }

    /// Manually close the circuit (for testing or recovery)
    pub fn close(&self) {
        let mut state = self.state.lock().unwrap();
        state.transition_to(CircuitState::Closed);
    }

    /// Reset circuit breaker state and metrics
    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        state.call_outcomes.clear();
        state.consecutive_successes = 0;
        state.half_open_calls = 0;
        state.opened_at = None;
        state.metrics = CircuitBreakerMetrics::default();
        state.transition_to(CircuitState::Closed);
    }

    /// Get unique identifier
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get service name
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Check if performance requirement (<5ms state checks) is being met
    pub fn meets_performance_requirement(&self) -> bool {
        let state = self.state.lock().unwrap();
        if state.state_check_times.is_empty() {
            return true;
        }
        
        let sum: u128 = state.state_check_times.iter().sum();
        let avg = sum as f64 / state.state_check_times.len() as f64;
        avg < 5000.0 // 5ms in microseconds
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> crate::error_handling::context::PerformanceStats {
        let state = self.state.lock().unwrap();
        
        if state.state_check_times.is_empty() {
            return crate::error_handling::context::PerformanceStats {
                sample_count: 0,
                average_time_us: 0.0,
                min_time_us: 0,
                max_time_us: 0,
                meets_requirement: true,
            };
        }

        let min_time = *state.state_check_times.iter().min().unwrap();
        let max_time = *state.state_check_times.iter().max().unwrap();
        let sum: u128 = state.state_check_times.iter().sum();
        let avg_time = sum as f64 / state.state_check_times.len() as f64;

        crate::error_handling::context::PerformanceStats {
            sample_count: state.state_check_times.len(),
            average_time_us: avg_time,
            min_time_us: min_time,
            max_time_us: max_time,
            meets_requirement: avg_time < 5000.0, // 5ms requirement
        }
    }

    /// Determine if an error should count as a circuit breaker failure
    fn should_count_as_failure(&self, error: &EnhancedError) -> bool {
        // Don't count user errors as service failures
        if error.recovery_strategy == RecoveryStrategy::UserRecoverable {
            return false;
        }

        // Don't count validation errors as service failures
        if error.domain == crate::error_handling::ErrorDomain::UI {
            return false;
        }

        let message_lower = error.message.to_lowercase();
        
        // Service-level failures that should open circuit
        if message_lower.contains("service unavailable") ||
           message_lower.contains("internal server error") ||
           message_lower.contains("connection timeout") ||
           message_lower.contains("connection refused") ||
           message_lower.contains("network error") ||
           message_lower.contains("database connection") ||
           message_lower.contains("service error") {
            return true;
        }

        // High severity system errors should count
        if error.domain == crate::error_handling::ErrorDomain::System &&
           error.severity != crate::error_handling::ErrorSeverity::Low {
            return true;
        }

        // Data access failures might indicate service issues
        if error.domain == crate::error_handling::ErrorDomain::Data &&
           (message_lower.contains("timeout") || 
            message_lower.contains("connection") ||
            message_lower.contains("unavailable")) {
            return true;
        }

        false
    }
}

/// Circuit breaker registry for managing multiple circuit breakers
pub struct CircuitBreakerRegistry {
    breakers: Arc<Mutex<std::collections::HashMap<String, Arc<CircuitBreaker>>>>,
}

impl CircuitBreakerRegistry {
    /// Create a new circuit breaker registry
    pub fn new() -> Self {
        Self {
            breakers: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Get or create a circuit breaker for a service
    pub fn get_or_create(&self, service_name: &str, config: Option<CircuitBreakerConfig>) -> Arc<CircuitBreaker> {
        let mut breakers = self.breakers.lock().unwrap();
        
        if let Some(breaker) = breakers.get(service_name) {
            breaker.clone()
        } else {
            let config = config.unwrap_or_default();
            let breaker = Arc::new(CircuitBreaker::new(service_name.to_string(), config).unwrap());
            breakers.insert(service_name.to_string(), breaker.clone());
            breaker
        }
    }

    /// Get all registered circuit breakers
    pub fn get_all(&self) -> Vec<Arc<CircuitBreaker>> {
        let breakers = self.breakers.lock().unwrap();
        breakers.values().cloned().collect()
    }

    /// Remove a circuit breaker
    pub fn remove(&self, service_name: &str) -> Option<Arc<CircuitBreaker>> {
        let mut breakers = self.breakers.lock().unwrap();
        breakers.remove(service_name)
    }

    /// Reset all circuit breakers
    pub fn reset_all(&self) {
        let breakers = self.breakers.lock().unwrap();
        for breaker in breakers.values() {
            breaker.reset();
        }
    }

    /// Get metrics for all circuit breakers
    pub fn get_all_metrics(&self) -> Vec<(String, CircuitBreakerMetrics)> {
        let breakers = self.breakers.lock().unwrap();
        breakers.iter()
            .map(|(name, breaker)| (name.clone(), breaker.metrics()))
            .collect()
    }
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_handling::{ErrorSeverity, ErrorDomain};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_circuit_breaker_config_validation() {
        let mut config = CircuitBreakerConfig::default();
        assert!(config.validate().is_ok());
        
        // Invalid failure threshold
        config.failure_threshold = 0;
        assert!(config.validate().is_err());
        
        config = CircuitBreakerConfig::default();
        config.failure_threshold = 150;
        assert!(config.validate().is_err());
        
        // Invalid timeout
        config = CircuitBreakerConfig::default();
        config.timeout_ms = 400000; // > 5 minutes
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_circuit_breaker_config_presets() {
        let conservative = CircuitBreakerConfig::conservative();
        assert_eq!(conservative.failure_threshold, 3);
        assert_eq!(conservative.timeout_ms, 30000);
        
        let aggressive = CircuitBreakerConfig::aggressive();
        assert_eq!(aggressive.failure_threshold, 10);
        assert_eq!(aggressive.timeout_ms, 5000);
        
        let disabled = CircuitBreakerConfig::disabled();
        assert!(!disabled.enabled);
    }

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let config = CircuitBreakerConfig::new(3, 2, 1000);
        let circuit = CircuitBreaker::new("test_service".to_string(), config).unwrap();
        
        assert_eq!(circuit.state(), CircuitState::Closed);
        
        // Successful calls should work normally
        let result = circuit.execute(|| async {
            Ok::<i32, EnhancedError>(42)
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(circuit.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig::new(2, 2, 1000); // Open after 2 failures
        let circuit = CircuitBreaker::new("test_service".to_string(), config).unwrap();
        
        let call_count = Arc::new(AtomicU32::new(0));
        
        // Make failing calls
        for _ in 0..3 {
            let count_clone = call_count.clone();
            let result: Result<i32, EnhancedError> = circuit.execute(move || {
                count_clone.fetch_add(1, Ordering::SeqCst);
                async move {
                    Err(EnhancedError::new(
                        ErrorSeverity::High,
                        ErrorDomain::System,
                        RecoveryStrategy::AutoRecoverable,
                        "Service unavailable".to_string(),
                    ))
                }
            }).await;
            
            assert!(result.is_err());
        }
        
        // Circuit should be open after threshold failures
        assert_eq!(circuit.state(), CircuitState::Open);
        
        // Subsequent calls should fail fast without executing
        let initial_count = call_count.load(Ordering::SeqCst);
        let result = circuit.execute(|| async {
            call_count.fetch_add(1, Ordering::SeqCst);
            Ok::<i32, EnhancedError>(42)
        }).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Circuit breaker is open"));
        assert_eq!(call_count.load(Ordering::SeqCst), initial_count); // No additional execution
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_transition() {
        let config = CircuitBreakerConfig::new(1, 1, 100); // Quick timeout for testing
        let circuit = CircuitBreaker::new("test_service".to_string(), config).unwrap();
        
        // Cause a failure to open circuit
        let result: Result<i32, EnhancedError> = circuit.execute(|| async {
            Err(EnhancedError::new(
                ErrorSeverity::High,
                ErrorDomain::System,
                RecoveryStrategy::AutoRecoverable,
                "Service error".to_string(),
            ))
        }).await;
        assert!(result.is_err());
        assert_eq!(circuit.state(), CircuitState::Open);
        
        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Next call should transition to half-open
        let result = circuit.execute(|| async {
            Ok::<i32, EnhancedError>(42)
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(circuit.state(), CircuitState::Closed); // Should close after successful call
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_open() {
        let config = CircuitBreakerConfig::new(1, 2, 100); // Need 2 successes to close
        let circuit = CircuitBreaker::new("test_service".to_string(), config).unwrap();
        
        // Open the circuit
        let result: Result<i32, EnhancedError> = circuit.execute(|| async {
            Err(EnhancedError::new(
                ErrorSeverity::High,
                ErrorDomain::System,
                RecoveryStrategy::AutoRecoverable,
                "Service error".to_string(),
            ))
        }).await;
        assert!(result.is_err());
        
        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // First call transitions to half-open, then fail
        let result: Result<i32, EnhancedError> = circuit.execute(|| async {
            Err(EnhancedError::new(
                ErrorSeverity::High,
                ErrorDomain::System,
                RecoveryStrategy::AutoRecoverable,
                "Still failing".to_string(),
            ))
        }).await;
        
        assert!(result.is_err());
        assert_eq!(circuit.state(), CircuitState::Open); // Should go back to open
    }

    #[test]
    fn test_circuit_breaker_should_count_as_failure() {
        let config = CircuitBreakerConfig::default();
        let circuit = CircuitBreaker::new("test_service".to_string(), config).unwrap();
        
        // Service errors should count
        let service_error = EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::System,
            RecoveryStrategy::AutoRecoverable,
            "Service unavailable".to_string(),
        );
        assert!(circuit.should_count_as_failure(&service_error));
        
        // User errors should not count
        let user_error = EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::UI,
            RecoveryStrategy::UserRecoverable,
            "Invalid input".to_string(),
        );
        assert!(!circuit.should_count_as_failure(&user_error));
        
        // Database connection errors should count
        let db_error = EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            "Database connection timeout".to_string(),
        );
        assert!(circuit.should_count_as_failure(&db_error));
    }

    #[test]
    fn test_circuit_breaker_metrics() {
        let config = CircuitBreakerConfig::default();
        let circuit = CircuitBreaker::new("test_service".to_string(), config).unwrap();
        
        let initial_metrics = circuit.metrics();
        assert_eq!(initial_metrics.state, CircuitState::Closed);
        assert_eq!(initial_metrics.total_calls, 0);
        assert_eq!(initial_metrics.successful_calls, 0);
        assert_eq!(initial_metrics.failed_calls, 0);
        assert_eq!(initial_metrics.failure_rate, 0.0);
    }

    #[test]
    fn test_circuit_breaker_manual_operations() {
        let config = CircuitBreakerConfig::default();
        let circuit = CircuitBreaker::new("test_service".to_string(), config).unwrap();
        
        // Manual open
        circuit.open();
        assert_eq!(circuit.state(), CircuitState::Open);
        
        // Manual close
        circuit.close();
        assert_eq!(circuit.state(), CircuitState::Closed);
        
        // Reset
        circuit.reset();
        assert_eq!(circuit.state(), CircuitState::Closed);
        let metrics = circuit.metrics();
        assert_eq!(metrics.total_calls, 0);
    }

    #[test]
    fn test_circuit_breaker_registry() {
        let registry = CircuitBreakerRegistry::new();
        
        // Get or create new circuit breaker
        let breaker1 = registry.get_or_create("service1", None);
        assert_eq!(breaker1.service_name(), "service1");
        
        // Get existing circuit breaker
        let breaker1_again = registry.get_or_create("service1", None);
        assert_eq!(breaker1.id(), breaker1_again.id());
        
        // Create different service
        let breaker2 = registry.get_or_create("service2", None);
        assert_ne!(breaker1.id(), breaker2.id());
        
        // Check all breakers
        let all_breakers = registry.get_all();
        assert_eq!(all_breakers.len(), 2);
        
        // Remove breaker
        let removed = registry.remove("service1");
        assert!(removed.is_some());
        assert_eq!(registry.get_all().len(), 1);
    }

    #[test]
    fn test_performance_tracking() {
        let config = CircuitBreakerConfig::default();
        let circuit = CircuitBreaker::new("test_service".to_string(), config).unwrap();
        
        // Initially meets performance requirement
        assert!(circuit.meets_performance_requirement());
        
        let stats = circuit.get_performance_stats();
        assert_eq!(stats.sample_count, 0);
        assert!(stats.meets_requirement);
        
        // Simulate some state check times
        {
            let mut state = circuit.state.lock().unwrap();
            state.track_state_check_time(2000); // 2ms
            state.track_state_check_time(3000); // 3ms
            state.track_state_check_time(4000); // 4ms
        }
        
        let stats = circuit.get_performance_stats();
        assert_eq!(stats.sample_count, 3);
        assert_eq!(stats.min_time_us, 2000);
        assert_eq!(stats.max_time_us, 4000);
        assert!((stats.average_time_us - 3000.0).abs() < 0.1);
        assert!(stats.meets_requirement); // All under 5ms
        
        // Add a slow check
        {
            let mut state = circuit.state.lock().unwrap();
            state.track_state_check_time(8000); // 8ms
        }
        
        assert!(!circuit.meets_performance_requirement()); // Now exceeds 5ms average
    }
}