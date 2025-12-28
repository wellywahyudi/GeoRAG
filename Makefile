.PHONY: help fmt fmt-check lint test build clean check all

help:
	@echo "Available commands:"
	@echo "  make fmt        - Format all Rust code"
	@echo "  make fmt-check  - Check if code is formatted"
	@echo "  make lint       - Run clippy linter"
	@echo "  make test       - Run all tests"
	@echo "  make build      - Build the project"
	@echo "  make check      - Run fmt-check, lint, and test"
	@echo "  make clean      - Clean build artifacts"
	@echo "  make all        - Format, lint, test, and build"

fmt:
	@echo "Formatting Rust code..."
	@cargo fmt --all

fmt-check:
	@echo "Checking Rust code formatting..."
	@cargo fmt --all -- --check

lint:
	@echo "Running clippy..."
	@cargo clippy --all-targets --all-features -- -D warnings

test:
	@echo "Running tests..."
	@cargo test --all

build:
	@echo "Building project..."
	@cargo build --all

build-release:
	@echo "Building release..."
	@cargo build --all --release

clean:
	@echo "Cleaning build artifacts..."
	@cargo clean

check: fmt-check lint test
	@echo "All checks passed!"

all: fmt lint test build
	@echo "Build complete!"

# Development helpers
watch:
	@echo "Watching for changes..."
	@cargo watch -x check -x test

doc:
	@echo "Building documentation..."
	@cargo doc --all --no-deps --open

doc-private:
	@echo "Building documentation (including private items)..."
	@cargo doc --all --no-deps --document-private-items --open
