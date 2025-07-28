use crate::error_handling::{EnhancedError, ErrorSeverity, ErrorDomain, RecoveryStrategy, ErrorContext, classification::ErrorClassificationEngine};
use anyhow::Error as AnyhowError;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Trait for converting enhanced errors to string format with context preservation
pub trait ErrorConverter {
    /// Convert enhanced error to user-friendly string (hides sensitive context)
    fn to_user_string(&self) -> String;
    
    /// Convert enhanced error to debug string (includes enhanced context)
    fn to_debug_string(&self) -> String;
    
    /// Convert enhanced error for backward compatibility (alias for to_user_string)
    fn to_compat_string(&self) -> String {
        self.to_user_string()
    }
}

/// Implementation of ErrorConverter trait for EnhancedError
impl ErrorConverter for EnhancedError {
    /// Convert enhanced error to user-friendly string (hides sensitive context)
    fn to_user_string(&self) -> String {
        match self.severity {
            ErrorSeverity::Critical => {
                // For critical errors, hide sensitive details and provide generic message
                "A critical error has occurred. Please contact support.".to_string()
            },
            ErrorSeverity::High => {
                // For high severity, show the message but filter sensitive operations
                if let Some(ref operation) = self.operation {
                    if operation.contains("password") || operation.contains("secret") || operation.contains("key") {
                        "A security-related error occurred. Please try again.".to_string()
                    } else {
                        self.message.clone()
                    }
                } else {
                    self.message.clone()
                }
            },
            ErrorSeverity::Medium | ErrorSeverity::Low => {
                // For medium/low severity, show the actual message
                self.message.clone()
            }
        }
    }
    
