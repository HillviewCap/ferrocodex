# Development Workflow

This document outlines the complete development workflow for FerroCodex, including prerequisites, initial setup, development commands, and environment configuration.

## Prerequisites

### Required Software

| Tool | Version | Purpose | Installation |
|------|---------|---------|--------------|
| **Node.js** | 18.0.0+ | Frontend development & build tools | [nodejs.org](https://nodejs.org/) |
| **npm** | 8.0.0+ | Package management | Included with Node.js |
| **Rust** | 1.78.0+ | Backend development | [rustup.rs](https://rustup.rs/) |
| **Git** | Latest | Version control | [git-scm.com](https://git-scm.com/) |

### Platform-Specific Dependencies

#### Windows
```bash
# Install Visual Studio Build Tools or Visual Studio Community
# Required for Rust native dependencies
winget install Microsoft.VisualStudio.2022.BuildTools
```

#### macOS
```bash
# Install Xcode Command Line Tools
xcode-select --install
```

#### Linux (Ubuntu/Debian)
```bash
# Install build essentials and WebKit development libraries
sudo apt update
sudo apt install build-essential curl wget file libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

### Rust Toolchain Setup

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Add WebAssembly target (for future web builds)
rustup target add wasm32-unknown-unknown

# Install Tauri CLI
cargo install tauri-cli@^2.0.0
```

## Initial Setup

### 1. Clone Repository
```bash
git clone https://github.com/ferrocodex/ferrocodex.git
cd ferrocodex
```

### 2. Install Dependencies
```bash
# Install root dependencies and setup monorepo
npm install

# Navigate to desktop app and install frontend dependencies
cd apps/desktop
npm install

# Return to root for development
cd ../..
```

### 3. Environment Configuration

Create environment files based on the examples:

```bash
# Copy environment template (if exists)
cp apps/desktop/.env.example apps/desktop/.env.local

# Configure database path (optional - defaults to app data directory)
# DATABASE_PATH=./data/ferrocodex.db

# Configure logging level for development
# RUST_LOG=debug
# TAURI_DEV_WATCHER=true
```

### 4. Database Initialization

The database is automatically initialized on first run, but you can manually initialize it:

```bash
# The application will create the database on first launch
# Database file location: 
# - Windows: %APPDATA%/com.ferrocodex.app/ferrocodex.db
# - macOS: ~/Library/Application Support/com.ferrocodex.app/ferrocodex.db  
# - Linux: ~/.local/share/com.ferrocodex.app/ferrocodex.db
```

## Development Commands

### Primary Development Workflow

```bash
# Start development server with hot reload (recommended)
npm run dev

# Alternative: Start development from desktop app directory
cd apps/desktop
npm run tauri:dev
```

This command:
- Starts the Vite development server for the frontend
- Compiles and runs the Rust backend
- Enables hot reload for both frontend and backend changes
- Opens the application window automatically

### Building and Testing

```bash
# Run all tests across the monorepo
npm run test

# Run frontend tests only
cd apps/desktop
npm run test           # Watch mode
npm run test:run       # Single run
npm run test:coverage  # With coverage report

# Run backend tests only
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml

# Build production version
npm run build

# Build desktop app for current platform
cd apps/desktop
npm run tauri:build
```

### Linting and Code Quality

```bash
# Run linting across all workspaces
npm run lint

# Frontend-specific linting
cd apps/desktop
npx eslint src/
npx prettier --check src/

# Backend-specific linting
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml -- -D warnings
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
```

### Database Operations

```bash
# Reset database (deletes all data)
# Stop the application first, then delete the database file
# Database will be recreated on next launch

# Run database migrations manually (if needed)
# Migrations run automatically on application startup
```

## Environment Configuration

### .env.example Template

```bash
# Development Configuration
NODE_ENV=development
VITE_APP_VERSION=0.3.0
VITE_APP_NAME=FerroCodex

# Rust Backend Configuration
RUST_LOG=info
RUST_BACKTRACE=1

# Tauri Development Configuration
TAURI_DEV_WATCHER=true
TAURI_SKIP_DEVSERVER_CHECK=false

# Database Configuration (optional)
# DATABASE_PATH=./data/ferrocodex.db
# DATABASE_ENCRYPTION_KEY=auto-generated

# Security Configuration
# SESSION_TIMEOUT_HOURS=8
# MAX_LOGIN_ATTEMPTS=5
# RATE_LIMIT_WINDOW_MINUTES=15

# Firmware Analysis Configuration
# MAX_FIRMWARE_SIZE_MB=2048
# ANALYSIS_TIMEOUT_MINUTES=30
# BINWALK_PATH=binwalk

# File Storage Configuration
# STORAGE_PATH=./data/storage
# MAX_STORAGE_SIZE_GB=100

# Logging Configuration
# LOG_LEVEL=info
# LOG_FILE=./logs/ferrocodex.log
# MAX_LOG_FILES=10
# MAX_LOG_SIZE_MB=10
```

### Environment Variables Reference

| Variable | Default | Description |
|----------|---------|-------------|
| `NODE_ENV` | `development` | Build environment |
| `RUST_LOG` | `info` | Rust logging level |
| `TAURI_DEV_WATCHER` | `true` | Enable hot reload |
| `DATABASE_PATH` | Auto | Custom database location |
| `SESSION_TIMEOUT_HOURS` | `8` | User session timeout |
| `MAX_LOGIN_ATTEMPTS` | `5` | Login attempt limit |
| `MAX_FIRMWARE_SIZE_MB` | `2048` | Firmware upload limit |
| `ANALYSIS_TIMEOUT_MINUTES` | `30` | Analysis timeout |

## Development Workflow Patterns

### 1. Feature Development Workflow

```bash
# 1. Create feature branch
git checkout -b feature/epic-3-firmware-analysis

# 2. Start development server
npm run dev

# 3. Make changes to frontend and/or backend
# Frontend: apps/desktop/src/
# Backend: apps/desktop/src-tauri/src/

# 4. Run tests
npm run test

# 5. Commit changes
git add .
git commit -m "feat: Add firmware analysis component"

# 6. Push and create PR
git push origin feature/epic-3-firmware-analysis
```

### 2. Database Schema Changes

```bash
# 1. Create migration file
# File: apps/desktop/src-tauri/migrations/YYYYMMDD_description.sql

# 2. Update Rust models
# File: apps/desktop/src-tauri/src/*/models.rs

# 3. Update TypeScript types
# File: apps/desktop/src/types/*.ts

# 4. Test migration
npm run dev  # Migration runs automatically

# 5. Run tests to ensure compatibility
npm run test
```

### 3. Epic 3 Firmware Development

```bash
# Focus areas for Epic 3 development:

# Backend firmware module
apps/desktop/src-tauri/src/firmware/
apps/desktop/src-tauri/src/firmware_analysis/

# Frontend firmware components  
apps/desktop/src/components/firmware/
apps/desktop/src/store/firmware.ts
apps/desktop/src/types/firmware.ts

# Recovery components
apps/desktop/src/components/recovery/
apps/desktop/src-tauri/src/recovery/
```

## Debugging and Troubleshooting

### Frontend Debugging

```bash
# Enable React DevTools
# Install browser extension: React Developer Tools

# Enable detailed error messages
export NODE_ENV=development

# Debug Tauri IPC calls
# Check browser console for IPC communication logs
```

### Backend Debugging

```bash
# Enable detailed Rust logging
export RUST_LOG=debug

# Enable Rust backtraces
export RUST_BACKTRACE=full

# Debug database queries
export RUST_LOG=sqlx=debug

# Run with debugger (requires setup)
cargo run --manifest-path apps/desktop/src-tauri/Cargo.toml
```

### Common Issues and Solutions

| Issue | Solution |
|-------|----------|
| Port 1420 in use | Kill process: `lsof -ti:1420 \| xargs kill` |
| Rust compilation fails | Update toolchain: `rustup update` |
| Frontend build fails | Clear cache: `rm -rf node_modules && npm install` |
| Database locked | Stop all app instances, restart |
| Permission denied (Linux) | Install webkit2gtk: `sudo apt install webkit2gtk-4.0-dev` |

## Performance Optimization

### Development Performance

```bash
# Use fast builds during development
export TAURI_BUNDLE_TARGETS=none

# Reduce Rust compilation time
export RUSTC_WRAPPER=sccache  # Install sccache first

# Parallel compilation
export CARGO_BUILD_JOBS=4
```

### Production Builds

```bash
# Optimize for size and performance
npm run build

# The build process automatically:
# - Minifies JavaScript/CSS
# - Optimizes Rust binary
# - Compresses assets
# - Generates platform-specific installers
```

## Team Development Guidelines

### Code Review Process

1. All changes require PR review
2. Automated tests must pass
3. Frontend and backend changes reviewed separately
4. Security-sensitive changes require admin review

### Branch Naming Conventions

```bash
feature/epic-N-description   # New features
bugfix/issue-description     # Bug fixes  
hotfix/critical-issue        # Critical production fixes
refactor/component-name      # Code refactoring
docs/section-name           # Documentation updates
```

### Commit Message Format

```bash
type(scope): description

# Examples:
feat(firmware): Add firmware analysis component
fix(auth): Resolve session timeout issue
docs(api): Update API specification
test(components): Add Dashboard component tests
```

This development workflow ensures efficient, consistent, and high-quality development of FerroCodex while supporting the complex requirements of Epic 3 firmware management features.