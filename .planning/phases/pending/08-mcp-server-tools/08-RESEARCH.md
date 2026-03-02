# Phase 8: MCP Server Tools - Research

**Researched:** 2026-03-02
**Domain:** MCP server tool implementation with rmcp, tool registration, parameter schemas, error handling, agent-facing descriptions
**Confidence:** HIGH

## Summary

Phase 8 replaces the Phase 2 spike server with three real MCP tools (`spec_get`, `spec_list`, `gate_run`) that expose Assay's spec and gate operations to AI agents. The spike code in `crates/assay-mcp/src/spike.rs` already demonstrates the complete rmcp server lifecycle -- `#[tool_router]` for tool registration, `#[tool_handler]` for ServerHandler impl, `#[tool]` for individual methods, and `SpikeServer::new().serve(stdio()).await` for transport. This phase is a controlled replacement: same macro patterns, same transport, same tracing strategy, but with real tools that delegate to `assay-core`.

The primary technical challenges are:

1. **Tool parameter schemas** -- rmcp's `Parameters<T>` wrapper auto-generates JSON Schema from `#[derive(JsonSchema, Deserialize)]` structs. Each tool's input becomes a discoverable schema that agents can inspect.
2. **spawn_blocking bridge** -- `gate_run` must call sync `assay_core::gate::evaluate_all()` from async tool handlers. This is a documented pattern with one nuance: the closure must own all data (`spec`, `working_dir`, `config_timeout`) because `spawn_blocking` requires `'static`.
3. **Bounded response design** -- Per brainstorm decision, `gate_run` returns summary by default (pass/fail, exit code, duration per criterion). The optional `include_evidence` parameter controls whether full stdout/stderr is included, keeping default responses bounded.
4. **Error surface strategy** -- MCP spec distinguishes protocol errors (JSON-RPC `error`) from tool execution errors (`isError: true` in result). Tool execution errors are fed back to the LLM for self-correction; protocol errors are not. All Assay tool errors (spec not found, config missing, gate failures) should use `isError: true`.

No new workspace dependencies are required. The existing `rmcp`, `serde`, `serde_json`, `schemars`, and `tokio` dependencies in `assay-mcp/Cargo.toml` are sufficient.

## Standard Stack

### Core (already in assay-mcp/Cargo.toml)

| Library       | Version | Purpose                                          | Notes                                           |
| ------------- | ------- | ------------------------------------------------ | ----------------------------------------------- |
| rmcp          | 0.17    | MCP server SDK: macros, transport, model types   | `server` + `transport-io` features              |
| tokio         | 1       | Async runtime, `spawn_blocking` for sync bridge  | `full` feature                                  |
| serde         | 1       | Deserialize tool parameters                      | `derive` feature                                |
| serde_json    | 1       | Serialize tool results to JSON                   | For `CallToolResult::success(vec![Content::text])` |
| schemars      | 1       | Auto-generate JSON Schema for tool input schemas | Used via `Parameters<T>` wrapper                |
| tracing       | 0.1     | Structured logging to stderr                     | Already configured in spike                     |

### Consumed from other workspace crates

| Library       | Crate       | Purpose                                          |
| ------------- | ----------- | ------------------------------------------------ |
| assay-core    | assay-core  | `spec::load`, `spec::scan`, `gate::evaluate_all`, `config::load` |
| assay-types   | (transitive)| `Spec`, `Criterion`, `GateResult`, `Config`      |

### New Dependencies Required

**None.** `assay-mcp/Cargo.toml` already has everything needed. The `assay-types` crate is accessed transitively through `assay-core`.

### Alternatives Considered

| Instead of                     | Could Use                  | Why Not                                                              |
| ------------------------------ | -------------------------- | -------------------------------------------------------------------- |
| `Parameters<T>` wrapper        | Manual JSON deserialization | Wrapper auto-generates schema; manual loses agent discoverability    |
| `CallToolResult::success` text | `CallToolResult::structured` | `structured` uses `structuredContent` field requiring `outputSchema`; text JSON in `content` is simpler for v0.1 and universally supported by all MCP clients |
| `#[tool(tool_box)]` pattern    | `#[tool_router]`/`#[tool_handler]` | `tool_box` is an alternative rmcp API; spike already uses `tool_router`/`tool_handler` and it works. Consistency over novelty. |
| State in `Arc<Mutex<T>>`       | Reload on each call        | No mutable server state needed; config/specs are loaded per-call from disk. Simplest correct approach for v0.1. |

