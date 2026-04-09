---
title: Remove redundant checkpoint test after Option<When> migration
area: assay-core
severity: suggestion
source: PR review (Phase 61)
---

`criterion_with_no_when_skipped_at_tool_call_phase` and `criterion_with_session_end_when_skipped_at_tool_call_phase` in gate/mod.rs are now identical after the `Option<When>` to `When` migration (both use `When::SessionEnd`). One should be removed or differentiated.
