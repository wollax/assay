# S04: Gate-Gated PR Workflow

**Goal:** `assay pr create <milestone>` opens a real GitHub PR only when all chunk gates pass. Failing chunks are listed with their failed criteria. PR number and URL are stored in the milestone file.
**Demo:** A developer runs `assay pr create my-feature` against a milestone where all chunks have passing gates. A real GitHub PR is created. The milestone TOML is updated with the PR number and URL. Running the same command again returns "PR already created: #42". If any chunk's required gates fail, the command exits 1 with a structured list of failing chunks.

## Must-Haves

- `Milestone` type has `pr_number: Option<u64>` and `pr_url: Option<String>` fields; all workspace tests still pass after the type change
- `pr_check_milestone_gates` returns `Ok(vec![])` when all chunk required gates pass; returns `Ok(failures)` when any chunk's required gates fail; returns `Err` only for I/O errors
- `pr_create_if_gates_pass` creates a PR via `gh pr create --json number,url` when all gates pass; returns `PrCreateResult { pr_number, pr_url }`
- `pr_create_if_gates_pass` returns a structured error listing failing chunk slugs when gates fail; does not touch `gh`
- `pr_create_if_gates_pass` returns an actionable error ("gh CLI not found — install from https://cli.github.com") when `gh` is not on PATH
- Successful PR creation saves `pr_number` and `pr_url` to the milestone TOML atomically
- If milestone is in `Verify` status at PR creation time, it transitions to `Complete` and is saved
- `pr_create_if_gates_pass` returns error "PR already created: #N — <url>" when `milestone.pr_number` is already set
- `assay pr create <milestone>` CLI subcommand exists; prints "PR created: #N — <url>" on success; exits 1 on failure
- `pr_create` MCP tool is registered in the router; presence test passes
- `cargo test --workspace` green; 8 new integration tests in `crates/assay-core/tests/pr.rs` all pass

## Proof Level

- This slice proves: integration — `pr_check_milestone_gates` and `pr_create_if_gates_pass` exercised with real gate evaluation and a mock `gh` binary; CLI and MCP wiring proven by presence tests and CLI unit tests
- Real runtime required: no — mock `gh` binary used in integration tests; actual GitHub PR creation is UAT only
- Human/UAT required: yes — actual `gh pr create` against a real GitHub repo must be verified manually; gate-check pass/fail logic is proven programmatically

## Verification

- `cargo test -p assay-core --features assay-types/orchestrate --test pr` → 8 tests pass
- `cargo test -p assay-cli -- pr` → CLI presence and unit tests pass
- `cargo test -p assay-mcp -- pr_create` → MCP presence test passes
- `cargo test --workspace` → all workspace tests green (≥1325 expected)
- `cat .assay/milestones/<slug>.toml` after mock-gh test → contains `pr_number = 42` and `pr_url = "..."`
- `cargo check --workspace` → zero compile errors after Milestone struct change

## Observability / Diagnostics

- Runtime signals: `AssayError::Io` with `operation: "pr_create_if_gates_pass"` and milestone slug as path on all failure paths; `ChunkGateFailure { chunk_slug, required_failed }` list for gate failures; `gh` stderr echoed for `gh`-specific errors
- Inspection surfaces: `cat .assay/milestones/<slug>.toml` shows `pr_number`/`pr_url` after success; `assay milestone list` shows milestone status; `assay pr create --help` shows usage
- Failure visibility: gate failures list chunk slugs + required_failed counts; gh not found distinguishes NotFound spawn error from non-zero exit; "PR already created: #N" prevents duplicate creation
- Redaction constraints: none (no secrets in PR workflow; gh uses ambient git/gh auth)

## Integration Closure

- Upstream surfaces consumed:
  - `assay_core::milestone::{milestone_load, milestone_save}` — load milestone for check + save after PR creation
  - `assay_core::milestone::cycle::{active_chunk, milestone_phase_transition}` — detect Verify→Complete transition
  - `assay_core::gate::evaluate_all_gates` — gate evaluation per chunk
  - `assay_core::spec::load_spec_entry_with_diagnostics` — load spec for each chunk
  - `assay_types::{Milestone, MilestoneStatus}` — type access
