# Error Paths of `record_gate_result` / `complete_session` Are Untested

## Description

`record_gate_result` and `complete_session` both return an error when called from an invalid session phase. Neither of those error paths is covered by a test. Adding tests that call each function from a wrong phase (e.g., `Idle`) would pin the expected error messages and prevent silent regressions if the phase-guard logic changes.

## File Reference

`crates/assay-core/src/work_session.rs` — `record_gate_result`, `complete_session`

## Category

testing / error-handling
