# `WorkSession.id` Should Be a `SessionId` Newtype

## Description

`WorkSession.id` is declared as a bare `String`. Wrapping it in a `SessionId` newtype would catch accidental misuse at compile time (e.g., passing a gate-run ID where a session ID is expected) and provides a natural place to encode invariants (ULID format, 26-char length). The companion `WorkSessionTransition` error variant also stores `session_id: String`; both would benefit from the newtype.

## File Reference

`crates/assay-types/src/work_session.rs` — `WorkSession::id` (line 130)
`crates/assay-core/src/error.rs` — `WorkSessionTransition { session_id: String, .. }`

## Category

types / type-safety
