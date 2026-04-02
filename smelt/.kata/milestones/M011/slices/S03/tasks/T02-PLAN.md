---
estimated_steps: 4
estimated_files: 3
---

# T02: README update + milestone verification pass

**Slice:** S03 — Health endpoint + final verification
**Milestone:** M011

## Description

Add a "Health Check" section to the README documenting the `/health` endpoint. Then run the full M011 milestone verification pass — every success criterion checked with evidence. S02-blocked criteria (eprintln migration, flaky test fix, tracing output) are documented as blocked/failing with clear attribution to S02's incomplete merge.

## Steps

1. Read `README.md` to locate the `smelt serve` section.
2. Add a "Health Check" subsection documenting: endpoint URL (`GET /health`), unauthenticated access, expected response (`200 {"status": "ok"}`), and use case (load balancer probes).
3. Run the full milestone verification pass and record evidence:
   - `wc -l` on decomposed files (S01 criterion — should still pass)
   - `rg 'eprintln!' crates/smelt-cli/src/ -c` (S02 criterion — will show remaining calls; document as S02-blocked)
   - `cargo test --workspace` (≥290 tests, 0 failures)
   - `cargo clippy --workspace` (zero warnings)
   - `cargo doc --workspace --no-deps` (zero warnings)
   - `cargo test -p smelt-cli health` (S03 health test passes)
4. Write the verification results into the slice summary, clearly separating S03-owned results (all should pass) from S02-blocked results (eprintln, tracing, flaky test).

## Must-Haves

- [ ] README contains "Health Check" section under `smelt serve` documentation
- [ ] Section documents: endpoint, unauthenticated nature, response format, use case
- [ ] Milestone verification pass executed with evidence for every criterion
- [ ] S02-blocked criteria clearly labeled as blocked (not S03 failures)
- [ ] `cargo test --workspace` ≥290 tests, 0 failures
- [ ] `cargo clippy --workspace` and `cargo doc --workspace --no-deps` clean

## Verification

- `rg 'Health Check' README.md` — section exists
- `rg '/health' README.md` — endpoint documented
- All cargo commands from step 3 pass (except S02-blocked criteria documented as known gaps)

## Observability Impact

- Signals added/changed: None (documentation task)
- How a future agent inspects this: read README.md; read slice summary verification table
- Failure state exposed: None

## Inputs

- `README.md` — current README with `smelt serve` section
- T01 output — health endpoint implemented and tested
- M011 success criteria from `M011-ROADMAP.md`
- S01-SUMMARY.md — prior verification results for decomposition criteria

## Expected Output

- `README.md` — modified: "Health Check" section added
- `.kata/milestones/M011/slices/S03/S03-SUMMARY.md` — written: verification report with pass/fail table
