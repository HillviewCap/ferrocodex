use crate::error_handling::{EnhancedError, ErrorSeverity, ErrorDomain, RecoveryStrategy};
use std::time::Instant;

/// Error classification engine for automatic categorization
pub struct ErrorClassificationEngine {
    /// Performance tracking for classification processing
    classification_times: Vec<u128>, // microseconds
}

impl ErrorClassificationEngine {
    /// Create a new error classification engine
    pub fn new() -> Self {
        Self {
            classification_times: Vec::new(),
        }
    }

    /// Classify an error based on its message and context
    pub fn classify_error(&mut self, error_message: &str, component: Option<&str>, operation: Option<&str>) -> (ErrorSeverity, ErrorDomain, RecoveryStrategy) {
        let start = Instant::now();
        
        let severity = self.assess_severity(error_message, component, operation);
        let domain = self.classify_domain(error_message, component, operation);
        let recovery_strategy = self.determine_recovery_strategy(error_message, &severity, &domain);
        
        let elapsed = start.elapsed().as_micros();
        self.classification_times.push(elapsed);
        
        // Keep only last 1000 measurements for performance monitoring
        if self.classification_times.len() > 1000 {
            self.classification_times.remove(0);
        }
        
        (severity, domain, recovery_strategy)
    }

    /// Create an enhanced error from a standard error with automatic classification
    pub fn create_enhanced_error(&mut self, error_message: String, component: Option<String>, operation: Option<String>) -> EnhancedError {
        let (severity, domain, recovery_strategy) = self.classify_error(
            &error_message,
            component.as_deref(),
            operation.as_deref(),
        );
        
        let mut enhanced_error = EnhancedError::new(severity, domain, recovery_strategy, error_message);
        
        if let Some(comp) = component {
            enhanced_error = enhanced_error.with_component(comp);
        }
        
        if let Some(op) = operation {
            enhanced_error = enhanced_error.with_operation(op);
        }
        
        enhanced_error
    }

    /// Assess error severity based on message content and context
    fn assess_severity(&self, error_message: &str, component: Option<&str>, operation: Option<&str>) -> ErrorSeverity {
        let message_lower = error_message.to_lowercase();
        
        // Critical severity indicators
        if message_lower.contains("critical") ||
           message_lower.contains("fatal") ||
           message_lower.contains("panic") ||
           message_lower.contains("system failure") ||
           message_lower.contains("database corruption") ||
           message_lower.contains("security breach") ||
           message_lower.contains("memory corruption") ||
           message_lower.contains("stack overflow") ||
           (component == Some("database") && message_lower.contains("connection") && message_lower.contains("failed")) ||
           (operation == Some("initialize_database") && message_lower.contains("failed")) {
            return ErrorSeverity::Critical;
        }
        
        // High severity indicators
        if message_lower.contains("authentication failed") ||
           message_lower.contains("authorization denied") ||
           message_lower.contains("permission denied") ||
           message_lower.contains("access denied") ||
           message_lower.contains("invalid credentials") ||
           message_lower.contains("connection timeout") ||
           message_lower.contains("file not found") ||
           message_lower.contains("network error") ||
           message_lower.contains("encryption failed") ||
           message_lower.contains("decryption failed") ||
           (component == Some("auth") && message_lower.contains("failed")) ||
           (component == Some("vault") && message_lower.contains("access")) {
            return ErrorSeverity::High;
        }
        
        // Medium severity indicators
        if message_lower.contains("validation failed") ||
           message_lower.contains("invalid input") ||
           message_lower.contains("missing required") ||
           message_lower.contains("format error") ||
           message_lower.contains("parsing failed") ||
           message_lower.contains("configuration error") ||
           message_lower.contains("timeout") ||
           message_lower.contains("retry limit exceeded") ||
           (component == Some("validation") && message_lower.contains("failed")) ||
           (operation.is_some() && message_lower.contains("validation")) {
            return ErrorSeverity::Medium;
        }
        
        // Default to low severity for unclassified errors
        ErrorSeverity::Low
    }

