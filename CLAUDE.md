# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Ferrocodex is a secure OT (Operational Technology) configuration management platform built as a cross-platform desktop application using Tauri 2.0. It combines a Rust backend with a React/TypeScript frontend to provide offline-first configuration management for industrial equipment.

## Development Commands

### Quick Start

```bash
# Install dependencies (from root)
npm install

# Start development server (recommended)
npm run dev

# Or specifically for Tauri development
cd apps/desktop
npm run tauri:dev
```

### Essential Commands

```bash
# From root directory:
npm run dev          # Start development server with hot reload
npm run build        # Build all packages
npm run test         # Run all tests across workspaces
npm run lint         # Run linting across all workspaces

# From apps/desktop directory:
npm run test:run     # Run frontend tests once
npm run test:coverage # Run tests with coverage report
npm run tauri:build  # Build production desktop app

# Backend tests (from root):
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
```

### Release & Deployment

```bash
# Create a release (triggers automatic binary building):
git tag v1.0.0
git push origin v1.0.0

# GitHub Actions will automatically:
# - Build binaries for Windows, macOS (Intel + ARM), and Linux
# - Run all tests across platforms
# - Create GitHub release with downloadable assets
```

### Running Individual Tests

```bash
# Frontend test for specific file
cd apps/desktop
npm run test -- src/components/Dashboard.test.tsx

# Backend test for specific module
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml -- users::tests
```

## Architecture Overview

### Technology Stack

- **Frontend**: React 18 + TypeScript + Vite + Ant Design + Zustand
- **Backend**: Rust + Tauri 2.0 + SQLite (encrypted) + bcrypt
- **Testing**: Vitest (frontend) + Rust built-in tests (backend)
- **Monorepo**: Turborepo for task orchestration

### Key Architectural Patterns

1. **Tauri IPC Communication**: Frontend invokes Rust commands via `invoke` API
   - All commands defined in `src-tauri/src/main.rs`
   - TypeScript types in `src/types/` mirror Rust structures
   - Example: `await invoke('create_user', { userData })`

2. **State Management**:
   - Frontend: Zustand store in `src/store/`
   - Backend: Thread-safe Mutex-wrapped state
   - Session management with token validation

3. **Database Layer**:
   - Repository pattern: Each entity has its own repository (`src-tauri/src/*/repository.rs`)
   - All data encrypted at rest (AES-256)
   - Audit trail for all operations

4. **Security**:
   - Role-based access control (Administrator, Engineer)
   - Rate limiting on sensitive operations
   - Input validation on both frontend and backend
   - Session tokens with automatic validation

### Project Structure

```
apps/desktop/
├── src/                    # React frontend
│   ├── components/         # UI components organized by feature
│   ├── store/             # Zustand state management
│   ├── types/             # TypeScript type definitions
│   └── utils/             # Utility functions
└── src-tauri/             # Rust backend
    └── src/
        ├── assets/        # Asset (equipment) management
        ├── audit/         # Audit logging
        ├── auth/          # Authentication & sessions
        ├── branches/      # Configuration branching
        ├── configurations/ # Config file management
        ├── database/      # SQLite database layer
        ├── encryption/    # AES-256 encryption
        ├── users/         # User management
        └── validation/    # Input validation
```

### Important Conventions

1. **Error Handling**:
   - Backend returns `Result<T, String>` for all operations
   - Frontend displays user-friendly error messages via Ant Design notifications
   - All errors logged to audit trail

2. **Testing**:
   - Frontend components use React Testing Library
   - Mock Tauri API calls in tests with `vi.mock('@tauri-apps/api/core')`
   - Backend uses `#[cfg(test)]` modules with test fixtures

3. **Database Operations**:
   - Always use prepared statements to prevent SQL injection
   - Transaction support for multi-step operations
   - Automatic database initialization on first run

4. **Frontend Routing**:
   - Routes defined in `src/App.tsx`
   - Protected routes check authentication status
   - Role-based component rendering

### Development Notes

- **Port 1420** must be available for development server
- **Node.js 18+** and **Rust 1.78.0+** required
- Database automatically created in app data directory
- Hot reload works for both frontend and Rust backend
- Tauri commands are async - always use await or proper promise handling
