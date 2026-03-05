# Phase 14 Verification: Run History Core

**Date:** 2026-03-05
**Verifier:** Claude Code (kata phase verifier)
**Status:** passed
**Score:** 13/13 must_haves verified

---

## Summary

All must_haves from Plan 01 and Plan 02 are present in the actual codebase. All 10 history tests pass. The full `just ready` suite (fmt, clippy, 122 unit tests + 29 schema roundtrip + 17 schema snapshot tests, cargo-deny) passes with zero errors.

---

## Must-Have Verification (Plan 01)

### MH-01: GateRunRecord wraps GateRunSummary with run_id, assay_version, timestamp, and optional working_dir

**Verified.** `crates/assay-types/src/gate_run.rs` lines 70–84:

```rust
pub struct GateRunRecord {
    pub run_id: String,
    pub assay_version: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    pub summary: GateRunSummary,
}
```

### MH-02: GateRunRecord uses deny_unknown_fields and includes assay_version for future schema migration

**Verified.** `crates/assay-types/src/gate_run.rs` line 71: `#[serde(deny_unknown_fields)]` is present on `GateRunRecord`. The doc comment explicitly states: "`assay_version` supports future schema migration."

### MH-03: history::save() writes a GateRunRecord atomically via tempfile-then-rename

**Verified.** `crates/assay-core/src/history/mod.rs` lines 39–86. The function:
1. Creates the results dir with `create_dir_all`
2. Serializes to JSON with `serde_json::to_string_pretty`
3. Creates a `NamedTempFile` in the target directory
4. Writes JSON bytes, then calls `sync_all()`
5. Renames via `tmpfile.persist(&final_path)`

File path pattern is `.assay/results/<spec-name>/<run-id>.json` (assay_dir is passed as the `.assay` directory by convention).

### MH-04: history::list() returns sorted run IDs for a spec

**Verified.** `crates/assay-core/src/history/mod.rs` lines 116–143. Reads `results/<spec-name>/`, filters for `.json` files, strips extension to extract run IDs, and calls `ids.sort()`. Returns empty vec (not error) when directory does not exist.

### MH-05: history::load() deserializes a specific run from its JSON file

**Verified.** `crates/assay-core/src/history/mod.rs` lines 93–110. Constructs path as `results/<spec-name>/<run-id>.json`, reads to string, deserializes with `serde_json::from_str`.

### MH-06: Auto-creates .assay/results/<spec-name>/ on first save (mkdir -p equivalent)

**Verified.** `crates/assay-core/src/history/mod.rs` lines 42–46: `std::fs::create_dir_all(&results_dir)` is called unconditionally at the start of `save()`.

### MH-07: Two concurrent saves for the same spec produce two distinct files

**Verified.** Test `test_save_does_not_clobber` (lines 286–304) covers sequential uniqueness. Test `test_concurrent_saves_produce_distinct_files` (lines 307–346) spawns 10 threads each calling `save()` concurrently for the same spec and asserts all 10 resulting paths are distinct.

### MH-08: A crash during write does not leave a corrupt JSON file

**Verified.** Test `test_partial_write_leaves_no_corrupt_file` (lines 349–372) simulates this by writing truncated JSON to a `.tmp_partial_write` file (no `.json` extension) in the results dir. `list()` excludes it because it filters for `.json` only. The `NamedTempFile`-based implementation ensures any real crash leaves only a temp file without `.json` extension, which is excluded by `list()`.

### MH-09: Persisted GateRunRecord round-trips through serde_json faithfully

**Verified.** Test `test_load_roundtrip` (lines 235–255) saves and reloads a record, asserting all individual fields match.

---

## Must-Have Verification (Plan 02)

### MH-10: Two concurrent saves for the same spec produce two distinct, valid files

**Verified.** Test `test_concurrent_saves_produce_distinct_files` (lines 307–346) runs 10 threads concurrently, asserts 10 distinct paths, then calls `load()` for each ID and verifies it deserializes without error.

### MH-11: A partially-written file (simulated crash) does not leave a corrupt JSON in results

**Verified.** See MH-08 above. Same test covers this requirement.

### MH-12: Persisted GateRunRecord deserializes back to the same logical content that was saved

**Verified.** Test `test_load_roundtrip` covers basic round-trip. Test `test_full_fidelity_roundtrip` covers this requirement explicitly.

### MH-13: GateRunRecord with all fields populated (including working_dir, non-empty results with enforcement) round-trips faithfully

**Verified.** Test `test_full_fidelity_roundtrip` (lines 375–479) constructs a fully populated `GateRunRecord` with:
- `working_dir: Some("/tmp/test-project")`
- 4 `CriterionResult` entries with `Required` and `Advisory` enforcement
- `GateKind::Command`, `GateKind::FileExists`, and a skipped criterion
- Truncated output (`truncated: true`, `original_bytes: Some(131_072)`)
- Non-default `EnforcementSummary` counts

Asserts `record == loaded` (full structural equality via `PartialEq`) and also performs independent deserialization from raw file bytes.

---

## Dependencies Verified

**serde_json and tempfile in Cargo.toml:** `crates/assay-core/Cargo.toml` lists both `serde_json.workspace = true` and `tempfile.workspace = true` as direct dependencies.

**pub mod history in lib.rs:** `crates/assay-core/src/lib.rs` line 22: `pub mod history;`

**Imports:** `history/mod.rs` imports `NamedTempFile` from `tempfile`, `GateRunRecord` from `assay_types`, and `AssayError` from `crate::error`.

---

## Test Run Results

```
cargo test -p assay-core -- history
10 passed, 115 filtered out (2 suites, 0.07s)
```

Tests passing:
- `test_generate_run_id_format`
- `test_save_creates_file`
- `test_save_creates_directories`
- `test_load_roundtrip`
- `test_list_empty_dir`
- `test_list_returns_sorted`
- `test_save_does_not_clobber`
- `test_concurrent_saves_produce_distinct_files`
- `test_partial_write_leaves_no_corrupt_file`
- `test_full_fidelity_roundtrip`

```
just ready
All checks passed.
```

Full suite: 122 assay-core unit tests, 18 assay-mcp tests, 21 assay-types tests, 29 schema roundtrip tests, 17 schema snapshot tests — all pass. Zero warnings from clippy. cargo-deny clean.

---

## Gaps

None.
