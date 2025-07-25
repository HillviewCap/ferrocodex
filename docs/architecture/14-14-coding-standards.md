# Coding Standards

This document establishes mandatory coding standards for the Ferrocodex project to ensure consistency, maintainability, and security across the TypeScript frontend and Rust backend.

## Core Principles

### Security-First Development
- **Input Validation**: All user inputs must be validated on both frontend and backend
- **Error Handling**: Never expose internal system details in error messages
- **Authentication**: All sensitive operations require valid session tokens
- **Data Sanitization**: Sanitize all user inputs before database operations
- **Secret Management**: Never commit secrets, API keys, or passwords to version control

### Type Safety
- **TypeScript**: Strict mode enabled, no `any` types except for well-documented legacy integrations
- **Rust**: Leverage the type system for compile-time safety, avoid `unwrap()` in production code
- **Interface Consistency**: Frontend TypeScript types must mirror backend Rust structures
- **Null Safety**: Explicit handling of optional values using TypeScript `undefined`/`null` and Rust `Option<T>`

### Explicit Error Handling
- **Backend**: All functions return `Result<T, String>` for operations that can fail
- **Frontend**: Use explicit error states in components and handle all async operation failures
- **Consistency**: Error messages follow standard format and are user-friendly
- **Logging**: All errors are logged with appropriate context before being returned

## TypeScript Standards

### Code Organization
```typescript
// File structure example: components/UserManagement.tsx
import React, { useState, useEffect } from 'react';
import { Button, Table, Modal, notification } from 'antd';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../store/auth';
import { UserInfo, CreateUserRequest, UserRole } from '../types/auth';

// Component implementation follows...
```

### Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| **Components** | PascalCase | `UserManagement`, `AssetList` |
| **Hooks** | camelCase with `use` prefix | `useAuthStore`, `useAssetData` |
| **Variables** | camelCase | `userName`, `configurationId` |
| **Constants** | SCREAMING_SNAKE_CASE | `MAX_FILE_SIZE`, `DEFAULT_TIMEOUT` |
| **Types/Interfaces** | PascalCase | `UserInfo`, `ConfigurationData` |
| **Functions** | camelCase | `handleSubmit`, `validateInput` |
| **Files** | PascalCase for components, camelCase for utilities | `Dashboard.tsx`, `roleUtils.ts` |
| **Directories** | camelCase | `components`, `store`, `utils` |

### Type Definitions
```typescript
// types/auth.ts - Mirror Rust structures exactly
export interface UserInfo {
  id: number;
  username: string;
  role: UserRole;
  created_at: string;
  is_active: boolean;
}

export interface CreateUserRequest {
  username: string;
  password: string;
  role: UserRole;
}

export type UserRole = 'Administrator' | 'Engineer';

// Use discriminated unions for complex state
export type LoadingState = 
  | { status: 'idle' }
  | { status: 'loading' }
  | { status: 'success'; data: UserInfo[] }
  | { status: 'error'; error: string };
```

### Component Standards
```typescript
// Mandatory component structure
interface Props {
  // Explicit prop types
  userId?: number;
  onUpdate?: (user: UserInfo) => void;
}

const UserManagement: React.FC<Props> = ({ userId, onUpdate }) => {
  // State declarations with explicit types
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  
  // Auth store usage
  const { user: currentUser, token } = useAuthStore();
  
  // Effect hooks with dependencies
  useEffect(() => {
    if (currentUser?.role === 'Administrator') {
      loadUsers();
    }
  }, [currentUser]);
  
  // Async function with explicit error handling
  const loadUsers = async (): Promise<void> => {
    setLoading(true);
    setError(null);
    
    try {
      const result = await invoke<UserInfo[]>('get_users', { token });
      setUsers(result);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error';
      setError(errorMessage);
      notification.error({
        message: 'Error Loading Users',
        description: errorMessage,
      });
    } finally {
      setLoading(false);
    }
  };
  
  // Event handlers with explicit types
  const handleUserCreate = async (userData: CreateUserRequest): Promise<void> => {
    // Implementation...
  };
  
  // Render with conditional logic
  if (error) {
    return <div>Error: {error}</div>;
  }
  
  return (
    <div>
      {/* Component JSX */}
    </div>
  );
};

export default UserManagement;
```

