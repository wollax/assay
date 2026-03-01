# Research Summary: Assay v0.1.0 Proof of Concept

**Synthesized:** 2026-02-28
**Source files:** STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md
**Purpose:** Decision-ready synthesis for roadmap planning

---

## Executive Summary

Assay v0.1.0 is buildable now, with one mandatory prerequisite and four critical pitfalls to design around before writing a line of implementation code. The mandatory prerequisite is upgrading schemars from 0.8 to 1.x — this is forced by rmcp's `server` feature and cannot be deferred. The good news: it is a mechanical two-line change with near-zero breakage risk given the codebase uses only derive macros and no schemars APIs. The bad news: it touches every type in assay-types and must be the first commit of the implementation sprint.

The architecture is sound. Four researchers independently arrived at the same structure: a new `assay-mcp` library crate sitting between `assay-cli` and `assay-core`, with gate evaluation kept synchronous in core and bridged via `tokio::task::spawn_blocking` in the MCP layer. This separation is not optional — it is the direct mitigation for three of the four critical pitfalls (pipe buffer deadlock, async/sync collision, zombie processes). The stdout corruption pitfall (P-01) is the single most dangerous failure mode: any `println!()` in any crate reachable from the MCP server path silently breaks the protocol without a useful error. This constraint must be enforced at the design level, not patched after the fact.

The feature set for v0.1.0 is deliberately narrow and correct. The vertical slice is: config init, TOML spec files, command gate evaluation, MCP server (two tools: `spec_get` + `gate_run`), and Claude Code plugin. Nothing else ships in v0.1. The dual-track gate differentiator (agent-evaluated criteria) is correctly deferred to v0.2, but the type system should be designed to accommodate it from day one — specifically, `Criterion.cmd` as `Option<String>` with a reserved `prompt` field path.

---

## Key Findings

### Stack (STACK.md)

**New workspace dependencies required:**

| Crate | Version | Purpose |
|---|---|---|
| `rmcp` | 0.17 | Official MCP Rust SDK (modelcontextprotocol org) |
| `toml` | 1 | TOML parsing for config and spec files |
| `tracing` | 0.1 | Structured logging (MCP server must log to stderr) |
| `tracing-subscriber` | 0.3 | Log subscriber for MCP binary |

**Mandatory upgrade:**

`schemars = "0.8"` → `schemars = "1"` — forced by rmcp's `server` feature. The upgrade is mechanical: only `derive(JsonSchema)` is used in assay-types, and the derive macro works identically in 1.x. No schemars API surfaces are called. Risk: low. Deferability: zero.

**rmcp maturity:** rmcp 0.17.0 was released 2026-02-27. It is the canonical SDK (hosted under the `modelcontextprotocol` GitHub org), has MCP conformance tests as of 0.17, and covers stdio transport fully. It is pre-1.0 and has released 7 times in 3 months. Pin to `"0.17"` (minor, not patch) and plan for potential breaking changes at `0.18`. The MCP server code should be fully isolated in `assay-mcp` to minimize upgrade blast radius.

**Crate placement:**

- `toml`: workspace dep, consumed by `assay-core`
- `rmcp`, `tracing`, `tracing-subscriber`: workspace deps, consumed by `assay-mcp` only
- `assay-cli` gains `assay-mcp` as a dependency and `tokio` (to call the async `serve()` function)

---

### Features (FEATURES.md)

**The non-negotiable v0.1.0 table stakes (11 items):**

1. `AssayError` enum with thiserror, `#[non_exhaustive]`
2. `GateKind` enum, `GateResult` with evidence fields (stdout, stderr, exit_code, duration, timestamp)
3. `assay init` — creates `.assay/config.toml` + `specs/` directory
4. Config loading — TOML parse + validation as free functions in core
5. TOML spec files — criteria with optional `cmd` field
6. Spec validation — name required, unique criteria names, trim-then-validate
7. Gate evaluation — command execution, exit code, stdout/stderr capture, timeout
8. CLI subcommands — `init`, `validate`, `gate run`, `spec show`, `mcp serve`
9. MCP server — stdio via rmcp, `spec_get` + `gate_run` tools
10. Claude Code plugin — `plugin.json` + `.mcp.json` + gate-check skill
11. Schema generation — schemars-based binary + `just schemas`

**The differentiators worth shipping if sprint capacity allows:**