## Architecture Patterns

### Pattern 1: Server Struct with ToolRouter

The server struct holds a `ToolRouter<Self>` and optionally cached project state. For v0.1, per-call resolution (load config + specs on each tool call) is simpler and correct -- no stale state issues, no startup validation needed. The tradeoff is disk I/O per call, which is negligible for local `.toml` files.

```rust
// crates/assay-mcp/src/server.rs

#[derive(Clone)]
pub struct AssayServer {
    tool_router: ToolRouter<Self>,
}

impl Default for AssayServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl AssayServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    // Tools defined here with #[tool] attribute
}

#[tool_handler]
impl ServerHandler for AssayServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Assay development kit. Manages specs (what to build) and gates \
                 (quality checks). Use spec_list to discover specs, spec_get to \
                 read one, gate_run to evaluate criteria."
                    .to_string(),
            ),
        }
    }
}
```

**Resolution timing decision: per-call.** Reasons:
- MCP servers are long-lived processes; specs/config can change while the server runs
- Per-call resolution means every response reflects current disk state
- Config + spec loading is fast (TOML parse of small files)
- No startup failure mode to handle (server starts even if `.assay/` is missing; tools return errors when called)
- Matches the CLI pattern where each subcommand loads fresh config

### Pattern 2: Tool Parameters via Derive Structs

rmcp's `Parameters<T>` wrapper automatically generates JSON Schema from structs that derive `JsonSchema` and `Deserialize`. The `#[schemars(description = "...")]` attribute provides parameter-level documentation that agents see during tool discovery.

```rust
use rmcp::handler::server::wrapper::Parameters;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
struct SpecGetParams {
    /// The spec name (filename without .toml extension)
    #[schemars(description = "Spec name (filename without .toml extension, e.g. 'auth-flow')")]
    name: String,
}

#[derive(Deserialize, JsonSchema)]
struct GateRunParams {
    /// The spec name whose criteria to evaluate
    #[schemars(description = "Spec name to evaluate gates for (filename without .toml extension)")]
    name: String,

    /// When true, includes full stdout/stderr for each criterion
    #[schemars(description = "Include full stdout/stderr evidence per criterion (default: false, returns summary only)")]
    #[serde(default)]
    include_evidence: bool,
}
```

`spec_list` takes no parameters. For tools with no parameters, rmcp generates an empty object schema `{ "type": "object" }` automatically when the method signature has no `Parameters<T>` argument.

### Pattern 3: spawn_blocking Bridge for Gate Evaluation

`gate::evaluate_all()` is synchronous (spawns child processes, blocks on pipe reads). MCP tool handlers are async. The bridge uses `tokio::task::spawn_blocking` with owned data.

```rust
#[tool(description = "Run quality gate checks for a spec's criteria")]
async fn gate_run(
    &self,
    params: Parameters<GateRunParams>,
) -> Result<CallToolResult, McpError> {
    let cwd = resolve_cwd()?;
    let config = load_config(&cwd)?;
    let spec = load_spec(&cwd, &config, &params.0.name)?;
    let include_evidence = params.0.include_evidence;

    let working_dir = resolve_working_dir(&cwd, &config);
    let config_timeout = config.gates.as_ref().map(|g| g.default_timeout);

    // Clone what the closure needs to own
    let spec_owned = spec.clone();
    let working_dir_owned = working_dir.clone();

    let summary = tokio::task::spawn_blocking(move || {
        assay_core::gate::evaluate_all(
            &spec_owned,
            &working_dir_owned,
            None, // no CLI timeout in MCP context
            config_timeout,
        )
    })
    .await
    .map_err(|e| McpError::internal_error(format!("gate evaluation panicked: {e}"), None))?;

    let response = format_gate_response(&summary, include_evidence);
    Ok(CallToolResult::success(vec![Content::text(response)]))
}
```

