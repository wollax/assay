---
id: M011
provides:
  - assay-backends crate (crates/assay-backends/) with linear, github, ssh feature flags and backend_from_config() factory fn
  - StateBackendConfig::Linear { team_id, project_id }, GitHub { repo, label }, Ssh { host, remote_assay_dir, user, port } named variants
  - Schema snapshots updated for state-backend-config-schema and run-manifest-orchestrate-schema
  - LinearBackend implementing all 7 StateBackend methods via reqwest::blocking GraphQL client; .linear-issue-id file lifecycle
  - GitHubBackend implementing all 7 StateBackend methods via gh CLI subprocess; .github-issue-number file lifecycle
  - SshSyncBackend implementing all 7 StateBackend methods via scp/ssh Command::arg() chaining; CapabilitySet::all()
  - backend_from_config() fully resolves all 4 StateBackendConfig variants to real backends
  - All 6 CLI/MCP OrchestratorConfig construction sites use backend_from_config(); zero hardcoded LocalFsBackend::new() at manifest-dispatch sites
key_decisions:
  - D160 — assay-backends as new leaf crate (depends on assay-core + assay-types, not vice versa)
  - D163 — SshSyncBackend uses Command::arg() chaining for scp (no shell string interpolation)
  - D164 — LinearBackend capabilities: messaging=false, gossip_manifest=false, annotations=true, checkpoints=false
  - D165 — backend_from_config factory fn in assay_backends::factory
  - D168 — D161 superseded: LinearBackend uses reqwest::blocking, not scoped async runtime
  - D169 — backend_from_config graceful fallback to NoopBackend when LINEAR_API_KEY missing
  - D170 — GitHubBackend capabilities all-false (CapabilitySet::none())
  - D171 — GitHubBackend uses --body-file - with stdin pipe for all body content
  - D172 — GitHubBackend factory dispatch has no env-var gate (unlike LinearBackend)
  - D173 — SshSyncBackend uses ssh_run() with shell_quote() for remote commands; scp paths use Command::arg()
  - D174 — SshSyncBackend read_run_state returns Ok(None) on scp pull failure (first-access semantics)
patterns_established:
  - backend_from_config() dispatches config enum to Arc<dyn StateBackend>; graceful fallback to NoopBackend when credentials absent
  - .linear-issue-id / .github-issue-number file lifecycle: create on first push_session_event, read on subsequent
  - Dual-arm cfg(feature) / cfg(not(feature)) pattern in factory.rs for each backend feature flag
  - Contract test pattern: mock transport (mockito / mock binary / mock scp script) → call backend method → assert mock called + side effects
  - write_mock_gh / write_mock_scp / write_mock_ssh pattern: multi-subcommand dispatcher via positional arg inspection in single shell script
  - with_mock_path / with_mock_gh_path closures with #[serial] for PATH-override test isolation
observability_surfaces:
  - tracing::info! on issue creation (Linear: issue_id, GitHub: issue_number + repo) and comment creation
  - tracing::warn! on GraphQL errors, LINEAR_API_KEY absent in factory, gh non-zero exit, ssh feature disabled at build time
  - tracing::debug! at start of scp_push, scp_pull, ssh_run with operation name
  - .linear-issue-id file in run_dir maps each run to a Linear issue
  - .github-issue-number file in run_dir maps each run to a GitHub issue
  - AssayError::io() with operation labels on all failure modes; secret values never logged
  - Factory tracing::warn! when backend credentials absent (LINEAR_API_KEY) or feature disabled (ssh)
