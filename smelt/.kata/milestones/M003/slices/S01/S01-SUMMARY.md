---
id: S01
parent: M003
milestone: M003
provides:
  - "`smelt_core::forge` module — `ForgeClient` trait, `GitHubForge` impl, `PrHandle`, `PrStatus`, `PrState`, `CiStatus`, `ForgeConfig` types"
  - "`forge` feature flag in `smelt-core/Cargo.toml` gating `octocrab` and `serde_json` as optional deps; zero octocrab in no-feature dep tree"
  - "`GitHubForge::create_pr()` via octocrab pulls().create().body().send(); maps errors to `SmeltError::Forge`"
  - "`GitHubForge::poll_pr_status()` — `PrState` from `pr.merged`/`pr.state`; `CiStatus` from `/commits/{sha}/status`; `review_count` from `pr.review_comments`"
  - "`SmeltError::Forge { operation, message }` variant with `forge()` and `forge_with_source()` constructors"
  - "6 wiremock unit tests passing: create_pr happy/401/422; poll_pr_status open-pending/merged-passing/closed-failing"
  - "`parse_repo()` private helper for owner/repo splitting — reusable across create_pr and poll_pr_status"
requires: []
affects:
  - S02
  - S03
  - S05
key_files:
  - crates/smelt-core/Cargo.toml
  - crates/smelt-core/src/forge.rs
  - crates/smelt-core/src/error.rs
  - crates/smelt-core/src/lib.rs
  - .kata/DECISIONS.md
key_decisions:
  - "D052: octocrab (not gh CLI) for GitHub API — required for R005 library embedding"
  - "D053: octocrab::Error wrapping as stringify initially; D056 confirmed Send+Sync+'static but kept as stringify for S01 MVP"
  - "D054: review_count from pr.review_comments (inline diff comments); revisable in S03 if approval count needed"
  - "D055: ForgeConfig and trait types unconditional; only GitHubForge gated behind forge feature — allows S02 to parse ForgeConfig without pulling in octocrab"
  - "D056: octocrab::Error IS Send+Sync+'static — upgrade to source field is a clear path when needed"
patterns_established:
  - "`forge_for_server()` test helper: constructs `GitHubForge { client }` via direct field access to redirect octocrab at a MockServer URI"
  - "wiremock + octocrab test pattern: MockServer::start() → OctocrabBuilder::base_uri(server.uri()) → GitHubForge { client } → mount mocks → call method"
  - "`CombinedStatus` local struct pattern: private `#[derive(Deserialize)]` inside poll_pr_status for single-use deserialization"
  - "Non-fatal CI status fetch: _get() errors and parse errors fall back to CiStatus::Unknown — ensures status polling never fails due to missing CI"
  - "`parse_repo()` private helper for owner/repo splitting — called from both create_pr and poll_pr_status"
observability_surfaces:
  - "`SmeltError::Forge { operation, message }` — operation field ('create_pr', 'poll_pr_status') tags the failing API call; message carries octocrab error verbatim"
  - "`CiStatus::Unknown` — silent fallback for unreachable CI status endpoint; distinguishable from Pending/Passing/Failing in caller logic"
  - "`cargo test -p smelt-core --features forge -- --nocapture` shows wiremock request matching logs for all forge endpoints"
drill_down_paths:
  - .kata/milestones/M003/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M003/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M003/slices/S01/tasks/T03-SUMMARY.md
duration: 70min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
---

# S01: GitHub Forge Client

**`ForgeClient` trait + `GitHubForge` impl behind `forge` feature flag; 6 wiremock tests prove PR creation and status polling against mock HTTP responses; zero octocrab in no-feature builds.**

## What Happened

Three tasks built the forge module from scaffold to full implementation:

**T01** established the foundation: `forge` feature flag in `smelt-core/Cargo.toml` with `octocrab` as optional dep and `wiremock`/`serde_json` as unconditional dev-deps. Created `forge.rs` with all five public types (`ForgeConfig`, `PrHandle`, `PrState`, `CiStatus`, `PrStatus`) and the `ForgeClient` trait defined unconditionally — keeping S02's manifest parser independent of the forge feature. `GitHubForge` and its impl are under `#[cfg(feature = "forge")]`. Added `SmeltError::Forge { operation, message }` with `forge()` and `forge_with_source()` constructors mirroring the Provider pattern. Wired up 6 test skeletons that compiled and panicked at `unimplemented!("T02")` / `unimplemented!("T03")`.

**T02** implemented `create_pr()`: added `parse_repo()` private helper for owner/repo splitting, called `self.client.pulls(owner, repo_name).create(title, head, base).body(body).send().await`, and mapped octocrab errors to `SmeltError::Forge`. Required empirically discovering the minimal set of non-optional `PullRequest` serde fields: `url` (String), `id` (PullRequestId), `number`, and `head`/`base` with `ref`+`sha`. Confirmed `octocrab::Error: Send + Sync + 'static` with a compile-time assertion (D056), recorded in DECISIONS.md. All 3 create_pr tests turned green.

**T03** implemented `poll_pr_status()`: reused `parse_repo()`, fetched the PR via `self.client.pulls(owner, repo_name).get(number).await`, derived `PrState` from `pr.merged` and `pr.state` (importing `octocrab::models::IssueState`), fetched CI status via `self.client._get(commits/{sha}/status)` with a private `CombinedStatus` local struct, and computed `review_count` from `pr.review_comments`. All error paths in the CI fetch fall back to `CiStatus::Unknown` — non-fatal by design. Added `serde_json` as an optional dep under the forge feature (not only dev-dep) since production code uses `serde_json::from_str`. All 6 tests green; 118 total smelt-core tests passing; zero octocrab in no-feature dep tree confirmed.

## Verification

```bash
# No-forge build compiles cleanly
cargo build -p smelt-core 2>&1 | grep -v "^$" | head -5
→ Finished `dev` profile

# All 6 forge unit tests pass (plus 112 pre-existing)
cargo test -p smelt-core --features forge 2>&1 | tail -5
→ test result: ok. 118 passed; 0 failed; 0 ignored

# Zero octocrab in no-forge dep tree
cargo tree -p smelt-core | grep -c octocrab
→ 0

# octocrab present with forge feature
cargo tree -p smelt-core --features forge | grep octocrab
→ ├── octocrab v0.49.5
```

Named test cases all passing:
- `test_create_pr_happy_path` ✓
- `test_create_pr_auth_error` ✓
- `test_create_pr_validation_error` ✓
- `test_poll_pr_status_open_pending` ✓
- `test_poll_pr_status_merged_passing` ✓
- `test_poll_pr_status_closed_failing` ✓

## Requirements Advanced

- R001 — `GitHubForge::create_pr()` now exists and is unit-tested; S02 can wire it into `execute_run()` Phase 9
- R005 — `ForgeClient` trait and `GitHubForge` are the first elements of the stable library API surface; feature-flag isolation proves the embedding story is viable

## Requirements Validated

- none — S01 is a contract-verification slice (mock HTTP only); R001 and R005 require real runtime integration in S02 and S05 respectively

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- `serde_json` added as both dev-dep (T01) and optional production dep under `forge` feature (T03) — the task plan said it was available transitively via octocrab, but Rust requires explicit declaration for `serde_json::from_str` in production code. No architectural impact.
- Mock JSON for test scaffolds (T01) was incomplete — empirically discovered in T02/T03 that `url` (String), `id` (PullRequestId), and `head`/`base` with `ref+sha` are required non-Option fields. Plan's mock snippets were illustrative.
- `unimplemented!("T02")` in Rust 1.93.1 produces `"not implemented: T02"` not `"not yet implemented: T02"` as the plan's verification grep expected. Tests behave correctly; grep string in plan was wrong.

## Known Limitations

