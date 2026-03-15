> **Closed:** 2026-03-15 — Won't fix. Superseded by v0.4.0 architecture (phases 35-44).


---
title: "Type invariant enforcement gaps in domain types"
area: assay-types
priority: medium
source: PR review #24
---

# Type Invariant Enforcement Gaps

## Problem

Several domain types accept structurally invalid values:

1. **`cmd: String` accepts empty strings** in both `GateKind::Command` and `Criterion.cmd` — an empty command will fail at execution time with an opaque error
2. **`GateResult` allows inconsistent `kind`/`exit_code` combinations** — `AlwaysPass` with `exit_code: Some(1)` or `Command` with `exit_code: None` are structurally valid but semantically wrong
3. **`Criterion.name`/`description` accept empty strings** — validation is an assay-core concern per DTO rules, but no validation boundary is documented
4. **`GateKind` missing `#[non_exhaustive]`** — future `prompt`-based variant addition would be a breaking change

## Solution

Consider: `CommandStr` newtype for non-empty commands, `GateResult` constructors for coherence enforcement, `#[non_exhaustive]` on `GateKind`. Evaluate timing — some may be appropriate for Phase 7 (gate evaluation) when constructors are actually needed.