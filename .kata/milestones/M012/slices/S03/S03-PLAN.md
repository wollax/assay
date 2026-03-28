# S03: GitHub Issues Tracker Backend

**Goal:** `GithubTrackerSource` polls GitHub Issues via `gh` CLI, transitions lifecycle labels, and generates manifests from templates — proven by unit tests with mock `gh` client and integration tests gated by `SMELT_GH_TEST=1`.
**Demo:** Unit tests exercise full `GhClient` trait contract (list issues, add/remove labels, create labels, auth check) via `MockGhClient`. `GithubTrackerSource` implements `TrackerSource` trait with double-dispatch prevention (ready→queued before enqueue). Integration tests against a real repo run with `SMELT_GH_TEST=1`.

## Must-Haves

- `GhClient` trait defined with methods: `list_issues`, `edit_labels`, `create_label`, `auth_status`
- `SubprocessGhClient` implements `GhClient` via `gh` CLI subprocess (mirrors `SubprocessSshClient` pattern)
- `MockGhClient` with VecDeque-based test double (mirrors `MockSshClient` pattern)
- `GithubTrackerSource` implements `TrackerSource` from S02 using `GhClient`
- `TrackerConfig.repo` field added (`Option<String>`), validated when `provider == "github"`
- Binary discovery via `which::which("gh")` with structured `SmeltError::Tracker` on missing binary
- Auth check via `gh auth status` with structured error on failure
- Label auto-creation via `gh label create --force` (idempotent)
- Double-dispatch prevention: `smelt:ready → smelt:queued` transition before enqueue (D157)
- `--limit 50` cap on issue listing
- All `gh` commands use `-R owner/repo` explicitly (never infer from CWD)
- Unit tests: ≥12 tests covering GhClient mock, GithubTrackerSource happy path, error paths, config validation
- Integration tests gated by `SMELT_GH_TEST=1` + `SMELT_GH_REPO=owner/repo`
- All existing 337+ workspace tests pass (zero regressions)

## Proof Level

- This slice proves: contract + integration (mock-based unit tests + gated real `gh` CLI tests)
- Real runtime required: no (mock tests run without `gh`; integration tests are gated)
- Human/UAT required: no (integration tests prove real `gh` CLI interaction)

## Verification

- `cargo test -p smelt-cli --lib -- serve::github` — all GhClient and GithubTrackerSource unit tests pass
- `cargo test -p smelt-cli --lib -- serve::config` — repo field validation tests pass
- `cargo test --workspace` — all 337+ tests pass, 0 failures, 0 regressions
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings
- Integration: `SMELT_GH_TEST=1 SMELT_GH_REPO=owner/repo cargo test -p smelt-cli -- --ignored github` — gh CLI tests pass (manual/CI)

## Observability / Diagnostics

- Runtime signals: `tracing::debug!` on `gh` subprocess invocations (command + args); `tracing::warn!` on non-zero exit codes; `tracing::info!` on successful label creation and issue transitions
- Inspection surfaces: `SmeltError::Tracker { operation, message }` for all GitHub-specific errors; structured fields include repo, issue number, label names
- Failure visibility: Missing `gh` binary → `SmeltError::Tracker { operation: "gh_binary", message }`; auth failure → `SmeltError::Tracker { operation: "auth_status", message }`; label transition failure → logged and issue skipped (not fatal to poller)
- Redaction constraints: None — no secrets flow through `gh` CLI (it uses its own auth store)

## Integration Closure

- Upstream surfaces consumed: `TrackerSource` trait, `TrackerConfig`, `TrackerIssue`, `TrackerState`, `issue_to_manifest()`, `SmeltError::Tracker` from S02
- New wiring introduced in this slice: `GithubTrackerSource` struct implementing `TrackerSource`; `repo` field on `TrackerConfig`; `serve/github/` module hierarchy
- What remains before the milestone is truly usable end-to-end: S04 (Linear backend), S05 (TrackerPoller integration into `smelt serve` dispatch loop, state_backend passthrough, TUI display)

## Tasks

