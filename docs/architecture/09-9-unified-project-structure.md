# Unified Project Structure

This document provides the complete folder structure of the FerroCodex monorepo, showing the organization of the Tauri application, Rust backend, React frontend, and supporting infrastructure.

## Root Directory Structure

```
ferrocodex/
├── .github/
│   ├── workflows/
│   │   ├── build.yml                    # CI/CD pipeline for testing
│   │   └── release.yml                  # Automated release builds
│   └── ISSUE_TEMPLATE.md
├── .bmad-core/                          # BMad agent configuration
│   ├── core-config.yaml                # Core BMad settings
│   ├── tasks/                          # Reusable tasks
│   ├── templates/                      # Document templates
│   └── checklists/                     # Quality checklists
├── apps/
│   └── desktop/                        # Main Tauri application
│       ├── src/                        # React frontend source
│       ├── src-tauri/                  # Rust backend source
│       ├── dist/                       # Built frontend assets
│       ├── node_modules/               # Node.js dependencies
│       ├── package.json                # Frontend dependencies & scripts
│       ├── package-lock.json           # Dependency lock file
│       ├── tsconfig.json               # TypeScript configuration
│       ├── vite.config.ts              # Vite build configuration
│       └── index.html                  # Main HTML entry point
├── docs/                               # Documentation
│   ├── PRD.md                         # Product Requirements Document
│   ├── ARCHITECTURE.md                 # Master architecture document
│   ├── architecture/                   # Sharded architecture docs
│   ├── prd/                           # Sharded PRD documents
│   ├── stories/                       # User stories
│   └── archive/                       # Historical documents
├── node_modules/                       # Root-level Node.js dependencies
├── package.json                       # Monorepo configuration
├── package-lock.json                  # Root dependency lock file
├── turbo.json                         # Turborepo configuration
├── tsconfig.json                      # Root TypeScript configuration
├── ferrocodex.code-workspace          # VS Code workspace settings
├── CLAUDE.md                          # Claude AI instructions
├── README.md                          # Project overview
└── LICENSE                            # AGPL-3.0 license
```

## Frontend Structure (React/TypeScript)

```
apps/desktop/src/
├── components/                         # React components
│   ├── AdminSetup.tsx                 # Initial admin account setup
│   ├── Dashboard.tsx                  # Main dashboard view
│   ├── LoginScreen.tsx                # User authentication
│   ├── UserManagement.tsx             # Admin user management
│   ├── AssetManagement.tsx            # Asset listing and management
│   ├── ConfigurationHistoryView.tsx   # Version history display
│   ├── BranchManagement.tsx           # Branch creation and management
│   ├── ImportConfigurationModal.tsx   # Configuration import wizard
│   ├── ExportConfirmationModal.tsx    # Export confirmation dialog
│   ├── PromoteToGoldenWizard.tsx      # Golden promotion workflow
│   ├── StatusHistoryModal.tsx         # Status change history
│   ├── CreateBranchModal.tsx          # Branch creation dialog
│   ├── ChangeStatusModal.tsx          # Status change interface
│   ├── RestoreConfirmationModal.tsx   # Restore confirmation
│   ├── ProtectedRoute.tsx             # Route authentication wrapper
│   ├── RoleGuard.tsx                  # Role-based access control
│   ├── LoadingScreen.tsx              # Loading state component
│   ├── EulaModal.tsx                  # End-user license agreement
│   ├── firmware/                      # Epic 3: Firmware components
│   │   ├── FirmwareAnalysis.tsx       # Analysis results display
│   │   ├── FirmwareHistoryTimeline.tsx # Firmware version timeline
│   │   └── FirmwareStatusDialog.tsx   # Firmware status management
│   ├── recovery/                      # Epic 3: Recovery components
│   │   ├── CompleteRecoveryModal.tsx  # Complete recovery export
│   │   └── RecoveryProgress.tsx       # Export progress indicator
│   └── __tests__/                     # Component tests
│       ├── Dashboard.test.tsx
│       ├── LoginScreen.test.tsx
│       ├── UserManagement.test.tsx
│       ├── AssetManagement.test.tsx
│       └── [component].test.tsx
├── store/                             # Zustand state management
│   ├── index.ts                       # Store configuration
│   ├── auth.ts                        # Authentication state
│   ├── assets.ts                      # Asset and configuration state
│   ├── branches.ts                    # Branch management state
│   ├── firmware.ts                    # Epic 3: Firmware state
│   ├── userManagement.ts              # User management state
│   └── app.ts                         # Global application state
├── types/                             # TypeScript type definitions
│   ├── tauri.d.ts                     # Tauri IPC type definitions
│   ├── assets.ts                      # Asset-related types
│   ├── branches.ts                    # Branch-related types
│   ├── firmware.ts                    # Epic 3: Firmware types
│   ├── recovery.ts                    # Epic 3: Recovery types
│   └── dashboard.ts                   # Dashboard types
├── utils/                             # Utility functions
│   ├── roleUtils.ts                   # Role-based access utilities
│   └── roleUtils.test.ts              # Utility tests
├── assets/                            # Static assets
│   ├── ferrocodex-logo.png           # Application logo
│   ├── ferrocodex-logo1.png          # Logo variant 1
│   └── ferrocodex-logo2.png          # Logo variant 2
├── App.tsx                            # Main application component
├── App.test.tsx                       # Application tests
├── main.tsx                           # React application entry point
├── main.ts                            # Tauri frontend entry point
├── styles.css                         # Global styles
├── test-setup.ts                      # Test environment setup
└── vite-env.d.ts                      # Vite environment types
```

