---
id: T02
parent: S01
milestone: M005
provides:
  - "milestone_scan(assay_dir) -> Result<Vec<Milestone>>: returns Ok(vec![]) for missing dir, reads and parses all .toml files sorted by slug"
  - "milestone_load(assay_dir, slug) -> Result<Milestone>: validates slug, reads and parses TOML, canonicalizes slug from parameter"
  - "milestone_save(assay_dir, milestone) -> Result<()>: validates slug, creates dir if needed, atomic NamedTempFile+sync_all+persist write"
  - "pub mod milestone in assay-core crate root"
  - "5-test integration suite in crates/assay-core/tests/milestone_io.rs covering roundtrip, scan-empty, scan-all, missing-slug, traversal-rejection"
key_files:
  - crates/assay-core/src/milestone/mod.rs
  - crates/assay-core/src/lib.rs
  - crates/assay-core/tests/milestone_io.rs
key_decisions:
  - "milestone_scan does not set slug from filename — slug must be present in the TOML itself (consistent with deny_unknown_fields contract in Milestone type)"
  - "milestone_load overrides the parsed slug with the canonical slug from the filename parameter — filename is the source of truth for slug in load-by-slug operations"
  - "parse errors from toml::from_str mapped to AssayError::Io with operation 'parsing milestone TOML' and ErrorKind::InvalidData — avoids adding a new error variant, consistent with existing TOML error mapping in the codebase"
patterns_established:
  - "milestone I/O follows same atomic write pattern as work_session.rs: NamedTempFile::new_in(dir) + write_all + flush + sync_all + persist"
  - "validate_path_component(slug, 'milestone slug') called at top of both milestone_load and milestone_save before any I/O"
  - "milestone_scan returns Ok(vec![]) (not error) when milestones dir absent — callers treat missing dir as empty collection"
observability_surfaces:
  - "milestone_load returns AssayError::Io { operation: 'reading milestone', path: <file_path>, source } on file not found or unreadable"
  - "milestone_load returns AssayError::Io { operation: 'parsing milestone TOML', path: <file_path>, source } on TOML parse failure"
  - "milestone_scan returns AssayError::Io { operation: 'reading milestones directory entry', path: <dir>, source } per-entry on unreadable entries"
  - "milestone_save returns AssayError::Io with operation label and path at every failure point (dir creation, tmpfile, write, flush, sync, persist)"
  - "cargo test -p assay-core --test milestone_io is the primary verification surface"
duration: 20min
verification_result: passed
completed_at: 2026-03-19T00:00:00Z
blocker_discovered: false
---

# T02: Implement milestone_load, milestone_save, milestone_scan in assay-core

**Three atomic-write TOML I/O functions for milestone persistence in `assay-core::milestone`, with 5-test integration suite — all green.**

## What Happened

Created `crates/assay-core/src/milestone/mod.rs` with three public functions following the atomic tempfile-rename pattern from `work_session.rs` and slug validation from `history::validate_path_component`.

- **`milestone_scan`**: Constructs `assay_dir/milestones/`, returns `Ok(vec![])` if absent, iterates entries skipping non-`.toml` files, parses each with `toml::from_str::<Milestone>`, returns sorted by slug.
- **`milestone_load`**: Validates slug, reads `assay_dir/milestones/<slug>.toml`, parses TOML, overwrites `milestone.slug` with the parameter (filename is canonical).
- **`milestone_save`**: Validates slug, creates milestones dir with `create_dir_all`, serializes to TOML, writes via `NamedTempFile::new_in` + `write_all` + `flush` + `sync_all` + `persist`.

Added `pub mod milestone;` to `crates/assay-core/src/lib.rs`.

Created `crates/assay-core/tests/milestone_io.rs` with 5 integration tests using `tempfile::TempDir`.

## Verification

```
cargo test -p assay-core --test milestone_io  # 5/5 pass
cargo test -p assay-core --features orchestrate  # all tests pass, no regressions
grep -r "pub mod milestone" crates/assay-core/src/lib.rs  # confirmed
```

All 5 integration tests pass:
- `test_milestone_save_and_load_roundtrip` ✓
- `test_milestone_scan_empty_for_missing_dir` ✓
- `test_milestone_scan_returns_all_milestones` ✓
- `test_milestone_load_error_for_missing_slug` ✓
- `test_milestone_slug_validation_rejects_traversal` ✓

## Diagnostics

- `cargo test -p assay-core --test milestone_io` — primary verification surface
- All errors carry the file path and operation label via `AssayError::Io { operation, path, source }`, enabling a future agent to localize corrupt or missing TOML files by file path alone
- `milestone_scan` skips non-`.toml` files silently; emits per-entry `AssayError::Io` on unreadable entries with the milestones dir path

## Deviations

The task plan said to map toml parse errors to `AssayError::Io`. The plan didn't specify how to wrap `toml::ser::Error` / `toml::de::Error` (which are not `std::io::Error`). Used `std::io::Error::new(ErrorKind::InvalidData, e.to_string())` to wrap them, consistent with what other modules do when mapping non-IO errors into `AssayError::Io`. This avoids adding a new error variant and keeps the error surface stable.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/milestone/mod.rs` — new module with `milestone_scan`, `milestone_load`, `milestone_save` (all `pub`)
- `crates/assay-core/src/lib.rs` — added `pub mod milestone`
- `crates/assay-core/tests/milestone_io.rs` — 5 integration tests
