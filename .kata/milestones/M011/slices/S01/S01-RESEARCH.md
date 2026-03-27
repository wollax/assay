# S01: Decompose manifest.rs and git/cli.rs — Research

**Date:** 2026-03-24
**Domain:** Rust module decomposition
**Confidence:** HIGH

## Summary

Both files are dominated by tests — `manifest.rs` is 1924L total but only 525L of implementation (1399L tests), and `git/cli.rs` is 1365L total but only 330L of implementation (1035L tests). The implementation code in both files is already well under 500L. The decomposition challenge is primarily about distributing tests into coherent groups that co-locate with the code they exercise.

The M009 decomposition established the exact pattern to follow (D128): convert `foo.rs` → `foo/mod.rs` + child modules, re-export all public items from `mod.rs`. The `git/` directory already uses this pattern (`git/mod.rs` + `git/cli.rs`), so `cli.rs` becomes `cli/mod.rs` + child modules. For `manifest.rs`, it becomes `manifest/mod.rs` + child modules.

## Recommendation

### manifest.rs (1924L → target <500L each)

Natural seams for splitting:

1. **`manifest/mod.rs`** (~200L) — All struct definitions (`JobManifest`, `Environment`, `SessionDef`, `ComposeService`, `KubernetesConfig`, `JobMeta`, `CredentialConfig`, `MergeConfig`, `ValidationErrors`, `CredentialStatus`), `load()`/`from_str()`, `resolve_credentials()`, and `resolve_repo_path()`. Re-exports everything public.
2. **`manifest/validation.rs`** (~250L) — `validate()` method and `detect_cycle()` helper. These form one cohesive unit (210 lines of validation logic). Extract as a private function that takes `&JobManifest` and call from `impl JobManifest` in mod.rs.
3. **`manifest/tests.rs`** — The 48 tests split naturally into groups:
   - **Core parsing/validation tests** (~26 tests, ~620L) — `parse_valid_manifest`, `validate_*` for basic fields, sessions, dependencies, cycles, multiple errors, credentials.
   - **Forge tests** (~5 tests, ~200L) — `test_parse_manifest_with_forge`, `test_validate_forge_*`, `test_forge_deny_unknown_fields`.
   - **Compose/services tests** (~8 tests, ~300L) — `test_compose_*`, `test_validate_compose_*`, `test_validate_services_*`.
   - **Kubernetes tests** (~9 tests, ~280L) — `test_kubernetes_*`, `test_validate_kubernetes_*`, `test_validate_runtime_*`.

Test fixtures (`VALID_MANIFEST`, `VALID_COMPOSE_MANIFEST`, `load_from_str`) are shared across test groups and need to be accessible from all test modules.

### git/cli.rs (1365L → target <500L each)

The `GitCli` struct has only 2 private methods (`run`, `run_in`) and the `GitOps` trait impl with ~25 async methods. The implementation is already compact (330L). The 29 tests (1035L) are the bulk. Natural seams:

1. **`cli/mod.rs`** (~330L) — The full `GitCli` struct + `GitOps` impl stays together. All methods are tightly coupled (they all call `run()` or `run_in()`). Splitting the trait impl across files would be awkward and non-idiomatic.
2. **`cli/tests.rs`** — Split into groups by operation family:
   - **Basic ops tests** (~5 tests, ~200L) — `test_repo_root`, `test_current_branch`, `test_head_short`, `test_is_inside_work_tree`, `test_rev_parse`.
   - **Worktree tests** (~5 tests, ~250L) — `test_worktree_add_and_list`, `test_worktree_remove`, `test_worktree_is_dirty`, `test_worktree_add_existing`.
   - **Branch tests** (~5 tests, ~250L) — `test_branch_exists`, `test_branch_delete`, `test_branch_is_merged`, `test_branch_create`, `test_merge_base`.
   - **Commit/diff tests** (~8 tests, ~350L) — `test_add_and_commit`, `test_commit_returns_valid_hash`, `test_rev_list_count`, `test_add_specific_paths`, `test_add_and_commit_in_worktree`, `test_diff_numstat`, `test_diff_name_only*`, `test_log_subjects*`.
   - **Merge/conflict tests** (~4 tests, ~250L) — `test_merge_squash_clean`, `test_merge_squash_conflict`, `test_reset_hard`, `test_unmerged_files_empty_when_clean`.
   - **Fetch tests** (~1 test, ~100L) — `test_fetch_ref_creates_local_branch`.

