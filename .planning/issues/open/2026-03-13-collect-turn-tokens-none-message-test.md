---
created: 2026-03-13T10:45
title: Test collect_turn_tokens with assistant message=None
area: core
provenance: local
files:
  - crates/assay-core/src/context/tokens.rs:117-128
---

## Problem

`collect_turn_tokens_from_entries` uses `a.message.as_ref()?.usage.as_ref().map(...)`, so an assistant entry with `message: None` should be silently skipped. No test covers this case, which is a real-world occurrence (sidechain entries sometimes have null messages).

## Solution

Add a test entry with `message: None` to the `collect_turn_tokens_filters_sidechains` test fixture and verify the entry is excluded from results.