    /// Classify error domain based on message content and context
    fn classify_domain(&self, error_message: &str, component: Option<&str>, operation: Option<&str>) -> ErrorDomain {
        let message_lower = error_message.to_lowercase();
        
        // Auth domain indicators
        if message_lower.contains("authentication") ||
           message_lower.contains("authorization") ||
           message_lower.contains("login") ||
           message_lower.contains("session") ||
           message_lower.contains("token") ||
           message_lower.contains("credentials") ||
           message_lower.contains("password") ||
           message_lower.contains("permission") ||
           component == Some("auth") ||
           component == Some("session") ||
           operation == Some("login") ||
           operation == Some("authenticate") ||
           operation == Some("authorize") {
            return ErrorDomain::Auth;
        }
        
        // Data domain indicators
        if message_lower.contains("database") ||
           message_lower.contains("sql") ||
           message_lower.contains("query") ||
           message_lower.contains("table") ||
           message_lower.contains("connection") ||
           message_lower.contains("transaction") ||
           message_lower.contains("constraint") ||
           message_lower.contains("foreign key") ||
           message_lower.contains("unique") ||
           component == Some("database") ||
           component == Some("repository") ||
           component.map_or(false, |c| c.contains("Repository")) ||
           operation == Some("create_user") ||
           operation == Some("find_by_id") ||
           operation == Some("update") ||
           operation == Some("delete") {
            return ErrorDomain::Data;
        }
        
        // Assets domain indicators
        if message_lower.contains("asset") ||
           message_lower.contains("configuration") ||
           message_lower.contains("firmware") ||
           message_lower.contains("branch") ||
           message_lower.contains("version") ||
           message_lower.contains("deployment") ||
           message_lower.contains("recovery") ||
           component == Some("assets") ||
           component == Some("configurations") ||
           component == Some("firmware") ||
           component == Some("branches") ||
           component == Some("recovery") ||
           operation.map_or(false, |op| op.contains("asset")) ||
           operation.map_or(false, |op| op.contains("configuration")) ||
           operation.map_or(false, |op| op.contains("firmware")) {
            return ErrorDomain::Assets;
        }
        
        // System domain indicators
        if message_lower.contains("system") ||
           message_lower.contains("file") ||
           message_lower.contains("directory") ||
           message_lower.contains("path") ||
           message_lower.contains("network") ||
           message_lower.contains("memory") ||
           message_lower.contains("disk") ||
           message_lower.contains("cpu") ||
           message_lower.contains("process") ||
           message_lower.contains("thread") ||
           message_lower.contains("initialization") ||
           component == Some("system") ||
           component == Some("file_system") ||
           component == Some("network") ||
           operation == Some("initialize") ||
           operation == Some("startup") ||
           operation == Some("shutdown") {
            return ErrorDomain::System;
        }
        
        // UI domain indicators  
        if message_lower.contains("ui") ||
           message_lower.contains("interface") ||
           message_lower.contains("component") ||
           message_lower.contains("render") ||
           message_lower.contains("display") ||
           message_lower.contains("view") ||
           message_lower.contains("form") ||
           message_lower.contains("input") ||
           component == Some("ui") ||
           component.map_or(false, |c| c.contains("Component")) ||
           operation.map_or(false, |op| op.contains("render")) ||
           operation.map_or(false, |op| op.contains("display")) {
            return ErrorDomain::UI;
        }
        
        // Default based on component if no clear domain indicators
        match component {
            Some("vault") => ErrorDomain::Assets, // Vault is part of asset management
            Some("audit") => ErrorDomain::Data,   // Audit is data-related
            Some("encryption") => ErrorDomain::System, // Encryption is system-level
            Some("validation") => ErrorDomain::System, // Validation is system-level
            _ => ErrorDomain::System, // Default to system for unclassified errors
        }
    }