Key details:
- `spawn_blocking` returns `JoinHandle<GateRunSummary>` -- the `JoinError` only fires on panic, not on gate failures
- Gate failures are captured in `GateRunSummary.results` (each criterion has `passed: bool`)
- The closure must own `spec_owned` and `working_dir_owned` because `spawn_blocking` requires `'static`
- No `cli_timeout` in MCP context -- per-criterion and config timeouts still apply

### Pattern 4: Error Surface Strategy

The MCP spec defines two error channels:

1. **Protocol errors** (`Err(McpError)`) -- returned as JSON-RPC `error` objects. Clients may or may not show these to the LLM. Used for: unknown tool, malformed request, server crash.
2. **Tool execution errors** (`Ok(CallToolResult { is_error: true })`) -- returned as successful JSON-RPC responses with `isError: true`. Clients feed these back to the LLM for self-correction. Used for: spec not found, config missing, invalid input.

**Decision: Use tool execution errors for all domain errors.** This ensures the agent sees the error and can self-correct (e.g., try a different spec name, ask the user to run `assay init`).

```rust
/// Convert an AssayError into a tool execution error that the agent can see and act on.
fn domain_error(err: &assay_core::AssayError) -> CallToolResult {
    CallToolResult::error(vec![Content::text(err.to_string())])
}
```

Error mapping:

| Error Condition         | Error Channel      | Agent Sees It? | Example Message                                    |
| ----------------------- | ------------------ | -------------- | -------------------------------------------------- |
| Spec not found          | `isError: true`    | Yes            | `spec 'foo' not found in specs/`                   |
| Config missing/invalid  | `isError: true`    | Yes            | `reading config at .assay/config.toml: No such file` |
| Spec parse error        | `isError: true`    | Yes            | `parsing spec .assay/specs/foo.toml: TOML error...`|
| Gate evaluation failure | Summary in result  | Yes            | Normal result with `failed > 0` in summary         |
| spawn_blocking panic    | `Err(McpError)`    | Maybe          | `gate evaluation panicked: ...`                    |
| Serde serialization bug | `Err(McpError)`    | Maybe          | `internal: failed to serialize response`           |

### Pattern 5: Tool Response Formatting

#### spec_get Response

Return the full parsed spec as JSON. The spec is small (name + description + criteria array) and already has `Serialize` + `skip_serializing_if` annotations. This gives agents everything they need to understand what to build and what gates will check.

```rust
#[tool(description = "Get a spec by name, returning its full definition including criteria")]
async fn spec_get(
    &self,
    params: Parameters<SpecGetParams>,
) -> Result<CallToolResult, McpError> {
    let cwd = resolve_cwd()?;
    let config = load_config(&cwd)?;
    let spec = load_spec(&cwd, &config, &params.0.name)?;

    let json = serde_json::to_string(&spec)
        .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

    Ok(CallToolResult::success(vec![Content::text(json)]))
}
```

The response is inherently bounded: a spec has a name, optional description, and a Vec of criteria (each with name, description, optional cmd, optional timeout). Practical upper bound is ~2-5 KB even for large specs.

#### spec_list Response

Return array of `{ name, description, criteria_count }` objects. This provides enough metadata for an agent to decide which spec to inspect further without loading every spec's full criteria.

```rust
#[derive(Serialize)]
struct SpecListEntry {
    name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    description: String,
    criteria_count: usize,
}
```

#### gate_run Response (Summary Mode -- Default)

Return JSON with aggregate counts and per-criterion status. Exclude stdout/stderr by default.

```rust
#[derive(Serialize)]
struct GateRunResponse {
    spec_name: String,
    passed: usize,
    failed: usize,
    skipped: usize,
    total_duration_ms: u64,
    criteria: Vec<CriterionSummary>,
}

#[derive(Serialize)]
struct CriterionSummary {
    name: String,
    status: &'static str, // "passed", "failed", "skipped"
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>, // stderr summary for failures only

    // Only present when include_evidence is true
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stderr: Option<String>,
}
```

The summary mode is bounded: fixed structure, no variable-length output. The `reason` field for failures is extracted from stderr (first line or truncated), giving agents enough to understand what failed without the full output.

