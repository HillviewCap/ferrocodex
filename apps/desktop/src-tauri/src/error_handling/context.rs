use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

/// Comprehensive error context with request IDs, user identification, and operation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Unique context identifier
    pub context_id: Uuid,
    /// Request identifier for tracking operations
    pub request_id: Uuid,
    /// User ID associated with the operation (if applicable)
    pub user_id: Option<i64>,
    /// Operation that was being performed
    pub operation: String,
    /// Component where the operation was initiated
    pub component: String,
    /// Additional context data as key-value pairs
    pub metadata: HashMap<String, String>,
    /// Timestamp when context was created
    pub created_at: DateTime<Utc>,
    /// Optional correlation ID for cross-layer tracking
    pub correlation_id: Option<Uuid>,
    /// Session information if available
    pub session_info: Option<SessionInfo>,
    /// Request path or route information
    pub request_path: Option<String>,
    /// Client information (user agent, IP, etc.)
    pub client_info: Option<ClientInfo>,
}

/// Session information for error context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session token identifier
    pub session_id: String,
    /// Username associated with session
    pub username: String,
    /// User role
    pub role: String,
    /// Session start time
    pub session_start: DateTime<Utc>,
}

/// Client information for error context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// User agent string
    pub user_agent: Option<String>,
    /// Client IP address
    pub ip_address: Option<String>,
    /// Platform information
    pub platform: Option<String>,
}

impl ErrorContext {
    /// Create a new error context with required fields
    pub fn new(operation: String, component: String) -> Self {
        Self {
            context_id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            user_id: None,
            operation,
            component,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            correlation_id: None,
            session_info: None,
            request_path: None,
            client_info: None,
        }
    }

    /// Set user ID for the context
    pub fn with_user_id(mut self, user_id: i64) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set correlation ID for cross-layer tracking
    pub fn with_correlation_id(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Set session information
    pub fn with_session_info(mut self, session_info: SessionInfo) -> Self {
        self.session_info = Some(session_info);
        self
    }

    /// Set request path
    pub fn with_request_path(mut self, request_path: String) -> Self {
        self.request_path = Some(request_path);
        self
    }

    /// Set client information
    pub fn with_client_info(mut self, client_info: ClientInfo) -> Self {
        self.client_info = Some(client_info);
        self
    }

    /// Add metadata key-value pair
    pub fn add_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add multiple metadata entries
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata.extend(metadata);
        self
    }

    /// Get metadata value by key
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Create a child context for nested operations
    pub fn create_child_context(&self, operation: String, component: String) -> Self {
        Self {
            context_id: Uuid::new_v4(),
            request_id: self.request_id, // Inherit request ID
            user_id: self.user_id,
            operation,
            component,
            metadata: self.metadata.clone(), // Inherit metadata
            created_at: Utc::now(),
            correlation_id: self.correlation_id,
            session_info: self.session_info.clone(),
            request_path: self.request_path.clone(),
            client_info: self.client_info.clone(),
        }
    }
}

/// Context correlation system for cross-layer tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCorrelation {
    /// Correlation identifier
    pub correlation_id: Uuid,
    /// Layer where context was created (frontend, backend, database)
    pub layer: String,
    /// Component within the layer
    pub component: String,
    /// Associated error context ID
    pub context_id: Uuid,
    /// Parent correlation ID for nested operations
    pub parent_correlation_id: Option<Uuid>,
    /// Timestamp when correlation was created
    pub created_at: DateTime<Utc>,
    /// Additional correlation metadata
    pub metadata: HashMap<String, String>,
}

