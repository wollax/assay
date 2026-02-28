# Technology Stack

**Analysis Date:** 2026-02-28

## Languages
**Primary:**
- Rust (stable, 2024 edition) - Core domain logic, CLI binary, TUI application
- Python 3 - CI/CD plugin validation scripts (minimal, in GitHub Actions)
- TypeScript - OpenCode plugin implementation (`plugins/opencode/`)
- JSON - Plugin configuration and manifests

## Runtime
**Environment:**
- Rust stable toolchain - All compiled binaries (`crates/assay-cli`, `crates/assay-tui`)
- Linux/macOS/Windows (cross-platform via Rust)

**Package Manager:**
- Cargo (Rust) - Workspace dependency management
- npm - Node.js package management for TypeCode plugin
- Lockfile: `Cargo.lock` (present), `package-lock.json` (for plugins)

## Frameworks
**Core:**
- Tokio 1.x (full features) - Async runtime (imported but not actively used in MVP)
- Serde 1.x - Serialization/deserialization (JSON, YAML via external tools)
- Schemars 0.8 - JSON Schema generation from Rust types

**CLI:**
- Clap 4.x - Command-line argument parsing and interface
  - Dependency: `crates/assay-cli/Cargo.toml`
  - Used in: `/Users/wollax/Git/personal/assay/crates/assay-cli/src/main.rs`

**TUI:**
- Ratatui 0.30 - Terminal user interface framework
  - Dependency: `crates/assay-tui/Cargo.toml`
  - Used in: `/Users/wollax/Git/personal/assay/crates/assay-tui/src/main.rs`
- Crossterm 0.28 - Terminal event handling and rendering backend
  - Dependency: `crates/assay-tui/Cargo.toml`
- Color-eyre 0.6 - Error reporting and formatting for TUI

**Error Handling:**
- Thiserror 2.x - Error type derivation and handling
  - Dependency: `crates/assay-core/Cargo.toml`

**Testing:**
- Rust built-in test framework (cargo test) - Unit tests
- No external testing framework dependencies configured

**Build/Dev:**
- Just (latest) - Task runner for build commands (`.mise.toml`, `justfile`)
- Cargo-deny (latest) - License auditing, security advisory checking, dependency bans
  - Configured in: `/Users/wollax/Git/personal/assay/deny.toml`
- Cargo-watch - File watcher for development rebuilds
  - Referenced in: `justfile` (`just dev` recipe)
- Rustfmt (stable component) - Code formatting
- Clippy (stable component) - Linting with cognitive complexity threshold of 25

## Key Dependencies
**Critical:**
- `serde` 1.x, `serde_json` 1.x - JSON serialization; enables type-driven configuration
- `schemars` 0.8 - Schema generation from types; enables spec validation
- `clap` 4.x - CLI parsing; all CLI subcommands and flags
- `ratatui` 0.30 + `crossterm` 0.28 - TUI rendering; all terminal interactions
- `tokio` 1.x (full) - Async runtime foundation (prepared for future async operations)
- `thiserror` 2.x - Custom error types across domain logic
- `color-eyre` 0.6 - User-friendly error reports in TUI

**Workspace Dependencies Management:**
- All dependencies declared in `[workspace.dependencies]` in root `/Users/wollax/Git/personal/assay/Cargo.toml`
- Each crate references via `.workspace = true` to ensure version consistency
- Reference: `/Users/wollax/Git/personal/assay/crates/assay-types/Cargo.toml` (type crate example)

## Configuration
**Environment:**
- `.mise.toml` - Polyglot tool management (Rust, Node.js 24, Just, cargo-deny)
  - Env var: `RUST_BACKTRACE = 1` (for debugging)
- `rust-toolchain.toml` - Rust version and components lock
  - Channel: stable
  - Components: rustfmt, clippy

**Build:**
- Root Cargo.toml: Workspace configuration with members at `crates/*`
- Edition: 2024 (modern Rust semantics)
- License: MIT (workspace-level)
- Per-crate Cargo.toml files: `crates/assay-types/`, `crates/assay-core/`, `crates/assay-cli/`, `crates/assay-tui/`

**Linting & Code Quality:**
- `rustfmt.toml` - Formatting config (edition 2024)
- `clippy.toml` - Cognitive complexity threshold: 25
- `deny.toml` - License allowlist (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode variants)
  - Multiple versions warning; bans unknown registries/git sources

## Platform Requirements
**Development:**
- macOS Tahoe 26.2 or Linux/Windows
- Rust stable (managed via mise)
- Node.js 24 LTS (for OpenCode plugin development)
- Just (task runner)
- Git (version control)
- iTerm2 + zsh (optional; developer preference)

**Production:**
- Linux/macOS/Windows binaries via Rust compilation
- No external runtime dependencies (statically linked via Rust)
- Terminal with ANSI color support (for TUI)

---
*Stack analysis: 2026-02-28*
