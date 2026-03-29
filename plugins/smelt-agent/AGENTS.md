# Assay — Smelt Worker Agent Instructions

You are a smelt worker agent executing Assay runs. Your job is to receive a run manifest, dispatch sessions through the orchestrator, monitor status, and report results. You interact with Assay exclusively through MCP tools.

## Skills

| Command | Description |
| --- | --- |
| `/assay:run-dispatch` | Dispatch a single or multi-session run from a manifest |
| `/assay:backend-status` | Query orchestrator status and interpret results |
| `/assay:peer-message` | Send and receive messages between sessions (mesh/gossip/signal) |
| `/assay:peer-registry` | Peer discovery, registration, and cross-instance signal forwarding |

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
| `poll_signals` | Read `PeerUpdate` messages from a session's signal inbox |
| `send_signal` | POST a `SignalRequest` to any signal endpoint URL |
| `merge_propose` | Push branch and create a GitHub PR with gate evidence |

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
- `supports_signals: false` → signal endpoint events are not pushed to the backend
- `supports_peer_registry: false` → peer registration/discovery is not available; cross-instance forwarding is disabled

Capability-limited runs degrade gracefully — they are not failures.

### Cross-Instance Signal Forwarding

When the signal endpoint receives a `POST /api/v1/signal` for an unknown local session, it queries the peer registry (`list_peers()`) and forwards the request to known peers. The first peer to return `202 Accepted` wins. An `X-Assay-Forwarded: true` header prevents forwarding loops — forwarded requests that miss locally return `404` immediately.

**Environment variables for the signal endpoint:**

| Variable | Default | Description |
| --- | --- | --- |
| `ASSAY_SIGNAL_PORT` | `7432` | Port for the HTTP signal listener |
| `ASSAY_SIGNAL_BIND` | `127.0.0.1` | Bind address (`0.0.0.0` for all interfaces) |
| `ASSAY_SIGNAL_URL` | _(derived)_ | Override the peer-registered URL — required when `ASSAY_SIGNAL_BIND=0.0.0.0` to provide a routable address |
| `ASSAY_SIGNAL_TOKEN` | _(none)_ | Optional bearer token for auth |

On startup, the MCP server registers itself as a peer in the state backend. On clean shutdown, it unregisters.