impl ContextCorrelation {
    /// Create a new context correlation
    pub fn new(layer: String, component: String, context_id: Uuid) -> Self {
        Self {
            correlation_id: Uuid::new_v4(),
            layer,
            component,
            context_id,
            parent_correlation_id: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Set parent correlation ID for nested operations
    pub fn with_parent(mut self, parent_correlation_id: Uuid) -> Self {
        self.parent_correlation_id = Some(parent_correlation_id);
        self
    }

    /// Add metadata to correlation
    pub fn add_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Request ID generation and propagation system
pub struct RequestIdManager {
    current_request_id: Option<Uuid>,
}

impl RequestIdManager {
    /// Create a new request ID manager
    pub fn new() -> Self {
        Self {
            current_request_id: None,
        }
    }

    /// Generate a new request ID
    pub fn generate_request_id(&mut self) -> Uuid {
        let request_id = Uuid::new_v4();
        self.current_request_id = Some(request_id);
        request_id
    }

    /// Get current request ID
    pub fn get_current_request_id(&self) -> Option<Uuid> {
        self.current_request_id
    }

    /// Set current request ID (for propagation)
    pub fn set_current_request_id(&mut self, request_id: Uuid) {
        self.current_request_id = Some(request_id);
    }

    /// Clear current request ID
    pub fn clear_request_id(&mut self) {
        self.current_request_id = None;
    }
}

impl Default for RequestIdManager {
    fn default() -> Self {
        Self::new()
    }
}

/// High-performance context creation utilities with <1ms requirement
pub struct ContextFactory {
    /// Pre-allocated request ID manager
    request_id_manager: RequestIdManager,
    /// Performance tracking
    creation_times: Vec<u128>, // microseconds
}

impl ContextFactory {
    /// Create a new context factory
    pub fn new() -> Self {
        Self {
            request_id_manager: RequestIdManager::new(),
            creation_times: Vec::new(),
        }
    }

    /// Create error context with performance tracking
    pub fn create_context(&mut self, operation: String, component: String) -> ErrorContext {
        let start = Instant::now();
        
        let request_id = self.request_id_manager.generate_request_id();
        let context = ErrorContext {
            context_id: Uuid::new_v4(),
            request_id,
            user_id: None,
            operation,
            component,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            correlation_id: None,
            session_info: None,
            request_path: None,
            client_info: None,
        };
        
        let elapsed = start.elapsed().as_micros();
        self.creation_times.push(elapsed);
        
        // Keep only last 1000 measurements for performance monitoring
        if self.creation_times.len() > 1000 {
            self.creation_times.remove(0);
        }
        
        context
    }

    /// Get average context creation time in microseconds
    pub fn get_average_creation_time(&self) -> Option<f64> {
        if self.creation_times.is_empty() {
            None
        } else {
            let sum: u128 = self.creation_times.iter().sum();
            Some(sum as f64 / self.creation_times.len() as f64)
        }
    }

    /// Check if performance requirement (<1ms) is being met
    pub fn meets_performance_requirement(&self) -> bool {
        match self.get_average_creation_time() {
            Some(avg_time) => avg_time < 1000.0, // 1000 microseconds = 1ms
            None => true, // No measurements yet, assume OK
        }
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        if self.creation_times.is_empty() {
            return PerformanceStats {
                sample_count: 0,
                average_time_us: 0.0,
                min_time_us: 0,
                max_time_us: 0,
                meets_requirement: true,
            };
        }

        let min_time = *self.creation_times.iter().min().unwrap();
        let max_time = *self.creation_times.iter().max().unwrap();
        let avg_time = self.get_average_creation_time().unwrap();

        PerformanceStats {
            sample_count: self.creation_times.len(),
            average_time_us: avg_time,
            min_time_us: min_time,
            max_time_us: max_time,
            meets_requirement: avg_time < 1000.0,
        }
    }
}

impl Default for ContextFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance statistics for context creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub sample_count: usize,
    pub average_time_us: f64,
    pub min_time_us: u128,
    pub max_time_us: u128,
    pub meets_requirement: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_creation() {
        let context = ErrorContext::new("test_operation".to_string(), "TestComponent".to_string());
        
        assert!(!context.context_id.is_nil());
        assert!(!context.request_id.is_nil());
        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.component, "TestComponent");
        assert!(context.user_id.is_none());
        assert!(context.metadata.is_empty());
    }

    #[test]
    fn test_error_context_builder_pattern() {
        let session_info = SessionInfo {
            session_id: "session123".to_string(),
            username: "testuser".to_string(),
            role: "Engineer".to_string(),
            session_start: Utc::now(),
        };

        let client_info = ClientInfo {
            user_agent: Some("TestAgent/1.0".to_string()),
            ip_address: Some("127.0.0.1".to_string()),
            platform: Some("Windows".to_string()),
        };

        let correlation_id = Uuid::new_v4();
        
        let context = ErrorContext::new("test_op".to_string(), "TestComp".to_string())
            .with_user_id(123)
            .with_correlation_id(correlation_id)
            .with_session_info(session_info.clone())
            .with_request_path("api/test".to_string())
            .with_client_info(client_info.clone())
            .add_metadata("key1".to_string(), "value1".to_string())
            .add_metadata("key2".to_string(), "value2".to_string());

        assert_eq!(context.user_id, Some(123));
        assert_eq!(context.correlation_id, Some(correlation_id));
        assert_eq!(context.session_info.as_ref().unwrap().username, "testuser");
        assert_eq!(context.request_path, Some("api/test".to_string()));
        assert_eq!(context.client_info.as_ref().unwrap().user_agent, Some("TestAgent/1.0".to_string()));
        assert_eq!(context.get_metadata("key1"), Some(&"value1".to_string()));
        assert_eq!(context.get_metadata("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_child_context_creation() {
        let parent_context = ErrorContext::new("parent_op".to_string(), "ParentComp".to_string())
            .with_user_id(456)
            .add_metadata("inherited".to_string(), "value".to_string());

        let child_context = parent_context.create_child_context(
            "child_op".to_string(),
            "ChildComp".to_string(),
        );

        assert_ne!(child_context.context_id, parent_context.context_id);
        assert_eq!(child_context.request_id, parent_context.request_id); // Inherited
        assert_eq!(child_context.user_id, Some(456)); // Inherited
        assert_eq!(child_context.operation, "child_op");
        assert_eq!(child_context.component, "ChildComp");
        assert_eq!(child_context.get_metadata("inherited"), Some(&"value".to_string())); // Inherited
    }

    #[test]
    fn test_context_correlation() {
        let context_id = Uuid::new_v4();
        let parent_correlation_id = Uuid::new_v4();
        
        let correlation = ContextCorrelation::new(
            "backend".to_string(),
            "UserService".to_string(),
            context_id,
        )
        .with_parent(parent_correlation_id)
        .add_metadata("trace_level".to_string(), "info".to_string());

        assert!(!correlation.correlation_id.is_nil());
        assert_eq!(correlation.layer, "backend");
        assert_eq!(correlation.component, "UserService");
        assert_eq!(correlation.context_id, context_id);
        assert_eq!(correlation.parent_correlation_id, Some(parent_correlation_id));
        assert_eq!(correlation.metadata.get("trace_level"), Some(&"info".to_string()));
    }

    #[test]
    fn test_request_id_manager() {
        let mut manager = RequestIdManager::new();
        
        assert!(manager.get_current_request_id().is_none());
        
        let request_id = manager.generate_request_id();
        assert_eq!(manager.get_current_request_id(), Some(request_id));
        
        let another_id = Uuid::new_v4();
        manager.set_current_request_id(another_id);
        assert_eq!(manager.get_current_request_id(), Some(another_id));
        
        manager.clear_request_id();
        assert!(manager.get_current_request_id().is_none());
    }

    #[test]
    fn test_context_factory_performance() {
        let mut factory = ContextFactory::new();
        
        // Create multiple contexts to test performance
        for i in 0..10 {
            let _context = factory.create_context(
                format!("operation_{}", i),
                "TestComponent".to_string(),
            );
        }
        
        let stats = factory.get_performance_stats();
        assert_eq!(stats.sample_count, 10);
        assert!(stats.average_time_us > 0.0);
        assert!(stats.min_time_us <= stats.max_time_us);
        
        // Performance requirement should be met for simple context creation
        assert!(factory.meets_performance_requirement());
    }

    #[test]
    fn test_context_factory_performance_monitoring() {
        let mut factory = ContextFactory::new();
        
        // Initially no measurements
        assert!(factory.get_average_creation_time().is_none());
        assert!(factory.meets_performance_requirement());
        
        // Create a context
        let _context = factory.create_context("test".to_string(), "Test".to_string());
        
        // Now we should have measurements
        assert!(factory.get_average_creation_time().is_some());
        let avg_time = factory.get_average_creation_time().unwrap();
        assert!(avg_time > 0.0);
        
        // Should meet performance requirement for simple operations
        assert!(factory.meets_performance_requirement());
    }
}