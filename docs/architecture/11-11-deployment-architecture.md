# Deployment Architecture

_(This section details the strategy for creating native installers via GitHub Releases, deploying the serverless backend via AWS CDK, the CI/CD pipeline steps, and the definitions for Development, Staging, and Production environments.)_

## Automated Binary Building with GitHub Actions

Ferrocodex uses GitHub Actions for automated cross-platform binary building and release management.

### Release Workflow

**Trigger**: Git tags starting with `v` (e.g., `v1.0.0`) or manual dispatch
**File**: `.github/workflows/release.yml`

**Supported Platforms**:
- Windows (x64)
- Linux (x64, Ubuntu 22.04)
- macOS Intel (x86_64)
- macOS Apple Silicon (ARM64)

**Process**:
1. Checkout code and setup Node.js LTS
2. Install Rust stable toolchain with platform-specific targets
3. Install platform dependencies (WebKit for Linux)
4. Install npm dependencies
5. Build Tauri application using `tauri-action`
6. Create GitHub release with binaries as downloadable assets
7. Release created as draft for review before publishing

### Build/Test Workflow

**Trigger**: Pushes to main/develop branches and pull requests
**File**: `.github/workflows/build.yml`

**Validation Steps**:
1. Run frontend tests (Vitest)
2. Run backend tests (Cargo)
3. Build Tauri app to ensure compilation works
4. Test on all supported platforms

### Usage

**Creating a Release**:
```bash
git tag v1.0.0
git push origin v1.0.0
```

This automatically triggers the release workflow, building binaries for all platforms and creating a GitHub release with downloadable assets.