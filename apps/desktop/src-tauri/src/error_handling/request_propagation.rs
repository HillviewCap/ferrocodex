use uuid::Uuid;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::error_handling::{ErrorContext, RequestIdManager};

/// Request propagation utilities for cross-layer tracking
/// Enables request ID correlation from Frontend → Tauri → Backend → Database

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPropagationContext {
    /// Original request ID from frontend
    pub request_id: Uuid,
    /// Current layer (frontend, tauri, backend, database)
    pub current_layer: String,
    /// Operation being performed
    pub operation: String,
    /// Component processing the request
    pub component: String,
    /// User ID if available
    pub user_id: Option<i64>,
    /// Session information
    pub session_id: Option<String>,
    /// Request metadata
    pub metadata: HashMap<String, String>,
    /// Timestamp when request entered this layer
    pub layer_entry_time: DateTime<Utc>,
    /// Parent context for nested operations
    pub parent_context: Option<Box<RequestPropagationContext>>,
}

impl RequestPropagationContext {
    /// Create a new request propagation context
    pub fn new(
        request_id: Uuid,
        operation: String,
        component: String,
        layer: String,
    ) -> Self {
        Self {
            request_id,
            current_layer: layer,
            operation,
            component,
            user_id: None,
            session_id: None,
            metadata: HashMap::new(),
            layer_entry_time: Utc::now(),
            parent_context: None,
        }
    }

    /// Set user context
    pub fn with_user(mut self, user_id: i64, session_id: Option<String>) -> Self {
        self.user_id = Some(user_id);
        self.session_id = session_id;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Create child context for nested operations
    pub fn create_child(&self, operation: String, component: String, layer: String) -> Self {
        Self {
            request_id: self.request_id, // Preserve original request ID
            current_layer: layer,
            operation,
            component,
            user_id: self.user_id,
            session_id: self.session_id.clone(),
            metadata: self.metadata.clone(), // Inherit parent metadata
            layer_entry_time: Utc::now(),
            parent_context: Some(Box::new(self.clone())),
        }
    }

    /// Convert to error context for error handling integration
    pub fn to_error_context(&self) -> ErrorContext {
        ErrorContext::new(self.operation.clone(), self.component.clone())
            .with_correlation_id(self.request_id)
            .with_metadata(self.metadata.clone())
            .add_metadata("layer".to_string(), self.current_layer.clone())
            .add_metadata("request_id".to_string(), self.request_id.to_string())
    }
}

/// Request propagation manager for tracking requests across layers
pub struct RequestPropagationManager {
    active_requests: Arc<Mutex<HashMap<Uuid, RequestPropagationContext>>>,
    request_id_manager: RequestIdManager,
}

impl RequestPropagationManager {
    /// Create a new request propagation manager
    pub fn new() -> Self {
        Self {
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            request_id_manager: RequestIdManager::new(),
        }
    }

    /// Start tracking a request
    pub fn start_request(
        &mut self,
        request_id: Option<Uuid>,
        operation: String,
        component: String,
        layer: String,
    ) -> Result<RequestPropagationContext, String> {
        let id = match request_id {
            Some(id) => {
                // Use provided request ID (from frontend)
                self.request_id_manager.set_current_request_id(id);
                id
            }
            None => {
                // Generate new request ID (for server-initiated operations)
                self.request_id_manager.generate_request_id()
            }
        };

        let context = RequestPropagationContext::new(id, operation, component, layer);
        
        let mut active_requests = self.active_requests.lock()
            .map_err(|_| "Failed to acquire request tracking lock".to_string())?;
        
        active_requests.insert(id, context.clone());
        Ok(context)
    }

    /// Get request context by ID
    pub fn get_request(&self, request_id: Uuid) -> Result<Option<RequestPropagationContext>, String> {
        let active_requests = self.active_requests.lock()
            .map_err(|_| "Failed to acquire request tracking lock".to_string())?;
        
        Ok(active_requests.get(&request_id).cloned())
    }

