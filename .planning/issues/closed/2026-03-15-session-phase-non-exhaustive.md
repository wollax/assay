# `SessionPhase` Should Be `#[non_exhaustive]`

## Description

`SessionPhase` is a serialized, on-disk enum that is expected to evolve (the doc comment already references a linear pipeline that may grow). Without `#[non_exhaustive]`, downstream `match` arms compiled against a future binary will break without a warning. Adding `#[non_exhaustive]` forces all external `match` sites to include a catch-all, making forward-compatible evolution explicit rather than accidental.

## File Reference

`crates/assay-types/src/work_session.rs` — `SessionPhase` enum (line 20)

## Category

types / forward-compatibility