When `include_evidence: true`, the full `stdout` and `stderr` are included per criterion. This is opt-in because gate output can be large (cargo test output, linter results, etc.). The truncation already applied in `gate::evaluate` (64 KB per stream) provides an absolute upper bound.

### Pattern 6: Shared Helper Functions

Several operations are common across tools. Extract them as private functions in the server module to avoid duplication.

```rust
/// Resolve the current working directory.
fn resolve_cwd() -> Result<PathBuf, McpError> {
    std::env::current_dir()
        .map_err(|e| McpError::internal_error(
            format!("cannot determine working directory: {e}"), None
        ))
}

/// Load and validate the Assay config from CWD.
fn load_config(cwd: &Path) -> Result<Config, CallToolResult> {
    assay_core::config::load(cwd).map_err(|e| domain_error(&e))
}

/// Load a spec by name from the configured specs directory.
fn load_spec(cwd: &Path, config: &Config, name: &str) -> Result<Spec, CallToolResult> {
    let specs_dir = cwd.join(".assay").join(&config.specs_dir);
    let spec_path = specs_dir.join(format!("{name}.toml"));
    assay_core::spec::load(&spec_path).map_err(|e| domain_error(&e))
}

/// Resolve the gate working directory from config, matching CLI behavior.
fn resolve_working_dir(cwd: &Path, config: &Config) -> PathBuf {
    match config.gates.as_ref().and_then(|g| g.working_dir.as_deref()) {
        Some(dir) => {
            let path = Path::new(dir);
            if path.is_absolute() { path.to_path_buf() } else { cwd.join(path) }
        }
        None => cwd.to_path_buf(),
    }
}
```

**Important note on helper return types:** `load_config` and `load_spec` return `Result<T, CallToolResult>` (not `Result<T, McpError>`) because domain errors should be tool execution errors (`isError: true`), not protocol errors. The tool handler uses a match/early-return pattern:

```rust
let config = match load_config(&cwd) {
    Ok(c) => c,
    Err(err_result) => return Ok(err_result), // Returns Ok(CallToolResult { is_error: true })
};
```

## Don't Hand-Roll

| Problem                          | Use Instead                              | Rationale                                                        |
| -------------------------------- | ---------------------------------------- | ---------------------------------------------------------------- |
| Tool input schema generation     | `Parameters<T>` with `#[derive(JsonSchema)]` | rmcp generates correct JSON Schema automatically; manual schemas drift |
| Tool registration / routing      | `#[tool_router]` + `#[tool_handler]` macros | Generates `call_tool` and `list_tools` boilerplate               |
| Sync-to-async bridge             | `tokio::task::spawn_blocking`            | Correct way to call blocking code from async; do not block the tokio runtime |
| JSON serialization for responses | `serde_json::to_string` on `#[derive(Serialize)]` structs | Consistent, testable, matches existing codebase patterns         |
| Server info / version            | `Implementation::from_build_env()`       | Auto-populates from Cargo.toml; already used in spike            |
| Config/spec loading              | `assay_core::config::load`, `assay_core::spec::load`, `assay_core::spec::scan` | Existing validated functions; never reimplement loading/validation |
| Gate evaluation                  | `assay_core::gate::evaluate_all`         | Existing function handles all criteria types, timeouts, truncation |

## Common Pitfalls

### P-01: stdout Corruption (CRITICAL)

**Problem:** Any `println!()` or write to stdout in any crate reachable from the MCP server path corrupts the JSON-RPC protocol. The MCP server owns stdout exclusively for JSON-RPC messages.

**Mitigation:** Already handled -- tracing is configured to write to stderr only (established in Phase 2 spike). The real risk is accidental `println!()` or `dbg!()` calls in `assay-core` or `assay-types`. Gate commands write to their own piped stdout/stderr, not the server's stdout.

**Verification:** Integration test that starts the MCP server, sends a `tools/list` request, and verifies the response parses as valid JSON-RPC without extraneous bytes.

### P-02: Blocking the Tokio Runtime

**Problem:** Calling `assay_core::gate::evaluate()` directly in an async handler blocks the tokio runtime thread, preventing concurrent MCP message processing (including cancellation signals).

