# S02 Post-Slice Roadmap Assessment

**Date:** 2026-03-21
**Verdict:** Roadmap unchanged — no slice modifications needed.

## Success Criteria Coverage

| Criterion | Remaining Owner |
|-----------|----------------|
| `smelt run` with `[forge]` creates PR, prints URL | S06 (end-to-end proof; mechanism complete in S01+S02) |
| `smelt status` renders PR section (state, CI, review count) | S03 |
| `smelt watch` blocks until merged (exit 0) or closed (exit 1) | S03 |
| `smelt init` generates skeleton manifest passing `--dry-run` | S04 |
| `smelt-core` usable as path dependency with `forge` feature | S05 |
| Concurrent runs use isolated state directories | S04 |

All six success criteria have at least one remaining owning slice. Coverage intact.

## Risk Retirement

- **GitHub API auth/rate limits** — retired in S01. Unit tests with wiremock mock HTTP servers covered create_pr happy path, 401, 422, and poll_pr_status state transitions.
- **Phase 9 guard logic** — retired in S02. `should_create_pr()` covers all 8 (no_pr × no_changes × forge) combinations with unit tests.

## Boundary Contract Accuracy

S02 delivered exactly what the boundary map specifies:
- `RunState.pr_url: Option<String>` and `RunState.pr_number: Option<u64>` — persisted to `.smelt/run-state.toml` after Phase 9.
- `--no-pr` flag — skips Phase 9 even when forge config is present.

S03 can read `pr_url`/`pr_number` directly from `RunState` with no new state infrastructure.

## Forward Notes for S03

- **D058 applies:** smelt-cli unconditionally enables the forge feature. S03's `watch.rs` needs no `#[cfg(feature = "forge")]` guards and no Cargo.toml changes.
- **`JobPhase::Failed` ambiguity:** Phase 9 failure propagates as `JobPhase::Failed` with `pr_url: None` — indistinguishable from container failure at the state level. S03's `smelt watch` should guard on `pr_url: None` and emit a clear message ("no PR found for this job") rather than silently polling. A distinct `JobPhase::PrFailed` is worth considering in S04/S05 API surface cleanup but is not a blocker.
- **review_count source (D054):** Currently `pr.review_comments` (inline diff comments). If S03 needs submitted approval count for `smelt status`, switch to `pulls.list_reviews(number)` — D054 is marked revisable for S03.

## Requirement Coverage

- R001 (smelt run creates GitHub PR) — active, Phase 9 functional; live proof deferred to S06 UAT.
- R002 (manifest forge config block) — validated by S02 automated tests.
- R003 (smelt status PR section) — active, owned by S03. ✓
- R004 (smelt watch) — active, owned by S03. ✓
- R005 (smelt-core library API) — active, owned by S05. ✓
- R006 (concurrent state isolation) — active, owned by S04. ✓
- R007 (smelt init) — active, owned by S04. ✓
- R008 (.assay/ gitignore guard) — active, owned by S04. ✓

All active requirements retain valid slice ownership. No gaps introduced by S02.
