use crate::error_handling::{EnhancedError, RecoveryStrategy};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Manual recovery action type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ManualRecoveryActionType {
    /// User needs to check and correct input data
    CheckInput,
    /// User needs to verify network connectivity
    CheckNetworkConnection,
    /// User needs to verify database access
    CheckDatabaseAccess,
    /// User needs to restart the service
    RestartService,
    /// User needs to check system resources
    CheckSystemResources,
    /// User needs to verify permissions
    CheckPermissions,
    /// User needs to retry the operation manually
    ManualRetry,
    /// User needs to contact administrator
    ContactAdministrator,
    /// User needs to review configuration
    ReviewConfiguration,
    /// Custom action with specific instructions
    Custom(String),
}

impl std::fmt::Display for ManualRecoveryActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManualRecoveryActionType::CheckInput => write!(f, "Check Input Data"),
            ManualRecoveryActionType::CheckNetworkConnection => write!(f, "Check Network Connection"),
            ManualRecoveryActionType::CheckDatabaseAccess => write!(f, "Check Database Access"),
            ManualRecoveryActionType::RestartService => write!(f, "Restart Service"),
            ManualRecoveryActionType::CheckSystemResources => write!(f, "Check System Resources"),
            ManualRecoveryActionType::CheckPermissions => write!(f, "Check Permissions"),
            ManualRecoveryActionType::ManualRetry => write!(f, "Manual Retry"),
            ManualRecoveryActionType::ContactAdministrator => write!(f, "Contact Administrator"),
            ManualRecoveryActionType::ReviewConfiguration => write!(f, "Review Configuration"),
            ManualRecoveryActionType::Custom(desc) => write!(f, "{}", desc),
        }
    }
}

/// Manual recovery action with detailed guidance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualRecoveryAction {
    /// Unique identifier for this action
    pub id: Uuid,
    /// Type of manual action required
    pub action_type: ManualRecoveryActionType,
    /// Short description of the action
    pub title: String,
    /// Detailed instructions for the user
    pub instructions: Vec<String>,
    /// Priority level (1 = highest, 5 = lowest)
    pub priority: u8,
    /// Estimated time to complete (minutes)
    pub estimated_duration_minutes: Option<u16>,
    /// Prerequisites that must be met before this action
    pub prerequisites: Vec<String>,
    /// Expected outcome after completing this action
    pub expected_outcome: String,
    /// Whether this action requires admin privileges
    pub requires_admin: bool,
    /// Whether this action is destructive (needs confirmation)
    pub is_destructive: bool,
}

impl ManualRecoveryAction {
    /// Create a new manual recovery action
    pub fn new(
        action_type: ManualRecoveryActionType,
        title: String,
        instructions: Vec<String>,
        expected_outcome: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            action_type,
            title,
            instructions,
            priority: 3,
            estimated_duration_minutes: None,
            prerequisites: Vec::new(),
            expected_outcome,
            requires_admin: false,
            is_destructive: false,
        }
    }

    /// Set priority level
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.clamp(1, 5);
        self
    }

    /// Set estimated duration
    pub fn with_duration(mut self, minutes: u16) -> Self {
        self.estimated_duration_minutes = Some(minutes);
        self
    }

    /// Add prerequisites
    pub fn with_prerequisites(mut self, prerequisites: Vec<String>) -> Self {
        self.prerequisites = prerequisites;
        self
    }

    /// Mark as requiring admin privileges
    pub fn requires_admin(mut self) -> Self {
        self.requires_admin = true;
        self
    }

    /// Mark as destructive action
    pub fn is_destructive(mut self) -> Self {
        self.is_destructive = true;
        self
    }
}

/// Result of manual recovery attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualRecoveryResult {
    /// Action ID that was attempted
    pub action_id: Uuid,
    /// Whether the action was successful
    pub success: bool,
    /// User feedback about the action
    pub user_feedback: Option<String>,
    /// Time taken to complete the action
    pub duration: Duration,
    /// Whether the action resolved the original error
    pub resolved_error: bool,
    /// Additional context about the result
    pub notes: Option<String>,
}

/// Manual recovery guidance generator
pub struct ManualRecoveryGuide {
    /// Cache of generated actions for performance
    action_cache: HashMap<String, Vec<ManualRecoveryAction>>,
    /// Performance tracking
    generation_times: Vec<u128>, // microseconds
}