- `review_count` uses `pr.review_comments` (inline diff comment count), not `list_reviews()` (approved reviews/requested changes). Sufficient for display; S03 should evaluate switching to `pulls.list_reviews()` if approval count is needed.
- `forge_with_source()` currently stringifies `octocrab::Error` instead of carrying a `#[source]` field despite D056 confirming the bound holds. Upgrade path is clear when error chain traversal is needed.
- No ETag / conditional-request support in `poll_pr_status()` — GitHub rate limit optimization is deferred to S03.

## Follow-ups

- S02: wire `GitHubForge::create_pr()` into `execute_run()` Phase 9; add `[forge]` section to `JobManifest`
- S03: evaluate switching `review_count` to `list_reviews()` for approval count; add ETag conditional requests to stay within GitHub rate limits
- S03: `smelt watch` polling loop should use `poll_pr_status()` directly; consider `forge_with_source()` upgrade for better error chain visibility

## Files Created/Modified

- `crates/smelt-core/Cargo.toml` — forge feature flag; octocrab+serde_json optional deps; wiremock+serde_json dev-deps
- `crates/smelt-core/src/forge.rs` — new: all forge types, ForgeClient trait, GitHubForge impl, 6 passing wiremock unit tests
- `crates/smelt-core/src/error.rs` — SmeltError::Forge variant + forge() and forge_with_source() constructors
- `crates/smelt-core/src/lib.rs` — forge module declaration + pub use re-exports (unconditional for types/trait, feature-gated for GitHubForge)
- `.kata/DECISIONS.md` — D056 appended (octocrab::Error Send+Sync+'static confirmed)

## Forward Intelligence

### What the next slice should know
- `ForgeConfig` is already exported unconditionally — S02 can import it in `manifest.rs` via `use smelt_core::forge::ForgeConfig` without enabling the `forge` feature in smelt-cli's Cargo.toml. The feature only gates `GitHubForge` and the octocrab/serde_json deps.
- The minimal `PullRequest` mock JSON for octocrab tests requires: `url` (String, not Option), `id` (PullRequestId — a u64 newtype), `number`, `head.ref`, `head.sha`, `base.ref`, `base.sha`. All other fields are `Option` or `#[serde(default)]`. Use T02's mock as the canonical template.
- `parse_repo()` is a private helper in `forge.rs` — if S02 needs repo validation at manifest parse time, it will need its own implementation or the helper needs to be made pub(crate).
- `octocrab::models::IssueState` enum has `Open` and `Closed` variants; `Merged` is not a state but a boolean field `pr.merged`. The derivation order matters: check `merged` first, then `state`.

### What's fragile
- CI status fetch uses `self.client._get(url).await` (the internal `_get` method) — this is a semi-private octocrab API that doesn't have the same stability guarantees as named client methods. If octocrab updates remove `_get`, S03 may need to find an alternative approach (e.g. `reqwest` directly or a different octocrab API).
- `review_count` from `pr.review_comments` counts inline diff comments, not formal reviews/approvals. This is a known approximation (D054) that will likely need correction in S03 for `smelt status` to show meaningful review information.

### Authoritative diagnostics
- `cargo test -p smelt-core --features forge -- forge --nocapture` — shows wiremock request matching logs + exact error strings for all 6 forge tests; first place to look for forge test failures
- `SmeltError::Forge { operation, message }` in runtime errors — operation field ('create_pr', 'poll_pr_status') pinpoints the failing API call; message carries the full octocrab error string including HTTP status and GitHub error body

### What assumptions changed
- Plan assumed wiremock mock JSON scaffolds in the task plan were complete — they were illustrative. Both create_pr (T02) and poll_pr_status (T03) required empirical discovery of required serde fields by running tests and reading deserialization panics.
- Plan assumed `serde_json` was available at runtime via transitive octocrab dep — technically true in dep tree but Rust requires explicit declaration for use in production code. Added as optional dep under forge feature.