- `spec_list` MCP tool (enumerate available specs) — low effort, high agent UX value
- Aggregate gate results ("3/5 passed" summary) — agents need this for decision-making
- PostToolUse hook (auto-gate after code write/edit) — turns the plugin from passive to active
- Stop hook (prevent agent completion without passing gates) — the core behavioral guarantee

**Explicit anti-features (do not add, even if easy):**

- Interactive init wizard — this is an agent-first tool
- Markdown spec bodies — structured TOML is the deliberate differentiation from spec-kit/OpenSpec
- Gate caching — gates must always re-evaluate fresh
- SSE/HTTP transport — local-only in v0.1
- Parallel gate execution — document async guidance, ship sync; avoid complexity

---

### Architecture (ARCHITECTURE.md)

**New dependency graph:**

```
assay-cli ──> assay-mcp ──> assay-core ──> assay-types
assay-tui ──────────────> assay-core ──> assay-types
```

`assay-mcp` is a library crate (`crates/assay-mcp/`) exposing `pub async fn serve() -> Result<()>`. The CLI calls it; there is no separate binary. This produces a single `assay` binary, simplifying plugin configuration and distribution.

**assay-mcp structure (intentionally small, ~200-300 lines):**

```
crates/assay-mcp/
  src/
    lib.rs      # pub async fn serve(), module declarations
    server.rs   # AssayServer struct, #[tool_router] impl, ServerHandler impl
```

**The sync/async bridge (mandatory pattern):**

Gate evaluation is synchronous (`std::process::Command`). Every call from an async rmcp tool handler must use `tokio::task::spawn_blocking`:

```rust
#[tool(name = "gate_run", description = "Run gate criteria for a spec")]
async fn gate_run(&self, Parameters(req): Parameters<GateRunRequest>) -> Result<CallToolResult, McpError> {
    let result = tokio::task::spawn_blocking(move || {
        assay_core::gate::evaluate(&gate, &working_dir)
    }).await
      .map_err(|e| McpError::internal_error(e.to_string(), None))?
      .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    // ...
}
```

**Build order:**

1. assay-types (schemars upgrade, domain model redesign)
2. assay-core (error types, config, spec, gate modules)
3. assay-mcp (MCP server, depends on core)
4. assay-cli (CLI subcommands, depends on core + mcp)
5. assay-tui (no changes, must still compile)
6. plugins/claude-code (static JSON config files)

**No workspace member changes needed:** `members = ["crates/*"]` already covers the new `assay-mcp` crate.

---

### Pitfalls (PITFALLS.md)

**Four critical pitfalls that require design-level mitigation (not patches):**

**P-01 — Stdout corruption (Critical):** Any non-JSON-RPC output to stdout breaks the MCP protocol silently. This includes `println!()`, tracing output defaulting to stdout, and clap error messages. Mitigation: initialize `tracing-subscriber` with `.with_writer(std::io::stderr).with_ansi(false)` as the first line of the MCP server path. Treat stdout corruption as a P0 bug, not a debug issue. Audit assay-core for any `println!()` calls — core must return data, never print.

**P-02 — Pipe buffer deadlock (Critical):** Using `Command::spawn()` + reading stdout + reading stderr + `wait()` deadlocks when output exceeds pipe buffer (~64KB on macOS). Use `Command::output()` exclusively in gate evaluation. Document this in a code comment to prevent future "optimization."

**P-03 — Async/sync collision (Critical):** Calling `Command::output()` directly in an async tool handler blocks the tokio worker thread. Under single-threaded runtimes (including `#[tokio::test]`), this deadlocks. Mitigation: `spawn_blocking` for all gate evaluation calls from async context. Keep assay-core gate functions fully synchronous — no async dependency.

**P-04 — Zombie processes (Critical):** `std::process::Child` has no `Drop` that kills the child. Dropped children become orphans; timed-out children become zombies unless explicitly `kill()`ed then `wait()`ed. Implement timeout as: poll with `try_wait()`, on timeout call `child.kill()` then `child.wait()`. Set a default timeout (300 seconds) configurable per gate.

**Eight moderate pitfalls (design-level mitigations):**

