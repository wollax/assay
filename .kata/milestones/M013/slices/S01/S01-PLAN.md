# S01: GitHubBackend correctness fixes (Q001–Q004)

**Goal:** `GitHubBackend::new` warns on malformed repo, `read_issue_number` rejects issue `0`, `GhRunner` error helper extracted, factory.rs doc cleaned — all proven by contract tests.
**Demo:** `cargo test -p assay-backends --features github` passes with new Q001–Q004 tests; `just ready` green; zero `(M011/S` in factory.rs doc.

## Must-Haves

- Q001: `GitHubBackend::new` emits `tracing::warn!` when `repo` is empty or missing `/` (D177: stays infallible, no `Result<Self>`)
- Q002: `read_issue_number` returns `Err` when parsed number is `0` (corruption signal, not `Ok(None)`)
- Q003: `GhRunner::gh_error` helper extracted — all three methods (`create_issue`, `create_comment`, `get_issue_json`) use it
- Q004: factory.rs doc comment cleaned of `(M011/S02)`, `(M011/S03)`, `(M011/S04)` identifiers

## Proof Level

- This slice proves: contract
- Real runtime required: no (mock `gh` scripts + tracing-test subscriber)
- Human/UAT required: no

## Verification

- `cargo test -p assay-backends --features github` — all existing 8 tests + new Q001–Q004 tests pass
- `just ready` — full workspace green (1529+ tests)
- `grep -c '(M011/S' crates/assay-backends/src/factory.rs` returns `0`
- Q001 warn test: `tracing-test` `logs_contain("malformed")` assertion for empty repo and missing-slash repo
- Q002 issue-0 test: `read_issue_number` returns `Err` when file contains `"0"`
- Q003 refactor: `grep -c "from_utf8_lossy" crates/assay-backends/src/github.rs` returns `1` (only inside `gh_error`)

## Observability / Diagnostics

- Runtime signals: `tracing::warn!` event with `repo` field when `GitHubBackend::new` receives malformed repo
- Inspection surfaces: `tracing-test` subscriber capture in tests; `RUST_LOG=warn` at runtime
- Failure visibility: `read_issue_number` `Err` message includes the literal `"0"` and path for diagnosability
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `crates/assay-backends/src/github.rs` (all changes), `crates/assay-backends/src/factory.rs` (doc only)
- New wiring introduced in this slice: none (leaf fixes, no new API surface)
- What remains before the milestone is truly usable end-to-end: S02 (trace viewer), S03 (OTel metrics), S04 (wizard cmd field)

## Tasks

- [ ] **T01: Write contract tests for Q001–Q004** `est:30m`
  - Why: Test-first — define the contracts before implementing. Tests should fail/not compile initially.
  - Files: `crates/assay-backends/tests/github_backend.rs`, `crates/assay-backends/Cargo.toml`
  - Do: Add `tracing-test` dev-dep to assay-backends. Write 4 new tests: (1) Q001 warn on empty repo via `#[traced_test]` + `logs_contain`, (2) Q001 warn on repo missing `/`, (3) Q002 `read_issue_number` rejects `0`, (4) Q003 assert `gh_error` is used (tested indirectly via existing error tests). Add a Q004 `grep` assertion to verification. Keep `#[serial]` on PATH-mutating tests; Q001/Q002/Q004 tests don't need `#[serial]`.
  - Verify: Tests compile but Q001/Q002 fail (no warn emitted yet); Q003 fails (method doesn't exist yet)
  - Done when: 4+ new test functions exist in `github_backend.rs` targeting Q001–Q004 behavior

- [ ] **T02: Implement Q001–Q004 fixes** `est:30m`
  - Why: Make all new tests pass by implementing the actual fixes.
  - Files: `crates/assay-backends/src/github.rs`, `crates/assay-backends/src/factory.rs`
  - Do: (1) Q001: Add `tracing::warn!` in `GitHubBackend::new` when `repo.is_empty() || !repo.contains('/')`. Keep constructor infallible (D177). (2) Q002: Add `if number == 0 { return Err(...) }` guard in `read_issue_number` after successful parse. Error message must include `"0"` and path. (3) Q003: Extract `GhRunner::gh_error(&self, operation: &str, output: &std::process::Output) -> AssayError` that does `from_utf8_lossy(stderr)` → `tracing::warn!` → `Err(AssayError::io(...))`. Replace the 3 duplicated blocks in `create_issue`, `create_comment`, `get_issue_json`. (4) Q004: Remove `(M011/S02)`, `(M011/S03)`, `(M011/S04)` from factory.rs doc comment.
  - Verify: `cargo test -p assay-backends --features github` — all 12+ tests pass; `grep -c '(M011/S' crates/assay-backends/src/factory.rs` returns `0`; `grep -c "from_utf8_lossy" crates/assay-backends/src/github.rs` returns `1`
  - Done when: All new Q001–Q004 tests pass, all 8 existing tests still pass, factory.rs doc clean

- [ ] **T03: Full workspace verification** `est:15m`
  - Why: Confirm no regressions across the full workspace after the changes.
  - Files: none (verification-only task)
  - Do: Run `just ready` (fmt, lint, test, deny). Fix any clippy warnings or formatting issues introduced by the changes.
  - Verify: `just ready` exits 0 with 1529+ tests passing
  - Done when: `just ready` green, no new warnings

## Files Likely Touched

- `crates/assay-backends/src/github.rs`
- `crates/assay-backends/src/factory.rs`
- `crates/assay-backends/tests/github_backend.rs`
- `crates/assay-backends/Cargo.toml`
