# FerroCodex Fullstack Architecture Document

### 1. Introduction

This document outlines the complete fullstack architecture for FerroCodex (Secure OT Configuration Management Platform), including the backend systems, frontend implementation, and their integration. The v0.4.0 release introduces the Asset Identity Vault feature set, providing comprehensive credential and access management for industrial assets. This evolution extends the platform from asset recovery to complete lifecycle security management. It serves as the single source of truth for AI-driven development, ensuring consistency across the entire technology stack.

#### Starter Template or Existing Project

The project is built using the Tauri framework, which integrates a Rust-based backend with a web-based frontend. This choice was finalized in the PRD and serves as our foundational "starter," providing a secure, performant, and cross-platform application shell from the outset.

#### Change Log

|Date|Version|Description|Author|
|---|---|---|---|
|2025-07-26|0.4.0|Architectural updates for Asset Identity Vault.|Winston (Architect)|
|2025-07-22|0.3.0|Architectural updates for Firmware Management.|Winston (Architect)|
|2025-07-18|1.0|Initial draft based on PRD and UI/UX Spec.|Winston (Architect)|

---

### 2. High Level Architecture

#### Technical Summary

The system is a cross-platform desktop application built using the Tauri framework, which features a Rust backend for maximum security and performance, and a React frontend for a polished user interface. It operates primarily as a modular monolith in an offline-first model, storing all data in a local, encrypted SQLite database.

The architectural evolution continues in v0.4.0 with the introduction of the Asset Identity Vault, a comprehensive credential management system that securely stores and manages authentication data for industrial assets. This builds upon the hybrid storage model established in v0.3.0, where structured metadata is stored in the encrypted SQLite database while large binary files (firmware) are stored as encrypted files on the native file system. The vault uses military-grade encryption with hardware security module integration where available, ensuring maximum protection for sensitive credential data.

A monorepo structure will manage the codebase. For the optional, intermittent sync feature, the application will communicate with a secure, serverless backend hosted on AWS, ensuring scalability and cost-efficiency. The architecture prioritizes security, data integrity, and a responsive, intuitive experience for OT engineers.

#### Platform and Infrastructure Choice

- **Platform:** AWS (Amazon Web Services) will be used for the optional sync and update functionality.

- **Key Services:** AWS Lambda (for compute), API Gateway (for the sync endpoint), S3 (for software update storage), and Cognito (for potential future cloud identity services).

- **Deployment Host and Regions:** The desktop application is self-hosted by the user. The serverless backend will be deployed to `us-east-1` and `eu-west-1` for redundancy.

#### Repository Structure

- **Structure:** Monorepo.

- **Monorepo Tool:** Turborepo is recommended to manage workspaces and optimize build processes.

- **Package Organization:** The monorepo will contain separate packages for the Tauri application (`apps/desktop`) and any future cloud infrastructure or shared libraries (`packages/shared-types`).

#### High Level Architecture Diagram

```mermaid
graph TD
    subgraph User's Environment
        A[User: OT Engineer] -- Interacts with --> B[Tauri Desktop App];
        B -- Contains --> C[React UI];
        B -- Contains --> D[Rust Core Logic];
        D -- Reads/Writes Metadata --> E[Encrypted SQLite DB];
        D -- Reads/Writes Large Files --> G[Encrypted File Storage (Firmware)];
        D -- Manages Credentials --> V[Asset Identity Vault];
        V -- Encrypted Storage --> VC[Vault Credential Store];
        V -- HSM Integration --> VH[Hardware Security Module];
    end

    subgraph AWS Cloud (Optional Sync)
        F[API Gateway] --> H[AWS Lambda];
        H --> I[Amazon S3];
    end

    B -- User-Initiated Sync --> F;
```

#### Architectural Patterns

- **Hybrid Storage Model:** Using a transactional SQL database for structured metadata and the native file system for storing large, unstructured binary files.

- **Asset Identity Vault:** A secure credential management system with hardware security module integration and zero-trust architecture for asset authentication data.

- **Firmware Analysis Engine:** The Rust core will integrate the binwalk library to perform automated analysis on uploaded firmware files.

- **Modular Monolith (Desktop App):** The core application is a single deployable unit, but its internal code will be structured in a modular way to ensure maintainability and separation of concerns.

- **Serverless (Cloud Sync):** The backend for handling software updates and optional telemetry will be built using serverless functions to ensure it is scalable and cost-effective.

- **Component-Based UI:** The React frontend will be built as a collection of reusable, stateless, and well-defined components.

- **Repository Pattern (Rust Core):** The Rust backend will use the repository pattern to abstract the database logic from the core business logic.

---

