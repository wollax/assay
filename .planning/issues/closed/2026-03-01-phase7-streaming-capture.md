> **Closed:** 2026-03-15 — Won't fix. Superseded by v0.4.0 architecture (phases 35-44).


---
created: 2026-03-01T05:30
title: Use streaming capture with byte budget for gate evaluation
area: assay-core
phase: 7
provenance: brainstorm:2026-02-28T23-16-brainstorm/deterministic-report.md
files:
  - crates/assay-core/src/lib.rs
---

## Problem

`Command::output()` captures unbounded stdout/stderr into `Vec<u8>`. Verbose test suites (10K+ lines of stack traces) cause runaway memory allocation and massive token consumption when evidence reaches agents.

## Solution

Use `BufReader` with configurable byte limit instead of `Command::output()`. Exit-code-aware capture strategy:

- **Passing gates (exit_code == 0):** Aggressive budget. First N + last N lines within limit.
- **Failing gates (exit_code != 0):** Conservative budget with error-marker preservation:
  1. Scan for common failure markers (`FAIL`, `ERROR`, `panicked`, `assertion failed`)
  2. Preserve marker lines + surrounding context
  3. Truncate longest blocks if markers exceed budget, but keep ALL markers
  4. Fall back to first N + last N if no markers found

Configuration:
```toml
[gate.defaults]
max_output_bytes = 32768  # ~8K tokens, configurable per-gate
```

**Limitation to document:** Marker scanning is best-effort, English-centric. Custom test runners with non-standard markers degrade gracefully to first N + last N truncation.

Also implement `OutputDetail` rendering branches (from Phase 3 enum) in the gate evaluation path.