# Phase 30: Core Tech Debt — Verification

**Status:** passed
**Score:** 5/5 must-haves verified

## Must-Haves

### 1. Shared validation implementation
**Status:** PASS
**Evidence:** Both `validate()` (line 90) and `validate_gates_spec()` (line 373) in `crates/assay-core/src/spec/mod.rs` delegate criterion-level checks to the shared private function `validate_criteria()` (line 123). The name-empty check, duplicate-name check, cmd/path mutual exclusion, AgentReport incompatibility, and "at least one required executable" logic all live in `validate_criteria()` — neither `validate()` nor `validate_gates_spec()` duplicate any enforcement logic.

### 2. Shared evaluation extraction
**Status:** PASS
**Evidence:** Both `evaluate_all()` (line 97) and `evaluate_all_gates()` (line 126) in `crates/assay-core/src/gate/mod.rs` build a `Vec<(Criterion, Enforcement)>` and delegate to the shared private function `evaluate_criteria()` (line 154). The iteration, AgentReport skipping, timeout resolution, evaluation dispatch, and result/enforcement summary accumulation all live in `evaluate_criteria()` — no duplicated loop logic.

### 3. history::list() warning
**Status:** PASS
**Evidence:** In `crates/assay-core/src/history/mod.rs` line 233-237, `list()` uses `.filter_map()` on `read_dir` entries with an explicit `Err(e)` arm that calls `eprintln!("Warning: skipping history entry: {e}")` instead of silently dropping unreadable entries.

### 4. generate_run_id visibility
**Status:** PASS
**Evidence:** In `crates/assay-core/src/history/mod.rs` line 47, `generate_run_id` is declared as `pub(crate) fn generate_run_id(...)` — not `pub`.

### 5. Guard daemon persistence
**Status:** PASS
**Evidence:**
- **5a (fsync):** In `crates/assay-core/src/guard/pid.rs` lines 72-76, `create_pid_file()` calls `file.sync_all()` immediately after `write_all()`, ensuring the PID is fsynced to disk.
- **5b (project_dir):** In `crates/assay-core/src/guard/daemon.rs` line 311, `try_save_checkpoint()` calls `crate::checkpoint::extract_team_state(&self.project_dir, ...)` using the stored `project_dir` field (set at construction on line 43), not a hardcoded or derived path.