- New wiring introduced in this slice:
  - `assay-core::pr` module with `pr_check_milestone_gates` + `pr_create_if_gates_pass` (new public functions)
  - `assay-cli::commands::pr` with `PrCommand::Create` dispatching to `pr_create_if_gates_pass`
  - `assay-mcp::server` `pr_create` tool method with `spawn_blocking` wrapping
  - `Milestone` type gains `pr_number`/`pr_url` fields (schema snapshot updated)
- What remains before the milestone is truly usable end-to-end: S05 (Claude Code plugin) and S06 (Codex plugin) consume the `pr_create` MCP tool via skills

## Tasks

- [x] **T01: Extend Milestone type, update literals, write failing integration tests** `est:45m`
  - Why: Provides the `pr_number`/`pr_url` type contract, updates all struct literals (compile safety), and writes the full `tests/pr.rs` test suite before the implementation exists (test-first discipline)
  - Files: `crates/assay-types/src/milestone.rs`, `crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap`, `crates/assay-core/tests/milestone_io.rs`, `crates/assay-core/tests/cycle.rs`, `crates/assay-core/tests/pr.rs` (new)
  - Do: Add `pr_number: Option<u64>` and `pr_url: Option<String>` to `Milestone` with `#[serde(default, skip_serializing_if = "Option::is_none")]`; insert before `created_at`/`updated_at`. Update `milestone_toml_roundtrip` and `milestone_minimal_toml_roundtrip` struct literals in `milestone.rs`. Update `make_milestone` in `milestone_io.rs` (add `pr_number: None, pr_url: None`). Update `make_milestone_with_status` and any inline `Milestone { ... }` literals in `cycle.rs` (add `pr_number: None, pr_url: None`). Run `INSTA_UPDATE=always cargo test -p assay-types` to regenerate the milestone schema snapshot. Write `crates/assay-core/tests/pr.rs` with 8 tests that import `assay_core::pr::{pr_check_milestone_gates, pr_create_if_gates_pass}` — this file will fail to compile until T02 since `assay_core::pr` doesn't exist yet. The tests must cover: all-gates-pass, one-gate-fails, missing-spec (Err), PR-already-created, gates-fail-no-gh-call, gh-not-found (serial, mutates PATH), mock-gh-success (serial, mutates PATH + milestone saved), Verify→Complete transition (serial). Use `use serial_test::serial;` for the PATH-mutating tests.
  - Verify: `cargo test -p assay-types` passes. `cargo test -p assay-core --features assay-types/orchestrate --test milestone_io` passes. `cargo test -p assay-core --features assay-types/orchestrate --test cycle` passes. `cargo test -p assay-core --features assay-types/orchestrate --test pr` FAILS with compile error (expected — `assay_core::pr` not yet created). `cargo test --workspace` passes for all non-pr tests.
  - Done when: All existing workspace tests still pass; the new `tests/pr.rs` file fails to compile only because the implementation module is missing, not because of syntax errors in the tests themselves.

