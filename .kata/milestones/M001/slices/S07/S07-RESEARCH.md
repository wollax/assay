# S07: End-to-End Pipeline — Research

**Date:** 2026-03-16

## Summary

S07 is the capstone slice that composes S04 (Claude adapter), S05 (worktree enhancements), and S06 (manifest parsing) into a single pipeline: `RunManifest → worktree create → harness config generate → agent launch → gate evaluate → merge propose`. The three active requirements are R017 (pipeline), R018 (MCP tool), and R019 (structured errors).

The codebase is well-prepared. All building blocks exist: `manifest::load()` for parsing, `worktree::create()` with session linkage and collision prevention, `claude::generate_config()` / `write_config()` / `build_cli_args()` for harness setup, `work_session::start_session()` for session lifecycle, `gate::evaluate_all()` for gate evaluation, and `merge::merge_check()` for merge readiness. The pipeline orchestrator is the only new logic — it sequences these calls and maps failures to structured pipeline-stage errors.

The primary risk is process lifecycle management for the Claude Code subprocess: timeout, crash recovery, exit code mapping, and zombie prevention. The evaluator module (`evaluator.rs`) provides a proven async subprocess pattern with timeout+kill that should be adapted for the harness launcher (but with different CLI args and longer timeouts). A secondary risk is that the pipeline is sync-core per D007, so the async subprocess launch must be wrapped appropriately.

## Recommendation

