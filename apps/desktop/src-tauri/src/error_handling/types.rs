use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;
use anyhow;

/// Error severity levels for multi-level classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Critical system errors that require immediate attention
    Critical,
    /// High-priority errors that significantly impact functionality
    High,
    /// Medium-priority errors with moderate impact
    Medium,
    /// Low-priority errors with minimal impact
    Low,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Critical => write!(f, "Critical"),
            ErrorSeverity::High => write!(f, "High"),
            ErrorSeverity::Medium => write!(f, "Medium"),
            ErrorSeverity::Low => write!(f, "Low"),
        }
    }
}

impl std::str::FromStr for ErrorSeverity {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Critical" => Ok(ErrorSeverity::Critical),
            "High" => Ok(ErrorSeverity::High),
            "Medium" => Ok(ErrorSeverity::Medium),
            "Low" => Ok(ErrorSeverity::Low),
            _ => Err(anyhow::anyhow!("Invalid error severity: {}", s)),
        }
    }
}

/// Domain classification system covering all application areas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorDomain {
    /// Authentication and authorization errors
    Auth,
    /// Database and data persistence errors
    Data,
    /// Asset management and configuration errors
    Assets,
    /// System-level and infrastructure errors
    System,
    /// User interface and frontend errors
    UI,
}

impl fmt::Display for ErrorDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorDomain::Auth => write!(f, "Auth"),
            ErrorDomain::Data => write!(f, "Data"),
            ErrorDomain::Assets => write!(f, "Assets"),
            ErrorDomain::System => write!(f, "System"),
            ErrorDomain::UI => write!(f, "UI"),
        }
    }
}

impl std::str::FromStr for ErrorDomain {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Auth" => Ok(ErrorDomain::Auth),
            "Data" => Ok(ErrorDomain::Data),
            "Assets" => Ok(ErrorDomain::Assets),
            "System" => Ok(ErrorDomain::System),
            "UI" => Ok(ErrorDomain::UI),
            _ => Err(anyhow::anyhow!("Invalid error domain: {}", s)),
        }
    }
}

/// Recovery strategy classification for automated response selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Errors that can be automatically recovered by the system
    AutoRecoverable,
    /// Errors that require user intervention to resolve
    UserRecoverable,
    /// Errors that require administrator privileges to resolve
    AdminRecoverable,
    /// Errors that require manual intervention and cannot be automatically resolved
    ManualRecoverable,
}

impl fmt::Display for RecoveryStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryStrategy::AutoRecoverable => write!(f, "AutoRecoverable"),
            RecoveryStrategy::UserRecoverable => write!(f, "UserRecoverable"),
            RecoveryStrategy::AdminRecoverable => write!(f, "AdminRecoverable"),
            RecoveryStrategy::ManualRecoverable => write!(f, "ManualRecoverable"),
        }
    }
}

impl std::str::FromStr for RecoveryStrategy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AutoRecoverable" => Ok(RecoveryStrategy::AutoRecoverable),
            "UserRecoverable" => Ok(RecoveryStrategy::UserRecoverable),
            "AdminRecoverable" => Ok(RecoveryStrategy::AdminRecoverable),
            "ManualRecoverable" => Ok(RecoveryStrategy::ManualRecoverable),
            _ => Err(anyhow::anyhow!("Invalid recovery strategy: {}", s)),
        }
    }
}

/// Enhanced error struct with comprehensive classification and context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedError {
    /// Unique identifier for this error instance
    pub id: Uuid,
    /// Error severity level
    pub severity: ErrorSeverity,
    /// Domain classification
    pub domain: ErrorDomain,
    /// Recovery strategy
    pub recovery_strategy: RecoveryStrategy,
    /// Original error message
    pub message: String,
    /// Detailed error description for debugging
    pub details: Option<String>,
    /// Error context identifier
    pub context_id: Option<Uuid>,
    /// Timestamp when error occurred
    pub timestamp: DateTime<Utc>,
    /// Optional correlation ID for tracking across layers
    pub correlation_id: Option<Uuid>,
    /// Component where error originated
    pub component: Option<String>,
    /// Operation that was being performed when error occurred
    pub operation: Option<String>,
}