    /// Convert enhanced error to debug string (includes enhanced context)
    fn to_debug_string(&self) -> String {
        let mut debug_info = Vec::new();
        
        // Basic error information
        debug_info.push(format!("ID: {}", self.id));
        debug_info.push(format!("Severity: {}", self.severity));
        debug_info.push(format!("Domain: {}", self.domain));
        debug_info.push(format!("Recovery: {}", self.recovery_strategy));
        debug_info.push(format!("Message: {}", self.message));
        debug_info.push(format!("Timestamp: {}", self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
        
        // Optional context information
        if let Some(ref details) = self.details {
            debug_info.push(format!("Details: {}", details));
        }
        
        if let Some(context_id) = self.context_id {
            debug_info.push(format!("Context ID: {}", context_id));
        }
        
        if let Some(correlation_id) = self.correlation_id {
            debug_info.push(format!("Correlation ID: {}", correlation_id));
        }
        
        if let Some(ref component) = self.component {
            debug_info.push(format!("Component: {}", component));
        }
        
        if let Some(ref operation) = self.operation {
            debug_info.push(format!("Operation: {}", operation));
        }
        
        format!("[Enhanced Error Debug]\n{}", debug_info.join("\n"))
    }
}

/// Conversion utilities for existing error handling patterns
pub struct ErrorConversionLayer {
    /// Shared classification engine for automatic categorization
    classification_engine: Arc<Mutex<ErrorClassificationEngine>>,
    /// Current error context for propagation
    current_context: Option<ErrorContext>,
}

impl ErrorConversionLayer {
    /// Create a new error conversion layer
    pub fn new() -> Self {
        Self {
            classification_engine: Arc::new(Mutex::new(ErrorClassificationEngine::new())),
            current_context: None,
        }
    }

    /// Set current error context for propagation
    pub fn set_context(&mut self, context: ErrorContext) {
        self.current_context = Some(context);
    }

    /// Clear current error context
    pub fn clear_context(&mut self) {
        self.current_context = None;
    }

    /// Convert EnhancedError to String for backward compatibility
    pub fn enhanced_to_string(&self, error: &EnhancedError) -> String {
        // Use the ErrorConverter trait for consistent conversion
        error.to_user_string()
    }
    
    /// Convert EnhancedError to debug string with all context
    pub fn enhanced_to_debug_string(&self, error: &EnhancedError) -> String {
        error.to_debug_string()
    }

    /// Convert String error to EnhancedError with automatic classification
    pub fn string_to_enhanced(&mut self, error_message: String, component: Option<String>, operation: Option<String>) -> EnhancedError {
        let mut engine = self.classification_engine.lock().unwrap();
        let mut enhanced_error = engine.create_enhanced_error(error_message, component, operation);
        
        // Associate with current context if available
        if let Some(ref context) = self.current_context {
            enhanced_error = enhanced_error
                .with_context_id(context.context_id)
                .with_correlation_id(context.correlation_id.unwrap_or_else(|| Uuid::new_v4()));
        }
        
        enhanced_error
    }

    /// Convert anyhow::Error to EnhancedError
    pub fn anyhow_to_enhanced(&mut self, error: AnyhowError, component: Option<String>, operation: Option<String>) -> EnhancedError {
        self.string_to_enhanced(error.to_string(), component, operation)
    }

    /// Convert std::io::Error to EnhancedError
    pub fn io_to_enhanced(&mut self, error: std::io::Error, component: Option<String>, operation: Option<String>) -> EnhancedError {
        let severity = match error.kind() {
            std::io::ErrorKind::NotFound => ErrorSeverity::Medium,
            std::io::ErrorKind::PermissionDenied => ErrorSeverity::High,
            std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::ConnectionAborted => ErrorSeverity::High,
            std::io::ErrorKind::TimedOut => ErrorSeverity::Medium,
            std::io::ErrorKind::InvalidInput | std::io::ErrorKind::InvalidData => ErrorSeverity::Medium,
            _ => ErrorSeverity::High,
        };

        let domain = if component.as_deref() == Some("database") || operation.as_deref().map_or(false, |op| op.contains("db")) {
            ErrorDomain::Data
        } else {
            ErrorDomain::System
        };

        let recovery_strategy = match error.kind() {
            std::io::ErrorKind::TimedOut => RecoveryStrategy::AutoRecoverable,
            std::io::ErrorKind::InvalidInput => RecoveryStrategy::UserRecoverable,
            std::io::ErrorKind::PermissionDenied => RecoveryStrategy::AdminRecoverable,
            _ => RecoveryStrategy::AdminRecoverable,
        };

        let mut enhanced_error = EnhancedError::new(severity, domain, recovery_strategy, error.to_string());
        
        if let Some(comp) = component {
            enhanced_error = enhanced_error.with_component(comp);
        }
        
        if let Some(op) = operation {
            enhanced_error = enhanced_error.with_operation(op);
        }

        // Associate with current context if available
        if let Some(ref context) = self.current_context {
            enhanced_error = enhanced_error
                .with_context_id(context.context_id)
                .with_correlation_id(context.correlation_id.unwrap_or_else(|| Uuid::new_v4()));
        }
        
        enhanced_error
    }

    /// Convert rusqlite::Error to EnhancedError
    pub fn rusqlite_to_enhanced(&mut self, error: rusqlite::Error, component: Option<String>, operation: Option<String>) -> EnhancedError {
        use rusqlite::Error as SqliteError;
        
        let (severity, recovery_strategy) = match &error {
            SqliteError::SqliteFailure(_, _) => (ErrorSeverity::High, RecoveryStrategy::AdminRecoverable),
            SqliteError::QueryReturnedNoRows => (ErrorSeverity::Medium, RecoveryStrategy::UserRecoverable),
            SqliteError::InvalidColumnType(_, _, _) => (ErrorSeverity::Medium, RecoveryStrategy::AdminRecoverable),
            SqliteError::InvalidPath(_) => (ErrorSeverity::High, RecoveryStrategy::AdminRecoverable),
            SqliteError::ExecuteReturnedResults => (ErrorSeverity::Medium, RecoveryStrategy::AdminRecoverable),
            SqliteError::InvalidParameterName(_) => (ErrorSeverity::Low, RecoveryStrategy::AdminRecoverable),
            _ => (ErrorSeverity::High, RecoveryStrategy::AdminRecoverable),
        };

        let mut enhanced_error = EnhancedError::new(severity, ErrorDomain::Data, recovery_strategy, error.to_string());
        
        if let Some(comp) = component {
            enhanced_error = enhanced_error.with_component(comp);
        } else {
            enhanced_error = enhanced_error.with_component("database".to_string());
        }
        
        if let Some(op) = operation {
            enhanced_error = enhanced_error.with_operation(op);
        }

        // Associate with current context if available
        if let Some(ref context) = self.current_context {
            enhanced_error = enhanced_error
                .with_context_id(context.context_id)
                .with_correlation_id(context.correlation_id.unwrap_or_else(|| Uuid::new_v4()));
        }
        
        enhanced_error
    }

    /// Get shared classification engine for manual use
    pub fn get_classification_engine(&self) -> Arc<Mutex<ErrorClassificationEngine>> {
        Arc::clone(&self.classification_engine)
    }
}

impl Default for ErrorConversionLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Compatibility wrapper for existing Tauri commands
pub struct CompatibilityWrapper {
    conversion_layer: ErrorConversionLayer,
}

impl CompatibilityWrapper {
    /// Create a new compatibility wrapper
    pub fn new() -> Self {
        Self {
            conversion_layer: ErrorConversionLayer::new(),
        }
    }

    /// Execute a function with enhanced error handling while maintaining string return type
    pub fn execute_with_context<F, T>(&mut self, context: ErrorContext, _component: &str, _operation: &str, func: F) -> Result<T, String>
    where
        F: FnOnce() -> Result<T, String>,
    {
        // Set context for error conversion
        self.conversion_layer.set_context(context);
        
        // Execute function
        let result = func();
        
        // Clear context
        self.conversion_layer.clear_context();
        
        // Return result (errors are already strings for backward compatibility)
        result
    }

    /// Execute a function that returns anyhow::Result and convert to String result
    pub fn execute_anyhow_with_context<F, T>(&mut self, context: ErrorContext, component: &str, operation: &str, func: F) -> Result<T, String>
    where
        F: FnOnce() -> anyhow::Result<T>,
    {
        // Set context for error conversion
        self.conversion_layer.set_context(context);
        
        // Execute function and convert error
        let result = func().map_err(|e| {
            let enhanced_error = self.conversion_layer.anyhow_to_enhanced(
                e,
                Some(component.to_string()),
                Some(operation.to_string()),
            );
            self.conversion_layer.enhanced_to_string(&enhanced_error)
        });
        
        // Clear context
        self.conversion_layer.clear_context();
        
        result
    }

    /// Convert any error type to enhanced error with context
    pub fn convert_error_with_context<E>(&mut self, error: E, context: ErrorContext, component: &str, operation: &str) -> EnhancedError
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.conversion_layer.set_context(context);
        
        let enhanced_error = self.conversion_layer.string_to_enhanced(
            error.to_string(),
            Some(component.to_string()),
            Some(operation.to_string()),
        );
        
        self.conversion_layer.clear_context();
        enhanced_error
    }
}

impl Default for CompatibilityWrapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for easy conversion of existing error handling patterns
#[macro_export]
macro_rules! enhance_error {
    ($error:expr) => {
        {
            let mut conversion_layer = $crate::error_handling::conversion::ErrorConversionLayer::new();
            conversion_layer.string_to_enhanced($error.to_string(), None, None)
        }
    };
    ($error:expr, $component:expr) => {
        {
            let mut conversion_layer = $crate::error_handling::conversion::ErrorConversionLayer::new();
            conversion_layer.string_to_enhanced($error.to_string(), Some($component.to_string()), None)
        }
    };
    ($error:expr, $component:expr, $operation:expr) => {
        {
            let mut conversion_layer = $crate::error_handling::conversion::ErrorConversionLayer::new();
            conversion_layer.string_to_enhanced($error.to_string(), Some($component.to_string()), Some($operation.to_string()))
        }
    };
}

/// Macro for converting enhanced errors back to strings for backward compatibility
#[macro_export]
macro_rules! to_string_error {
    ($enhanced_error:expr) => {
        {
            let conversion_layer = $crate::error_handling::conversion::ErrorConversionLayer::new();
            conversion_layer.enhanced_to_string(&$enhanced_error)
        }
    };
}

/// Macro for creating Tauri command wrappers that maintain backward compatibility
/// while adding enhanced error handling capabilities
#[macro_export]
macro_rules! tauri_command_wrapper {
    (
        $(#[$attr:meta])*
        $vis:vis async fn $name:ident($($param:ident: $param_type:ty),*) -> Result<$ok_type:ty, String> {
            $($body:tt)*
        }
    ) => {
        $(#[$attr])*
        $vis async fn $name($($param: $param_type),*) -> Result<$ok_type, String> {
            use $crate::error_handling::conversion::{ErrorConverter, ErrorConversionLayer};
            
            // Create a closure for the main logic
            let result_fn = || -> Result<$ok_type, String> {
                $($body)*
            };
            
            // Execute and handle any enhanced errors transparently
            match result_fn() {
                Ok(value) => Ok(value),
                Err(error_string) => {
                    // Enhanced error information could be logged here for internal tracking
                    // while returning the string for backward compatibility
                    Err(error_string)
                }
            }
        }
    };
    
    (
        $(#[$attr:meta])*
        $vis:vis fn $name:ident($($param:ident: $param_type:ty),*) -> Result<$ok_type:ty, String> {
            $($body:tt)*
        }
    ) => {
        $(#[$attr])*
        $vis fn $name($($param: $param_type),*) -> Result<$ok_type, String> {
            use $crate::error_handling::conversion::{ErrorConverter, ErrorConversionLayer};
            
            // Create a closure for the main logic
            let result_fn = || -> Result<$ok_type, String> {
                $($body)*
            };
            
            // Execute and handle any enhanced errors transparently
            match result_fn() {
                Ok(value) => Ok(value),
                Err(error_string) => {
                    // Enhanced error information could be logged here for internal tracking
                    // while returning the string for backward compatibility
                    Err(error_string)
                }
            }
        }
    };
}

/// Macro for wrapping existing error handling with enhanced error conversion
#[macro_export]
macro_rules! with_enhanced_error_context {
    ($component:expr, $operation:expr, $body:block) => {
        {
            use $crate::error_handling::conversion::ErrorConversionLayer;
            use $crate::error_handling::context::ErrorContext;
            
            let mut conversion_layer = ErrorConversionLayer::new();
            let context = ErrorContext::new($operation.to_string(), $component.to_string());
            conversion_layer.set_context(context);
            
            let result = (|| $body)();
            
            conversion_layer.clear_context();
            result
        }
    };
}

/// Compatibility wrapper for automatically converting anyhow::Result to Result<T, String>
#[macro_export]
macro_rules! anyhow_to_string_result {
    ($result:expr) => {
        {
            use $crate::error_handling::conversion::ErrorConversionLayer;
            let mut conversion_layer = ErrorConversionLayer::new();
            $result.map_err(|e| {
                let enhanced_error = conversion_layer.anyhow_to_enhanced(e, None, None);
                conversion_layer.enhanced_to_string(&enhanced_error)
            })
        }
    };
    ($result:expr, $component:expr) => {
        {
            use $crate::error_handling::conversion::ErrorConversionLayer;
            let mut conversion_layer = ErrorConversionLayer::new();
            $result.map_err(|e| {
                let enhanced_error = conversion_layer.anyhow_to_enhanced(e, Some($component.to_string()), None);
                conversion_layer.enhanced_to_string(&enhanced_error)
            })
        }
    };
    ($result:expr, $component:expr, $operation:expr) => {
        {
            use $crate::error_handling::conversion::ErrorConversionLayer;
            let mut conversion_layer = ErrorConversionLayer::new();
            $result.map_err(|e| {
                let enhanced_error = conversion_layer.anyhow_to_enhanced(e, Some($component.to_string()), Some($operation.to_string()));
                conversion_layer.enhanced_to_string(&enhanced_error)
            })
        }
    };
}

/// Feature flag system for gradual enhancement activation
#[derive(Debug, Clone)]
pub struct ErrorHandlingFeatureFlags {
    /// Enable enhanced error classification
    pub enable_classification: bool,
    /// Enable error context tracking
    pub enable_context_tracking: bool,
    /// Enable cross-layer correlation
    pub enable_correlation: bool,
    /// Enable database storage of error data
    pub enable_database_storage: bool,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
}

impl Default for ErrorHandlingFeatureFlags {
    fn default() -> Self {
        Self {
            enable_classification: true,
            enable_context_tracking: true,
            enable_correlation: true,
            enable_database_storage: true,
            enable_performance_monitoring: true,
        }
    }
}

impl ErrorHandlingFeatureFlags {
    /// Create minimal feature flags for gradual rollout
    pub fn minimal() -> Self {
        Self {
            enable_classification: true,
            enable_context_tracking: false,
            enable_correlation: false,
            enable_database_storage: false,
            enable_performance_monitoring: false,
        }
    }

    /// Create full feature flags for complete enhancement
    pub fn full() -> Self {
        Self::default()
    }

    /// Check if enhanced error handling is fully enabled
    pub fn is_fully_enabled(&self) -> bool {
        self.enable_classification &&
        self.enable_context_tracking &&
        self.enable_correlation &&
        self.enable_database_storage &&
        self.enable_performance_monitoring
    }
}

/// Enhanced error handling manager with feature flags
pub struct ErrorHandlingManager {
    /// Feature flags for controlling enhancement activation
    pub flags: ErrorHandlingFeatureFlags,
    /// Conversion layer for backward compatibility
    conversion_layer: ErrorConversionLayer,
    /// Compatibility wrapper for existing commands
    compatibility_wrapper: CompatibilityWrapper,
}

impl ErrorHandlingManager {
    /// Create new error handling manager with specified feature flags
    pub fn new(flags: ErrorHandlingFeatureFlags) -> Self {
        Self {
            flags,
            conversion_layer: ErrorConversionLayer::new(),
            compatibility_wrapper: CompatibilityWrapper::new(),
        }
    }

    /// Get mutable reference to conversion layer
    pub fn conversion_layer_mut(&mut self) -> &mut ErrorConversionLayer {
        &mut self.conversion_layer
    }

    /// Get mutable reference to compatibility wrapper
    pub fn compatibility_wrapper_mut(&mut self) -> &mut CompatibilityWrapper {
        &mut self.compatibility_wrapper
    }

    /// Check if specific feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "classification" => self.flags.enable_classification,
            "context_tracking" => self.flags.enable_context_tracking,
            "correlation" => self.flags.enable_correlation,
            "database_storage" => self.flags.enable_database_storage,
            "performance_monitoring" => self.flags.enable_performance_monitoring,
            _ => false,
        }
    }
}

impl Default for ErrorHandlingManager {
    fn default() -> Self {
        Self::new(ErrorHandlingFeatureFlags::default())
    }
}

/// Enhanced Error Interface - Provides access to detailed error information
/// This trait allows code to optionally access enhanced error details while
/// maintaining compatibility with existing string-based error handling
pub trait EnhancedErrorInterface {
    /// Get unique error identifier
    fn get_error_id(&self) -> Option<Uuid>;
    
    /// Get error severity level
    fn get_severity(&self) -> Option<ErrorSeverity>;
    
    /// Get error domain classification
    fn get_domain(&self) -> Option<ErrorDomain>;
    
    /// Get recovery strategy
    fn get_recovery_strategy(&self) -> Option<RecoveryStrategy>;
    
    /// Get error context ID
    fn get_context_id(&self) -> Option<Uuid>;
    
    /// Get correlation ID for tracking
    fn get_correlation_id(&self) -> Option<Uuid>;
    
    /// Get component where error originated
    fn get_component(&self) -> Option<String>;
    
    /// Get operation that was being performed
    fn get_operation(&self) -> Option<String>;
    
    /// Get detailed error description
    fn get_details(&self) -> Option<String>;
    
    /// Get error timestamp
    fn get_timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>>;
    
    /// Check if error is critical
    fn is_critical(&self) -> bool;
    
    /// Check if error is auto-recoverable
    fn is_auto_recoverable(&self) -> bool;
    
    /// Get user-friendly message
    fn get_user_message(&self) -> String;
    
    /// Get debug message with all context
    fn get_debug_message(&self) -> String;
}

/// Implementation of EnhancedErrorInterface for EnhancedError
impl EnhancedErrorInterface for EnhancedError {
    fn get_error_id(&self) -> Option<Uuid> {
        Some(self.id)
    }
    
    fn get_severity(&self) -> Option<ErrorSeverity> {
        Some(self.severity)
    }
    
    fn get_domain(&self) -> Option<ErrorDomain> {
        Some(self.domain)
    }
    
    fn get_recovery_strategy(&self) -> Option<RecoveryStrategy> {
        Some(self.recovery_strategy)
    }
    
    fn get_context_id(&self) -> Option<Uuid> {
        self.context_id
    }
    
    fn get_correlation_id(&self) -> Option<Uuid> {
        self.correlation_id
    }
    
    fn get_component(&self) -> Option<String> {
        self.component.clone()
    }
    
    fn get_operation(&self) -> Option<String> {
        self.operation.clone()
    }
    
    fn get_details(&self) -> Option<String> {
        self.details.clone()
    }
    
    fn get_timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        Some(self.timestamp)
    }
    
    fn is_critical(&self) -> bool {
        self.severity == ErrorSeverity::Critical
    }
    
    fn is_auto_recoverable(&self) -> bool {
        self.recovery_strategy == RecoveryStrategy::AutoRecoverable
    }
    
    fn get_user_message(&self) -> String {
        self.to_user_string()
    }
    
    fn get_debug_message(&self) -> String {
        self.to_debug_string()
    }
}

/// Enhanced error access wrapper for optional enhancement
/// This struct wraps any error type and provides enhanced access when available
pub struct OptionalEnhancedErrorWrapper<T> {
    error: T,
    enhanced_info: Option<EnhancedError>,
}

impl<T> OptionalEnhancedErrorWrapper<T> {
    /// Create a new wrapper without enhanced information
    pub fn from_basic(error: T) -> Self {
        Self {
            error,
            enhanced_info: None,
        }
    }
    
    /// Create a new wrapper with enhanced information
    pub fn from_enhanced(error: T, enhanced_error: EnhancedError) -> Self {
        Self {
            error,
            enhanced_info: Some(enhanced_error),
        }
    }
    
    /// Get the underlying error
    pub fn get_basic_error(&self) -> &T {
        &self.error
    }
    
    /// Check if enhanced information is available
    pub fn has_enhanced_info(&self) -> bool {
        self.enhanced_info.is_some()
    }
    
    /// Get enhanced error information if available
    pub fn get_enhanced_info(&self) -> Option<&EnhancedError> {
        self.enhanced_info.as_ref()
    }
}

impl<T> EnhancedErrorInterface for OptionalEnhancedErrorWrapper<T> 
where
    T: std::fmt::Display,
{
    fn get_error_id(&self) -> Option<Uuid> {
        self.enhanced_info.as_ref().map(|e| e.id)
    }
    
    fn get_severity(&self) -> Option<ErrorSeverity> {
        self.enhanced_info.as_ref().map(|e| e.severity)
    }
    
    fn get_domain(&self) -> Option<ErrorDomain> {
        self.enhanced_info.as_ref().map(|e| e.domain)
    }
    
    fn get_recovery_strategy(&self) -> Option<RecoveryStrategy> {
        self.enhanced_info.as_ref().map(|e| e.recovery_strategy)
    }
    
    fn get_context_id(&self) -> Option<Uuid> {
        self.enhanced_info.as_ref().and_then(|e| e.context_id)
    }
    
    fn get_correlation_id(&self) -> Option<Uuid> {
        self.enhanced_info.as_ref().and_then(|e| e.correlation_id)
    }
    
    fn get_component(&self) -> Option<String> {
        self.enhanced_info.as_ref().and_then(|e| e.component.clone())
    }
    
    fn get_operation(&self) -> Option<String> {
        self.enhanced_info.as_ref().and_then(|e| e.operation.clone())
    }
    
    fn get_details(&self) -> Option<String> {
        self.enhanced_info.as_ref().and_then(|e| e.details.clone())
    }
    
    fn get_timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.enhanced_info.as_ref().map(|e| e.timestamp)
    }
    
    fn is_critical(&self) -> bool {
        self.enhanced_info.as_ref().map_or(false, |e| e.is_critical())
    }
    
    fn is_auto_recoverable(&self) -> bool {
        self.enhanced_info.as_ref().map_or(false, |e| e.is_auto_recoverable())
    }
    
    fn get_user_message(&self) -> String {
        self.enhanced_info.as_ref()
            .map_or_else(|| self.error.to_string(), |e| e.to_user_string())
    }
    
    fn get_debug_message(&self) -> String {
        self.enhanced_info.as_ref()
            .map_or_else(|| self.error.to_string(), |e| e.to_debug_string())
    }
}

/// Implementation for basic String errors
impl EnhancedErrorInterface for String {
    fn get_error_id(&self) -> Option<Uuid> {
        None
    }
    