### 3. Tech Stack

|Category|Technology|Version|Purpose|Rationale|
|---|---|---|---|---|
|**Frontend Language**|TypeScript|`~5.4.5`|Language for UI development|Provides strong typing to reduce errors and improve maintainability.|
|**Frontend Framework**|React|`~18.3.1`|UI library for building components|Robust ecosystem, excellent performance, and pairs well with Tauri.|
|**UI Component Lib**|Ant Design (AntD)|`~5.17.4`|Pre-built UI components|Provides a professional, data-dense look and feel out of the box, accelerating development.|
|**State Management**|Zustand|`~4.5.2`|Manages UI state|A simple, lightweight, and unopinionated state management solution that avoids boilerplate.|
|**Backend Language**|Rust|`~1.78.0`|Core application logic, security|Guarantees memory safety and world-class performance, ideal for a security-critical app.|
|**App Framework**|Tauri|`~2.0.0`|Cross-platform desktop app shell|Unifies Rust backend and web frontend into a small, secure, and fast native binary.|
|**API Style**|Tauri IPC / REST|`N/A`|FE/BE Communication|Tauri's Inter-Process Communication for the desktop app; REST for the optional cloud sync.|
|**Database**|SQLite|`~3.45.3`|Local, embedded data storage|A serverless, self-contained, and reliable database perfect for offline desktop applications.|
|**DB Access (Rust)**|`rusqlite` crate|`~0.31.0`|Rust interface for SQLite|Provides a safe and idiomatic way to interact with the SQLite database from the Rust core.|
|**Password Hashing**|`bcrypt` crate|`~0.17.0`|Securely hash user passwords|Industry-standard library for securing user credentials at rest.|
|**Firmware Analysis**|`binwalk` crate|`~3.1.0`|Firmware analysis & metadata extraction|Enables automated firmware analysis and metadata extraction for v0.3.0 features.|
|**Credential Encryption**|`aes-gcm` crate|`~0.10.3`|Advanced encryption for vault credentials|Military-grade AES-256-GCM encryption for credential data in the Asset Identity Vault.|
|**Key Derivation**|`argon2` crate|`~0.5.3`|Secure key derivation for vault|Memory-hard key derivation function for vault master keys and credential encryption.|
|**HSM Integration**|`pkcs11` crate|`~0.8.0`|Hardware security module support|Enables integration with hardware security modules for enhanced key protection.|
|**Frontend Testing**|Vitest|`~1.6.0`|Unit & Integration testing for UI|Modern, fast, and Jest-compatible test runner that integrates seamlessly with Vite.|
|**Backend Testing**|Rust Test Suite|`(built-in)`|Unit & Integration testing for core|Rust's powerful, built-in testing capabilities are sufficient and idiomatic.|
|**IaC Tool**|AWS CDK|`~2.144.0`|Infrastructure as Code for AWS|Define cloud infrastructure programmatically in TypeScript for reliability and repeatability.|
|**CI / CD**|GitHub Actions|`N/A`|Automated builds, tests, releases|Ubiquitous, powerful, and well-integrated with source control.|
|**Monitoring**|AWS CloudWatch|`N/A`|Monitor serverless sync functions|Native AWS solution for logging and monitoring the optional backend.|
|**Logging (Rust)**|`tracing` crate|`~0.1.40`|Structured application logging|A modern and powerful logging framework for Rust applications.|

---

### 4. Data Models

#### v0.4.0 Asset Identity Vault Models

The Asset Identity Vault introduces comprehensive credential management capabilities with military-grade security standards. The following models support secure storage, retrieval, and management of authentication data for industrial assets.

##### Credential Model

**Purpose:** Stores encrypted authentication credentials for asset access with comprehensive metadata and audit trails.

**Attributes:**
- `id`: Unique identifier (UUID v4)
- `asset_id`: Foreign key linking to the asset
- `author_id`: User who created/last modified the credential
- `name`: Human-readable credential name
- `credential_type`: Type of credential (password, certificate, ssh_key, api_key, custom)
- `encrypted_data`: AES-256-GCM encrypted credential payload
- `encryption_key_id`: Reference to the encryption key used
- `metadata`: JSON object for credential-specific metadata
- `tags`: Array of tags for organization and filtering
- `expires_at`: Optional expiration timestamp
- `last_used_at`: Timestamp of last successful authentication
- `usage_count`: Number of times credential has been used
- `is_active`: Boolean flag for credential status
- `created_at`: Creation timestamp
- `updated_at`: Last modification timestamp