### State Management (Zustand)
```typescript
// store/auth.ts
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface AuthState {
  user: UserInfo | null;
  token: string | null;
  isAuthenticated: boolean;
  
  // Actions with explicit types
  setUser: (user: UserInfo | null) => void;
  setToken: (token: string | null) => void;
  logout: () => void;
}

const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      user: null,
      token: null,
      isAuthenticated: false,
      
      setUser: (user) => set({ 
        user, 
        isAuthenticated: !!user 
      }),
      
      setToken: (token) => set({ token }),
      
      logout: () => set({ 
        user: null, 
        token: null, 
        isAuthenticated: false 
      }),
    }),
    {
      name: 'auth-storage',
      // Only persist non-sensitive data
      partialize: (state) => ({ 
        user: state.user,
        isAuthenticated: state.isAuthenticated 
      }),
    }
  )
);

export default useAuthStore;
```

### Error Handling Patterns
```typescript
// Async operations with comprehensive error handling
const handleAsyncOperation = async (): Promise<void> => {
  try {
    setLoading(true);
    const result = await invoke<OperationResult>('backend_command', { 
      data: sanitizedInput 
    });
    
    // Handle success
    onSuccess(result);
    
  } catch (error) {
    // Log error with context
    console.error('Operation failed:', error);
    
    // User-friendly error display
    const message = error instanceof Error 
      ? error.message 
      : 'An unexpected error occurred';
      
    notification.error({
      message: 'Operation Failed',
      description: message,
      duration: 5,
    });
    
    // Update error state
    setError(message);
    
  } finally {
    setLoading(false);
  }
};
```

### Import Organization
```typescript
// 1. React and React ecosystem
import React, { useState, useEffect, useCallback } from 'react';
import { useNavigate, useParams } from 'react-router-dom';

// 2. Third-party libraries
import { Button, Table, Modal, Form, Input } from 'antd';
import { invoke } from '@tauri-apps/api/core';

// 3. Internal stores and utilities
import useAuthStore from '../store/auth';
import { validateInput, sanitizeInput } from '../utils/validation';

// 4. Type definitions
import { UserInfo, CreateUserRequest } from '../types/auth';
import { AssetInfo } from '../types/assets';
```

### Testing Standards
```typescript
// Component.test.tsx
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { expect, test, vi, beforeEach, describe } from 'vitest';
import userEvent from '@testing-library/user-event';
import Component from './Component';

// Mock external dependencies
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../store/auth', () => ({
  default: vi.fn(() => ({
    user: mockUser,
    token: 'test-token',
  })),
}));

describe('Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });
  
  test('should handle user interaction correctly', async () => {
    const user = userEvent.setup();
    render(<Component />);
    
    const button = screen.getByRole('button', { name: /submit/i });
    await user.click(button);
    
    await waitFor(() => {
      expect(screen.getByText('Success message')).toBeInTheDocument();
    });
  });
});
```

## Rust Standards

### Code Organization
```rust
// Module structure example: users/mod.rs
mod repository;
mod types;

#[cfg(test)]
mod tests;

pub use repository::*;
pub use types::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
```

### Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| **Modules** | snake_case | `user_management`, `asset_repository` |
| **Functions** | snake_case | `create_user`, `validate_input` |
| **Variables** | snake_case | `user_name`, `config_id` |
| **Constants** | SCREAMING_SNAKE_CASE | `MAX_ATTEMPTS`, `DEFAULT_TIMEOUT` |
| **Types/Structs** | PascalCase | `UserInfo`, `CreateUserRequest` |
| **Enums** | PascalCase for type, PascalCase for variants | `UserRole::Administrator` |
| **Traits** | PascalCase | `UserRepository`, `Validatable` |
| **Files** | snake_case | `user_repository.rs`, `auth_types.rs` |

### Error Handling
```rust
// All public functions return Results
pub async fn create_user(
    db: &Database,
    request: CreateUserRequest,
) -> Result<UserInfo, String> {
    // Input validation
    if request.username.trim().is_empty() {
        return Err("Username cannot be empty".to_string());
    }
    
    // Business logic with proper error handling
    let password_hash = hash_password(&request.password)
        .map_err(|e| format!("Password hashing failed: {}", e))?;
    
    let user_id = db
        .create_user(&request.username, &password_hash, request.role)
        .await
        .map_err(|e| {
            error!("Database error creating user: {}", e);
            "Failed to create user".to_string()
        })?;
    
    // Success case
    let user_info = UserInfo {
        id: user_id,
        username: request.username,
        role: request.role,
        created_at: chrono::Utc::now().to_rfc3339(),
        is_active: true,
    };
    
    info!("User created successfully: {}", user_info.username);
    Ok(user_info)
}
```

