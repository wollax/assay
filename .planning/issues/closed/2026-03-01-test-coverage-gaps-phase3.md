---
title: "Test coverage gaps from Phase 3 PR review"
area: assay-types, assay-core
priority: medium
source: PR review #24
---

# Test Coverage Gaps from Phase 3 PR Review

## Problem

Phase 3 tests cover happy-path serialization but miss several important cases identified during PR review:

1. **No deserialization failure tests for GateKind** — unknown variant (`kind = "Unknown"`) and missing tag (`cmd = "..."` without `kind`) should produce clean `Err`, not panic
2. **No GateResult JSON roundtrip test** — the `skip_serializing_if` + `#[serde(default)]` pairing on stdout/stderr/exit_code is only tested for serialization, not deserialization back
3. **No Criterion deserialization failure test** — missing required fields (`name` without `description`) should fail cleanly
4. **No GateKind JSON roundtrip** — internal tagging tested in TOML only, not JSON (cross-format)
5. **Exact `assert_eq!` on Display format is brittle** — the three `contains` assertions already verify the contract; exact match is redundant

## Solution

Add ~5 tests to assay-types and consider relaxing the exact Display assertion in assay-core.


## Resolution

Partially resolved in Phase 19 Plan 02 (2026-03-06). Key gaps addressed: GateKind unknown variant test, GateResult JSON roundtrip with skip fields, Criterion deser failure, scan empty directory, SpecError Display format. Remaining items (assert_eq Display brittleness, whitespace-only criterion name, multi-error criteria) are low-priority suggestions.
