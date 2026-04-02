# S01: GitHub Forge Client

**Goal:** Implement `smelt_core::forge` — a `ForgeClient` trait and `GitHubForge` implementation backed by octocrab — behind a `forge` feature flag, with unit-tested PR creation and status polling against a mock HTTP server.
**Demo:** `cargo test -p smelt-core --features forge` proves `GitHubForge::create_pr()` and `poll_pr_status()` against mock HTTP responses; `cargo build -p smelt-core` (no forge feature) compiles with zero octocrab dependencies.

## Must-Haves

- `smelt_core::forge::ForgeClient` trait: `async fn create_pr(repo, head, base, title, body) -> Result<PrHandle>` and `async fn poll_pr_status(repo, number) -> Result<PrStatus>` — Rust 2024 RPITIT, not object-safe (D019)
- `smelt_core::forge::GitHubForge` struct with `new(token: String)` constructor, implementing `ForgeClient` via octocrab
- `smelt_core::forge::{PrHandle, PrStatus, PrState, CiStatus, ForgeConfig}` all exported from the forge module
- `smelt-core/Cargo.toml`: `octocrab = "0.49"` as an optional dep; `[features] forge = ["dep:octocrab"]`; `wiremock = "0.6"` in dev-deps (always present so tests compile without the feature)
- `forge` feature flag is additive-only: `cargo build -p smelt-core` (without `--features forge`) produces zero octocrab dependency nodes
- Unit tests using wiremock: `create_pr` happy path (201), 401 auth error, 422 validation error; `poll_pr_status` for Open/Merged/Closed states with Pending/Passing/Failing CI status
- `SmeltError::Forge { operation: String, message: String }` variant with `forge()` and `forge_with_source()` constructors mirroring the Provider pattern

## Proof Level

- This slice proves: contract verification (mock HTTP unit tests)
- Real runtime required: no (all tests use wiremock mock server)
- Human/UAT required: no

## Verification

The objective stopping condition is:

```bash
# No-forge build must compile cleanly
cargo build -p smelt-core 2>&1 | grep -v "^$" | head -5

# All forge unit tests must pass
cargo test -p smelt-core --features forge 2>&1 | tail -20

# Zero octocrab in non-forge dependency tree
cargo tree -p smelt-core | grep -c octocrab  # must print 0

# Feature-gated tree must show octocrab
cargo tree -p smelt-core --features forge | grep octocrab  # must show octocrab
```

Named test cases in `crates/smelt-core/src/forge.rs` (`#[cfg(test)]` module):
- `test_create_pr_happy_path` — mock 201 response → `PrHandle { url, number }` matches expected values
- `test_create_pr_auth_error` — mock 401 response → `Err(SmeltError::Forge { .. })`
- `test_create_pr_validation_error` — mock 422 response → `Err(SmeltError::Forge { .. })`
- `test_poll_pr_status_open_pending` — mock open PR + pending CI → `PrStatus { state: Open, ci_status: Pending, .. }`
- `test_poll_pr_status_merged_passing` — mock merged PR + success CI → `PrStatus { state: Merged, ci_status: Passing, .. }`
- `test_poll_pr_status_closed_failing` — mock closed PR + failure CI → `PrStatus { state: Closed, ci_status: Failing, .. }`

## Observability / Diagnostics

- Runtime signals: `SmeltError::Forge { operation, message }` carries the octocrab error message verbatim so HTTP status codes and GitHub's error body are always surfaced to the caller
- Inspection surfaces: `cargo test -p smelt-core --features forge -- --nocapture` shows wiremock request matching logs; octocrab's error type exposes the HTTP status and GitHub error body in `.to_string()`
- Failure visibility: each `forge()` / `forge_with_source()` constructor site tags the `operation` string (e.g. `"create_pr"`, `"poll_pr_status"`) so the error message always identifies which API call failed
- Redaction constraints: `GITHUB_TOKEN` values must never appear in error messages or logs; the token is stored only in `GitHubForge` internal state and passed to octocrab headers; never printed

## Integration Closure