impl EnhancedError {
    /// Create a new enhanced error with all required fields
    pub fn new(
        severity: ErrorSeverity,
        domain: ErrorDomain,
        recovery_strategy: RecoveryStrategy,
        message: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            severity,
            domain,
            recovery_strategy,
            message,
            details: None,
            context_id: None,
            timestamp: Utc::now(),
            correlation_id: None,
            component: None,
            operation: None,
        }
    }

    /// Set detailed error description
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    /// Set error context identifier
    pub fn with_context_id(mut self, context_id: Uuid) -> Self {
        self.context_id = Some(context_id);
        self
    }

    /// Set correlation ID for cross-layer tracking
    pub fn with_correlation_id(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Set component where error originated
    pub fn with_component(mut self, component: String) -> Self {
        self.component = Some(component);
        self
    }

    /// Set operation that was being performed
    pub fn with_operation(mut self, operation: String) -> Self {
        self.operation = Some(operation);
        self
    }

    /// Check if error is critical
    pub fn is_critical(&self) -> bool {
        self.severity == ErrorSeverity::Critical
    }

    /// Check if error is recoverable by the system
    pub fn is_auto_recoverable(&self) -> bool {
        self.recovery_strategy == RecoveryStrategy::AutoRecoverable
    }

    /// Get user-friendly error message (no internal details)
    pub fn user_message(&self) -> String {
        match self.severity {
            ErrorSeverity::Critical => "A critical error has occurred. Please contact support.".to_string(),
            ErrorSeverity::High => self.message.clone(),
            ErrorSeverity::Medium | ErrorSeverity::Low => self.message.clone(),
        }
    }
}

impl fmt::Display for EnhancedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}:{}:{}] {}",
            self.severity, self.domain, self.recovery_strategy, self.message
        )
    }
}

impl std::error::Error for EnhancedError {}

