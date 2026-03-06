# Phase 19 Research: Testing & Tooling

## Standard Stack

| Component | Library/Tool | Version | Confidence |
|-----------|-------------|---------|------------|
| MCP server | rmcp | 0.17 (workspace) | HIGH |
| Snapshot testing | insta | 1.46 (workspace, `json` feature) | HIGH |
| Temp dirs | tempfile | 3 (workspace) | HIGH |
| JSON handling | serde_json | 1 (workspace) | HIGH |
| Async runtime | tokio | 1 (workspace, `full` feature) | HIGH |
| Dep linting | cargo-deny | latest (via `mise install`) | HIGH |
| Schema validation | jsonschema | 0.43 (workspace, dev-dependency) | HIGH |

No new dependencies required. Everything needed is already in the workspace.

## Architecture Patterns

### MCP Handler Testing

**Direct handler tests (unit-level):** The handlers (`spec_list`, `spec_get`, `gate_run`, `gate_report`, `gate_finalize`, `gate_history`) are methods on `AssayServer` decorated with `#[tool]`. They accept `Parameters<T>` wrappers and return `Result<CallToolResult, McpError>`. Direct invocation in tests is **not straightforward** because:

1. The `Parameters<T>` wrapper is extracted from `ToolCallContext` via the `FromContextPart` trait in rmcp's macro system.
2. The `#[tool_router]` and `#[tool_handler]` macros generate dispatch code that routes `CallToolRequestParam` to individual handlers.
3. Handlers are `async fn` methods on `&self` -- they can be called if you can construct the `Parameters<T>` wrapper.

**Confidence: MEDIUM** -- The `Parameters<T>` type is a newtype wrapper (`pub struct Parameters<T>(pub T)`). Based on rmcp source inspection, it should be constructable as `Parameters(MyParams { ... })`. This needs verification at implementation time, but the pattern works for unit-level testing of the handler logic.

**Recommended approach for direct tests:**

```rust
#[tokio::test]
async fn spec_list_valid_project_returns_specs() {
    let dir = create_project(r#"project_name = "test""#);
    create_spec(dir.path(), "specs", "auth.toml", VALID_SPEC);

    // Set CWD to tempdir (handlers use resolve_cwd())
    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();
    let result = server.spec_list().await.unwrap();

    // Assert CallToolResult fields
    assert!(result.is_error.is_none() || !result.is_error.unwrap());
    let text = extract_text(&result);
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["specs"][0]["name"], "auth");
}
```

**Critical caveat:** `resolve_cwd()` calls `std::env::current_dir()`. Multiple tests setting CWD will race. Use `#[serial_test::serial]` or restructure to avoid CWD dependency. However, the CONTEXT.md decision says "no mock abstraction layer" -- so the CWD approach with serialization is the pragmatic path.

**Alternative: Use `ServerHandler::call_tool` trait method.** Construct a `CallToolRequestParam` with the tool name and JSON arguments, then call `server.call_tool(params, context)`. This tests the full dispatch path including parameter deserialization. However, constructing a valid `RequestContext` is non-trivial.

**Recommended strategy:**
- For handlers that don't need CWD (`format_gate_response`, `extract_agent_criteria_info`, helper functions): continue testing directly as pure functions (already done).
- For handler integration: use tempdir + `set_current_dir` + direct method call. Serialize tests with `#[serial_test::serial]` or run in separate processes.
- For protocol-level: defer to integration tests in `crates/assay-mcp/tests/`.

### Protocol-Level (JSON-RPC) Integration Tests

rmcp provides `transport::io::stdio` for the production transport. For testing, use `transport::io` with in-memory channels or paired pipes:

```rust
// In crates/assay-mcp/tests/protocol.rs
use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn jsonrpc_spec_list() {
    // Create paired streams
    let (client_read, server_write) = tokio::io::duplex(4096);
    let (server_read, client_write) = tokio::io::duplex(4096);

    // Start server on one side
    let server = AssayServer::new();
    tokio::spawn(async move {
        let service = server.serve((server_read, server_write)).await.unwrap();
        service.waiting().await.unwrap();
    });

    // Send JSON-RPC from client side
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": "spec_list", "arguments": {} }
    });
    // ... write request, read response, parse, snapshot
}
```

**Confidence: LOW** -- The exact transport API for in-memory testing in rmcp 0.17 needs verification. The `ServiceExt` trait's `.serve()` method signature and what it accepts for transport may differ. If in-memory transport is too complex, fall back to process-based tests spawning the binary.

### Test File Organization

