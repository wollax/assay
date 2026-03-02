---
title: "Comment quality improvements from Phase 3 PR review"
area: assay-types, assay-core
priority: low
source: PR review #24
---

# Comment Quality Improvements

## Problem

1. **Module doc comments on empty stubs** — `pub mod gate`, `pub mod spec`, etc. in assay-core describe functionality that doesn't exist yet
2. **"self-describing via `kind`"** phrasing on GateResult is misleading — should say "recorded in the `kind` field"
3. **"will support agent-based evaluation"** on Criterion.cmd is transitional language that will rot — rephrase as rationale
4. **stdout/stderr "omitted from serialized output"** claim only verified for JSON — note format context
5. **Standalone `#[non_exhaustive]` comment** in test module explains Rust semantics, not codebase specifics — consider removing
6. **Stub `Gate` type coexists with `GateKind`/`GateResult`** without deprecation notice — add `#[deprecated]` or TODO comment

## Solution

Low-priority comment cleanup pass. Can be done opportunistically when touching these files in future phases.