requirement_outcomes:
  - id: R076
    from_status: active
    to_status: validated
    proof: "S02 — LinearBackend implements all 7 StateBackend methods; push_session_event creates issue on first call and comments on subsequent; read_run_state deserializes latest comment; annotate_run posts [assay:manifest] tagged comment; capabilities()=D164 flags; 8 mockito contract tests pass (linear feature); backend_from_config dispatches Linear→LinearBackend; just ready green with 1501 tests"
  - id: R077
    from_status: active
    to_status: validated
    proof: "S03 — GitHubBackend implements all 7 StateBackend methods; push_session_event creates issue on first call and comments on subsequent via --body-file - stdin pipe; read_run_state deserializes latest comment (falls back to issue body if no comments); capabilities() returns CapabilitySet::none(); 8 mock-subprocess contract tests pass (github feature); backend_from_config dispatches GitHub→GitHubBackend; just ready green with 1501 tests"
  - id: R078
    from_status: active
    to_status: validated
    proof: "S04 — SshSyncBackend implements all 7 StateBackend methods via Command::arg() chaining (D163); CapabilitySet::all() returned; 9 contract tests with mock scp/ssh binaries (PATH override + #[serial]) cover all methods, injection safety (path with spaces), and first-access Ok(None) semantics; backend_from_config dispatches Ssh→SshSyncBackend behind cfg(feature = 'ssh'); just ready green with 1499 tests"
  - id: R079
    from_status: active
    to_status: validated
    proof: "S01 — assay-backends crate compiles; StateBackendConfig has Linear, GitHub, Ssh variants with schema snapshots committed; backend_from_config() dispatches all 5 variants (LocalFs→LocalFsBackend, others→NoopBackend stubs); serde round-trip tests pass for all variants; just ready green with 1497 tests. S04 — CLI/MCP construction sites use backend_from_config(); grep -r 'LocalFsBackend::new' crates/assay-cli crates/assay-mcp returns no matches"
duration: ~75min total (S01: ~10m, S02: ~22m, S03: ~16m, S04: ~27m)
verification_result: passed
completed_at: 2026-03-27
---

# M011: Concrete Remote Backends

**Four-slice milestone delivering `assay-backends` crate with `LinearBackend`, `GitHubBackend`, and `SshSyncBackend` — all 7 `StateBackend` methods implemented and contract-tested — plus `backend_from_config()` wired into all 6 CLI/MCP construction sites; `just ready` green with 1526 tests (all features) and zero regression.**

## What Happened

M011 built on M010's `StateBackend` trait abstraction by delivering three production backends and the factory plumbing to connect them to the CLI/MCP dispatch path.

**S01** established the foundation: the `assay-backends` leaf crate with `linear`/`github`/`ssh` feature flags, `StateBackendConfig::Linear`, `GitHub`, and `Ssh` named variants in `assay-types`, regenerated JSON Schema snapshots (both orchestrate and non-orchestrate), and a `backend_from_config()` factory fn that dispatched `LocalFs` to `LocalFsBackend` and stubbed the others with `NoopBackend`. Serde round-trip tests locked all five variant shapes. 1497 tests passing.