Per CONTEXT.md decisions:
- **Unit tests:** `#[cfg(test)] mod tests` in `crates/assay-mcp/src/server.rs` (already has ~30 tests)
- **Integration tests:** `crates/assay-mcp/tests/` (new directory, new files)
- **Phase 3/6 gap tests:** Add to existing `#[cfg(test)]` modules in respective crates
- **Naming:** `{feature}_{scenario}_{expected}` -- no `test_` prefix for new tests

### Tempdir Test Setup Pattern

Already established in server.rs tests:

```rust
fn create_project(config_toml: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let assay_dir = dir.path().join(".assay");
    std::fs::create_dir_all(&assay_dir).unwrap();
    std::fs::write(assay_dir.join("config.toml"), config_toml).unwrap();
    dir
}

fn create_spec(project_dir: &Path, specs_dir: &str, filename: &str, content: &str) {
    let specs_path = project_dir.join(".assay").join(specs_dir);
    std::fs::create_dir_all(&specs_path).unwrap();
    std::fs::write(specs_path.join(filename), content).unwrap();
}
```

Reuse these helpers. Do not create a shared test utility crate (per CONTEXT.md).

### Insta Snapshot Pattern

Already established in `crates/assay-types/tests/schema_snapshots.rs`:

```rust
use insta::assert_json_snapshot;

#[test]
fn spec_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::Spec);
    assert_json_snapshot!("spec-schema", schema.to_value());
}
```

For MCP response snapshots, use the same pattern on the JSON response payloads:

```rust
use insta::assert_json_snapshot;

#[test]
fn gate_run_response_snapshot() {
    let summary = sample_summary();
    let response = format_gate_response(&summary, false);
    let json = serde_json::to_value(&response).unwrap();
    assert_json_snapshot!("gate-run-response-summary", json);
}
```

Snapshot files go in `crates/assay-mcp/src/snapshots/` (for unit tests) or `crates/assay-mcp/tests/snapshots/` (for integration tests).

## Don't Hand-Roll

| Problem | Use Instead |
|---------|-------------|
| Temp directories for tests | `tempfile::tempdir()` (already in workspace) |
| JSON snapshot assertions | `insta::assert_json_snapshot!` (already in workspace) |
| Async test runtime | `#[tokio::test]` (already in workspace) |
| Dependency version auditing | `cargo-deny` with `[bans]` config (already set up) |
| Serial test execution | `serial_test` crate OR separate test binaries via integration test files |
| MCP parameter construction | `Parameters(T { ... })` -- do not reimplement rmcp dispatch |

## Common Pitfalls

### 1. CWD Race Conditions in Handler Tests
**Risk: HIGH**

MCP handlers call `resolve_cwd()` which uses `std::env::current_dir()`. Tests that `set_current_dir()` will race with each other when run in parallel (cargo test default).

**Mitigation:** Either:
- Add `serial_test` as a dev-dependency and annotate CWD-dependent tests with `#[serial]`
- Use separate integration test files (each gets its own process) -- preferred since CONTEXT.md already specifies `crates/assay-mcp/tests/` for integration tests

### 2. cargo-deny Skip Entries Need Specific Versions
**Risk: MEDIUM**

When switching `multiple-versions` from `warn` to `deny`, the build will fail with 14 duplicate warnings. All are transitive dependency version splits:

| Crate | Versions | Root Cause |
|-------|----------|------------|
| `crossterm` | 0.28.1 vs 0.29.0 | assay-tui pins 0.28, ratatui uses 0.29 via ratatui-crossterm |
| `getrandom` | 0.3.4 vs 0.4.1 | jsonschema (via ahash) uses 0.3, others use 0.4 |
| `linux-raw-sys` | 2 versions | transitive via rustix versions |
| `rustix` | 2 versions | transitive via crossterm/polling versions |
| `windows-sys` | 3 versions | foundational Windows crate, version fragmentation |
| `windows-targets` + arch crates | 2 versions each | follow windows-sys split |

**Fix:** Add `skip` entries for each duplicate with version+reason, or use `skip-tree` for the windows-sys family. Example:

```toml
[bans]
multiple-versions = "deny"

skip = [
    { crate = "crossterm@0.28.1", reason = "ratatui 0.30 uses 0.29 via ratatui-crossterm, assay-tui directly uses 0.28" },
    { crate = "getrandom@0.3.4", reason = "ahash (jsonschema dep) uses old version" },
]

skip-tree = [
    { crate = "windows-sys@0.52.0", reason = "foundational crate with frequent version bumps" },
    { crate = "windows-sys@0.59.0", reason = "foundational crate with frequent version bumps" },
]
```

**Alternative:** Upgrade `crossterm` to 0.29 in workspace to eliminate that duplicate. Check if ratatui 0.30 is compatible with crossterm 0.29 directly (it uses ratatui-crossterm as an adapter). This could eliminate the crossterm + rustix + linux-raw-sys duplicates in one change.