**TypeScript Interface:**
```typescript
interface Credential {
  id: string;
  asset_id: string;
  author_id: string;
  name: string;
  credential_type: CredentialType;
  encrypted_data: string;
  encryption_key_id: string;
  metadata: Record<string, any>;
  tags: string[];
  expires_at?: string;
  last_used_at?: string;
  usage_count: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

type CredentialType = 'password' | 'certificate' | 'ssh_key' | 'api_key' | 'custom';
```

##### VaultKey Model

**Purpose:** Manages encryption keys for the Asset Identity Vault with rotation capabilities and HSM integration.

**Attributes:**
- `id`: Unique key identifier
- `key_type`: Type of key (master, credential, backup)
- `algorithm`: Encryption algorithm used (AES-256-GCM)
- `encrypted_key_data`: HSM or software-encrypted key material
- `hsm_key_id`: Hardware Security Module key reference (if applicable)
- `created_at`: Key creation timestamp
- `expires_at`: Key expiration timestamp
- `is_active`: Whether the key is currently active
- `rotation_count`: Number of times this key has been rotated

**TypeScript Interface:**
```typescript
interface VaultKey {
  id: string;
  key_type: KeyType;
  algorithm: string;
  encrypted_key_data?: string;
  hsm_key_id?: string;
  created_at: string;
  expires_at?: string;
  is_active: boolean;
  rotation_count: number;
}

type KeyType = 'master' | 'credential' | 'backup';
```

##### VaultAccess Model

**Purpose:** Tracks and audits all access attempts to vault credentials with comprehensive logging.

**Attributes:**
- `id`: Unique access log identifier
- `credential_id`: Reference to accessed credential
- `user_id`: User who attempted access
- `access_type`: Type of access (read, decrypt, delete, rotate)
- `success`: Whether the access attempt was successful
- `ip_address`: Source IP address of the request
- `user_agent`: User agent string for audit purposes
- `failure_reason`: Reason for failed access attempts
- `accessed_at`: Timestamp of access attempt

**TypeScript Interface:**
```typescript
interface VaultAccess {
  id: string;
  credential_id: string;
  user_id: string;
  access_type: AccessType;
  success: boolean;
  ip_address?: string;
  user_agent?: string;
  failure_reason?: string;
  accessed_at: string;
}

type AccessType = 'read' | 'decrypt' | 'delete' | 'rotate' | 'export';
```

##### VaultPolicy Model

**Purpose:** Defines access control policies and security requirements for vault operations.

**Attributes:**
- `id`: Unique policy identifier
- `name`: Human-readable policy name
- `description`: Policy description and purpose
- `rules`: JSON object defining access rules and restrictions
- `applies_to`: Scope of policy application (global, role-based, asset-specific)
- `priority`: Policy priority for conflict resolution
- `is_active`: Whether the policy is currently enforced
- `created_by`: User who created the policy
- `created_at`: Policy creation timestamp
- `updated_at`: Last policy modification timestamp

**TypeScript Interface:**
```typescript
interface VaultPolicy {
  id: string;
  name: string;
  description: string;
  rules: PolicyRules;
  applies_to: PolicyScope;
  priority: number;
  is_active: boolean;
  created_by: string;
  created_at: string;
  updated_at: string;
}

interface PolicyRules {
  max_concurrent_access?: number;
  require_mfa?: boolean;
  allowed_ip_ranges?: string[];
  time_restrictions?: TimeRestriction[];
  rotation_requirements?: RotationPolicy;
}

interface TimeRestriction {
  days_of_week: number[];
  start_time: string;
  end_time: string;
  timezone: string;
}

interface RotationPolicy {
  max_age_days: number;
  warning_days: number;
  auto_rotate: boolean;
}

type PolicyScope = 'global' | 'role_based' | 'asset_specific';
```

### Enhanced Asset Model (v0.5.0)

**Purpose**: Represents both organizational folders and individual devices in a hierarchical structure with customizable metadata.

**Attributes:**
- `id` (string): Unique identifier (UUID v4)
- `name` (string): Asset name following cybersecurity conventions
- `description` (string): Asset description (0-500 chars)
- `parent_asset_id` (string?): Parent folder reference for hierarchy
- `asset_type` (AssetType): Either 'folder' or 'device'
- `metadata` (object): JSON object with custom fields
- `schema_id` (string?): Reference to metadata validation schema
- `security_classification` (string): Security level (PUBLIC, INTERNAL, CONFIDENTIAL, RESTRICTED)
- `created_by` (number): User ID of creator
- `created_at` (string): ISO 8601 timestamp
- `updated_at` (string): Last modification timestamp

