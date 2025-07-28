# Enhanced Error Handling Migration Guide

## Overview

This guide explains how to migrate from existing string-based error handling to the new enhanced error handling system while maintaining full backward compatibility. The enhanced system is designed for gradual adoption without requiring immediate changes to existing code.

## Backward Compatibility Guarantees

### Zero Breaking Changes
- All existing `Result<T, String>` signatures remain unchanged
- Existing error handling code continues to work without modification
- String-based error messages are preserved
- Frontend error display components function identically
- No changes required to existing tests

### Enhanced Features (Optional)
- Detailed error classification (severity, domain, recovery strategy)
- Error correlation and tracking
- Enhanced debugging information
- Context preservation
- Performance monitoring

## Migration Strategies

### Strategy 1: No Changes Required (Default)
Your existing code works unchanged:

```rust
// Backend - Existing code continues to work
#[tauri::command]
async fn existing_command() -> Result<String, String> {
    match some_operation() {
        Ok(result) => Ok(result),
        Err(e) => Err(format!("Operation failed: {}", e))
    }
}
```

```typescript
// Frontend - Existing code continues to work
try {
    const result = await invoke('existing_command');
    console.log(result);
} catch (error) {
    console.error('Error:', error); // Still receives string
    showNotification(error, 'error');
}
```

### Strategy 2: Optional Enhancement (Gradual Adoption)

#### Backend Enhancement
Use new macros for automatic error enhancement while maintaining string returns:

```rust
use ferrocodex::anyhow_to_string_result;

#[tauri::command]
async fn enhanced_command() -> Result<String, String> {
    // Convert anyhow::Result to Result<T, String> with enhanced error information
    anyhow_to_string_result!(
        some_anyhow_operation(),
        "my_component",
        "my_operation"
    )
}
```

#### Frontend Enhancement
Use new utilities for optional enhanced error access:

```typescript
import { ErrorHandlingUtils, analyzeError } from '@/utils/errorHandling';

try {
    const result = await invoke('enhanced_command');
    console.log(result);
} catch (error) {
    // Existing behavior preserved
    console.error('Error:', error);
    
    // Optional: Use enhanced error analysis
    const analysis = analyzeError(error);
    if (analysis.isRecoverable) {
        showRetryNotification(analysis.userMessage, analysis.action);
    } else {
        showErrorNotification(analysis.userMessage);
    }
}
```

### Strategy 3: Full Enhancement (New Code)

#### Backend - Use Enhanced Error Types
```rust
use ferrocodex::error_handling::{EnhancedError, ErrorSeverity, ErrorDomain, RecoveryStrategy, ErrorConverter};

#[tauri::command]
async fn fully_enhanced_command() -> Result<String, String> {
    match some_operation() {
        Ok(result) => Ok(result),
        Err(e) => {
            let enhanced_error = EnhancedError::new(
                ErrorSeverity::High,
                ErrorDomain::Data,
                RecoveryStrategy::UserRecoverable,
                "Database operation failed".to_string(),
            )
            .with_details(e.to_string())
            .with_component("database_service".to_string())
            .with_operation("user_query".to_string());
            
            // Still returns string for compatibility
            Err(enhanced_error.to_user_string())
        }
    }
}
```

#### Frontend - Enhanced Error Wrapper Usage
```typescript
import { enhancedInvoke, withRetry } from '@/utils/errorHandling';

try {
    // Enhanced invoke with automatic error wrapping
    const result = await withRetry(
        () => enhancedInvoke('fully_enhanced_command'),
        { maxRetries: 3, retryIf: (error) => isRecoverableError(error) }
    );
    console.log(result);
} catch (error) {
    // Still works with existing error handling
    const errorInfo = analyzeError(error);
    console.error(`[${errorInfo.severity}:${errorInfo.domain}] ${errorInfo.userMessage}`);
    
    if (errorInfo.isRecoverable) {
        setTimeout(() => retryOperation(), 2000);
    }
}
```

## Progressive Enhancement Examples

### Example 1: Database Error Handling

#### Before (Existing Code - No Changes Required)
```rust
#[tauri::command]
async fn get_user(user_id: i64) -> Result<User, String> {
    database
        .get_user(user_id)
        .map_err(|e| format!("Failed to get user: {}", e))
}
```

#### After (Optional Enhancement)
```rust
use ferrocodex::with_enhanced_error_context;

#[tauri::command]
async fn get_user(user_id: i64) -> Result<User, String> {
    with_enhanced_error_context!("user_service", "get_user", {
        database
            .get_user(user_id)
            .map_err(|e| format!("Failed to get user: {}", e))
    })
}
```

### Example 2: Authentication Error Handling

#### Frontend - Gradual Enhancement
```typescript
// Existing error handling (unchanged)
async function login(username: string, password: string) {
    try {
        return await invoke('login', { username, password });
    } catch (error) {
        // String error as before
        if (error.includes('Invalid credentials')) {
            showError('Please check your username and password');
        } else {
            showError('Login failed. Please try again.');
        }
        throw error;
    }
}

// Enhanced error handling (optional)
async function loginEnhanced(username: string, password: string) {
    try {
        return await enhancedInvoke('login', { username, password });
    } catch (error) {
        const analysis = analyzeError(error);
        
        switch (analysis.domain) {
            case 'Auth':
                if (analysis.severity === 'High') {
                    showError('Authentication failed. Please check your credentials.');
                } else {
                    showError('Please try logging in again.');
                }
                break;
            case 'System':
                showError('System unavailable. Please try again later.');
                break;
            default:
                showError(analysis.userMessage);
        }
        
        throw error;
    }
}
```

