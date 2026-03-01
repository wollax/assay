# Architecture Research

**Date:** 2026-02-28
**Scope:** v0.1.0 vertical slice -- how MCP servers, spec parsers, and gate evaluators integrate with the existing 4-crate Rust workspace.

---

## Existing Architecture Baseline

### Current Workspace

```
assay-cli ──> assay-core ──> assay-types
assay-tui ──> assay-core ──> assay-types
```

- **assay-types** (pub DTOs): `Spec`, `Gate`, `Review`, `Workflow`, `Config` with `serde` + `schemars` derives. Zero logic.
- **assay-core** (domain logic): 5 modules (`spec`, `gate`, `review`, `workflow`, `config`) as empty stubs with doc comments. Depends on `assay-types` + `thiserror`.
- **assay-cli** (clap binary): Skeleton `Cli` struct, prints version. Depends on `assay-core` + `clap`.
- **assay-tui** (ratatui binary): Skeleton event loop. Depends on `assay-core` + `ratatui` + `crossterm` + `color-eyre`.

All modules are empty stubs -- v0.1.0 is writing the first real code into these modules.

### Current Workspace Dependencies

```toml
# Root Cargo.toml [workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "0.8"    # NOTE: version conflict with rmcp -- see finding below
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
thiserror = "2"
```

---

## New Components for v0.1.0

### Component Inventory

| Component | Type | Location | New Crate? | New Dependencies |
|-----------|------|----------|-----------|-----------------|
| Error types | Module | `assay-core::error` | No | None (thiserror already present) |
| Domain model hardening | Types redesign | `assay-types::lib.rs` | No | None |
| Config loading | Module impl | `assay-core::config` | No | `toml` |
| Spec parsing | Module impl | `assay-core::spec` | No | None (toml reused from config) |
| Gate evaluation | Module impl | `assay-core::gate` | No | None (std::process::Command) |
| MCP server | New crate | `crates/assay-mcp` | **Yes** | `rmcp`, `tracing`, `tracing-subscriber` |
| CLI subcommands | Module expansion | `assay-cli::main.rs` | No | `color-eyre` (for consistent error display) |
| Claude Code plugin | Static config files | `plugins/claude-code/` | No | None (JSON config) |
| Schema generation | Example binary | `assay-types/examples/` | No | None (schemars already present) |

---

## Critical Finding: schemars Version Conflict

### The Problem

The workspace currently depends on `schemars = "0.8"`. The `rmcp` crate (v0.17.0) requires `schemars = "^1.0"` as an optional dependency (enabled by the `schemars` feature flag). These are **incompatible major versions** -- Cargo will resolve them as two separate crates, but the `JsonSchema` trait from schemars 0.8 is a different trait than the one from schemars 1.0. The rmcp `#[tool]` macro uses schemars 1.0's `JsonSchema` derive, while assay-types uses schemars 0.8's.

### Impact

This means **assay-types structs cannot be directly used as rmcp tool parameter types** without upgrading schemars. The `Parameters<T>` wrapper in rmcp requires `T: schemars::JsonSchema` where `schemars` is version 1.0.

### Resolution Options

1. **Upgrade workspace to schemars 1.0** (recommended). The migration from 0.8 to 1.0 is mostly mechanical: derive macros work the same way, `schema_for!()` returns `Schema` instead of `RootSchema`, and the `schemars::gen` module moves to `schemars::generate`. Since the codebase has zero consumers of schemars APIs beyond derive macros, this is low-risk. The `#[serde(tag = "type")]` pattern used by `GateKind` is supported in schemars 1.0.

2. **Define separate MCP-specific parameter types** in assay-mcp that derive schemars 1.0's `JsonSchema`, and convert to/from assay-types. Adds boilerplate and defeats the purpose of shared types.

3. **Don't use rmcp's schemars feature**. Define tool schemas manually. Loses the benefit of derive-based schema generation.

**Recommendation:** Option 1. Upgrade schemars to 1.0 across the workspace as a prerequisite task. The migration is straightforward given zero existing schemars API usage beyond derives.

### Migration Checklist for schemars 0.8 -> 1.0

- Change `schemars = "0.8"` to `schemars = "1.0"` in root `Cargo.toml`
- `schemars::schema::RootSchema` -> `schemars::Schema` (if used anywhere beyond derives)
- `schemars::gen` -> `schemars::generate` (Rust 2024 edition reserves `gen` as keyword)
- `schema_for!()` now returns `Schema` instead of `RootSchema`
- Re-run `just ready` to verify no breakage

---

## Architecture Decision: Where Does the MCP Server Live?

