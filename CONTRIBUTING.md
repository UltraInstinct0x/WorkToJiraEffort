# Contributing to WorkToJiraEffort

Thank you for your interest in contributing to WorkToJiraEffort! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please be respectful and considerate in all interactions.

## Getting Started

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/WorkToJiraEffort.git
   cd WorkToJiraEffort
   ```
3. Build the project:
   ```bash
   cargo build
   ```

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Git
- (Optional) Screenpipe for testing

### Building

```bash
cargo build
```

### Running

```bash
cargo run -- <command>
```

### Testing

```bash
cargo test
```

### Formatting

Before submitting code, ensure it's properly formatted:

```bash
cargo fmt
```

### Linting

Check for common mistakes:

```bash
cargo clippy
```

## Project Structure

```
src/
├── main.rs         # CLI entry point and command handling
├── config.rs       # Configuration management
├── screenpipe.rs   # Screenpipe API client
├── jira.rs         # Jira API integration
├── salesforce.rs   # Salesforce API integration
└── tracker.rs      # Core tracking logic
```

## Making Changes

1. Create a new branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes

3. Test your changes:
   ```bash
   cargo test
   cargo build
   cargo run -- check
   ```

4. Commit with a clear message:
   ```bash
   git commit -m "Add feature: description of your changes"
   ```

5. Push to your fork:
   ```bash
   git push origin feature/your-feature-name
   ```

6. Create a Pull Request

## Pull Request Guidelines

- Provide a clear description of the changes
- Reference any related issues
- Ensure all tests pass
- Update documentation if needed
- Follow the existing code style

## Areas for Contribution

### High Priority

- Additional API integrations (GitHub Projects, Linear, etc.)
- Improved activity detection algorithms
- GUI application
- Better error handling and recovery

### Documentation

- Tutorials and guides
- API documentation
- Code comments
- Examples

### Testing

- Unit tests
- Integration tests
- End-to-end tests

## Reporting Bugs

Please use GitHub Issues and include:

- Description of the bug
- Steps to reproduce
- Expected behavior
- Actual behavior
- Environment (OS, Rust version)
- Logs (with `RUST_LOG=debug`)

## Feature Requests

We welcome feature requests! Please:

- Check if it's already requested
- Provide a clear use case
- Explain the expected behavior

## Questions?

Feel free to open an issue for questions or discussions.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