- [ ] **T02: Implement `assay-core::pr` module** `est:45m`
  - Why: Delivers the gate-check logic and PR creation function that the tests in T01 are written against; makes all 8 `tests/pr.rs` tests pass
  - Files: `crates/assay-core/src/pr.rs` (new), `crates/assay-core/src/lib.rs`
  - Do: Create `crates/assay-core/src/pr.rs`. Define `pub struct ChunkGateFailure { pub chunk_slug: String, pub required_failed: usize }`. Define `pub struct PrCreateResult { pub pr_number: u64, pub pr_url: String }`. Implement `pub fn pr_check_milestone_gates(assay_dir, specs_dir, working_dir, milestone_slug) -> Result<Vec<ChunkGateFailure>>`: loads milestone by slug, iterates `milestone.chunks` sorted by order, calls `load_spec_entry_with_diagnostics` for each chunk's slug, calls `evaluate_all_gates` for each spec's gates, collects `ChunkGateFailure` entries where `summary.enforcement.required_failed > 0`, returns `Ok(failures)`. Implement `pub fn pr_create_if_gates_pass(assay_dir, specs_dir, working_dir, milestone_slug, title, body: Option<&str>) -> Result<PrCreateResult>`: load milestone; if `pr_number.is_some()` return `AssayError::Io` "PR already created: #{N} — {url}"; call `pr_check_milestone_gates`; if failures return `AssayError::Io` with formatted chunk list; build `gh` args: `["pr", "create", "--title", title, "--base", base_branch, "--json", "number,url"]` plus optional `["--body", body]`; run via `Command::new("gh").args(...).current_dir(working_dir).output()`; map spawn `NotFound` → `AssayError::Io` "gh CLI not found — install from https://cli.github.com"; map non-zero exit → `AssayError::Io` from stderr; parse `{"number":N,"url":"..."}` via `serde_json::from_slice`; set `milestone.pr_number`, `milestone.pr_url`, `milestone.updated_at`; if `milestone.status == Verify` call `milestone_phase_transition(&mut milestone, Complete)`; `milestone_save`; return `PrCreateResult`. Add `pub mod pr;` to `lib.rs`.
  - Verify: `cargo test -p assay-core --features assay-types/orchestrate --test pr` → 8 tests pass, 0 failures. `cargo test --workspace` → all tests green.
  - Done when: All 8 `tests/pr.rs` integration tests pass; no regressions in existing test suite.

- [ ] **T03: Wire CLI `assay pr create` and MCP `pr_create` tool** `est:35m`
  - Why: Exposes `pr_create_if_gates_pass` to users via the CLI and to agents via the MCP transport; completes R045 at the integration level
  - Files: `crates/assay-cli/src/commands/pr.rs` (new), `crates/assay-cli/src/commands/mod.rs`, `crates/assay-cli/src/main.rs`, `crates/assay-mcp/src/server.rs`
  - Do: Create `crates/assay-cli/src/commands/pr.rs` with `PrCommand::Create { milestone: String, title: Option<String>, body: Option<String> }`. The `title` defaults to `format!("feat: {}", milestone)` if not provided. `pr_create_cmd` calls `pr_create_if_gates_pass`; on success prints `"PR created: #{N} — {url}"`; on `Err` uses `eprintln!("Error: {e}")` + `return Ok(1)` (D072 pattern). Add two unit tests: `pr_create_cmd_no_milestones_exits_1` (no .assay dir → exits 1) and `pr_create_cmd_already_created_exits_1` (milestone with pr_number set → exits 1). Add `pub mod pr;` to `commands/mod.rs`. Add `Pr { command: commands::pr::PrCommand }` variant + dispatch arm to `main.rs` (after `Plan`, following the existing ordering). In `server.rs`: define `PrCreateParams { milestone_slug: String, title: String, body: Option<String> }` with `#[derive(Deserialize, JsonSchema)]`; add `pub async fn pr_create` tool method following the `cycle_advance` pattern: resolve cwd, load config, `spawn_blocking` wrapping `assay_core::pr::pr_create_if_gates_pass`; return JSON `{ pr_number, pr_url }` on success via `serde_json::json!`; `domain_error` on failure. Add `pr_create_tool_in_router` presence test. Update the doc comment at the top of `server.rs` that lists all tools to include `pr_create`.
  - Verify: `cargo test -p assay-cli -- pr` passes (2 unit tests). `cargo test -p assay-mcp -- pr_create` passes (presence test). `cargo test --workspace` green. `assay pr create --help` prints usage. `cargo clippy --workspace -- -D warnings` clean.
  - Done when: `assay pr create` subcommand is reachable; `pr_create` is in the MCP tool router; `just ready` green.

## Files Likely Touched

- `crates/assay-types/src/milestone.rs`
- `crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap`
- `crates/assay-core/tests/milestone_io.rs`
- `crates/assay-core/tests/cycle.rs`
- `crates/assay-core/tests/pr.rs` (new)
- `crates/assay-core/src/pr.rs` (new)
- `crates/assay-core/src/lib.rs`
- `crates/assay-cli/src/commands/pr.rs` (new)
- `crates/assay-cli/src/commands/mod.rs`
- `crates/assay-cli/src/main.rs`
- `crates/assay-mcp/src/server.rs`
