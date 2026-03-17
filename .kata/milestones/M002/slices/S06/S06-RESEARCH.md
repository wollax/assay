# S06: MCP Tools & End-to-End Integration — Research

**Date:** 2026-03-17

## Summary

S06 is the capstone slice that wires together all M002 components into an end-to-end system. It has three work streams: (1) two new MCP tools (`orchestrate_run`, `orchestrate_status`) that expose multi-session orchestration over MCP, (2) CLI routing in `assay run` to detect multi-session manifests and dispatch to the orchestrator with post-execution merge, and (3) integration tests proving the full path from manifest load through DAG validation → parallel execution → scope-enforced harness config → sequential merge → status reporting.

The existing codebase is well-positioned. All building blocks are proven: `run_orchestrated()` (S02) handles parallel DAG-driven execution, `merge_completed_sessions()` (S03) handles sequential merge, `check_scope()`/`generate_scope_prompt()` (S05) handle scope enforcement, and the MCP server pattern (`#[tool_router]` + `spawn_blocking`) is established across 20 existing tools. The primary work is composition and routing — no new algorithms or data structures needed.

The primary risk is integration complexity: the orchestrator runs sync threads, merges happen on the base branch, and MCP tools need async wrappers around all of it. The status tool needs to read persisted state from `.assay/orchestrator/<run_id>/state.json` — this path is already established by `persist_state()` in executor.rs. The CLI routing decision (single vs multi-session) is straightforward: manifests with >1 session OR any `depends_on` declarations route to the orchestrator.

## Recommendation

Build S06 in three tasks:

**T01: MCP Tools** — Add `orchestrate_run` and `orchestrate_status` tools to `AssayServer`. `orchestrate_run` accepts a manifest path, loads it, detects multi-session, builds `OrchestratorConfig` + `PipelineConfig`, wraps `run_orchestrated()` in `spawn_blocking`, then calls `merge_completed_sessions()` with `default_conflict_handler()`, and returns a combined response with execution outcomes + merge report. `orchestrate_status` accepts a `run_id`, reads `.assay/orchestrator/<run_id>/state.json`, and returns the `OrchestratorStatus`. Both tools follow the existing pattern: param struct → `#[tool]` annotation → `spawn_blocking` wrapper → JSON response.

**T02: CLI Routing** — Modify `commands/run.rs` to detect multi-session manifests (sessions.len() > 1 or any depends_on) and route to `run_orchestrated()` + `merge_completed_sessions()` instead of `run_manifest()`. Single-session manifests continue using the existing `run_manifest()` path unchanged. The harness writer closure needs to be `Sync` for the orchestrator — wrap with `Arc` or construct per-thread. Add `--failure-policy` and `--merge-strategy` flags.

**T03: Integration Tests** — End-to-end tests exercising the real `assay run` CLI entrypoint (or direct function calls) with 3+ session manifests including mixed dependencies, intentional failures, and merge ordering verification. Tests use real git repos with mock session runners (not real agents). Verify: DAG validation errors, parallel execution with correct ordering, failure propagation + skip, sequential merge in topological order, scope prompt injection, status file persistence.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Parallel execution | `run_orchestrated()` in executor.rs | Proven with 18 tests including panic recovery, abort policy, state persistence |
| Sequential merge | `merge_completed_sessions()` in merge_runner.rs | Proven with 6 integration tests on real git repos |
| Merge ordering | `order_sessions()` in ordering.rs | CompletionTime + FileOverlap strategies with 8 tests |
| MCP tool registration | `#[tool_router]` macro + `#[tool]` attribute | Pattern used by all 20 existing tools |
| State persistence | `persist_state()` in executor.rs | Atomic tempfile-rename, already writes to `.assay/orchestrator/<run_id>/state.json` |
| Scope enforcement | `inject_scope_layer()` + `check_scope()` in assay-harness | 9 scope tests + 11 CLI tests |
| Conflict handler | `default_conflict_handler()` in merge_runner.rs | Returns Skip; AI handler deferred to M003 |

## Existing Code and Patterns