**TypeScript Interface:**
```typescript
interface Asset {
  id: string;
  name: string;
  description: string;
  parent_asset_id?: string;
  asset_type: AssetType;
  metadata: AssetMetadata;
  schema_id?: string;
  security_classification: SecurityLevel;
  created_by: number;
  created_at: string;
  updated_at: string;
}

interface AssetInfo extends Asset {
  created_by_username: string;
  parent_asset_name?: string;
  child_count: number;
  configuration_count: number;
  firmware_count: number;
  path: string[]; // Full hierarchy path
}

interface AssetMetadata {
  ip_address?: string;
  location?: string;
  install_date?: string;
  facility?: string;
  notes?: string;
  [key: string]: any; // Custom fields
}

type AssetType = 'folder' | 'device';
type SecurityLevel = 'PUBLIC' | 'INTERNAL' | 'CONFIDENTIAL' | 'RESTRICTED';
```

**Business Rules:**
- Asset names must follow cybersecurity naming pattern
- Folder assets cannot have configurations or firmware
- Device assets can have multiple configurations and firmware
- Hierarchical depth unlimited but UI optimized for reasonable levels
- Metadata validation through JSON Schema when schema_id provided

### Asset Schema Model (v0.5.0)

**Purpose**: Defines validation rules and field definitions for customizable asset metadata.

**Attributes:**
- `id` (string): Unique schema identifier
- `schema_name` (string): Human-readable schema name
- `description` (string): Schema purpose and usage
- `json_schema` (object): JSON Schema specification for validation
- `is_system_schema` (boolean): Whether schema is system-provided
- `created_by` (number): User ID of creator
- `created_at` (string): Creation timestamp
- `updated_at` (string): Last modification timestamp

**TypeScript Interface:**
```typescript
interface AssetSchema {
  id: string;
  schema_name: string;
  description: string;
  json_schema: object;
  is_system_schema: boolean;
  created_by: number;
  created_at: string;
  updated_at: string;
}
```

_(This section also contains the detailed definitions for the User, Branch, ConfigurationVersion, and FirmwareVersion models from previous versions.)_

---

### 5. API Specification

#### v0.4.0 Asset Identity Vault API Commands

The Asset Identity Vault extends the existing Tauri IPC API with comprehensive credential management operations. All vault operations require appropriate user permissions and generate detailed audit logs.

##### Vault Management Commands

```rust
// Core vault operations
#[tauri::command]
async fn vault_initialize(master_password: String) -> Result<VaultStatus, String>;

#[tauri::command]
async fn vault_unlock(password: String, mfa_token: Option<String>) -> Result<VaultSession, String>;

#[tauri::command]
async fn vault_lock() -> Result<(), String>;

#[tauri::command]
async fn vault_status() -> Result<VaultStatus, String>;

#[tauri::command]
async fn vault_change_master_password(
    current_password: String,
    new_password: String,
    mfa_token: Option<String>
) -> Result<(), String>;
```

##### Credential Management Commands

```rust
// Credential CRUD operations
#[tauri::command]
async fn credential_create(
    credential_data: CreateCredentialRequest
) -> Result<Credential, String>;

#[tauri::command]
async fn credential_get(
    credential_id: String,
    decrypt: bool
) -> Result<Credential, String>;

#[tauri::command]
async fn credential_list(
    asset_id: Option<String>,
    filters: CredentialFilters
) -> Result<Vec<Credential>, String>;

#[tauri::command]
async fn credential_update(
    credential_id: String,
    updates: UpdateCredentialRequest
) -> Result<Credential, String>;

#[tauri::command]
async fn credential_delete(credential_id: String) -> Result<(), String>;

#[tauri::command]
async fn credential_rotate(
    credential_id: String,
    new_credential_data: RotateCredentialRequest
) -> Result<Credential, String>;
```

##### Key Management Commands

```rust
// Encryption key operations
#[tauri::command]
async fn vault_key_rotate(key_type: KeyType) -> Result<VaultKey, String>;

#[tauri::command]
async fn vault_key_backup(
    backup_location: String,
    encryption_password: String
) -> Result<String, String>;

#[tauri::command]
async fn vault_key_restore(
    backup_file: String,
    decryption_password: String
) -> Result<(), String>;

#[tauri::command]
async fn vault_hsm_configure(hsm_config: HsmConfiguration) -> Result<(), String>;
```

##### Access Control Commands

```rust
// Policy and access management
#[tauri::command]
async fn vault_policy_create(policy: CreatePolicyRequest) -> Result<VaultPolicy, String>;

#[tauri::command]
async fn vault_policy_list() -> Result<Vec<VaultPolicy>, String>;

#[tauri::command]
async fn vault_policy_update(
    policy_id: String,
    updates: UpdatePolicyRequest
) -> Result<VaultPolicy, String>;

#[tauri::command]
async fn vault_policy_delete(policy_id: String) -> Result<(), String>;

#[tauri::command]
async fn vault_access_log(
    filters: AccessLogFilters
) -> Result<Vec<VaultAccess>, String>;
```