### 3. sources Policy Is Already Clean
**Risk: LOW**

`cargo deny check sources` currently passes with `unknown-registry = "warn"` and `unknown-git = "warn"`. Changing to `deny` should pass immediately since all deps come from crates.io. Verify with `cargo deny check sources` after the change.

### 4. Insta Snapshots Must Be Committed
**Risk: LOW**

New snapshot files created by `insta` tests need to be committed. Run `cargo insta review` to accept new snapshots before committing. If running in CI, use `INSTA_UPDATE=no` to prevent silent snapshot updates.

### 5. Dogfooding Spec Depends on .assay/ Directory Existing
**Risk: MEDIUM**

The self-check.toml requires `.assay/config.toml` to exist. Currently there is NO `.assay/` directory in the repo. The phase must either:
- Run `assay init` to create it, OR
- Manually create `.assay/config.toml` and `.assay/specs/self-check.toml`

The latter is cleaner since `assay init` creates example files we don't want.

### 6. Handler Tests That Execute Real Commands
**Risk: MEDIUM**

`gate_run` handler actually spawns shell commands. Integration tests must use benign commands (`echo ok`, `true`, `false`) and never depend on the host having specific tools installed. The existing test fixtures already follow this pattern.

### 7. Session Timeout Spawns Background Tasks
**Risk: LOW**

The `gate_run` handler spawns a `tokio::spawn` for session timeout (30 min). In tests, this task will leak. Not a correctness issue for test assertions but may cause noisy warnings. The tokio test runtime handles this gracefully by default.

## Code Examples

### Constructing Parameters for Direct Handler Calls

```rust
use rmcp::handler::server::wrapper::Parameters;

// For spec_get:
let params = Parameters(SpecGetParams {
    name: "auth-flow".to_string(),
});
let result = server.spec_get(params).await.unwrap();

// For gate_run:
let params = Parameters(GateRunParams {
    name: "auth-flow".to_string(),
    include_evidence: false,
    timeout: None,
});
let result = server.gate_run(params).await.unwrap();
```

### Extracting Text from CallToolResult

```rust
fn extract_text(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match &c.raw {
            RawContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect()
}
```

This pattern is already used in existing tests (lines 1202-1210, 1333-1340 of server.rs).

### cargo-deny Configuration for deny-level Bans

```toml
[bans]
multiple-versions = "deny"
wildcards = "allow"
skip = [
    { crate = "crossterm@0.28.1", reason = "ratatui 0.30 transitively requires 0.29 via ratatui-crossterm" },
    { crate = "getrandom@0.3.4", reason = "ahash in jsonschema transitively pulls old version" },
    { crate = "linux-raw-sys@0.4.15", reason = "follows rustix version split" },
    { crate = "rustix@0.38.44", reason = "crossterm 0.28 uses polling 3.x which needs old rustix" },
]
skip-tree = [
    { crate = "windows-sys@0.52.0", reason = "foundational Windows crate, version fragmentation unavoidable" },
    { crate = "windows-sys@0.59.0", reason = "foundational Windows crate, version fragmentation unavoidable" },
]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

### Dogfooding Spec (self-check.toml)

```toml
name = "self-check"
description = "Assay's own quality gates -- dogfooding spec"

[gate]
enforcement = "required"

[[criteria]]
name = "formatting"
description = "Code is formatted with rustfmt"
cmd = "cargo fmt --check"

[[criteria]]
name = "linting"
description = "No clippy warnings"
cmd = "cargo clippy --workspace -- -D warnings"

[[criteria]]
name = "tests"
description = "All tests pass"
cmd = "cargo test --workspace"

[[criteria]]
name = "deny"
description = "Dependency policies pass"
cmd = "cargo deny check"