- `crates/assay-mcp/src/server.rs` — 20 existing MCP tools follow identical pattern: param struct with `Deserialize + JsonSchema`, `#[tool(description = "...")]` annotation, `resolve_cwd()` + `load_config()`, `spawn_blocking` for sync core calls. The `run_manifest` tool (line 2539) is the closest template for `orchestrate_run`. Response structs use `Serialize` with `skip_serializing_if` for optional fields. Errors go through `domain_error()` helper returning `CallToolResult` with `isError: true`.

- `crates/assay-mcp/src/lib.rs` — Re-exports param/response types under `#[cfg(any(test, feature = "testing"))]` for integration test access. New param structs (`OrchestrateRunParams`, `OrchestrateStatusParams`) need the same treatment.

- `crates/assay-core/src/orchestrate/executor.rs` — `run_orchestrated()` (line 143) is generic over `F: Fn(&ManifestSession, &PipelineConfig) -> Result<PipelineResult, PipelineError> + Sync`. The session runner closure must be `Sync` because it's called from `std::thread::scope` workers. In production, the closure captures `HarnessWriter` — but `HarnessWriter` is `dyn Fn` (not `Sync`). Per D035, the harness writer is captured inside the session runner closure. The MCP `run_manifest` tool already constructs the `Box<HarnessWriter>` inside `spawn_blocking` — the orchestrate tool will do the same, wrapping `run_session` with the harness writer in a closure.

- `crates/assay-core/src/orchestrate/merge_runner.rs` — `merge_completed_sessions()` (line 42) takes `Vec<CompletedSession>` + `MergeRunnerConfig` + conflict handler closure. `extract_completed_sessions()` (line 246) bridges `OrchestratorResult.outcomes` to `Vec<CompletedSession>`. The merge runner requires the working directory to be on the base branch with a clean tree. **Important:** after orchestrated execution, the main repo may not be on the base branch — need to `git checkout <base>` before merging.

- `crates/assay-cli/src/commands/run.rs` — Current `execute()` loads manifest, builds `PipelineConfig`, constructs `harness_writer` closure, calls `run_manifest()`. The multi-session path will parallel this but call `run_orchestrated()` + merge. The `harness_writer` is `Box<dyn Fn>` (not `Sync`) — need to construct a `Sync`-compatible version for the orchestrator. Since `assay_harness::claude::generate_config/write_config/build_cli_args` are all plain functions (no captured state), the closure itself is trivially `Sync` if we don't box it through `dyn`.

- `crates/assay-core/src/pipeline.rs` — `run_session()` (line 724) is the single-session primitive composed of `setup_session()` + `execute_session()`. `run_manifest()` (line 738) just maps over sessions sequentially. The orchestrator replaces `run_manifest` for multi-session cases but `run_session` remains the per-session unit.

- `crates/assay-mcp/tests/mcp_handlers.rs` — Integration tests create temp projects with `create_project()` + `create_spec()`, set CWD, call handler methods directly on `AssayServer`. Uses `#[serial]` for CWD isolation. New orchestration tests will follow this pattern but need git repo setup (like merge_runner tests).

## Constraints

- **Additive MCP tools only (D005/R031):** The 20 existing tools must remain unchanged. `orchestrate_run` and `orchestrate_status` are new additions in the `orchestrate_*` namespace.

- **Sync core (D007):** `run_orchestrated()` and `merge_completed_sessions()` are sync. MCP handlers wrap with `spawn_blocking`.

- **HarnessWriter is not Sync (D035):** The `HarnessWriter = dyn Fn(&HarnessProfile, &Path) -> Result<Vec<String>, String>` type alias is not `Sync`. For the orchestrator, the session runner closure must be `Sync`. Solution: don't pass `HarnessWriter` through `run_orchestrated()` — construct the harness writing logic directly in the session runner closure using plain function calls (not through the `dyn Fn` indirection).

- **Closure-based conflict handler (D026):** `merge_completed_sessions()` takes `Fn(&str, &[String], &ConflictScan, &Path) -> ConflictAction`. Use `default_conflict_handler()` for now.

- **Feature gate (D002/D033):** Orchestration code is behind `cfg(feature = "orchestrate")`. Both `assay-mcp` and `assay-cli` already enable this feature.

