---
id: M001
provides:
  - Full single-agent pipeline (manifest → worktree → harness config → agent launch → gate evaluate → merge propose)
  - assay-harness crate with Claude Code adapter (generate_config, write_config, build_cli_args)
  - HarnessProfile type system (6 types) with prompt builder and settings merger
  - GateEvalContext persistence with write-through cache surviving MCP restarts
  - Worktree session linkage, orphan detection, and collision prevention
  - RunManifest TOML parsing with [[sessions]] array format and caret-pointer error diagnostics
  - CLI `assay run <manifest.toml>` subcommand with --timeout, --json, --base-branch flags
  - MCP `run_manifest` tool with spawn_blocking wrapper
  - PipelineStage/PipelineError/PipelineResult types with structured error handling
  - HarnessWriter dependency-injection pattern for harness adapters
key_decisions:
  - "D001: Closures/callbacks for control inversion (zero-trait convention preserved)"
  - "D003: assay-harness as leaf crate (implementations depend on core, not vice versa)"
  - "D004: TOML with [[sessions]] array (forward-compatible for multi-agent)"
  - "D005: MCP tools additive only (never modify existing signatures)"
  - "D006: AgentSession → GateEvalContext vocabulary cleanup"
  - "D007: Sync launcher with spawn_blocking (async deferred to M002)"
  - "D009: JSON file-per-record persistence (consistent with existing pattern)"
  - "D014: ManifestSession uses inline optional overrides, not embedded HarnessProfile"
  - "D015: HarnessWriter function parameter for dependency inversion (assay-core independent of assay-harness)"
  - "D016: PipelineError wraps String, not AssayError (AssayError is not Clone)"
patterns_established:
  - "HarnessWriter type alias Fn(&HarnessProfile, &Path) -> Result<Vec<String>, String> for adapter injection"
  - "Write-through cache pattern: HashMap + disk persistence, disk fallback on cache miss"
  - "Prompt layers formatted as ## {name}\\n\\n{content} joined by \\n\\n---\\n\\n"
  - "Replace semantics for Vec fields in settings merger (non-empty override wins entirely)"
  - "Explicit struct construction (no ..base) for compile-time field coverage safety"
  - "Stage-tagged structured errors with recovery guidance strings"
  - "Concrete adapter composition at call site (CLI and MCP wire claude functions into closures)"
  - "BTreeMap for deterministic JSON key ordering in generated harness config"
observability_surfaces:
  - "PipelineStage enum + PipelineError.recovery — primary failure signal for any pipeline stage"
  - "CLI --json returns structured RunResponse with per-session outcomes and stage timings"
  - "CLI exit codes (0=success, 1=pipeline error, 2=gate/merge failure) for automation"
  - "MCP run_manifest returns structured JSON with isError flag"
  - ".assay/gate_sessions/*.json — persisted in-progress gate evaluation sessions"
  - "detect_orphans() identifies worktrees with no active session linked"
  - "WorktreeCollision error with spec_slug and existing_path for collision diagnosis"
  - "ManifestParse errors include file path + caret-pointer line/column display"
  - "ManifestValidation errors list all issues at once with field paths"
  - "12 insta snapshot files in assay-harness lock Claude Code config format"
