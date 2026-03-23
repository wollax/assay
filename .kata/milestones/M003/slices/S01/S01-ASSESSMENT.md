# S01 Post-Slice Roadmap Assessment

**Assessed:** 2026-03-21
**Verdict:** Roadmap unchanged — all remaining slices are still correctly scoped and ordered.

## Risk Retirement

S01 was marked `risk:high` for GitHub API auth/rate limits. The risk is retired:
- 6 wiremock tests prove `create_pr` and `poll_pr_status` contracts (happy path, 401, 422, state transitions)
- `octocrab::Error: Send+Sync+'static` confirmed (D056) — error wrapping path is clear
- `forge` feature flag isolation confirmed — zero octocrab in no-feature dep tree

Manual confirmation with a real `GITHUB_TOKEN` + test repo was intentionally deferred to S02, which performs a real `smelt run`. This is appropriate — mock-HTTP contract verification is all S01 needed to retire the API-shape risk.

## Boundary Map Accuracy

S01 delivered exactly what the boundary map specified. One implementation note for S02:
- `parse_repo()` is private in `forge.rs` — S02's manifest validation (`owner/repo` format check) must implement its own parser or duplicate the logic. This is within S02's scope and doesn't affect the boundary contract.
- D055 holds: `ForgeConfig` is unconditional, so `smelt-cli` can parse `[forge]` in `manifest.rs` without enabling the `forge` feature.

## Success Criterion Coverage

| Criterion | Remaining Owner |
|-----------|----------------|
| `smelt run` with `[forge]` creates PR, prints URL | S02 |
| `smelt status` renders PR section (state, CI, review count) | S03 |
| `smelt watch` blocks until merged (0) or closed (1) | S03 |
| `smelt init` generates skeleton manifest passing `--dry-run` | S04 |
| `smelt-core` usable as path dependency programmatically | S05 |
| Concurrent `smelt run` jobs don't clobber state | S04 |

All 6 success criteria covered. ✓

## Requirement Coverage

All 8 active requirements (R001–R008) retain their owning slices. No requirements were validated, invalidated, or re-scoped by S01. R001 and R005 advanced (forge types exist, API surface begun) but are not yet validated — that happens in S02 and S05 respectively.

## Known Limitations Carried Forward

- `review_count` from `pr.review_comments` (inline diff comments, not formal approvals) — S03 should evaluate switching to `list_reviews()` if approval count is needed
- No ETag/conditional-request support in `poll_pr_status()` — rate limit optimization scoped to S03
- `octocrab._get()` is a semi-private API used for CI status fetch — S03 should note this as fragile if octocrab is upgraded

## Conclusion

No roadmap changes. S02 is the correct next slice.
