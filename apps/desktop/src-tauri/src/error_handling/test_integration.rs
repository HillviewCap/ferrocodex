/// Integration tests for error handling system
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::error_handling::{ErrorSeverity, ErrorDomain, RecoveryStrategy, EnhancedError, ErrorContext};
    use crate::error_handling::classification::ErrorClassificationEngine;
    use crate::error_handling::repository::{ErrorContextRepository, SqliteErrorContextRepository};
    use std::sync::{Arc, Mutex};
    use tempfile::NamedTempFile;
    use rusqlite::Connection;

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        
        let repo = SqliteErrorContextRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_full_error_handling_integration() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteErrorContextRepository::new(&conn);
        let mut classification_engine = ErrorClassificationEngine::new();

        // 1. Create error context
        let context = ErrorContext::new("user_login".to_string(), "auth".to_string())
            .with_user_id(123);

        // 2. Store context in database
        repo.store_error_context(&context).unwrap();

        // 3. Create and classify an error
        let enhanced_error = classification_engine.create_enhanced_error(
            "Authentication failed".to_string(),
            Some("auth".to_string()),
            Some("login".to_string()),
        )
        .with_context_id(context.context_id);

        // 4. Store error classification
        repo.store_error_classification(&enhanced_error).unwrap();

        // 5. Retrieve and verify
        let retrieved_context = repo.get_error_context(context.context_id).unwrap().unwrap();
        assert_eq!(retrieved_context.operation, "user_login");
        assert_eq!(retrieved_context.component, "auth");
        assert_eq!(retrieved_context.user_id, Some(123));

        let retrieved_error = repo.get_error_classification(enhanced_error.id).unwrap().unwrap();
        assert_eq!(retrieved_error.severity_level, "High");
        assert_eq!(retrieved_error.domain, "Auth");
        assert_eq!(retrieved_error.recovery_strategy, "UserRecoverable");
        assert_eq!(retrieved_error.message, "Authentication failed");
    }

    #[test]
    fn test_error_severity_classification() {
        let mut engine = ErrorClassificationEngine::new();

        // Test various error types
        let test_cases = vec![
            ("Database corruption detected", ErrorSeverity::Critical),
            ("Authentication failed", ErrorSeverity::High),
            ("Validation failed", ErrorSeverity::Medium),
            ("Minor UI issue", ErrorSeverity::Low),
        ];

        for (message, expected_severity) in test_cases {
            let (severity, _domain, _recovery) = engine.classify_error(message, None, None);
            assert_eq!(severity, expected_severity, "Failed for message: {}", message);
        }
    }

    #[test]
    fn test_performance_requirements() {
        let mut engine = ErrorClassificationEngine::new();

        // Test classification performance (<500μs requirement)
        for i in 0..100 {
            let _ = engine.classify_error(
                &format!("Test error {}", i),
                Some("test_component"),
                Some("test_operation"),
            );
        }

        let stats = engine.get_performance_stats();
        assert!(stats.meets_requirement, "Classification performance requirement not met");
        assert!(stats.average_time_us < 500.0, "Average time exceeded 500μs: {}", stats.average_time_us);

        // Test context creation performance (<1ms requirement)
        let mut context_factory = crate::error_handling::context::ContextFactory::new();
        
        for i in 0..100 {
            let _context = context_factory.create_context(
                format!("operation_{}", i),
                "TestComponent".to_string(),
            );
        }

        let context_stats = context_factory.get_performance_stats();
        assert!(context_stats.meets_requirement, "Context creation performance requirement not met");
        assert!(context_stats.average_time_us < 1000.0, "Average context creation time exceeded 1ms: {}", context_stats.average_time_us);
    }

    #[test]
    fn test_cross_layer_correlation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteErrorContextRepository::new(&conn);

        // Create parent context (frontend)
        let frontend_context = ErrorContext::new("user_action".to_string(), "UserInterface".to_string())
            .with_user_id(456);
        repo.store_error_context(&frontend_context).unwrap();

        // Create child context (backend)
        let backend_context = frontend_context.create_child_context("process_request".to_string(), "BackendService".to_string());
        repo.store_error_context(&backend_context).unwrap();

        // Create correlation data
        let correlation = crate::error_handling::context::ContextCorrelation::new(
            "backend".to_string(),
            "BackendService".to_string(),
            backend_context.context_id,
        );
        repo.store_context_correlation(&correlation).unwrap();

        // Verify correlation chain
        let chain = repo.get_correlation_chain(backend_context.context_id).unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].layer, "backend");
        assert_eq!(chain[0].component, "BackendService");

        // Verify context inheritance
        assert_eq!(backend_context.request_id, frontend_context.request_id);
        assert_eq!(backend_context.user_id, Some(456));
    }

    #[test]
    fn test_backward_compatibility() {
        use crate::error_handling::conversion::ErrorConversionLayer;
        
        let mut conversion_layer = ErrorConversionLayer::new();

        // Test string to enhanced error conversion
        let enhanced_error = conversion_layer.string_to_enhanced(
            "Database connection failed".to_string(),
            Some("database".to_string()),
            Some("connect".to_string()),
        );

        assert_eq!(enhanced_error.severity, ErrorSeverity::Critical);
        assert_eq!(enhanced_error.domain, ErrorDomain::Data);
        assert_eq!(enhanced_error.message, "Database connection failed");

        // Test enhanced error to string conversion
        let string_error = conversion_layer.enhanced_to_string(&enhanced_error);
        
        // Critical errors should return generic message
        assert_eq!(string_error, "A critical error has occurred. Please contact support.");

        // Test non-critical error
        let non_critical = EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::UI,
            RecoveryStrategy::UserRecoverable,
            "Form validation failed".to_string(),
        );

        let string_error = conversion_layer.enhanced_to_string(&non_critical);
        assert_eq!(string_error, "Form validation failed");
    }

    #[test]
    fn test_feature_flags() {
        use crate::error_handling::conversion::{ErrorHandlingFeatureFlags, ErrorHandlingManager};
        
        let minimal_flags = ErrorHandlingFeatureFlags::minimal();
        let manager = ErrorHandlingManager::new(minimal_flags);

        assert!(manager.is_feature_enabled("classification"));
        assert!(!manager.is_feature_enabled("context_tracking"));
        assert!(!manager.is_feature_enabled("correlation"));
        assert!(!manager.is_feature_enabled("database_storage"));
        assert!(!manager.is_feature_enabled("performance_monitoring"));

        let full_flags = ErrorHandlingFeatureFlags::full();
        let full_manager = ErrorHandlingManager::new(full_flags);

        assert!(full_manager.is_feature_enabled("classification"));
        assert!(full_manager.is_feature_enabled("context_tracking"));
        assert!(full_manager.is_feature_enabled("correlation"));
        assert!(full_manager.is_feature_enabled("database_storage"));
        assert!(full_manager.is_feature_enabled("performance_monitoring"));
    }
}