### Decision: New crate `assay-mcp`

The MCP server requires its own binary crate. The rationale:

1. **Separate binary target.** The MCP server is a long-running stdio process (`assay mcp serve`), architecturally equivalent to `assay-cli` and `assay-tui` -- it is another "surface" over assay-core. It needs its own `main()`.

2. **Dependency isolation.** rmcp + tokio runtime + tracing are MCP-specific dependencies. Adding them to assay-cli would bloat a currently-sync binary with async runtime overhead. Adding them to assay-core would violate the "core has no transport/presentation concerns" principle.

3. **Consistent with existing pattern.** The workspace already separates surfaces: `assay-cli` for humans via CLI, `assay-tui` for humans via TUI, `assay-mcp` for agents via MCP. Each depends on `assay-core`.

4. **CLI delegates to MCP binary.** The `assay mcp serve` subcommand in assay-cli can `exec` into the assay-mcp binary, or assay-mcp can be a library crate with a `run()` function called from CLI's `main()`. The latter is simpler for v0.1.

### Alternative Considered: Module in assay-core

Rejected. The MCP server handler struct (`AssayServer`) implements rmcp's `ServerHandler` trait, which requires `tokio` async runtime and transport dependencies. These are presentation-layer concerns, not domain logic. Putting them in assay-core violates the layering.

### Alternative Considered: Module in assay-cli

Partially viable. The `assay mcp serve` subcommand could inline the MCP server code. However, this means assay-cli depends on rmcp + tokio, and `cargo build -p assay-cli` always compiles MCP dependencies even for users who only want CLI commands. A feature flag could mitigate this, but a separate crate is cleaner.

### Revised Dependency Graph

```
assay-cli ──> assay-core ──> assay-types
assay-tui ──> assay-core ──> assay-types
assay-mcp ──> assay-core ──> assay-types
```

CLI invokes MCP via one of:
- **Option A:** `assay-cli` depends on `assay-mcp` as a library, calls `assay_mcp::serve()` from the `mcp serve` subcommand. Produces a single `assay` binary.
- **Option B:** `assay-mcp` is a standalone binary. `assay mcp serve` execs into it. Two binaries.

**Recommendation:** Option A for v0.1. Single binary is simpler for distribution and plugin configuration (`.mcp.json` points to `assay mcp serve`). The `assay-mcp` crate exposes a library function, and the CLI calls it.

```
assay-cli ──> assay-mcp ──> assay-core ──> assay-types
assay-tui ──────────────> assay-core ──> assay-types
```

### assay-mcp Crate Structure

```
crates/assay-mcp/
  Cargo.toml
  src/
    lib.rs          # pub async fn serve() -> Result<()>, module declarations
    server.rs       # AssayServer struct + #[tool_router] impl + ServerHandler impl
```

The crate is intentionally small -- ~200-300 lines. It:
1. Defines `AssayServer` struct with a `ToolRouter<Self>` field
2. Implements tools via `#[tool_router]` macro
3. Implements `ServerHandler` trait with `get_info()`
4. Exports a `pub async fn serve() -> Result<()>` that wires stdio transport

---

## MCP Server Architecture Pattern (rmcp)

### Verified Pattern from rmcp v0.17.0

The rmcp SDK uses a macro-driven approach for MCP server implementation. Based on the official examples in the `modelcontextprotocol/rust-sdk` repository:

```rust
// 1. Define server struct with ToolRouter field
#[derive(Clone)]
struct AssayServer {
    tool_router: ToolRouter<Self>,
}

// 2. Implement tools via #[tool_router] macro
#[tool_router]
impl AssayServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(name = "spec_get", description = "Get a spec by name")]
    fn spec_get(
        &self,
        Parameters(req): Parameters<SpecGetRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Delegate to assay_core::spec functions
    }

    #[tool(name = "gate_run", description = "Run gate criteria for a spec")]
    fn gate_run(
        &self,
        Parameters(req): Parameters<GateRunRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Delegate to assay_core::gate functions
    }
}

// 3. Implement ServerHandler trait
#[tool_handler]
impl ServerHandler for AssayServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Assay spec-driven development server".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// 4. Main entry point with stdio transport
pub async fn serve() -> Result<()> {
    let service = AssayServer::new()
        .serve(rmcp::transport::stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}
```

### Key rmcp Patterns

