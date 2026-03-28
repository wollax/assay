# S01: GitHubBackend correctness fixes (Q001–Q004) — Research

**Date:** 2026-03-27

## Summary

This slice addresses four correctness issues in `crates/assay-backends/src/github.rs` and `factory.rs`, all identified during PR #193 review. The scope is small and entirely contained within the `assay-backends` crate. No architectural changes, no new dependencies, no new crate boundaries.

Q001 (repo validation) and Q002 (issue-0 rejection) are the substantive fixes — they prevent silent runtime failures by catching bad inputs early. Q003 (error helper extraction) is a pure refactor that reduces ~30 lines of duplicated error-handling boilerplate across three `GhRunner` methods. Q004 (doc cleanup) is a one-line edit to `factory.rs`.

All four items are leaf changes with no cross-crate impact. The existing 8 contract tests in `tests/github_backend.rs` provide a solid test harness with mock `gh` scripts and PATH override — new tests follow the same pattern.

## Recommendation

Implement all four fixes in a single slice with test-first approach:

1. **T01: Write contract tests** for Q001 (warn on malformed repo), Q002 (reject issue 0), Q003 (error helper used by all three methods), Q004 (no milestone identifiers in factory doc).
2. **T02: Implement fixes** — add `tracing::warn!` in `GitHubBackend::new`, add `0` guard in `read_issue_number`, extract `GhRunner::gh_error()`, clean factory.rs doc.
3. **T03: Verify** — `just ready` green, new tests pass, existing 8 github_backend tests unchanged.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Mock gh binary for tests | `write_mock_gh()` + `with_mock_gh_path()` in `tests/github_backend.rs` | Battle-tested helpers with PATH override + `#[serial]` isolation |
| Error construction | `AssayError::io(operation, path, io_error)` | Established pattern across all backends |
| Tracing assertions | `tracing-test` crate (workspace dev-dep, D135) | Used in M009 span tests; `#[traced_test]` + `logs_contain()` |

## Existing Code and Patterns

- `crates/assay-backends/src/github.rs` lines 79–93, 150–165, 192–207 — Three nearly identical error-handling blocks in `create_issue`, `create_comment`, `get_issue_json`. Each does: `String::from_utf8_lossy(stderr)` → `tracing::warn!(repo, exit_code, stderr)` → `Err(AssayError::io(...))`. Extract to `GhRunner::gh_error(operation: &str, output: &Output) -> AssayError`.
- `crates/assay-backends/src/github.rs` line 253 — `GitHubBackend::new` is infallible (`-> Self`). D177 mandates warn-not-error pattern: keep infallible constructor, add `tracing::warn!` when `repo` is empty or missing `/`. Do NOT change signature to `Result<Self>`.
- `crates/assay-backends/src/github.rs` lines 263–286 — `read_issue_number` parses `u64` from file contents. Currently any valid `u64` is accepted including `0`. Add a guard: `if number == 0 { return Err(...) }`.
- `crates/assay-backends/tests/github_backend.rs` — 8 existing contract tests with `#[serial]`, mock gh scripts, PATH override. New tests follow this exact pattern.
- `crates/assay-backends/src/factory.rs` lines 12–18 — Doc comment mentions `(M011/S02)`, `(M011/S03)`, `(M011/S04)`. Clean these to remove milestone identifiers.
- `crates/assay-backends/src/linear.rs` — LinearBackend's `new()` takes an API key parameter. Different pattern from GitHubBackend (no env-var-at-construction). Not relevant to Q001–Q004 but confirms D169/D172 divergence.

## Constraints

- **D177: warn-not-error at construction** — `GitHubBackend::new` must remain infallible (`-> Self`). Changing to `Result<Self>` would break `backend_from_config` (returns `Arc<dyn StateBackend>`, not `Result`) and all 8 existing test sites. Emit `tracing::warn!` only.
- **D172: no env-var gate for GitHub** — Unlike LinearBackend, GitHubBackend has no env-var check at construction. Q001 validates the `repo` format only, not `gh` CLI availability.
- **`#[serial]` requirement** — All tests using `with_mock_gh_path` must be `#[serial]` due to PATH environment mutation. Already established pattern.
- **Feature gate** — Tests in `tests/github_backend.rs` are gated with `#![cfg(feature = "github")]`. New tests go in the same file.
- **`just ready` uses `cargo nextest run --workspace`** — nextest does NOT pass `--features github` by default. The github_backend tests may only run when explicitly invoked with `cargo test -p assay-backends --features github`. Verify this is the case; new tests must also be exercisable this way.

## Common Pitfalls

- **Q001 validation too strict** — Don't reject repos with orgs that contain dots or hyphens (e.g. `my-org/my-repo`). The only validation is: non-empty AND contains at least one `/`. A full regex for GitHub owner/repo format is overkill and fragile.
- **Q002 returning `Err` vs `Ok(None)` for issue 0** — `read_issue_number` currently returns `Ok(Some(n))` for any valid u64. Issue 0 should return `Err` (not `Ok(None)`) because 0 indicates file corruption, not absence. `Ok(None)` means "no file exists" which is a different semantic.
- **Q003 error helper signature** — The helper must take `&self` (needs `self.repo` for the tracing field) and the `operation` string. Consider `fn gh_error(&self, operation: &str, output: &std::process::Output) -> AssayError` which encapsulates the stderr extraction, warn!, and error construction.
- **Q004 scope creep** — Only clean the milestone identifiers from the doc comment (e.g. `(M011/S02)`). Do NOT restructure the doc comment or change its meaning.

## Open Risks

- **`just ready` may not exercise github feature tests** — If nextest doesn't pass `--features github`, the new Q001–Q004 tests won't run in CI. Verify the test invocation and document if manual `--features github` is needed.
- **tracing-test availability in assay-backends** — Confirm `tracing-test` is a dev-dep of `assay-backends`. If not, Q001's warn assertion test needs it added. Alternative: test Q001 indirectly by verifying the backend still constructs successfully (warn doesn't block construction).

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / Cargo | N/A — standard Rust, no external framework | none needed |

No external technologies or frameworks are involved. This slice is pure Rust refactoring and correctness fixes within existing code.

## Sources

- `crates/assay-backends/src/github.rs` — full implementation read
- `crates/assay-backends/src/factory.rs` — full implementation read
- `crates/assay-backends/tests/github_backend.rs` — 8 existing contract tests, test harness patterns
- D177, D170, D172 from DECISIONS.md — governing decisions for GitHubBackend
- R081 from REQUIREMENTS.md — primary requirement this slice owns