## Backend Structure (Rust)

```
apps/desktop/src-tauri/
├── src/
│   ├── main.rs                        # Tauri application entry point
│   ├── lib.rs                         # Library exports and modules
│   ├── auth/                          # Authentication module
│   │   ├── mod.rs                     # Auth module exports
│   │   ├── session.rs                 # Session management
│   │   ├── rate_limiter.rs            # Brute force protection
│   │   └── password.rs                # Password hashing utilities
│   ├── users/                         # User management module
│   │   ├── mod.rs                     # User module exports
│   │   ├── repository.rs              # User database operations
│   │   ├── models.rs                  # User data structures
│   │   └── validation.rs              # User input validation
│   ├── assets/                        # Asset management module
│   │   ├── mod.rs                     # Asset module exports
│   │   ├── repository.rs              # Asset database operations
│   │   ├── models.rs                  # Asset data structures
│   │   └── service.rs                 # Asset business logic
│   ├── configurations/                # Configuration management
│   │   ├── mod.rs                     # Configuration exports
│   │   ├── repository.rs              # Configuration database ops
│   │   ├── models.rs                  # Configuration structures
│   │   ├── service.rs                 # Configuration business logic
│   │   ├── status.rs                  # Status workflow management
│   │   └── status_tests.rs            # Status workflow tests
│   ├── branches/                      # Branch management module
│   │   ├── mod.rs                     # Branch module exports
│   │   ├── repository.rs              # Branch database operations
│   │   ├── models.rs                  # Branch data structures
│   │   └── service.rs                 # Branch business logic
│   ├── firmware/                      # Epic 3: Firmware module
│   │   ├── mod.rs                     # Firmware module exports
│   │   ├── repository.rs              # Firmware database ops
│   │   ├── models.rs                  # Firmware structures
│   │   ├── service.rs                 # Firmware business logic
│   │   └── file_storage.rs            # File system operations
│   ├── firmware_analysis/             # Epic 3: Analysis module
│   │   ├── mod.rs                     # Analysis module exports
│   │   ├── analyzer.rs                # Binwalk integration
│   │   ├── queue.rs                   # Background job queue
│   │   ├── models.rs                  # Analysis data structures
│   │   └── repository.rs              # Analysis result storage
│   ├── recovery/                      # Epic 3: Recovery module
│   │   ├── mod.rs                     # Recovery module exports
│   │   ├── service.rs                 # Recovery business logic
│   │   ├── manifest.rs                # Export manifest generation
│   │   ├── tests.rs                   # Recovery tests
│   │   └── mod_old.rs                 # Legacy recovery code
│   ├── database/                      # Database layer
│   │   ├── mod.rs                     # Database module exports
│   │   ├── connection.rs              # Connection management
│   │   ├── migrations.rs              # Schema migrations
│   │   └── health.rs                  # Database health checks
│   ├── encryption/                    # Encryption module
│   │   ├── mod.rs                     # Encryption exports
│   │   ├── aes.rs                     # AES-256 implementation
│   │   ├── key_derivation.rs          # Key management
│   │   └── file_encryption.rs         # File encryption utilities
│   ├── audit/                         # Audit logging module
│   │   ├── mod.rs                     # Audit module exports
│   │   ├── logger.rs                  # Audit log implementation
│   │   ├── models.rs                  # Audit data structures
│   │   └── repository.rs              # Audit storage operations
│   └── validation/                    # Input validation module
│       ├── mod.rs                     # Validation exports
│       ├── rules.rs                   # Validation rules
│       └── sanitization.rs            # Input sanitization
├── migrations/                        # Database schema migrations
│   ├── 20250101_initial_schema.sql    # Initial database schema
│   ├── 20250115_add_branches.sql      # Branch support migration
│   └── 20250123_add_firmware_status_workflow.sql # Epic 3 migration
├── capabilities/                      # Tauri security capabilities
│   └── default.json                   # Default capability set
├── icons/                            # Application icons
│   ├── 32x32.png                     # Small icon
│   ├── 128x128.png                   # Medium icon
│   ├── 128x128@2x.png                # High-DPI medium icon
│   ├── icon.icns                     # macOS icon
│   ├── icon.ico                      # Windows icon
│   └── icon.png                      # Default icon
├── gen/                              # Generated Tauri files
│   └── schemas/                      # Generated schemas
├── target/                           # Rust build artifacts
├── Cargo.toml                        # Rust dependencies & metadata
├── Cargo.lock                        # Dependency lock file
├── tauri.conf.json                   # Tauri configuration
└── build.rs                          # Build script
```

