# `PhaseTransition.trigger` Should Be an Enum

## Description

`PhaseTransition.trigger` is a free-form `String` but behaves as a structured enum in practice. The test code already demonstrates the two patterns in use: simple verb tokens (`"agent_started"`, `"gate_passed"`, `"auto_complete"`, `"user_abandoned"`) and a structured `gate_run:<id>` form that embeds a gate-run ID in free text. This makes the field hard to parse, impossible to match exhaustively, and inconsistent.

A `TransitionTrigger` enum with variants like `AgentStarted`, `GateRun { run_id: String }`, `AutoComplete`, `UserAbandoned`, etc. would make the structured data explicit and allow exhaustive handling.

## File Reference

`crates/assay-types/src/work_session.rs` — `PhaseTransition::trigger` (line 85)
`crates/assay-core/src/work_session.rs` — test `transition_appends_audit_entry` (line 232), `full_lifecycle` (line 396)

## Category

types / design
