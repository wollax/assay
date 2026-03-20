---
estimated_steps: 6
estimated_files: 2
---

# T02: Implement `assay-core::pr` module

**Slice:** S04 — Gate-Gated PR Workflow
**Milestone:** M005

## Description

Create `crates/assay-core/src/pr.rs` with `ChunkGateFailure`, `PrCreateResult`, `pr_check_milestone_gates`, and `pr_create_if_gates_pass`. Register the module in `lib.rs`. This task turns all 8 tests in `tests/pr.rs` from compile errors to green. No new `AssayError` variants — all domain errors use `AssayError::Io` (consistent with D065, D008, S01/S02 patterns).

## Steps

1. Create `crates/assay-core/src/pr.rs`. Add the module-level doc comment and imports:
   ```rust
   use std::io;
   use std::path::Path;
   use chrono::Utc;
   use assay_types::{Milestone, MilestoneStatus};
   use crate::error::{AssayError, Result};
   use crate::gate::evaluate_all_gates;
   use crate::milestone::{milestone_load, milestone_save};
   use crate::milestone::cycle::milestone_phase_transition;
   use crate::spec::{SpecEntry, load_spec_entry_with_diagnostics};
   ```

2. Define local result types (not in assay-types — consistent with D046/D051/D073):
   ```rust
   pub struct ChunkGateFailure {
       pub chunk_slug: String,
       pub required_failed: usize,
   }

   pub struct PrCreateResult {
       pub pr_number: u64,
       pub pr_url: String,
   }
   ```

3. Implement `pr_check_milestone_gates`:
   - Load milestone: `milestone_load(assay_dir, milestone_slug)?`
   - Sort chunks by `order` ascending
   - For each chunk: `load_spec_entry_with_diagnostics(&chunk.slug, specs_dir)?`; extract `gates` from `SpecEntry::Directory`; return `AssayError::Io` for legacy specs (same pattern as `cycle_advance`); call `evaluate_all_gates(&gates, working_dir, None, None)`; if `summary.enforcement.required_failed > 0` push to failures vec
   - Return `Ok(failures)`

4. Implement `pr_create_if_gates_pass`:
   - Load milestone (for `pr_number` check + `pr_base`): `milestone_load(assay_dir, milestone_slug)?`
   - If `milestone.pr_number.is_some()`: return `AssayError::Io` with message `format!("PR already created: #{} — {}", n, url)`, `operation: "pr_create_if_gates_pass"`, `path: PathBuf::from(milestone_slug)`
   - Call `pr_check_milestone_gates(assay_dir, specs_dir, working_dir, milestone_slug)?` and if failures non-empty format a multi-line message: `format!("gates failed for {} chunk(s):\n{}", failures.len(), failures.iter().map(|f| format!("  - {}: {} required criteria failed", f.chunk_slug, f.required_failed)).collect::<Vec<_>>().join("\n"))` → return `AssayError::Io`
   - Determine base branch: `milestone.pr_base.as_deref().unwrap_or("main")`
   - Build args: start with `vec!["pr", "create", "--title", title, "--base", base_branch, "--json", "number,url"]`; if `body.is_some()` push `"--body"` and body value
   - Run: `Command::new("gh").args(&args).current_dir(working_dir).output()` — on spawn error: check `e.kind() == io::ErrorKind::NotFound` → `AssayError::Io` with message `"gh CLI not found — install from https://cli.github.com"`; on other spawn errors → `AssayError::Io` with error text
   - On non-zero exit: extract stderr → `AssayError::Io` with stderr content; `operation: "gh pr create"`, `path: PathBuf::from(milestone_slug)`
   - Parse stdout: `serde_json::from_slice::<serde_json::Value>(&output.stdout)?` (map parse error to `AssayError::Io`); extract `number` as `u64` and `url` as `String`; if extraction fails use defensive fallback: `pr_number = 0`, `pr_url = String::from_utf8_lossy(&output.stdout).trim().to_string()`
   - Reload milestone (fresh state for mutation): `let mut milestone = milestone_load(assay_dir, milestone_slug)?`
   - Set `milestone.pr_number = Some(pr_number)`, `milestone.pr_url = Some(pr_url.clone())`, `milestone.updated_at = Utc::now()`
   - If `milestone.status == MilestoneStatus::Verify`: call `milestone_phase_transition(&mut milestone, MilestoneStatus::Complete)?`
   - `milestone_save(assay_dir, &milestone)?`
   - Return `Ok(PrCreateResult { pr_number, pr_url })`