| Pattern | Detail |
|---------|--------|
| `#[tool_router]` on impl block | Generates tool registration. Must have a `tool_router: ToolRouter<Self>` field on the struct. |
| `#[tool(name = "...", description = "...")]` on methods | Registers individual tools. Method returns `Result<CallToolResult, McpError>` or simpler types like `String`. |
| `Parameters<T>` wrapper | Deserializes JSON tool arguments into typed struct `T`. Requires `T: Deserialize + JsonSchema`. |
| `#[tool_handler]` on `ServerHandler` impl | Wires the tool router's `list_tools` and `call_tool` into the handler. |
| `ServerHandler::get_info()` | Returns server metadata: name, version, capabilities, instructions. |
| `stdio()` transport | Creates `(tokio::io::Stdin, tokio::io::Stdout)` pair. Feature: `transport-io`. |
| `.serve(transport).await` | Starts the MCP server, performs capability negotiation. |
| `service.waiting().await` | Blocks until the client disconnects. |
| Tracing to stderr | MCP uses stdout for JSON-RPC. All logging **must** go to stderr. |

### Required rmcp Features

```toml
rmcp = { version = "0.17", features = ["server", "transport-io", "schemars"] }
```

- `server` -- enables `ServerHandler`, `ToolRouter`, `#[tool_router]`, `#[tool]`, `#[tool_handler]`
- `transport-io` -- enables `rmcp::transport::stdio()`
- `schemars` -- enables re-export of schemars 1.0 and schema generation from `Parameters<T>`
- `macros` -- enabled by default, provides proc macros

---

## Data Flow: How Core Logic is Shared

### Spec Loading Flow

```
                    assay-cli                          assay-mcp
                        |                                  |
              assay gate run <spec>              MCP tool: gate/run {spec}
                        |                                  |
                        v                                  v
                  assay_core::config::load(".assay/config.toml")
                        |
                        v
                  assay_core::spec::load_spec(spec_dir, name)
                        |
                        v
              Parse TOML frontmatter (+++ delimited)
                        |
                        v
                  assay_types::Spec { name, description, criteria }
```

Both CLI and MCP call the same `assay-core` free functions. The MCP server holds a reference to the project root (working directory) and passes it to core functions, just like the CLI resolves it from `cwd` or `--config` flag.

### Gate Evaluation Flow

```
  CLI: assay gate run <spec> [criterion]
  MCP: gate/run { spec: "name", criterion?: "name" }
            |
            v
    assay_core::spec::load_spec(spec_dir, spec_name)
            |
            v
    For each criterion (or single if specified):
            |
            v
    assay_core::gate::evaluate(gate, working_dir)
            |
            +---> std::process::Command::new("sh").arg("-c").arg(cmd)
            |            .current_dir(working_dir)
            |            .output()
            |
            v
    assay_types::GateResult { status, evidence: { stdout, stderr, exit_code }, duration_ms, timestamp }
            |
            v
    CLI: format and print to terminal
    MCP: serialize as CallToolResult content (JSON text)
```

### Sync/Async Boundary

Gate evaluation is **synchronous** (`std::process::Command`). The MCP server runs on a tokio async runtime. The integration point:

```rust
// In assay-mcp server.rs, within a #[tool] method:
#[tool(name = "gate_run", description = "Run gate criteria for a spec")]
async fn gate_run(&self, Parameters(req): Parameters<GateRunRequest>) -> Result<CallToolResult, McpError> {
    // Bridge sync core function into async context
    let result = tokio::task::spawn_blocking(move || {
        assay_core::gate::evaluate(&gate, &working_dir)
    }).await
      .map_err(|e| McpError::internal_error(e.to_string(), None))?
      .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(CallToolResult::success(vec![
        Content::text(serde_json::to_string_pretty(&result).unwrap())
    ]))
}
```

`tokio::task::spawn_blocking` is the documented pattern for calling sync blocking functions from async rmcp tool handlers. This avoids blocking the tokio runtime that serves the MCP protocol.

---

## MCP Server Working Directory

### The Problem

The MCP server is launched as `assay mcp serve` from a project directory. It needs to know:
1. Where `.assay/config.toml` lives (config root)
2. Where `.assay/specs/` lives (spec directory)
3. What `working_dir` to pass to `gate::evaluate()` for command execution

### Design

The `AssayServer` struct holds the project root path, determined at startup:

```rust
#[derive(Clone)]
struct AssayServer {
    project_root: PathBuf,  // Resolved at startup from cwd
    tool_router: ToolRouter<Self>,
}
```

- **Config root:** `{project_root}/.assay/config.toml`
- **Spec directory:** `{project_root}/.assay/specs/`
- **Gate working_dir:** `{project_root}` (commands run relative to project root, matching CLI behavior)

This is set once at server construction. The MCP server does not support switching projects mid-session.

---

## Spec File Parsing Architecture