Build the pipeline in `assay-core::pipeline` as a new module (not the `orchestrate` module from D002 — that's for multi-agent M002). The pipeline module should contain:

1. **`PipelineStage` enum** — ManifestLoad, WorktreeCreate, HarnessConfig, AgentLaunch, GateEvaluate, MergePropose — for structured error context (R019).
2. **`PipelineError` struct** — wraps `AssayError` with stage context, recovery guidance, and elapsed time.
3. **`run_session()` function** — sync function executing one `ManifestSession` through the full pipeline. Takes project context (paths, config) and returns a `PipelineResult`.
4. **`run_manifest()` function** — iterates `RunManifest.sessions` calling `run_session()` for each (single-session for M001, but the loop is future-proof for M002).
5. **`launch_agent()` function** — sync subprocess launch with `std::process::Command`, timeout via thread-based wait, and structured exit code mapping. Follows evaluator pattern but adapted for harness (longer timeout, different args, CWD set to worktree root).

For the MCP tool (R018), add a single `run_manifest` tool that accepts a manifest path, calls `run_manifest()` via `spawn_blocking`, and returns structured JSON results. This follows the existing MCP pattern.

For the CLI (R017), add `assay run <manifest.toml>` as a new subcommand.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Subprocess timeout+kill | `evaluator.rs` spawn_and_collect pattern | Proven timeout → kill → collect pattern with exit code mapping |
| Session lifecycle management | `work_session::{start_session, record_gate_result, complete_session, abandon_session}` | Full phase state machine with audit trail already implemented |
| Worktree creation with collision check | `worktree::create()` with session_id param | Collision prevention and session linkage already built in S05 |
| Manifest loading+validation | `manifest::load()` | Full parse → validate → load pipeline with caret-pointer errors from S06 |
| Config generation | `claude::{generate_config, write_config, build_cli_args}` | Snapshot-locked adapter from S04 |
| Merge readiness check | `merge::merge_check()` | Zero-side-effect merge-tree based conflict detection |
| Settings merging | `harness::settings::merge_settings()` | Replace semantics for Vec fields, overlay for Option fields |
| Prompt building | `harness::prompt::build_prompt()` | Priority-sorted layer assembly |

## Existing Code and Patterns

- `crates/assay-core/src/evaluator.rs` (L455–540) — **Subprocess pattern reference.** `spawn_and_collect` uses `tokio::process::Command` with async stdin write, separate stdout/stderr tasks, and `tokio::time::timeout` wrapping `child.wait()` followed by `child.kill()` on timeout. The pipeline launcher should use the sync equivalent (`std::process::Command` + thread-based timeout per D007).
- `crates/assay-core/src/work_session.rs` — **Session lifecycle.** `start_session()` creates + transitions to AgentRunning + saves. `record_gate_result()` transitions to GateEvaluated. `complete_session()` and `abandon_session()` handle terminal states. Pipeline must call these in order.
- `crates/assay-core/src/worktree.rs` (L267) — **`create()` signature:** `(project_root, spec_slug, base_branch, worktree_base, specs_dir, session_id)`. Pipeline must pass `Some(&session.id)` for session linkage.
- `crates/assay-harness/src/claude.rs` — **Adapter API:** `generate_config(&HarnessProfile) -> ClaudeConfig`, `write_config(&ClaudeConfig, &Path) -> io::Result<()>`, `build_cli_args(&ClaudeConfig) -> Vec<String>`. **Critical:** `build_cli_args()` returns relative paths — CWD must be worktree root.
- `crates/assay-core/src/gate/mod.rs` (L105, L142) — **Gate evaluation.** `evaluate()` and `evaluate_all()` are sync. The MCP `gate_evaluate` handler wraps them with `spawn_blocking`. Pipeline can call `evaluate_all()` directly since the pipeline itself is sync.
- `crates/assay-mcp/src/server.rs` — **MCP tool pattern.** Each tool is an async method on `AssayServer` using `#[tool(...)]` attribute macro. Sync core calls wrapped in `tokio::task::spawn_blocking`. Params are `Parameters<T>` where T is a derive struct with `#[tool(param)]` attributes.
- `crates/assay-core/src/error.rs` — **Error pattern.** `AssayError` is `#[non_exhaustive]` with thiserror derives. New pipeline variants should follow existing patterns (operation + path + source for I/O, structured fields for domain errors).
- `crates/assay-cli/src/main.rs` — **CLI pattern.** Subcommands via clap derive. Each command delegates to a handler in `commands/`. The `run` subcommand should follow this pattern.
- `crates/assay-types/src/manifest.rs` — **ManifestSession fields.** `settings: Option<SettingsOverride>`, `hooks: Vec<HookContract>`, `prompt_layers: Vec<PromptLayer>`. Pipeline constructs `HarnessProfile` from these + defaults (per D014).

## Constraints

- **Sync core, async surfaces (D007):** The pipeline orchestration function must be sync. The agent subprocess launch uses `std::process::Command` (sync) with thread-based timeout, not `tokio::process::Command`. MCP handler wraps with `spawn_blocking`.
- **Zero-trait convention (D001, R009):** Pipeline uses plain functions and closures, not trait objects. No `Launcher` trait, no `Pipeline` trait.
- **MCP additive only (D005):** New `run_manifest` tool added alongside existing 18 tools. No modifications to existing tool signatures.
- **`deny_unknown_fields` on persisted types:** Any new pipeline result types that persist to disk must have this attribute.
- **`build_cli_args()` relative paths:** Must set CWD to worktree root when spawning the Claude process. The S04 summary explicitly flags this.
- **Feature gate (D002):** The orchestrate module should be feature-gated for rollback safety. However, S07 creates `pipeline` not `orchestrate` — the pipeline module is the M001 single-agent path. Feature gating is optional for M001 but should be considered.
- **Shell out to `claude` CLI (D008):** No library bindings. Use `std::process::Command::new("claude")`.

## Common Pitfalls

- **Forgetting CWD for claude subprocess** — `build_cli_args()` returns relative paths (`.mcp.json`, `.claude/settings.json`). If CWD isn't set to the worktree root, claude will fail to find its config. Set `.current_dir(&worktree_path)` on the Command.
- **Session lifecycle ordering** — Must follow Created → AgentRunning → GateEvaluated → Completed. Skipping phases (e.g., going straight to GateEvaluated) returns `WorkSessionTransition` error. Use the existing convenience functions (`start_session`, `record_gate_result`, `complete_session`).
- **Blocking the async runtime** — The pipeline function is sync and may run for minutes (agent execution). Must be wrapped in `spawn_blocking` from the MCP handler. Never call it directly from an async context.
- **Zombie processes on timeout** — When the agent subprocess times out, must `kill()` the child process before returning. The evaluator does this correctly; the pipeline launcher must do the same.
- **HarnessProfile construction from ManifestSession** — ManifestSession has inline optional overrides (D014), not an embedded HarnessProfile. Pipeline must construct HarnessProfile by: (1) loading spec for prompt layers, (2) merging session.settings with defaults via `merge_settings()`, (3) combining session.hooks and session.prompt_layers.
- **Worktree cleanup on pipeline failure** — If the pipeline fails after worktree creation but before completion, the worktree is left behind. The pipeline should abandon the session (not clean up the worktree) so the user can inspect the state. Cleanup is the user's responsibility via `assay worktree cleanup`.
- **Gate evaluation needs a Spec, not just criteria** — `evaluate_all()` takes a `&Spec`. Need to load the spec from the worktree's specs directory.

## Open Risks

- **Claude Code `--print` mode stability** — The `--print` flag and `--output-format json` are assumed stable from evaluator.rs usage. If Claude Code has changed its CLI interface since the evaluator was written, the launch will fail. Mitigation: the evaluator tests exercise the same flags, so if `just test` passes, the flags are still valid (assuming claude CLI is installed).
- **Process timeout for agent work** — The evaluator uses 120s default timeout for single-turn evaluation. Agent work (implementing a spec) could take 10–30 minutes. The pipeline needs a configurable timeout (from manifest or config) with a sensible default (e.g., 600s). `SettingsOverride.max_turns` exists but there's no explicit timeout field — may need to add one or derive from max_turns.
- **Gate evaluation in worktree context** — Gates need to run in the worktree directory (where the agent made changes), not the project root. Must pass `worktree_path` as `working_dir` to `evaluate_all()`.
- **Merge proposal semantics** — "Merge propose" in the roadmap description is ambiguous. For M001, this should be `merge_check()` (conflict detection) not actual merge execution. Actual merge is M002 (R023). The pipeline should report merge readiness, not perform the merge.
- **Agent launch without real Claude Code** — Integration tests can't easily spawn a real Claude Code agent. Unit tests should mock the subprocess (or test the pipeline stages individually). The E2E integration test is a manual verification step.
- **`--system-prompt` flag** — `build_cli_args()` doesn't include `--system-prompt` in its output — it expects CLAUDE.md file to serve as the system prompt. Verify this is the correct approach (CLAUDE.md is auto-loaded by Claude Code when present in CWD).

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | N/A | Core language — no skill needed |
| Claude Code CLI | N/A | Internal tool, no public skill exists |
| Git worktrees | N/A | Already well-handled by existing codebase |

No relevant professional skills found — this is domain-specific Rust infrastructure work using existing codebase patterns.

## Sources

- S04 summary forward intelligence: CWD must be worktree root for `build_cli_args()` relative paths
- S05 summary forward intelligence: `create()` takes `session_id: Option<&str>`, `detect_orphans()` available for pre-flight cleanup
- S06 summary forward intelligence: `manifest::load(path)` is the entry point, ManifestSession overrides are inline not embedded HarnessProfile
- D007: Sync launcher with `std::process::Command` + `spawn_blocking`
- D002: Orchestration in `assay-core` module (feature-gated), but S07 is single-agent pipeline, not full orchestration
- D014: ManifestSession uses inline optional fields, not embedded HarnessProfile
- Evaluator subprocess pattern: `crates/assay-core/src/evaluator.rs` L455–540
