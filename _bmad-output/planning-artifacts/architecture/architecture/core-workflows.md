# Core Workflows

This document illustrates the key workflows in FerroCodex, showing how internal components interact to complete critical tasks. These workflows demonstrate the end-to-end processes that users rely on for configuration and firmware management.

## 1. Restore Golden Image Workflow

The "Restore Golden Image" workflow is the core recovery operation in FerroCodex, allowing engineers to quickly restore equipment to a known-good configuration.

```mermaid
sequenceDiagram
    participant U as User
    participant UI as Dashboard UI
    participant Store as Zustand Store
    participant API as API Service
    participant IPC as Tauri IPC
    participant Auth as Auth Module
    participant Core as Core Logic
    participant DB as Database
    participant Crypto as Encryption
    participant Audit as Audit Log
    participant FS as File System

    U->>UI: Click "Restore Golden" on Asset
    UI->>Store: Set loading state
    Store->>API: exportConfigurationVersion(goldenVersionId)
    API->>IPC: invoke('export_configuration_version')
    
    IPC->>Auth: Validate session token
    Auth->>DB: Check session validity
    DB-->>Auth: Session valid
    Auth->>Auth: Check export permissions
    Auth-->>IPC: User authorized
    
    IPC->>Core: Execute export operation
    Core->>DB: Fetch configuration metadata
    DB-->>Core: Version metadata
    Core->>DB: Fetch encrypted file content
    DB-->>Core: Encrypted bytes
    
    Core->>Crypto: Decrypt file content
    Crypto-->>Core: Decrypted content
    Core->>FS: Write to export path
    FS-->>Core: File written successfully
    
    Core->>Audit: Log export operation
    Audit->>DB: Store audit record
    Core-->>IPC: Export successful
    
    IPC-->>API: Success response
    API->>Store: Update export status
    Store->>UI: Show success notification
    UI-->>U: "Golden image exported successfully"
```

## 2. Import Configuration Workflow

This workflow shows how new configuration files are imported and versioned in the system.

```mermaid
sequenceDiagram
    participant U as User
    participant UI as Import Wizard
    participant Store as Asset Store
    participant API as Asset Service
    participant IPC as Tauri IPC
    participant Core as Asset Core
    participant DB as Database
    participant Crypto as Encryption
    participant Audit as Audit Log

    U->>UI: Select "Import Configuration"
    UI->>U: Show file picker dialog
    U->>UI: Select configuration file
    UI->>Store: Validate file selection
    Store->>API: importConfiguration(assetName, filePath, notes)
    
    API->>IPC: invoke('import_configuration')
    IPC->>Core: Process import request
    
    Core->>Core: Read file from disk
    Core->>Core: Calculate SHA-256 hash
    Core->>Core: Validate file format
    
    Core->>DB: Check if asset exists
    alt Asset doesn't exist
        Core->>DB: Create new asset
        DB-->>Core: Asset created
    end
    
    Core->>Crypto: Encrypt file content
    Crypto-->>Core: Encrypted bytes
    
    Core->>DB: Store configuration version
    DB-->>Core: Version saved with ID
    
    Core->>Audit: Log import operation
    Audit->>DB: Store audit record
    
    Core-->>IPC: Import successful
    IPC-->>API: Return asset and version info
    API->>Store: Update asset list
    Store->>UI: Show success message
    UI-->>U: "Configuration imported successfully"
```

## 3. Firmware Upload and Analysis Workflow (v0.3.0)

This workflow demonstrates the Epic 3 firmware management capability.

```mermaid
sequenceDiagram
    participant U as User
    participant UI as Firmware UI
    participant Store as Firmware Store
    participant API as Firmware Service
    participant IPC as Tauri IPC
    participant Core as Firmware Core
    participant Analyzer as Firmware Analyzer
    participant Queue as Analysis Queue
    participant DB as Database
    participant FileStore as File Storage
    participant Audit as Audit Log

    U->>UI: Upload firmware file
    UI->>Store: Set upload progress
    Store->>API: uploadFirmware(filePath, metadata)
    
    API->>IPC: invoke('upload_firmware')
    IPC->>Core: Process firmware upload
    
    Core->>Core: Validate file exists
    Core->>Core: Calculate file hash
    Core->>FileStore: Copy to secure storage
    FileStore->>FileStore: Encrypt file
    FileStore-->>Core: Secure path returned
    
    Core->>DB: Create firmware record
    DB-->>Core: Firmware ID assigned
    
    Core->>Queue: Queue for analysis
    Queue->>Analyzer: Start background analysis
    
    Core->>Audit: Log upload operation
    Core-->>IPC: Upload successful
    IPC-->>API: Return firmware info
    API->>Store: Update firmware list
    Store->>UI: Show upload success
    
    par Background Analysis
        Analyzer->>FileStore: Read encrypted firmware
        FileStore->>Analyzer: Decrypted content
        Analyzer->>Analyzer: Run binwalk analysis
        Analyzer->>DB: Store analysis results
        Analyzer->>Store: Notify analysis complete
        Store->>UI: Update analysis status
    end
```

