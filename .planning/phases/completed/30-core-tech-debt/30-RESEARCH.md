# Phase 30: Core Tech Debt - Research

**Completed:** 2026-03-09
**Level:** 0 (internal refactoring, existing patterns only)

## Standard Stack

No new dependencies. All work uses existing stdlib, `tracing`, and `eprintln!` patterns already in the codebase.

## Architecture Patterns

### CORE-02: Validation Deduplication

**Confidence: HIGH**

`validate()` (line 90) and `validate_gates_spec()` (line 358) in `crates/assay-core/src/spec/mod.rs` share nearly identical logic:
- Empty name check (identical)
- Empty criteria check (identical)
- Duplicate criterion name check (identical)
- `cmd` + `path` mutual exclusion check (identical)
- `AgentReport` + `cmd`/`path` check (identical)
- Executable criterion enforcement check (identical logic, different types: `Criterion` vs `GateCriterion`)

**Key difference:** `validate()` operates on `&Spec` (which contains `Vec<Criterion>`), while `validate_gates_spec()` operates on `&GatesSpec` (which contains `Vec<GateCriterion>`). Both types have the same fields used in validation: `name`, `cmd`, `path`, `kind`, `enforcement`.

**Pattern:** Extract a shared `validate_criteria()` helper that takes an iterator of trait-compatible items. Two approaches:
1. **Trait approach:** Define a trait with accessors (`name()`, `cmd()`, `path()`, `kind()`, `enforcement()`) implemented on both `Criterion` and `GateCriterion`, then pass `&[impl CriterionLike]`. This is the cleanest but requires a new trait.
2. **Conversion approach:** Use the existing `to_criterion()` function (line 311 in `gate/mod.rs`) to convert `GateCriterion` to `Criterion` before validation. Simpler but allocates.
3. **Closure/adapter approach:** Extract the criteria-validation loop into a function that takes closures for field access.

**Recommendation:** Use approach 2 (conversion). `to_criterion()` already exists. `validate_gates_spec()` can convert criteria to `Criterion` references and delegate to a shared `validate_criteria_list()` function. The allocation cost is negligible (validation is not hot-path).

Alternatively, `validate_gates_spec` could build a temporary `Spec` and call `validate()`. But this requires fabricating a `name` and `gate` section. Better to extract the criteria loop into a shared helper that both functions call.

### CORE-03: Evaluation Deduplication

**Confidence: HIGH**

`evaluate_all()` (line 97) and `evaluate_all_gates()` (line 204) in `crates/assay-core/src/gate/mod.rs` are ~90 lines each of nearly identical code. The only difference:
- `evaluate_all` iterates `&spec.criteria` (type `Criterion`) and calls `resolve_enforcement(criterion.enforcement, spec.gate.as_ref())`
- `evaluate_all_gates` iterates `&gates.criteria` (type `GateCriterion`), calls `to_criterion()` first, then same logic

**Pattern:** Extract a shared `evaluate_criteria()` helper:
```rust
fn evaluate_criteria(
    spec_name: &str,
    criteria: impl Iterator<Item = (Criterion, Enforcement)>,
    working_dir: &Path,
    cli_timeout: Option<u64>,
    config_timeout: Option<u64>,
) -> GateRunSummary
```

Both public functions become thin wrappers that map their types into this shared core. `evaluate_all_gates` already calls `to_criterion()`, so the conversion is natural.

### CORE-04: History List Error Handling

**Confidence: HIGH**

`history::list()` at `crates/assay-core/src/history/mod.rs:196` currently uses `filter_map` on `read_dir` entries. Unreadable entries trigger `eprintln!("Warning: skipping history entry: {e}")` (line 213) — this already works.

**Wait — re-reading the requirement:** "emits a warning for unreadable directory entries instead of silently dropping them." The code at line 210-216 already does emit a warning. Let me verify this is the right function... Yes, `history::list()` at line 196. It already prints warnings on line 213. The requirement may already be satisfied, or the requirement may want a different warning mechanism (e.g., `tracing::warn!` instead of `eprintln!`).

**Current behavior:** `eprintln!("Warning: skipping history entry: {e}")` — this is visible output, not silent.

**Recommendation:** If the intent is to align with the codebase's `tracing` usage (guard daemon uses `tracing::warn!`), switch from `eprintln!` to `tracing::warn!`. Otherwise, this may already be done.