**S02** delivered `LinearBackend` in test-first order: 8 mockito contract tests were written against the expected interface (red state), then the `LinearClient` GraphQL wrapper (using `reqwest::blocking` — D168 superseded D161's scoped async runtime approach) and `LinearBackend` implementation were written to make them green. Key design: `push_session_event` writes `.linear-issue-id` on first call (creates issue) and reads it on subsequent calls (creates comment); `read_run_state` fetches the latest comment and deserializes it. Capabilities follow D164 (annotations=true, others false). Factory updated to dispatch `Linear` to `LinearBackend` with graceful `NoopBackend` fallback when `LINEAR_API_KEY` absent (D169). 1501 tests.

**S03** delivered `GitHubBackend` using the same test-first pattern: 8 contract tests with a mock `gh` binary (PATH override + `#[serial]` isolation) before the `GhRunner` + `GitHubBackend` implementation. Body text is piped via `--body-file -` with `Stdio::piped()` (D171) to avoid ARG_MAX limits. Issue number stored in `.github-issue-number` mirroring the Linear pattern. Capabilities are all-false (`CapabilitySet::none()`, D170). `read_run_state` falls back to issue body when no comments exist (defensive). 1501 tests.

**S04** completed the milestone in three tasks: T01 wrote 9 `SshSyncBackend` contract tests with mock `scp`/`ssh` binaries; T02 implemented `ScpRunner` + `SshSyncBackend` (~200 lines) using `Command::arg()` chaining throughout (D163) and `shell_quote()` for remote shell commands (D173); T03 wired `backend_from_config()` into all 6 CLI/MCP construction sites in `run.rs` and `server.rs`, removing all hardcoded `LocalFsBackend::new(...)` at manifest-dispatch sites. 1499 tests.

The four slices connected cleanly: S01 produced the factory stub and config types; S02–S04 each replaced one `NoopBackend` arm with a real implementation; S04 closed the loop by wiring the factory into the callers that needed it.

## Cross-Slice Verification

**Success criterion: `just ready` green with all 1488+ tests passing after every slice**
- S01: 1497 tests ✅
- S02: 1501 tests ✅
- S03: 1501 tests ✅
- S04: 1499 tests ✅
- Final with all features: `cargo test --workspace --features "assay-backends/linear,assay-backends/github,assay-backends/ssh"` → 1526 tests, 0 failures ✅

**Success criterion: `StateBackendConfig` has `Linear`, `GitHub`, and `Ssh` named variants; schema snapshots updated and committed**
- `crates/assay-types/src/state_backend.rs` — `Linear { team_id, project_id }`, `GitHub { repo, label }`, `Ssh { host, remote_assay_dir, user, port }` variants present with correct serde attributes ✅
- Both schema snapshot files regenerated via `cargo insta accept` and committed in S01 ✅
- `#[serde(rename = "github")]` on GitHub variant overrides `rename_all = "snake_case"` default ✅

**Success criterion: `assay-backends` crate exists with `linear`, `github`, `ssh` feature flags; each backend compiles and passes contract tests**
- `crates/assay-backends/Cargo.toml` with `linear`, `github`, `ssh` feature flags ✅
- `cargo test -p assay-backends --features linear` — 8 LinearBackend contract tests + 5 factory tests pass ✅
- `cargo test -p assay-backends --features github` — 8 GitHubBackend contract tests + 5 factory tests pass ✅
- `cargo test -p assay-backends --features ssh` — 9 SshSyncBackend contract tests + 5 factory tests pass ✅

**Success criterion: `LinearBackend::push_session_event` creates a Linear issue (first call) or appends a comment (subsequent); `read_run_state` reads the latest comment back**
- `test_push_first_event_creates_issue` — mockito proves issue creation on first call ✅
- `test_push_subsequent_event_creates_comment` — mockito proves comment on subsequent calls ✅
- `test_read_run_state_deserializes_latest_comment` — mockito proves deserialization ✅

**Success criterion: `GitHubBackend::push_session_event` creates a GitHub issue (first call) or appends a comment (subsequent calls) via `gh` CLI; `read_run_state` reads back via `gh issue view`**
- `test_push_creates_issue_on_first_call` — mock gh binary proves issue creation ✅
- `test_push_creates_comment_on_subsequent_calls` — mock gh binary proves comment ✅
- `test_read_run_state` — mock gh binary proves deserialization from latest comment ✅

**Success criterion: `SshSyncBackend` implements all 7 trait methods by shelling out to `scp`; `CapabilitySet::all()` returned**
- `test_capabilities_returns_all` — `CapabilitySet::all()` confirmed ✅
- 9 contract tests cover all 7 methods including injection safety ✅
- `test_scp_arg_construction_with_spaces` — path `/remote/assay dir with spaces` passed as single unbroken arg token ✅

**Success criterion: `backend_from_config()` factory fn resolves any `StateBackendConfig` variant to an `Arc<dyn StateBackend>`; CLI/MCP construction sites use it**
- `crates/assay-backends/src/factory.rs` — all 4 variants dispatched to real backends ✅
- `grep -r "LocalFsBackend::new" crates/assay-cli crates/assay-mcp` — no matches ✅
- 6 construction sites in `run.rs` (3) and `server.rs` (3) use `backend_from_config()` ✅

**Success criterion: No existing behavior changes for `local_fs` users — `LocalFsBackend` remains default**
- `manifest.state_backend.as_ref().unwrap_or(&StateBackendConfig::LocalFs)` at all 6 callsites ✅
- All existing integration tests pass unchanged ✅

## Requirement Changes

- R076: active → validated — S02: 8 mockito contract tests prove LinearBackend's full interface; `just ready` green with 1501 tests
- R077: active → validated — S03: 8 mock-subprocess contract tests prove GitHubBackend's full interface; `just ready` green with 1501 tests
- R078: active → validated — S04: 9 mock scp/ssh contract tests prove SshSyncBackend's full interface + injection safety; `just ready` green with 1499 tests
- R079: active → validated — S01+S04: crate scaffold + schema snapshots (S01); CLI/MCP wiring completes the requirement (S04); `grep -r 'LocalFsBackend::new' crates/assay-cli crates/assay-mcp` returns no matches

## Forward Intelligence

### What the next milestone should know
- `backend_from_config()` is live at all 6 CLI/MCP callsites — manifests with any `state_backend` variant route to the correct backend at runtime
- `reqwest` (blocking, with json+blocking features) is in `assay-backends` behind the `linear` feature — if a future backend also needs HTTP, add it to the same dep rather than duplicating
- The `.linear-issue-id` / `.github-issue-number` file lifecycle creates a per-run mapping to an external issue — if the run_dir is cleaned between tasks, the issue link is lost and the next push creates a new issue (orphaning the old one)
- `read_run_state` in LinearBackend picks up the latest comment regardless of type — `annotate_run` after `push_session_event` will make the next `read_run_state` fail to deserialize the annotation as `OrchestratorStatus`. Ordering discipline is currently caller responsibility.
- `poll_inbox` in SshSyncBackend emits non-fatal `tracing::warn!` on ssh rm failure — inbox messages could be delivered twice on retry
- The factory dispatch tests in `factory.rs` inline `#[cfg(test)]` module are the fastest signal that `backend_from_config()` dispatches correctly — run `cargo test -p assay-backends` for a quick sanity check

### What's fragile
- `read_run_state` (LinearBackend) picks up latest comment regardless of type — annotation comments placed after event comments cause deserialization failure. No ordering guarantee.
- Mock gh binary tests use `#[serial]` + PATH mutation — parallel test execution without the serial attribute will corrupt the PATH env and cause flaky failures.
- `poll_inbox` ssh ls parsing splits on newlines — any filename with embedded newlines would corrupt the parse. No guard exists; not expected in practice.
- `shell_quote()` handles spaces and single-quote escaping but test coverage only exercises spaces. Paths with single-quote characters in remote_assay_dir are not tested.

### Authoritative diagnostics
- `cargo test -p assay-backends --features linear -- --nocapture` — shows mockito request/response detail for LinearBackend debugging
- `.linear-issue-id` / `.github-issue-number` in run_dir — ground truth for which external issue a run maps to
- `tracing::warn!` in factory.rs when LINEAR_API_KEY absent — signals graceful degradation to NoopBackend
- `RUST_LOG=warn cargo run -- ...` — factory degradation warnings appear at this log level

### What assumptions changed
- D161 (scoped async runtime per method for LinearBackend) was superseded by D168 (reqwest::blocking) — the async-in-sync risk from M011-ROADMAP is fully retired; `reqwest::blocking` internalizes its own runtime without nested-runtime panic risk
- Original mock scp direction detection used unquoted `$ARGS` iteration — actual implementation required `"$@"` to preserve argument boundaries for paths with spaces (T01 mock script bug caught in T02)

## Files Created/Modified

- `crates/assay-types/src/state_backend.rs` — Added Linear, GitHub, Ssh variants to StateBackendConfig
- `crates/assay-backends/Cargo.toml` — New crate manifest with linear/github/ssh feature flags + reqwest, mockito, serial_test, tempfile deps
- `crates/assay-backends/src/lib.rs` — Module root exposing factory, linear, github, ssh behind feature gates
- `crates/assay-backends/src/factory.rs` — backend_from_config() factory fn + 5 dispatch tests
- `crates/assay-backends/src/linear.rs` — LinearClient + LinearBackend (~250 lines)
- `crates/assay-backends/src/github.rs` — GhRunner + GitHubBackend (~320 lines)
- `crates/assay-backends/src/ssh.rs` — ScpRunner + SshSyncBackend (~200 lines)
- `crates/assay-backends/tests/linear_backend.rs` — 8 mockito contract tests for LinearBackend
- `crates/assay-backends/tests/github_backend.rs` — 8 contract tests with mock gh binary
- `crates/assay-backends/tests/ssh_backend.rs` — 9 contract tests with mock scp/ssh binaries
- `crates/assay-core/tests/state_backend.rs` — 7 serde round-trip tests for new StateBackendConfig variants
- `crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap` — Regenerated with new variants
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-orchestrate-schema.snap` — Regenerated with new variants
- `crates/assay-cli/Cargo.toml` — Added assay-backends workspace dep
- `crates/assay-mcp/Cargo.toml` — Added assay-backends workspace dep
- `crates/assay-cli/src/commands/run.rs` — Replaced 3 LocalFsBackend::new() callsites with backend_from_config()
- `crates/assay-mcp/src/server.rs` — Replaced 3 LocalFsBackend::new() callsites with backend_from_config()
- `Cargo.toml` — Added assay-backends to workspace members and dependencies
