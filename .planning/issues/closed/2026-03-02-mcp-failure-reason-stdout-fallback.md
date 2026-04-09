---
created: 2026-03-02T14:30
title: Failure reason only checks stderr, misses stdout-only errors
area: mcp
provenance: github:wollax/assay#30
severity: important
files:
  - crates/assay-mcp/src/server.rs:305-307
---

## Problem

`format_gate_response` extracts the failure `reason` from stderr only via `first_nonempty_line(&gate_result.stderr)`. Some tools (e.g., linters) write errors to stdout, not stderr. When stderr is empty and stdout has content, the `reason` field is `"unknown"` even though actionable text is available.

## Solution

Fall back to stdout before `"unknown"`:
```rust
let reason = first_nonempty_line(&gate_result.stderr)
    .or_else(|| first_nonempty_line(&gate_result.stdout))
    .unwrap_or("unknown")
    .to_string();
```
Add a test case for the stdout fallback.