[[criteria]]
name = "code-quality-review"
description = "Agent reviews code quality and architecture"
kind = "AgentReport"
enforcement = "advisory"
prompt = "Review the codebase for architectural issues, code smells, and potential improvements"
```

### Multi-Step Session Test Pattern

```rust
#[tokio::test]
async fn gate_lifecycle_run_report_finalize() {
    let dir = create_project(CONFIG);
    create_spec(dir.path(), "specs", "mixed.toml", MIXED_SPEC_WITH_AGENT);
    std::env::set_current_dir(dir.path()).unwrap();

    let server = AssayServer::new();

    // Step 1: gate_run creates session
    let run_result = server.gate_run(Parameters(GateRunParams {
        name: "mixed".to_string(),
        include_evidence: false,
        timeout: Some(10),
    })).await.unwrap();

    let run_json: serde_json::Value = serde_json::from_str(&extract_text(&run_result)).unwrap();
    let session_id = run_json["session_id"].as_str().unwrap().to_string();
    assert!(!session_id.is_empty());

    // Step 2: gate_report submits evaluation
    let report_result = server.gate_report(Parameters(GateReportParams {
        session_id: session_id.clone(),
        criterion_name: "code-review".to_string(),
        passed: true,
        evidence: "All good".to_string(),
        reasoning: "No issues found".to_string(),
        confidence: Some(Confidence::High),
        evaluator_role: EvaluatorRole::SelfEval,
    })).await.unwrap();

    assert!(report_result.is_error.is_none() || !report_result.is_error.unwrap());

    // Step 3: gate_finalize persists
    let finalize_result = server.gate_finalize(Parameters(GateFinalizeParams {
        session_id: session_id.clone(),
    })).await.unwrap();

    let fin_json: serde_json::Value = serde_json::from_str(&extract_text(&finalize_result)).unwrap();
    assert_eq!(fin_json["persisted"], true);
}
```

## Open Issue Audit Strategy

### Scale

123 open issues in `.planning/issues/open/`. Of these, ~25 are test-related (grep for "test" in filenames). The remaining ~98 are refactoring, docs, naming, and code cleanup issues that are OUT OF SCOPE per CONTEXT.md.

### Test-Related Issues (in scope)

Key test issues to address:

1. **Phase 3 gaps** (`2026-03-01-test-coverage-gaps-phase3.md`): 5 missing tests -- GateKind unknown variant, GateResult JSON roundtrip, Criterion deser failure, GateKind JSON roundtrip, Display format brittleness.

2. **Phase 6 gaps** (`2026-03-01-test-coverage-gaps-phase6.md`): 8 missing tests -- scan duplicate first-wins, scan empty dir, whitespace criterion name, multi-error criteria, duplicate criterion error detail, SpecError Display, empty description schema roundtrip, format_criteria_type ANSI padding.

3. **MCP handler coverage** (`2026-03-02-mcp-tool-handler-test-coverage.md`): Zero direct tests for tool handler methods. This is the primary TEST-01 deliverable.

4. **Individual test issues** (~22 files matching "test" pattern): These are specific test cases identified during PR reviews across phases 4-18. Each needs individual audit for staleness -- some may have been addressed by later phases.

### Audit Process

1. **Batch read** all 25 test-related issue files
2. **Cross-reference** each with current test code to check if already addressed
3. **Close stale** issues where later refactors or phases already cover the scenario
4. **Implement** remaining valid test cases as part of TEST-02 and TEST-03
5. **Close each** issue individually with resolution note

### Non-Test Issues (out of scope)

The remaining ~98 issues (doc comments, naming, refactoring, code cleanup) are explicitly excluded from Phase 19 per CONTEXT.md. They should remain open for future phases.

## Dependencies Between Deliverables

```
TEST-01 (MCP tests)  ─────────────────┐
TEST-02 (Phase 3/6 gap tests) ────────┤
TEST-03 (New feature tests) ──────────┤
                                       ├──→ TOOL-01/02 (cargo-deny tightening)
                                       │     No dependency on tests, but should
                                       │     run after to verify nothing breaks
                                       │
TOOL-03 (dogfooding spec) ────────────┘
  Depends on: .assay/ directory existing
  Depends on: TOOL-01/02 passing (cargo deny check in self-check.toml)
```

Suggested execution order:
1. TOOL-01 + TOOL-02 first (cargo-deny -- small, self-contained, unblocks TOOL-03)
2. Open issue triage (audit + close stale issues)
3. TEST-01 + TEST-02 + TEST-03 (bulk test writing)
4. TOOL-03 last (dogfooding spec -- depends on cargo-deny passing and tests existing)

## Dev-Dependencies Needed

| Crate | Purpose | Status |
|-------|---------|--------|
| `tempfile` | Temp dirs for integration tests | Already in assay-mcp dev-deps |
| `insta` | Snapshot testing for MCP responses | In workspace, needs adding to assay-mcp dev-deps |
| `tokio` (test feature) | `#[tokio::test]` macro | Already in workspace with `full` feature |
| `serde_json` | JSON parsing in test assertions | Already in assay-mcp deps |
| `serial_test` | Optional: serialize CWD-dependent tests | NOT in workspace -- add if needed, or use integration test file isolation |

**Recommendation:** Prefer integration test file isolation over adding `serial_test`. Each file in `crates/assay-mcp/tests/` runs in its own process, avoiding CWD races naturally. Reserve `serial_test` only if unit tests in server.rs need CWD access (unlikely given existing test patterns).
