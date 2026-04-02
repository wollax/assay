---
estimated_steps: 6
estimated_files: 1
---

# T02: Implement create_pr() and make its three tests pass

**Slice:** S01 — GitHub Forge Client
**Milestone:** M003

## Description

Replace the `unimplemented!("T02")` stub in `GitHubForge::create_pr()` with a real octocrab PR creation call. The three `test_create_pr_*` tests established in T01 drive the implementation — they must all pass at the end of this task. This also resolves the choice of how to wrap octocrab errors (D019/Provider pattern decision).

## Steps

1. **Parse repo param**: Extract a private `fn parse_repo(repo: &str) -> crate::Result<(&str, &str)>` helper that splits `"owner/repo"` on the first `/` and returns both parts. Return `SmeltError::forge("create_pr", "invalid repo: expected 'owner/repo' format")` if no `/` found. Re-use this helper in T03's `poll_pr_status`.

2. **Check octocrab::Error Send + Sync**: At the top of the implementation, add a compile-time assertion to confirm octocrab's error type bounds:
   ```rust
   // In a test or at module top — remove after confirming
   fn _assert_octocrab_error_send_sync()
   where octocrab::Error: Send + Sync + 'static {}
   ```
   If this compiles: upgrade `forge_with_source` in `error.rs` to store the source properly (add a `#[source]` field like `Provider`). If it does NOT compile: leave `forge_with_source` as the stringify version from T01. Record the finding as decision D053.

3. **Implement create_pr**: Replace `unimplemented!("T02")` with:
   ```rust
   let (owner, repo_name) = parse_repo(repo)?;
   let pr = self.client
       .pulls(owner, repo_name)
       .create(title, head, base)
       .body(body)
       .send()
       .await
       .map_err(|e| SmeltError::forge("create_pr", e.to_string()))?;
   Ok(PrHandle {
       url: pr.html_url.map(|u| u.to_string()).unwrap_or_default(),
       number: pr.number,
   })
   ```

4. **Fix happy-path test mock JSON**: Run `cargo test -p smelt-core --features forge test_create_pr_happy_path` and iterate on the mock JSON until octocrab deserializes successfully. Start with this minimal JSON body and add fields as tests fail:
   ```json
   {
     "url": "https://api.github.com/repos/owner/repo/pulls/42",
     "id": 1,
     "node_id": "PR_1",
     "html_url": "https://github.com/owner/repo/pull/42",
     "number": 42,
     "state": "open",
     "locked": false,
     "maintainer_can_modify": false,
     "head": {"label": "owner:feature", "ref": "feature", "sha": "abc123", "repo": null, "user": null},
     "base": {"label": "owner:main", "ref": "main", "sha": "def456", "repo": null, "user": null}
   }
   ```
   The `#[non_exhaustive]` on `PullRequest` means serde will tolerate missing fields with `Option` defaults. Add fields that octocrab marks as required (`id`, `number`, `state`, `locked`, `head`, `base` are the non-optional ones — discover the exact set empirically).

5. **Fix 401 test mock**: Respond with `{"message": "Bad credentials", "documentation_url": "https://docs.github.com/rest"}` at status 401. Octocrab wraps this as `octocrab::Error::GitHub { .. }`. The test asserts `matches!(result, Err(SmeltError::Forge { .. }))`.

6. **Fix 422 test mock**: Respond with `{"message": "Validation Failed", "errors": [{"message": "No commits between main and feature"}], "documentation_url": "..."}` at status 422. Same assertion pattern as 401.

## Must-Haves

- [ ] `test_create_pr_happy_path` passes: `PrHandle { url: "https://github.com/owner/repo/pull/42", number: 42 }`
- [ ] `test_create_pr_auth_error` passes: `Err(SmeltError::Forge { operation: "create_pr", .. })`
- [ ] `test_create_pr_validation_error` passes: `Err(SmeltError::Forge { operation: "create_pr", .. })`
- [ ] `test_poll_pr_status_*` tests still fail with `unimplemented!("T03")` (not regressed to compile error)
- [ ] `cargo test -p smelt-core` (no features) still passes
- [ ] D053 appended to `.kata/DECISIONS.md` recording the octocrab::Error Send+Sync finding

## Verification

```bash
# create_pr tests must pass
cargo test -p smelt-core --features forge test_create_pr 2>&1 | tail -5
# Expected: test result: ok. 3 passed; 0 failed

# poll_pr_status tests still fail with unimplemented (not compile errors)
cargo test -p smelt-core --features forge test_poll_pr_status 2>&1 | grep "not yet implemented" | wc -l
# Expected: 3

# No-forge build still clean
cargo test -p smelt-core 2>&1 | tail -3
# Expected: test result: ok. N passed; 0 failed
```

## Observability Impact

- Signals added/changed: octocrab HTTP error messages are now surfaced verbatim in `SmeltError::Forge.message` — callers see the GitHub API response text (HTTP status, `message` field from GitHub JSON) whenever a PR creation fails
- How a future agent inspects this: `cargo test -p smelt-core --features forge test_create_pr -- --nocapture` shows the exact error string that would surface to end users
- Failure state exposed: the `operation: "create_pr"` tag on all errors makes the call site unambiguous in logs

## Inputs

- `crates/smelt-core/src/forge.rs` — T01 output with stub and test skeletons
- `crates/smelt-core/src/error.rs` — T01 output with Forge variant
- Research notes: `OctocrabBuilder::new().pulls(owner, repo).create(title, head, base).body(body).send().await` is the call chain; `pr.html_url: Option<Url>`, `pr.number: u64`

## Expected Output

- `crates/smelt-core/src/forge.rs` — `create_pr()` implemented; `parse_repo()` helper added; happy-path and error mock JSON finalized in test bodies; 3 create_pr tests passing
- `.kata/DECISIONS.md` — D053 appended (octocrab::Error Send+Sync status + chosen error wrapping approach)