    /// Update request context
    pub fn update_request(
        &self,
        request_id: Uuid,
        updates: impl Fn(&mut RequestPropagationContext),
    ) -> Result<(), String> {
        let mut active_requests = self.active_requests.lock()
            .map_err(|_| "Failed to acquire request tracking lock".to_string())?;
        
        if let Some(context) = active_requests.get_mut(&request_id) {
            updates(context);
        }
        
        Ok(())
    }

    /// Complete request tracking
    pub fn complete_request(&self, request_id: Uuid) -> Result<bool, String> {
        let mut active_requests = self.active_requests.lock()
            .map_err(|_| "Failed to acquire request tracking lock".to_string())?;
        
        Ok(active_requests.remove(&request_id).is_some())
    }

    /// Get all active requests (for debugging/monitoring)
    pub fn get_active_requests(&self) -> Result<Vec<RequestPropagationContext>, String> {
        let active_requests = self.active_requests.lock()
            .map_err(|_| "Failed to acquire request tracking lock".to_string())?;
        
        Ok(active_requests.values().cloned().collect())
    }

    /// Clean up old requests (older than specified minutes)
    pub fn cleanup_old_requests(&self, max_age_minutes: i64) -> Result<usize, String> {
        let mut active_requests = self.active_requests.lock()
            .map_err(|_| "Failed to acquire request tracking lock".to_string())?;
        
        let cutoff = Utc::now() - chrono::Duration::minutes(max_age_minutes);
        let mut removed_count = 0;
        
        active_requests.retain(|_, context| {
            if context.layer_entry_time < cutoff {
                removed_count += 1;
                false
            } else {
                true
            }
        });
        
        Ok(removed_count)
    }
}

impl Default for RequestPropagationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global request propagation manager instance
static mut GLOBAL_PROPAGATION_MANAGER: Option<RequestPropagationManager> = None;
static MANAGER_INIT: std::sync::Once = std::sync::Once::new();

/// Get global request propagation manager
pub fn get_global_propagation_manager() -> &'static mut RequestPropagationManager {
    unsafe {
        MANAGER_INIT.call_once(|| {
            GLOBAL_PROPAGATION_MANAGER = Some(RequestPropagationManager::new());
        });
        GLOBAL_PROPAGATION_MANAGER.as_mut().unwrap()
    }
}

/// Utility macros for request propagation in Tauri commands

/// Extract request ID from command arguments
pub fn extract_request_id(args: &serde_json::Value) -> Option<Uuid> {
    if let Some(request_id_value) = args.get("request_id") {
        if let Some(request_id_str) = request_id_value.as_str() {
            return Uuid::parse_str(request_id_str).ok();
        }
    }
    None
}

/// Create request context for Tauri command
pub fn create_tauri_request_context(
    request_id: Option<Uuid>,
    command_name: &str,
    component: &str,
) -> Result<RequestPropagationContext, String> {
    let manager = get_global_propagation_manager();
    manager.start_request(
        request_id,
        command_name.to_string(),
        component.to_string(),
        "tauri".to_string(),
    )
}

/// Complete request tracking for Tauri command
pub fn complete_tauri_request(request_id: Uuid) -> Result<bool, String> {
    let manager = get_global_propagation_manager();
    manager.complete_request(request_id)
}

/// Propagate request to backend operation
pub fn propagate_to_backend(
    parent_context: &RequestPropagationContext,
    operation: String,
    component: String,
) -> RequestPropagationContext {
    parent_context.create_child(operation, component, "backend".to_string())
}