##### TypeScript Request/Response Types

```typescript
// Request types
interface CreateCredentialRequest {
  asset_id: string;
  name: string;
  credential_type: CredentialType;
  credential_data: any;
  metadata?: Record<string, any>;
  tags?: string[];
  expires_at?: string;
}

interface UpdateCredentialRequest {
  name?: string;
  credential_data?: any;
  metadata?: Record<string, any>;
  tags?: string[];
  expires_at?: string;
  is_active?: boolean;
}

interface RotateCredentialRequest {
  new_credential_data: any;
  revoke_old: boolean;
  notify_users: boolean;
}

interface CredentialFilters {
  credential_type?: CredentialType;
  tags?: string[];
  is_active?: boolean;
  expires_before?: string;
  search_term?: string;
}

// Response types
interface VaultStatus {
  is_initialized: boolean;
  is_unlocked: boolean;
  hsm_available: boolean;
  encryption_algorithm: string;
  total_credentials: number;
  expired_credentials: number;
  last_backup?: string;
}

interface VaultSession {
  session_id: string;
  user_id: string;
  expires_at: string;
  permissions: string[];
}

interface HsmConfiguration {
  provider: string;
  slot_id: number;
  pin: string;
  key_label: string;
}
```

#### v0.5.0 Asset Management Commands

```rust
// Hierarchical asset operations
#[tauri::command]
async fn asset_create_folder(
    name: String,
    description: Option<String>,
    parent_id: Option<String>
) -> Result<Asset, String>;

#[tauri::command]
async fn asset_create_device(
    name: String,
    description: Option<String>,
    parent_id: Option<String>,
    metadata: AssetMetadata,
    schema_id: Option<String>
) -> Result<Asset, String>;

#[tauri::command]
async fn asset_get_hierarchy(
    root_id: Option<String>
) -> Result<Vec<AssetHierarchyNode>, String>;

#[tauri::command]
async fn asset_move(
    asset_id: String,
    new_parent_id: Option<String>
) -> Result<Asset, String>;

#[tauri::command]
async fn asset_update_metadata(
    asset_id: String,
    metadata: AssetMetadata
) -> Result<Asset, String>;

// Schema management
#[tauri::command]
async fn schema_create(
    schema_data: CreateSchemaRequest
) -> Result<AssetSchema, String>;

#[tauri::command]
async fn schema_list() -> Result<Vec<AssetSchema>, String>;

#[tauri::command]
async fn schema_validate_metadata(
    schema_id: String,
    metadata: AssetMetadata
) -> Result<ValidationResult, String>;

// Search and filtering
#[tauri::command]
async fn asset_search(
    query: String,
    filters: AssetSearchFilters
) -> Result<Vec<AssetSearchResult>, String>;
```

**TypeScript Types for Asset Management:**
```typescript
interface AssetHierarchyNode {
  asset: Asset;
  children: AssetHierarchyNode[];
  level: number;
  has_children: boolean;
}

interface CreateSchemaRequest {
  schema_name: string;
  description: string;
  json_schema: object;
}

interface ValidationResult {
  is_valid: boolean;
  errors: ValidationError[];
}

interface ValidationError {
  field: string;
  message: string;
  value?: any;
}

interface AssetSearchFilters {
  asset_type?: AssetType;
  parent_id?: string;
  security_classification?: SecurityLevel;
  metadata_filters?: Record<string, any>;
  created_after?: string;
  created_before?: string;
}

interface AssetSearchResult {
  asset: Asset;
  path: string[];
  relevance_score: number;
  matching_fields: string[];
}
```

_(This section also contains the existing definitions for the Local API via Tauri IPC and the OpenAPI 3.0 specification for the optional Cloud Sync REST API.)_

---

### 6. Components

#### Existing Components

_(This section details the logical components of the application: UI (React), IPC Handler (Rust), Core Logic (Rust), Database Module (Rust), and Security Module (Rust), complete with an interaction diagram.)_

#### New Component: Firmware Analyzer (Rust)

- **Responsibility:** To analyze firmware binaries using the binwalk crate to extract metadata.
- **Dependencies:** The binwalk Rust crate.

#### New Component: Asset Identity Vault (Rust)

- **Responsibility:** Secure storage, encryption, and management of authentication credentials for industrial assets.
- **Key Features:**
  - Military-grade AES-256-GCM encryption for credential data
  - Hardware Security Module (HSM) integration for key management
  - Comprehensive audit logging and access tracking
  - Policy-based access control with role-based permissions
  - Automatic credential rotation and expiration management
