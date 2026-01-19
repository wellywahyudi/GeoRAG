# Contributing to GeoRAG

Thank you for your interest in contributing to GeoRAG! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Code Style](#code-style)
- [Documentation](#documentation)

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please be respectful and constructive in all interactions.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/georag.git
   cd georag
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/wellywahyudi/georag.git
   ```

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Cargo
- Ollama (for embedding generation)
- Git

### Build the Project

```bash
# Build all crates
cargo build

# Build with release optimizations
cargo build --release

# Build specific crate
cargo build -p georag-core
```

### Run Tests

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p georag-core

# Run with output
cargo test -- --nocapture
```

## Making Changes

### Branch Naming

Use descriptive branch names:

- `feature/add-postgres-support` - New features
- `fix/crs-validation-bug` - Bug fixes
- `docs/improve-readme` - Documentation
- `refactor/simplify-query-api` - Refactoring
- `test/add-property-tests` - Tests

### Commit Messages

Write clear, descriptive commit messages:

```
Add PostgreSQL vector store integration

- Implement PostgresStore trait
- Add connection pooling
- Include integration tests
- Update documentation

Closes #123
```

**Format:**

- First line: Brief summary (50 chars or less)
- Blank line
- Detailed description (wrap at 72 chars)
- Reference issues/PRs

### Keep Your Fork Updated

```bash
# Fetch upstream changes
git fetch upstream

# Merge upstream main into your branch
git merge upstream/main
```

## Testing

### Unit Tests

Write unit tests for all new functionality:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_initialization() {
        let config = WorkspaceConfig::default();
        let workspace = Workspace::new(config);
        assert!(workspace.is_ok());
    }
}
```

### Integration Tests

Add integration tests in the `tests/` directory:

```rust
// tests/workspace_integration_test.rs
#[test]
fn test_full_workflow() {
    // Test complete workflow
}
```

### Property-Based Tests

For core functionality, add property-based tests:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_crs_normalization(crs in 1000u32..10000u32) {
        // Property test
    }
}
```

### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_workspace_initialization

# With logging
RUST_LOG=debug cargo test

# Integration tests only
cargo test --test '*'
```

## Submitting Changes

### Before Submitting

1. **Run tests**: `cargo test`
2. **Run clippy**: `cargo clippy -- -D warnings`
3. **Format code**: `cargo fmt`
4. **Update docs**: If you changed APIs
5. **Add tests**: For new functionality

### Pull Request Process

1. **Push your branch** to your fork
2. **Open a Pull Request** against `main`
3. **Fill out the PR template** completely
4. **Link related issues** using keywords (Fixes #123)
5. **Wait for review** and address feedback

### PR Checklist

- [ ] Tests pass locally
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated
- [ ] CHANGELOG.md updated (if applicable)
- [ ] Tests added for new functionality
- [ ] Commit messages are clear

## Code Style

### Rust Style

Follow the [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/):

- Use `cargo fmt` for formatting
- Follow naming conventions:
  - `snake_case` for functions and variables
  - `PascalCase` for types and traits
  - `SCREAMING_SNAKE_CASE` for constants
- Keep functions focused and small
- Write descriptive variable names

### Documentation

Document all public APIs:

````rust
/// Creates a new workspace with the given configuration.
///
/// # Arguments
///
/// * `path` - The workspace directory path
/// * `config` - Workspace configuration
///
/// # Returns
///
/// Returns `Ok(Workspace)` on success, or an error if initialization fails.
///
/// # Examples
///
/// ```
/// use georag_core::{Workspace, WorkspaceConfig};
///
/// let config = WorkspaceConfig::default();
/// let workspace = Workspace::init("./my-workspace", config)?;
/// ```
pub fn init(path: impl AsRef<Path>, config: WorkspaceConfig) -> Result<Self> {
    // Implementation
}
````

### Error Handling

- Use `Result<T, E>` for fallible operations
- Use `anyhow::Result` for application code
- Use custom error types for libraries
- Provide context with `.context()`

```rust
use anyhow::{Context, Result};

fn load_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .context("Failed to read config file")?;

    toml::from_str(&content)
        .context("Failed to parse config")
}
```

## Documentation

### Code Documentation

- Document all public items
- Include examples in doc comments
- Explain complex algorithms
- Document panics and safety

### User Documentation

Update relevant documentation in `docs/`:

- `docs/README.md` - Main documentation
- `docs/CLI.md` - CLI commands
- `docs/API.md` - REST API reference
- Add new guides as needed

### Examples

Add examples to `crates/*/examples/`:

```rust
// crates/georag-core/examples/basic_usage.rs
use georag_core::Workspace;

fn main() -> anyhow::Result<()> {
    // Example code
    Ok(())
}
```

## Project Structure

```
georag/
├── crates/
│   ├── georag-core/       # Core domain logic, geo ops, llm traits
│   ├── georag-retrieval/  # Search and ranking
│   ├── georag-store/      # Storage abstractions
│   ├── georag-cli/        # Command-line interface
│   └── georag-api/        # HTTP API
├── docs/                  # Documentation
└── examples/              # Example code
```

## Areas for Contribution

### High Priority

- [ ] Property-based testing suite
- [ ] PostgreSQL/PostGIS integration
- [ ] Additional embedding providers
- [ ] Performance optimizations
- [ ] Documentation improvements

### Good First Issues

Look for issues labeled `good-first-issue` on GitHub. These are great starting points for new contributors.

### Feature Requests

Have an idea? [Open a discussion](https://github.com/wellywahyudi/georag/discussions) to talk about it before implementing.

## Getting Help

- 📖 Read the [documentation](docs/)
- 💬 Ask in [Discussions](https://github.com/wellywahyudi/georag/discussions)
- 🐛 Check existing [issues](https://github.com/wellywahyudi/georag/issues)
- 📧 Contact the maintainers

## Recognition

Contributors will be recognized in:

- README.md contributors section
- Release notes
- Project documentation

Thank you for contributing to GeoRAG! 🌍