- **Backward compatibility:** Single-session manifests (sessions.len() == 1 with no depends_on) must continue using the existing `run_manifest()` path to avoid any behavior change.

- **Base branch checkout before merge:** After `run_orchestrated()`, the main repo working tree is at the project root but may not be on the desired base branch. The merge runner requires the base branch to be checked out. The CLI/MCP orchestration flow needs an explicit `git checkout <base_branch>` step between execution and merge.

## Common Pitfalls

- **HarnessWriter Sync boundary** — `run_orchestrated()` requires `F: Fn + Sync` for the session runner. If the session runner closure captures a `Box<dyn Fn>` (the HarnessWriter), it won't be `Sync`. **Avoidance:** Use plain function calls (`assay_harness::claude::generate_config` etc.) directly in the session runner closure instead of going through the `HarnessWriter` trait alias. This is what D035 recommends.

- **CWD races in MCP tests** — MCP handler tests use `set_current_dir()` which is process-global. Tests must use `#[serial]` to avoid races. Orchestration tests that create worktrees will also modify git state — need careful cleanup.

- **Merge runner expects clean base branch** — `merge_completed_sessions()` pre-flight checks for clean working tree and no MERGE_HEAD. After `run_orchestrated()` completes, need to verify the project root is on the base branch and clean before starting merges. Worktrees are separate directories, so the main repo should be clean — but verify.

- **Branch name population** — `extract_completed_sessions()` derives branch names from session names via `assay/<slug>` when `branch_name` is empty. In production, `SessionOutcome::Completed` should have real branch names from the worktree. The executor populates `branch_name` from the pipeline result — verify this is correct and matches the actual worktree branch.

- **Adapter selection for multi-agent** — The current `run_manifest` MCP tool and CLI hardcode Claude Code as the adapter. For multi-session orchestration, different sessions may target different adapters. The manifest `ManifestSession` doesn't currently have an adapter field. For S06, hardcode Claude Code adapter (same as current behavior). Multi-adapter per-session dispatch is a future concern.

- **Status tool timing** — `orchestrate_status` reads state from disk. If called while the orchestrator is running, it reads the last persisted snapshot. State is persisted after each session completion (not continuously), so there may be a brief window where in-flight sessions show as Pending. This is acceptable for S06.

## Open Risks

- **Real agent integration testing** — S06 integration tests will use mock session runners (not real Claude/Codex processes). The real end-to-end test with actual agents is a manual UAT concern, not automated. There's a gap between "orchestrator mechanics work" and "real agents produce valid output that merges cleanly."

- **Concurrent git operations during merge** — The merge runner runs sequentially after all parallel sessions complete, so there's no concurrency risk during merge itself. But if the user runs another git command while the merge runner is executing (e.g., from another terminal), it could interfere. Low probability, documented behavior.

- **State file read during active run** — `orchestrate_status` reads `.assay/orchestrator/<run_id>/state.json`. The executor writes this atomically (tempfile-rename), so partial reads shouldn't occur. But on some filesystems, rename is not atomic — extremely low risk.

- **Multi-adapter session runner** — Currently, the session runner closure hardcodes Claude Code adapter. When sessions need different adapters, the runner needs to dispatch based on a per-session adapter field. This is not in S06 scope but the closure design should not preclude it. Using plain function calls (not a captured `HarnessWriter`) makes this easier to extend later.

## Candidate Requirements

None — S06 delivers against existing R020 and R021.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust MCP (rmcp) | none found | The rmcp crate is niche; patterns are established in codebase |
| Rust concurrency | none relevant | Core uses std::thread::scope, no specialized skill needed |

No skills are relevant. All patterns are well-established in the existing codebase.

## Sources

- Existing codebase: `run_orchestrated()` signature and semantics from S02
- Existing codebase: `merge_completed_sessions()` contract from S03
- Existing codebase: MCP tool registration pattern from 20 existing tools in server.rs
- Existing codebase: CLI run command pattern from commands/run.rs
- S03-SUMMARY forward intelligence: merge runner operates on base branch, `extract_completed_sessions()` bridges types
- S05-SUMMARY forward intelligence: `inject_scope_layer()` adds scope prompt before adapter dispatch
