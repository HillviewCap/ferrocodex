# Technical Assumptions

## Repository Structure: Monorepo

The project will be managed within a single monorepo.

- **Rationale:** This approach will simplify dependency management and code sharing between the core desktop application and future components, such as the DMZ proxy or an update server.
    

## Service Architecture: Modular Monolith with Serverless Sync

The desktop application itself will be a modular monolith. The optional, intermittent sync functionality will be supported by a serverless backend (e.g., using AWS Lambda or similar).

- **Rationale:** The desktop app is naturally a self-contained unit (a monolith), but enforcing modularity internally will make it easier to maintain and extend. A serverless backend for the sync feature is highly cost-effective and scalable, perfectly suited for infrequent connections.
    

## Testing Requirements: Unit + Integration

The MVP will require comprehensive unit tests for individual components and integration tests to ensure these components work together correctly.

- **Rationale:** This two-layered approach is critical for a high-trust product. It ensures that both the smallest pieces of logic and their interactions are reliable and function as expected.
    

## Additional Technical Assumptions and Requests

The application will be built using the Tauri framework. The core backend logic ("Engine") will be written in Rust for maximum security and performance. The user interface ("Dashboard") will be built with React and TypeScript to ensure a polished and modern user experience. An embedded, encrypted local database (e.g., SQLite via a Rust crate) will be used for storage.