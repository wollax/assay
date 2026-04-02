# List available recipes
default:
    @just --list

# Build all workspace crates
build:
    cargo build --workspace

# Run all tests
test:
    cargo test --workspace

# Run clippy lints
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Run all checks (CI-equivalent)
ready: fmt-check lint test
    @echo "All checks passed."

# Remove build artifacts older than 14 days (requires cargo-sweep)
sweep:
    cargo sweep -t 14

# Install cargo-sweep if not present
install-sweep:
    cargo install cargo-sweep
