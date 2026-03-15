# `full_lifecycle` Test Skips Per-Transition Field Assertions

## Description

The `full_lifecycle` test verifies the phase count (`transitions.len() == 3`) and overall equality after a save-and-load round-trip, but never inspects individual `PhaseTransition` fields (e.g., `from`, `to`, `trigger`, `notes`). The `transition_appends_audit_entry` test covers field-level assertions for a single transition in isolation, but the lifecycle test's transitions (including the `notes: Some("all criteria passed")` case) are not individually validated. A regression that corrupts specific fields could pass `assert_eq!(loaded, session)` if it is symmetric.

## File Reference

`crates/assay-core/src/work_session.rs` — `full_lifecycle` test (line 374)

## Category

tests / coverage
