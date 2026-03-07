---
created: 2026-03-02T14:30
title: gate_run has no timeout parameter for agents
area: mcp
provenance: github:wollax/assay#31
severity: important
files:
  - crates/assay-mcp/src/server.rs:198
  - crates/assay-core/src/gate/mod.rs:112-117
---

## Problem

`gate_run` hardcodes `cli_timeout: None` when calling `evaluate_all`. The `evaluate_all` signature takes `cli_timeout: Option<u64>` as the highest-precedence timeout override, but the MCP tool has no `timeout` parameter exposed in `GateRunParams`. Agents cannot override timeouts on slow gate commands, unlike the CLI which exposes `--timeout`.

## Solution

Add `timeout_secs: Option<u64>` with `#[serde(default)]` to `GateRunParams` and pass it as the `cli_timeout` argument to `evaluate_all`. Update the tool description to mention the timeout parameter.

## Resolution

Resolved during Phase 17-01. `timeout: Option<u64>` on `GateRunParams`.
