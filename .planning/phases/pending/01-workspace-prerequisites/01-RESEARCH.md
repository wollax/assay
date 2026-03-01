# Phase 1: Workspace Prerequisites - Research

**Researched:** 2026-02-28
**Domain:** Rust workspace dependency management, schemars migration, MCP server scaffolding
**Confidence:** HIGH

## Summary

This phase has two concrete deliverables: upgrading schemars from 0.8 to 1.x and scaffolding a new `assay-mcp` library crate with rmcp as its primary dependency. Both are well-understood infrastructure tasks with low ambiguity.

The schemars 0.8 to 1.x upgrade is straightforward for this project because the codebase uses only basic `#[derive(JsonSchema)]` on simple structs with `String`, `bool`, and `Vec<T>` fields. No manual schema construction, no `RootSchema` usage, no visitors/transforms, no validation attributes. The derive macro syntax is unchanged between versions.

The rmcp crate (v0.17.0) is the official Rust SDK for the Model Context Protocol. Its `server` feature automatically pulls in schemars 1.x. The crate provides proc macros (`#[tool_router]`, `#[tool]`) for implementing MCP servers with minimal boilerplate.

**Primary recommendation:** Pin schemars to `=1.0.0` (the exact version rmcp was built against), add rmcp `0.17` with `server` and `transport-io` features, and scaffold `assay-mcp` as a thin library crate that depends on `assay-core` and `rmcp`.

## Standard Stack

The established libraries/tools for this domain:

### Core

| Library  | Version | Purpose                  | Why Standard                                                        |
| -------- | ------- | ------------------------ | ------------------------------------------------------------------- |
| schemars | =1.0.0  | JSON Schema derive macro | Required by rmcp; pinned to exact version rmcp specifies in its dep |
| rmcp     | 0.17    | MCP server SDK           | Official Rust SDK for Model Context Protocol (modelcontextprotocol) |

### Supporting

| Library    | Version | Purpose                     | When to Use                            |
| ---------- | ------- | --------------------------- | -------------------------------------- |
| tokio      | 1       | Async runtime               | Already workspace dep; rmcp requires it |
| tracing    | 0.1     | Structured logging          | Future phases; rmcp uses it internally  |
| futures    | 0.3     | Async utilities             | Pulled transitively by rmcp            |
| serde      | 1       | Serialization               | Already workspace dep                  |
| serde_json | 1       | JSON serialization          | Already workspace dep                  |

### Alternatives Considered

| Instead of | Could Use    | Tradeoff                                                                                    |
| ---------- | ------------ | ------------------------------------------------------------------------------------------- |
| rmcp       | mcp-protocol-sdk | rmcp is the official SDK (modelcontextprotocol org); no reason to use unofficial alternatives |
| schemars 1.0.0 | schemars 1.2.1 (latest) | Pinning to 1.0.0 matches rmcp's `"1.0"` dep exactly; upgrading is safe but unnecessary risk for this phase |

**Version pinning rationale:**

rmcp's Cargo.toml specifies `schemars = { version = "1.0", ... }` which is semver-compatible caret range (>=1.0.0, <2.0.0). The CONTEXT.md decision says "pin to exact version rmcp requires." The safest exact pin is `=1.0.0` since that's the base version rmcp declares. However, since rmcp's caret range accepts any 1.x, pinning to `"1"` (caret, accepting 1.0.0 through 1.x) is equally valid and more practical. **Recommendation:** Use `"1"` (caret range matching rmcp's own declaration) rather than an exact pin, because:
1. rmcp itself uses `"1.0"` caret, not `=1.0.0`
2. schemars 1.x follows semver; patch/minor bumps are safe
3. Exact pinning creates maintenance burden for no safety gain

If the user strongly prefers exact pinning per CONTEXT.md, use `"1.0.0"` (which cargo resolves as `>=1.0.0, <2.0.0` anyway — to get true exact, use `=1.0.0`).

## Architecture Patterns

### Recommended Workspace Layout After Phase 1

```
crates/
├── assay-types/    # Shared DTOs (serde, schemars)
├── assay-core/     # Domain logic
├── assay-cli/      # CLI binary
├── assay-tui/      # TUI binary
└── assay-mcp/      # MCP server library (NEW)
```

### Dependency Graph After Phase 1