## 4. Branch Creation and Version Management Workflow

This workflow shows how engineers create branches for isolated development.

```mermaid
sequenceDiagram
    participant U as User
    participant UI as Branch UI
    participant Store as Branch Store
    participant API as Branch Service
    participant IPC as Tauri IPC
    participant Core as Branch Core
    participant DB as Database
    participant Audit as Audit Log

    U->>UI: Click "Create Branch"
    UI->>U: Show branch creation dialog
    U->>UI: Enter branch name and description
    UI->>Store: Validate branch input
    Store->>API: createBranch(name, assetId, baseVersionId)
    
    API->>IPC: invoke('create_branch')
    IPC->>Core: Process branch creation
    
    Core->>DB: Check branch name uniqueness
    Core->>DB: Validate base version exists
    Core->>DB: Create branch record
    DB-->>Core: Branch created with ID
    
    Core->>Audit: Log branch creation
    Core-->>IPC: Branch creation successful
    IPC-->>API: Return branch info
    API->>Store: Update branch list
    Store->>UI: Show success message
    UI-->>U: "Branch created successfully"
```

## 5. Status Promotion Workflow

This workflow illustrates how configurations are promoted through status levels to Golden.

```mermaid
sequenceDiagram
    participant U as User
    participant UI as Status Dialog
    participant Store as Asset Store
    participant API as Asset Service
    participant IPC as Tauri IPC
    participant Auth as Auth Module
    participant Core as Status Core
    participant DB as Database
    participant Audit as Audit Log

    U->>UI: Click "Promote to Golden"
    UI->>U: Show promotion wizard
    U->>UI: Confirm promotion with reason
    UI->>Store: Validate user permissions
    Store->>API: promoteToGolden(versionId, reason)
    
    API->>IPC: invoke('promote_to_golden')
    IPC->>Auth: Check administrator role
    Auth-->>IPC: User authorized
    
    IPC->>Core: Process promotion
    Core->>DB: Check current status is Approved
    Core->>DB: Find existing Golden version
    
    alt Existing Golden found
        Core->>DB: Update old Golden to Archived
        Core->>Audit: Log status change
    end
    
    Core->>DB: Update version to Golden status
    Core->>DB: Record status change
    Core->>Audit: Log promotion
    
    Core-->>IPC: Promotion successful
    IPC-->>API: Success response
    API->>Store: Refresh asset data
    Store->>UI: Update status display
    UI-->>U: "Configuration promoted to Golden"
```

## 6. Complete Recovery Export Workflow (v0.3.0)

This Epic 3 workflow shows how firmware and configuration are exported together.

```mermaid
sequenceDiagram
    participant U as User
    participant UI as Recovery UI
    participant Store as Recovery Store
    participant API as Recovery Service
    participant IPC as Tauri IPC
    participant Core as Recovery Core
    participant DB as Database
    participant FileStore as File Storage
    participant Crypto as Encryption
    participant FS as File System

    U->>UI: Click "Export Recovery Package"
    UI->>U: Show export dialog
    U->>UI: Select export location
    UI->>Store: Set export progress
    Store->>API: exportCompleteRecovery(versionId, path)
    
    API->>IPC: invoke('export_complete_recovery')
    IPC->>Core: Process recovery export
    
    Core->>DB: Fetch configuration version
    Core->>DB: Fetch linked firmware
    Core->>FS: Create export directory
    
    par Export Configuration
        Core->>DB: Get encrypted config content
        Core->>Crypto: Decrypt configuration
        Core->>FS: Write config file
    and Export Firmware
        Core->>FileStore: Read encrypted firmware
        Core->>Crypto: Decrypt firmware
        Core->>FS: Write firmware files
    end
    
    Core->>Core: Generate manifest with checksums
    Core->>FS: Write manifest.json
    
    Core-->>IPC: Export complete
    IPC-->>API: Return export details
    API->>Store: Update export status
    Store->>UI: Show success message
    UI-->>U: "Recovery package exported"
```

## Workflow Security Considerations

All workflows implement these security measures:

1. **Authentication**: Every workflow validates session tokens
2. **Authorization**: Role-based permissions checked before operations
3. **Audit Trail**: All operations logged with user context
4. **Encryption**: File content encrypted at rest and in transit
5. **Validation**: Input validation at multiple layers
6. **Rate Limiting**: Prevents abuse of system resources

## Error Handling Patterns

Each workflow implements consistent error handling:

- **Validation Errors**: Caught early and returned to user
- **Permission Errors**: Clear messaging about insufficient access
- **System Errors**: Graceful degradation with retry options
- **Rollback**: Atomic operations that can be safely reverted

These workflows form the backbone of FerroCodex operations, ensuring reliable and secure management of critical industrial configuration and firmware assets.