    /// Determine recovery strategy based on error characteristics
    fn determine_recovery_strategy(&self, error_message: &str, severity: &ErrorSeverity, domain: &ErrorDomain) -> RecoveryStrategy {
        let message_lower = error_message.to_lowercase();
        
        // Manual recovery indicators (highest priority)
        if message_lower.contains("corruption") ||
           message_lower.contains("fatal") ||
           message_lower.contains("panic") ||
           message_lower.contains("system failure") ||
           message_lower.contains("security breach") ||
           *severity == ErrorSeverity::Critical {
            return RecoveryStrategy::ManualRecoverable;
        }
        
        // Admin recovery indicators
        if message_lower.contains("permission denied") ||
           message_lower.contains("access denied") ||
           message_lower.contains("authorization") ||
           message_lower.contains("administrator") ||
           message_lower.contains("configuration error") ||
           message_lower.contains("database connection") ||
           (*domain == ErrorDomain::Auth && *severity == ErrorSeverity::High) ||
           (*domain == ErrorDomain::System && *severity != ErrorSeverity::Low) {
            return RecoveryStrategy::AdminRecoverable;
        }
        
        // User recovery indicators
        if message_lower.contains("invalid credentials") ||
           message_lower.contains("authentication failed") ||
           message_lower.contains("validation failed") ||
           message_lower.contains("invalid input") ||
           message_lower.contains("missing required") ||
           message_lower.contains("format error") ||
           (*domain == ErrorDomain::UI) ||
           (*domain == ErrorDomain::Auth && message_lower.contains("password")) {
            return RecoveryStrategy::UserRecoverable;
        }
        
        // Auto recovery indicators
        if message_lower.contains("timeout") ||
           message_lower.contains("retry") ||
           message_lower.contains("temporary") ||
           message_lower.contains("transient") ||
           message_lower.contains("network") ||
           (*severity == ErrorSeverity::Low && *domain != ErrorDomain::Auth) {
            return RecoveryStrategy::AutoRecoverable;
        }
        
        // Default based on severity and domain
        match (severity, domain) {
            (ErrorSeverity::Critical, _) => RecoveryStrategy::ManualRecoverable,
            (ErrorSeverity::High, ErrorDomain::Auth) => RecoveryStrategy::UserRecoverable,
            (ErrorSeverity::High, ErrorDomain::System) => RecoveryStrategy::AdminRecoverable,
            (ErrorSeverity::High, _) => RecoveryStrategy::AdminRecoverable,
            (ErrorSeverity::Medium, ErrorDomain::UI) => RecoveryStrategy::UserRecoverable,
            (ErrorSeverity::Medium, _) => RecoveryStrategy::AdminRecoverable,
            (ErrorSeverity::Low, _) => RecoveryStrategy::AutoRecoverable,
        }
    }

    /// Get average classification time in microseconds
    pub fn get_average_classification_time(&self) -> Option<f64> {
        if self.classification_times.is_empty() {
            None
        } else {
            let sum: u128 = self.classification_times.iter().sum();
            Some(sum as f64 / self.classification_times.len() as f64)
        }
    }

    /// Check if performance requirement (<500μs) is being met
    pub fn meets_performance_requirement(&self) -> bool {
        match self.get_average_classification_time() {
            Some(avg_time) => avg_time < 500.0, // 500 microseconds
            None => true, // No measurements yet, assume OK
        }
    }

    /// Get performance statistics for classification
    pub fn get_performance_stats(&self) -> crate::error_handling::context::PerformanceStats {
        if self.classification_times.is_empty() {
            return crate::error_handling::context::PerformanceStats {
                sample_count: 0,
                average_time_us: 0.0,
                min_time_us: 0,
                max_time_us: 0,
                meets_requirement: true,
            };
        }

        let min_time = *self.classification_times.iter().min().unwrap();
        let max_time = *self.classification_times.iter().max().unwrap();
        let avg_time = self.get_average_classification_time().unwrap();

        crate::error_handling::context::PerformanceStats {
            sample_count: self.classification_times.len(),
            average_time_us: avg_time,
            min_time_us: min_time,
            max_time_us: max_time,
            meets_requirement: avg_time < 500.0,
        }
    }
}

impl Default for ErrorClassificationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for quick error classification
pub mod quick_classify {
    use super::*;

    /// Quickly classify an authentication error
    pub fn auth_error(message: String, severity: Option<ErrorSeverity>) -> EnhancedError {
        let severity = severity.unwrap_or(ErrorSeverity::High);
        let recovery_strategy = match severity {
            ErrorSeverity::Critical => RecoveryStrategy::ManualRecoverable,
            ErrorSeverity::High => RecoveryStrategy::UserRecoverable,
            _ => RecoveryStrategy::UserRecoverable,
        };
        
        EnhancedError::new(severity, ErrorDomain::Auth, recovery_strategy, message)
            .with_component("auth".to_string())
    }

    /// Quickly classify a database error
    pub fn database_error(message: String, severity: Option<ErrorSeverity>) -> EnhancedError {
        let severity = severity.unwrap_or(ErrorSeverity::High);
        let recovery_strategy = match severity {
            ErrorSeverity::Critical => RecoveryStrategy::ManualRecoverable,
            ErrorSeverity::High => RecoveryStrategy::AdminRecoverable,
            ErrorSeverity::Medium => RecoveryStrategy::AdminRecoverable,
            ErrorSeverity::Low => RecoveryStrategy::AutoRecoverable,
        };
        
        EnhancedError::new(severity, ErrorDomain::Data, recovery_strategy, message)
            .with_component("database".to_string())
    }

