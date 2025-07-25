# Testing Strategy

This document defines the comprehensive testing strategy for Ferrocodex, following a test pyramid approach with appropriate coverage across unit, integration, and end-to-end testing layers.

## Testing Philosophy

### Core Principles
- **Test Pyramid**: Heavy emphasis on unit tests, moderate integration tests, minimal E2E tests
- **Test-First Development**: Critical security and data integrity features require tests before implementation
- **Risk-Based Testing**: Higher coverage for security, authentication, and data persistence layers
- **Isolation**: Tests should be independent and executable in any order
- **Fast Feedback**: Test suite should run quickly to support rapid development cycles

### Coverage Targets
- **Unit Tests**: 90%+ coverage for business logic, utilities, and security functions
- **Integration Tests**: 70%+ coverage for API endpoints and database operations
- **Component Tests**: 80%+ coverage for React components with complex state logic
- **E2E Tests**: Critical user workflows and security scenarios

## Frontend Testing Strategy

### Technology Stack
- **Test Framework**: Vitest (configured in `vite.config.ts`)
- **Testing Library**: React Testing Library for component testing
- **Test Utilities**: @testing-library/jest-dom for DOM assertions
- **Mocking**: Vitest's built-in mocking capabilities

### Test Organization

#### Component Tests
Located in `src/components/` alongside components or in `__tests__` directories:

```typescript
// Example: Dashboard.test.tsx
import { render, screen, fireEvent } from '@testing-library/react';
import { expect, test, vi, beforeEach } from 'vitest';
import Dashboard from './Dashboard';

// Mock Tauri API calls
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({
    total_assets: 0,
    total_versions: 0,
    encryption_type: 'AES-256'
  }),
}));

test('renders dashboard with user information', () => {
  render(<Dashboard />);
  expect(screen.getByText('Welcome, admin')).toBeInTheDocument();
});
```

#### State Management Tests
Located in `src/store/`:

```typescript
// Example: auth.test.ts
import { renderHook, act } from '@testing-library/react';
import useAuthStore from './auth';

test('should handle login flow', () => {
  const { result } = renderHook(() => useAuthStore());
  
  act(() => {
    result.current.setUser({
      id: 1,
      username: 'testuser',
      role: 'Administrator'
    });
  });
  
  expect(result.current.user?.username).toBe('testuser');
});
```

#### Utility Function Tests
Located in `src/utils/`:

```typescript
// Example: roleUtils.test.ts
import { hasPermission, canAccessFeature } from './roleUtils';

test('administrator should have all permissions', () => {
  expect(hasPermission('Administrator', 'manage_users')).toBe(true);
  expect(hasPermission('Administrator', 'manage_assets')).toBe(true);
});
```

### Frontend Test Patterns

#### Mocking Tauri API
All Tauri API calls must be mocked in tests:

```typescript
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// In test setup
const mockInvoke = vi.mocked(invoke);
mockInvoke.mockImplementation((command: string, args?: any) => {
  switch (command) {
    case 'get_dashboard_stats':
      return Promise.resolve({ total_assets: 5, total_versions: 12 });
    case 'create_user':
      return Promise.resolve({ success: true });
    default:
      return Promise.reject(new Error(`Unhandled command: ${command}`));
  }
});
```

#### Component Testing Best Practices
- Test user interactions, not implementation details
- Mock external dependencies (Tauri API, external services)
- Use semantic queries (getByRole, getByText) over brittle selectors
- Test error states and loading states
- Verify accessibility attributes

#### Ant Design Component Testing
Special setup for Ant Design components is configured in `test-setup.ts`:

```typescript
// Mock window.matchMedia for Ant Design
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
  })),
});
```

## Backend Testing Strategy

### Technology Stack
- **Test Framework**: Rust's built-in test framework
- **Test Organization**: `#[cfg(test)]` modules in each source file
- **Database Testing**: In-memory SQLite databases for isolation
- **Async Testing**: Tokio test runtime for async operations