- **Dependencies:** `aes-gcm`, `argon2`, `pkcs11`, Database Module, Security Module.

#### New Component: Vault UI Components (React)

- **Responsibility:** Provide intuitive user interfaces for vault management and credential operations.
- **Key Components:**
  - `VaultDashboard`: Overview of vault status and credential health
  - `CredentialManager`: CRUD operations for credentials with secure input handling
  - `PolicyEditor`: Visual policy creation and management interface
  - `AuditViewer`: Comprehensive access logs and security reporting
  - `KeyManagement`: HSM configuration and key rotation interfaces
- **Dependencies:** Ant Design components, Vault API integration, React hooks for state management.

#### Core Logic (Rust) - Updated Dependencies

- **Dependencies:** Database Module, Security Module, Firmware Analyzer, Asset Identity Vault.

---

### 7. Core Workflows

_(This section contains the sequence diagram illustrating the "Restore Golden Image" workflow, showing how all the internal components interact to complete the task.)_

---

### 8. Database Schema

#### v0.4.0 Schema Updates

This schema adds comprehensive Asset Identity Vault tables for secure credential management, building upon the existing structure.

```sql
-- Existing tables remain unchanged from previous versions
CREATE TABLE users (
    -- ... existing schema ...
);

-- Enhanced hierarchical assets with metadata
CREATE TABLE assets (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT DEFAULT '',
    parent_asset_id TEXT,
    asset_type TEXT NOT NULL CHECK(asset_type IN ('folder', 'device')),
    metadata TEXT NOT NULL DEFAULT '{}', -- JSON object for custom fields
    schema_id TEXT,
    security_classification TEXT NOT NULL DEFAULT 'INTERNAL'
        CHECK(security_classification IN ('PUBLIC', 'INTERNAL', 'CONFIDENTIAL', 'RESTRICTED')),
    created_by INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    FOREIGN KEY (parent_asset_id) REFERENCES assets(id) ON DELETE CASCADE,
    FOREIGN KEY (schema_id) REFERENCES asset_schemas(id) ON DELETE SET NULL,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE RESTRICT,

    -- Cybersecurity naming constraint
    CHECK (name GLOB '[A-Z0-9][A-Z0-9_-]*' AND length(name) BETWEEN 3 AND 50)
);

CREATE INDEX idx_assets_parent ON assets(parent_asset_id);
CREATE INDEX idx_assets_type ON assets(asset_type);
CREATE INDEX idx_assets_name ON assets(name);
CREATE INDEX idx_assets_classification ON assets(security_classification);

-- Asset metadata schemas for validation
CREATE TABLE asset_schemas (
    id TEXT PRIMARY KEY NOT NULL,
    schema_name TEXT NOT NULL UNIQUE,
    description TEXT DEFAULT '',
    json_schema TEXT NOT NULL, -- JSON Schema specification
    is_system_schema BOOLEAN NOT NULL DEFAULT FALSE,
    created_by INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_schemas_name ON asset_schemas(schema_name);
CREATE INDEX idx_schemas_system ON asset_schemas(is_system_schema);

-- Asset hierarchy materialized path for efficient queries
CREATE TABLE asset_paths (
    asset_id TEXT NOT NULL,
    ancestor_id TEXT NOT NULL,
    depth INTEGER NOT NULL,

    PRIMARY KEY (asset_id, ancestor_id),
    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
    FOREIGN KEY (ancestor_id) REFERENCES assets(id) ON DELETE CASCADE
);

CREATE INDEX idx_asset_paths_ancestor ON asset_paths(ancestor_id);
CREATE INDEX idx_asset_paths_depth ON asset_paths(depth);

CREATE TABLE branches (
    -- ... existing schema ...
);

CREATE TABLE configuration_versions (
    -- ... existing schema ...
);

CREATE TABLE firmware_versions (
    -- ... existing schema from v0.3.0 ...
);

-- New tables for Asset Identity Vault

-- Vault encryption keys with HSM support
CREATE TABLE vault_keys (
    id TEXT PRIMARY KEY NOT NULL,
    key_type TEXT NOT NULL CHECK(key_type IN ('master', 'credential', 'backup')),
    algorithm TEXT NOT NULL DEFAULT 'AES-256-GCM',
    encrypted_key_data TEXT, -- NULL if stored in HSM
    hsm_key_id TEXT, -- Reference to HSM key if applicable
    created_at TEXT NOT NULL,
    expires_at TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    rotation_count INTEGER NOT NULL DEFAULT 0,
    
    INDEX idx_vault_keys_type_active (key_type, is_active),
    INDEX idx_vault_keys_created (created_at)
);

-- Encrypted credential storage
CREATE TABLE credentials (
    id TEXT PRIMARY KEY NOT NULL,
    asset_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    name TEXT NOT NULL,
    credential_type TEXT NOT NULL CHECK(credential_type IN ('password', 'certificate', 'ssh_key', 'api_key', 'custom')),
    encrypted_data TEXT NOT NULL, -- AES-256-GCM encrypted credential payload
    encryption_key_id TEXT NOT NULL,
    metadata TEXT, -- JSON object for credential-specific metadata
    tags TEXT, -- JSON array of tags
    expires_at TEXT,
    last_used_at TEXT,
    usage_count INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    
    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
    FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE RESTRICT,
    FOREIGN KEY (encryption_key_id) REFERENCES vault_keys(id) ON DELETE RESTRICT,
    
    INDEX idx_credentials_asset (asset_id),
    INDEX idx_credentials_type (credential_type),
    INDEX idx_credentials_active (is_active),
    INDEX idx_credentials_expires (expires_at),
    INDEX idx_credentials_tags (tags)
);

-- Comprehensive access audit logging
CREATE TABLE vault_access (
    id TEXT PRIMARY KEY NOT NULL,
    credential_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    access_type TEXT NOT NULL CHECK(access_type IN ('read', 'decrypt', 'delete', 'rotate', 'export')),
    success BOOLEAN NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    failure_reason TEXT,
    accessed_at TEXT NOT NULL,
    
    FOREIGN KEY (credential_id) REFERENCES credentials(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    
    INDEX idx_vault_access_credential (credential_id),
    INDEX idx_vault_access_user (user_id),
    INDEX idx_vault_access_timestamp (accessed_at),
    INDEX idx_vault_access_success (success)
);

-- Access control policies
CREATE TABLE vault_policies (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    rules TEXT NOT NULL, -- JSON object defining access rules
    applies_to TEXT NOT NULL CHECK(applies_to IN ('global', 'role_based', 'asset_specific')),
    priority INTEGER NOT NULL DEFAULT 100,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE RESTRICT,
    
    INDEX idx_vault_policies_active (is_active),
    INDEX idx_vault_policies_priority (priority),
    INDEX idx_vault_policies_applies_to (applies_to)
);

-- Policy assignments for fine-grained access control
CREATE TABLE vault_policy_assignments (
    id TEXT PRIMARY KEY NOT NULL,
    policy_id TEXT NOT NULL,
    target_type TEXT NOT NULL CHECK(target_type IN ('user', 'role', 'asset')),
    target_id TEXT NOT NULL,
    assigned_by TEXT NOT NULL,
    assigned_at TEXT NOT NULL,
    
    FOREIGN KEY (policy_id) REFERENCES vault_policies(id) ON DELETE CASCADE,
    FOREIGN KEY (assigned_by) REFERENCES users(id) ON DELETE RESTRICT,
    
    UNIQUE(policy_id, target_type, target_id),
    INDEX idx_policy_assignments_target (target_type, target_id),
    INDEX idx_policy_assignments_policy (policy_id)
);

-- Credential rotation history for compliance
CREATE TABLE credential_rotation_history (
    id TEXT PRIMARY KEY NOT NULL,
    credential_id TEXT NOT NULL,
    old_key_id TEXT NOT NULL,
    new_key_id TEXT NOT NULL,
    rotation_type TEXT NOT NULL CHECK(rotation_type IN ('manual', 'automatic', 'emergency')),
    initiated_by TEXT NOT NULL,
    reason TEXT,
    rotated_at TEXT NOT NULL,
    
    FOREIGN KEY (credential_id) REFERENCES credentials(id) ON DELETE CASCADE,
    FOREIGN KEY (old_key_id) REFERENCES vault_keys(id) ON DELETE RESTRICT,
    FOREIGN KEY (new_key_id) REFERENCES vault_keys(id) ON DELETE RESTRICT,
    FOREIGN KEY (initiated_by) REFERENCES users(id) ON DELETE RESTRICT,
    
    INDEX idx_rotation_history_credential (credential_id),
    INDEX idx_rotation_history_timestamp (rotated_at)
);
```

