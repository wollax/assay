# Phase 19: Testing & Tooling — UAT

## Tests

| # | Test | Expected | Status |
|---|------|----------|--------|
| 1 | cargo deny check bans | Exits 0 with multiple-versions=deny | PASS |
| 2 | cargo deny check sources | Exits 0 with unknown-registry=deny, unknown-git=deny | PASS |
| 3 | MCP handler unit tests pass | cargo test -p assay-mcp passes (47 tests) | PASS |
| 4 | Integration tests pass | cargo test -p assay-mcp --test mcp_handlers passes (7 tests) | PASS |
| 5 | Full workspace tests pass | cargo test --workspace passes (302 tests, 3 ignored) | PASS |
| 6 | Dogfooding spec parses | assay spec show self-check displays 5 criteria | PASS |
| 7 | Dogfooding gate run passes | assay gate run self-check exits 0 (4 pass, 1 skip) | PASS |
| 8 | Test-related issues triaged | 19 issues moved to .planning/issues/closed/ with notes | PASS |
| 9 | Insta snapshots exist | 3 snapshot files in assay-mcp (2 unit, 1 integration) | PASS |
| 10 | just ready passes | Full check suite green | PASS |

## Result: 10/10 PASSED
