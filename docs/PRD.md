# FerroCodex Product Requirements Document (PRD)

### 1. Goals and Background Context

#### Goals

- Reduce the average time for an engineer to recover a device using a "golden" configuration to under 5 minutes.

- Achieve high user adoption by creating a highly polished, intuitive platform that engineers prefer over insecure, ad-hoc methods.

- Establish a market foothold with a successful MVP that can be expanded into a future enterprise-tier product.

- Increase engineer confidence in the integrity of their configurations and change management processes.

- **[v0.3.0]** Enable comprehensive asset recovery by managing both configurations and firmware in a unified platform.

#### Background Context

The platform addresses a critical gap in the Operational Technology (OT) and Industrial Control Systems (ICS) space. Engineers currently rely on risky, inefficient methods like USB drives and network shares for managing vital equipment configurations. This leads to extended downtime during failures and lacks the security and auditability required in modern industrial environments. 

**[v0.3.0 Update]** The platform evolves beyond configuration management to become a comprehensive asset recovery solution by integrating firmware management capabilities. This addresses the complete recovery scenario where both firmware and configuration may need to be restored.

This PRD outlines the requirements for an offline-first, secure, and user-friendly platform to solve this problem.

#### Change Log

| Date       | Version | Description                                      | Author    |
| ---------- | ------- | ------------------------------------------------ | --------- |
| 2025-07-22 | 0.3.0   | Added Integrated Firmware Management requirements | John (PM) |
| 2025-07-17 | 1.0     | Initial draft based on Project Brief.            | John (PM) |

---

### 2. Requirements

#### Functional

1. **FR1:** The system MUST provide a secure, local repository to import, store, and manage equipment configuration files.

2. **FR2:** The system MUST support both text-based and proprietary binary file formats.

3. **FR3:** The system MUST automatically create a new, versioned entry for a configuration file each time it is updated.

4. **FR4:** The system MUST maintain a complete, auditable history for each configuration, tracking the author, timestamp, and user-provided notes for every version.

5. **FR5:** Users MUST be able to create a "branch" from any configuration version to work on changes in isolation.

6. **FR6:** Users MUST be able to assign a status to any configuration version (e.g., `Draft`, `Approved`, `Golden`).

7. **FR7:** The system MUST provide a guided, wizard-like interface for promoting a configuration version to the official "Golden" status.

8. **FR8:** Users MUST be able to select any previous version from a configuration's history and restore/export it with a single action.

9. **FR9:** The system MUST require users to log in with a unique username and password to access the platform.

10. **FR10:** The system MUST support at least two user roles: `Administrator` and `Engineer`.

11. **FR11:** An `Administrator` MUST be able to create, manage, and deactivate `Engineer` user accounts locally within the application.

12. **[v0.3.0] FR12:** The system MUST provide secure storage and management for firmware files associated with assets.

13. **[v0.3.0] FR13:** The system MUST automatically analyze firmware files upon upload to extract metadata using integrated analysis tools.

14. **[v0.3.0] FR14:** Users MUST be able to link firmware versions to configuration versions for complete asset recovery scenarios.

15. **[v0.3.0] FR15:** The system MUST support the same versioning and status workflow for firmware as for configurations.

16. **[v0.5.0] FR16:** The system MUST support hierarchical asset organization with unlimited nesting depth through folder structures.

17. **[v0.5.0] FR17:** Users MUST be able to define custom metadata fields for assets with validation rules and data type enforcement.

18. **[v0.5.0] FR18:** The system MUST enforce cybersecurity-compliant naming conventions for all assets, folders, and uploaded files.

19. **[v0.5.0] FR19:** Asset creation MUST precede configuration and firmware imports, establishing clear parent-child relationships.

20. **[v0.5.0] FR20:** The system MUST provide pre-built metadata templates for common industrial asset properties (IP address, location, install date, facility, notes).

#### Non-Functional

1. **NFR1:** The system MUST be fully functional in a completely disconnected, offline environment.

2. **NFR2:** All configuration files stored at rest within the local repository MUST be encrypted using AES-256 or a superior, industry-standard algorithm.

3. **NFR3:** Critical user operations (e.g., reverting to a golden image, viewing history) MUST complete in under 2 seconds.

4. **NFR4:** The user interface MUST be designed for simplicity and intuitive operation by non-developer technical personnel.

5. **NFR5:** The system MUST support an optional, user-initiated, end-to-end encrypted channel for software updates and anonymous telemetry sync.

6. **NFR6:** The application MUST be able to run on current versions of Windows, macOS, and Linux.