```
assay-cli ──→ assay-core ──→ assay-types
assay-tui ──→ assay-core ──→ assay-types
assay-mcp ──→ assay-core ──→ assay-types
```

**Recommendation: assay-mcp depends on assay-core (not just assay-types).**

Rationale: The brainstorm summary establishes that MCP tools like `spec/get` and `gate/run` delegate to core business logic. assay-mcp needs assay-core to call those functions. Depending only on assay-types would force duplicating or re-routing logic.

### Pattern 1: Workspace Dependency Declaration

**What:** All dependencies declared in root `Cargo.toml` `[workspace.dependencies]`, referenced via `.workspace = true` in crate-level Cargo.toml files.

**When to use:** Always (project convention from CLAUDE.md).

**Example:**
```toml
# Root Cargo.toml
[workspace.dependencies]
rmcp = { version = "0.17", features = ["server", "transport-io"] }
schemars = "1"

# crates/assay-mcp/Cargo.toml
[dependencies]
rmcp.workspace = true
schemars.workspace = true
assay-core.workspace = true
```

### Pattern 2: Minimal Library Crate Scaffold

**What:** Create `assay-mcp` as a library crate with just `lib.rs` and its `Cargo.toml`. No module stubs until business logic arrives in later phases.

**When to use:** Phase 1 scaffold — keep it minimal so `cargo check -p assay-mcp` passes.

**Example:**
```rust
// crates/assay-mcp/src/lib.rs

//! MCP server library for Assay.
//!
//! Provides Model Context Protocol server implementation
//! that exposes Assay's spec and gate functionality to AI agents.
```

### Anti-Patterns to Avoid

- **Premature module stubs:** Don't create `server.rs`, `tools.rs`, etc. in Phase 1. Empty modules with `todo!()` macros add noise and risk failing `just lint`.
- **Re-exporting rmcp types from assay-mcp:** Keep rmcp as an implementation detail. If downstream crates need MCP types, that's a Phase 2+ decision.
- **Adding rmcp to assay-types:** The types crate must stay dependency-light. MCP server concerns belong in assay-mcp.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem                  | Don't Build            | Use Instead       | Why                                         |
| ------------------------ | ---------------------- | ----------------- | ------------------------------------------- |
| MCP protocol handling    | Custom JSON-RPC        | rmcp              | Protocol spec is complex; rmcp handles it   |
| JSON Schema generation   | Manual schema building | schemars derive   | Derive macro handles all standard Rust types |
| Async stdio transport    | Custom stdin/stdout    | rmcp transport-io | Handles framing, buffering, tokio integration |

**Key insight:** This phase is pure infrastructure — there is literally nothing to hand-roll. The entire value is in correctly wiring existing crates.

## Common Pitfalls

### Pitfall 1: schemars 0.8/1.x Version Conflict

**What goes wrong:** Having both schemars 0.8 and 1.x in the dependency tree causes compilation errors because `JsonSchema` trait is different between versions.

**Why it happens:** If any workspace crate still references schemars 0.8 (directly or transitively) while rmcp pulls in 1.x.

**How to avoid:** Change the workspace-level `schemars` dependency from `"0.8"` to `"1"` in a single atomic commit. All crates in the workspace inherit from workspace deps, so they all move together.

**Warning signs:** `cargo check` errors about conflicting `JsonSchema` trait implementations or "expected schemars::Schema, found schemars::Schema" (two different versions).

### Pitfall 2: schemars Module Path Changes

**What goes wrong:** Code that imports from `schemars::schema::Schema` or uses `RootSchema` fails to compile.

**Why it happens:** schemars 1.x removed the `schemars::schema` module entirely. `Schema` is now at `schemars::Schema`. `RootSchema` no longer exists.

**How to avoid:** Check all `use schemars::` imports in the codebase. The current code uses `use schemars::JsonSchema` which is unchanged between 0.8 and 1.x — so this project is NOT affected.

**Warning signs:** Compilation errors mentioning `schemars::schema` module not found.

### Pitfall 3: dyn-clone Removal in schemars 1.x

**What goes wrong:** Code that depends on `Schema` being `Clone` via `dyn-clone` may fail.

**Why it happens:** schemars 1.x dropped the `dyn-clone` dependency. The `Schema` type changed from a struct to a wrapper around `serde_json::Value`.

