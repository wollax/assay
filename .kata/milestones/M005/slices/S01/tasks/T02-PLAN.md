---
estimated_steps: 6
estimated_files: 4
---

# T02: Implement milestone_load, milestone_save, milestone_scan in assay-core

**Slice:** S01 — Milestone & Chunk Type Foundation
**Milestone:** M005

## Description

Create the `assay-core::milestone` module with three public functions: `milestone_scan`, `milestone_load`, and `milestone_save`. These follow the atomic tempfile-rename pattern proven in `work_session.rs` and the slug validation from `history::validate_path_component`. This is the I/O foundation that MCP tools, the CLI, and the S02 cycle state machine will all call.

The module lives at `crates/assay-core/src/milestone/mod.rs` (D064). Slug = filename stem (e.g., `my-feature.toml` → slug `my-feature`). All writes are atomic (NamedTempFile + sync_all + persist). Missing `.assay/milestones/` directory is not an error for scan — return empty vec.

## Steps

1. Create `crates/assay-core/src/milestone/mod.rs`. Add the three public functions:
   - `pub fn milestone_scan(assay_dir: &Path) -> Result<Vec<Milestone>>` — constructs `milestones_dir = assay_dir.join("milestones")`; returns `Ok(vec![])` if the directory doesn't exist (use `if !milestones_dir.exists()`); reads entries with `std::fs::read_dir`; for each entry, checks the extension is `.toml`; reads file content, parses with `toml::from_str::<Milestone>` (slug is in the TOML — no need to set from filename); returns all parsed milestones sorted by slug.
   - `pub fn milestone_load(assay_dir: &Path, slug: &str) -> Result<Milestone>` — calls `crate::history::validate_path_component(slug, "milestone slug")`; constructs path `assay_dir/milestones/<slug>.toml`; reads with `std::fs::read_to_string`, wraps IO errors as `AssayError::Io`; parses with `toml::from_str::<Milestone>`; sets `milestone.slug = slug.to_string()` after parsing.
   - `pub fn milestone_save(assay_dir: &Path, milestone: &Milestone) -> Result<()>` — calls `validate_path_component(&milestone.slug, "milestone slug")`; creates `assay_dir/milestones/` with `std::fs::create_dir_all`; serializes with `toml::to_string`; creates `NamedTempFile` in the milestones dir; writes content, calls `file.flush()` then `file.as_file().sync_all()`; calls `file.persist(path)` (path = `milestones_dir/<slug>.toml`). Map errors to `AssayError::Io { operation, path, source }`.

2. Add `pub mod milestone;` to `crates/assay-core/src/lib.rs`.

3. Verify `assay-types` is imported correctly: `assay-types` is already a dep of `assay-core`. Use `assay_types::{Milestone, MilestoneStatus}` imports at the top of `milestone/mod.rs`.

4. Verify the `toml` crate is available in `assay-core` — it is listed as `toml.workspace = true` in `assay-core/Cargo.toml`. Confirm by checking `Cargo.toml`.

5. Create `crates/assay-core/tests/milestone_io.rs` with tests using `tempfile::TempDir`:
   - `test_milestone_save_and_load_roundtrip` — saves a `Milestone` and loads it back; asserts all fields equal
   - `test_milestone_scan_empty_for_missing_dir` — scans a fresh temp assay dir (no `milestones/` subdir); asserts `Ok(vec![])`
   - `test_milestone_scan_returns_all_milestones` — saves two milestones, scans; asserts both slugs present
   - `test_milestone_load_error_for_missing_slug` — loads nonexistent slug; asserts `Err` with path info
   - `test_milestone_slug_validation_rejects_traversal` — calls `milestone_save` with slug `"../evil"`; asserts `Err`

6. Run `cargo test -p assay-core --test milestone_io` to confirm all 5 tests pass. Run `cargo test -p assay-core` to confirm no regressions.

## Must-Haves

- [ ] `milestone_scan` returns `Ok(vec![])` for a directory with no `milestones/` subdir
- [ ] `milestone_save` + `milestone_load` round-trip with equal result (all fields)
- [ ] `milestone_scan` returns all saved milestones when `milestones/` dir exists with `.toml` files
- [ ] `milestone_load` returns `Err` with path info for a nonexistent slug
- [ ] `milestone_save` rejects traversal slugs (e.g., `"../evil"`)
- [ ] `pub mod milestone` is accessible from `assay-core` crate root
- [ ] `cargo test -p assay-core` fully green

## Verification

- `cargo test -p assay-core --test milestone_io` — all 5 tests pass
- `cargo test -p assay-core` — no regressions in existing test suite
- `grep -r "pub mod milestone" crates/assay-core/src/lib.rs` — confirms module is public

## Observability Impact

- Signals added/changed: `milestone_load` returns `AssayError::Io { operation: "reading milestone", path: <file_path>, source }` on any failure; parse errors mapped to `AssayError::Io` with `operation: "parsing milestone TOML"`
- How a future agent inspects this: `cargo test -p assay-core --test milestone_io` is the verification surface; errors carry the full file path so the agent knows exactly which file is corrupt or missing
- Failure state exposed: `milestone_scan` propagates read errors per-entry with the file path; a future agent can localize which `.toml` file in `.assay/milestones/` is causing a parse failure

## Inputs

- `crates/assay-types/src/milestone.rs` — `Milestone`, `ChunkRef`, `MilestoneStatus` types (produced by T01)
- `crates/assay-core/src/history/mod.rs` — `validate_path_component` (pub(crate), accessible within assay-core)
- `crates/assay-core/src/work_session.rs` — atomic tempfile-rename pattern (`NamedTempFile`, `sync_all`, `persist`)
- `crates/assay-core/src/error.rs` — `AssayError::Io` variant shape
- `crates/assay-core/src/spec/mod.rs` — scan/load pattern for directory-based specs

## Expected Output

- `crates/assay-core/src/milestone/mod.rs` — new file with `milestone_scan`, `milestone_load`, `milestone_save` (all `pub`)
- `crates/assay-core/src/lib.rs` — `pub mod milestone` added
- `crates/assay-core/tests/milestone_io.rs` — new test file with 5 integration tests