**Mitigation:** Always use `tokio::task::spawn_blocking` for gate evaluation. Spec loading and config loading are fast enough (~1ms for small TOML files) that they can run on the async thread without practical impact, but gate evaluation can take minutes.

**Verification:** The `spawn_blocking` call is in the tool handler, not in a helper function, making it visible in code review. A test that runs `gate_run` with a slow command while sending `tools/list` concurrently would verify non-blocking behavior but is complex for v0.1 -- flag as a v0.2 test.

### P-03: Closure Ownership in spawn_blocking

**Problem:** `spawn_blocking` requires a `'static` closure. References to `spec`, `working_dir`, `config` from the outer async scope do not satisfy `'static`.

**Mitigation:** Clone the needed data before the closure:

```rust
let spec_owned = spec.clone();
let working_dir_owned = working_dir.clone();
tokio::task::spawn_blocking(move || {
    assay_core::gate::evaluate_all(&spec_owned, &working_dir_owned, None, config_timeout)
})
```

`config_timeout` is `Option<u64>` which is `Copy`, so it moves without cloning. `Spec` and `PathBuf` are `Clone`.

### P-04: Tool Errors vs Protocol Errors

**Problem:** Returning `Err(McpError)` for domain errors (spec not found, config missing) means the agent likely never sees the error message -- protocol errors are consumed by the MCP client, not fed back to the LLM.

**Mitigation:** Use `CallToolResult::error(vec![Content::text(...)])` for all domain errors. Reserve `Err(McpError)` for genuine infrastructure failures (spawn_blocking panic, serialization bug).

**Verification:** Unit test that calls each tool handler with invalid input and asserts the result is `Ok(CallToolResult { is_error: Some(true), ... })`, not `Err(...)`.

### P-05: Missing .assay/ Directory

**Problem:** The MCP server starts in a directory without `.assay/`. All tool calls will fail with config loading errors.

**Mitigation:** Per-call resolution handles this gracefully -- each tool attempts to load config and returns a clear `isError: true` message like "reading config at `.assay/config.toml`: No such file or directory". The server itself starts fine (no startup validation).

**Verification:** Unit test with a tempdir that has no `.assay/`, calling each tool and asserting a helpful error message.

### P-06: Stale Tool Descriptions

**Problem:** If tool descriptions are vague or misleading, agents select the wrong tool or pass wrong parameters, wasting tokens and failing tasks.

**Mitigation:** Follow MCP tool description best practices (see Code Examples section). Each description should state: what the tool does, what it returns, and when to use it. Parameter descriptions should specify format and constraints. Validated by: reading the `tools/list` output and checking descriptions are self-sufficient for an agent with zero prior knowledge of Assay.

## Code Examples

### Example 1: Complete Tool Description Quality

Good tool descriptions answer three questions for the agent: What does it do? What does it return? When should I use it?

```rust
#[tool(description = "List all specs in the current Assay project. Returns an array of \
    {name, description, criteria_count} objects. Use this to discover available specs \
    before calling spec_get or gate_run.")]
async fn spec_list(&self) -> Result<CallToolResult, McpError> { ... }

#[tool(description = "Get a spec by name. Returns the full spec definition as JSON \
    including name, description, and all criteria with their commands. \
    Use spec_list first to find available spec names.")]
async fn spec_get(&self, params: Parameters<SpecGetParams>) -> Result<CallToolResult, McpError> { ... }

#[tool(description = "Run quality gate checks for a spec. Evaluates all executable \
    criteria (shell commands) and returns pass/fail status per criterion with aggregate \
    counts. Set include_evidence=true for full stdout/stderr output per criterion.")]
async fn gate_run(&self, params: Parameters<GateRunParams>) -> Result<CallToolResult, McpError> { ... }
```

Key qualities:
- Start with a verb (List, Get, Run)
- State the return shape (array of objects, full spec as JSON, pass/fail status)
- Include usage guidance (use spec_list first, set include_evidence for details)
- No jargon without context ("criteria" is explained as "shell commands")

### Example 2: Parameter Descriptions with schemars

