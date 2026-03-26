---
name: backend-status
description: Query orchestrate_status, interpret OrchestratorStatus fields (phase, session states, mesh_status, gossip_status), and check CapabilitySet degradation. Use when monitoring an in-progress or completed run.
---

# Skill: Backend Status

## Overview

After dispatching a run with `orchestrate_run`, poll `orchestrate_status` to track progress. The response is an `OrchestratorStatus` JSON object. Understand the schema before reporting results to the controller.

## Step 1 — Query `orchestrate_status`

```json
{
  "tool": "orchestrate_status",
  "arguments": {
    "run_id": "01HXY..."
  }
}
```

Returns `{ "status": OrchestratorStatus, "merge_report": MergeReport | null }`.

## Step 2 — Interpret `OrchestratorStatus`

| Field | Type | Meaning |
| --- | --- | --- |
| `run_id` | string | Unique ULID for this run |
| `phase` | string | `"running"` \| `"completed"` \| `"partial_failure"` |
| `failure_policy` | string | `"skip_dependents"` or `"abort"` |
| `sessions` | SessionStatus[] | Per-session outcomes |
| `started_at` | ISO 8601 | When the run began |
| `completed_at` | ISO 8601 \| null | When the run finished (null if still running) |
| `mesh_status` | MeshStatus \| null | Present in Mesh mode only |
| `gossip_status` | GossipStatus \| null | Present in Gossip mode only |

## Step 3 — Interpret `SessionStatus`

| Field | Meaning |
| --- | --- |
| `state: "pending"` | Not yet started |
| `state: "running"` | Executing now |
| `state: "completed"` | Finished successfully |
| `state: "failed"` | Finished with error (see `error` field) |
| `state: "skipped"` | Skipped due to upstream failure (see `skip_reason`) |

## Step 4 — Read mode-specific status

**Mesh mode (`mesh_status`):**
```json
{
  "members": [{ "name": "session-a", "state": "completed", "last_heartbeat_at": "..." }],
  "messages_routed": 4
}
```
Member states: `"alive"` | `"suspect"` | `"dead"` | `"completed"`

**Gossip mode (`gossip_status`):**
```json
{
  "sessions_synthesized": 3,
  "knowledge_manifest_path": "/path/to/.assay/orchestrator/<run_id>/gossip/knowledge.json",
  "coordinator_rounds": 5
}
```

## Step 5 — Check CapabilitySet degradation

`CapabilitySet` is not directly in the MCP response, but degradation is visible from behavior:
- `messages_routed == 0` in Mesh mode despite active sessions → messaging capability absent; routing thread was not spawned
- No `"gossip-knowledge-manifest"` in session prompt layers → gossip manifest capability absent; PromptLayer was not injected

When `supports_messaging = false`, sessions still run in parallel — peer-to-peer coordination simply doesn't occur. When `supports_gossip_manifest = false`, sessions still complete — cross-session knowledge sharing is unavailable.

Report these degradation conditions to the controller with `warn` severity, not error.

## Polling pattern

Poll every 10–30 seconds until `phase` is `"completed"` or `"partial_failure"`. Report final `OrchestratorStatus` to the controller.
