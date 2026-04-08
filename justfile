set dotenv-load

# List available recipes
default:
    @just --list

# Build all workspace crates
build:
    cargo build --workspace

# Build only assay crates
build-assay:
    cargo build --workspace --exclude smelt-core --exclude smelt-cli

# Build only smelt crates
build-smelt:
    cargo build -p smelt-core -p smelt-cli

# Run all tests (with per-test timeout via cargo-nextest)
test:
    cargo nextest run --workspace

# Run only assay tests
test-assay:
    cargo nextest run --workspace --exclude smelt-core --exclude smelt-cli

# Run only smelt tests (docker_lifecycle tests skip gracefully when Docker is unavailable)
test-smelt:
    cargo nextest run -p smelt-core -p smelt-cli

# Run only smelt unit tests, excluding docker_lifecycle integration tests
test-smelt-unit:
    cargo nextest run -p smelt-core -p smelt-cli -E 'not binary(docker_lifecycle)'

# Run clippy lints
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Run clippy lints for assay crates only
lint-assay:
    cargo clippy --workspace --exclude smelt-core --exclude smelt-cli --all-targets -- -D warnings

# Run clippy lints for smelt crates only
lint-smelt:
    cargo clippy -p smelt-core -p smelt-cli --all-targets -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Format only assay crates.
# Note: cargo fmt has no --exclude flag, so we use an explicit -p list here.
# Unlike build-assay/test-assay/lint-assay (which use --workspace --exclude smelt-*),
# this recipe must be updated manually when new assay crates are added.
fmt-assay:
    cargo fmt -p assay-types -p assay-core -p assay-backends -p assay-harness -p assay-mcp -p assay-cli -p assay-tui

# Format only smelt crates
fmt-smelt:
    cargo fmt -p smelt-core -p smelt-cli

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Run cargo-deny checks (licenses, advisories, bans)
deny:
    cargo deny check

# Run all checks (CI-equivalent)
ready: fmt-check lint test deny check-plugin-version
    @echo "All checks passed."

# Remove build artifacts older than 14 days (requires cargo-sweep)
sweep:
    cargo sweep -t 14

# Install cargo-sweep if not present
install-sweep:
    cargo install cargo-sweep

# Set up git hooks (run once after clone)
setup:
    git config core.hooksPath .githooks
    @echo "Git hooks installed from .githooks/"

# Watch for changes and rebuild
dev:
    cargo watch -x 'build --workspace'

# Run the CLI
cli *ARGS:
    cargo run -p assay-cli -- {{ ARGS }}

# Run the TUI
tui:
    cargo run -p assay-tui

# Run the close-the-loop demo (M024 checkpoint + auto-promote)
demo-close-the-loop:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== Close the Loop Demo (M024) ==="
    echo ""
    # Setup
    ./examples/close-the-loop/setup.sh
    echo ""
    # Abort path
    ./examples/close-the-loop/run-abort.sh
    echo ""
    # Reset for clean path
    ./examples/close-the-loop/reset.sh
    echo ""
    # Promote path
    ./examples/close-the-loop/run-promote.sh

# Generate JSON Schema files for all public types
schemas:
    cargo run -p assay-types --example generate-schemas

# Check that generated schemas are up to date (for CI)
schemas-check:
    #!/usr/bin/env bash
    set -euo pipefail
    tmpdir=$(mktemp -d)
    cp -r schemas/ "$tmpdir/schemas-expected"
    just schemas
    if ! diff -r schemas/ "$tmpdir/schemas-expected" > /dev/null 2>&1; then
        echo "ERROR: schemas/ is out of date. Run 'just schemas' and commit."
        rm -rf "$tmpdir"
        exit 1
    fi
    rm -rf "$tmpdir"
    echo "Schemas are up to date."

# Extract workspace version from Cargo.toml
_workspace-version:
    @grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'

plugin_json := "plugins/claude-code/.claude-plugin/plugin.json"

# Sync plugin.json version with workspace Cargo.toml version
sync-plugin-version:
    #!/usr/bin/env bash
    set -euo pipefail
    version=$(just _workspace-version)
    f="{{ plugin_json }}"
    if [ -f "$f" ]; then
        jq --arg v "$version" '.version = $v' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
        echo "  synced $f -> $version"
    fi
    echo "Plugin versions synced to $version"

# Check plugin.json version matches workspace version
check-plugin-version:
    #!/usr/bin/env bash
    set -euo pipefail
    expected=$(just _workspace-version)
    f="{{ plugin_json }}"
    if [ -f "$f" ]; then
        actual=$(jq -r '.version' "$f")
        if [ "$expected" != "$actual" ]; then
            echo "ERROR: $f version ($actual) != workspace version ($expected)"
            echo "Run 'just sync-plugin-version' to fix."
            exit 1
        fi
    fi
    echo "Plugin versions match workspace ($expected)."