5. Add `pub mod pr;` to `crates/assay-core/src/lib.rs` (after `pub mod milestone;`).

6. Run `cargo test -p assay-core --features assay-types/orchestrate --test pr` and iterate until all 8 tests pass. Then run `cargo test --workspace` to confirm no regressions.

## Must-Haves

- [ ] `pr_check_milestone_gates` returns `Ok(vec![])` when all chunks pass; `Ok(failures)` (non-empty) when any required gates fail; `Err(AssayError::Io)` only for I/O/spec errors
- [ ] `pr_create_if_gates_pass` returns `AssayError::Io` with "PR already created" message when `milestone.pr_number` is already set
- [ ] `pr_create_if_gates_pass` returns `AssayError::Io` with chunk failure list when `pr_check_milestone_gates` returns non-empty failures
- [ ] `pr_create_if_gates_pass` returns `AssayError::Io` with "gh CLI not found — install from https://cli.github.com" when spawn returns `NotFound`
- [ ] `pr_create_if_gates_pass` with mock `gh` returns `Ok(PrCreateResult { pr_number: 42, pr_url: "..." })` and saves `pr_number`/`pr_url` to milestone TOML
- [ ] Milestone with `Verify` status transitions to `Complete` after successful PR creation
- [ ] All 8 `tests/pr.rs` tests pass; no regressions in `cargo test --workspace`
- [ ] No new `AssayError` variants added (uses `AssayError::Io` for all domain errors, consistent with S01/S02)

## Verification

- `cargo test -p assay-core --features assay-types/orchestrate --test pr` → 8 passed, 0 failed
- `cargo test --workspace` → all suites green
- `cargo clippy --workspace -- -D warnings` → clean
- After `test_pr_create_success_mock_gh`: reload milestone TOML and assert `pr_number = 42` + `pr_url` field present

## Observability Impact

- Signals added/changed:
  - `AssayError::Io { operation: "pr_create_if_gates_pass", path: milestone_slug, ... }` on all failure paths
  - `AssayError::Io { operation: "gh pr create", path: milestone_slug, source: stderr }` when gh fails
  - `ChunkGateFailure { chunk_slug, required_failed }` list surfaced in error message for gate failures
- How a future agent inspects this: `cat .assay/milestones/<slug>.toml` shows `pr_number`/`pr_url` after success; check `is_error` field in MCP tool response for failure details; `assay milestone status` shows updated phase
- Failure state exposed: "PR already created: #N — url" prevents re-creation; chunk failure list names exactly which chunks block the PR; "gh CLI not found" is actionable

## Inputs

- `crates/assay-core/tests/pr.rs` — 8 tests from T01; implement to make them all pass
- `crates/assay-core/src/milestone/cycle.rs` — `milestone_phase_transition` and `active_chunk` functions (reference for Verify→Complete check)
- `crates/assay-core/src/milestone/mod.rs` — `milestone_load`, `milestone_save` I/O API
- `crates/assay-core/src/gate/mod.rs` — `evaluate_all_gates` (same call signature as in `cycle_advance`)
- `crates/assay-core/src/spec/mod.rs` — `load_spec_entry_with_diagnostics` (same call as in `cycle_advance`)
- S02-SUMMARY.md — `cycle_advance` is the reference pattern; `pr_create_if_gates_pass` follows same fail-safe structure

## Expected Output

- `crates/assay-core/src/pr.rs` — new: `ChunkGateFailure`, `PrCreateResult`, `pr_check_milestone_gates`, `pr_create_if_gates_pass`
- `crates/assay-core/src/lib.rs` — `pub mod pr;` added
