# Code Formatting Guide

This project uses `rustfmt` for consistent Rust code formatting.

## Quick Start

```bash
# Format all code
make fmt

# Or use cargo directly
cargo fmt --all

# Check formatting without modifying files
make fmt-check
cargo fmt --all -- --check
```

## Configuration

The project uses `rustfmt.toml` for formatting configuration:

- **Max line width**: 100 characters
- **Indentation**: 4 spaces
- **Import reordering**: Enabled
- **Try shorthand**: Enabled (`?` operator)
- **Field init shorthand**: Enabled

## Editor Integration

### VS Code

The `.vscode/settings.json` file is configured to:

- Format on save
- Use rust-analyzer as the formatter
- Run clippy on save

Install the [rust-analyzer extension](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

### Other Editors

Most editors support rustfmt through plugins:

- **Vim/Neovim**: Use `rust.vim` or `coc-rust-analyzer`
- **Emacs**: Use `rustic-mode`
- **IntelliJ/CLion**: Built-in Rust plugin supports rustfmt
- **Sublime Text**: Use `RustEnhanced` package

## CI Integration

The project includes formatting checks in CI:

```bash
# This will fail if code is not formatted
cargo fmt --all -- --check
```

## Advanced Features (Nightly)

Some formatting options require nightly Rust. To use them:

1. Install nightly: `rustup toolchain install nightly`
2. Uncomment nightly features in `rustfmt.toml`
3. Format with: `cargo +nightly fmt`

Nightly features include:

- Import grouping and granularity
- Comment formatting
- Trailing commas
- Brace style control
- And more...

## Pre-commit Hook

To automatically format code before committing:

```bash
# Create pre-commit hook
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/sh
cargo fmt --all -- --check
if [ $? -ne 0 ]; then
    echo "Code is not formatted. Run 'cargo fmt --all' to fix."
    exit 1
fi
EOF

chmod +x .git/hooks/pre-commit
```

## Makefile Commands

```bash
make fmt        # Format all code
make fmt-check  # Check formatting
make lint       # Run clippy
make test       # Run tests
make check      # Run fmt-check, lint, and test
make all        # Format, lint, test, and build
```

## EditorConfig

The `.editorconfig` file ensures consistent settings across editors:

- UTF-8 encoding
- LF line endings
- Trim trailing whitespace
- Insert final newline

Most modern editors support EditorConfig automatically.