requirement_outcomes:
  - id: R001
    from_status: active
    to_status: validated
    proof: "S01 — persistence round-trip test, write-through cache in MCP server, disk fallback in gate_finalize, 19 gate session tests pass"
  - id: R002
    from_status: active
    to_status: validated
    proof: "S01 — rg 'AgentSession' --type rust crates/ returns zero matches, schema snapshot updated to gate-eval-context"
  - id: R003
    from_status: active
    to_status: validated
    proof: "S02 — cargo build -p assay-harness compiles with correct dependency edges, workspace dep in root Cargo.toml"
  - id: R004
    from_status: active
    to_status: validated
    proof: "S02 — 6 types with full derives, deny_unknown_fields, inventory registration, 6 schema snapshots locked"
  - id: R005
    from_status: active
    to_status: validated
    proof: "S03 — build_prompt() with 7 unit tests covering priority ordering, stability, empty-layer filtering"
  - id: R006
    from_status: active
    to_status: validated
    proof: "S03 — merge_settings() with 6 unit tests covering overlay, replace, and preservation semantics"
  - id: R007
    from_status: active
    to_status: validated
    proof: "S03 — 4 tests validate HookContract/HookEvent construction and JSON round-trip; S04 translates to Claude Code hooks.json format"
  - id: R008
    from_status: active
    to_status: validated
    proof: "S04 — generate_config, write_config, build_cli_args locked by 12 insta snapshots and 6 file/args tests"
  - id: R009
    from_status: active
    to_status: validated
    proof: "S04 — all adapter functions are plain functions, zero traits in codebase; S07 uses HarnessWriter closure, not trait"
  - id: R010
    from_status: active
    to_status: validated
    proof: "S05 — detect_orphans() with 4 unit tests covering no-session, active, terminal, and missing-session classification"
  - id: R011
    from_status: active
    to_status: validated
    proof: "S05 — collision check in create() with WorktreeCollision error, 3 unit tests covering active/terminal/no-existing scenarios"
  - id: R012
    from_status: active
    to_status: validated
    proof: "S05 — session_id: Option<String> on WorktreeMetadata with serde defaults, deny_unknown_fields, schema snapshot, round-trip test"
  - id: R013
    from_status: active
    to_status: validated
    proof: "S05 — zero eprintln in worktree.rs, zero detect_main_worktree refs, schema snapshots for WorktreeInfo/WorktreeStatus, 3 new edge-case tests"
  - id: R014
    from_status: active
    to_status: validated
    proof: "S06 — RunManifest/ManifestSession types with schema snapshots, TOML round-trip tests pass"
  - id: R015
    from_status: active
    to_status: validated
    proof: "S06 — from_str/validate/load with 13 tests covering round-trip, unknown fields, caret-pointer errors, field-level validation"
  - id: R016
    from_status: active
    to_status: validated
    proof: "S06 — all test fixtures use [[sessions]] array syntax; Vec<ManifestSession> type enforces it"
  - id: R017
    from_status: active
    to_status: validated
    proof: "S07 — run_session() orchestrates 6-stage pipeline with 18 pipeline tests; CLI and MCP entry points compile and pass; just ready green"
  - id: R018
    from_status: active
    to_status: validated
    proof: "S07 — run_manifest MCP tool registered in router, param schema correct, spawn_blocking wrapping verified, 5 MCP tests pass"
  - id: R019
    from_status: active
    to_status: validated
    proof: "S07 — PipelineError carries stage, message, recovery, elapsed at every failure point; tests verify stage-tagged errors for SpecLoad, WorktreeCreate, AgentLaunch"
duration: ~2.5h
verification_result: passed
completed_at: 2026-03-16
---

# M001: Single-Agent Harness End-to-End

**Full single-agent pipeline from TOML manifest through worktree creation, Claude Code harness config generation, agent launch, gate evaluation, to merge proposal — with structured errors, session persistence, and worktree lifecycle management.**

## What Happened

Seven slices built the complete pipeline bottom-up over ~2.5 hours:

**Foundation (S01, S05):** S01 renamed AgentSession → GateEvalContext across all crates and added write-through disk persistence with MCP restart survival. S05 enhanced worktrees with session linkage (`session_id` on WorktreeMetadata), orphan detection (`detect_orphans()`), collision prevention (`WorktreeCollision` error in `create()`), and resolved all 15 tracked tech debt items including `eprintln` → `tracing::warn`, missing schema registry entries, and edge-case test coverage.

**Type System (S02, S06):** S02 scaffolded the `assay-harness` crate and defined the 6 HarnessProfile types (`HarnessProfile`, `PromptLayer`, `PromptLayerKind`, `SettingsOverride`, `HookContract`, `HookEvent`) with full derives and schema snapshots. S06 added `RunManifest` and `ManifestSession` types with TOML `[[sessions]]` parsing, semantic validation collecting all errors in one pass, and caret-pointer diagnostics for parse errors.

**Harness Logic (S03, S04):** S03 implemented the prompt builder (`build_prompt()` with priority ordering and empty-layer filtering) and settings merger (`merge_settings()` with replace/overlay semantics). S04 built the Claude Code adapter — `generate_config()` translates HarnessProfile to Claude Code artifacts (CLAUDE.md, .mcp.json, settings.json, hooks.json), `write_config()` writes them to a worktree, and `build_cli_args()` produces the `claude --print` CLI invocation — all locked by 12 insta snapshots.

**Pipeline (S07):** The capstone slice composed everything into a 6-stage orchestrator (`run_session()`: SpecLoad → WorktreeCreate → HarnessConfig → AgentLaunch → GateEvaluate → MergeCheck) with `PipelineStage`/`PipelineError`/`PipelineResult` types. A key architectural insight: `assay-core` cannot depend on `assay-harness` (reverse dependency direction), so the pipeline accepts a `HarnessWriter` closure parameter — concrete Claude adapter composition happens at CLI/MCP call sites. The `assay run <manifest.toml>` CLI subcommand and `run_manifest` MCP tool both wire the full pipeline.

