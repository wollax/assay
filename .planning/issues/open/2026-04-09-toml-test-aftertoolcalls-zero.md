---
title: Add TOML test for AfterToolCalls n=0 rejection
area: assay-types
severity: suggestion
source: PR review (Phase 61)
---

The `when_after_tool_calls_zero_rejected` test uses JSON only. Add a TOML variant to verify NonZeroU32 rejection works via TOML deserialization path too.