**How to avoid:** This project does NOT manually construct or clone Schema objects — only uses `#[derive(JsonSchema)]`. No impact expected.

**Warning signs:** Compilation errors about `Clone` not implemented for schema types.

### Pitfall 4: cargo-deny License Failures

**What goes wrong:** `just deny` fails after adding rmcp because new transitive dependencies introduce licenses not in the allow list.

**Why it happens:** rmcp brings in transitive deps. The `ident_case` crate (via darling, via rmcp-macros) uses `MIT/Apache-2.0` format which is a non-standard SPDX expression. cargo-deny may flag it.

**How to avoid:** After adding rmcp, run `just deny` immediately. If it fails on license issues, check the specific crate and either:
1. Add the license to `[licenses].allow` in deny.toml
2. Add a specific exception in `[licenses].exceptions`

**Warning signs:** `just deny` failing with "license not in allow list" errors.

### Pitfall 5: Workspace Member Glob Not Matching

**What goes wrong:** New `assay-mcp` crate isn't picked up by `cargo build --workspace`.

**Why it happens:** Forgetting to verify the workspace members pattern matches the new crate directory.

**How to avoid:** The current workspace uses `members = ["crates/*"]` glob pattern. As long as `assay-mcp` is created under `crates/`, it's automatically included. Verify with `cargo metadata --no-deps` after creation.

**Warning signs:** `cargo check -p assay-mcp` works but `cargo build --workspace` doesn't include it.

### Pitfall 6: tokio Feature Flag Mismatch

**What goes wrong:** Build errors about missing tokio features when rmcp needs `io-util` or `io-std`.

**Why it happens:** The workspace currently declares `tokio = { version = "1", features = ["full"] }`. This includes everything, so it's not a problem. But if someone later trims features, rmcp needs specific ones.

**How to avoid:** Keep `features = ["full"]` on the workspace tokio dependency. rmcp's `transport-io` feature requires `tokio/io-std` and `tokio/io-util`, both included in `full`.

**Warning signs:** tokio compilation errors about missing `io::stdin` or `io::stdout`.

## Code Examples

Verified patterns from official sources:

### Current assay-types Derive Pattern (Unchanged After Migration)

```rust
// Source: crates/assay-types/src/lib.rs (current code)
// This exact pattern works identically with schemars 0.8 AND 1.x
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Spec {
    pub name: String,
    pub description: String,
}
```

### Workspace Dependency Declaration for New Dependencies

```toml
# Root Cargo.toml — add to [workspace.dependencies]
rmcp = { version = "0.17", features = ["server", "transport-io"] }

# Update existing schemars entry
schemars = "1"  # was "0.8"
```

### Minimal assay-mcp Cargo.toml

```toml
[package]
name = "assay-mcp"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "MCP server library for Assay"

[dependencies]
assay-core.workspace = true
rmcp.workspace = true
schemars.workspace = true
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
```

### Minimal assay-mcp lib.rs

```rust
//! MCP server library for Assay.
//!
//! Provides Model Context Protocol server implementation
//! that exposes Assay's spec and gate functionality to AI agents.
```

### rmcp Server Pattern (Reference for Later Phases)

```rust
// Source: Context7 /websites/rs_rmcp - docs.rs/rmcp/latest/rmcp/index
// NOT for Phase 1 — included as reference for Phase 2+
use rmcp::{
    ErrorData as McpError, model::*,
    tool, tool_router,
    handler::server::tool::ToolRouter,
};

#[derive(Clone)]
pub struct AssayServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AssayServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get a specification by name")]
    async fn spec_get(&self, /* params */) -> Result<CallToolResult, McpError> {
        // Delegate to assay-core functions
        todo!()
    }
}
```

## State of the Art

| Old Approach                    | Current Approach                 | When Changed       | Impact                                |
| ------------------------------- | -------------------------------- | ------------------ | ------------------------------------- |
| schemars 0.8 (`schemars::schema::Schema`) | schemars 1.x (`schemars::Schema`) | June 2025 (v1.0.0) | Module path change; Schema type is now serde_json::Value wrapper |
| schemars 0.8 `RootSchema`      | schemars 1.x `Schema` (unified) | June 2025 (v1.0.0) | No more separate root schema type     |
| schemars 0.8 Visitors          | schemars 1.x Transforms         | June 2025 (v1.0.0) | Complete API rename (not used in this project) |
| rmcp (various pre-1.0)         | rmcp 0.17.0                      | Feb 2026            | Official SDK under modelcontextprotocol org |

