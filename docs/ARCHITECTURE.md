# FerroCodex Fullstack Architecture Document

### 1. Introduction

This document outlines the complete fullstack architecture for FerroCodex (Secure OT Configuration Management Platform), including the backend systems, frontend implementation, and their integration. The v0.3.0 release introduces the Integrated Firmware Management feature set, evolving the platform into a comprehensive asset recovery solution. It serves as the single source of truth for AI-driven development, ensuring consistency across the entire technology stack.

#### Starter Template or Existing Project

The project is built using the Tauri framework, which integrates a Rust-based backend with a web-based frontend. This choice was finalized in the PRD and serves as our foundational "starter," providing a secure, performant, and cross-platform application shell from the outset.

#### Change Log

|Date|Version|Description|Author|
|---|---|---|---|
|2025-07-22|0.3.0|Architectural updates for Firmware Management.|Winston (Architect)|
|2025-07-18|1.0|Initial draft based on PRD and UI/UX Spec.|Winston (Architect)|

---

### 2. High Level Architecture

#### Technical Summary

The system is a cross-platform desktop application built using the Tauri framework, which features a Rust backend for maximum security and performance, and a React frontend for a polished user interface. It operates primarily as a modular monolith in an offline-first model, storing all data in a local, encrypted SQLite database.

The primary architectural evolution in v0.3.0 is the adoption of a hybrid storage model. All structured metadata will continue to be stored in the local, encrypted SQLite database, while large binary files (firmware) will be stored as individual encrypted files on the native file system. This ensures the application remains highly performant and scalable. The Rust core will also integrate the binwalk library for automated firmware analysis.

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
    end

    subgraph AWS Cloud (Optional Sync)
        F[API Gateway] --> H[AWS Lambda];
        H --> I[Amazon S3];
    end

    B -- User-Initiated Sync --> F;
```

#### Architectural Patterns

- **Hybrid Storage Model:** Using a transactional SQL database for structured metadata and the native file system for storing large, unstructured binary files.

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
|**Frontend Testing**|Vitest|`~1.6.0`|Unit & Integration testing for UI|Modern, fast, and Jest-compatible test runner that integrates seamlessly with Vite.|
|**Backend Testing**|Rust Test Suite|`(built-in)`|Unit & Integration testing for core|Rust's powerful, built-in testing capabilities are sufficient and idiomatic.|
|**IaC Tool**|AWS CDK|`~2.144.0`|Infrastructure as Code for AWS|Define cloud infrastructure programmatically in TypeScript for reliability and repeatability.|
|**CI / CD**|GitHub Actions|`N/A`|Automated builds, tests, releases|Ubiquitous, powerful, and well-integrated with source control.|
|**Monitoring**|AWS CloudWatch|`N/A`|Monitor serverless sync functions|Native AWS solution for logging and monitoring the optional backend.|
|**Logging (Rust)**|`tracing` crate|`~0.1.40`|Structured application logging|A modern and powerful logging framework for Rust applications.|

---

### 4. Data Models

_(This section contains the detailed definitions for the User, Asset, Branch, ConfigurationVersion, and the new FirmwareVersion models, including their purposes, attributes, relationships, and TypeScript interfaces.)_

---

### 5. API Specification

_(This section contains the definitions for the Local API via Tauri IPC, including all core commands and events, and the OpenAPI 3.0 specification for the optional Cloud Sync REST API. New commands for firmware management will be added.)_

---

### 6. Components

#### Existing Components

_(This section details the logical components of the application: UI (React), IPC Handler (Rust), Core Logic (Rust), Database Module (Rust), and Security Module (Rust), complete with an interaction diagram.)_

#### New Component: Firmware Analyzer (Rust)

- **Responsibility:** To analyze firmware binaries using the binwalk crate to extract metadata.
- **Dependencies:** The binwalk Rust crate.

#### Core Logic (Rust) - Updated Dependencies

- **Dependencies:** Database Module, Security Module, Firmware Analyzer.

---

### 7. Core Workflows

_(This section contains the sequence diagram illustrating the "Restore Golden Image" workflow, showing how all the internal components interact to complete the task.)_

---

### 8. Database Schema

#### v0.3.0 Schema Updates

This schema adds the firmware_versions table and updates the configuration_versions table to support the hybrid storage model.

```sql
-- Existing tables remain unchanged
CREATE TABLE users (
    -- ... existing schema ...
);

CREATE TABLE assets (
    -- ... existing schema ...
);

CREATE TABLE branches (
    -- ... existing schema ...
);

CREATE TABLE configuration_versions (
    -- ... existing schema with addition below ...
);

-- New table for firmware management
CREATE TABLE firmware_versions (
    id TEXT PRIMARY KEY NOT NULL,
    asset_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    vendor TEXT,
    model TEXT,
    version TEXT NOT NULL,
    notes TEXT,
    status TEXT NOT NULL CHECK(status IN ('Draft', 'Golden', 'Archived')),
    file_path TEXT NOT NULL, -- Path to the encrypted file on the file system
    file_hash TEXT NOT NULL, -- SHA-256 hash of the encrypted file
    created_at TEXT NOT NULL,
    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
    FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE RESTRICT
);

-- Add column to link configurations to firmware
ALTER TABLE configuration_versions
ADD COLUMN firmware_version_id TEXT
REFERENCES firmware_versions(id) ON DELETE SET NULL;
```

_(The complete SQL DDL `CREATE TABLE` statements for all tables including indexes and constraints.)_

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

_(This section outlines security requirements for the frontend and backend, including a strict CSP, input validation, and secure authentication. It also defines performance optimization strategies like list virtualization and non-blocking backend operations.)_

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

This concludes the FerroCodex Fullstack Architecture Document v0.3.0. All required artifacts—the Project Brief, PRD, UI/UX Specification, and this Architecture Document—are now complete and updated to include the Integrated Firmware Management features.

The project is fully specified and ready to transition from planning to the development phase. The next step is to move to an IDE environment where a **Scrum Master** will begin creating user stories for the **Developer** to implement, starting with Epic 1.
