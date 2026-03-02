---
phase: 08-mcp-server-tools
status: passed
started: 2026-03-02
completed: 2026-03-02
---

# Phase 8: MCP Server Tools — UAT

## Tests

| # | Test | Expected | Status |
|---|------|----------|--------|
| 1 | MCP server starts cleanly | First byte on stdout is `{` (no clap banner, no tracing) | PASS |
| 2 | tools/list returns three tools | gate_run, spec_get, spec_list with descriptions | PASS |
| 3 | spec_list returns spec entries | Array of {name, description, criteria_count} | PASS |
| 4 | spec_get returns full spec | Complete spec JSON with criteria and commands | PASS |
| 5 | spec_get with bad name | isError: true with helpful message | PASS |
| 6 | gate_run returns bounded summary | passed/failed/skipped counts, no stdout/stderr by default | PASS |
| 7 | gate_run with evidence | stdout/stderr included when include_evidence=true | PASS |
| 8 | Missing .assay/ handled | isError: true with config/path message | PASS |
| 9 | Tool descriptions self-documenting | Agent unfamiliar with Assay can understand each tool | PASS |
| 10 | Workspace checks pass | `just ready` passes (fmt + lint + test + deny) | PASS |

## Result

10/10 tests passed. No issues found.
