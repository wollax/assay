# Assay — Smelt Agent Instructions

You are a smelt worker agent operating within an Assay multi-agent orchestration run. You dispatch and monitor runs, query run state from the configured `StateBackend`, and coordinate with peer agents via backend messaging. All orchestration state flows through the `StateBackend` — use the MCP tools below to interact with it.

## Skills

| File | Description |
| --- | --- |
| `skills/run-dispatch.md` | Read a RunManifest, configure a StateBackendConfig, and dispatch a run |
| `skills/backend-status.md` | Query `orchestrate_status`, interpret `OrchestratorStatus`, and check CapabilitySet degradation |
| `skills/peer-message.md` | Use `send_message`/`poll_inbox` for agent-to-agent coordination via mesh outbox/inbox |

## MCP Tools

| Tool | Description |
| --- | --- |
| `run_manifest` | Execute a single-session RunManifest (manifest_path, timeout_secs) |
| `orchestrate_run` | Dispatch a multi-session RunManifest; returns run_id for status queries |
| `orchestrate_status` | Query status of a run by run_id; returns OrchestratorStatus JSON |

## Workflow

1. **Dispatch:** Load the `RunManifest` from disk. Set `state_backend` to match the controller's backend config. Call `orchestrate_run` (multi-session) or `run_manifest` (single-session).
2. **Monitor:** Poll `orchestrate_status` with the returned `run_id` until `phase` is `completed` or `partial_failure`.
3. **Coordinate:** In Mesh mode, write messages to your outbox (`<run_dir>/mesh/<name>/outbox/<target>/`) so the routing thread delivers them to peer inboxes.
4. **Report:** Read `OrchestratorStatus.sessions` for per-session outcomes. Report `phase`, `sessions[*].state`, and any `error` fields back to the controller.
5. **Degrade gracefully:** Check `CapabilitySet` flags before relying on optional features. If `supports_messaging` is false, skip peer messaging. If `supports_gossip_manifest` is false, skip knowledge manifest reads.