## Cross-Slice Verification

**Success Criterion: A TOML manifest can drive the full single-agent pipeline end-to-end**
→ Verified: `assay run <manifest.toml>` CLI subcommand and `run_manifest` MCP tool both implemented and tested. Pipeline tests exercise manifest → spec → worktree → harness → agent → gate → merge flow. `cargo run --bin assay -- run --help` displays correct usage.

**Success Criterion: Claude Code is launched in an isolated worktree with generated CLAUDE.md, .mcp.json, settings, and hooks**
→ Verified: S04 adapter generates all config files from HarnessProfile, locked by 12 insta snapshots. `write_config()` creates CLAUDE.md, .mcp.json, and .claude/settings.json in target directory (verified by tempfile tests). `build_cli_args()` produces correct `--print --output-format json --mcp-config --settings` flags.

**Success Criterion: Pipeline failures at any stage produce structured errors with recovery guidance**
→ Verified: `PipelineError` struct carries `stage: PipelineStage`, `message: String`, `recovery: String`, `elapsed: Duration`. 18 pipeline tests cover stage display, error construction, and failure paths for SpecLoad, WorktreeCreate, and AgentLaunch stages.

**Success Criterion: Worktrees are linked to sessions with orphan detection and collision prevention**
→ Verified: `session_id: Option<String>` on WorktreeMetadata (S05), `detect_orphans()` with 4 classification tests, `WorktreeCollision` error with 3 scenario tests. Schema snapshot updated for WorktreeMetadata.

**Success Criterion: GateEvalContext persists to disk, surviving MCP restarts**
→ Verified: Write-through cache in MCP server (S01) — `gate_run`/`gate_report` save after HashMap mutation, `gate_finalize` falls back to disk load when session not in HashMap. 19 gate session tests pass including round-trip persistence.

**Milestone Definition of Done:**
- ✅ All 7 slices complete with passing verification (all slice summaries document `verification_result: passed`)
- ✅ E2E pipeline exercised with automated tests (18 pipeline + 5 MCP + 4 CLI tests); real Claude Code runtime invocation is manual UAT
- ✅ Generated harness config valid (12 insta snapshots lock CLAUDE.md, .mcp.json, settings.json, hooks.json formats)
- ✅ Pipeline errors structured with stage context at every failure point (PipelineError tested for 3 failure stages)
- ✅ `just ready` passes — 991 tests, 0 failures, fmt/clippy/deny all clean

## Requirement Changes

All 19 requirements (R001–R019) transitioned from active to validated during this milestone:

- R001: active → validated — GateEvalContext persistence with write-through cache and disk fallback (S01)
- R002: active → validated — AgentSession → GateEvalContext rename, zero matches remaining (S01)
- R003: active → validated — assay-harness crate compiles with correct dependency edges (S02)
- R004: active → validated — HarnessProfile type system with 6 types and schema snapshots (S02)
- R005: active → validated — build_prompt() with 7 unit tests (S03)
- R006: active → validated — merge_settings() with 6 unit tests (S03)
- R007: active → validated — HookContract/HookEvent types validated by construction + S04 adapter translation (S03, S04)
- R008: active → validated — Claude Code adapter with 12 snapshots and 6 file/args tests (S04)
- R009: active → validated — All adapter and pipeline functions are plain functions, zero traits (S04, S07)
- R010: active → validated — detect_orphans() with 4 classification tests (S05)
- R011: active → validated — WorktreeCollision in create() with 3 scenario tests (S05)
- R012: active → validated — session_id on WorktreeMetadata with schema snapshot (S05)
- R013: active → validated — 15 tech debt items resolved or explicitly deferred (S05)
- R014: active → validated — RunManifest type with schema snapshots and TOML round-trip (S06)
- R015: active → validated — Manifest parsing with 13 tests and caret-pointer errors (S06)
- R016: active → validated — [[sessions]] array format enforced by type system (S06)
- R017: active → validated — 6-stage pipeline with 18 tests, CLI and MCP entry points (S07)
- R018: active → validated — run_manifest MCP tool with 5 tests (S07)
- R019: active → validated — PipelineError with stage, message, recovery, elapsed (S07)

## Forward Intelligence

