# 100-Session Cap in `recover_stale_sessions` Is Untested

## Description

`recover_stale_sessions` silently stops processing after 100 sessions (line 318). This cap is not covered by any test: neither the boundary (exactly 100 sessions) nor the overflow case (101+) is exercised. A test with a fixture of 101+ sessions would confirm the cap is enforced and that the `truncated` signal (if added — see related issue) surfaces correctly.

## File Reference

`crates/assay-core/src/work_session.rs` — `recover_stale_sessions`, line 318

## Category

testing / recovery