## Documentation Structure

```
docs/
├── PRD.md                            # Master PRD document
├── ARCHITECTURE.md                    # Master architecture document
├── architecture-validation-report.md  # Architecture validation
├── architecture/                     # Sharded architecture sections
│   ├── index.md                      # Architecture navigation
│   ├── 01-1-introduction.md          # Introduction
│   ├── 02-2-high-level-architecture.md # High-level overview
│   ├── 03-3-tech-stack.md            # Technology stack
│   ├── 04-4-data-models.md           # Data models and types
│   ├── 05-5-api-specification.md     # API documentation
│   ├── 06-6-components.md            # Component architecture
│   ├── 07-7-core-workflows.md        # Workflow diagrams
│   ├── 08-8-database-schema.md       # Database schema
│   ├── 09-9-unified-project-structure.md # This document
│   ├── 10-10-development-workflow.md # Development setup
│   ├── 11-11-deployment-architecture.md # Deployment strategy
│   ├── 12-12-security-and-performance.md # Security requirements
│   ├── 13-13-testing-strategy.md     # Testing approach
│   ├── 14-14-coding-standards.md     # Coding conventions
│   ├── 15-15-error-handling-strategy.md # Error handling
│   └── 16-16-monitoring-and-observability.md # Monitoring
├── prd/                              # Sharded PRD sections
│   ├── index.md                      # PRD navigation
│   ├── epic-1-foundation-core-versioning.md
│   ├── epic-2-advanced-configuration-management.md
│   ├── epic-3-integrated-firmware-management-v030.md
│   ├── epic-list.md
│   ├── goals-and-background-context.md
│   ├── requirements.md
│   ├── technical-assumptions.md
│   └── user-interface-design-goals.md
├── stories/                          # User stories by epic
│   ├── 1.1.story.md                  # Story 1.1: Project Initialization
│   ├── 1.2.story.md                  # Story 1.2: Initial User Account
│   ├── 1.3.story.md                  # Story 1.3: User Management
│   ├── 1.4.story.md                  # Story 1.4: Import Configuration
│   ├── 1.5.story.md                  # Story 1.5: View History
│   ├── 2.1.story.md                  # Story 2.1: Create Branch
│   ├── 2.2.story.md                  # Story 2.2: Add Version to Branch
│   ├── 2.3.story.md                  # Story 2.3: Assign Status
│   ├── 2.4.story.md                  # Story 2.4: Promote to Golden
│   ├── 2.5.story.md                  # Story 2.5: Revert Version
│   ├── 3.1.story.md                  # Story 3.1: Import Firmware
│   ├── 3.2.story.md                  # Story 3.2: Firmware Analysis
│   ├── 3.3.story.md                  # Story 3.3: Link Firmware/Config
│   ├── 3.4.story.md                  # Story 3.4: Firmware Versioning
│   ├── 3.5.story.md                  # Story 3.5: Complete Recovery
│   └── story-archive-workflow-enhancement.md
├── archive/                          # Historical documents
│   ├── ProjectBrief.md               # Original project brief
│   ├── UI UX Specification.md        # UI/UX specifications
│   ├── Alpha-Readiness-Checklist.md  # Alpha release checklist
│   ├── v0.1.0-release-notes.md       # Version 0.1.0 notes
│   └── story-draft-checklist.md      # Story quality checklist
├── conf.py                           # Sphinx documentation config
├── index.rst                         # Documentation index
├── getting-started.rst               # Getting started guide
├── user-guide.rst                    # User manual
├── security.rst                      # Security documentation
├── troubleshooting.rst               # Common issues
├── requirements.txt                  # Documentation dependencies
└── image.png                         # Documentation image
```