- Upstream surfaces consumed: none (first slice — all new)
- New wiring introduced in this slice: `#[cfg(feature = "forge")] pub mod forge` in `lib.rs`; optional octocrab dep in `smelt-core/Cargo.toml`
- What remains before the milestone is truly usable end-to-end: S02 (manifest `[forge]` section + Phase 9 in `execute_run()`), S03 (smelt status/watch), S04 (init/state isolation), S05 (library API surface)

## Tasks

- [x] **T01: Feature flag, types, ForgeClient trait, and failing test skeleton** `est:45m`
  - Why: Establishes the `forge` feature flag, all public types, the `ForgeClient` trait, a stubbed `GitHubForge` (methods panic with `unimplemented!()`), and the complete test file with real assertions that fail because the impl isn't done. This gives T02 and T03 a clear target to make green.
  - Files: `crates/smelt-core/Cargo.toml`, `crates/smelt-core/src/forge.rs`, `crates/smelt-core/src/error.rs`, `crates/smelt-core/src/lib.rs`
  - Do:
    1. Add `octocrab = { version = "0.49", optional = true }` to `[dependencies]` and `[features] forge = ["dep:octocrab"]` in `smelt-core/Cargo.toml`; add `wiremock = "0.6"` to `[dev-dependencies]` (unconditional so tests always compile)
    2. Add `SmeltError::Forge { operation: String, message: String }` variant to `error.rs` with `#[error("forge {operation} failed: {message}")]`; add `forge()` (no source) and `forge_with_source()` constructors mirroring `provider()` / `provider_with_source()`
    3. Create `crates/smelt-core/src/forge.rs`: define `ForgeConfig` (with `#[serde(deny_unknown_fields)]`, Deserialize), `PrHandle`, `PrState`, `CiStatus`, `PrStatus`; define `ForgeClient` trait with `async fn create_pr` and `async fn poll_pr_status` (Rust 2024 RPITIT — bare `async fn` in trait is legal; add `+ Send` on the returned future via where clause if needed); define `GitHubForge { client: octocrab::Octocrab }` with `new(token: String) -> Self` calling `OctocrabBuilder::new().personal_token(token).build()?` (handle the Result); stub `create_pr` and `poll_pr_status` with `unimplemented!("T02")` / `unimplemented!("T03")`
    4. Add `pub mod forge;` (unconditional — types like `ForgeConfig` must be accessible without the feature flag, e.g. for S02 manifest parsing) and the matching re-exports to `lib.rs`: `#[cfg(feature = "forge")] pub use forge::GitHubForge;` and `pub use forge::{ForgeClient, ForgeConfig, PrHandle, PrState, CiStatus, PrStatus};` — only `GitHubForge` and the octocrab-dependent code inside `forge.rs` need the cfg gate
    5. Write `#[cfg(test)]` module in `forge.rs` with all 6 test functions: each function sets up a `MockServer`, creates an `octocrab` instance pointed at `server.uri()`, constructs `GitHubForge { client: octocrab }`, calls the method, and asserts the expected `Ok(...)` or `Err(SmeltError::Forge { .. })` — tests will fail at runtime because impls panic, which is correct for T01
  - Verify: `cargo build -p smelt-core` passes (no forge); `cargo build -p smelt-core --features forge` compiles (forge with stubs); `cargo test -p smelt-core --features forge 2>&1 | grep "panicked\|FAILED"` shows 6 test failures (not compile errors); `cargo tree -p smelt-core | grep octocrab` prints nothing (zero deps without feature)
  - Done when: both builds compile cleanly, 6 tests exist and fail at runtime (not at compile time), zero octocrab in no-forge dep tree

