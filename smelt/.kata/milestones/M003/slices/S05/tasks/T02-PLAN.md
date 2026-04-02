---
estimated_steps: 6
estimated_files: 4
---

# T02: Doc comments for error.rs, forge.rs, manifest.rs, git/mod.rs

**Slice:** S05 — smelt-core Library API
**Milestone:** M003

## Description

Mechanical doc comment pass across four files, eliminating 34 of the 52 missing_docs errors. All additions are `///` one-liners on named enum variant fields and struct fields. After T01, the running missing_docs error count is 52 (T01 doesn't reduce it since the assay types already had docs). After this task it drops to 18 (only monitor.rs remains). Strategy: read each file, add docs, verify that file is clean, move to the next.

Current error distribution:
- `error.rs`: 17 — named fields in thiserror enum variants
- `forge.rs`: 10 — PrState/CiStatus variants + PrStatus fields
- `manifest.rs`: 2 — `source` fields in `CredentialStatus` variants
- `git/mod.rs`: 5 — `GitWorktreeEntry` struct fields

## Steps

1. **error.rs (17 items)** — Read the file. For every named field in `SmeltError` enum variants, add a `/// short description` above each field. The fields (by variant) are:
   - `GitExecution { operation, message }` → doc each field
   - `MergeConflict { operation, message }` → doc each
   - `Manifest { field, session }` → doc each
   - `InvalidRepoPath { files }` → doc the `files` field (Vec of invalid paths)
   - `Provider { provider, path, source }` → doc all three including the `#[source]` field
   - `Forge { operation, message }` → doc each (mirrors GitExecution pattern)
   - `Credential { env_var }` → doc the field
   - `Config { key, path }` → doc each
   - `Io { path, source }` → doc each
   After: `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "error\.rs" | wc -l` → 0

2. **forge.rs (10 items)** — Read the file. Add `///` docs to:
   - `PrState` variants: `Open`, `Merged`, `Closed`
   - `CiStatus` variants: `Pending`, `Passing`, `Failing`, `Unknown`
   - `PrStatus` fields: `state`, `ci_status`, `review_count`
   After: `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "forge\.rs" | wc -l` → 0

3. **manifest.rs (2 items)** — The two errors are at lines 353 and 355: the `source: String` fields inside `CredentialStatus::Resolved { source }` and `CredentialStatus::Missing { source }`. Add `///` docs inline within the enum variant bodies. After: `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "manifest\.rs" | wc -l` → 0

4. **git/mod.rs (5 items)** — Read the file. The 5 missing items are the fields of `GitWorktreeEntry`: `path`, `head`, `branch`, `is_bare`, `is_locked`. Keep the struct `pub` — it is used in the return type of `GitOps::worktree_list()`. Add `///` doc above each field. After: `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "git/mod\.rs" | wc -l` → 0

5. **Full verification** — Run `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "\.rs:" | grep -v "monitor\.rs"` — should produce zero lines (only monitor.rs errors may remain). Also run with forge feature: `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1 | grep "\.rs:" | grep -v "monitor\.rs"` → also zero.

6. **Test regression check** — `cargo test -p smelt-core --features forge -q` → all pass.

## Must-Haves

- [ ] Every named field in `SmeltError` variants has a `///` doc comment (17 items)
- [ ] All `PrState` and `CiStatus` variants have `///` doc comments (7 items)
- [ ] All `PrStatus` struct fields have `///` doc comments (3 items)
- [ ] Both `source` fields in `CredentialStatus` variants have `///` doc comments (2 items)
- [ ] All 5 `GitWorktreeEntry` fields have `///` doc comments
- [ ] `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "\.rs:" | grep -v "monitor\.rs"` → zero output
- [ ] `cargo test -p smelt-core --features forge -q` passes

## Verification

```bash
# After each file: check that file is clean
RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "error\.rs" | wc -l  # → 0
RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "forge\.rs" | wc -l  # → 0
RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "manifest\.rs" | wc -l  # → 0
RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "git/mod\.rs" | wc -l  # → 0

# Final: only monitor.rs errors remain
RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "^error: missing" | wc -l
# expected: 18 (the monitor.rs errors; will be fixed in T03)

# No test regressions
cargo test -p smelt-core --features forge -q 2>&1 | tail -3
```

## Observability Impact

- Signals added/changed: None — doc comments are stripped at runtime
- How a future agent inspects this: `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "^error: missing" | wc -l` — the remaining count is the authoritative progress signal
- Failure state exposed: None at runtime; any regression shows as a doc-error line with exact file:line

## Inputs

- T01 completed — assay types demoted, Cargo metadata added, lib.rs doc enhanced
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep -A1 "missing documentation" | grep "\.rs:"` — run this first to confirm the exact 34 locations before editing

## Expected Output

- `crates/smelt-core/src/error.rs` — 17 new `///` doc comments on named enum variant fields
- `crates/smelt-core/src/forge.rs` — 10 new `///` doc comments on enum variants and struct fields
- `crates/smelt-core/src/manifest.rs` — 2 new `///` doc comments on `CredentialStatus` variant `source` fields
- `crates/smelt-core/src/git/mod.rs` — 5 new `///` doc comments on `GitWorktreeEntry` fields
