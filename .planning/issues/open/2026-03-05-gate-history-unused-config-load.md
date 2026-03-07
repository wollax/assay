# gate_history loads config unnecessarily

**Source:** PR #57 review (code quality)
**Severity:** Important
**Area:** assay-mcp

## Description

`gate_history` loads the full config and discards it (`_config`). This performs filesystem I/O that serves no purpose. If the intent is to validate that we're in an Assay project, a lightweight validation function would be better.

## Location

`crates/assay-mcp/src/server.rs` — `gate_history` method (~line 672)

## Suggested Fix

Either remove the config load, or extract a lightweight `validate_assay_dir` helper.
