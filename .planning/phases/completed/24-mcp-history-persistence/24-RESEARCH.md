# Phase 24 Research: MCP History Persistence Fix

## Problem Statement

The MCP `gate_run` handler (`crates/assay-mcp/src/server.rs:442-568`) does not persist history for command-only specs. When the spec contains agent criteria, a session is created and history is eventually saved via `gate_finalize` or session timeout. When the spec has no agent criteria (the `agent_info` is `None`), the handler returns the response without any `history::save()` call.

The CLI `handle_gate_run` (`crates/assay-cli/src/main.rs:1117-1223`) always calls `save_run_record()` after evaluation, regardless of whether criteria are agent or command-only.

## Standard Stack

- **History API**: `assay_core::history::save(assay_dir, &record, max_history)` — the single save entry point. Confidence: HIGH.
- **Record construction**: `GateRunRecord` from `assay_types` with fields `run_id`, `assay_version`, `timestamp`, `working_dir`, `summary`. Confidence: HIGH.
- **Run ID generation**: `assay_core::history::generate_run_id(&timestamp)` — generates `YYYYMMDDTHHMMSSZ-<6hex>`. Confidence: HIGH.
- **Test framework**: `tokio::test` + `serial_test::serial` + `tempfile::tempdir()` in `crates/assay-mcp/tests/mcp_handlers.rs`. Confidence: HIGH.

## Architecture Patterns

### CLI save pattern (the reference implementation)

The CLI uses a `save_run_record()` helper at `crates/assay-cli/src/main.rs:1247-1274`:

```rust
fn save_run_record(
    assay_dir: &Path,
    name: &str,
    working_dir: &Path,
    summary: GateRunSummary,
    max_history: Option<usize>,
    suppress_prune_msg: bool,
) {
    let timestamp = chrono::Utc::now();
    let run_id = assay_core::history::generate_run_id(&timestamp);
    let record = GateRunRecord {
        run_id,
        assay_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp,
        working_dir: Some(working_dir.display().to_string()),
        summary,
    };
    match assay_core::history::save(assay_dir, &record, max_history) { ... }
}
```

### MCP gate_run handler flow (current)

1. Resolve CWD, load config, load spec entry
2. Extract `agent_info` (pre-move)
3. `spawn_blocking` → `evaluate_all` / `evaluate_all_gates` → returns `GateRunSummary`
4. `format_gate_response(&summary, include_evidence)` → `GateRunResponse`
5. **If agent_info is Some**: create session, store it, spawn timeout task (which saves on timeout)
6. **If agent_info is None**: nothing — no history save ← THE BUG
7. Serialize response and return

### Where to insert the fix

After step 4 (after `format_gate_response`) and before the agent_info branch, add a history save for the command-only case. Specifically, when `agent_info.is_none()`, construct a `GateRunRecord` from the `summary` and call `history::save()`.

The fix should go in the `else` branch of the `if let Some(info) = agent_info` block (line 506), or as a separate block before that branch when `agent_info.is_none()`.

Key variables available at the save point:
- `cwd` — project root (`PathBuf`)
- `config` — loaded `Config` (has `config.gates.as_ref().and_then(|g| g.max_history)`)
- `working_dir` — resolved working directory (`PathBuf`)
- `summary` — the `GateRunSummary` (still available, only borrowed by `format_gate_response`)

The `assay_dir` is `cwd.join(".assay")` — note this is already computed inside the agent_info branch (line 528) but would need to be computed earlier or in the new else branch.

### MCP test pattern

Tests in `crates/assay-mcp/tests/mcp_handlers.rs` follow this pattern:

1. `create_project(config_toml)` → `tempfile::TempDir` with `.assay/config.toml`
2. `create_spec(dir.path(), filename, content)` → writes spec to `.assay/specs/`
3. `std::env::set_current_dir(dir.path())` — required because MCP resolves CWD
4. `AssayServer::new()` → stateless server (sessions are internal `Arc<Mutex<HashMap>>`)
5. Call handler methods directly: `server.gate_run(Parameters(GateRunParams { ... })).await`
6. Parse JSON from `extract_text(&result)` and assert
7. Tests use `#[serial]` because of CWD mutation

Confidence: HIGH.

## Don't Hand-Roll

- **Run ID generation**: Use `assay_core::history::generate_run_id()`. Do not generate custom IDs.
- **Record construction**: Build `GateRunRecord` using the same pattern as the CLI helper. Do not invent a new structure.
- **Atomic writes**: `history::save()` handles atomic temp-file-then-rename. Do not implement custom file persistence.
- **Test project setup**: Use existing `create_project()` and `create_spec()` helpers from `mcp_handlers.rs`.

## Common Pitfalls

1. **Moving `summary` before the save**: The `summary` is used by `format_gate_response()` (which borrows it) and then by the agent session branch (which clones `summary.results`). The save for command-only specs must use `summary.clone()` or happen before `format_gate_response` consumes any ownership. Currently `format_gate_response` takes `&summary` (borrow), so the summary is still available. Confidence: HIGH.