    fn get_severity(&self) -> Option<ErrorSeverity> {
        None
    }
    
    fn get_domain(&self) -> Option<ErrorDomain> {
        None
    }
    
    fn get_recovery_strategy(&self) -> Option<RecoveryStrategy> {
        None
    }
    
    fn get_context_id(&self) -> Option<Uuid> {
        None
    }
    
    fn get_correlation_id(&self) -> Option<Uuid> {
        None
    }
    
    fn get_component(&self) -> Option<String> {
        None
    }
    
    fn get_operation(&self) -> Option<String> {
        None
    }
    
    fn get_details(&self) -> Option<String> {
        None
    }
    
    fn get_timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        None
    }
    
    fn is_critical(&self) -> bool {
        let lower = self.to_lowercase();
        lower.contains("critical") || lower.contains("fatal") || lower.contains("system failure")
    }
    
    fn is_auto_recoverable(&self) -> bool {
        let lower = self.to_lowercase();
        lower.contains("timeout") || lower.contains("network") || lower.contains("retry")
    }
    
    fn get_user_message(&self) -> String {
        if self.is_critical() {
            "A critical error has occurred. Please contact support.".to_string()
        } else {
            self.clone()
        }
    }
    
    fn get_debug_message(&self) -> String {
        format!("[String Error] {}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_handling::context::ErrorContext;

    #[test]
    fn test_error_converter_trait() {
        // Test non-critical error (should return original message)
        let error = EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Auth,
            RecoveryStrategy::UserRecoverable,
            "Invalid credentials".to_string(),
        );
        
        assert_eq!(error.to_user_string(), "Invalid credentials");
        assert_eq!(error.to_compat_string(), "Invalid credentials");
        assert!(error.to_debug_string().contains("Invalid credentials"));
        assert!(error.to_debug_string().contains("Medium"));
        assert!(error.to_debug_string().contains("Auth"));
        
        // Test critical error (should return generic message)
        let critical_error = EnhancedError::new(
            ErrorSeverity::Critical,
            ErrorDomain::System,
            RecoveryStrategy::ManualRecoverable,
            "Internal system corruption detected".to_string(),
        );
        
        assert_eq!(critical_error.to_user_string(), "A critical error has occurred. Please contact support.");
        assert!(critical_error.to_debug_string().contains("Internal system corruption detected"));
        assert!(critical_error.to_debug_string().contains("Critical"));
        
        // Test security-sensitive operation filtering
        let security_error = EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::Auth,
            RecoveryStrategy::UserRecoverable,
            "Authentication failed".to_string(),
        ).with_operation("password_validation".to_string());
        
        assert_eq!(security_error.to_user_string(), "A security-related error occurred. Please try again.");
        assert!(security_error.to_debug_string().contains("password_validation"));
    }

