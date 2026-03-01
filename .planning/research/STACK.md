# Stack Research: v0.1.0 Vertical Slice

**Research Date:** 2026-02-28
**Scope:** New crate additions for specs, gates, MCP server, and Claude Code plugin
**Existing Stack:** Rust 1.93 stable (2024 edition), serde 1.x, schemars 0.8, clap 4, ratatui 0.30, thiserror 2, tokio 1, color-eyre 0.6

---

## Executive Summary

The v0.1.0 vertical slice requires three new workspace dependencies (rmcp, toml, tracing-subscriber) and one **breaking upgrade** (schemars 0.8 -> 1.x). The rmcp crate is the official MCP Rust SDK, actively maintained under the `modelcontextprotocol` GitHub org with a release as recent as 2026-02-27. The schemars upgrade is forced by rmcp's `server` feature, which depends on schemars 1.x -- these two versions cannot coexist in the same crate graph without type incompatibility. This upgrade is manageable because assay-types uses only `derive(JsonSchema)` with no advanced schemars APIs.

---

## New Dependencies

### 1. rmcp -- MCP Server SDK

| Attribute | Value |
|---|---|
| **Crate** | `rmcp` |
| **Version** | 0.17.0 |
| **Released** | 2026-02-27 |
| **License** | Apache-2.0 (in deny.toml allowlist) |
| **Repository** | [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) |
| **Docs** | [docs.rs/rmcp](https://docs.rs/rmcp/latest/rmcp/) |
| **Edition** | 2024 |
| **Status** | Official MCP SDK for Rust, actively maintained |

**Required Features for v0.1.0:**

| Feature | Purpose | Pulls In |
|---|---|---|
| `server` (default) | ServerHandler trait, tool routing, tool macros | schemars 1.x, pastey, transport-async-rw |
| `macros` (default) | `#[tool]` and `#[tool_router]` proc macros | rmcp-macros |
| `transport-io` | stdio transport (`rmcp::transport::stdio()`) | tokio io-std, transport-async-rw |

**Recommended Cargo.toml entry:**
```toml
rmcp = { version = "0.17", features = ["server", "transport-io"] }
```

Note: `server` and `macros` are default features. `transport-io` must be explicitly enabled for stdio support.

**Transitive Dependencies (notable):**
- `tokio` 1.x (sync, macros, rt, time, io-util, io-std) -- compatible with existing workspace tokio
- `serde` 1.x + `serde_json` 1.x -- already in workspace
- `schemars` 1.x -- **requires upgrade from 0.8** (see Migration section)
- `thiserror` 2.x -- already in workspace
- `tracing` 0.1.x -- new, lightweight (used internally by rmcp)
- `futures` 0.3 -- new transitive dependency
- `tokio-util` 0.7 -- new transitive dependency
- `async-trait` 0.1 -- new transitive dependency
- `pin-project-lite` 0.2 -- new transitive dependency
- `chrono` 0.4 -- new transitive dependency (used in MCP protocol messages)

**Server Implementation Pattern (from official examples):**
```rust
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, transport::stdio};

#[derive(Debug, Clone)]
struct AssayServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AssayServer {
    fn new() -> Self {
        Self { tool_router: Self::tool_router() }
    }

    #[tool(description = "Get a spec by name")]
    fn spec_get(&self, params: Parameters<SpecGetRequest>) -> String {
        // ...
    }
}

#[tool_handler]
impl ServerHandler for AssayServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Assay MCP server".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let service = AssayServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

**Key API Notes:**
- Tool parameter types must derive `serde::Deserialize` + `schemars::JsonSchema`
- Tool return types: `String`, `Result<CallToolResult, McpError>`, or `Result<Json<T>, E>` for structured output
- `#[tool_handler]` macro implements `ServerHandler` trait boilerplate, delegating tool routing to the `ToolRouter`
- Server uses `tracing` for internal logging; stderr is the convention (stdout is the MCP transport)
- `rmcp::schemars` re-exports the schemars crate for use in tool parameter derives

**Maturity Assessment:**
- Hosted under the `modelcontextprotocol` GitHub org (canonical/official)
- 12 example servers covering stdio, streaming HTTP, auth, structured output, prompts, sampling
- MCP SDK conformance tests added in v0.17.0
- Active development: 7 releases in the last 3 months
- 700+ GitHub stars, multiple production users
- **Risk:** Rapid release cadence means API churn is possible between minor versions. Pin to `0.17` (not `0.17.0`) for patch fixes, but be prepared for breaking changes on `0.18`.

### 2. toml -- TOML Parsing

| Attribute | Value |
|---|---|
| **Crate** | `toml` |
| **Version** | 1.0.3+spec-1.1.0 |
| **Released** | 2026-02-18 |
| **License** | MIT OR Apache-2.0 (both in deny.toml allowlist) |
| **Repository** | [toml-rs/toml](https://github.com/toml-rs/toml) |
| **Docs** | [docs.rs/toml](https://docs.rs/toml/latest/toml/) |
| **TOML Spec** | 1.1.0 |

**Required Features for v0.1.0:**

Default features (`std`, `serde`, `parse`, `display`) are sufficient. No special features needed.

**Recommended Cargo.toml entry:**
```toml
toml = "1"
```

**Usage Pattern:**
```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct AssayConfig {
    project_name: String,
    // ...
}

let config: AssayConfig = toml::from_str(&contents)?;
```

**Integration Notes:**
- Works directly with existing serde derives on assay-types structs
- `toml::from_str()` for deserialization, `toml::to_string_pretty()` for serialization (init command)
- Error types are `toml::de::Error` / `toml::ser::Error` -- wrap in AssayError via thiserror
- No async; file I/O is `std::fs::read_to_string()` which aligns with the sync gate evaluation decision

**Why not alternatives:**

| Alternative | Why Not |
|---|---|
| `basic-toml` | Minimal subset, missing features like preserve_order. Less maintained. |
| `toml_edit` | Preserves formatting/comments for round-trip editing. Overkill for v0.1.0 read-only config. Consider for future spec editing features. |
| `serde_toml` | Doesn't exist as a separate crate; `toml` IS the serde-based TOML crate. |

### 3. tracing-subscriber -- Logging for MCP Server

| Attribute | Value |
|---|---|
| **Crate** | `tracing-subscriber` |
| **Version** | 0.3.22 |
| **License** | MIT (in deny.toml allowlist) |
| **Repository** | [tokio-rs/tracing](https://github.com/tokio-rs/tracing) |

**Required Features:**
```toml
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
```

**Rationale:**
- rmcp uses `tracing` internally for debug/info/error logging
- The MCP server binary needs a subscriber to capture these logs
- `env-filter` enables `RUST_LOG` environment variable for log level control
- Logs MUST go to stderr (stdout is the MCP stdio transport)
- All rmcp examples use this exact setup

**Usage Pattern (from rmcp examples):**
```rust
tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env()
        .add_directive(tracing::Level::DEBUG.into()))
    .with_writer(std::io::stderr)
    .with_ansi(false)
    .init();
```

**Placement:** Only needed in `assay-cli` (the binary that runs the MCP server). Core and types crates do not need it.

### 4. tracing -- Structured Logging (Transitive, but Explicit)

| Attribute | Value |
|---|---|
| **Crate** | `tracing` |
| **Version** | 0.1.44 |
| **License** | MIT (in deny.toml allowlist) |

**Rationale for Explicit Dependency:**
While `tracing` comes transitively via rmcp, assay-core and assay-cli should depend on it explicitly if they emit their own log events (e.g., gate evaluation logging). For v0.1.0, this is optional -- only add if we instrument gate evaluation or config loading with `tracing::info!()` / `tracing::debug!()`.

**Recommendation:** Add to workspace deps but defer usage to implementation phase. The MCP server binary needs it minimally for the subscriber setup.

---

## Required Upgrade: schemars 0.8 -> 1.x

### The Problem

rmcp 0.17's `server` feature depends on `schemars = "1.0"`. The current workspace uses `schemars = "0.8"`. These are **semver-incompatible** -- the `JsonSchema` trait in schemars 0.8 is a different type from the one in schemars 1.x. Since rmcp's `#[tool]` macro requires tool parameter types to implement `schemars::JsonSchema` (from 1.x), and assay-types derives `JsonSchema` (from 0.8), they cannot interoperate.

### Migration Impact Assessment

**Current schemars 0.8 usage in assay-types (the only consumer):**
```rust
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Spec { ... }
// Same pattern for Gate, Review, Workflow, Config
```

This is exclusively `derive(JsonSchema)` -- no custom schema implementations, no `SchemaGenerator` usage, no `Visitor` transforms, no `RootSchema` references.

**Breaking changes that affect us:** None. The derive macro works identically in 1.x for basic struct/enum schemas.

**Breaking changes that do NOT affect us (but to be aware of):**
- `Schema` type changed from a struct with fields to a wrapper around `serde_json::Value`
- `Visitor` trait renamed to `Transform` -- we don't use it
- `RootSchema` removed -- we don't reference it
- Optional dependency feature names changed (e.g., `chrono` -> `chrono04`) -- we don't use optional features
- Import path changed from `schemars::schema::Schema` to `schemars::Schema` -- we only import `JsonSchema`

**Migration Steps:**
1. Change workspace `Cargo.toml`: `schemars = "0.8"` -> `schemars = "1"`
2. Run `cargo check` -- expect clean compilation
3. If the schema generation pipeline (future `just schemas`) uses `schema_for!()` or `SchemaGenerator`, update API calls per migration guide

**Risk:** Low. The upgrade is mechanical for our usage pattern.

**Migration Reference:** [Schemars Migration Guide](https://graham.cool/schemars/migrating/)

---

## Workspace Dependency Changes Summary

### New Additions to Root Cargo.toml `[workspace.dependencies]`

```toml
# MCP server
rmcp = { version = "0.17", features = ["server", "transport-io"] }

# TOML parsing
toml = "1"

# Logging (for MCP server binary)
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
```

### Modifications to Existing Workspace Dependencies

```toml
# UPGRADE: 0.8 -> 1 (required by rmcp server feature)
schemars = "1"
```

### Per-Crate Dependency Additions

| Crate | New Dependencies | Rationale |
|---|---|---|
| `assay-types` | (none new, schemars upgraded) | Types only need serde + schemars derives |
| `assay-core` | `toml` | Config and spec file parsing (free functions) |
| `assay-cli` | `rmcp`, `tokio`, `tracing`, `tracing-subscriber` | MCP server binary, `mcp serve` subcommand |

**Note on assay-cli and tokio:** The workspace already declares `tokio = { version = "1", features = ["full"] }`. The CLI crate should use `.workspace = true` for tokio. The `#[tokio::main]` is needed for the MCP server's async runtime.

### Not Adding (Considered and Rejected)

| Crate | Version | Why Considered | Why Rejected |
|---|---|---|---|
| `dirs` | 6.0.0 | XDG-compliant config directory discovery | v0.1.0 uses project-local `.assay/` directory, no user-global config needed yet. Add when needed. |
| `toml_edit` | 0.22.x | Round-trip TOML editing preserving comments | v0.1.0 only reads config/specs and writes fresh files via `toml::to_string_pretty()`. No need to preserve formatting. |
| `anyhow` | 1.x | Ergonomic error handling in binaries | Already using `thiserror` for typed errors + `color-eyre` in TUI. Mixing in `anyhow` adds confusion. Use `thiserror` in core, `color-eyre` or `Result<_, Box<dyn Error>>` in binaries. |
| `rust-mcp-sdk` | 0.x | Alternative MCP SDK with HTTP/SSE focus | Not the official SDK. rmcp is canonical, hosted under modelcontextprotocol org. |
| `async-mcp` | 0.x | Lightweight alternative MCP implementation | Less mature, fewer features, not official. |
| `miette` | latest | Rich error diagnostics | color-eyre already fills this role for user-facing errors. |

---

## Claude Code Plugin Structure

### .mcp.json Format (Project Scope)

The Claude Code plugin needs a `.mcp.json` file at the plugin root that configures the MCP server. For the assay plugin, this will be a stdio transport pointing to the compiled binary.

**Canonical format (from Claude Code docs):**
```json
{
  "mcpServers": {
    "assay": {
      "command": "${CLAUDE_PLUGIN_ROOT}/../../target/release/assay-cli",
      "args": ["mcp", "serve"],
      "env": {}
    }
  }
}
```

**Key considerations:**
- `${CLAUDE_PLUGIN_ROOT}` expands to the plugin directory at runtime
- The binary path must resolve to the compiled `assay-cli` binary
- `command` + `args` form the full invocation: `assay-cli mcp serve`
- `env` can pass configuration (e.g., `RUST_LOG` for debug logging)
- For development, the path can use `cargo run` instead of release binary

**Plugin discovery:**
- Plugins define MCP servers in `.mcp.json` at the plugin root OR inline in `plugin.json`
- When a plugin is enabled, its MCP servers start automatically
- Plugin servers appear alongside manually configured MCP tools
- Claude Code prompts for approval before using project-scoped MCP servers

**Existing plugin.json (already in repo):**
```json
{
  "name": "assay",
  "version": "0.1.0",
  "description": "Assay plugin for Claude Code -- spec-driven workflows with gated quality checks",
  "author": "wollax"
}
```

**v0.1.0 Plugin Deliverables:**
1. `.mcp.json` at `plugins/claude-code/` with stdio server config
2. Updated `plugin.json` with inline `mcpServers` (alternative approach)
3. CLAUDE.md snippet documenting available MCP tools for the agent

### Plugin-to-Binary Path Strategy

Two viable approaches for v0.1.0:

| Approach | .mcp.json command | Pros | Cons |
|---|---|---|---|
| **Installed binary** | `assay-cli` | Clean, no path assumptions | Requires `cargo install` or PATH setup |
| **Relative to plugin** | `${CLAUDE_PLUGIN_ROOT}/../../target/release/assay-cli` | Works from repo clone | Fragile path, requires pre-built binary |

**Recommendation:** Use the installed binary approach (`"command": "assay-cli"`) with documentation noting the user must have the binary in PATH. This is how most MCP stdio servers work (e.g., `npx -y @package`).

---

## Integration Considerations

### schemars Version Alignment

The upgrade to schemars 1.x means rmcp's `#[tool]` macro and assay-types both use the **same** `JsonSchema` trait. This enables a clean pattern where MCP tool parameter types can be defined in assay-types and shared with the MCP server:

```rust
// In assay-types (derives schemars 1.x JsonSchema)
#[derive(Deserialize, JsonSchema)]
pub struct SpecGetRequest {
    pub name: String,
}

// In assay-cli MCP server (rmcp uses schemars 1.x internally)
#[tool(description = "Get a spec by name")]
fn spec_get(&self, params: Parameters<SpecGetRequest>) -> String { ... }
```

This is a significant architectural benefit of the upgrade.

### tokio Runtime Sharing

Both rmcp and the existing workspace use tokio 1.x. The MCP server in assay-cli uses `#[tokio::main]` which starts a multi-threaded runtime. Since gate evaluation is sync (`std::process::Command`), it should be wrapped in `tokio::task::spawn_blocking()` when called from MCP tool handlers to avoid blocking the async runtime.

### Error Type Bridge

rmcp tools return `Result<_, McpError>` (rmcp's error type) or `Result<_, String>`. The assay-core error types (AssayError via thiserror) need a conversion path:

```rust
// Option A: impl From<AssayError> for McpError
// Option B: .map_err(|e| McpError::internal(e.to_string()))
```

Option B is simpler for v0.1.0 and avoids coupling assay-core to rmcp types.

### Logging Architecture

| Component | Logging Approach |
|---|---|
| assay-core | `tracing` macros (optional, can defer) |
| assay-cli (normal mode) | stdout/stderr printing (existing) |
| assay-cli (MCP mode) | `tracing-subscriber` to stderr, stdout reserved for MCP transport |
| assay-tui | `color-eyre` for errors (existing) |

The MCP server MUST NOT write to stdout except via the MCP protocol. All diagnostic output goes to stderr via tracing.

### License Compatibility

All new dependencies use licenses already in `deny.toml`:

| Dependency | License | In Allowlist |
|---|---|---|
| rmcp | Apache-2.0 | Yes |
| toml | MIT OR Apache-2.0 | Yes (both) |
| tracing | MIT | Yes |
| tracing-subscriber | MIT | Yes |
| schemars 1.x | MIT | Yes |
| futures (transitive) | MIT OR Apache-2.0 | Yes |
| chrono (transitive) | MIT OR Apache-2.0 | Yes |
| async-trait (transitive) | MIT OR Apache-2.0 | Yes |

No new license additions needed in `deny.toml`.

### cargo-deny Impact

The schemars 0.8 -> 1.x upgrade may trigger a `multiple-versions` warning if any transitive dependency still pulls in schemars 0.8. Since the workspace is small and only assay-types uses schemars directly, this is unlikely. Verify with `just deny` after the upgrade.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| rmcp 0.18 breaks API | Medium | Medium | Pin to `0.17`, monitor changelog, keep MCP server code isolated in assay-cli |
| schemars 1.x migration breaks schema generation | Low | Low | Only derive macros used; migration is mechanical |
| tokio version conflicts | Very Low | High | Both rmcp and workspace use tokio 1.x with compatible feature sets |
| MCP protocol spec changes | Low | Medium | rmcp tracks spec; update SDK when needed |
| Binary distribution for plugin | Medium | Low | Document `cargo install` path; consider `cargo-binstall` for future |

---

## Recommended Dependency Version Pins

```toml
[workspace.dependencies]
# Existing (unchanged)
assay-types = { path = "crates/assay-types" }
assay-core = { path = "crates/assay-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
ratatui = "0.30"
crossterm = "0.28"
thiserror = "2"
color-eyre = "0.6"

# Upgraded
schemars = "1"

# New
rmcp = { version = "0.17", features = ["server", "transport-io"] }
toml = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
```

---

## Sources

- [rmcp crate on crates.io](https://crates.io/crates/rmcp) -- v0.17.0
- [rmcp GitHub repository](https://github.com/modelcontextprotocol/rust-sdk) -- official MCP Rust SDK
- [rmcp API documentation](https://docs.rs/rmcp/latest/rmcp/) -- ServerHandler, tool macros, stdio transport
- [rmcp v0.17.0 release notes](https://github.com/modelcontextprotocol/rust-sdk/releases/tag/rmcp-v0.17.0) -- released 2026-02-27
- [toml crate on crates.io](https://crates.io/crates/toml) -- v1.0.3+spec-1.1.0
- [toml GitHub repository](https://github.com/toml-rs/toml) -- Cargo.toml verified
- [schemars migration guide](https://graham.cool/schemars/migrating/) -- 0.8 to 1.0 breaking changes
- [schemars changelog](https://github.com/GREsau/schemars/blob/master/CHANGELOG.md)
- [Claude Code MCP documentation](https://code.claude.com/docs/en/mcp) -- .mcp.json format, plugin MCP servers, scopes
- [tracing documentation](https://docs.rs/tracing/latest/tracing/) -- v0.1.44
- [tracing-subscriber documentation](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/) -- v0.3.22
- [dirs crate](https://docs.rs/dirs/latest/dirs/) -- v6.0.0 (evaluated, deferred)

---
*Stack research completed: 2026-02-28*
