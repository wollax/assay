# Assay — Smelt Worker Agent Instructions

You are a smelt worker agent executing Assay runs. Your job is to receive a run manifest, dispatch sessions through the orchestrator, monitor status, and report results. You interact with Assay exclusively through MCP tools.

## Skills

| Command | Description |
| --- | --- |
| `/assay:run-dispatch` | Dispatch a single or multi-session run from a manifest |
| `/assay:backend-status` | Query orchestrator status and interpret results |
| `/assay:peer-message` | Send and receive messages between sessions (mesh/gossip) |

## MCP Tools

| Tool | Description |
| --- | --- |
| `run_manifest` | Execute a single-session manifest |
| `orchestrate_run` | Launch a multi-session orchestrated run |
| `orchestrate_status` | Query status of an orchestrated run by run_id |
| `spec_list` | List all specs in the project |
| `spec_get` | Get a spec's full definition and criteria |
| `gate_run` | Run quality gates for a spec |
| `cycle_status` | Get active milestone progress |
| `cycle_advance` | Advance the active chunk |
| `chunk_status` | Get gate results for a specific chunk |

## Workflow

1. **Receive manifest:** Read the RunManifest TOML file to understand sessions, mode, and backend config
2. **Configure backend:** If `state_backend` is set, ensure the backend is available before dispatch
3. **Dispatch run:** Use `run_manifest` for single sessions or `orchestrate_run` for multi-session orchestration
4. **Monitor status:** Poll `orchestrate_status` with the returned `run_id` until the run completes
5. **Handle messaging:** In mesh mode, use outbox/inbox paths from the roster; in gossip mode, read the knowledge manifest
6. **Report results:** Interpret final `OrchestratorStatus` and surface per-session outcomes

### Backend Capability Awareness

Not all backends support every feature. Check the `CapabilitySet` before relying on messaging or gossip manifests:

- `supports_messaging: false` → mesh routing thread is inactive; messages won't be delivered
- `supports_gossip_manifest: false` → gossip knowledge manifest may not persist between rounds
- `supports_annotations: false` → run annotations are not stored
- `supports_checkpoints: false` → team checkpoints are not persisted

Capability-limited runs degrade gracefully — they are not failures.
