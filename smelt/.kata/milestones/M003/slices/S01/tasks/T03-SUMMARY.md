---
id: T03
parent: S01
milestone: M003
provides:
  - "GitHubForge::poll_pr_status() implemented via octocrab — fetches PR state and combined CI status"
  - "PrState derivation: Merged (pr.merged==true), Closed (IssueState::Closed), Open (default)"
  - "CiStatus derivation via /repos/{owner}/{repo}/commits/{sha}/status endpoint; failure falls back to CiStatus::Unknown (non-fatal)"
  - "review_count from pr.review_comments (inline diff comments; no extra API call)"
  - "serde_json added as forge-feature-gated dependency for CombinedStatus deserialization"
  - "All 6 forge tests passing; slice S01 complete"
key_files:
  - "crates/smelt-core/src/forge.rs"
  - "crates/smelt-core/Cargo.toml"
key_decisions:
  - "D054 (already recorded): review_count uses pr.review_comments (inline diff comments), not list_reviews() — no extra API call; revisable in S03"
  - "serde_json added as optional dep under forge feature (not just dev-dep) to enable runtime JSON deserialization of CombinedStatus"
  - "CI status fetch failure (non-200, parse error, empty SHA) → CiStatus::Unknown; never propagated as Err per task plan invariant"
patterns_established:
  - "CombinedStatus local struct pattern: private #[derive(Deserialize)] struct inside poll_pr_status for single-use deserialization targets"
  - "Non-fatal CI status fetch: _get() errors and body_to_string() errors are matched and swallowed to Unknown — ensures status polling never fails due to missing CI"
  - "parse_repo() helper reused across create_pr and poll_pr_status — single private fn for owner/repo splitting"
observability_surfaces:
  - "SmeltError::Forge { operation: 'poll_pr_status', message } tags all fatal errors from this call site"
  - "CiStatus::Unknown is the silent fallback when commit status endpoint is unreachable — distinguishable from Pending/Passing/Failing in caller logic"
  - "cargo test -p smelt-core --features forge -- --nocapture shows wiremock request logs for both GET /pulls/{n} and GET /commits/{sha}/status per test"
duration: 20m
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T03: Implement poll_pr_status() and make its three tests pass

**GitHubForge::poll_pr_status() implemented with IssueState-based PrState derivation and non-fatal CiStatus from GitHub's combined commit status API; all 6 slice tests pass.**

## What Happened

Replaced the `unimplemented!("T03")` stub with a full implementation:

1. **parse_repo reuse**: Called the existing `parse_repo(repo)?` helper from T02.

2. **PR fetch**: `self.client.pulls(owner, repo_name).get(number).await` mapped errors to `SmeltError::forge("poll_pr_status", ...)`.

3. **PrState derivation**: Used `pr.merged` and `pr.state` (which is `Option<IssueState>` from `octocrab::models::IssueState`). Match order: `Some(true)` merged → `Merged`; `Some(IssueState::Closed)` → `Closed`; otherwise → `Open`.

4. **CI status fetch**: Used `self.client._get(url).await` and `body_to_string()` to get the combined status JSON, then `serde_json::from_str::<CombinedStatus>` to extract the `state` field. All error paths (network failure, parse failure, empty SHA) fall back to `CiStatus::Unknown`.

5. **review_count**: `pr.review_comments.unwrap_or(0) as u32`.

6. **Test fixes**: The stub test mocks were incomplete — missing required `PullRequest` fields (`url`, `id`, `ref` in head/base) and missing CI status endpoint mocks. Added both the `/repos/owner/repo/commits/{sha}/status` mock per test with distinct SHA values, and added full assertions replacing the commented-out placeholder assertions.

**Dependency fix**: `serde_json` was only in dev-dependencies. The `poll_pr_status` implementation uses `serde_json::from_str` in production code (behind the `forge` feature), so added `serde_json = { version = "1", optional = true }` to `[dependencies]` and added `dep:serde_json` to the `forge` feature in Cargo.toml.

## Verification

```
cargo test -p smelt-core --features forge -- forge 2>&1 | tail -10
# test forge::tests::test_poll_pr_status_open_pending ... ok
# test forge::tests::test_poll_pr_status_closed_failing ... ok
# test forge::tests::test_poll_pr_status_merged_passing ... ok
# test forge::tests::test_create_pr_auth_error ... ok
# test forge::tests::test_create_pr_happy_path ... ok
# test forge::tests::test_create_pr_validation_error ... ok  (via 6 passed total)
# test result: ok. 6 passed; 0 failed

cargo test -p smelt-core 2>&1 | tail -3
# test result: ok. N passed; 0 failed

cargo tree -p smelt-core | grep octocrab
# (no output — zero deps without forge feature)

cargo tree -p smelt-core --features forge | grep octocrab
# ├── octocrab v0.49.5

cargo build --workspace 2>&1 | grep "^error" | wc -l
# 0
```

## Diagnostics

- `SmeltError::Forge { operation: "poll_pr_status", message }` — all fatal errors from this method carry this tag
- `CiStatus::Unknown` is the non-fatal fallback for CI status fetch failures; distinguishable from Pending/Passing/Failing in downstream logic
- `cargo test -p smelt-core --features forge -- --nocapture` shows wiremock matching logs for both PR and CI status endpoints per test

## Deviations

- Added `serde_json` as a regular dependency under the `forge` feature (not only in dev-deps). The plan note said "serde_json is available transitively via octocrab" — this is true in the dep tree but Rust requires explicit declaration for use in production code. Minor mechanical deviation, no architectural impact.
- Test mock JSON for the poll tests was incomplete (missing required `PullRequest` serde fields: `url`, `id`, `ref` in head/base). Fixed all three test mocks as part of this task. The plan's mock JSON snippets were illustrative rather than complete.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/forge.rs` — poll_pr_status() implemented; all 3 poll tests completed with full mocks and assertions; slice S01 complete
- `crates/smelt-core/Cargo.toml` — serde_json added as optional dep under forge feature