### Type Definitions
```rust
// types/auth.rs - Must match TypeScript interfaces exactly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub role: UserRole,
    pub created_at: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    Administrator,
    Engineer,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Administrator => write!(f, "Administrator"),
            UserRole::Engineer => write!(f, "Engineer"),
        }
    }
}
```

### Repository Pattern
```rust
// Abstract trait for testability
#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn create_user(&self, request: CreateUserRequest) -> Result<UserInfo, String>;
    async fn get_user_by_id(&self, id: i64) -> Result<Option<UserInfo>, String>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<UserInfo>, String>;
    async fn list_users(&self) -> Result<Vec<UserInfo>, String>;
    async fn update_user_status(&self, id: i64, is_active: bool) -> Result<(), String>;
}

// Concrete implementation
pub struct SqliteUserRepository {
    db: Arc<Database>,
}

impl SqliteUserRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl UserRepository for SqliteUserRepository {
    async fn create_user(&self, request: CreateUserRequest) -> Result<UserInfo, String> {
        // Implementation with proper error handling
        let password_hash = hash_password(&request.password)
            .map_err(|e| format!("Password hashing failed: {}", e))?;
        
        let user_id = self.db
            .execute_query(
                "INSERT INTO users (username, password_hash, role, created_at, is_active) 
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                &[
                    &request.username,
                    &password_hash,
                    &request.role.to_string(),
                    &chrono::Utc::now().to_rfc3339(),
                    &true,
                ],
            )
            .await
            .map_err(|e| format!("Database error: {}", e))?;
        
        Ok(UserInfo {
            id: user_id,
            username: request.username,
            role: request.role,
            created_at: chrono::Utc::now().to_rfc3339(),
            is_active: true,
        })
    }
}
```

### Tauri Commands
```rust
// All Tauri commands follow this pattern
#[tauri::command]
pub async fn create_user(
    app: AppHandle,
    db_state: State<'_, DatabaseState>,
    session_state: State<'_, SessionManagerState>,
    user_data: CreateUserRequest,
) -> Result<UserInfo, String> {
    // Authentication check
    let session_manager = session_state.lock().unwrap();
    let current_user = session_manager
        .get_current_user()
        .ok_or("Authentication required")?;
    
    // Authorization check
    if current_user.role != UserRole::Administrator {
        return Err("Administrator role required".to_string());
    }
    
    // Input validation
    validate_user_request(&user_data)?;
    
    // Business logic
    let db = db_state
        .lock()
        .unwrap()
        .as_ref()
        .ok_or("Database not initialized")?;
    
    let user_repo = SqliteUserRepository::new(db.clone());
    let result = user_repo.create_user(user_data).await?;
    
    // Audit logging
    info!("User created by {}: {}", current_user.username, result.username);
    
    Ok(result)
}
```