**Deprecated/outdated:**
- `schemars::schema` module: Removed in 1.x. Use `schemars::Schema` directly.
- `RootSchema`: Removed in 1.x. `schema_for!()` now returns `Schema`.
- `dyn-clone` dependency: Removed from schemars 1.x dep tree.
- `serde_derive_internals` dependency: Removed from schemars_derive 1.x dep tree (was 0.8-specific).

## Open Questions

Things that couldn't be fully resolved:

1. **Exact schemars pin version**
   - What we know: rmcp declares `schemars = { version = "1.0" }` which is caret range >=1.0.0, <2.0.0. Latest schemars is 1.2.1. The user decision says "pin to exact version rmcp requires."
   - What's unclear: Whether the user wants `=1.0.0` (truly exact), `"1.0"` (caret, matching rmcp's declaration), or `"1"` (widest caret). All three are compatible with rmcp.
   - Recommendation: Use `"1"` — matches rmcp's own declaration style, picks up patch fixes, no compatibility risk within semver 1.x. If user insists on exact, use `"=1.0.0"`.

2. **cargo-deny with ident_case crate**
   - What we know: `ident_case` (transitive dep via darling via rmcp-macros) uses non-standard `MIT/Apache-2.0` license expression instead of SPDX `MIT OR Apache-2.0`.
   - What's unclear: Whether cargo-deny will flag this or handle it gracefully. Recent cargo-deny versions handle common non-SPDX expressions.
   - Recommendation: Run `just deny` after adding rmcp. If it fails on ident_case, add `{ allow = ["ident_case"], name = "MIT/Apache-2.0" }` exception or update the crate's license interpretation in deny.toml.

3. **Whether to add tracing as workspace dependency now**
   - What we know: rmcp uses tracing internally. Future phases will need structured logging.
   - What's unclear: Whether to add tracing to workspace deps proactively or wait until a phase needs it directly.
   - Recommendation: Don't add it in Phase 1. rmcp pulls it transitively. Add to workspace deps only when assay-mcp or assay-core needs to emit traces directly.

## Sources

### Primary (HIGH confidence)
- Context7 `/websites/rs_rmcp` — rmcp server setup, tool macros, transport, ServerHandler trait, dependencies
- `cargo info rmcp` (v0.17.0) — confirmed latest version, feature flags
- `cargo info schemars@1.2.1` — confirmed latest 1.x version, feature flags
- GitHub `modelcontextprotocol/rust-sdk` main branch `Cargo.toml` — rmcp 0.17.0 workspace, schemars `"1.0"` dependency declaration
- GitHub `modelcontextprotocol/rust-sdk` `crates/rmcp/Cargo.toml` — exact dependency versions, feature flag definitions
- Schemars migration guide (https://graham.cool/schemars/migrating/) — breaking changes from 0.8 to 1.x
- Schemars changelog (https://github.com/GREsau/schemars/blob/master/CHANGELOG.md) — version history, breaking changes
- Schemars derive docs (https://graham.cool/schemars/deriving/) — derive macro usage in 1.x

### Secondary (MEDIUM confidence)
- `/tmp/rmcp-check` test project — verified rmcp 0.17 + schemars 1.x compile together, confirmed transitive license tree
- Schemars v1.0.0 release notes (https://github.com/GREsau/schemars/releases/tag/v1.0.0) — release date, key changes

### Tertiary (LOW confidence)
- None — all findings verified with primary or secondary sources.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — versions confirmed via `cargo info`, APIs verified via Context7 and official docs
- Architecture: HIGH — workspace pattern is already established in the project; new crate follows existing convention
- Pitfalls: HIGH — schemars migration impact verified by inspecting actual codebase usage; license tree verified by building test project
- schemars migration: HIGH — current codebase only uses `#[derive(JsonSchema)]` on basic types, which is unchanged between 0.8 and 1.x

**Research date:** 2026-02-28
**Valid until:** 2026-03-30 (stable libraries, 30-day validity)
