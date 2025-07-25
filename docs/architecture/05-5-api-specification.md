# API Specification

This document provides the complete API specification for FerroCodex, including the Local API via Tauri IPC and the optional Cloud Sync REST API.

## Table of Contents

1. [Local API (Tauri IPC)](#local-api-tauri-ipc)
2. [Authentication & Authorization](#authentication--authorization)
3. [Error Handling](#error-handling)
4. [Rate Limiting](#rate-limiting)
5. [Cloud Sync API (Future)](#cloud-sync-api-future)

## Local API (Tauri IPC)

The local API uses Tauri's Inter-Process Communication (IPC) mechanism to communicate between the React frontend and Rust backend. All commands follow a consistent pattern and return results wrapped in Rust's `Result<T, String>` type.

### General Commands

#### greet

Simple test command for verifying IPC communication.

```typescript
invoke('greet', { name: string }) => Promise<string>
```

**Example:**

```typescript
const message = await invoke('greet', { name: 'World' });
// Returns: "Hello, World! You've been greeted from Rust!"
```

### Database Management

#### initialize_database

Initializes the SQLite database in the application data directory.

```typescript
invoke('initialize_database') => Promise<boolean>
```

**Returns:** `true` if initialization successful
**Errors:** Database creation or migration failures

#### database_health_check

Checks if the database connection is healthy.

```typescript
invoke('database_health_check') => Promise<boolean>
```

**Returns:** `true` if database is accessible and healthy

#### is_first_launch

Determines if this is the first application launch (no admin users exist).

```typescript
invoke('is_first_launch') => Promise<boolean>
```

**Returns:** `true` if no administrator accounts exist

### Authentication & Session Management

#### create_admin_account

Creates the initial administrator account. Only works if no admin accounts exist.

```typescript
invoke('create_admin_account', {
  username: string,
  password: string
}) => Promise<LoginResponse>
```

**Response:**

```typescript
interface LoginResponse {
  token: string;
  user: UserInfo;
}

interface UserInfo {
  id: number;
  username: string;
  role: 'Administrator' | 'Engineer';
  is_active: boolean;
  created_at: string;
  last_login?: string;
}
```

**Validation:**

- Username: 3-50 alphanumeric characters, underscores, hyphens
- Password: Minimum 8 characters
- Fails if admin accounts already exist

#### login

Authenticates a user and creates a session.

```typescript
invoke('login', {
  username: string,
  password: string
}) => Promise<LoginResponse>
```

**Security:**

- Rate limited to prevent brute force attacks
- Failed attempts are tracked per username
- Account locked after 5 failed attempts for 15 minutes

#### logout

Invalidates the current session.

```typescript
invoke('logout', {
  token: string
}) => Promise<void>
```

#### check_session

Validates a session token and returns user information.

```typescript
invoke('check_session', {
  token: string
}) => Promise<UserInfo>
```

**Errors:** "Invalid or expired session" if token is invalid

### User Management

All user management commands require Administrator role.

#### create_engineer_user

Creates a new Engineer user account.

```typescript
invoke('create_engineer_user', {
  token: string,
  username: string,
  initial_password: string
}) => Promise<UserInfo>
```

**Rate Limited:** 10 operations per minute

#### list_users

Lists all users in the system.

```typescript
invoke('list_users', {
  token: string
}) => Promise<UserInfo[]>
```

#### deactivate_user

Deactivates a user account, preventing login.

```typescript
invoke('deactivate_user', {
  token: string,
  user_id: number
}) => Promise<void>
```

**Business Rules:**

- Cannot deactivate your own account
- Cannot deactivate if it would leave no active admins

#### reactivate_user

Reactivates a previously deactivated user account.

```typescript
invoke('reactivate_user', {
  token: string,
  user_id: number
}) => Promise<void>
```

### Asset Management

#### create_asset

Creates a new asset/equipment entry.

```typescript
invoke('create_asset', {
  token: string,
  name: string,
  description: string
}) => Promise<AssetInfo>
```

**Response:**

```typescript
interface AssetInfo {
  id: number;
  name: string;
  description: string;
  created_at: string;
  created_by: string;
  configuration_count: number;
  branch_count: number;
  golden_version?: ConfigurationVersion;
}
```

**Rate Limited:** 30 operations per minute

#### get_dashboard_assets

Retrieves assets for dashboard display.

```typescript
invoke('get_dashboard_assets', {
  token: string
}) => Promise<AssetInfo[]>
```

#### get_dashboard_stats

Retrieves statistical information for the dashboard.

```typescript
invoke('get_dashboard_stats', {
  token: string
}) => Promise<DashboardStats>
```

**Response:**

```typescript
interface DashboardStats {
  total_assets: number;
  total_configurations: number;
  total_users: number;
  recent_activity: ActivityItem[];
}
```

#### get_asset_details

Retrieves detailed information about a specific asset.

```typescript
invoke('get_asset_details', {
  token: string,
  asset_id: number
}) => Promise<AssetDetails>
```

**Response:**

```typescript
interface AssetDetails {
  info: AssetInfo;
  configurations: ConfigurationVersion[];
  branches: BranchInfo[];
}
```

### Configuration Management

#### import_configuration

Imports a configuration file for an asset.

```typescript
invoke('import_configuration', {
  token: string,
  asset_name: string,
  file_path: string,
  notes: string
}) => Promise<ImportConfigurationResult>
```

**Response:**

```typescript
interface ImportConfigurationResult {
  asset: AssetInfo;
  configuration: ConfigurationVersion;
}

interface ConfigurationVersion {
  id: number;
  asset_id: number;
  version_number: number;
  file_name: string;
  file_hash: string;
  file_size: number;
  notes: string;
  status: ConfigurationStatus;
  author: string;
  created_at: string;
  branch_id?: number;
  firmware_version_id?: number;
}
```

**Process:**

1. Creates asset if it doesn't exist
2. Reads and encrypts file content
3. Stores encrypted content in database
4. Creates version record with metadata
5. Logs operation in audit trail

#### get_configuration_versions

Retrieves all configuration versions for an asset.

```typescript
invoke('get_configuration_versions', {
  token: string,
  asset_id: number
}) => Promise<ConfigurationVersion[]>
```

### Branch Management

#### create_branch

Creates a new configuration branch.

```typescript
invoke('create_branch', {
  token: string,
  name: string,
  description?: string,
  asset_id: number,
  parent_branch_id?: number,
  base_version_id?: number
}) => Promise<BranchInfo>
```

**Response:**

```typescript
interface BranchInfo {
  id: number;
  name: string;
  description?: string;
  asset_id: number;
  parent_branch_id?: number;
  base_version_id?: number;
  created_by: string;
  created_at: string;
  is_active: boolean;
  version_count: number;
  latest_version?: ConfigurationVersion;
}
```

#### get_branches

Retrieves all branches for an asset.

```typescript
invoke('get_branches', {
  token: string,
  asset_id: number
}) => Promise<BranchInfo[]>
```

#### get_branch_details

Retrieves detailed information about a branch.

```typescript
invoke('get_branch_details', {
  token: string,
  branch_id: number
}) => Promise<BranchDetails>
```

**Response:**

```typescript
interface BranchDetails {
  info: BranchInfo;
  versions: ConfigurationVersion[];
  parent_branch?: BranchInfo;
  child_branches: BranchInfo[];
}
```

#### import_version_to_branch

Imports a new configuration version to a branch.

```typescript
invoke('import_version_to_branch', {
  token: string,
  branch_id: number,
  file_path: string,
  notes: string
}) => Promise<ConfigurationVersion>
```

#### get_branch_versions

Retrieves paginated versions for a branch.

```typescript
invoke('get_branch_versions', {
  token: string,
  branch_id: number,
  page?: number,
  limit?: number
}) => Promise<PaginatedBranchVersions>
```

**Response:**

```typescript
interface PaginatedBranchVersions {
  versions: ConfigurationVersion[];
  total: number;
  page: number;
  limit: number;
  has_more: boolean;
}
```

### Status Management

#### update_configuration_status

Updates the status of a configuration version.

```typescript
invoke('update_configuration_status', {
  token: string,
  version_id: number,
  new_status: ConfigurationStatus,
  change_reason?: string
}) => Promise<void>
```

**Status Values:**

- `Draft` - Initial state
- `InReview` - Under review
- `Approved` - Approved for use
- `Golden` - Master recovery version
- `Archived` - No longer in use

**Business Rules:**

- Only administrators can change from `Approved` back to `Draft`
- Only one `Golden` version per asset
- Previous `Golden` automatically becomes `Archived`

#### promote_to_golden

Promotes a configuration version to golden status.

```typescript
invoke('promote_to_golden', {
  token: string,
  version_id: number,
  promotion_reason?: string
}) => Promise<void>
```

**Requirements:**

- Version must be in `Approved` status
- Automatically archives previous golden version

### Export Operations

#### export_configuration_version

Exports a configuration version to a file.

```typescript
invoke('export_configuration_version', {
  token: string,
  version_id: number,
  export_path: string
}) => Promise<ExportResult>
```

**Response:**

```typescript
interface ExportResult {
  exported_path: string;
  file_size: number;
  file_hash: string;
}
```

**Process:**

1. Retrieves encrypted content from database
2. Decrypts content
3. Writes to specified path
4. Logs export in audit trail

### Firmware Management

#### upload_firmware

Uploads and analyzes a firmware file.

```typescript
invoke('upload_firmware', {
  token: string,
  file_path: string,
  notes?: string,
  asset_id?: number,
  firmware_version?: string,
  device_type?: string,
  manufacturer?: string,
  model?: string
}) => Promise<FirmwareUploadResult>
```

**Response:**

```typescript
interface FirmwareUploadResult {
  firmware: FirmwareInfo;
  analysis?: FirmwareAnalysis;
}

interface FirmwareInfo {
  id: number;
  file_name: string;
  file_size: number;
  file_hash: string;
  firmware_version?: string;
  device_type?: string;
  manufacturer?: string;
  model?: string;
  upload_date: string;
  uploaded_by: string;
  notes?: string;
  status: FirmwareStatus;
  asset_id?: number;
  analysis_status: AnalysisStatus;
}
```

**Process:**

1. Validates file exists and is readable
2. Copies and encrypts file to secure storage
3. Creates database record
4. Queues for automated analysis
5. Returns immediately with analysis pending

#### get_firmware_analysis

Retrieves firmware analysis results.

```typescript
invoke('get_firmware_analysis', {
  token: string,
  firmware_id: number
}) => Promise<FirmwareAnalysis>
```

**Response:**

```typescript
interface FirmwareAnalysis {
  firmware_id: number;
  status: AnalysisStatus;
  started_at?: string;
  completed_at?: string;
  file_type?: string;
  detected_architecture?: string;
  detected_os?: string;
  extracted_version?: string;
  security_findings?: SecurityFinding[];
  metadata?: Record<string, any>;
  error_message?: string;
}
```

#### link_firmware_to_configuration

Links firmware to a configuration version.

```typescript
invoke('link_firmware_to_configuration', {
  token: string,
  configuration_version_id: number,
  firmware_id: number,
  link_notes?: string
}) => Promise<void>
```

#### export_complete_recovery

Exports configuration with linked firmware for complete recovery.

```typescript
invoke('export_complete_recovery', {
  token: string,
  configuration_version_id: number,
  export_path: string
}) => Promise<RecoveryExportResult>
```

**Response:**

```typescript
interface RecoveryExportResult {
  export_directory: string;
  configuration_file: string;
  firmware_files: string[];
  manifest_file: string;
  total_size: number;
}
```

**Creates:**

- Directory with timestamp
- Configuration file
- Linked firmware files
- JSON manifest with checksums

## Authentication & Authorization

All API commands (except `greet`, `initialize_database`, `is_first_launch`, and `create_admin_account`) require authentication via session token.

### Session Token Format

- 32-character random alphanumeric string
- Created on successful login
- Valid for 8 hours of inactivity
- Automatically extended on each use

### Role-Based Access Control

| Operation | Administrator | Engineer |
|-----------|--------------|----------|
| User Management | ✓ | ✗ |
| Create Assets | ✓ | ✓ |
| Import Configurations | ✓ | ✓ |
| Update Status | ✓ | Limited |
| Promote to Golden | ✓ | ✗ |
| Delete Operations | ✓ | ✗ |
| View Operations | ✓ | ✓ |

## Error Handling

All errors follow a consistent format:

```typescript
interface ErrorResponse {
  error: string;  // User-friendly error message
  code?: string;  // Optional error code for client handling
}
```

### Common Error Messages

| Error | Description |
|-------|-------------|
| "Invalid or expired session" | Session token invalid or expired |
| "Insufficient permissions" | User lacks required role |
| "Resource not found" | Requested item doesn't exist |
| "Validation error: {details}" | Input validation failed |
| "Rate limit exceeded" | Too many requests |
| "Database error" | Internal database operation failed |

## Rate Limiting

Rate limiting is applied per user session to prevent abuse:

| Operation Type | Limit |
|----------------|--------|
| Authentication | 5 attempts per 15 minutes |
| User Creation | 10 per minute |
| Asset Creation | 30 per minute |
| File Operations | 60 per minute |
| Read Operations | 300 per minute |

Exceeded limits return: "Rate limit exceeded. Please try again later."

## Cloud Sync API (Future)

The optional cloud sync functionality will use a REST API hosted on AWS:

### Base URL

```
https://api.ferrocodex.com/v1
```

### Authentication

```http
Authorization: Bearer {jwt_token}
```

### Endpoints (Planned)

#### Software Updates

```http
GET /updates/check?version={current_version}&platform={platform}
```

#### Telemetry Submission

```http
POST /telemetry
Content-Type: application/json

{
  "events": [...],
  "timestamp": "2024-01-01T00:00:00Z"
}
```

#### Sync Status

```http
GET /sync/status
```

### Security

- All communication over HTTPS
- JWT tokens with short expiration
- Request signing for integrity
- Rate limiting per API key

---

This API specification provides the foundation for all client-server communication in FerroCodex. The design prioritizes security, consistency, and developer experience while maintaining the offline-first architecture that is core to the product's value proposition.