impl ManualRecoveryGuide {
    /// Create a new manual recovery guide
    pub fn new() -> Self {
        Self {
            action_cache: HashMap::new(),
            generation_times: Vec::new(),
        }
    }

    /// Generate manual recovery actions for an error
    pub fn generate_recovery_actions(&mut self, error: &EnhancedError) -> Vec<ManualRecoveryAction> {
        let start = Instant::now();
        
        // Create cache key from error characteristics
        let cache_key = format!("{}:{}:{}", error.domain, error.severity, error.recovery_strategy);
        
        // Check cache first
        if let Some(cached_actions) = self.action_cache.get(&cache_key) {
            return cached_actions.clone();
        }

        let mut actions = Vec::new();

        // Generate actions based on error domain and context
        match error.domain {
            crate::error_handling::ErrorDomain::Auth => {
                actions.extend(self.generate_auth_actions(error));
            }
            crate::error_handling::ErrorDomain::Data => {
                actions.extend(self.generate_data_actions(error));
            }
            crate::error_handling::ErrorDomain::Assets => {
                actions.extend(self.generate_asset_actions(error));
            }
            crate::error_handling::ErrorDomain::System => {
                actions.extend(self.generate_system_actions(error));
            }
            crate::error_handling::ErrorDomain::UI => {
                actions.extend(self.generate_ui_actions(error));
            }
        }

        // Add general recovery actions based on recovery strategy
        if error.recovery_strategy == RecoveryStrategy::UserRecoverable {
            actions.push(self.create_retry_action());
        }

        if error.recovery_strategy == RecoveryStrategy::AdminRecoverable {
            actions.push(self.create_admin_contact_action());
        }

        // Sort by priority
        actions.sort_by_key(|a| a.priority);

        // Cache the result
        self.action_cache.insert(cache_key, actions.clone());

        // Track performance
        let elapsed = start.elapsed().as_micros();
        self.generation_times.push(elapsed);
        if self.generation_times.len() > 1000 {
            self.generation_times.remove(0);
        }

        actions
    }

    /// Generate authentication-specific recovery actions
    fn generate_auth_actions(&self, error: &EnhancedError) -> Vec<ManualRecoveryAction> {
        let mut actions = Vec::new();
        let message_lower = error.message.to_lowercase();

        if message_lower.contains("invalid credentials") || message_lower.contains("authentication failed") {
            actions.push(
                ManualRecoveryAction::new(
                    ManualRecoveryActionType::CheckInput,
                    "Verify Login Credentials".to_string(),
                    vec![
                        "1. Check that your username is spelled correctly".to_string(),
                        "2. Verify that Caps Lock is not enabled".to_string(),
                        "3. Ensure you're using the correct password".to_string(),
                        "4. Try typing your password in a text editor first to verify it".to_string(),
                    ],
                    "You should be able to log in successfully".to_string(),
                )
                .with_priority(1)
                .with_duration(2)
            );

            actions.push(
                ManualRecoveryAction::new(
                    ManualRecoveryActionType::ManualRetry,
                    "Reset Password".to_string(),
                    vec![
                        "1. Contact your administrator to reset your password".to_string(),
                        "2. Use the password reset function if available".to_string(),
                        "3. Try logging in with the new password".to_string(),
                    ],
                    "You should be able to access the system with new credentials".to_string(),
                )
                .with_priority(2)
                .with_duration(10)
            );
        }

        if message_lower.contains("permission denied") || message_lower.contains("access denied") {
            actions.push(
                ManualRecoveryAction::new(
                    ManualRecoveryActionType::CheckPermissions,
                    "Verify User Permissions".to_string(),
                    vec![
                        "1. Check that your user account has the required permissions".to_string(),
                        "2. Verify that your role allows this operation".to_string(),
                        "3. Contact an administrator if you need additional permissions".to_string(),
                    ],
                    "You should have the necessary permissions to perform this operation".to_string(),
                )
                .with_priority(1)
                .with_duration(5)
                .requires_admin()
            );
        }

        actions
    }