### File Format

```
+++
name = "add-auth-flow"
description = "Implement JWT authentication"

[[criteria]]
description = "All tests pass"
cmd = "cargo test"

[[criteria]]
description = "No clippy warnings"
cmd = "cargo clippy -- -D warnings"
+++
```

### Where Parsing Lives

- **File I/O:** `assay_core::spec::load_spec(spec_dir: &Path, name: &str) -> Result<Spec>`
- **String parsing:** `assay_core::spec::parse_spec(content: &str) -> Result<Spec>`
- **Validation:** `assay_core::spec::validate(spec: &Spec) -> Result<()>`

The `parse_spec` function handles `+++` delimiter splitting and TOML parsing of the frontmatter. The `toml` crate (already needed for config loading) handles deserialization.

### Types Involved

```rust
// assay-types
pub struct Spec {
    pub name: String,
    pub description: String,
    pub criteria: Vec<Criterion>,
}

pub struct Criterion {
    pub description: String,
    pub cmd: Option<String>,  // Optional for forward-compat with agent-evaluated criteria
}
```

Both CLI and MCP consume `Spec` identically. The MCP tool `spec/get` serializes it to JSON via serde; the CLI formats it for terminal display.

---

## Plugin Architecture

### Claude Code Plugin Structure

```
plugins/claude-code/
  .claude-plugin/
    plugin.json         # Existing: name, version, description
  .mcp.json             # NEW: MCP server registration
  CLAUDE.md             # NEW: Workflow instructions snippet
  hooks/
    hooks.json          # Existing: empty hooks array
  commands/             # Existing: empty
  agents/               # Existing: empty
  skills/               # Existing: empty
```

### .mcp.json Format

```json
{
  "mcpServers": {
    "assay": {
      "type": "stdio",
      "command": "assay",
      "args": ["mcp", "serve"]
    }
  }
}
```

This tells Claude Code to launch `assay mcp serve` as a subprocess and communicate over stdio JSON-RPC. The `assay` binary must be on the user's `$PATH` (installed via `cargo install` or local build).

### CLAUDE.md Snippet

A markdown file providing workflow guidance to Claude Code agents:

```markdown
## Assay Integration

This project uses Assay for spec-driven development with quality gates.

**Workflow:**
1. Read the active spec via the `spec_get` MCP tool before starting work
2. Implement against the spec's criteria
3. Run `gate_run` MCP tool to verify all criteria pass
4. Address any failing gates before considering work complete
```

### Plugin Does NOT Include

- No skills (MCP tools replace them)
- No hooks (no lifecycle events in v0.1)
- No agents (the plugin provides tools, not agent definitions)
- No commands (MCP tools are the command surface)

---

## Build Order and Dependency Integration

### New Workspace Dependencies

```toml
# Added to root Cargo.toml [workspace.dependencies]
schemars = "1.0"          # UPGRADED from 0.8
toml = "0.8"              # For config/spec TOML parsing
rmcp = { version = "0.17", features = ["server", "transport-io", "schemars"] }
tracing = "0.1"           # For MCP server logging (to stderr)
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### New Crate Dependencies

```toml
# crates/assay-core/Cargo.toml
[dependencies]
assay-types.workspace = true
thiserror.workspace = true
toml.workspace = true           # NEW: config + spec parsing

# crates/assay-mcp/Cargo.toml   (NEW CRATE)
[package]
name = "assay-mcp"
version.workspace = true
edition.workspace = true

[dependencies]
assay-core.workspace = true
assay-types.workspace = true
rmcp.workspace = true
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true

