# gate_finalize uses untyped JSON response

**Source:** PR #57 review (type design, code quality)
**Severity:** Important
**Area:** assay-mcp

## Description

`gate_finalize` uses an inline `serde_json::json!` macro while every other tool response uses a typed struct. This loses compile-time field checking and is inconsistent. It also lacks `required_passed`, `advisory_passed`, and `blocked` fields that `gate_run` provides.

## Location

`crates/assay-mcp/src/server.rs` — `gate_finalize` method (~line 646-655)

## Suggested Fix

Create a `GateFinalizeResponse` struct with proper fields and doc comments.
