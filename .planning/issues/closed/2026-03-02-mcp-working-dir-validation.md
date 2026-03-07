---
created: 2026-03-02T14:30
title: resolve_working_dir does not validate path exists
area: mcp
provenance: github:wollax/assay#37
severity: important
files:
  - crates/assay-mcp/src/server.rs:250-262
---

## Problem

If `config.gates.working_dir` is set to a non-existent directory, `resolve_working_dir` returns the path silently. `evaluate_all` then fails each criterion with spawn errors (because `Command::current_dir` to a non-existent path fails), producing cryptic per-criterion failures instead of one clear upfront error.

## Solution

Add a `Path::exists()` check in `resolve_working_dir` (or in `gate_run` after calling it) and return a domain error before running any gates if the directory doesn't exist.

## Resolution

Resolved during Phase 17-01. `is_dir()` check before gate evaluation.
