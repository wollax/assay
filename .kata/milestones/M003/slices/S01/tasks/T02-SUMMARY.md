---
id: T02
parent: S01
milestone: M003
provides:
  - "parse_repo() helper: splits 'owner/repo' slugs, returns SmeltError::Forge on missing '/'"
  - "GitHubForge::create_pr() implemented: calls octocrab pulls().create().body().send(), maps errors to SmeltError::Forge"
  - "Happy-path test passing: mock 201 response with required PullRequest fields verified against PrHandle{url,number}"
  - "Error tests passing: 401 and 422 responses both produce Err(SmeltError::Forge{operation:'create_pr',..)}"
  - "octocrab::Error Send+Sync+'static confirmed (D056): compile-time assertion passed in Rust 1.93.1/octocrab 0.49.5"
key_files:
  - crates/smelt-core/src/forge.rs
  - .kata/DECISIONS.md
key_decisions:
  - "D056: octocrab::Error IS Send+Sync+'static — forge_with_source() could carry #[source] field; kept as stringify for S01 MVP"
  - "Required PullRequest serde fields: url (String), id (PullRequestId), number (u64), head.ref+sha, base.ref+sha; all others are Option or serde(default)"
  - "Rust 1.93.1 unimplemented!(\"T03\") panics with 'not implemented: T03' (not 'not yet implemented: T03')"
patterns_established:
  - "parse_repo() private helper in forge.rs — reuse in T03's poll_pr_status for owner/repo splitting"
  - "Mock JSON for octocrab PullRequest: url+id+number+head{ref,sha}+base{ref,sha} are the required fields; locked/maintainer_can_modify have serde(default)"
observability_surfaces:
  - "SmeltError::Forge{operation:'create_pr', message} carries octocrab error verbatim including HTTP status and GitHub error body"
  - "cargo test -p smelt-core --features forge test_create_pr -- --nocapture shows exact error strings surfaced to callers"
duration: 20min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T02: Implement create_pr() and make its three tests pass

**`GitHubForge::create_pr()` implemented via octocrab; all 3 create_pr tests pass; octocrab::Error Send+Sync confirmed.**

## What Happened

Replaced `unimplemented!("T02")` in `GitHubForge::create_pr()` with a real octocrab call. Added a private `parse_repo()` helper that splits `"owner/repo"` slugs on the first `/` and returns `SmeltError::Forge` on missing separator — this helper is ready for reuse in T03's `poll_pr_status`.

The happy-path test required updating the mock JSON to include all non-optional `PullRequest` serde fields: `url` (String, not Option), `id` (PullRequestId), and valid `head`/`base` objects. The initial test scaffold was missing `url` and `id`, which would have caused serde deserialization failures.

Added a compile-time assertion `fn _assert_octocrab_error_send_sync() where octocrab::Error: Send + Sync + 'static {}` in the test module. It compiled without errors, confirming the bound holds for octocrab 0.49.5 with Rust 1.93.1. Appended D056 to record this finding (D053 was the planning placeholder, D056 is the confirmed resolution).

The error tests (401, 422) work without mock JSON changes — octocrab wraps these as `octocrab::Error::GitHub { .. }` and `.to_string()` surfaces the GitHub error body, which then propagates verbatim in `SmeltError::Forge.message`.

## Verification

```
# All 3 create_pr tests pass
cargo test -p smelt-core --features forge test_create_pr 2>&1 | tail -5
→ test result: ok. 3 passed; 0 failed

# poll_pr_status tests still fail with unimplemented (3 occurrences of "not implemented")
cargo test -p smelt-core --features forge test_poll_pr_status 2>&1 | grep "not implemented" | wc -l
→ 3

# No-forge build still clean
cargo test -p smelt-core 2>&1 | tail -5
→ test result: ok. 2 passed; 0 failed
```

Note: `unimplemented!("T03")` in Rust 1.93.1 produces `"not implemented: T03"` (not `"not yet implemented: T03"`). The task plan's verification grep uses `"not yet implemented"` which produces 0, but `"not implemented"` correctly matches 3.

## Diagnostics

- `SmeltError::Forge{operation: "create_pr", message}` carries octocrab's `.to_string()` output verbatim — includes HTTP status code and GitHub JSON error body for all failure cases
- `cargo test -p smelt-core --features forge test_create_pr -- --nocapture` shows the exact error strings that would surface to callers
- `operation: "create_pr"` tag on all errors makes the call site unambiguous in structured logs

## Deviations

- Mock JSON for happy-path test required adding `url` and `id` fields not present in the T01 scaffold. The plan's suggested JSON was updated empirically by reading the octocrab `PullRequest` struct source — `url: String`, `id: PullRequestId`, `number: u64`, `head/base` with `ref+sha` are required; all others are `Option` or `#[serde(default)]`.
- `unimplemented!("T03")` message is `"not implemented: T03"` in Rust 1.93.1, not `"not yet implemented: T03"` as the plan's grep expected. The verification check in the task plan uses the wrong string, but the test behavior is correct.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/forge.rs` — `parse_repo()` helper added; `create_pr()` implemented; test mock JSON fixed; assertions added to test bodies; `_assert_octocrab_error_send_sync()` compile-time check added
- `.kata/DECISIONS.md` — D056 appended (octocrab::Error Send+Sync+'static confirmed)
