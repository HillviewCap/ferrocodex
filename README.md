# Ferrocodex

A secure cross-platform desktop application for password management and encrypted storage built with Tauri 2.0, React, and Rust.

## Architecture

- **Frontend**: React 18 with TypeScript and Ant Design
- **Backend**: Rust with Tauri 2.0 framework
- **Database**: SQLite with rusqlite
- **State Management**: Zustand
- **Build System**: Turborepo monorepo
- **Testing**: Vitest (frontend) + Rust built-in testing (backend)

## Prerequisites

- **Node.js**: 18+ 
- **Rust**: 1.78.0+
- **Platform-specific dependencies**:
  - **Linux**: `libwebkit2gtk-4.0-dev`, `build-essential`, `curl`, `wget`, `file`, `libgtk-3-dev`, `librsvg2-dev`
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Microsoft Visual Studio C++ Build Tools

## Installation

1. **Clone the repository**:
   ```bash
   git clone <repository-url>
   cd ferrocodex
   ```

2. **Install dependencies**:
   ```bash
   npm install
   ```

3. **Install platform dependencies** (Linux example):
   ```bash
   sudo apt update
   sudo apt install libwebkit2gtk-4.0-dev build-essential curl wget file libgtk-3-dev librsvg2-dev
   ```

## Development

### Running in Development Mode

```bash
# Start the development server (both frontend and backend)
npm run dev

# Or run Tauri development mode directly
npm run tauri:dev
```

The application will start with hot reload enabled for both the React frontend and Rust backend.

### Building for Production

```bash
# Build the application
npm run build

# Build Tauri application (creates platform-specific bundles)
npm run tauri:build
```

### Testing

```bash
# Run frontend tests
npm run test

# Run frontend tests once
npm run test:run

# Run backend tests
cargo test --manifest-path src-tauri/Cargo.toml

# Run all tests
npm run test && cargo test --manifest-path src-tauri/Cargo.toml
```

## Project Structure

```
ferrocodex/
├── apps/
│   └── desktop/              # Main Tauri application
│       ├── src/              # React frontend source
│       │   ├── components/   # React components
│       │   ├── store/        # Zustand state management
│       │   └── ...
│       ├── src-tauri/        # Rust backend source
│       │   ├── src/          # Rust source code
│       │   │   ├── database/ # Database module
│       │   │   └── ...
│       │   └── Cargo.toml    # Rust dependencies
│       ├── package.json      # Frontend dependencies
│       └── vite.config.ts    # Vite configuration
├── packages/
│   └── shared-types/         # Shared TypeScript types
├── package.json              # Root workspace configuration
├── turbo.json               # Turborepo configuration
└── tsconfig.json            # Root TypeScript configuration
```

## Key Features

- **Cross-platform**: Builds for Windows, macOS, and Linux
- **Secure**: Rust backend ensures memory safety
- **Local-first**: SQLite database for offline functionality
- **Modern UI**: Ant Design components with React
- **Type-safe**: Full TypeScript support
- **Testing**: Comprehensive test coverage
- **Hot Reload**: Fast development experience

## Database

The application uses SQLite for local data storage with the following features:

- **Location**: Stored in the user's application data directory
- **Schema**: Automatic initialization with version tracking
- **Performance**: WAL mode and optimized settings
- **Encryption**: Infrastructure prepared for future implementation

### Database Commands

The Tauri backend exposes the following commands for database operations:

- `initialize_database()`: Initialize the database connection
- `database_health_check()`: Verify database connectivity

## Contributing

1. Ensure all tests pass before submitting changes
2. Follow the existing code style and conventions
3. Add tests for new functionality
4. Update documentation as needed

## Troubleshooting

### Common Issues

1. **Build fails on Linux**: Ensure all system dependencies are installed
2. **Tauri dev mode doesn't start**: Check that ports 1420-1421 are available
3. **Database initialization fails**: Verify write permissions to app data directory

### Platform-specific Notes

- **macOS**: Bundle identifier should not end with `.app` (warning will be shown)
- **Linux**: Requires WebKit2GTK for web view functionality
- **Windows**: May require specific Visual Studio components

## License

[License information]

## Support

[Support information]