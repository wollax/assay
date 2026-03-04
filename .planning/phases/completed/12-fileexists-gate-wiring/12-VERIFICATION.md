# Phase 12 Verification: FileExists Gate Wiring

**Status:** passed
**Score:** 5/5 must-haves verified
**Date:** 2026-03-04

---

## Artifact Checks

### Artifact 1: `crates/assay-types/src/criterion.rs` contains `pub path: Option<String>`

**Result: PASS**

Found at line 29 of `/Users/wollax/Git/personal/assay/crates/assay-types/src/criterion.rs`:

```rust
#[serde(skip_serializing_if = "Option::is_none", default)]
pub path: Option<String>,
```

The field carries both `skip_serializing_if` and `default` attributes, ensuring backward compatibility with existing TOML files that lack the `path` key.

### Artifact 2: `crates/assay-types/src/gates_spec.rs` contains `pub path: Option<String>`

**Result: PASS**

Found at line 30 of `/Users/wollax/Git/personal/assay/crates/assay-types/src/gates_spec.rs`:

```rust
#[serde(skip_serializing_if = "Option::is_none", default)]
pub path: Option<String>,
```

Same serde attributes as `Criterion`. The field is present on `GateCriterion`.

### Artifact 3: `crates/assay-core/src/gate/mod.rs` contains `evaluate_file_exists` in `evaluate()` dispatch

**Result: PASS**

Found at line 65 of `/Users/wollax/Git/personal/assay/crates/assay-core/src/gate/mod.rs`:

```rust
match (&criterion.cmd, &criterion.path) {
    (Some(cmd), _) => evaluate_command(cmd, working_dir, timeout),
    (None, Some(path)) => evaluate_file_exists(path, working_dir),
    (None, None) => evaluate_always_pass(),
}
```

`evaluate_file_exists` is reachable via the `(None, Some(path))` arm.

---

## Key Link Checks

### Link 1: `evaluate()` dispatches to `evaluate_file_exists()` via match on `(cmd, path)` tuple

**Result: PASS**

Verified above. The match at line 63 is an exhaustive 3-arm match on `(&criterion.cmd, &criterion.path)`. The `(None, Some(path))` arm dispatches to `evaluate_file_exists(path, working_dir)`. `GateKind::FileExists` is no longer dead code.

### Link 2: `to_criterion()` copies `path` field from `GateCriterion` to `Criterion`

**Result: PASS**

Found at lines 233-241 of `/Users/wollax/Git/personal/assay/crates/assay-core/src/gate/mod.rs`:

```rust
pub fn to_criterion(gc: &GateCriterion) -> Criterion {
    Criterion {
        name: gc.name.clone(),
        description: gc.description.clone(),
        cmd: gc.cmd.clone(),
        path: gc.path.clone(),
        timeout: gc.timeout,
    }
}
```

The `path` field is explicitly cloned from `GateCriterion` into `Criterion`.

---

## Must-Have Truth Checks

### Must-have 1: "A spec with a FileExists criterion pointing to an existing file evaluates to `passed: true` with `GateKind::FileExists` evidence"

**Result: PASS**

Test `evaluate_dispatches_file_exists_present` (line 905) creates a temp file `target.txt`, builds a `Criterion` with `path: Some("target.txt")` and `cmd: None`, calls `evaluate()`, and asserts:
- `result.passed == true`
- `result.kind` matches `GateKind::FileExists { path }` where `path == "target.txt"`

Also covered by the direct unit test `evaluate_file_exists_present` (line 610) which calls `evaluate_file_exists()` directly and verifies `result.passed` and `result.stderr.is_empty()`.

Both tests pass.

### Must-have 2: "A spec with a FileExists criterion pointing to a missing file evaluates to `passed: false` with a clear reason in stderr"

**Result: PASS**

Test `evaluate_dispatches_file_exists_missing` (line 924) builds a `Criterion` with `path: Some("missing.txt")` pointing at a nonexistent file and asserts:
- `!result.passed`
- `result.stderr.contains("file not found")`

The `evaluate_file_exists` function at line 277 sets stderr to `format!("file not found: {}", full_path.display())` when the file is absent.

Test passes.

### Must-have 3: "`GateKind::FileExists` is no longer dead code — it is reachable through the `evaluate()` dispatch"

**Result: PASS**

The match in `evaluate()` at line 63 has a dedicated arm `(None, Some(path)) => evaluate_file_exists(path, working_dir)` which constructs and returns a `GateResult` with `kind: GateKind::FileExists { path: path.to_string() }` (line 273). The variant is reachable and exercised by the passing tests.

No `unreachable!` or `todo!` macros reference `FileExists` anywhere in the codebase:
```
grep -r "unreachable\|todo!" crates/ | grep -i fileexists
# (no output)
```

### Must-have 4: "Criteria with only `path` (no `cmd`) are NOT skipped by `evaluate_all`/`evaluate_all_gates` — they dispatch to `evaluate_file_exists`"

**Result: PASS**

`evaluate_all` at line 99 checks `criterion.cmd.is_none() && criterion.path.is_none()` before skipping. Criteria with `path: Some(...)` and `cmd: None` fall through to `evaluate()` which routes them to `evaluate_file_exists`.

`evaluate_all_gates` at line 175 applies the identical guard after calling `to_criterion(gate_criterion)`.

Test `evaluate_all_includes_file_exists_criteria` (line 942) creates a spec with one `path`-only criterion and one fully-empty criterion, runs `evaluate_all`, and asserts:
- `summary.passed == 1` (the file-exists criterion for the existing file)
- `summary.skipped == 1` (the empty criterion)
- `summary.failed == 0`

Test passes.

### Must-have 5: "Existing tests continue to pass — backward compatibility preserved via `serde(default)` on `path` field"

**Result: PASS**

Both `Criterion` and `GateCriterion` annotate `path` with `#[serde(skip_serializing_if = "Option::is_none", default)]`. The `default` attribute means existing TOML/JSON that omits `path` deserializes correctly with `path: None`.

Full `just ready` output confirms:
- `cargo fmt --all -- --check`: passed
- `cargo clippy --workspace --all-targets -- -D warnings`: passed (0 warnings)
- `cargo test --workspace`: 99 + 18 + 21 + 23 + 14 = 175 tests passed, 0 failed
- `cargo deny`: advisories ok, bans ok, licenses ok, sources ok
- Plugin version check: passed

---

## `just ready` Result

```
cargo fmt --all -- --check        PASS
cargo clippy -D warnings          PASS (0 warnings)
cargo test --workspace            PASS (175 tests, 0 failed, 3 ignored)
cargo deny                        PASS
Plugin version check              PASS
All checks passed.
```

Test suite breakdown:
- `assay-cli` (bin): 0 tests
- `assay-core` (lib): 99 tests (includes all new FileExists tests)
- `assay-mcp` (lib): 18 tests
- `assay-tui` (bin): 0 tests
- `assay-types` (lib): 21 tests
- Other integration suites: 23 + 14 tests

---

## Summary

All 5 must-have truths are verified against the actual codebase. All 3 required artifacts are present with correct content. All key links (`evaluate()` dispatch and `to_criterion()` path copy) exist as specified. The three explicitly named tests pass. `just ready` completes clean with zero failures.

Phase 12 goal is fully achieved: `GateKind::FileExists` is wired into the evaluation dispatch and produces real results.