## Feature Flag Configuration

The enhanced error handling system supports gradual rollout through feature flags:

```rust
use ferrocodex::error_handling::ErrorHandlingFeatureFlags;

// Minimal rollout - only basic classification
let minimal_flags = ErrorHandlingFeatureFlags::minimal();

// Full rollout - all enhanced features
let full_flags = ErrorHandlingFeatureFlags::full();

// Custom configuration
let custom_flags = ErrorHandlingFeatureFlags {
    enable_classification: true,
    enable_context_tracking: true,
    enable_correlation: false,  // Not ready yet
    enable_database_storage: false,  // Not ready yet
    enable_performance_monitoring: true,
};
```

## Testing Migration

### Existing Tests (No Changes Required)
All existing tests continue to pass without modification:

```rust
#[test]
fn test_existing_error_handling() {
    let result = some_operation_that_fails();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Expected error message");
}
```

### Enhanced Tests (Optional)
Add new tests for enhanced functionality:

```rust
#[test]
fn test_enhanced_error_handling() {
    use ferrocodex::error_handling::EnhancedErrorInterface;
    
    let enhanced_error = create_enhanced_error();
    assert_eq!(enhanced_error.get_severity(), Some(ErrorSeverity::High));
    assert_eq!(enhanced_error.get_domain(), Some(ErrorDomain::Auth));
    assert!(enhanced_error.get_error_id().is_some());
}
```

## Performance Considerations

### Overhead
- Error conversion: <100μs per error
- No impact on success paths
- Minimal memory overhead
- Existing error handling performance preserved

### Monitoring
The enhanced system includes optional performance monitoring:

```rust
// Check if performance requirements are met
let manager = ErrorHandlingManager::default();
if !manager.conversion_layer().meets_performance_requirement() {
    log::warn!("Error handling performance degraded");
}
```

## Common Patterns

### Pattern 1: Wrapper Macros for Existing Functions
```rust
// Convert existing error handling
macro_rules! enhance_existing_command {
    ($command:ident) => {
        #[tauri::command]
        async fn $command() -> Result<String, String> {
            anyhow_to_string_result!(original_command(), stringify!($command))
        }
    };
}
```

### Pattern 2: Error Boundary Components
```typescript
// Enhanced error boundary with backward compatibility
function ErrorBoundary({ children }: { children: React.ReactNode }) {
    return (
        <ErrorBoundaryWrapper
            fallback={(error) => {
                const analysis = analyzeError(error);
                return (
                    <div>
                        <h2>Something went wrong</h2>
                        <p>{analysis.userMessage}</p>
                        {analysis.isRecoverable && (
                            <button onClick={() => window.location.reload()}>
                                Try Again
                            </button>
                        )}
                    </div>
                );
            }}
        >
            {children}
        </ErrorBoundaryWrapper>
    );
}
```

### Pattern 3: Notification System Integration
```typescript
// Enhanced notification system with fallback
function showErrorNotification(error: any) {
    const analysis = analyzeError(error);
    
    notification.error({
        message: 'Error',
        description: analysis.userMessage,
        duration: analysis.severity === 'Critical' ? 0 : 4.5,
        btn: analysis.isRecoverable ? (
            <Button size="small" onClick={retryLastOperation}>
                Retry
            </Button>
        ) : undefined,
    });
}
```

## Migration Checklist

### Phase 1: Preparation
- [ ] Review existing error handling patterns
- [ ] Identify high-value areas for enhancement
- [ ] Set up feature flags configuration
- [ ] Run existing test suite to establish baseline

### Phase 2: Backend Migration
- [ ] Add enhanced error handling imports
- [ ] Convert high-priority commands using wrapper macros
- [ ] Add enhanced error classification for new errors
- [ ] Test backward compatibility

### Phase 3: Frontend Migration
- [ ] Add enhanced error handling utilities
- [ ] Update error notification system
- [ ] Enhance error boundary components
- [ ] Add retry logic for recoverable errors

### Phase 4: Validation
- [ ] Verify all existing tests still pass
- [ ] Test enhanced error scenarios
- [ ] Validate performance requirements
- [ ] Monitor error handling in development

### Phase 5: Production Rollout
- [ ] Deploy with minimal feature flags
- [ ] Monitor error handling performance
- [ ] Gradually enable enhanced features
- [ ] Collect feedback and iterate

## Troubleshooting

### Common Issues

#### Issue: Existing tests failing
**Solution**: Check that string error messages haven't changed format. Enhanced error handling preserves exact string formats.

#### Issue: Performance degradation
**Solution**: Use feature flags to disable expensive features. Check that error conversion is not happening in hot paths.

#### Issue: TypeScript compilation errors
**Solution**: Enhanced error types are optional. Existing code should compile without changes.

### Debug Tools

```typescript
// Check if enhanced error handling is working
console.log('Enhanced error handling available:', isEnhancedErrorHandlingAvailable());

// Debug error information
const debugInfo = getErrorHandlingDebugInfo(error);
console.log('Error debug info:', debugInfo);
```

## Support and Resources

- **Migration Support**: Contact the development team for migration assistance
- **Performance Monitoring**: Use built-in performance monitoring tools
- **Feature Flags**: Configure gradual rollout based on your needs
- **Testing**: All existing tests should continue to pass unchanged

## Conclusion

The enhanced error handling system is designed for zero-friction adoption. You can:

1. **Continue as-is**: No changes required, everything works as before
2. **Gradual enhancement**: Use new features where they add value
3. **Full adoption**: Take advantage of all enhanced capabilities for new code

The choice is yours, and you can migrate at your own pace without breaking existing functionality.