# Phase 12: FileExists Gate Wiring - Research

**Researched:** 2026-03-04
**Confidence:** HIGH (all findings are from direct codebase reading)

## Executive Summary

The `evaluate_file_exists` function already exists and works correctly in `crates/assay-core/src/gate/mod.rs:265-287`. The gap is that it is never called from the `evaluate()` dispatch function (line 57-66), which only maps `Criterion` fields to `Command` or `AlwaysPass`. The `Criterion` type lacks a `path` field, so there is no way to express a FileExists criterion in a spec today. This phase needs to:

1. Add a `path` field to `Criterion` (and `GateCriterion`)
2. Update `evaluate()` dispatch to route `path`-bearing criteria to `evaluate_file_exists`
3. Update `evaluate_all` and `evaluate_all_gates` skip logic (currently only checks `cmd.is_none()`)

## Existing Code Map

### Types (assay-types)

| File | Item | Purpose |
|------|------|---------|
| `crates/assay-types/src/gate.rs:13-28` | `GateKind` enum | Has `Command`, `AlwaysPass`, `FileExists { path: String }` -- **type exists** |
| `crates/assay-types/src/gate.rs:42-79` | `GateResult` struct | Output of any gate evaluation, carries `kind: GateKind` |
| `crates/assay-types/src/criterion.rs:13-30` | `Criterion` struct | Has `name`, `description`, `cmd: Option<String>`, `timeout: Option<u64>` -- **no `path` field** |
| `crates/assay-types/src/gates_spec.rs:16-34` | `GateCriterion` struct | Like `Criterion` + `requirements: Vec<String>` -- **also no `path` field** |
| `crates/assay-types/src/gate_run.rs:33-39` | `CriterionResult` struct | Wraps `Option<GateResult>` with criterion name |

### Evaluation Logic (assay-core)

| File | Item | Purpose |
|------|------|---------|
| `crates/assay-core/src/gate/mod.rs:57-66` | `evaluate()` | **THE GAP** -- dispatches `Some(cmd) -> Command`, `None -> AlwaysPass`. FileExists unreachable. |
| `crates/assay-core/src/gate/mod.rs:84-150` | `evaluate_all()` | Iterates spec criteria. Skips when `cmd.is_none()` (line 97). |
| `crates/assay-core/src/gate/mod.rs:158-226` | `evaluate_all_gates()` | Same pattern for `GatesSpec`. Skips when `cmd.is_none()` (line 173). |
| `crates/assay-core/src/gate/mod.rs:231-237` | `to_criterion()` | Converts `GateCriterion -> Criterion`. Drops `requirements`. **Needs to also copy `path`.** |
| `crates/assay-core/src/gate/mod.rs:265-287` | `evaluate_file_exists()` | **ALREADY IMPLEMENTED** -- resolves path relative to working_dir, uses `Path::exists()`, returns `GateResult` with `GateKind::FileExists`. |

### Callers (CLI and MCP)

| File | Lines | What it does |
|------|-------|--------------|
| `crates/assay-cli/src/main.rs:709,800` | `evaluate_all` / `evaluate_all_gates` | CLI calls aggregate evaluators. No changes needed (they delegate to core). |
| `crates/assay-mcp/src/server.rs:240-252` | `gate_run` handler | MCP calls same aggregate evaluators. No changes needed. |

## The Exact Gap

### 1. `Criterion` has no `path` field

```rust
// Current (crates/assay-types/src/criterion.rs:13-30)
pub struct Criterion {
    pub name: String,
    pub description: String,
    pub cmd: Option<String>,      // Command gate
    pub timeout: Option<u64>,
    // MISSING: pub path: Option<String>  -- FileExists gate
}
```

The `evaluate()` function derives `GateKind` from criterion fields. Without a `path` field, there is no input to trigger `FileExists`.

### 2. `evaluate()` has no FileExists arm