### Testing Standards
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio_test;
    
    async fn setup_test_database() -> Database {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        Database::new(db_path.to_str().unwrap()).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_user_creation_success() {
        let db = setup_test_database().await;
        let user_repo = SqliteUserRepository::new(Arc::new(db));
        
        let request = CreateUserRequest {
            username: "testuser".to_string(),
            password: "password123".to_string(),
            role: UserRole::Engineer,
        };
        
        let result = user_repo.create_user(request).await;
        assert!(result.is_ok());
        
        let user = result.unwrap();
        assert_eq!(user.username, "testuser");
        assert_eq!(user.role, UserRole::Engineer);
        assert!(user.is_active);
    }
    
    #[tokio::test]
    async fn test_user_creation_duplicate_username() {
        let db = setup_test_database().await;
        let user_repo = SqliteUserRepository::new(Arc::new(db));
        
        let request = CreateUserRequest {
            username: "duplicate".to_string(),
            password: "password123".to_string(),
            role: UserRole::Engineer,
        };
        
        // First creation should succeed
        assert!(user_repo.create_user(request.clone()).await.is_ok());
        
        // Second creation should fail
        let result = user_repo.create_user(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }
}
```

## Shared Standards

### Database Queries
```rust
// Always use parameterized queries
let query = "SELECT * FROM users WHERE username = ?1 AND is_active = ?2";
let params = [&username, &true];

// Never use string concatenation
// BAD: let query = format!("SELECT * FROM users WHERE username = '{}'", username);
```

### Logging Standards
```rust
// Rust logging with context
use tracing::{error, info, warn, debug};

info!("User authentication successful: {}", username);
warn!("Rate limit approaching for user: {}", username);
error!("Database connection failed: {}", error);
debug!("Cache hit for asset: {}", asset_id);
```

```typescript
// TypeScript logging
console.info('User logged in:', { username: user.username, timestamp: new Date() });
console.warn('API request failed, retrying:', { attempt: retryCount, error: error.message });
console.error('Critical error:', { component: 'UserManagement', error: error.stack });
```

### Security Requirements

#### Input Validation
```rust
// Rust validation
fn validate_username(username: &str) -> Result<(), String> {
    if username.trim().is_empty() {
        return Err("Username cannot be empty".to_string());
    }
    if username.len() < 3 || username.len() > 50 {
        return Err("Username must be between 3 and 50 characters".to_string());
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err("Username can only contain letters, numbers, underscores, and hyphens".to_string());
    }
    Ok(())
}
```

```typescript
// TypeScript validation
const validateUsername = (username: string): string | null => {
  if (!username.trim()) {
    return 'Username cannot be empty';
  }
  if (username.length < 3 || username.length > 50) {
    return 'Username must be between 3 and 50 characters';
  }
  if (!/^[a-zA-Z0-9_-]+$/.test(username)) {
    return 'Username can only contain letters, numbers, underscores, and hyphens';
  }
  return null;
};
```

#### Authentication Requirements
- All sensitive operations require valid session tokens
- Session tokens expire after 1 hour of inactivity
- Failed login attempts are rate-limited
- All authentication events are logged for audit

#### Data Sanitization
- All user inputs are sanitized before database operations
- File uploads are validated for type and size
- SQL injection prevention through parameterized queries
- XSS prevention through proper input encoding

## Code Review Checklist

### TypeScript Review Points
- [ ] All types explicitly defined, no `any` usage
- [ ] Error handling implemented for all async operations
- [ ] Components follow established patterns
- [ ] Tauri API calls properly mocked in tests
- [ ] State management follows Zustand patterns
- [ ] Accessibility attributes included where needed

### Rust Review Points
- [ ] All functions return `Result<T, String>` where appropriate
- [ ] No `unwrap()` calls in production code
- [ ] Input validation implemented
- [ ] Database queries use parameterized statements
- [ ] Proper error logging with context
- [ ] Tests cover success and failure cases

### Security Review Points
- [ ] Authentication checks for sensitive operations
- [ ] Input validation on both frontend and backend
- [ ] No secrets in code or logs
- [ ] Rate limiting implemented where needed
- [ ] Proper error messages (no internal details exposed)
- [ ] Audit trail for all user actions

## Performance Standards

### Frontend Performance
- Components should render in under 100ms for typical operations
- Large lists should use virtualization when exceeding 100 items
- Images and assets should be optimized and lazy-loaded
- Bundle size should be monitored and kept minimal

### Backend Performance
- Database queries should complete in under 500ms for typical operations
- File operations should include progress reporting for operations over 1 second
- Memory usage should be monitored and kept reasonable
- CPU-intensive operations should be asynchronous

## Documentation Requirements

### Code Documentation
```rust
/// Creates a new user account with the specified role.
/// 
/// # Arguments
/// * `request` - User creation request containing username, password, and role
/// 
/// # Returns
/// * `Ok(UserInfo)` - Successfully created user information
/// * `Err(String)` - Error message if creation fails
/// 
/// # Security
/// This function requires Administrator role and validates all inputs
/// before creating the user account. Passwords are hashed using bcrypt.
pub async fn create_user(request: CreateUserRequest) -> Result<UserInfo, String> {
    // Implementation...
}
```

```typescript
/**
 * Handles user creation form submission with validation and error handling.
 * 
 * @param userData - Form data containing user details
 * @returns Promise that resolves when user is successfully created
 * 
 * @throws Will display error notification if creation fails
 */
const handleUserCreate = async (userData: CreateUserRequest): Promise<void> => {
  // Implementation...
};
```

### README Updates
All significant features must include:
- Usage examples
- Configuration options
- Known limitations
- Security considerations

This comprehensive coding standards document ensures consistent, secure, and maintainable code across the entire Ferrocodex application.