- [x] **T01: GhClient trait, SubprocessGhClient, and MockGhClient** `est:45m`
  - Why: Foundation layer — defines the `gh` CLI abstraction boundary and test double, following the `SshClient`/`SubprocessSshClient`/`MockSshClient` pattern exactly. All subsequent tasks build on this.
  - Files: `crates/smelt-cli/src/serve/github/mod.rs`, `crates/smelt-cli/src/serve/github/client.rs`, `crates/smelt-cli/src/serve/github/mock.rs`, `crates/smelt-cli/src/serve/mod.rs`
  - Do: Create `serve/github/` module with `GhClient` trait (RPITIT per D019), `SubprocessGhClient` (which::which + tokio::process::Command), `MockGhClient` (VecDeque pattern), `GhIssue` struct for parsed JSON output. Methods: `list_issues(repo, label, limit)`, `edit_labels(repo, number, add, remove)`, `create_label(repo, name)`, `auth_status()`. Add `pub mod github` to `serve/mod.rs`.
  - Verify: `cargo test -p smelt-cli --lib -- serve::github` — mock tests pass; `cargo clippy --workspace -- -D warnings` clean
  - Done when: GhClient trait compiles with RPITIT, SubprocessGhClient uses `gh` binary, MockGhClient exercises the VecDeque pattern, ≥8 unit tests pass

- [x] **T02: GithubTrackerSource implementing TrackerSource** `est:45m`
  - Why: Wires `GhClient` to `TrackerSource` trait — the actual business logic of polling issues, transitioning labels, and preventing double-dispatch (D157). This is the core delivery for R070.
  - Files: `crates/smelt-cli/src/serve/github/mod.rs`, `crates/smelt-cli/src/serve/github/source.rs`
  - Do: Implement `GithubTrackerSource<G: GhClient>` with `poll_ready_issues()` (list issues by label, map `GhIssue` → `TrackerIssue`) and `transition_state()` (edit_labels to swap from→to labels, create labels if needed with `--force`). `poll_ready_issues` calls `auth_status()` first, uses `--limit 50`, and returns `SmeltError::Tracker` on failures. `transition_state` does `edit_labels(add=[to], remove=[from])` in a single call. Add `ensure_labels()` helper that calls `create_label --force` for all `TrackerState::ALL` variants.
  - Verify: `cargo test -p smelt-cli --lib -- serve::github` — GithubTrackerSource tests pass using MockGhClient; transitions, empty results, auth failures, and label creation all covered
  - Done when: `GithubTrackerSource` implements `TrackerSource` trait; ≥6 tests cover happy path, empty poll, auth failure, transition success, transition failure, ensure_labels

- [x] **T03: TrackerConfig.repo field, validation, and integration tests** `est:30m`
  - Why: Closes the config loop — `repo` is required for GitHub but optional for Linear; validates at startup. Adds gated integration tests against a real `gh` CLI.
  - Files: `crates/smelt-cli/src/serve/config.rs`, `crates/smelt-cli/src/serve/github/mod.rs`
  - Do: Add `repo: Option<String>` to `TrackerConfig` with `#[serde(default)]`. In `ServerConfig::validate()`, when `provider == "github"`: require `repo` is `Some` and matches `owner/repo` format (contains exactly one `/`). Add config tests for repo validation. Add `#[ignore]` integration tests gated by `SMELT_GH_TEST=1` + `SMELT_GH_REPO` that exercise `SubprocessGhClient::auth_status()` and `SubprocessGhClient::list_issues()` against a real repo.
  - Verify: `cargo test -p smelt-cli --lib -- serve::config` — repo validation tests pass; `cargo test --workspace` — all tests pass, zero regressions; `cargo clippy` + `cargo doc` clean
  - Done when: `TrackerConfig` has `repo` field; GitHub provider requires it; config tests cover present/absent/invalid repo; integration tests exist (gated); all 337+ workspace tests pass

## Files Likely Touched

- `crates/smelt-cli/src/serve/github/mod.rs` — GhClient trait, GhIssue, GithubTrackerSource, re-exports
- `crates/smelt-cli/src/serve/github/client.rs` — SubprocessGhClient implementation
- `crates/smelt-cli/src/serve/github/mock.rs` — MockGhClient test double
- `crates/smelt-cli/src/serve/github/source.rs` — GithubTrackerSource impl of TrackerSource
- `crates/smelt-cli/src/serve/config.rs` — TrackerConfig.repo field + validation
- `crates/smelt-cli/src/serve/mod.rs` — pub mod github registration