```rust
// Current (crates/assay-core/src/gate/mod.rs:57-66)
pub fn evaluate(criterion: &Criterion, working_dir: &Path, timeout: Duration) -> Result<GateResult> {
    match &criterion.cmd {
        Some(cmd) => evaluate_command(cmd, working_dir, timeout),
        None => evaluate_always_pass(),
    }
}
```

Needs to become something like:
```rust
pub fn evaluate(criterion: &Criterion, working_dir: &Path, timeout: Duration) -> Result<GateResult> {
    match (&criterion.cmd, &criterion.path) {
        (Some(cmd), _) => evaluate_command(cmd, working_dir, timeout),
        (None, Some(path)) => evaluate_file_exists(path, working_dir),
        (None, None) => evaluate_always_pass(),
    }
}
```

### 3. Skip logic in `evaluate_all` / `evaluate_all_gates` checks only `cmd`

```rust
// Current (line 97 in evaluate_all)
if criterion.cmd.is_none() {
    skipped += 1;
    // ...
}
```

This would incorrectly skip FileExists criteria (which have `cmd: None` but `path: Some(...)`). Needs to become:
```rust
if criterion.cmd.is_none() && criterion.path.is_none() {
    skipped += 1;
    // ...
}
```

### 4. `to_criterion()` doesn't copy `path`

```rust
// Current (line 231-237)
pub fn to_criterion(gc: &GateCriterion) -> Criterion {
    Criterion {
        name: gc.name.clone(),
        description: gc.description.clone(),
        cmd: gc.cmd.clone(),
        timeout: gc.timeout,
        // MISSING: path: gc.path.clone(),
    }
}
```

### 5. `GateCriterion` also needs `path`

Same as `Criterion` -- the `GateCriterion` in `gates_spec.rs` also needs `path: Option<String>` with `#[serde(skip_serializing_if = "Option::is_none", default)]`.

## Architecture Patterns

### Pattern: How evaluate_file_exists Already Works

```rust
pub fn evaluate_file_exists(path: &str, working_dir: &Path) -> Result<GateResult> {
    let start = Instant::now();
    let full_path = working_dir.join(path);
    let exists = full_path.exists();

    Ok(GateResult {
        passed: exists,
        kind: GateKind::FileExists { path: path.to_string() },
        stdout: String::new(),
        stderr: if exists { String::new() } else {
            format!("file not found: {}", full_path.display())
        },
        exit_code: None,
        duration_ms: start.elapsed().as_millis() as u64,
        timestamp: Utc::now(),
        truncated: false,
        original_bytes: None,
    })
}
```

Key observations:
- Uses `Path::exists()` (follows symlinks via `std::fs::metadata` -- this is standard behavior)
- Path resolved relative to `working_dir` (consistent with Command gates)
- No stdout for FileExists (file checks have no output to capture)
- Failure reason goes in `stderr` field: `"file not found: /full/path"`
- No exit code (not a process)
- Already has tests: `evaluate_file_exists_present` and `evaluate_file_exists_missing`

### Pattern: How Command Gates Report Evidence

For consistency reference:
- `stdout`: captured process output
- `stderr`: captured process stderr, or error messages
- `exit_code`: `Some(code)` for completed processes, `None` for non-process gates
- `kind`: self-describing via `GateKind`

FileExists already follows this pattern correctly.

## Don't Hand-Roll

- **File existence check** -- `Path::exists()` is already used and correct. Do not use `std::fs::metadata` directly unless adding metadata to evidence (and the existing implementation already does the right thing).
- **GateResult construction** -- follow the exact same struct literal pattern as `evaluate_file_exists()` already uses.
- **Serde attributes** -- use `#[serde(skip_serializing_if = "Option::is_none", default)]` for `path` field, matching `cmd` field exactly.

## Common Pitfalls

### 1. Schema Regeneration Required
Adding `path` field to `Criterion` and `GateCriterion` changes their JSON schemas. The schema snapshots in `crates/assay-types/tests/snapshots/` and generated schemas in `schemas/` must be updated. Use `just ready` (which runs schema snapshot tests) to catch this.