    /// Generate data-specific recovery actions
    fn generate_data_actions(&self, error: &EnhancedError) -> Vec<ManualRecoveryAction> {
        let mut actions = Vec::new();
        let message_lower = error.message.to_lowercase();

        if message_lower.contains("database") || message_lower.contains("connection") {
            actions.push(
                ManualRecoveryAction::new(
                    ManualRecoveryActionType::CheckDatabaseAccess,
                    "Check Database Connection".to_string(),
                    vec![
                        "1. Verify that the database service is running".to_string(),
                        "2. Check network connectivity to the database server".to_string(),
                        "3. Verify database credentials and permissions".to_string(),
                        "4. Check for database maintenance or downtime".to_string(),
                    ],
                    "Database connection should be restored".to_string(),
                )
                .with_priority(1)
                .with_duration(5)
            );
        }

        if message_lower.contains("validation") || message_lower.contains("invalid") {
            actions.push(
                ManualRecoveryAction::new(
                    ManualRecoveryActionType::CheckInput,
                    "Validate Input Data".to_string(),
                    vec![
                        "1. Review all input fields for correct format".to_string(),
                        "2. Check for required fields that may be empty".to_string(),
                        "3. Verify data types and value ranges".to_string(),
                        "4. Ensure special characters are properly handled".to_string(),
                    ],
                    "Input data should pass validation requirements".to_string(),
                )
                .with_priority(1)
                .with_duration(3)
            );
        }

        actions
    }

    /// Generate asset-specific recovery actions
    fn generate_asset_actions(&self, error: &EnhancedError) -> Vec<ManualRecoveryAction> {
        let mut actions = Vec::new();
        let message_lower = error.message.to_lowercase();

        if message_lower.contains("asset") || message_lower.contains("configuration") {
            actions.push(
                ManualRecoveryAction::new(
                    ManualRecoveryActionType::ReviewConfiguration,
                    "Review Asset Configuration".to_string(),
                    vec![
                        "1. Check that the asset exists and is properly configured".to_string(),
                        "2. Verify that all required fields are filled".to_string(),
                        "3. Ensure configuration syntax is correct".to_string(),
                        "4. Check for conflicting configuration values".to_string(),
                    ],
                    "Asset should be properly configured and accessible".to_string(),
                )
                .with_priority(1)
                .with_duration(10)
            );
        }

        if message_lower.contains("firmware") {
            actions.push(
                ManualRecoveryAction::new(
                    ManualRecoveryActionType::Custom("Check Firmware".to_string()),
                    "Verify Firmware Status".to_string(),
                    vec![
                        "1. Check that the firmware file exists and is accessible".to_string(),
                        "2. Verify firmware version compatibility".to_string(),
                        "3. Ensure firmware is not corrupted".to_string(),
                        "4. Check firmware deployment status".to_string(),
                    ],
                    "Firmware should be valid and deployable".to_string(),
                )
                .with_priority(2)
                .with_duration(15)
            );
        }

        actions
    }

    /// Generate system-specific recovery actions
    fn generate_system_actions(&self, error: &EnhancedError) -> Vec<ManualRecoveryAction> {
        let mut actions = Vec::new();
        let message_lower = error.message.to_lowercase();

        if message_lower.contains("network") || message_lower.contains("connection") {
            actions.push(
                ManualRecoveryAction::new(
                    ManualRecoveryActionType::CheckNetworkConnection,
                    "Check Network Connectivity".to_string(),
                    vec![
                        "1. Verify internet connection is working".to_string(),
                        "2. Check firewall settings and port access".to_string(),
                        "3. Test connectivity to specific services".to_string(),
                        "4. Check for network configuration changes".to_string(),
                    ],
                    "Network connectivity should be restored".to_string(),
                )
                .with_priority(1)
                .with_duration(5)
            );
        }

        if message_lower.contains("resource") || message_lower.contains("memory") || message_lower.contains("disk") {
            actions.push(
                ManualRecoveryAction::new(
                    ManualRecoveryActionType::CheckSystemResources,
                    "Check System Resources".to_string(),
                    vec![
                        "1. Check available disk space".to_string(),
                        "2. Monitor memory usage and free up resources if needed".to_string(),
                        "3. Check CPU usage and running processes".to_string(),
                        "4. Restart the application if resources are critically low".to_string(),
                    ],
                    "System resources should be sufficient for operation".to_string(),
                )
                .with_priority(1)
                .with_duration(5)
                .requires_admin()
            );
        }

        if message_lower.contains("service") || message_lower.contains("unavailable") {
            actions.push(
                ManualRecoveryAction::new(
                    ManualRecoveryActionType::RestartService,
                    "Restart Services".to_string(),
                    vec![
                        "1. Stop the affected service".to_string(),
                        "2. Wait 10 seconds for complete shutdown".to_string(),
                        "3. Start the service again".to_string(),
                        "4. Verify that the service is running properly".to_string(),
                    ],
                    "Service should be running and accessible".to_string(),
                )
                .with_priority(2)
                .with_duration(3)
                .requires_admin()
                .is_destructive()
            );
        }

        actions
    }