/// Propagate request to database operation
pub fn propagate_to_database(
    parent_context: &RequestPropagationContext,
    operation: String,
    component: String,
) -> RequestPropagationContext {
    parent_context.create_child(operation, component, "database".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_propagation_context_creation() {
        let request_id = Uuid::new_v4();
        let context = RequestPropagationContext::new(
            request_id,
            "test_operation".to_string(),
            "TestComponent".to_string(),
            "tauri".to_string(),
        );

        assert_eq!(context.request_id, request_id);
        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.component, "TestComponent");
        assert_eq!(context.current_layer, "tauri");
        assert!(context.user_id.is_none());
        assert!(context.metadata.is_empty());
    }

    #[test]
    fn test_request_propagation_context_builder() {
        let request_id = Uuid::new_v4();
        let context = RequestPropagationContext::new(
            request_id,
            "test_op".to_string(),
            "TestComp".to_string(),
            "tauri".to_string(),
        )
        .with_user(123, Some("session123".to_string()))
        .with_metadata("key1".to_string(), "value1".to_string());

        assert_eq!(context.user_id, Some(123));
        assert_eq!(context.session_id, Some("session123".to_string()));
        assert_eq!(context.metadata.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_child_context_creation() {
        let parent_request_id = Uuid::new_v4();
        let parent_context = RequestPropagationContext::new(
            parent_request_id,
            "parent_op".to_string(),
            "ParentComp".to_string(),
            "tauri".to_string(),
        )
        .with_user(456, Some("session456".to_string()))
        .with_metadata("inherited".to_string(), "value".to_string());

        let child_context = parent_context.create_child(
            "child_op".to_string(),
            "ChildComp".to_string(),
            "backend".to_string(),
        );

        assert_eq!(child_context.request_id, parent_request_id); // Same request ID
        assert_eq!(child_context.operation, "child_op");
        assert_eq!(child_context.component, "ChildComp");
        assert_eq!(child_context.current_layer, "backend");
        assert_eq!(child_context.user_id, Some(456)); // Inherited
        assert_eq!(child_context.session_id, Some("session456".to_string())); // Inherited
        assert_eq!(child_context.metadata.get("inherited"), Some(&"value".to_string())); // Inherited
        assert!(child_context.parent_context.is_some());
    }

    #[test]
    fn test_request_propagation_manager() {
        let mut manager = RequestPropagationManager::new();
        let request_id = Uuid::new_v4();

        // Start request
        let context = manager.start_request(
            Some(request_id),
            "test_operation".to_string(),
            "TestComponent".to_string(),
            "tauri".to_string(),
        ).unwrap();

        assert_eq!(context.request_id, request_id);

        // Get request
        let retrieved_context = manager.get_request(request_id).unwrap().unwrap();
        assert_eq!(retrieved_context.request_id, request_id);
        assert_eq!(retrieved_context.operation, "test_operation");

        // Complete request
        let completed = manager.complete_request(request_id).unwrap();
        assert!(completed);

        // Request should no longer exist
        let retrieved_after_completion = manager.get_request(request_id).unwrap();
        assert!(retrieved_after_completion.is_none());
    }

    #[test]
    fn test_extract_request_id() {
        let args_with_request_id = serde_json::json!({
            "request_id": "12345678-1234-4234-8234-123456789012",
            "other_param": "value"
        });

        let request_id = extract_request_id(&args_with_request_id);
        assert!(request_id.is_some());

        let args_without_request_id = serde_json::json!({
            "other_param": "value"
        });

        let no_request_id = extract_request_id(&args_without_request_id);
        assert!(no_request_id.is_none());
    }

    #[test]
    fn test_error_context_conversion() {
        let request_id = Uuid::new_v4();
        let context = RequestPropagationContext::new(
            request_id,
            "test_operation".to_string(),
            "TestComponent".to_string(),
            "tauri".to_string(),
        )
        .with_metadata("test_key".to_string(), "test_value".to_string());

        let error_context = context.to_error_context();
        
        assert_eq!(error_context.operation, "test_operation");
        assert_eq!(error_context.component, "TestComponent");
        assert_eq!(error_context.correlation_id, Some(request_id));
        assert_eq!(error_context.get_metadata("layer"), Some(&"tauri".to_string()));
        assert_eq!(error_context.get_metadata("request_id"), Some(&request_id.to_string()));
        assert_eq!(error_context.get_metadata("test_key"), Some(&"test_value".to_string()));
    }
}