7. **NFR7:** All user passwords MUST be securely stored using a modern, salt-based hashing algorithm (e.g., Argon2 or bcrypt).

8. **[v0.3.0] NFR8:** Large firmware files MUST be stored efficiently on the native file system while maintaining encryption and performance standards.

9. **[v0.3.0] NFR9:** Firmware analysis operations MUST not block the UI and should complete within reasonable time based on file size.

---

### 3. User Interface Design Goals

#### Overall UX Vision

The UX vision is to create a calm, clear, and trustworthy tool that empowers OT engineers, especially when they are under pressure. The interface must prioritize safety and simplicity over feature density, guiding users through complex version control concepts with an intuitive, step-by-step approach. The user should always feel confident and in control.

**[v0.3.0 Addition]** The integrated firmware management features must maintain the same level of simplicity and clarity, presenting firmware and configuration as complementary aspects of asset management rather than separate systems.

#### Key Interaction Paradigms

- **Wizard-Driven Workflows:** Key processes like "Promote to Golden" or creating a new branch will be guided by simple, multi-step wizards.

- **Visual History:** Configuration history will be presented as a clear, visual timeline, not just a text-based log.

- **"Single Pane of Glass" Dashboard:** A main dashboard will provide an at-a-glance overview of all managed devices and the status of their configurations.

- **Strong Status Cues:** The status of any configuration (`Golden`, `Approved`, `Draft`, etc.) will be immediately obvious through prominent labels and color-coding.

- **[v0.3.0] Unified Asset View:** Firmware and configuration information will be presented in a unified interface that shows the complete recovery state of an asset.

#### Core Screens and Views

- **Login Screen:** A secure screen for user authentication.

- **Main Dashboard:** Hierarchical tree view of all managed assets organized in folders (e.g., "Production Line 1" → devices).

- **Asset Detail View:** Shows complete asset information including custom metadata, associated configurations, firmware, and credentials.

- **Asset Creation Wizard:** Guided workflow for creating assets with metadata field selection and validation.

- **[v0.5.0] Hierarchical Asset Browser:** Tree-based navigation with search, filtering, and bulk operations across asset hierarchy.

- **[v0.5.0] Metadata Management:** Interface for defining custom field schemas and validation rules.

- **Device Detail View:** Shows the version history timeline for a single device.

- **Version Details Pane:** Displays the metadata for a specific version (author, notes, test results, etc.).

- **User Management Screen:** A simple interface for administrators to manage user accounts.

- **[v0.3.0] Firmware Management Tab:** Integrated into the Device Detail View to manage firmware versions alongside configurations.

- **[v0.3.0] Firmware Analysis Results:** Display automated analysis results from firmware uploads in a clear, actionable format.

#### Accessibility: WCAG AA

To ensure the product is usable by the widest range of engineers, the UI should meet Web Content Accessibility Guidelines (WCAG) 2.1 Level AA as a minimum standard.

#### Branding

The visual design should be clean, professional, and utilitarian. It should inspire confidence and trust. The color palette should be used meaningfully to convey status and warnings (e.g., green for `Golden`, red for critical alerts).

#### Target Device and Platforms: Cross-Platform

The application is a desktop product that must run natively on Windows, macOS, and Linux, and its UI must be responsive to typical desktop and laptop screen sizes.

---

### 4. Technical Assumptions

#### Repository Structure: Monorepo

The project will be managed within a single monorepo.

- **Rationale:** This approach will simplify dependency management and code sharing between the core desktop application and future components, such as the DMZ proxy or an update server.

#### Service Architecture: Modular Monolith with Serverless Sync

The desktop application itself will be a modular monolith. The optional, intermittent sync functionality will be supported by a serverless backend (e.g., using AWS Lambda or similar).

- **Rationale:** The desktop app is naturally a self-contained unit (a monolith), but enforcing modularity internally will make it easier to maintain and extend. A serverless backend for the sync feature is highly cost-effective and scalable, perfectly suited for infrequent connections.

- **[v0.3.0 Update]:** The modular architecture will be extended to include a firmware analysis module that integrates with the binwalk library for automated firmware inspection.

#### Testing Requirements: Unit + Integration

The MVP will require comprehensive unit tests for individual components and integration tests to ensure these components work together correctly.

- **Rationale:** This two-layered approach is critical for a high-trust product. It ensures that both the smallest pieces of logic and their interactions are reliable and function as expected.

#### Additional Technical Assumptions and Requests

The application will be built using the Tauri framework. The core backend logic ("Engine") will be written in Rust for maximum security and performance. The user interface ("Dashboard") will be built with React and TypeScript to ensure a polished and modern user experience. An embedded, encrypted local database (e.g., SQLite via a Rust crate) will be used for storage.