### CORE-06: `generate_run_id` Visibility

**Confidence: HIGH**

`pub fn generate_run_id()` at `crates/assay-core/src/history/mod.rs:47` is called from:
- `crates/assay-core/src/gate/session.rs:33` (internal — fine with `pub(crate)`)
- `crates/assay-core/src/history/mod.rs` tests (internal — fine)
- **`crates/assay-cli/src/commands/gate.rs:574`** (external — would break)
- **`crates/assay-mcp/src/server.rs:634`** (external — would break)

**Problem:** Two external callers use `generate_run_id` to construct a `GateRunRecord` before calling `history::save()`. Making it `pub(crate)` requires moving ID generation into `history::save()` or providing a higher-level API.

**Pattern:** Add a `history::create_record()` or modify `history::save()` to accept raw summary + metadata and generate the run ID internally. The external callers build a `GateRunRecord` with `run_id`, `assay_version`, `timestamp`, `working_dir`, and `summary`. A convenience function could handle this:

```rust
pub fn save_run(
    assay_dir: &Path,
    summary: GateRunSummary,
    working_dir: Option<&Path>,
    assay_version: &str,
    max_history: Option<usize>,
) -> Result<SaveResult>
```

This would generate `run_id` internally and construct the `GateRunRecord`. Then `generate_run_id` becomes `pub(crate)`.

### CORE-07: PID File `fsync()`

**Confidence: HIGH**

`create_pid_file()` at `crates/assay-core/src/guard/pid.rs:44` uses `fs::write()` (line 58) which does NOT fsync. The history module already uses fsync for its atomic writes (line 99 comment mentions "fsynced").

**Pattern:** Replace `fs::write(pid_path, ...)` with explicit `File::create` + `write_all` + `sync_all`:
```rust
use std::io::Write;
let mut f = std::fs::File::create(pid_path).map_err(|source| AssayError::Io { ... })?;
f.write_all(std::process::id().to_string().as_bytes()).map_err(|source| AssayError::Io { ... })?;
f.sync_all().map_err(|source| AssayError::Io { ... })?;
```

### CORE-08: `try_save_checkpoint` Uses Stored Project Dir

**Confidence: HIGH**

`try_save_checkpoint()` at `crates/assay-core/src/guard/daemon.rs:303` calls `std::env::current_dir()` to get the project directory. The `GuardDaemon` struct (line 20-27) does NOT store a project directory — it only stores `session_path`, `assay_dir`, `config`, `breaker`, and `last_check`.

**Pattern:** Add a `project_dir: PathBuf` field to `GuardDaemon`. The caller of `GuardDaemon::new()` should pass the project dir (it knows the project root). Then `try_save_checkpoint` uses `&self.project_dir` instead of `std::env::current_dir()`.

Need to check who constructs `GuardDaemon::new()` to ensure project_dir is available there.

### CORE-09: Spec Parse Errors Surfaced

**Confidence: HIGH**

`spec::scan()` at `crates/assay-core/src/spec/mod.rs:502` already collects parse errors into `ScanResult.errors`. All callers (CLI and MCP) already iterate and display these errors:
- `assay-cli/src/commands/gate.rs:307`: `eprintln!("Warning: {err}")`
- `assay-cli/src/commands/init.rs:31`: `eprintln!("Warning: {err}")`
- `assay-cli/src/commands/spec.rs:233`: `eprintln!("Warning: {err}")`
- `assay-mcp/src/server.rs:443`: Includes errors in response

**Assessment:** Spec parse errors are NOT silently ignored — they're collected and surfaced. If CORE-09 refers to a different code path (e.g., somewhere that catches parse errors with `.ok()` or `.unwrap_or`), additional investigation would be needed. However, the scan function and all its callers already handle this correctly. The requirement may be about consistency (using `tracing::warn!` instead of `eprintln!` in the CLI callers, or moving the warning into `assay-core` itself rather than relying on each caller).

**Recommendation:** Move the warning emission into `assay-core` itself (either in `scan()` or via a helper) so that callers don't need to remember to iterate `.errors`. Alternatively, confirm with the spec author whether this is already satisfied.

## Don't Hand-Roll