### What the next milestone should know
- The HarnessWriter closure pattern (`Fn(&HarnessProfile, &Path) -> Result<Vec<String>, String>`) is the extension point for new harness adapters. Adding Codex or OpenCode only requires a new closure at CLI/MCP call sites — no pipeline changes.
- `assay-core` cannot depend on `assay-harness` — this is load-bearing for the dependency graph. Any harness-specific logic must stay in assay-harness or be injected via closures.
- ManifestSession overrides (settings, hooks, prompt_layers) are inline optional fields, not a nested HarnessProfile. The pipeline's `build_harness_profile()` constructs HarnessProfile from these fields plus defaults.
- The `[[sessions]]` array format is already multi-session capable — M002 just needs to add parallel execution to `run_manifest()`.

### What's fragile
- `launch_agent()` timeout: child process ownership transfers to a wait thread, preventing explicit kill. Brief zombie window exists. Consider `shared_child` crate or `Arc<Mutex<Child>>` for M002 multi-agent where orphan accumulation matters.
- `build_cli_args()` uses relative paths (`.mcp.json`, `.claude/settings.json`), assuming CWD is worktree root. The pipeline must set CWD correctly — if this invariant breaks, Claude Code gets wrong config paths.
- Snapshot tests lock Claude Code's current config format — if Claude Code changes CLI flags or config schemas, snapshots break. This is intentional early-warning, not a bug.
- Pre-existing flaky tests: `session_create_happy_path` and two `set_current_dir` tests in assay-mcp occasionally fail under parallel execution (race conditions, not introduced by M001).

### Authoritative diagnostics
- `cargo insta test -p assay-harness` — detects any Claude Code config format drift via snapshot diffs
- `cargo insta test -p assay-types` — detects any type schema drift across all 38+ snapshot tests
- `cargo test -p assay-core -- pipeline` — 18 tests validate the full pipeline orchestration logic
- `PipelineError.stage` + `PipelineError.recovery` — primary signals for any runtime pipeline failure
- `.assay/gate_sessions/` directory — inspect persisted in-progress gate evaluation sessions

### What assumptions changed
- Plan assumed `--permission-mode` and `--allowed-tools` were valid Claude Code CLI flags — actual flags are `--system-prompt`, `--model`, `--mcp-config`, `--settings` (discovered in S04)
- Plan assumed ManifestLoad as a pipeline stage — manifest loading happens before the pipeline, so SpecLoad is the actual first stage (discovered in S07)
- Plan assumed direct assay-harness calls in pipeline — dependency direction requires HarnessWriter closure injection instead (architecturally cleaner, discovered in S07)

## Files Created/Modified

### New crate
- `crates/assay-harness/` — New workspace leaf crate with Claude Code adapter, prompt builder, settings merger

### Types (assay-types)
- `crates/assay-types/src/harness.rs` — HarnessProfile, PromptLayer, PromptLayerKind, SettingsOverride, HookContract, HookEvent
- `crates/assay-types/src/manifest.rs` — RunManifest, ManifestSession
- `crates/assay-types/src/session.rs` — GateEvalContext (renamed from AgentSession)
- `crates/assay-types/src/worktree.rs` — session_id field, WorktreeConfig::as_path(), inventory entries
- `crates/assay-types/tests/snapshots/` — 10+ new/updated schema snapshot files

### Core logic (assay-core)
- `crates/assay-core/src/pipeline.rs` — Pipeline orchestrator with 6 stages, PipelineError, run_session, run_manifest, launch_agent
- `crates/assay-core/src/manifest.rs` — from_str, validate, load with ManifestParse/ManifestValidation errors
- `crates/assay-core/src/gate/session.rs` — save_context, load_context, list_contexts persistence
- `crates/assay-core/src/worktree.rs` — session linkage, detect_orphans, collision prevention, tech debt fixes
- `crates/assay-core/src/error.rs` — GateEvalContextNotFound, WorktreeCollision, ManifestParse, ManifestValidation variants

### Harness (assay-harness)
- `crates/assay-harness/src/claude.rs` — ClaudeConfig, generate_config, write_config, build_cli_args
- `crates/assay-harness/src/prompt.rs` — build_prompt with priority ordering
- `crates/assay-harness/src/settings.rs` — merge_settings with replace/overlay semantics
- `crates/assay-harness/src/snapshots/` — 12 insta snapshot files for Claude Code config

### Entry points
- `crates/assay-cli/src/commands/run.rs` — CLI `assay run` subcommand
- `crates/assay-mcp/src/server.rs` — run_manifest MCP tool, write-through persistence in gate handlers
