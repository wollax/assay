# `list_sessions` Never Tested with Non-JSON Files in Sessions Directory

## Description

`list_sessions` filters directory entries by `.json` extension, silently skipping any other files. This filtering behaviour is never exercised in the tests: no test writes a `.txt`, `.tmp`, or no-extension file to the sessions directory and asserts it is excluded from the returned IDs. Without this, a regression that accidentally includes non-JSON filenames would go undetected.

## File Reference

`crates/assay-core/src/work_session.rs` — `list_sessions` (line 136), test `list_returns_sorted_ids` (line 337)

## Category

tests / coverage
