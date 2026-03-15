# `gate_runs: Vec<String>` ID Format Undocumented

## Description

The `gate_runs` field doc comment says only "IDs of gate runs associated with this session." It does not state what format these IDs have (e.g., whether they are ULIDs, the same format as history record IDs, or something else). Callers populating this field have no contract to rely on, and readers of persisted sessions cannot validate or parse the IDs without inspecting the broader codebase. The doc comment should cross-reference the canonical ID format and where these values originate.

## File Reference

`crates/assay-types/src/work_session.rs` — `WorkSession::gate_runs` (line 144)

## Category

docs
