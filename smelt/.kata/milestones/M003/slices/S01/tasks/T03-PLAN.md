---
estimated_steps: 6
estimated_files: 1
---

# T03: Implement poll_pr_status() and make its three tests pass

**Slice:** S01 — GitHub Forge Client
**Milestone:** M003

## Description

Replace the `unimplemented!("T03")` stub in `GitHubForge::poll_pr_status()` with a real octocrab implementation that derives `PrState` from the PR model and `CiStatus` from the combined commit status API. All 6 tests must pass at the end of this task, completing the slice.

## Steps

1. **Reuse parse_repo helper**: `poll_pr_status` starts with `let (owner, repo_name) = parse_repo(repo)?;` — the helper already exists from T02.

2. **Fetch PR via octocrab**: `let pr = self.client.pulls(owner, repo_name).get(number).await.map_err(|e| SmeltError::forge("poll_pr_status", e.to_string()))?;`

3. **Derive PrState**: 
   ```rust
   use octocrab::models::IssueState;
   let state = match (pr.merged, pr.state.as_ref()) {
       (Some(true), _) => PrState::Merged,
       (_, Some(IssueState::Closed)) => PrState::Closed,
       _ => PrState::Open,
   };
   ```
   Note: `pr.state` is `Option<IssueState>` in octocrab; import `octocrab::models::IssueState`. If `IssueState` is not in `octocrab::models` directly, check `octocrab::models::issues::IssueState` — verify import path at compile time.

4. **Fetch CiStatus via raw HTTP**: Get the HEAD SHA from `pr.head.sha.as_deref().unwrap_or("")`. If the SHA is empty, return `CiStatus::Unknown`. Otherwise call the combined status endpoint:
   ```rust
   #[derive(serde::Deserialize)]
   struct CombinedStatus { state: String }
   
   let url = format!("/repos/{owner}/{repo_name}/commits/{sha}/status");
   let ci_status = match self.client._get(url).await {
       Ok(resp) => {
           match self.client.body_to_string(resp).await {  // or use serde deserialization from response
               Ok(body) => {
                   // deserialize body as CombinedStatus
                   match serde_json::from_str::<CombinedStatus>(&body) {
                       Ok(cs) => match cs.state.as_str() {
                           "success" => CiStatus::Passing,
                           "failure" | "error" => CiStatus::Failing,
                           "pending" => CiStatus::Pending,
                           _ => CiStatus::Unknown,
                       },
                       Err(_) => CiStatus::Unknown,
                   }
               }
               Err(_) => CiStatus::Unknown,
           }
       }
       Err(_) => CiStatus::Unknown,
   };
   ```
   **Implementation note**: `octocrab._get()` returns a `Result<reqwest::Response>`. Use `response.text().await` or `octocrab::serde::from_response()`. Check octocrab 0.49 API for the exact response handling. The `serde_json` crate is available transitively via octocrab when the forge feature is enabled. If `_get` API differs from the above, adapt accordingly — the key invariant is: failure to fetch CI status must fall back to `CiStatus::Unknown` (non-fatal), never propagate as an error.
   
   Also note: if `pr.head` field access is `pr.head.sha` (requiring `#[cfg(feature = "forge")]`-gated octocrab PR type), just use `pr.head.sha.clone().unwrap_or_default()`.

5. **Derive review_count**:
   ```rust
   let review_count = pr.review_comments.unwrap_or(0) as u32;
   // NOTE: review_comments counts inline diff comments, not submitted reviews (approvals/changes).
   // If approval count is needed later, use pulls.list_reviews(number) in a future slice.
   ```

6. **Fix poll_pr_status test mocks**: Each test must mock two endpoints: `GET /repos/owner/repo/pulls/{number}` and `GET /repos/owner/repo/commits/{sha}/status`. Use a distinct SHA per test to avoid mock collisions:
   
   `test_poll_pr_status_open_pending`:
   - PR response: `state: "open"`, `merged: false`, `head.sha: "sha-open"`
   - Status response: `{"state": "pending", "statuses": []}`
   - Assert: `PrStatus { state: PrState::Open, ci_status: CiStatus::Pending, review_count: 0 }`
   
   `test_poll_pr_status_merged_passing`:
   - PR response: `state: "closed"`, `merged: true`, `head.sha: "sha-merged"`
   - Status response: `{"state": "success", "statuses": []}`
   - Assert: `PrStatus { state: PrState::Merged, ci_status: CiStatus::Passing, .. }`
   
   `test_poll_pr_status_closed_failing`:
   - PR response: `state: "closed"`, `merged: false`, `head.sha: "sha-closed"`
   - Status response: `{"state": "failure", "statuses": []}`
   - Assert: `PrStatus { state: PrState::Closed, ci_status: CiStatus::Failing, .. }`

## Must-Haves

- [ ] `test_poll_pr_status_open_pending` passes
- [ ] `test_poll_pr_status_merged_passing` passes
- [ ] `test_poll_pr_status_closed_failing` passes
- [ ] `cargo test -p smelt-core --features forge` shows `test result: ok. 6 passed; 0 failed`
- [ ] `cargo test -p smelt-core` (no forge feature) still passes
- [ ] `cargo tree -p smelt-core | grep octocrab` prints nothing (zero deps without feature flag)
- [ ] CI status fetch failure (e.g. 404 on status endpoint) falls back to `CiStatus::Unknown`, never propagates as `Err`
- [ ] Decision D054 appended: `review_comments` vs `list_reviews()` choice documented

## Verification

```bash
# All 6 forge tests pass
cargo test -p smelt-core --features forge 2>&1 | tail -5
# Expected: test result: ok. 6 passed; 0 failed

# No-forge tests still pass
cargo test -p smelt-core 2>&1 | tail -3
# Expected: test result: ok. N passed; 0 failed

# Feature isolation confirmed
cargo tree -p smelt-core | grep octocrab
# Expected: (no output)

cargo tree -p smelt-core --features forge | grep octocrab
# Expected: octocrab v0.49.x (line present)

# Full workspace still builds
cargo build --workspace 2>&1 | grep "^error" | wc -l
# Expected: 0
```

## Observability Impact

- Signals added/changed: `poll_pr_status` errors tag `operation: "poll_pr_status"`; CI status fetch failures are swallowed as `CiStatus::Unknown` with no error propagation — ensures that an unreachable commit status endpoint never breaks status polling
- How a future agent inspects this: add `tracing::debug!` calls (optional in S01, required in S03 when the poll loop is built) — the mock tests exercise state transitions that a future agent can trace in production via `RUST_LOG=debug`
- Failure state exposed: the `PrState` enum in `PrStatus` is the authoritative state indicator for downstream S03 polling logic; `CiStatus::Unknown` signals "couldn't determine" without masking other info

## Inputs

- `crates/smelt-core/src/forge.rs` — T02 output with `parse_repo()` helper, `create_pr()` implemented, 3 tests passing
- Research notes: `PrState` derivation from `pr.merged`/`pr.state`; combined status endpoint path `/repos/{owner}/{repo}/commits/{sha}/status`; octocrab `_get()` for raw HTTP; `review_comments` is the no-extra-call review count field

## Expected Output

- `crates/smelt-core/src/forge.rs` — `poll_pr_status()` implemented; all 6 tests passing; `CombinedStatus` local struct for CI deserialization; slice complete
- `.kata/DECISIONS.md` — D054 appended (review_count: inline comments vs submitted reviews choice)
