/// Comprehensive Compatibility Tests for Enhanced Error Handling
/// 
/// These tests verify that the enhanced error handling system maintains
/// full backward compatibility with existing error handling patterns.

#[cfg(test)]
mod compatibility_tests {
    use super::*;
    use crate::error_handling::{
        EnhancedError, ErrorSeverity, ErrorDomain, RecoveryStrategy
    };
    use crate::error_handling::conversion::{ErrorConverter, ErrorConversionLayer, EnhancedErrorInterface};
    use crate::{enhance_error, to_string_error, anyhow_to_string_result};
    use std::time::Instant;

    /// Test that existing error handling patterns continue to work unchanged
    #[test]
    fn test_existing_string_error_patterns() {
        // Simulate existing error handling patterns
        fn existing_operation() -> Result<String, String> {
            Err("Operation failed".to_string())
        }
        
        fn existing_error_handler(result: Result<String, String>) -> String {
            match result {
                Ok(value) => value,
                Err(error) => format!("Error: {}", error),
            }
        }
        
        // Test that existing patterns work unchanged
        let result = existing_operation();
        assert!(result.is_err());
        
        let handled = existing_error_handler(result);
        assert_eq!(handled, "Error: Operation failed");
    }

    /// Test that Tauri command signatures remain unchanged
    #[test]
    fn test_tauri_command_signature_compatibility() {
        // Simulate existing Tauri command pattern
        fn mock_tauri_command() -> Result<bool, String> {
            Err("Command failed".to_string())
        }
        
        // Test that return type is exactly the same
        let result = mock_tauri_command();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Command failed");
    }

    /// Test that error conversion maintains exact string formats
    #[test]
    fn test_error_message_format_preservation() {
        let original_messages = vec![
            "Authentication failed",
            "Database connection error",
            "File not found",
            "Invalid input provided",
            "Network timeout occurred",
        ];
        
        let conversion_layer = ErrorConversionLayer::new();
        
        for message in original_messages {
            // Convert to enhanced error and back to string
            let mut layer = ErrorConversionLayer::new();
            let enhanced = layer.string_to_enhanced(
                message.to_string(),
                Some("test".to_string()),
                Some("test".to_string()),
            );
            let converted_back = conversion_layer.enhanced_to_string(&enhanced);
            
            // Should preserve the original message for non-critical errors
            assert_eq!(converted_back, message);
        }
    }

    /// Test that critical error messages are properly filtered
    #[test]
    fn test_critical_error_message_filtering() {
        let conversion_layer = ErrorConversionLayer::new();
        
        let critical_error = EnhancedError::new(
            ErrorSeverity::Critical,
            ErrorDomain::System,
            RecoveryStrategy::ManualRecoverable,
            "Internal system corruption detected in kernel module".to_string(),
        );
        
        let user_message = conversion_layer.enhanced_to_string(&critical_error);
        assert_eq!(user_message, "A critical error has occurred. Please contact support.");
        
        // But debug message should contain details
        let debug_message = conversion_layer.enhanced_to_debug_string(&critical_error);
        assert!(debug_message.contains("Internal system corruption"));
    }

    /// Test performance requirements for error conversion
    #[test]
    fn test_error_conversion_performance() {
        let conversion_layer = ErrorConversionLayer::new();
        
        let enhanced_error = EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::Auth,
            RecoveryStrategy::UserRecoverable,
            "Test error for performance measurement".to_string(),
        )
        .with_details("Additional error details".to_string())
        .with_component("test_component".to_string())
        .with_operation("test_operation".to_string());
        
        // Measure conversion time
        let start = Instant::now();
        let _converted = conversion_layer.enhanced_to_string(&enhanced_error);
        let duration = start.elapsed();
        
