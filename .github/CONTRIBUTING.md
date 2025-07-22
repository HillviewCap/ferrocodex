# Contributing to Ferrocodex

First off, thank you for considering contributing to Ferrocodex! It's people like you that make Ferrocodex such a great tool for the OT community.

## Code of Conduct

This project and everyone participating in it is governed by the [Ferrocodex Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check existing issues as you might find out that you don't need to create one. When you are creating a bug report, please include as many details as possible:

* **Use a clear and descriptive title**
* **Describe the exact steps which reproduce the problem**
* **Provide specific examples to demonstrate the steps**
* **Describe the behavior you observed after following the steps**
* **Explain which behavior you expected to see instead and why**
* **Include screenshots and animated GIFs if relevant**
* **Include your environment details** (OS, Node version, Rust version)

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, please include:

* **Use a clear and descriptive title**
* **Provide a step-by-step description of the suggested enhancement**
* **Provide specific examples to demonstrate the steps**
* **Describe the current behavior and explain which behavior you expected to see instead**
* **Explain why this enhancement would be useful to most Ferrocodex users**

### Pull Requests

1. Fork the repo and create your branch from `main`
2. If you've added code that should be tested, add tests
3. If you've changed APIs, update the documentation
4. Ensure the test suite passes
5. Make sure your code lints
6. Issue that pull request!

## Development Process

1. **Setup Development Environment**
   ```bash
   git clone https://github.com/ferrocodex/ferrocodex.git
   cd ferrocodex
   npm install
   ```

2. **Make Your Changes**
   - Follow the existing code style
   - Write meaningful commit messages
   - Add tests for new functionality
   - Update documentation as needed

3. **Test Your Changes**
   ```bash
   # Run all tests
   npm run test
   
   # Run frontend tests
   cd apps/desktop
   npm run test:run
   
   # Run backend tests
   cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
   ```

4. **Submit Your Changes**
   - Push your changes to your fork
   - Create a pull request with a clear title and description
   - Link any relevant issues

## Styleguides

### Git Commit Messages

* Use the present tense ("Add feature" not "Added feature")
* Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
* Limit the first line to 72 characters or less
* Reference issues and pull requests liberally after the first line

### JavaScript/TypeScript Styleguide

* Use TypeScript for all new code
* Follow the existing code style
* Use meaningful variable names
* Add JSDoc comments for public APIs

### Rust Styleguide

* Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
* Use `cargo fmt` to format your code
* Use `cargo clippy` to catch common mistakes
* Write doc comments for public items

## Project Structure

Understanding the project structure will help you navigate the codebase:

```
ferrocodex/
├── apps/desktop/          # Main Tauri application
│   ├── src/              # React frontend
│   └── src-tauri/        # Rust backend
├── packages/             # Shared packages
└── docs/                 # Documentation
```

## License

By contributing to Ferrocodex, you agree that your contributions will be licensed under its AGPL-3.0 license.