# Changelog

All notable changes to Ferrocodex will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2025-01-25

### Fixed
- **Firmware Upload Permissions**: Fixed firmware upload access control to allow both Engineer and Administrator roles to upload firmware files
- **Role-Based Access Control**: Updated permission model to ensure proper access for administrative users
- **Test Coverage**: Updated test suites to reflect correct permission requirements

## [0.3.0] - 2025-01-25

### Added
- **Complete Asset Recovery System**: Comprehensive recovery package export functionality combining firmware and configuration data
- **Comprehensive Test Coverage**: Added extensive test coverage for critical components including firmware management, configuration handling, and UI components
- **Enhanced Architecture Documentation**: Complete documentation for API, data models, components, testing standards, coding standards, error handling, and monitoring

### Fixed
- **Firmware Management Improvements**:
  - Fixed firmware upload and analysis issues
  - Resolved firmware status change error handling
  - Increased firmware file size limit to 2GB
  - Fixed firmware dropdown width to prevent UI overlap
  - Firmware assignment now automatically updates on selection
  - Improved handling of missing or invalid firmware metadata
- **Configuration Management**:
  - Fixed refresh of version history table after branch operations
  - Compacted Export Configuration modal layout for better UX
- **Test Infrastructure**:
  - Resolved database schema test issues for better release stability
  - Fixed missing firmware schema initialization in configuration and branch tests
  - Removed duplicate table creation in firmware linking tests

### Technical Improvements
- Enhanced firmware analysis system with better error handling
- Improved configuration versioning and branch management
- Strengthened test suite with 390+ tests covering critical functionality
- Better separation of concerns in component architecture

### Developer Experience
- Complete architecture documentation now available
- Established coding standards and testing guidelines
- Improved error handling patterns across the application
- Enhanced monitoring and observability capabilities

## [0.2.0] - Previous Release
- Initial release with basic functionality
- Password management capabilities
- Encrypted storage system
- Cross-platform desktop application

[0.3.0]: https://github.com/ferrocodex/ferrocodex/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ferrocodex/ferrocodex/releases/tag/v0.2.0