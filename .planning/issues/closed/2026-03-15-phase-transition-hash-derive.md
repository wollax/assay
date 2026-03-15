# `PhaseTransition` Missing `Hash` Derive

## Description

`SessionPhase` derives `Hash` (line 18), but `PhaseTransition`, which embeds two `SessionPhase` values, does not. For consistency and to support use in `HashSet` or as a map key (e.g., deduplicating transition records), `PhaseTransition` should also derive `Hash`. `DateTime<Utc>` and `String` both implement `Hash`, so there is no blocker.

## File Reference

`crates/assay-types/src/work_session.rs` — `PhaseTransition` derive list (line 76)

## Category

types / derives
