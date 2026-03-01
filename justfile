set dotenv-load

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

# Run cargo-deny checks (licenses, advisories, bans)
deny:
    cargo deny check

# Run all checks (CI-equivalent)
ready: fmt-check lint test deny
    @echo "All checks passed."

# Watch for changes and rebuild
dev:
    cargo watch -x 'build --workspace'

# Run the CLI
cli *ARGS:
    cargo run -p assay-cli -- {{ ARGS }}

# Run the TUI
tui:
    cargo run -p assay-tui
