# `load_session` Does Not Validate Path Component

## Description

`save_session` calls `crate::history::validate_path_component(&session.id, "session ID")` before constructing the file path, guarding against path traversal. `load_session` constructs the path directly from the caller-supplied `session_id` string without the same validation. As defense in depth, `load_session` should apply the same check so that a maliciously crafted ID (e.g., `"../evil"`) cannot escape the sessions directory even on a read path.

## File Reference

`crates/assay-core/src/work_session.rs` — `load_session` (line 114), compare with `save_session` (line 86)

## Category

security / defense-in-depth
