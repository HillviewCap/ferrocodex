# High Level Architecture

## Technical Summary

The system is a cross-platform desktop application built using the Tauri framework, which features a Rust backend for maximum security and performance, and a React frontend for a polished user interface. It operates primarily as a modular monolith in an offline-first model, storing all data in a local, encrypted SQLite database.

The primary architectural evolution in v0.3.0 is the adoption of a hybrid storage model. All structured metadata will continue to be stored in the local, encrypted SQLite database, while large binary files (firmware) will be stored as individual encrypted files on the native file system. This ensures the application remains highly performant and scalable. The Rust core will also integrate the binwalk library for automated firmware analysis.

A monorepo structure will manage the codebase. For the optional, intermittent sync feature, the application will communicate with a secure, serverless backend hosted on AWS, ensuring scalability and cost-efficiency. The architecture prioritizes security, data integrity, and a responsive, intuitive experience for OT engineers.

## Platform and Infrastructure Choice

- **Platform:** AWS (Amazon Web Services) will be used for the optional sync and update functionality.
    
- **Key Services:** AWS Lambda (for compute), API Gateway (for the sync endpoint), S3 (for software update storage), and Cognito (for potential future cloud identity services).
    
- **Deployment Host and Regions:** The desktop application is self-hosted by the user. The serverless backend will be deployed to `us-east-1` and `eu-west-1` for redundancy.
    

## Repository Structure

- **Structure:** Monorepo.
    
- **Monorepo Tool:** Turborepo is recommended to manage workspaces and optimize build processes.
    
- **Package Organization:** The monorepo will contain separate packages for the Tauri application (`apps/desktop`) and any future cloud infrastructure or shared libraries (`packages/shared-types`).
    

## High Level Architecture Diagram

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

## Architectural Patterns

- **Hybrid Storage Model:** Using a transactional SQL database for structured metadata and the native file system for storing large, unstructured binary files.
    
- **Firmware Analysis Engine:** The Rust core will integrate the binwalk library to perform automated analysis on uploaded firmware files.
    
- **Modular Monolith (Desktop App):** The core application is a single deployable unit, but its internal code will be structured in a modular way to ensure maintainability and separation of concerns.
    
- **Serverless (Cloud Sync):** The backend for handling software updates and optional telemetry will be built using serverless functions to ensure it is scalable and cost-effective.
    
- **Component-Based UI:** The React frontend will be built as a collection of reusable, stateless, and well-defined components.
    
- **Repository Pattern (Rust Core):** The Rust backend will use the repository pattern to abstract the database logic from the core business logic.