/// Result type using EnhancedError for enhanced error handling
pub type EnhancedResult<T> = Result<T, EnhancedError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity_display_and_parse() {
        assert_eq!(ErrorSeverity::Critical.to_string(), "Critical");
        assert_eq!(ErrorSeverity::High.to_string(), "High");
        assert_eq!(ErrorSeverity::Medium.to_string(), "Medium");
        assert_eq!(ErrorSeverity::Low.to_string(), "Low");

        assert_eq!("Critical".parse::<ErrorSeverity>().unwrap(), ErrorSeverity::Critical);
        assert_eq!("High".parse::<ErrorSeverity>().unwrap(), ErrorSeverity::High);
        assert_eq!("Medium".parse::<ErrorSeverity>().unwrap(), ErrorSeverity::Medium);
        assert_eq!("Low".parse::<ErrorSeverity>().unwrap(), ErrorSeverity::Low);
        
        assert!("Invalid".parse::<ErrorSeverity>().is_err());
    }

    #[test]
    fn test_error_domain_display_and_parse() {
        assert_eq!(ErrorDomain::Auth.to_string(), "Auth");
        assert_eq!(ErrorDomain::Data.to_string(), "Data");
        assert_eq!(ErrorDomain::Assets.to_string(), "Assets");
        assert_eq!(ErrorDomain::System.to_string(), "System");
        assert_eq!(ErrorDomain::UI.to_string(), "UI");

        assert_eq!("Auth".parse::<ErrorDomain>().unwrap(), ErrorDomain::Auth);
        assert_eq!("Data".parse::<ErrorDomain>().unwrap(), ErrorDomain::Data);
        assert_eq!("Assets".parse::<ErrorDomain>().unwrap(), ErrorDomain::Assets);
        assert_eq!("System".parse::<ErrorDomain>().unwrap(), ErrorDomain::System);
        assert_eq!("UI".parse::<ErrorDomain>().unwrap(), ErrorDomain::UI);
        
        assert!("Invalid".parse::<ErrorDomain>().is_err());
    }

    #[test]
    fn test_recovery_strategy_display_and_parse() {
        assert_eq!(RecoveryStrategy::AutoRecoverable.to_string(), "AutoRecoverable");
        assert_eq!(RecoveryStrategy::UserRecoverable.to_string(), "UserRecoverable");
        assert_eq!(RecoveryStrategy::AdminRecoverable.to_string(), "AdminRecoverable");
        assert_eq!(RecoveryStrategy::ManualRecoverable.to_string(), "ManualRecoverable");

        assert_eq!("AutoRecoverable".parse::<RecoveryStrategy>().unwrap(), RecoveryStrategy::AutoRecoverable);
        assert_eq!("UserRecoverable".parse::<RecoveryStrategy>().unwrap(), RecoveryStrategy::UserRecoverable);
        assert_eq!("AdminRecoverable".parse::<RecoveryStrategy>().unwrap(), RecoveryStrategy::AdminRecoverable);
        assert_eq!("ManualRecoverable".parse::<RecoveryStrategy>().unwrap(), RecoveryStrategy::ManualRecoverable);
        
        assert!("Invalid".parse::<RecoveryStrategy>().is_err());
    }

    #[test]
    fn test_enhanced_error_creation() {
        let error = EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::Auth,
            RecoveryStrategy::UserRecoverable,
            "Authentication failed".to_string(),
        );

        assert_eq!(error.severity, ErrorSeverity::High);
        assert_eq!(error.domain, ErrorDomain::Auth);
        assert_eq!(error.recovery_strategy, RecoveryStrategy::UserRecoverable);
        assert_eq!(error.message, "Authentication failed");
        assert!(error.details.is_none());
        assert!(error.context_id.is_none());
        assert!(!error.id.is_nil());
    }

    #[test]
    fn test_enhanced_error_builder_pattern() {
        let correlation_id = Uuid::new_v4();
        let context_id = Uuid::new_v4();
        
        let error = EnhancedError::new(
            ErrorSeverity::Critical,
            ErrorDomain::System,
            RecoveryStrategy::AdminRecoverable,
            "Database connection failed".to_string(),
        )
        .with_details("Connection timeout after 30 seconds".to_string())
        .with_context_id(context_id)
        .with_correlation_id(correlation_id)
        .with_component("DatabaseManager".to_string())
        .with_operation("initialize_connection".to_string());

        assert_eq!(error.details, Some("Connection timeout after 30 seconds".to_string()));
        assert_eq!(error.context_id, Some(context_id));
        assert_eq!(error.correlation_id, Some(correlation_id));
        assert_eq!(error.component, Some("DatabaseManager".to_string()));
        assert_eq!(error.operation, Some("initialize_connection".to_string()));
    }

    #[test]
    fn test_enhanced_error_predicates() {
        let critical_error = EnhancedError::new(
            ErrorSeverity::Critical,
            ErrorDomain::System,
            RecoveryStrategy::ManualRecoverable,
            "System failure".to_string(),
        );

        let auto_recoverable_error = EnhancedError::new(
            ErrorSeverity::Low,
            ErrorDomain::UI,
            RecoveryStrategy::AutoRecoverable,
            "UI refresh needed".to_string(),
        );

        assert!(critical_error.is_critical());
        assert!(!critical_error.is_auto_recoverable());
        
        assert!(!auto_recoverable_error.is_critical());
        assert!(auto_recoverable_error.is_auto_recoverable());
    }

    #[test]
    fn test_user_message_generation() {
        let critical_error = EnhancedError::new(
            ErrorSeverity::Critical,
            ErrorDomain::System,
            RecoveryStrategy::ManualRecoverable,
            "Internal system error".to_string(),
        );

        let user_error = EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::Auth,
            RecoveryStrategy::UserRecoverable,
            "Invalid credentials".to_string(),
        );

        assert_eq!(critical_error.user_message(), "A critical error has occurred. Please contact support.");
        assert_eq!(user_error.user_message(), "Invalid credentials");
    }

    #[test]
    fn test_enhanced_error_display() {
        let error = EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::UserRecoverable,
            "Data validation failed".to_string(),
        );

        let display_string = error.to_string();
        assert!(display_string.contains("Medium"));
        assert!(display_string.contains("Data"));
        assert!(display_string.contains("UserRecoverable"));
        assert!(display_string.contains("Data validation failed"));
    }
}