    /// Quickly classify a validation error
    pub fn validation_error(message: String) -> EnhancedError {
        EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::UI,
            RecoveryStrategy::UserRecoverable,
            message,
        )
        .with_component("validation".to_string())
    }

    /// Quickly classify a system error
    pub fn system_error(message: String, severity: Option<ErrorSeverity>) -> EnhancedError {
        let severity = severity.unwrap_or(ErrorSeverity::High);
        let recovery_strategy = match severity {
            ErrorSeverity::Critical => RecoveryStrategy::ManualRecoverable,
            _ => RecoveryStrategy::AdminRecoverable,
        };
        
        EnhancedError::new(severity, ErrorDomain::System, recovery_strategy, message)
            .with_component("system".to_string())
    }

    /// Quickly classify an asset management error
    pub fn asset_error(message: String, severity: Option<ErrorSeverity>) -> EnhancedError {
        let severity = severity.unwrap_or(ErrorSeverity::Medium);
        let recovery_strategy = match severity {
            ErrorSeverity::Critical => RecoveryStrategy::ManualRecoverable,
            ErrorSeverity::High => RecoveryStrategy::AdminRecoverable,
            _ => RecoveryStrategy::UserRecoverable,
        };
        
        EnhancedError::new(severity, ErrorDomain::Assets, recovery_strategy, message)
            .with_component("assets".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_assessment() {
        let mut engine = ErrorClassificationEngine::new();
        
        // Test critical severity
        let (severity, _, _) = engine.classify_error("Critical system failure", None, None);
        assert_eq!(severity, ErrorSeverity::Critical);
        
        let (severity, _, _) = engine.classify_error("Database corruption detected", None, None);
        assert_eq!(severity, ErrorSeverity::Critical);
        
        // Test high severity
        let (severity, _, _) = engine.classify_error("Authentication failed", None, None);
        assert_eq!(severity, ErrorSeverity::High);
        
        let (severity, _, _) = engine.classify_error("Permission denied", None, None);
        assert_eq!(severity, ErrorSeverity::High);
        
        // Test medium severity
        let (severity, _, _) = engine.classify_error("Validation failed", None, None);
        assert_eq!(severity, ErrorSeverity::Medium);
        
        let (severity, _, _) = engine.classify_error("Invalid input provided", None, None);
        assert_eq!(severity, ErrorSeverity::Medium);
        
        // Test low severity (default)
        let (severity, _, _) = engine.classify_error("Minor issue occurred", None, None);
        assert_eq!(severity, ErrorSeverity::Low);
    }

    #[test]
    fn test_domain_classification() {
        let mut engine = ErrorClassificationEngine::new();
        
        // Test auth domain
        let (_, domain, _) = engine.classify_error("Authentication failed", Some("auth"), None);
        assert_eq!(domain, ErrorDomain::Auth);
        
        let (_, domain, _) = engine.classify_error("Invalid credentials", None, Some("login"));
        assert_eq!(domain, ErrorDomain::Auth);
        
        // Test data domain
        let (_, domain, _) = engine.classify_error("Database connection failed", Some("database"), None);
        assert_eq!(domain, ErrorDomain::Data);
        
        let (_, domain, _) = engine.classify_error("SQL query failed", None, Some("find_by_id"));
        assert_eq!(domain, ErrorDomain::Data);
        
        // Test assets domain
        let (_, domain, _) = engine.classify_error("Asset not found", Some("assets"), None);
        assert_eq!(domain, ErrorDomain::Assets);
        
        let (_, domain, _) = engine.classify_error("Configuration error", None, Some("create_asset"));
        assert_eq!(domain, ErrorDomain::Assets);
        
        // Test system domain
        let (_, domain, _) = engine.classify_error("File not found", Some("file_system"), None);
        assert_eq!(domain, ErrorDomain::System);
        
        let (_, domain, _) = engine.classify_error("Memory allocation failed", None, Some("initialize"));
        assert_eq!(domain, ErrorDomain::System);
        
        // Test UI domain
        let (_, domain, _) = engine.classify_error("Component render failed", Some("ui"), None);
        assert_eq!(domain, ErrorDomain::UI);
        
        let (_, domain, _) = engine.classify_error("Form validation error", None, Some("render_form"));
        assert_eq!(domain, ErrorDomain::UI);
    }

    #[test]
    fn test_recovery_strategy_determination() {
        let mut engine = ErrorClassificationEngine::new();
        
        // Test manual recovery
        let (_, _, recovery) = engine.classify_error("System corruption detected", None, None);
        assert_eq!(recovery, RecoveryStrategy::ManualRecoverable);
        
        let (_, _, recovery) = engine.classify_error("Fatal error occurred", None, None);
        assert_eq!(recovery, RecoveryStrategy::ManualRecoverable);
        
        // Test admin recovery
        let (_, _, recovery) = engine.classify_error("Permission denied", None, None);
        assert_eq!(recovery, RecoveryStrategy::AdminRecoverable);
        
        let (_, _, recovery) = engine.classify_error("Configuration error", None, None);
        assert_eq!(recovery, RecoveryStrategy::AdminRecoverable);
        
        // Test user recovery
        let (_, _, recovery) = engine.classify_error("Invalid credentials", None, None);
        assert_eq!(recovery, RecoveryStrategy::UserRecoverable);
        
        let (_, _, recovery) = engine.classify_error("Validation failed", None, None);
        assert_eq!(recovery, RecoveryStrategy::UserRecoverable);
        
        // Test auto recovery
        let (_, _, recovery) = engine.classify_error("Network timeout", None, None);
        assert_eq!(recovery, RecoveryStrategy::AutoRecoverable);
        
        let (_, _, recovery) = engine.classify_error("Temporary failure", None, None);
        assert_eq!(recovery, RecoveryStrategy::AutoRecoverable);
    }

    #[test]
    fn test_enhanced_error_creation() {
        let mut engine = ErrorClassificationEngine::new();
        
        let error = engine.create_enhanced_error(
            "Database connection failed".to_string(),
            Some("database".to_string()),
            Some("connect".to_string()),
        );
        
        assert_eq!(error.severity, ErrorSeverity::Critical);
        assert_eq!(error.domain, ErrorDomain::Data);
        assert_eq!(error.recovery_strategy, RecoveryStrategy::ManualRecoverable);
        assert_eq!(error.message, "Database connection failed");
        assert_eq!(error.component, Some("database".to_string()));
        assert_eq!(error.operation, Some("connect".to_string()));
    }

    #[test]
    fn test_classification_performance() {
        let mut engine = ErrorClassificationEngine::new();
        
        // Perform multiple classifications to test performance
        for i in 0..100 {
            let _ = engine.classify_error(
                &format!("Test error {}", i),
                Some("test_component"),
                Some("test_operation"),
            );
        }
        
        let stats = engine.get_performance_stats();
        assert_eq!(stats.sample_count, 100);
        assert!(stats.average_time_us > 0.0);
        assert!(stats.min_time_us <= stats.max_time_us);
        
        // Performance requirement should be met (<500μs)
        assert!(engine.meets_performance_requirement());
        assert!(stats.meets_requirement);
    }

    #[test]
    fn test_quick_classify_functions() {
        // Test auth error
        let auth_error = quick_classify::auth_error("Login failed".to_string(), None);
        assert_eq!(auth_error.domain, ErrorDomain::Auth);
        assert_eq!(auth_error.severity, ErrorSeverity::High);
        assert_eq!(auth_error.recovery_strategy, RecoveryStrategy::UserRecoverable);
        
        // Test database error
        let db_error = quick_classify::database_error("Query failed".to_string(), Some(ErrorSeverity::Critical));
        assert_eq!(db_error.domain, ErrorDomain::Data);
        assert_eq!(db_error.severity, ErrorSeverity::Critical);
        assert_eq!(db_error.recovery_strategy, RecoveryStrategy::ManualRecoverable);
        
        // Test validation error
        let validation_error = quick_classify::validation_error("Invalid input".to_string());
        assert_eq!(validation_error.domain, ErrorDomain::UI);
        assert_eq!(validation_error.severity, ErrorSeverity::Medium);
        assert_eq!(validation_error.recovery_strategy, RecoveryStrategy::UserRecoverable);
        
        // Test system error
        let system_error = quick_classify::system_error("File not found".to_string(), None);
        assert_eq!(system_error.domain, ErrorDomain::System);
        assert_eq!(system_error.severity, ErrorSeverity::High);
        assert_eq!(system_error.recovery_strategy, RecoveryStrategy::AdminRecoverable);
        
        // Test asset error
        let asset_error = quick_classify::asset_error("Asset not found".to_string(), Some(ErrorSeverity::Medium));
        assert_eq!(asset_error.domain, ErrorDomain::Assets);
        assert_eq!(asset_error.severity, ErrorSeverity::Medium);
        assert_eq!(asset_error.recovery_strategy, RecoveryStrategy::UserRecoverable);
    }
}