# crates/assay-cli/Cargo.toml
[dependencies]
assay-core.workspace = true
assay-mcp.workspace = true      # NEW: for `assay mcp serve` subcommand
clap.workspace = true
color-eyre.workspace = true     # NEW: consistent error display
tokio.workspace = true          # NEW: needed to run async assay_mcp::serve()
```

### Build Order (by dependency chain)

```
1. assay-types          (schemars 1.0 upgrade, domain model hardening)
2. assay-core           (error types, config, spec, gate modules)
3. assay-mcp            (MCP server, depends on core)
4. assay-cli            (CLI subcommands, depends on core + mcp)
5. assay-tui            (no changes in v0.1, but must still compile)
6. plugins/claude-code  (static config files, no Rust compilation)
```

### Workspace Member Registration

```toml
# Root Cargo.toml
[workspace]
resolver = "2"
members = ["crates/*"]   # Automatically picks up crates/assay-mcp
```

No change needed -- the glob pattern `crates/*` already covers new crates.

---

## Integration Point Summary

### Modified Components

| Component | File(s) | Change |
|-----------|---------|--------|
| Workspace root | `Cargo.toml` | Add `toml`, `rmcp`, `tracing`, `tracing-subscriber` to workspace deps; upgrade `schemars` to 1.0 |
| assay-types | `src/lib.rs` | Redesign types: new `GateKind`, `GateResult`, `Criterion`; remove `passed: bool` from Gate |
| assay-core | `src/lib.rs` | Add `pub mod error;` export |
| assay-core | `src/error.rs` | NEW file: `AssayError` enum with `thiserror` |
| assay-core | `src/config/mod.rs` | Implement `load()`, `from_str()`, `validate()` |
| assay-core | `src/spec/mod.rs` | Implement `load_spec()`, `parse_spec()`, `validate()` |
| assay-core | `src/gate/mod.rs` | Implement `evaluate()` |
| assay-core | `Cargo.toml` | Add `toml` dependency |
| assay-cli | `src/main.rs` | Add subcommands: `init`, `spec show`, `gate run`, `mcp serve` |
| assay-cli | `Cargo.toml` | Add `assay-mcp`, `color-eyre`, `tokio` dependencies |
| justfile | `justfile` | Add `just mcp` recipe, possibly `just schemas` |

### New Components

| Component | Location | Purpose |
|-----------|----------|---------|
| assay-mcp crate | `crates/assay-mcp/` | MCP server binary/library |
| assay-mcp lib | `crates/assay-mcp/src/lib.rs` | Public `serve()` function |
| assay-mcp server | `crates/assay-mcp/src/server.rs` | `AssayServer` + tool implementations |
| Plugin .mcp.json | `plugins/claude-code/.mcp.json` | MCP server registration for Claude Code |
| Plugin CLAUDE.md | `plugins/claude-code/CLAUDE.md` | Workflow instructions for agents |

### Unchanged Components

| Component | Why Unchanged |
|-----------|---------------|
| assay-tui | No TUI features in v0.1 scope. Must still compile after types redesign. |
| assay-core review/ | Reviews not in v0.1 scope. Module stays as empty stub. |
| assay-core workflow/ | Workflows not in v0.1 scope. Module stays as empty stub. |
| plugins/codex/ | Codex plugin not in v0.1 scope. |
| plugins/opencode/ | OpenCode plugin not in v0.1 scope. |
| schemas/ | Schema generation is a stretch goal. Directory exists for future use. |
| deny.toml | May need updates if new deps use licenses not in allowlist -- verify after adding rmcp. |

---

## Risk Assessment

### schemars 1.0 Migration (Medium Risk)

The upgrade is mechanical but touches every type in assay-types. All derives should continue to work. The risk is that schemars 1.0 generates different JSON Schema output for `#[serde(tag = "type")]` enums, which could affect schema validation if schemas are consumed externally. Mitigated by: running `just ready` after upgrade, spot-checking generated schemas.

### rmcp Stability (Medium Risk)

rmcp is at v0.17.0 -- pre-1.0. API breakage is possible. The MCP spike (days 1-2) gates this risk: build a hardcoded 1-tool server, verify it works with Claude Code, then proceed. Fallback: CLI-only v0.1, MCP deferred to v0.2.

### Tokio Runtime in CLI (Low Risk)

Adding tokio to assay-cli for the `mcp serve` subcommand means the CLI binary includes an async runtime even for sync commands like `assay init`. The overhead is compile-time only -- tokio is not initialized unless `mcp serve` is invoked. If binary size is a concern, a `#[cfg(feature = "mcp")]` feature flag could gate the dependency, but this is over-engineering for v0.1.

### cargo-deny License Check (Low Risk)

rmcp and its transitive dependencies may include licenses not in the current allowlist. Run `just deny` after adding rmcp to verify. The allowlist already covers MIT, Apache-2.0, BSD variants, and ISC, which covers most Rust ecosystem crates.

---

## Quality Gate Checklist

- [x] Integration points clearly identified (modified vs new vs unchanged components)
- [x] New vs modified components explicit (tables above)
- [x] Build order considers existing dependencies (assay-types first, then core, then mcp, then cli)
- [x] MCP server architecture pattern verified (rmcp v0.17.0 examples from modelcontextprotocol/rust-sdk)
- [x] schemars version conflict identified and resolution recommended
- [x] Sync/async boundary documented (spawn_blocking for gate evaluation)
- [x] Plugin connection to MCP server documented (.mcp.json format)
- [x] Data flow through all layers traced (spec loading, gate evaluation)

---

*Research completed: 2026-02-28*
