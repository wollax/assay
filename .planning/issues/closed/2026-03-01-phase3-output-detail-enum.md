> **Closed:** 2026-03-15 — Won't fix. Superseded by v0.4.0 architecture (phases 35-44).


---
created: 2026-03-01T05:30
title: Define OutputDetail enum for semantic verbosity control
area: assay-types
phase: 3
provenance: brainstorm:2026-02-28T23-16-brainstorm/architecture-report.md
files:
  - crates/assay-types/src/lib.rs
---

## Problem

Different consumers need different verbosity from gate results. Byte truncation (`max_output_bytes`) is blind and destroys structure. Semantic verbosity control — knowing that failures matter more than passing test names — is strictly better.

## Solution

Add an `OutputDetail` enum to the domain model:

```rust
enum OutputDetail {
    Full,       // everything: stdout, stderr, exit code, duration, all test names
    Standard,   // exit code + failures + summary (passing tests omitted)
    Compact,    // exit code + failure count only
}
```

Configurable per gate in TOML (`detail = "standard"`). Default to `Full` until the orchestrator exists. Implementation of the rendering branches happens in Phase 7 (gate evaluation).