---
name: run-dispatch
description: Read a RunManifest, configure a StateBackendConfig, and dispatch a run via Assay MCP tools. Use when a smelt worker needs to start an orchestrated run on a remote or local machine.
---

# Skill: Run Dispatch

## Overview

Dispatching a run means providing a `RunManifest` to Assay and receiving a `run_id` to track progress. The `RunManifest` optionally carries a `state_backend` field that overrides where orchestrator state is persisted.

## Step 1 — Locate the RunManifest

The manifest is a TOML file (`run.toml` or as specified by the controller). Required fields:

```toml
[[sessions]]
spec = "my-spec"
name = "session-a"

# Optional: override state backend
[state_backend]
type = "local_fs"
```

## Step 2 — Configure the StateBackendConfig (optional)

If the controller wants state on a remote backend, set the `state_backend` field. Currently supported variants:

- `{ type = "local_fs" }` — filesystem under `.assay/orchestrator/<run_id>/` (default)
- `{ type = "custom", name = "my-backend", config = { ... } }` — custom third-party backend

For local smelt workers, omit `state_backend` (defaults to `LocalFs`).

## Step 3 — Choose the right dispatch tool

| Scenario | Tool | Key params |
| --- | --- | --- |
| Single session | `run_manifest` | `manifest_path`, `timeout_secs` |
| Multi-session DAG/Mesh/Gossip | `orchestrate_run` | `manifest_path`, `failure_policy`, `merge_strategy` |

## Step 4 — Dispatch with `orchestrate_run`

```json
{
  "tool": "orchestrate_run",
  "arguments": {
    "manifest_path": "/path/to/run.toml",
    "failure_policy": "skip_dependents",
    "merge_strategy": "completion_time"
  }
}
```

Returns: `{ "run_id": "01HXY...", "sessions": [...], "summary": { ... } }`

Save `run_id` — it is required for status queries.

## Step 5 — Dispatch with `run_manifest` (single session)

```json
{
  "tool": "run_manifest",
  "arguments": {
    "manifest_path": "/path/to/run.toml",
    "timeout_secs": 300
  }
}
```

Returns: `{ "sessions": [...], "summary": { ... } }`

## Notes

- `failure_policy` options: `"skip_dependents"` (default) | `"abort"`
- `merge_strategy` options: `"completion_time"` (default) | `"file_overlap"`
- `mode` in the manifest controls execution: `"dag"` | `"mesh"` | `"gossip"`
- A missing `state_backend` field defaults to `LocalFs` — no config change needed for single-machine runs
