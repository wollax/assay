# External Integrations

**Analysis Date:** 2026-02-28

## APIs & External Services
**None configured in core codebase.**
- Assay is designed as a spec-driven development kit for local/internal workflows
- No API clients, HTTP libraries (no reqwest, hyper, etc.) in dependencies
- Future integrations likely: agent APIs, code review systems, version control APIs
- Prepared for async via Tokio 1.x (full features) but not yet utilized

## Data Storage
**File-based configuration:**
- JSON file format (via serde_json) for specs, gates, reviews, workflows
  - Config type: `Config` struct in `/Users/wollax/Git/personal/assay/crates/assay-types/src/lib.rs`
  - Structure: `{ project_name, workflows: [{ name, specs, gates }] }`
- No database integration (SQL, NoSQL, key-value stores)
- Configuration loading/validation module: `/Users/wollax/Git/personal/assay/crates/assay-core/src/config/mod.rs`

**Schema validation:**
- Schemars 0.8 generates JSON schemas from `Spec`, `Gate`, `Review`, `Workflow`, `Config` types
  - Types defined in: `/Users/wollax/Git/personal/assay/crates/assay-types/src/lib.rs`
  - Enables validation of external JSON config files against generated schemas

## Authentication & Identity
**None implemented.**
- No identity/auth mechanisms (OAuth, JWT, API keys, etc.)
- Agentic systems via plugins will manage their own authentication
  - Claude Code plugin: `/Users/wollax/Git/personal/assay/plugins/claude-code/`
  - OpenCode plugin: `/Users/wollax/Git/personal/assay/plugins/opencode/`
- Future: may require credential/token management for integrated agent APIs

## Monitoring & Observability
**Error handling:**
- Thiserror 2.x - Custom error types with context
  - Usage: `/Users/wollax/Git/personal/assay/crates/assay-core/Cargo.toml`
- Color-eyre 0.6 - Rich error formatting in TUI
  - Usage: `/Users/wollax/Git/personal/assay/crates/assay-tui/src/main.rs` (installed with `color_eyre::install()`)

**Logging:**
- No logging framework configured (log, tracing, slog)
- Future: tracing integration likely for workflow audit trails

**Debugging:**
- Environment: `RUST_BACKTRACE = 1` in `.mise.toml` for enhanced panic info

## CI/CD & Deployment
**GitHub Actions pipeline:**
- Workflow file: `/Users/wollax/Git/personal/assay/.github/workflows/ci.yml`
- Triggers: push to main, pull requests
- Jobs:
  1. **Check job** (ubuntu-latest):
     - Rust toolchain: dtolnay/rust-toolchain@stable with rustfmt, clippy
     - Caching: Swatinem/rust-cache@v2
     - Tool setup: jdx/mise-action@v2
     - Command: `just ready` (fmt-check + lint + test + deny)
  2. **Plugins job** (ubuntu-latest):
     - Python 3 JSON validation for plugin manifests:
       - `/Users/wollax/Git/personal/assay/plugins/claude-code/.claude-plugin/plugin.json`
       - `/Users/wollax/Git/personal/assay/plugins/opencode/opencode.json`
       - `/Users/wollax/Git/personal/assay/plugins/opencode/package.json`
       - `/Users/wollax/Git/personal/assay/plugins/opencode/tsconfig.json`

**Release/deployment:**
- No automated release pipeline configured
- Manual: `cargo build --release` via workspace Cargo.toml
- Binaries produced: `assay-cli`, `assay-tui`

## Environment Configuration
**Tool management (.mise.toml):**
```
rust = "stable"
just = "latest"
node = "24"
cargo:cargo-deny = "latest"
RUST_BACKTRACE = 1
```
- Location: `/Users/wollax/Git/personal/assay/.mise.toml`

**Rust toolchain (rust-toolchain.toml):**
```
channel = "stable"
components = ["rustfmt", "clippy"]
```
- Location: `/Users/wollax/Git/personal/assay/rust-toolchain.toml`

**Cargo workspace configuration:**
- Resolver: version 2 (faster, more correct)
- Edition: 2024 (across all crates)
- Root: `/Users/wollax/Git/personal/assay/Cargo.toml`

**License compliance:**
- Cargo-deny configured in `/Users/wollax/Git/personal/assay/deny.toml`
- Allowed licenses: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unicode-DFS-2016
- Package registry: crates.io only (unknown-registry = warn)
- Multiple versions detection: warn (no duplicates enforced)

## Webhooks & Callbacks
**None implemented.**
- Plugins system (Claude Code, OpenCode) will handle external integrations
  - Plugin discovery: hooks.json format in `/Users/wollax/Git/personal/assay/plugins/claude-code/hooks/hooks.json`
  - Plugin manifests define integration points
- Future: webhook support for spec updates, gate changes, review notifications

---
*Integration audit: 2026-02-28*