_(The complete SQL DDL `CREATE TABLE` statements for all tables including indexes and constraints from previous versions are also included.)_

---

### 9. Unified Project Structure

_(This section contains the detailed ASCII tree diagram of the monorepo folder structure, showing the layout for the Tauri app, Rust backend, React frontend, and shared packages.)_

---

### 10. Development Workflow

_(This section outlines the prerequisites, initial setup commands (`pnpm install`), development commands (`pnpm dev`), and the contents of the `.env.example` file.)_

---

### 11. Deployment Architecture

_(This section details the strategy for creating native installers via GitHub Releases, deploying the serverless backend via AWS CDK, the CI/CD pipeline steps, and the definitions for Development, Staging, and Production environments.)_

---

### 12. Security and Performance

#### v0.4.0 Enhanced Security Architecture

The Asset Identity Vault introduces military-grade security enhancements that extend the existing security model with comprehensive credential protection and access control.

##### Vault Security Features

**Multi-Layer Encryption Architecture:**
- **Data at Rest:** AES-256-GCM encryption for all credential data with unique per-credential keys
- **Key Management:** Hierarchical key structure with master keys, credential keys, and backup keys
- **HSM Integration:** Hardware Security Module support for tamper-resistant key storage
- **Key Derivation:** Argon2id for password-based key derivation with configurable memory and time parameters