The `setup_test_repo()` fixture is shared across all test groups.

**Key decision:** Whether to split the `validate()` method body into a separate file vs. keeping it inline. Recommendation: extract into `manifest/validation.rs` as a module-private function `pub(super) fn validate_manifest(m: &JobManifest) -> crate::Result<()>` plus `detect_cycle()`. This keeps the `impl JobManifest` in `mod.rs` thin (delegating to the validation module) while making the validation logic independently navigable.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Module re-export pattern | D128 established pattern in M009/S03 | Proven in this codebase; callers unaffected |
| Test co-location | D129 established pattern | Tests follow implementation module |

## Existing Code and Patterns

- `crates/smelt-cli/src/commands/run/mod.rs` — M009 decomposition exemplar. File-to-directory with `mod phases; mod dry_run; mod helpers;` and selective re-exports.
- `crates/smelt-cli/src/serve/ssh/mod.rs` — Another M009 decomposition. `pub use operations::*;` pattern for re-exporting, plus `pub(crate) mod tests` compatibility shim (D130).
- `crates/smelt-core/src/git/mod.rs` — Already a directory module. `cli.rs` is a child. Adding more children (`cli/mod.rs` + test files) follows the existing structure.
- `crates/smelt-cli/src/serve/tests/mod.rs` — Test distribution exemplar from M009. Tests split into `queue.rs`, `http.rs`, `dispatch.rs`, `ssh_dispatch.rs`, `config.rs`.

## Constraints

- `#![deny(missing_docs)]` enforced on smelt-core (D070/D127) — all new public items need doc comments. Since we're only re-exporting existing items, no new docs needed unless new public items are introduced.
- Re-exports must preserve all existing import paths: `smelt_core::manifest::JobManifest`, `smelt_core::manifest::resolve_repo_path`, `smelt_core::manifest::CredentialStatus`, etc.
- The `git/cli.rs` → `git/cli/mod.rs` conversion means `git/mod.rs` must change `mod cli;` (still works — Rust resolves either `cli.rs` or `cli/mod.rs`).
- Test helper functions (`setup_test_repo()`, `load_from_str()`, `VALID_MANIFEST` const) must be accessible from all test submodules. Use `pub(super)` visibility or define in a shared test utility module.

## Common Pitfalls

- **Breaking import paths** — If any `pub use` re-export is missed, downstream crates (`smelt-cli`, integration tests) will fail to compile. Mitigation: run `cargo build --workspace` after the structural change, before moving any code. The compiler will immediately surface missing re-exports.
- **Test fixture visibility** — Test constants like `VALID_MANIFEST` are `const` inside `#[cfg(test)] mod tests`. When splitting tests into submodules, the parent `mod tests` must make these accessible via `pub(super)` or the submodules need their own copies. The M009 pattern (D130) shows how to handle this with a re-export shim. Recommendation: define shared fixtures in the parent test module and import from child modules.
- **`impl` block split across files** — Rust allows `impl Foo` blocks in any file within the same crate. For `validate()` extraction, the cleanest approach is a free function in `validation.rs` called by a thin `pub fn validate(&self)` wrapper in `mod.rs`. This avoids splitting `impl JobManifest` across files, which is legal but confusing.
- **Cargo test filtering** — After moving tests to submodules, test names change (e.g., `manifest::tests::parse_valid_manifest` → `manifest::tests::core::parse_valid_manifest`). No external CI filter should break since tests are typically run with `cargo test --workspace`, but worth verifying.

## Open Risks

- **None significant.** This is a mechanical restructuring with a proven pattern (D128). The implementation code fits comfortably under 500L in both cases. The main work is distributing 2400+ lines of tests into coherent submodules without breaking anything.

## Skills Discovered

No specialized skills needed — this is pure Rust module restructuring using established codebase patterns.

| Technology | Skill | Status |
|------------|-------|--------|
| Rust module system | N/A — no external skill needed | N/A |

## Sources

- D126 (500L threshold), D128 (file-to-directory module conversion), D129 (tests follow implementation), D130 (SSH tests re-export shim) — all from `.kata/DECISIONS.md`
- M009/S03 — prior decomposition of `run.rs`, `ssh.rs`, `serve/tests.rs` — established the pattern
- Direct codebase exploration of `manifest.rs` (1924L: 525 impl + 1399 tests) and `git/cli.rs` (1365L: 330 impl + 1035 tests)