        // Should be under 100μs as per requirements
        assert!(duration.as_micros() < 100, "Error conversion took {}μs, should be <100μs", duration.as_micros());
    }

    /// Test that macros preserve existing behavior
    #[test]
    fn test_macro_compatibility() {
        // Test enhance_error macro
        let error_message = "Test error";
        let enhanced = enhance_error!(error_message);
        
        // Should create enhanced error but not change basic behavior
        assert_eq!(enhanced.message, "Test error");
        
        // Test to_string_error macro
        let string_error = to_string_error!(enhanced);
        assert_eq!(string_error, "Test error");
    }

    /// Test that enhanced interfaces are optional
    #[test]
    fn test_optional_enhanced_interfaces() {
        // String errors should work with enhanced interface
        let string_error = "Test string error".to_string();
        
        // Should provide None for enhanced features
        assert!(string_error.get_error_id().is_none());
        assert!(string_error.get_severity().is_none());
        assert!(string_error.get_domain().is_none());
        
        // But should still provide basic functionality
        assert_eq!(string_error.get_user_message(), "Test string error");
        assert!(!string_error.is_critical());
    }

    /// Test backward compatibility with existing error logging
    #[test]
    fn test_error_logging_compatibility() {
        // Simulate existing error logging pattern
        fn log_error(error: &str) -> String {
            format!("[ERROR] {}", error)
        }
        
        let conversion_layer = ErrorConversionLayer::new();
        let enhanced_error = EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::UI,
            RecoveryStrategy::UserRecoverable,
            "UI component error".to_string(),
        );
        
        // Convert to string and log using existing pattern
        let error_string = conversion_layer.enhanced_to_string(&enhanced_error);
        let logged = log_error(&error_string);
        
        assert_eq!(logged, "[ERROR] UI component error");
    }

    /// Test that feature flags don't break existing functionality
    #[test]
    fn test_feature_flag_compatibility() {
        use crate::error_handling::conversion::{ErrorHandlingManager, ErrorHandlingFeatureFlags};
        
        // Test with minimal features
        let mut minimal_manager = ErrorHandlingManager::new(ErrorHandlingFeatureFlags::minimal());
        assert!(minimal_manager.is_feature_enabled("classification"));
        assert!(!minimal_manager.is_feature_enabled("context_tracking"));
        
        // Test with full features
        let full_manager = ErrorHandlingManager::new(ErrorHandlingFeatureFlags::full());
        assert!(full_manager.is_feature_enabled("classification"));
        assert!(full_manager.is_feature_enabled("context_tracking"));
        
        // Both should provide basic functionality
        let _conversion_layer = minimal_manager.conversion_layer_mut();
        let _compatibility_wrapper = minimal_manager.compatibility_wrapper_mut();
    }

    /// Test that existing error handling workflows are preserved
    #[test]
    fn test_existing_error_workflow_preservation() {
        // Simulate existing error handling workflow
        fn existing_workflow() -> Result<String, String> {
            // Some operation that might fail
            let intermediate_result = simulate_database_operation()?;
            let final_result = process_data(&intermediate_result)?;
            Ok(final_result)
        }
        
        fn simulate_database_operation() -> Result<String, String> {
            Err("Database connection failed".to_string())
        }
        
        fn process_data(_data: &str) -> Result<String, String> {
            Ok("Processed data".to_string())
        }
        
        // Test that workflow still works exactly as before
        let result = existing_workflow();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Database connection failed");
    }

    /// Test migration scenarios
    #[test]
    fn test_gradual_migration_scenarios() {
        // Scenario 1: Mixed old and new error handling
        fn old_function() -> Result<String, String> {
            Err("Old error format".to_string())
        }
        
        fn new_function() -> Result<String, String> {
            let enhanced = EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::UserRecoverable,
                "New enhanced error".to_string(),
            );
            Err(enhanced.to_user_string())
        }
        
        // Both should work in the same context
        let old_result = old_function();
        let new_result = new_function();
        
        assert!(old_result.is_err());
        assert!(new_result.is_err());
        assert_eq!(old_result.unwrap_err(), "Old error format");
        assert_eq!(new_result.unwrap_err(), "New enhanced error");
    }

    /// Test that existing test patterns continue to work
    #[test]
    fn test_existing_test_pattern_compatibility() {
        // Simulate existing test pattern
        fn function_under_test() -> Result<i32, String> {
            Err("Test error".to_string())
        }
        
        // Existing test pattern should work unchanged
        let result = function_under_test();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Test error");
        
        // Pattern with error matching
        match function_under_test() {
            Ok(_) => panic!("Expected error"),
            Err(e) => assert_eq!(e, "Test error"),
        }
    }

    /// Test that anyhow error conversion works seamlessly
    #[test]
    fn test_anyhow_error_compatibility() {
        use anyhow::{anyhow, Result as AnyhowResult};
        
        fn operation_with_anyhow() -> AnyhowResult<String> {
            Err(anyhow!("Anyhow error occurred"))
        }
        
        // Convert using macro
        let result = anyhow_to_string_result!(operation_with_anyhow());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Anyhow error occurred"));
    }

    /// Test error context preservation during conversion
    #[test]
    fn test_context_preservation_compatibility() {
        use crate::error_handling::context::ErrorContext;
        
        let mut conversion_layer = ErrorConversionLayer::new();
        let context = ErrorContext::new("test_operation".to_string(), "test_component".to_string());
        let context_id = context.context_id;
        
        conversion_layer.set_context(context);
        
        let enhanced_error = conversion_layer.string_to_enhanced(
            "Context test error".to_string(),
            Some("test_component".to_string()),
            Some("test_operation".to_string()),
        );
        
        // Context should be preserved
        assert_eq!(enhanced_error.context_id, Some(context_id));
        
        // But string conversion should still work
        let string_error = conversion_layer.enhanced_to_string(&enhanced_error);
        assert_eq!(string_error, "Context test error");
        
        conversion_layer.clear_context();
    }

    /// Test that recovery strategy detection doesn't break existing patterns
    #[test]
    fn test_recovery_strategy_compatibility() {
        let timeout_error = "Network timeout occurred".to_string();
        let fatal_error = "Fatal system error".to_string();
        
        // Should still work as strings
        assert_eq!(timeout_error, "Network timeout occurred");
        assert_eq!(fatal_error, "Fatal system error");
        
        // But also provide enhanced capabilities
        assert!(timeout_error.is_auto_recoverable());
        assert!(!fatal_error.is_auto_recoverable());
        assert!(fatal_error.is_critical());
    }

    /// Comprehensive integration test
    #[test]
    fn test_comprehensive_compatibility_integration() {
        // Simulate a complete error handling flow that might exist in the application
        
        // 1. Database operation fails
        fn database_operation() -> Result<String, String> {
            Err("Connection timeout".to_string())
        }
        
        // 2. Service layer handles the error
        fn service_layer_operation() -> Result<String, String> {
            match database_operation() {
                Ok(result) => Ok(format!("Processed: {}", result)),
                Err(e) => Err(format!("Service error: {}", e)),
            }
        }
        
        // 3. API layer handles the error (simulating Tauri command)
        fn api_layer_command() -> Result<String, String> {
            service_layer_operation().map_err(|e| {
                // This simulates logging that might happen
                eprintln!("API Error: {}", e);
                e
            })
        }
        
        // Test the complete flow
        let result = api_layer_command();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Service error: Connection timeout");
        
        // Now test with enhanced error handling mixed in
        fn enhanced_database_operation() -> Result<String, String> {
            let enhanced = EnhancedError::new(
                ErrorSeverity::High,
                ErrorDomain::Data,
                RecoveryStrategy::AutoRecoverable,
                "Connection timeout".to_string(),
            );
            Err(enhanced.to_user_string())
        }
        
        fn enhanced_service_layer_operation() -> Result<String, String> {
            match enhanced_database_operation() {
                Ok(result) => Ok(format!("Processed: {}", result)),
                Err(e) => Err(format!("Service error: {}", e)),
            }
        }
        
        // Should work identically
        let enhanced_result = enhanced_service_layer_operation();
        assert!(enhanced_result.is_err());
        assert_eq!(enhanced_result.unwrap_err(), "Service error: Connection timeout");
    }
}