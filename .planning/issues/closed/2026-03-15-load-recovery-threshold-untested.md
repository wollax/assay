# `load_recovery_threshold` Is Entirely Untested

## Description

`load_recovery_threshold` in `server.rs` has no test coverage at all. It is the highest-value test gap in the recovery feature: the function determines whether a stale session crosses the threshold that triggers recovery, so any regression would silently change recovery behaviour without a failing test.

## File Reference

`crates/assay-mcp/src/server.rs` — `load_recovery_threshold`

## Category

testing / recovery
