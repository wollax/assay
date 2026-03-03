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
ready: fmt-check lint test deny check-plugin-version
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

# Sync plugin.json version with workspace Cargo.toml version
sync-plugin-version:
    #!/usr/bin/env bash
    set -euo pipefail
    version=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    for f in plugins/claude-code/.claude-plugin/plugin.json; do
        if [ -f "$f" ]; then
            jq --arg v "$version" '.version = $v' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
            echo "  synced $f -> $version"
        fi
    done
    echo "Plugin versions synced to $version"

# Check plugin.json version matches workspace version
check-plugin-version:
    #!/usr/bin/env bash
    set -euo pipefail
    expected=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    for f in plugins/claude-code/.claude-plugin/plugin.json; do
        if [ -f "$f" ]; then
            actual=$(jq -r '.version' "$f")
            if [ "$expected" != "$actual" ]; then
                echo "ERROR: $f version ($actual) != workspace version ($expected)"
                echo "Run 'just sync-plugin-version' to fix."
                exit 1
            fi
        fi
    done
    echo "Plugin versions match workspace ($expected)."