- [x] **T02: Implement create_pr() and make its three tests pass** `est:1h`
  - Why: Delivers the first half of the `ForgeClient` contract — PR creation. Makes `test_create_pr_*` green by replacing the `unimplemented!` stub with a real octocrab call. Tests drive discovery of required mock JSON fields (see pitfall: `PullRequest` is `#[non_exhaustive]`).
  - Files: `crates/smelt-core/src/forge.rs`
  - Do:
    1. Parse `repo` parameter (`"owner/repo"`) into `(owner, repo_name)` components by splitting on `/`; return `SmeltError::Forge { operation: "create_pr".into(), message: "invalid repo format".into() }` if no `/` present
    2. Call `self.client.pulls(owner, repo_name).create(title, head, base).body(body).send().await`; map octocrab errors to `SmeltError::forge()` (stringify via `.to_string()` since octocrab `Error` Send+Sync status is unknown at design time — check at implementation and upgrade to `forge_with_source` if it implements `Send + Sync + 'static`)
    3. Map `pr.html_url.map(|u| u.to_string()).unwrap_or_default()` and `pr.number` into `PrHandle { url, number }`
    4. For wiremock tests: mock JSON must include at minimum `"number"`, `"html_url"`, `"state"`, `"url"`, `"id"`, `"node_id"`, `"locked"`, `"maintainer_can_modify"`, `"head": { "label": "", "ref": "", "sha": "", "repo": null, "user": null }`, `"base": { "label": "", "ref": "", "sha": "", "repo": null, "user": null }` — start minimal, add fields until the test doesn't panic on deserialization
    5. For `test_create_pr_auth_error`: mount a 401 response with `{"message": "Bad credentials"}`; assert `matches!(err, SmeltError::Forge { .. })`
    6. For `test_create_pr_validation_error`: mount a 422 response with `{"message": "Validation Failed", "errors": []}`; assert `matches!(err, SmeltError::Forge { .. })`
  - Verify: `cargo test -p smelt-core --features forge test_create_pr 2>&1 | tail -10` shows `test result: ok. 3 passed`
  - Done when: all three `test_create_pr_*` tests pass; `test_poll_pr_status_*` tests still fail (expected); no regressions in `cargo test -p smelt-core` (no-forge)

- [x] **T03: Implement poll_pr_status() and make its three tests pass** `est:1h`
  - Why: Delivers the second half of the `ForgeClient` contract — PR status polling. Makes all 6 tests green and closes the slice.
  - Files: `crates/smelt-core/src/forge.rs`
  - Do:
    1. Parse `repo` into `(owner, repo_name)` (same split logic as T02 — extract a private `parse_repo()` helper if not already done)
    2. Call `self.client.pulls(owner, repo_name).get(number).await` to get the PR; map errors to `SmeltError::forge()`
    3. Derive `PrState`: if `pr.merged == Some(true)` → `Merged`; else if `pr.state == Some(IssueState::Closed)` → `Closed`; else → `Open` (import `octocrab::models::IssueState`)
    4. Get CI status via combined commit status API: try `octocrab._get(format!("/repos/{owner}/{repo_name}/commits/{sha}/status"))` where `sha = pr.head.sha.unwrap_or_default()`; deserialize into a local `struct CombinedStatus { state: String }` (not exported); if the call fails, `CiStatus::Unknown`; map `"success"` → `Passing`, `"failure"|"error"` → `Failing`, `"pending"` → `Pending`, _ → `Unknown`
    5. Set `review_count` from `pr.review_comments.unwrap_or(0) as u32`; document in a comment that this counts inline review comments (not submitted reviews/approvals) and reference that a future slice can switch to `pulls.list_reviews()` if needed
    6. For wiremock tests: mock both the GET pulls/{number} endpoint AND the GET commits/{sha}/status endpoint; use distinct sha values per test so the status mock matches correctly
  - Verify: `cargo test -p smelt-core --features forge 2>&1 | tail -10` shows `test result: ok. 6 passed; 0 failed`; `cargo test -p smelt-core 2>&1 | tail -5` still passes; `cargo tree -p smelt-core | grep -c octocrab` prints `0`
  - Done when: all 6 tests pass, no-forge build clean, octocrab properly absent from no-forge dep tree

## Files Likely Touched

- `crates/smelt-core/Cargo.toml`
- `crates/smelt-core/src/forge.rs` (new)
- `crates/smelt-core/src/error.rs`
- `crates/smelt-core/src/lib.rs`