## Configuration Files

```
ferrocodex/
├── .github/
│   └── workflows/
│       ├── build.yml                 # CI/CD configuration
│       └── release.yml               # Release automation
├── .bmad-core/
│   └── core-config.yaml             # BMad agent configuration
├── package.json                     # Root monorepo configuration
├── turbo.json                       # Turborepo task configuration
├── tsconfig.json                    # Root TypeScript configuration
├── ferrocodex.code-workspace        # VS Code workspace settings
├── apps/desktop/
│   ├── package.json                 # Frontend dependencies
│   ├── tsconfig.json                # Frontend TypeScript config
│   ├── vite.config.ts               # Vite build configuration
│   └── src-tauri/
│       ├── Cargo.toml               # Rust dependencies
│       ├── tauri.conf.json          # Tauri application config
│       └── build.rs                 # Rust build script
└── docs/
    ├── conf.py                      # Sphinx configuration
    └── requirements.txt             # Documentation dependencies
```

## Key Architectural Patterns

### Monorepo Organization
- **Root Level**: Monorepo configuration and shared tooling
- **Apps Directory**: Contains the main desktop application
- **Modular Structure**: Clear separation between frontend and backend
- **Shared Documentation**: Centralized docs with sharded architecture

### Frontend-Backend Separation
- **Frontend**: React/TypeScript in `src/`
- **Backend**: Rust in `src-tauri/`
- **Communication**: Tauri IPC bridge
- **State Management**: Zustand stores mirror backend structure

### Module Organization
- **Domain-Based**: Each business domain has its own module
- **Repository Pattern**: Data access abstracted through repositories
- **Service Layer**: Business logic separated from data access
- **Clean Architecture**: Dependencies flow inward

### Epic 3 Integration
- **Firmware Module**: Complete firmware management capability
- **Analysis Module**: Background firmware analysis
- **Recovery Module**: Unified configuration and firmware export
- **Hybrid Storage**: Database metadata + filesystem binaries

This structure supports the full FerroCodex v0.3.0 feature set while maintaining clear architectural boundaries and enabling efficient development workflows.