**Zero-Trust Access Control:**
- **Policy-Based Authorization:** Granular access policies with time-based restrictions
- **Multi-Factor Authentication:** TOTP/HOTP support for vault access operations
- **Session Management:** Time-limited vault sessions with automatic lock mechanisms
- **IP Whitelisting:** Network-based access restrictions for sensitive operations

**Comprehensive Audit Framework:**
- **Access Logging:** All vault operations logged with user attribution and timestamps
- **Integrity Monitoring:** Cryptographic checksums for detecting unauthorized modifications
- **Failed Access Tracking:** Brute force protection with configurable lockout policies
- **Compliance Reporting:** Automated reports for security audits and compliance requirements

##### Security Implementation Details

**Credential Encryption Process:**
1. User provides master password → Argon2id key derivation
2. Master key unlocks credential-specific encryption keys
3. Individual credentials encrypted with AES-256-GCM
4. Encrypted data includes authentication tags for integrity verification

**HSM Integration Workflow:**
1. HSM availability detection during vault initialization
2. Master keys stored in HSM when available, software fallback otherwise
3. All cryptographic operations performed within HSM security boundary
4. Key rotation through HSM APIs with audit trail generation

**Access Control Enforcement:**
1. Policy evaluation engine processes all vault access requests
2. Time-based restrictions enforced at the system level
3. Rate limiting prevents brute force attacks
4. Session tokens validated for every vault operation

##### Performance Optimizations

**Vault-Specific Performance Features:**
- **Lazy Decryption:** Credentials decrypted only when explicitly requested
- **Connection Pooling:** Efficient HSM connection management for high-throughput operations
- **Caching Strategy:** Temporary credential caching with automatic expiration
- **Background Rotation:** Non-blocking automatic credential rotation processes

**Database Optimization for Vault Operations:**
- **Indexed Queries:** Optimized indexes for credential lookups and audit searches
- **Batch Operations:** Bulk credential operations for improved performance
- **Transaction Management:** ACID compliance for multi-step vault operations
- **Query Optimization:** Prepared statements and query plan optimization

_(This section also includes the existing security requirements for the frontend and backend, including a strict CSP, input validation, and secure authentication, along with performance optimization strategies like list virtualization and non-blocking backend operations.)_

---

### 13. Testing Strategy

_(This section defines the testing pyramid, the organization for backend and frontend tests, and provides conceptual examples for both component tests and backend unit tests.)_

---

### 14. Coding Standards

_(This section lists the critical, mandatory rules for AI developers, including type safety, explicit error handling, and centralized state management. It also includes a table of naming conventions.)_

---

### 15. Error Handling Strategy

_(This section provides the unified error handling strategy based on the `Result` type, including the shared `AppError` format, backend and frontend implementation examples, and an error flow sequence diagram.)_

---

### 16. Monitoring and Observability

_(This section defines the strategy for monitoring the application, using local file-based logging for the desktop app and AWS CloudWatch for the optional cloud backend. It also lists the key metrics to be collected.)_

---

This concludes the FerroCodex Fullstack Architecture Document v0.4.0. All required artifacts—the Project Brief, PRD, UI/UX Specification, and this Architecture Document—are now complete and updated to include the comprehensive Asset Identity Vault features.

The v0.4.0 release represents a significant evolution in the platform's security capabilities, introducing military-grade credential management, Hardware Security Module integration, and comprehensive audit frameworks. These enhancements transform FerroCodex from a configuration management tool into a complete asset lifecycle security platform.

The project architecture is fully specified and ready for Epic 4 development. The next step is to move to an IDE environment where a **Scrum Master** will begin creating user stories for the **Developer** to implement the Asset Identity Vault feature set.