- **Atomic file writes:** Already implemented in `history::save()` via tempfile-then-rename. Use the same pattern for PID fsync (but PID doesn't need atomicity, just durability — `sync_all()` is sufficient).
- **Process liveness checks:** Already implemented in `guard/pid.rs` via `libc::kill(pid, 0)`. Do not reimplement.

## Common Pitfalls

| Pitfall | How to Avoid |
|---------|-------------|
| Breaking external callers when tightening `generate_run_id` to `pub(crate)` | Provide `history::save_run()` convenience API first, migrate callers, then tighten |
| Validation dedup introducing subtle behavior differences | Run existing test suite after refactor; the 14+ validation tests catch regressions |
| `evaluate_all` / `evaluate_all_gates` dedup losing the `to_criterion` conversion | The shared helper must accept already-resolved `(Criterion, Enforcement)` pairs |
| `fsync` on PID file failing on unusual filesystems | Map to `AssayError::Io` like existing code; the guard daemon already handles IO errors gracefully |
| `try_save_checkpoint` project_dir change breaking daemon startup | Ensure `GuardDaemon::new()` caller passes project dir; check `guard::start()` or equivalent |
| Mixing `eprintln!` and `tracing::warn!` patterns | Pick one per module: guard uses `tracing`, history/spec use `eprintln!`. Either unify or keep per-module consistency |

## Code Examples

### Validation criteria helper signature
```rust
fn validate_criteria<'a>(
    criteria: &'a [impl CriteriaFields],
    gate: Option<&GateSection>,
    errors: &mut Vec<SpecError>,
)
```

### Evaluation shared helper
```rust
fn evaluate_criteria_iter(
    spec_name: &str,
    criteria: Vec<(Criterion, Enforcement)>,
    working_dir: &Path,
    cli_timeout: Option<u64>,
    config_timeout: Option<u64>,
) -> GateRunSummary
```

### PID file with fsync
```rust
let mut file = fs::File::create(pid_path).map_err(/* ... */)?;
file.write_all(std::process::id().to_string().as_bytes()).map_err(/* ... */)?;
file.sync_all().map_err(/* ... */)?;
```

### GuardDaemon with stored project_dir
```rust
pub struct GuardDaemon {
    session_path: PathBuf,
    assay_dir: PathBuf,
    project_dir: PathBuf,  // NEW
    config: GuardConfig,
    breaker: CircuitBreaker,
    last_check: Option<Instant>,
}
```

## Key File Locations

| Requirement | Primary File | Lines |
|-------------|-------------|-------|
| CORE-02 | `crates/assay-core/src/spec/mod.rs` | 90-182 (`validate`), 358-450 (`validate_gates_spec`) |
| CORE-03 | `crates/assay-core/src/gate/mod.rs` | 97-196 (`evaluate_all`), 204-300 (`evaluate_all_gates`) |
| CORE-04 | `crates/assay-core/src/history/mod.rs` | 196-230 (`list`) |
| CORE-06 | `crates/assay-core/src/history/mod.rs` | 47 (`generate_run_id`); external callers in `assay-cli` and `assay-mcp` |
| CORE-07 | `crates/assay-core/src/guard/pid.rs` | 44-63 (`create_pid_file`) |
| CORE-08 | `crates/assay-core/src/guard/daemon.rs` | 303-327 (`try_save_checkpoint`), 20-27 (`GuardDaemon` struct) |
| CORE-09 | `crates/assay-core/src/spec/mod.rs` | 502-623 (`scan`) — already surfaces errors |

## Warning Pattern Analysis

The codebase uses two warning patterns:
1. **`eprintln!("Warning: ...")`** — used in `history/mod.rs` (lines 88, 213), `worktree.rs` (line 306)
2. **`tracing::warn!(...)`** — used in `guard/daemon.rs` (lines 132, 162, 295, 307, 319, 324)

The guard module uses `tracing` because it's a long-running daemon where structured logging matters. The rest of the codebase uses `eprintln!` for user-facing warnings. Both patterns are valid for their use cases.

**Recommendation:** Keep `eprintln!` for CORE-04 (history is CLI-facing). Use `tracing::warn!` only in daemon code.

## Existing `pub(crate)` Usage

No `pub(crate)` items currently exist in `assay-core/src/`. All public items are `pub`. This means CORE-06 introduces the first `pub(crate)` usage, setting the pattern for opportunistic tightening.

---

*Phase: 30-core-tech-debt*
*Research completed: 2026-03-09*