    /// Generate UI-specific recovery actions
    fn generate_ui_actions(&self, error: &EnhancedError) -> Vec<ManualRecoveryAction> {
        let mut actions = Vec::new();

        actions.push(
            ManualRecoveryAction::new(
                ManualRecoveryActionType::ManualRetry,
                "Refresh Interface".to_string(),
                vec![
                    "1. Refresh the current page or view".to_string(),
                    "2. Clear any cached data if possible".to_string(),
                    "3. Try the operation again".to_string(),
                ],
                "Interface should display correctly and function properly".to_string(),
            )
            .with_priority(1)
            .with_duration(1)
        );

        actions
    }

    /// Create a generic retry action
    fn create_retry_action(&self) -> ManualRecoveryAction {
        ManualRecoveryAction::new(
            ManualRecoveryActionType::ManualRetry,
            "Retry Operation".to_string(),
            vec![
                "1. Wait a few moments for any temporary issues to resolve".to_string(),
                "2. Try the operation again".to_string(),
                "3. If it still fails, try alternative approaches".to_string(),
            ],
            "Operation should complete successfully".to_string(),
        )
        .with_priority(3)
        .with_duration(2)
    }

    /// Create an admin contact action
    fn create_admin_contact_action(&self) -> ManualRecoveryAction {
        ManualRecoveryAction::new(
            ManualRecoveryActionType::ContactAdministrator,
            "Contact System Administrator".to_string(),
            vec![
                "1. Document the exact error message and steps that led to it".to_string(),
                "2. Note the time when the error occurred".to_string(),
                "3. Contact your system administrator with this information".to_string(),
                "4. Provide any relevant context about what you were trying to do".to_string(),
            ],
            "Administrator should be able to resolve the issue".to_string(),
        )
        .with_priority(4)
        .with_duration(30)
        .requires_admin()
    }

    /// Record the result of a manual recovery attempt
    pub fn record_recovery_result(&mut self, result: ManualRecoveryResult) {
        // This could be expanded to learn from user feedback and improve future suggestions
        // For now, we just store it (could be persisted to database in full implementation)
        
        // In a real implementation, this would:
        // 1. Update success rates for different action types
        // 2. Learn from user feedback to improve instructions
        // 3. Adjust priority rankings based on effectiveness
        // 4. Store patterns for similar errors
        
        tracing::info!(
            "Manual recovery result recorded: action_id={}, success={}, resolved_error={}",
            result.action_id,
            result.success,
            result.resolved_error
        );
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> crate::error_handling::context::PerformanceStats {
        if self.generation_times.is_empty() {
            return crate::error_handling::context::PerformanceStats {
                sample_count: 0,
                average_time_us: 0.0,
                min_time_us: 0,
                max_time_us: 0,
                meets_requirement: true,
            };
        }

        let min_time = *self.generation_times.iter().min().unwrap();
        let max_time = *self.generation_times.iter().max().unwrap();
        let sum: u128 = self.generation_times.iter().sum();
        let avg_time = sum as f64 / self.generation_times.len() as f64;

        crate::error_handling::context::PerformanceStats {
            sample_count: self.generation_times.len(),
            average_time_us: avg_time,
            min_time_us: min_time,
            max_time_us: max_time,
            meets_requirement: avg_time < 1000.0, // 1ms requirement for guidance generation
        }
    }

    /// Clear the action cache
    pub fn clear_cache(&mut self) {
        self.action_cache.clear();
    }
}

impl Default for ManualRecoveryGuide {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_handling::{ErrorSeverity, ErrorDomain};