### 2. `deny_unknown_fields` on Criterion and GateCriterion
Both structs use `#[serde(deny_unknown_fields)]`. Adding a `path` field is fine (it's a known field), but specs authored by users that include `path` must parse correctly. Old specs without `path` must also parse (hence `default` on the serde attr).

### 3. Mutual Exclusivity of `cmd` and `path`
The dispatch logic `match (&criterion.cmd, &criterion.path)` allows `(Some(cmd), Some(path))`. Decision needed: should specifying both be an error, or should `cmd` take precedence? The current pattern has `cmd` checked first, so `cmd` wins silently. Recommend: keep `cmd` precedence (simpler), and consider a validation warning in a future phase.

### 4. Skip Logic Must Account for Both Fields
The `evaluate_all` and `evaluate_all_gates` functions skip criteria when `cmd.is_none()`. After this change, skipping should only happen when BOTH `cmd` and `path` are `None` (a purely descriptive criterion).

### 5. `to_criterion()` Must Map the New Field
Forgetting to add `path: gc.path.clone()` to `to_criterion()` would silently break FileExists for directory-based specs.

## Code Examples

### Adding `path` to Criterion

```rust
// crates/assay-types/src/criterion.rs
pub struct Criterion {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cmd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timeout: Option<u64>,
}
```

### Updated Dispatch

```rust
pub fn evaluate(criterion: &Criterion, working_dir: &Path, timeout: Duration) -> Result<GateResult> {
    match (&criterion.cmd, &criterion.path) {
        (Some(cmd), _) => evaluate_command(cmd, working_dir, timeout),
        (None, Some(path)) => evaluate_file_exists(path, working_dir),
        (None, None) => evaluate_always_pass(),
    }
}
```

### Updated Skip Logic

```rust
// In evaluate_all and evaluate_all_gates
if criterion.cmd.is_none() && criterion.path.is_none() {
    skipped += 1;
    // ...
    continue;
}
```

### TOML Spec Example (How Users Write FileExists Criteria)

```toml
name = "auth-flow"

[[criteria]]
name = "config-exists"
description = "Auth config file must exist"
path = "config/auth.toml"

[[criteria]]
name = "tests-pass"
description = "All auth tests pass"
cmd = "cargo test -p auth"
```

## Scope Boundaries

### In Scope
- Add `path: Option<String>` to `Criterion` and `GateCriterion`
- Wire `evaluate()` dispatch to route `path` criteria to `evaluate_file_exists()`
- Fix skip logic in `evaluate_all` and `evaluate_all_gates`
- Update `to_criterion()` to copy `path`
- Update existing tests and add new integration tests
- Regenerate schemas

### Out of Scope (Future Phases)
- Richer file checks (size, content, permissions)
- Glob pattern matching for path
- Environment variable expansion in path
- Mutual exclusivity validation (cmd + path)
- FileExists evidence enhancement (metadata, resolved path)

## Change Impact Assessment

| File | Change | Risk |
|------|--------|------|
| `crates/assay-types/src/criterion.rs` | Add `path: Option<String>` | LOW -- additive, backward compatible |
| `crates/assay-types/src/gates_spec.rs` | Add `path: Option<String>` to `GateCriterion` | LOW -- additive, backward compatible |
| `crates/assay-core/src/gate/mod.rs:57-66` | Update `evaluate()` match | LOW -- 3-arm match replacing 2-arm |
| `crates/assay-core/src/gate/mod.rs:97` | Fix skip logic in `evaluate_all` | LOW -- one condition change |
| `crates/assay-core/src/gate/mod.rs:173` | Fix skip logic in `evaluate_all_gates` | LOW -- one condition change |
| `crates/assay-core/src/gate/mod.rs:231-237` | Add `path` to `to_criterion()` | LOW -- one field addition |
| Schema snapshots | Regenerate | LOW -- automated via test update |
| Existing tests | Add `path: None` to all `Criterion` literals | LOW -- mechanical |

**Total estimated new code:** ~30-50 lines (field additions, dispatch change, tests). Most of the work is updating existing `Criterion` struct literals in tests to include `path: None`.