### Test Organization

#### Unit Tests
Each module contains its own test module:

```rust
// In auth/mod.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_password_hashing() {
        let password = "test_password";
        let hashed = hash_password(password).unwrap();
        assert!(verify_password(password, &hashed).unwrap());
    }
    
    #[tokio::test]
    async fn test_session_token_validation() {
        let mut session_manager = SessionManager::new(3600);
        let token = session_manager.create_session(1, "testuser", UserRole::Administrator);
        assert!(session_manager.validate_token(&token.token).is_ok());
    }
}
```

#### Repository Tests
Database layer tests use temporary databases:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::tempdir;
    
    async fn setup_test_db() -> Database {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        Database::new(db_path.to_str().unwrap()).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_user_creation() {
        let db = setup_test_db().await;
        let user_repo = SqliteUserRepository::new(db);
        
        let request = CreateUserRequest {
            username: "testuser".to_string(),
            password: "password123".to_string(),
            role: UserRole::Engineer,
        };
        
        let result = user_repo.create_user(request).await;
        assert!(result.is_ok());
    }
}
```

### Backend Test Patterns

#### Error Handling Tests
Verify proper error handling for all failure cases:

```rust
#[tokio::test]
async fn test_invalid_credentials() {
    let db = setup_test_db().await;
    let session_manager = SessionManager::new(3600);
    
    let result = authenticate_user(
        &db, 
        &session_manager, 
        "invalid_user", 
        "wrong_password"
    ).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid credentials"));
}
```

#### Security Tests
Critical security functions require comprehensive testing:

```rust
#[tokio::test]
async fn test_rate_limiting() {
    let mut limiter = RateLimiter::new(2, Duration::from_secs(60));
    
    // First two attempts should succeed
    assert!(limiter.check_rate_limit("test_user").is_ok());
    assert!(limiter.check_rate_limit("test_user").is_ok());
    
    // Third attempt should be rate limited
    assert!(limiter.check_rate_limit("test_user").is_err());
}
```

#### Database Integration Tests
Test complex database operations and transactions:

```rust
#[tokio::test]
async fn test_configuration_version_history() {
    let db = setup_test_db().await;
    let config_repo = SqliteConfigurationRepository::new(db);
    
    // Create asset and configuration
    let asset_id = create_test_asset(&config_repo).await;
    let config_id = create_test_configuration(&config_repo, asset_id).await;
    
    // Verify version history
    let versions = config_repo.get_configuration_versions(config_id).await.unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].version_number, 1);
}
```

## Integration Testing

### API Integration Tests
Test complete request-response cycles for Tauri commands:

```rust
#[tokio::test]
async fn test_complete_user_creation_workflow() {
    let app = setup_test_app().await;
    
    // Initialize database
    let init_result = initialize_database(app.clone(), db_state.clone()).await;
    assert!(init_result.is_ok());
    
    // Create user
    let user_request = CreateUserRequest {
        username: "testuser".to_string(),
        password: "password123".to_string(),
        role: UserRole::Engineer,
    };
    
    let create_result = create_user(
        app.clone(),
        db_state.clone(),
        session_state.clone(),
        user_request
    ).await;
    
    assert!(create_result.is_ok());
}
```

### Cross-Module Integration
Test interactions between different modules:

```rust
#[tokio::test]
async fn test_asset_configuration_relationship() {
    let db = setup_test_db().await;
    let asset_repo = SqliteAssetRepository::new(db.clone());
    let config_repo = SqliteConfigurationRepository::new(db);
    
    // Create asset
    let asset_request = CreateAssetRequest {
        name: "Test Asset".to_string(),
        description: Some("Test Description".to_string()),
        location: Some("Test Location".to_string()),
    };
    let asset_id = asset_repo.create_asset(asset_request).await.unwrap();
    
    // Create configuration for asset
    let config_request = CreateConfigurationRequest {
        asset_id,
        filename: "config.bin".to_string(),
        content: vec![1, 2, 3, 4],
        description: Some("Test config".to_string()),
    };
    let config_id = config_repo.create_configuration(config_request).await.unwrap();
    
    // Verify relationship
    let configs = config_repo.get_configurations_for_asset(asset_id).await.unwrap();
    assert!(configs.iter().any(|c| c.id == config_id));
}
```

## End-to-End Testing

### Critical User Workflows
Test complete user workflows that span frontend and backend:

1. **User Registration & Login Flow**
   - Admin creates new user account
   - New user logs in with credentials
   - Session management and token validation

2. **Asset & Configuration Management**
   - Create new asset
   - Upload configuration file
   - View configuration history
   - Export configuration

3. **Security Workflows**
   - Rate limiting during login attempts
   - Session timeout and re-authentication
   - Role-based access control validation

4. **Firmware Management**
   - Upload firmware files
   - Link firmware to configurations
   - Firmware analysis pipeline
   - Recovery package creation

### E2E Testing Tools
- **Tauri**: Built-in testing capabilities for desktop app testing
- **WebDriver**: For complex UI automation scenarios
- **Test Data**: Isolated test databases and file systems

## Test Configuration

### Frontend Test Configuration (`vite.config.ts`)
```typescript
export default defineConfig({
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/test-setup.ts"],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'src/test-setup.ts',
        '**/*.test.*',
        '**/__tests__/**'
      ]
    }
  },
});
```

### Backend Test Configuration
```rust
// In Cargo.toml
[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.8"
serial_test = "3.0"  # For tests that need to run sequentially

// Test helper utilities
#[cfg(test)]
pub mod test_utils {
    use super::*;
    use tempfile::tempdir;
    
    pub async fn setup_test_database() -> Database {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        Database::new(db_path.to_str().unwrap()).await.unwrap()
    }
    
    pub fn create_test_user() -> CreateUserRequest {
        CreateUserRequest {
            username: format!("user_{}", uuid::Uuid::new_v4()),
            password: "test_password".to_string(),
            role: UserRole::Engineer,
        }
    }
}
```

## Running Tests

### Frontend Tests
```bash
# Run all frontend tests
cd apps/desktop && npm run test

# Run tests with coverage
npm run test:coverage

# Run specific test file
npm run test -- src/components/Dashboard.test.tsx

# Watch mode for development
npm run test -- --watch
```

### Backend Tests
```bash
# Run all backend tests
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml

# Run specific module tests
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml -- users::tests

# Run with output
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml -- --nocapture
```

### Full Test Suite
```bash
# From project root
npm run test
```

## Continuous Integration

### GitHub Actions Integration
Tests are automatically run on:
- Pull request creation and updates
- Pushes to main branch
- Release tag creation

### Test Requirements
- All tests must pass before code can be merged
- Coverage thresholds must be maintained
- No new code without corresponding tests for critical paths

## Test Data Management

### Test Fixtures
- **Database Fixtures**: Predefined database states for consistent testing
- **File Fixtures**: Sample configuration and firmware files
- **User Fixtures**: Standard user accounts with different roles

### Test Isolation
- Each test uses its own temporary database
- File system operations use temporary directories
- Tests clean up resources after completion

### Mock Data Generation
```typescript
// Frontend mock data
export const mockUser = {
  id: 1,
  username: 'testuser',
  role: 'Administrator' as UserRole,
  created_at: '2023-01-01',
  is_active: true,
};

export const mockAsset = {
  id: 1,
  name: 'Test Asset',
  description: 'Test Description',
  location: 'Test Location',
  created_at: '2023-01-01',
};
```

```rust
// Backend test data generation
impl CreateUserRequest {
    pub fn test_user(username: &str) -> Self {
        Self {
            username: username.to_string(),
            password: "test_password".to_string(),
            role: UserRole::Engineer,
        }
    }
}
```

This comprehensive testing strategy ensures high-quality, reliable software with robust error handling and security validation across all layers of the Ferrocodex application.