**[v0.3.0 Addition]** A hybrid storage model will be implemented where metadata remains in SQLite while large firmware files are stored as encrypted files on the native file system. The binwalk library will be integrated for firmware analysis capabilities.

---

### 5. Epic List

#### Epic 1: Foundation & Core Versioning

- **Goal:** Establish the secure, cross-platform application foundation, including user authentication and the core ability to import configurations and track their complete version history.

#### Epic 2: Advanced Configuration Management

- **Goal:** Empower engineers with advanced workflows to safely manage configuration changes using branching, status promotion, and one-click restore capabilities.

#### Epic 3: Integrated Firmware Management [v0.3.0]

- **Goal:** Transform the platform into a comprehensive asset recovery solution by adding secure firmware storage, automated analysis, and integrated management capabilities.

#### Epic 4: Asset Identity Vault

- **Goal:** Create a comprehensive credential and secret management system that serves as the single source of truth for all asset-related authentication information, eliminating the cognitive burden on engineers and enabling secure sharing of access credentials.

#### Epic 5: Asset-Centric Hierarchical Management [v0.5.0]

- **Goal:** Transform the platform into a comprehensive hierarchical asset management system with customizable metadata, cybersecurity-compliant naming, and intuitive folder-based organization that prioritizes simplicity while enabling complex asset relationships.

---

### Epic 1 Foundation & Core Versioning

**Epic Goal:** Establish the secure, cross-platform application foundation, including user authentication and the core ability to import configurations and track their complete version history. This epic delivers the initial core value: a secure, auditable "vault" for vital configurations.

#### Story 1.1: Project Initialization

As a Project Owner, I want the initial application structure set up, so that developers have a clean, consistent foundation to start building on.

- **Acceptance Criteria:**

    1. A new monorepo is created.

    2. A Tauri application is initialized with a Rust backend and a React/TypeScript frontend.

    3. The application compiles and launches to a blank window on Windows, macOS, and Linux.

    4. The local, encrypted database (e.g., SQLite) is included as a dependency.

#### Story 1.2: Initial User Account & Login

As an Administrator, I want to create the first user account and log in, so that the application is secured from unauthorized access.

- **Acceptance Criteria:**

    1. On first launch, the application presents a "Create Admin Account" screen.

    2. A new user account is created and stored in the local database with a securely hashed password.

    3. After account creation, the user is directed to a Login Screen.

    4. The user can successfully log in using the created credentials.

    5. Upon successful login, the user is presented with a main (currently empty) dashboard screen.

#### Story 1.3: User Management

As an Administrator, I want to manage Engineer accounts, so that I can control who has access to the platform.

- **Acceptance Criteria:**

    1. When logged in as an `Administrator`, a "User Management" area is visible.

    2. The Administrator can create new `Engineer` user accounts (username and initial password).

    3. The Administrator can deactivate and reactivate existing `Engineer` accounts.

    4. `Engineer` users do not see the "User Management" area.

#### Story 1.4: Import Configuration and Create First Version

As an Engineer, I want to import a configuration file for a new asset, so that it is securely stored and versioned in the platform.

- **Acceptance Criteria:**

    1. From the main dashboard, a logged-in user can select an "Import Configuration" action.

    2. The user is prompted to select a file from their local system.

    3. The user must provide a name for the asset/device (e.g., "PLC-Line5") and can add initial commit notes.

    4. Upon saving, the file is stored encrypted in the local database.

    5. A new version record (v1) is created, associated with the asset, and includes the author's username, timestamp, and notes.

    6. The newly created asset appears on the main dashboard.

#### Story 1.5: View Configuration History

As an Engineer, I want to view the complete version history of an asset, so that I have a full audit trail of all changes.

- **Acceptance Criteria:**

    1. Clicking on an asset from the dashboard navigates to a "History View."

    2. The History View displays a list of all saved versions in reverse chronological order.

    3. Each entry in the list clearly shows the version number, author, timestamp, and the user's notes for that change.

    4. The view is performant and loads in under 2 seconds.

---

### Epic 2 Advanced Configuration Management

**Epic Goal:** Empower engineers with advanced workflows to safely manage configuration changes using branching, status promotion, and one-click restore capabilities. This epic transforms the platform from a passive "vault" into an active, powerful tool for change management and disaster recovery.

#### Story 2.1: Create a Branch

As an Engineer, I want to create a branch from an existing configuration version, so that I can safely experiment with changes without affecting the main line of development.