2. **`assay_dir` scoping**: In the current code, `let assay_dir = cwd.join(".assay")` is computed inside the `if let Some(info) = agent_info` block. The command-only save path needs its own `assay_dir` computation. Consider hoisting `assay_dir` above both branches. Confidence: HIGH.

3. **`max_history` extraction**: Already done at line 529 as `config.gates.as_ref().and_then(|g| g.max_history)` inside the agent branch. Same extraction needed for the command-only path. Confidence: HIGH.

4. **Error handling**: The CLI treats save failures as non-fatal warnings (prints to stderr). The MCP handler should use `tracing::warn!` for save failures (consistent with the timeout save at line 546). Do not return an `McpError` for save failures — the gate evaluation succeeded. Confidence: HIGH.

5. **`working_dir` field**: The `GateRunRecord.working_dir` is `Option<String>`. Use `Some(working_dir.to_string_lossy().to_string())` matching the pattern at line 530. Confidence: HIGH.

6. **`env!("CARGO_PKG_VERSION")`**: This macro resolves to the version of the crate being compiled. In `assay-mcp`, this will be the MCP crate version, not assay-core's. This is consistent with how the CLI uses it (CLI's own version). Confidence: HIGH.

7. **Test thread serialization**: Integration tests that set CWD must use `#[serial]` from `serial_test`. Confidence: HIGH.

## Code Examples

### Fix: Save history for command-only specs in gate_run

Insert after `format_gate_response` and restructure the agent_info branch:

```rust
// Persist history for command-only specs (no agent criteria).
// Specs with agent criteria are persisted via gate_finalize or session timeout.
if agent_info.is_none() {
    let assay_dir = cwd.join(".assay");
    let max_history = config.gates.as_ref().and_then(|g| g.max_history);
    let timestamp = Utc::now();
    let run_id = assay_core::history::generate_run_id(&timestamp);
    let record = assay_types::GateRunRecord {
        run_id,
        assay_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp,
        working_dir: Some(working_dir.to_string_lossy().to_string()),
        summary: summary.clone(),
    };
    if let Err(e) = assay_core::history::save(&assay_dir, &record, max_history) {
        tracing::warn!(
            spec_name = %record.summary.spec_name,
            "failed to save command-only gate run history: {e}"
        );
    }
}
```

### Integration test: command-only spec history persistence

```rust
#[tokio::test]
#[serial]
async fn gate_run_command_only_persists_history() {
    let dir = create_project(r#"project_name = "cmd-history-test""#);
    create_spec(
        dir.path(),
        "cmd-spec.toml",
        r#"
name = "cmd-spec"
description = "Command-only spec"

[[criteria]]
name = "echo-check"
description = "Echo passes"
cmd = "echo ok"
"#,
    );

    std::env::set_current_dir(dir.path()).unwrap();
    let server = AssayServer::new();

    // Run the gate
    let result = server
        .gate_run(Parameters(GateRunParams {
            name: "cmd-spec".to_string(),
            include_evidence: false,
            timeout: Some(30),
        }))
        .await
        .unwrap();

    assert!(!result.is_error.unwrap_or(false));
    let run_json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
    assert_eq!(run_json["passed"], 1);
    assert!(run_json["session_id"].is_null(), "command-only spec should not have session_id");

    // Verify history was persisted
    let history_result = server
        .gate_history(Parameters(GateHistoryParams {
            name: "cmd-spec".to_string(),
            run_id: None,
            limit: None,
        }))
        .await
        .unwrap();

    assert!(!history_result.is_error.unwrap_or(false));
    let history_json: serde_json::Value = serde_json::from_str(&extract_text(&history_result)).unwrap();
    assert_eq!(history_json["total_runs"], 1, "should have exactly one history entry");

    let runs = history_json["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["passed"], 1);
    assert_eq!(runs[0]["failed"], 0);
}
```

## Key File Paths

| File | Role |
|------|------|
| `crates/assay-mcp/src/server.rs:442-568` | MCP `gate_run` handler (fix location) |
| `crates/assay-cli/src/main.rs:1247-1274` | CLI `save_run_record` helper (reference pattern) |
| `crates/assay-core/src/history/mod.rs:109-170` | `history::save()` API |
| `crates/assay-core/src/history/mod.rs:47-61` | `generate_run_id()` API |
| `crates/assay-types/src/gate_run.rs:72-84` | `GateRunRecord` struct |
| `crates/assay-mcp/tests/mcp_handlers.rs` | Existing integration tests (test pattern + helpers) |

## Scope Constraints

- The fix is a single `if agent_info.is_none()` block (~12 lines) in `server.rs`
- One integration test (~40 lines) in `mcp_handlers.rs`
- No new files, no new dependencies, no API changes
- No changes to `assay-core` or `assay-types`