    #[test]
    fn test_enhanced_to_string_conversion() {
        let conversion_layer = ErrorConversionLayer::new();
        
        // Test non-critical error (should return original message)
        let error = EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Auth,
            RecoveryStrategy::UserRecoverable,
            "Invalid credentials".to_string(),
        );
        
        let string_error = conversion_layer.enhanced_to_string(&error);
        assert_eq!(string_error, "Invalid credentials");
        
        // Test critical error (should return generic message)
        let critical_error = EnhancedError::new(
            ErrorSeverity::Critical,
            ErrorDomain::System,
            RecoveryStrategy::ManualRecoverable,
            "Internal system corruption detected".to_string(),
        );
        
        let string_error = conversion_layer.enhanced_to_string(&critical_error);
        assert_eq!(string_error, "A critical error has occurred. Please contact support.");
        
        // Test debug string conversion
        let debug_string = conversion_layer.enhanced_to_debug_string(&error);
        assert!(debug_string.contains("Enhanced Error Debug"));
        assert!(debug_string.contains("Invalid credentials"));
        assert!(debug_string.contains("Medium"));
    }

    #[test]
    fn test_string_to_enhanced_conversion() {
        let mut conversion_layer = ErrorConversionLayer::new();
        
        let enhanced_error = conversion_layer.string_to_enhanced(
            "Authentication failed".to_string(),
            Some("auth".to_string()),
            Some("login".to_string()),
        );
        
        assert_eq!(enhanced_error.severity, ErrorSeverity::High);
        assert_eq!(enhanced_error.domain, ErrorDomain::Auth);
        assert_eq!(enhanced_error.recovery_strategy, RecoveryStrategy::UserRecoverable);
        assert_eq!(enhanced_error.message, "Authentication failed");
        assert_eq!(enhanced_error.component, Some("auth".to_string()));
        assert_eq!(enhanced_error.operation, Some("login".to_string()));
    }

    #[test]
    fn test_io_error_conversion() {
        let mut conversion_layer = ErrorConversionLayer::new();
        
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let enhanced_error = conversion_layer.io_to_enhanced(
            io_error,
            Some("file_system".to_string()),
            Some("read_file".to_string()),
        );
        
        assert_eq!(enhanced_error.severity, ErrorSeverity::Medium);
        assert_eq!(enhanced_error.domain, ErrorDomain::System);
        assert_eq!(enhanced_error.component, Some("file_system".to_string()));
        assert_eq!(enhanced_error.operation, Some("read_file".to_string()));
    }

    #[test]
    fn test_rusqlite_error_conversion() {
        let mut conversion_layer = ErrorConversionLayer::new();
        
        let sqlite_error = rusqlite::Error::QueryReturnedNoRows;
        let enhanced_error = conversion_layer.rusqlite_to_enhanced(
            sqlite_error,
            Some("user_repository".to_string()),
            Some("find_by_id".to_string()),
        );
        
        assert_eq!(enhanced_error.severity, ErrorSeverity::Medium);
        assert_eq!(enhanced_error.domain, ErrorDomain::Data);
        assert_eq!(enhanced_error.recovery_strategy, RecoveryStrategy::UserRecoverable);
        assert_eq!(enhanced_error.component, Some("user_repository".to_string()));
        assert_eq!(enhanced_error.operation, Some("find_by_id".to_string()));
    }

    #[test]
    fn test_context_association() {
        let mut conversion_layer = ErrorConversionLayer::new();
        
        let context = ErrorContext::new("test_operation".to_string(), "TestComponent".to_string());
        let context_id = context.context_id;
        
        conversion_layer.set_context(context);
        
        let enhanced_error = conversion_layer.string_to_enhanced(
            "Test error".to_string(),
            Some("test".to_string()),
            Some("test_op".to_string()),
        );
        
        assert_eq!(enhanced_error.context_id, Some(context_id));
        assert!(enhanced_error.correlation_id.is_some());
        
        conversion_layer.clear_context();
        
        let enhanced_error2 = conversion_layer.string_to_enhanced(
            "Test error 2".to_string(),
            Some("test".to_string()),
            Some("test_op".to_string()),
        );
        
        assert!(enhanced_error2.context_id.is_none());
    }

    #[test]
    fn test_compatibility_wrapper() {
        let mut wrapper = CompatibilityWrapper::new();
        let context = ErrorContext::new("test_operation".to_string(), "TestComponent".to_string());
        
        // Test successful execution
        let result = wrapper.execute_with_context(
            context.clone(),
            "test_component",
            "test_operation",
            || Ok("success".to_string())
        );
        assert_eq!(result, Ok("success".to_string()));
        
        // Test error execution
        let result: Result<String, String> = wrapper.execute_with_context(
            context,
            "test_component",
            "test_operation",
            || Err("test error".to_string())
        );
        assert_eq!(result, Err("test error".to_string()));
    }

    #[test]
    fn test_feature_flags() {
        let minimal_flags = ErrorHandlingFeatureFlags::minimal();
        assert!(minimal_flags.enable_classification);
        assert!(!minimal_flags.enable_context_tracking);
        assert!(!minimal_flags.is_fully_enabled());
        
        let full_flags = ErrorHandlingFeatureFlags::full();
        assert!(full_flags.enable_classification);
        assert!(full_flags.enable_context_tracking);
        assert!(full_flags.is_fully_enabled());
    }

    #[test]
    fn test_error_handling_manager() {
        let mut manager = ErrorHandlingManager::new(ErrorHandlingFeatureFlags::minimal());
        
        assert!(manager.is_feature_enabled("classification"));
        assert!(!manager.is_feature_enabled("context_tracking"));
        assert!(!manager.is_feature_enabled("invalid_feature"));
        
        // Test access to components
        let _conversion_layer = manager.conversion_layer_mut();
        let _compatibility_wrapper = manager.compatibility_wrapper_mut();
    }

    #[test]
    fn test_enhance_error_macro() {
        let error = "Test error message";
        let enhanced = enhance_error!(error);
        
        assert_eq!(enhanced.message, "Test error message");
        assert_eq!(enhanced.severity, ErrorSeverity::Low); // Default classification
        
        let enhanced_with_component = enhance_error!(error, "test_component");
        assert_eq!(enhanced_with_component.component, Some("test_component".to_string()));
        
        let enhanced_with_operation = enhance_error!(error, "test_component", "test_operation");
        assert_eq!(enhanced_with_operation.component, Some("test_component".to_string()));
        assert_eq!(enhanced_with_operation.operation, Some("test_operation".to_string()));
    }

    #[test]
    fn test_to_string_error_macro() {
        let enhanced_error = EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::UI,
            RecoveryStrategy::UserRecoverable,
            "Validation failed".to_string(),
        );
        
        let string_error = to_string_error!(enhanced_error);
        assert_eq!(string_error, "Validation failed");
    }

    #[test]
    fn test_anyhow_to_string_result_macro() {
        use anyhow::{anyhow, Result as AnyhowResult};
        
        // Test basic conversion
        let ok_result: AnyhowResult<String> = Ok("success".to_string());
        let converted = anyhow_to_string_result!(ok_result);
        assert_eq!(converted, Ok("success".to_string()));
        
        // Test error conversion
        let err_result: AnyhowResult<String> = Err(anyhow!("Test error"));
        let converted = anyhow_to_string_result!(err_result);
        assert!(converted.is_err());
        assert!(converted.unwrap_err().contains("Test error"));
        
        // Test with component
        let err_result: AnyhowResult<String> = Err(anyhow!("Component error"));
        let converted = anyhow_to_string_result!(err_result, "test_component");
        assert!(converted.is_err());
        assert!(converted.unwrap_err().contains("Component error"));
        
        // Test with component and operation
        let err_result: AnyhowResult<String> = Err(anyhow!("Operation error"));
        let converted = anyhow_to_string_result!(err_result, "test_component", "test_operation");
        assert!(converted.is_err());
        assert!(converted.unwrap_err().contains("Operation error"));
    }

    #[test]
    fn test_with_enhanced_error_context_macro() {
        let result = with_enhanced_error_context!("test_component", "test_operation", {
            Ok::<String, String>("success".to_string())
        });
        assert_eq!(result, Ok("success".to_string()));
        
        let result = with_enhanced_error_context!("test_component", "test_operation", {
            Err::<String, String>("test_error".to_string())
        });
        assert_eq!(result, Err("test_error".to_string()));
    }

    #[test]
    fn test_enhanced_error_interface() {
        let enhanced_error = EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::Auth,
            RecoveryStrategy::UserRecoverable,
            "Authentication failed".to_string(),
        )
        .with_details("Invalid credentials provided".to_string())
        .with_component("auth_service".to_string())
        .with_operation("login".to_string());

        // Test EnhancedErrorInterface implementation
        assert!(enhanced_error.get_error_id().is_some());
        assert_eq!(enhanced_error.get_severity(), Some(ErrorSeverity::High));
        assert_eq!(enhanced_error.get_domain(), Some(ErrorDomain::Auth));
        assert_eq!(enhanced_error.get_recovery_strategy(), Some(RecoveryStrategy::UserRecoverable));
        assert_eq!(enhanced_error.get_component(), Some("auth_service".to_string()));
        assert_eq!(enhanced_error.get_operation(), Some("login".to_string()));
        assert_eq!(enhanced_error.get_details(), Some("Invalid credentials provided".to_string()));
        assert!(enhanced_error.get_timestamp().is_some());
        assert!(!enhanced_error.is_critical());
        assert!(!enhanced_error.is_auto_recoverable());
        assert_eq!(enhanced_error.get_user_message(), "Authentication failed");
        assert!(enhanced_error.get_debug_message().contains("Enhanced Error Debug"));
    }

    #[test]
    fn test_string_enhanced_error_interface() {
        let error = "Critical system failure detected".to_string();
        
        // Test EnhancedErrorInterface implementation for String
        assert!(error.get_error_id().is_none());
        assert!(error.get_severity().is_none());
        assert!(error.get_domain().is_none());
        assert!(error.get_recovery_strategy().is_none());
        assert!(error.get_context_id().is_none());
        assert!(error.get_correlation_id().is_none());
        assert!(error.get_component().is_none());
        assert!(error.get_operation().is_none());
        assert!(error.get_details().is_none());
        assert!(error.get_timestamp().is_none());
        assert!(error.is_critical());
        assert!(!error.is_auto_recoverable());
        assert_eq!(error.get_user_message(), "A critical error has occurred. Please contact support.");
        assert!(error.get_debug_message().contains("[String Error]"));
    }

    #[test]
    fn test_optional_enhanced_error_wrapper() {
        // Test wrapper without enhanced info
        let basic_wrapper = OptionalEnhancedErrorWrapper::from_basic("Basic error".to_string());
        assert!(!basic_wrapper.has_enhanced_info());
        assert!(basic_wrapper.get_enhanced_info().is_none());
        assert_eq!(basic_wrapper.get_basic_error(), "Basic error");
        assert!(basic_wrapper.get_error_id().is_none());
        assert_eq!(basic_wrapper.get_user_message(), "Basic error");
        
        // Test wrapper with enhanced info
        let enhanced_error = EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::UserRecoverable,
            "Database error".to_string(),
        );
        
        let enhanced_wrapper = OptionalEnhancedErrorWrapper::from_enhanced(
            "Database error".to_string(),
            enhanced_error,
        );
        
        assert!(enhanced_wrapper.has_enhanced_info());
        assert!(enhanced_wrapper.get_enhanced_info().is_some());
        assert_eq!(enhanced_wrapper.get_basic_error(), "Database error");
        assert!(enhanced_wrapper.get_error_id().is_some());
        assert_eq!(enhanced_wrapper.get_severity(), Some(ErrorSeverity::Medium));
        assert_eq!(enhanced_wrapper.get_domain(), Some(ErrorDomain::Data));
        assert_eq!(enhanced_wrapper.get_user_message(), "Database error");
    }

    #[test]
    fn test_string_error_auto_recovery_detection() {
        let timeout_error = "Network timeout occurred".to_string();
        let connection_error = "Connection failed, please retry".to_string();
        let invalid_error = "Invalid file format".to_string();
        
        assert!(timeout_error.is_auto_recoverable());
        assert!(connection_error.is_auto_recoverable());
        assert!(!invalid_error.is_auto_recoverable());
    }
}