- **Acceptance Criteria:**

    1. From the "History View" of an asset, the user can select any version.

    2. An action is available to "Create Branch from this Version."

    3. The user must provide a name for the new branch (e.g., "test-new-parameters").

    4. The new branch appears in the UI, clearly showing its relationship to the parent version it was branched from.

#### Story 2.2: Add New Version to a Branch

As an Engineer, I want to add a new configuration version to the branch I created, so that I can save my work-in-progress.

- **Acceptance Criteria:**

    1. The user can select an active branch.

    2. The user can use an "Update Branch" action to import a new version of the configuration file.

    3. The user must add notes for the new version.

    4. The new version is added to the history of that specific branch only.

    5. The main version history of the asset is not affected.

#### Story 2.3: Assign Configuration Status

As an Engineer, I want to assign a status to a configuration version, so that I and others know its state of readiness.

- **Acceptance Criteria:**

    1. In the history view, each version has a visible status (defaulting to `Draft`).

    2. A user can change the status of a version to `Approved`.

    3. The status is clearly displayed with a distinct color or icon.

    4. Only an `Administrator` can change a status back from `Approved` to `Draft`.

#### Story 2.4: Promote Version to "Golden Image"

As an Engineer, I want to use a guided wizard to promote an approved configuration to the official "Golden Image", so that it is clearly marked as the trusted, master version for disaster recovery.

- **Acceptance Criteria:**

    1. A "Promote to Golden" action is available only on versions with an `Approved` status.

    2. Activating this starts a wizard that explains the action and asks for final confirmation.

    3. Upon confirmation, the version's status is changed to `Golden`.

    4. If another version of the same asset was previously `Golden`, its status is automatically changed to `Archived`.

    5. The `Golden` version is prominently displayed on the asset's main view.

#### Story 2.5: Revert to a Previous Version

As an Engineer, I want to quickly restore any previous configuration version, so that I can rapidly recover from a failed deployment or equipment failure.

- **Acceptance Criteria:**

    1. Every version in the history view has a "Restore" or "Export" button.

    2. Clicking the button prompts the user for confirmation.

    3. Upon confirmation, the system exports the exact file for that selected version to a location the user chooses on their local machine.

    4. The export process is completed in under 2 seconds.

---

### Epic 3 Integrated Firmware Management [v0.3.0]

**Epic Goal:** Transform the platform into a comprehensive asset recovery solution by adding secure firmware storage, automated analysis, and integrated management capabilities. This epic extends the platform's value proposition from configuration-only to complete asset recovery.

#### Story 3.1: Import and Store Firmware

As an Engineer, I want to import firmware files for my assets, so that I have a complete recovery solution including both firmware and configuration.

- **Acceptance Criteria:**

    1. From an asset's detail view, users can access a "Firmware" tab.

    2. Users can upload firmware files of any size (up to system limits).

    3. The system stores firmware files encrypted on the native file system.

    4. Firmware metadata is stored in the database with version tracking.

    5. Upload progress is displayed for large files.

#### Story 3.2: Automated Firmware Analysis

As an Engineer, I want the system to automatically analyze uploaded firmware, so that I can understand its contents and verify integrity.

- **Acceptance Criteria:**

    1. Upon firmware upload, the system automatically initiates analysis.

    2. Analysis runs in the background without blocking the UI.

    3. Results include file type detection, embedded version information, and basic security checks.

    4. Analysis results are displayed in a clear, readable format.

    5. Users can view analysis results for any firmware version.

#### Story 3.3: Link Firmware to Configuration

As an Engineer, I want to associate firmware versions with configuration versions, so that I can manage complete asset recovery packages.

- **Acceptance Criteria:**

    1. When viewing a configuration version, users can link it to a firmware version.

    2. The link is bidirectional and visible from both firmware and configuration views.

    3. The system tracks which firmware/configuration combinations are known to work together.

    4. Linked versions can be exported together as a recovery package.

#### Story 3.4: Firmware Version Management

As an Engineer, I want to manage firmware versions with the same workflow as configurations, so that I have consistent version control across all asset components.

- **Acceptance Criteria:**

    1. Firmware versions support the same status workflow (Draft, Approved, Golden, Archived).

    2. Users can add notes to firmware versions.

    3. Firmware history is displayed in a timeline similar to configurations.

    4. The same role-based permissions apply to firmware management.

#### Story 3.5: Complete Asset Recovery

As an Engineer, I want to export both firmware and configuration for an asset, so that I can perform complete recovery in a single operation.

- **Acceptance Criteria:**

    1. A "Complete Recovery" option is available for assets with both firmware and configuration.

    2. Users can select specific versions of both firmware and configuration to export.

    3. The system exports both files to a user-selected location.

    4. Export includes a manifest file documenting versions and checksums.

    5. The entire export process maintains sub-2-second performance for configurations (firmware may take longer based on size).