```rust
#[derive(Deserialize, JsonSchema)]
struct GateRunParams {
    #[schemars(description = "Spec name to evaluate (filename without .toml extension, e.g. 'auth-flow')")]
    name: String,

    #[schemars(description = "Include full stdout/stderr evidence per criterion (default: false). \
        When false, returns summary only (pass/fail, exit code, duration). \
        When true, adds stdout and stderr fields to each criterion result.")]
    #[serde(default)]
    include_evidence: bool,
}
```

### Example 3: Tool Name Prefix Decision

**Recommendation: No prefix.** MCP tool names already carry server context -- agents see tools grouped by their server. Adding `assay_` to every tool name is redundant and wastes description tokens. The underscore convention (`spec_get`, not `specGet`) is already decided.

The server `instructions` field in `get_info()` provides the context: "Assay development kit. Manages specs and gates." Agents see this once and understand the namespace.

### Example 4: Server Instructions for Agent Orientation

The `instructions` field in `ServerInfo` is shown to the agent once during server initialization. Use it to provide a one-sentence orientation and a workflow hint:

```rust
instructions: Some(
    "Assay development kit. Manages specs (what to build) and gates \
     (quality checks). Use spec_list to discover specs, spec_get to \
     read one, gate_run to evaluate criteria."
        .to_string(),
),
```

### Example 5: Wire Format for gate_run Response

Summary mode (default, `include_evidence: false`):

```json
{
  "spec_name": "auth-flow",
  "passed": 2,
  "failed": 1,
  "skipped": 1,
  "total_duration_ms": 3450,
  "criteria": [
    { "name": "unit-tests", "status": "passed", "exit_code": 0, "duration_ms": 1200 },
    { "name": "type-check", "status": "passed", "exit_code": 0, "duration_ms": 800 },
    { "name": "integration", "status": "failed", "exit_code": 1, "duration_ms": 1450, "reason": "connection refused" },
    { "name": "review-checklist", "status": "skipped" }
  ]
}
```

Evidence mode (`include_evidence: true`):

```json
{
  "spec_name": "auth-flow",
  "passed": 2,
  "failed": 1,
  "skipped": 1,
  "total_duration_ms": 3450,
  "criteria": [
    {
      "name": "unit-tests",
      "status": "passed",
      "exit_code": 0,
      "duration_ms": 1200,
      "stdout": "running 12 tests\ntest auth::login ... ok\n...",
      "stderr": ""
    },
    {
      "name": "integration",
      "status": "failed",
      "exit_code": 1,
      "duration_ms": 1450,
      "reason": "connection refused",
      "stdout": "",
      "stderr": "Error: connection refused at 127.0.0.1:5432\n..."
    }
  ]
}
```

### Example 6: Module Structure

```
crates/assay-mcp/src/
  lib.rs          -- pub mod server; pub async fn serve()
  server.rs       -- AssayServer struct, tools, helpers
```

The spike module (`spike.rs`) is deleted entirely. The `lib.rs` public API remains `pub async fn serve()` -- the CLI calls it unchanged.

## Research Confidence

| Finding                                  | Confidence | Basis                                                      |
| ---------------------------------------- | ---------- | ---------------------------------------------------------- |
| `#[tool_router]`/`#[tool_handler]` macros | HIGH      | Working spike code + Context7 docs + official README        |
| `Parameters<T>` for input schemas        | HIGH       | Context7 docs + official README examples                   |
| `CallToolResult::success`/`::error`      | HIGH       | Context7 docs (source code) + MCP specification            |
| `isError: true` for domain errors        | HIGH       | MCP 2025-11-25 specification + multiple best practice guides |
| `spawn_blocking` for sync bridge         | HIGH       | tokio documentation + project STATE.md decision            |
| No tool name prefix                      | MEDIUM     | Based on MCP convention analysis; no hard spec requirement  |
| Per-call resolution (not startup)        | MEDIUM     | Architectural judgment; correct for v0.1 simplicity         |
| `schemars(description)` for param docs   | HIGH       | Context7 docs show rmcp reads JsonSchema annotations        |

---

*Phase: 08-mcp-server-tools*
*Research completed: 2026-03-02*