- **P-05:** Use `#[serde(tag = "kind")]` (internal tagging) on `GateKind`. External tagging does not work with TOML. Decide this before publishing any spec file format.
- **P-06:** Use `#[serde(deny_unknown_fields)]` on spec/config structs. Wrap TOML errors with file path and context. Poor error messages are user-hostile for a file format users write by hand.
- **P-07:** Avoid `#[from]` on error variants; use explicit `map_err` with context (file path, operation). Domain-specific variants, not passthrough wrappers.
- **P-08:** Keep MCP tool parameter structs flat — no `#[serde(flatten)]`, no custom serializers. Verify roundtrip in tests.
- **P-09:** Plugin `.mcp.json` should reference `"command": "assay"` (system PATH) not a relative workspace path. Document `cargo install` requirement. Use absolute `target/debug/` path in local dev override.
- **P-10:** Always set `Command::current_dir()` explicitly. Never inherit. Resolve relative `working_dir` values against the project root (directory containing `.assay/`).
- **P-11:** rmcp feature flags: `["server", "transport-io"]` — `macros` is included in `server`. Add a comment in `Cargo.toml` explaining each feature's purpose.
- **P-20:** When `assay mcp serve` is invoked, clap must not print anything to stdout. Configure clap error output to stderr. Test by running `assay mcp serve` and asserting first stdout byte is `{`.

---

## Implications for Roadmap

### Phase 0: Prerequisites (1-2 days, before any feature work)

**Do these first, in this order, or the sprint will stall:**

1. **Upgrade schemars 0.8 → 1.x** in root `Cargo.toml`. Run `just ready`. Verify clean. This is a blocker for everything involving rmcp.
2. **MCP spike** — build a hardcoded 1-tool rmcp server in `assay-mcp`, wire `assay mcp serve`, install the plugin in Claude Code, call the tool. This is a GO/NO-GO gate for the entire v0.1 architecture. If rmcp doesn't work with Claude Code's MCP client at the current version, the whole plan needs to pivot. Do not skip this spike. Budget 1 day.
3. **Add workspace deps** — `rmcp`, `toml`, `tracing`, `tracing-subscriber`, upgraded `schemars`.

Rationale: Three of the four critical pitfalls live at the MCP/async boundary. The spike validates the boundary is real, the `spawn_blocking` pattern works, and stdout stays clean before any domain logic is built on top.

---

### Phase 1: Domain Foundation (2-3 days)

**Implement in assay-types and assay-core, in dependency order:**

1. **assay-types redesign** — `GateKind` enum with `#[serde(tag = "kind")]`, `GateResult` with evidence fields, `Criterion` with `cmd: Option<String>`. No `passed` field on `Gate`. Forward-compatible `prompt` field path.
2. **AssayError** — thiserror enum, `#[non_exhaustive]`, domain-specific variants with context (not passthrough `#[from]`).
3. **Config loading** — `assay_core::config::load()`, `from_str()`, `validate()`. Uses `toml` crate. `#[serde(deny_unknown_fields)]` on config struct.
4. **Spec parsing** — `assay_core::spec::load_spec()`, `parse_spec()`, `validate()`. `+++` delimiter format. `#[serde(deny_unknown_fields)]`.
5. **Gate evaluation** — `assay_core::gate::evaluate()`. `Command::output()` (not spawn+wait). Explicit `current_dir`. Timeout with kill+wait. Returns `GateResult` with full evidence.
6. **Schema generation** — example binary in `assay-types/examples/`, `just schemas` justfile entry.

Rationale: This phase produces the entire domain without any presentation concerns. Both CLI and MCP server will delegate to these functions identically. Testing at this phase is pure Rust, no MCP complexity.

---

### Phase 2: CLI Surface (1-2 days)

**Thin clap wrappers over Phase 1:**

1. `assay init` — create `.assay/` + `config.toml` + `specs/` + example spec
2. `assay spec show <name>` — load and display parsed spec
3. `assay gate run <spec>` — evaluate all gates, print structured results
4. `assay mcp serve` — call `assay_mcp::serve().await` (Phase 3)

Rationale: CLI is the human-facing surface. Build it before the MCP surface to validate that the domain logic is correct from an interactive perspective. The `mcp serve` subcommand is a one-liner stub until Phase 3.

---

### Phase 3: MCP Server (2-3 days)

**assay-mcp crate implementation:**

1. `AssayServer` struct with `project_root: PathBuf` + `tool_router: ToolRouter<Self>`
2. `spec_get` tool — load and return `Spec` as JSON
3. `gate_run` tool — evaluate gates via `spawn_blocking`, return `Vec<GateResult>` as JSON
4. `ServerHandler` implementation with capabilities and instructions
5. `pub async fn serve()` wired to `rmcp::transport::stdio()`
6. tracing-subscriber initialized to stderr with ANSI disabled