    #[test]
    fn test_manual_recovery_action_creation() {
        let action = ManualRecoveryAction::new(
            ManualRecoveryActionType::CheckInput,
            "Test Action".to_string(),
            vec!["Step 1".to_string(), "Step 2".to_string()],
            "Expected outcome".to_string(),
        )
        .with_priority(1)
        .with_duration(5)
        .requires_admin();

        assert_eq!(action.title, "Test Action");
        assert_eq!(action.priority, 1);
        assert_eq!(action.estimated_duration_minutes, Some(5));
        assert!(action.requires_admin);
        assert!(!action.is_destructive);
    }

    #[test]
    fn test_manual_recovery_guide_auth_actions() {
        let mut guide = ManualRecoveryGuide::new();
        
        let error = EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::Auth,
            RecoveryStrategy::UserRecoverable,
            "Invalid credentials provided".to_string(),
        );

        let actions = guide.generate_recovery_actions(&error);
        
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| matches!(a.action_type, ManualRecoveryActionType::CheckInput)));
        assert!(actions.iter().any(|a| matches!(a.action_type, ManualRecoveryActionType::ManualRetry)));
    }

    #[test]
    fn test_manual_recovery_guide_data_actions() {
        let mut guide = ManualRecoveryGuide::new();
        
        let error = EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::Data,
            RecoveryStrategy::AdminRecoverable,
            "Database connection failed".to_string(),
        );

        let actions = guide.generate_recovery_actions(&error);
        
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| matches!(a.action_type, ManualRecoveryActionType::CheckDatabaseAccess)));
        assert!(actions.iter().any(|a| matches!(a.action_type, ManualRecoveryActionType::ContactAdministrator)));
    }

    #[test]
    fn test_manual_recovery_guide_system_actions() {
        let mut guide = ManualRecoveryGuide::new();
        
        let error = EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::System,
            RecoveryStrategy::UserRecoverable,
            "Network connection timeout".to_string(),
        );

        let actions = guide.generate_recovery_actions(&error);
        
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| matches!(a.action_type, ManualRecoveryActionType::CheckNetworkConnection)));
    }

    #[test]
    fn test_action_priority_sorting() {
        let mut guide = ManualRecoveryGuide::new();
        
        let error = EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::Auth,
            RecoveryStrategy::UserRecoverable,
            "Permission denied".to_string(),
        );

        let actions = guide.generate_recovery_actions(&error);
        
        // Actions should be sorted by priority (ascending)
        for i in 1..actions.len() {
            assert!(actions[i-1].priority <= actions[i].priority);
        }
    }

    #[test]
    fn test_action_caching() {
        let mut guide = ManualRecoveryGuide::new();
        
        let error = EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::UI,
            RecoveryStrategy::UserRecoverable,
            "UI refresh needed".to_string(),
        );

        let actions1 = guide.generate_recovery_actions(&error);
        let actions2 = guide.generate_recovery_actions(&error);
        
        // Should return the same actions (from cache)
        assert_eq!(actions1.len(), actions2.len());
        for (a1, a2) in actions1.iter().zip(actions2.iter()) {
            assert_eq!(a1.title, a2.title);
            assert_eq!(a1.priority, a2.priority);
        }
    }

    #[test]
    fn test_performance_tracking() {
        let mut guide = ManualRecoveryGuide::new();
        
        let error = EnhancedError::new(
            ErrorSeverity::Low,
            ErrorDomain::System,
            RecoveryStrategy::AutoRecoverable,
            "Minor system issue".to_string(),
        );

        // Generate actions to populate performance data
        for _ in 0..10 {
            guide.generate_recovery_actions(&error);
        }

        let stats = guide.get_performance_stats();
        assert!(stats.sample_count > 0);
        assert!(stats.average_time_us >= 0.0);
        assert!(stats.meets_requirement); // Should be under 1ms
    }

    #[test]
    fn test_recovery_result_recording() {
        let mut guide = ManualRecoveryGuide::new();
        
        let result = ManualRecoveryResult {
            action_id: Uuid::new_v4(),
            success: true,
            user_feedback: Some("This worked great!".to_string()),
            duration: Duration::from_secs(30),
            resolved_error: true,
            notes: Some("Fixed by restarting service".to_string()),
        };

        // Should not panic
        guide.record_recovery_result(result);
    }
}