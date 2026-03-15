# Phase 41 UAT: Session MCP Tools

## Tests

| # | Test | Expected | Status |
|---|------|----------|--------|
| 1 | session_create creates and persists a session | Returns session_id (ULID), phase="created", spec_name, created_at | PASS |
| 2 | session_create rejects unknown spec | Domain error mentioning spec not found | PASS |
| 3 | session_update transitions phase | previous_phase and current_phase correct, full lifecycle verified | PASS |
| 4 | session_update rejects invalid transition | Error for skip, terminal phase, and not-found | PASS |
| 5 | session_update links gate_run_ids | IDs appear on session, duplicates rejected | PASS |
| 6 | session_list with filters | spec_name, phase, combined, limit, and limit=0 clamp all correct | PASS |
| 7 | All responses include warnings field | warnings field present on all 4 tool response structs with skip_serializing_if | PASS |

## Result

7/7 tests passed. All Phase 41 success criteria verified.