Validation: Run `assay mcp serve`, pipe stdout through `jq`, assert clean JSON-RPC. Run gate tools against a real spec. Verify no stdout corruption.

---

### Phase 4: Claude Code Plugin (1 day)

**Static config files, no Rust:**

1. `.mcp.json` with `"command": "assay", "args": ["mcp", "serve"]`
2. `plugin.json` updated
3. `CLAUDE.md` workflow instructions snippet
4. gate-check skill (`SKILL.md`)
5. spec-show skill (`SKILL.md`)
6. (If time: PostToolUse hook for auto-gate after Write/Edit)

Validation: Install plugin in Claude Code. Ask Claude to call `spec_get`. Ask Claude to call `gate_run`. Observe structured results. Adjust tool descriptions based on what the agent does or doesn't understand.

---

### Differentiators to Ship Before v0.1.0 RC

In priority order (do these after Phase 4 if sprint capacity remains):

1. `spec_list` MCP tool — trivial to add, high agent UX value
2. Aggregate gate results — "3/5 passed" summary in `gate_run` response
3. Stop hook — prevents agent from considering work done with failing gates
4. PostToolUse hook — auto-trigger gate evaluation after code changes

---

## Confidence Assessment

| Finding | Confidence | Basis |
|---|---|---|
| schemars 0.8 → 1.x is a blocker | High | rmcp 0.17.0 Cargo.toml verified; trait incompatibility is fundamental |
| schemars upgrade is low-risk | High | Only derive macros used; no schemars API surfaces in assay-types |
| rmcp 0.17 works with Claude Code | Medium | SDK is official and has conformance tests, but not end-to-end tested in this codebase. Spike required. |
| `spawn_blocking` is correct bridge pattern | High | Documented tokio pattern; confirmed in rmcp examples |
| `Command::output()` prevents pipe deadlock | High | Well-documented Rust issue; `output()` is the stdlib-recommended solution |
| Plugin `.mcp.json` format is correct | High | Verified against Claude Code official plugin documentation |
| `#[serde(tag = "kind")]` required for TOML enums | High | Known TOML serde limitation; confirmed in multiple sources |
| rmcp API stability at 0.17 | Medium | Pre-1.0 with active development; changelog monitoring required |
| Gate timeout via `try_wait` polling | Medium | Works but is not elegant; `wait-timeout` crate may be cleaner |
| PostToolUse hook triggers gate evaluation | Medium | Hook format documented but not tested in this integration |

---

## Gaps to Address

**Before sprint kickoff (answers needed):**

1. **MCP spike result** — Does rmcp 0.17 + Claude Code's MCP client exchange protocol successfully? This is a binary yes/no that gates the entire architecture. Schedule this as day 1.

2. **Spec file format finalized** — The `+++` TOML frontmatter delimiter is in the architecture research but not validated against user expectations. Is this the right format, or should specs be pure TOML files (no delimiter)? The delimiter adds complexity to parsing with no clear benefit if specs don't have a markdown body section.

3. **Timeout implementation** — PITFALLS.md flags the `wait-timeout` crate as a cleaner option over manual `try_wait` polling. Evaluate before implementing gate evaluation. Using a crate adds a dependency; rolling it by hand adds a code maintenance burden.

4. **Gate `working_dir` resolution rule** — When `working_dir` is relative in a spec TOML, the anchor must be defined and documented before any spec files are written. Recommendation: anchor to the directory containing the `.assay/` directory (project root). This must be in the spec before implementation.

5. **rmcp `schemars` feature flag** — ARCHITECTURE.md lists `["server", "transport-io", "schemars"]` while STACK.md lists `["server", "transport-io"]` (noting `schemars` is implicit via `server`). Verify which features are required by running `cargo check` with both configurations during the spike.

6. **Plugin binary distribution strategy** — The plugin currently has no install mechanism. For v0.1.0 proof-of-concept, `cargo install` + PATH is sufficient. But the roadmap needs a decision on whether v0.2 will add a brew formula, cargo-binstall support, or another distribution channel before the plugin is usable by non-Rust developers.

---

*Synthesized from parallel research by 4 agents — 2026-02-28*