---

### Epic 4 Asset Identity Vault

**Epic Goal:** Create a comprehensive credential and secret management system that serves as the single source of truth for all asset-related authentication information, eliminating the cognitive burden on engineers and enabling secure sharing of access credentials. This epic addresses the core user needs of offloading cognitive burden, enabling secure sharing, compensating for device limitations, eliminating password reuse, and simplifying password rotation.

#### Story 4.1: PLC Identity Vault Creation

As an Engineer, I want to create an identity vault for a PLC that groups all its secrets together, so that I have a single, secure location for all authentication information related to that asset.

- **Acceptance Criteria:**

    1. From an asset's detail view, users can create an "Identity Vault" that stores multiple types of secrets for that PLC.

    2. The vault can store login passwords, IP addresses, VPN keys, and license files in a single, organized container.

    3. All vault contents are encrypted using the same AES-256 standard as configuration files.

    4. The vault maintains version history and audit trails for all changes to any secret within it.

    5. Vault creation is integrated with the existing asset recovery export functionality.

#### Story 4.2: Individual Asset Password Management

As an Engineer, I want to store and manage unique passwords for individual assets, so that I can eliminate password reuse and maintain strong, unique credentials for every device.

- **Acceptance Criteria:**

    1. Each asset's identity vault supports multiple credential entries with descriptive labels (e.g., "Admin Login", "Maintenance Account", "Read-Only Access").

    2. The system can generate strong, unique passwords automatically when requested.

    3. Password strength indicators are displayed when creating or updating credentials.

    4. Users can view password history to track when credentials were last changed.

    5. The system warns users if they attempt to reuse passwords across different assets or accounts.

#### Story 4.3: Standalone IT Asset Credential Storage

As an Engineer, I want to store passwords for jump hosts, databases, and other IT assets that aren't tracked as PLCs on the platform, so that I can manage all my industrial environment credentials in one secure location.

- **Acceptance Criteria:**

    1. Users can create standalone credential entries that are not associated with a tracked PLC asset.

    2. Standalone entries support the same credential types as PLC vaults (passwords, IP addresses, VPN keys, license files).

    3. Standalone credentials can be organized into categories (e.g., "Jump Hosts", "Databases", "Network Equipment").

    4. Search and filtering capabilities help users quickly locate specific standalone credentials.

    5. The same encryption, versioning, and audit trail features apply to standalone credentials.

#### Story 4.4: Integrated Recovery Bundle Export

As an Engineer, I want to export a complete recovery bundle that includes configuration, firmware, and identity vault information, so that I have everything needed for complete asset recovery in a single, secure package.

- **Acceptance Criteria:**

    1. The existing recovery export functionality is enhanced to include identity vault contents.

    2. Users can choose to include or exclude identity vault information in recovery bundles based on security policies.

    3. When identity vault information is included, it maintains the same encryption standards during export.

    4. The recovery bundle manifest documents all included components (configuration, firmware, credentials).

    5. Import functionality can restore the complete bundle including identity vault information to a new installation.

#### Story 4.5: Secure Credential Sharing

As an Administrator, I want to control which users can access identity vault information, so that I can enable secure sharing of credentials among authorized team members while maintaining security controls.

- **Acceptance Criteria:**

    1. Identity vault access is controlled by the existing role-based permission system.

    2. Administrators can grant or revoke access to specific identity vaults for individual Engineer users.

    3. All access to identity vault information is logged in the audit trail.

    4. Users can only view credentials for vaults they have been explicitly granted access to.

    5. The system supports time-limited access grants that automatically expire after a specified period.

#### Story 4.6: Password Rotation Workflow

As an Engineer, I want to easily rotate passwords and track the rotation history, so that I can maintain security best practices and respond quickly to security incidents.

- **Acceptance Criteria:**

    1. Each credential entry has a "Rotate Password" action that guides users through the password change process.

    2. The system tracks password rotation dates and can alert users when passwords are approaching recommended rotation intervals.

    3. Previous passwords are securely archived in the version history but marked as "Rotated" to prevent accidental reuse.

    4. Users can add notes during password rotation to document the reason for the change.

    5. Batch rotation workflows allow users to rotate multiple related passwords (e.g., all accounts on a specific PLC) in a coordinated manner.

---

This concludes the FerroCodex Product Requirements Document v0.3.0. The platform now addresses the complete asset recovery scenario, providing engineers with a comprehensive, secure, and user-friendly solution for managing configurations, firmware, and identity credentials in critical OT environments.