# `previous_phase` Captured via Outer Mutable Variable in `session_update` Handler

## Description

In the `session_update` MCP handler, `previous_phase` is captured by moving an outer `Option<SessionPhase>` variable inside a `FnOnce` closure passed to `with_session`. This pattern is fragile: the variable must be declared before the closure, its value is only meaningful after the closure runs, and the structure makes the data-flow non-obvious. Extracting the phase comparison into a return value from the closure (or using a dedicated struct) would make the intent clear and reduce the risk of the pattern breaking under refactoring.

## File Reference

`crates/assay-mcp/src/server.rs` — `session_update` handler

## Category

code-quality / maintainability
