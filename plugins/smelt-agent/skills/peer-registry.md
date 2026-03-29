---
name: peer-registry
description: >
  Peer discovery, registration, and cross-instance signal forwarding.
  Use when configuring multi-machine deployments where multiple Assay
  instances need to discover each other and forward signals across hosts.
---

# Peer Registry

Register, discover, and forward signals between Assay instances running on different machines.

## Overview

Each Assay MCP server can register itself as a **peer** in the state backend. Other instances query the peer registry to discover where to forward signals for sessions they don't own locally. This enables multi-machine orchestration without a central message broker.

```
┌──────────────┐         ┌──────────────┐
│  Machine A   │         │  Machine B   │
│  assay-mcp   │◄───────►│  assay-mcp   │
│  :7432       │  HTTP   │  :7432       │
│              │ forward │              │
│  worker-1    │         │  worker-2    │
│  worker-3    │         │  orchestrator│
└──────────────┘         └──────────────┘
        │                        │
        └────────┬───────────────┘
           peers.json (or Smelt API)
```

## PeerInfo Type

Each registered peer is a `PeerInfo` record:

```json
{
  "peer_id": "machine-a",
  "signal_url": "http://192.168.1.10:7432",
  "registered_at": "2026-03-29T12:00:00Z"
}
```

| Field | Type | Description |
| --- | --- | --- |
| `peer_id` | `String` | Unique identifier (typically hostname or UUID) |
| `signal_url` | `String` | HTTP endpoint for the signal server |
| `registered_at` | `DateTime<Utc>` | When this peer was registered |

## Backend Methods

The `StateBackend` trait provides three peer registry methods with default no-op implementations:

| Method | Description |
| --- | --- |
| `register_peer(peer: &PeerInfo)` | Upsert a peer entry (by `peer_id`) |
| `list_peers()` | Return all registered peers |
| `unregister_peer(peer_id: &str)` | Remove a peer entry (idempotent) |

### LocalFsBackend

Stores peers in `{assay_dir}/peers.json`. Writes are atomic (temp file + rename). Suitable for single-machine multi-process setups where all Assay instances share the same `.assay/` directory.

### SmeltBackend

Registers peers by POSTing `PeerInfo` JSON to `{smelt_url}/api/v1/peers`. Graceful degradation — registration failure logs a warning but does not abort startup. `list_peers` and `unregister_peer` use the default no-op implementations (Smelt manages peer lifecycle server-side).

### Other Backends

`NoopBackend`, `LinearBackend`, `GitHubBackend`, and `SshSyncBackend` all return `supports_peer_registry: false` and use the default no-op implementations.

## Automatic Lifecycle

The MCP server manages peer registration automatically:

1. **On startup** — after the signal endpoint binds, the server calls `register_peer` with its hostname and `signal_url` derived from `ASSAY_SIGNAL_BIND` and `ASSAY_SIGNAL_PORT`.
2. **On clean shutdown** — the server calls `unregister_peer` to remove itself.

No manual registration is needed for normal operation.

## Cross-Instance Signal Forwarding

When `POST /api/v1/signal` targets an unknown local session:

1. Check for `X-Assay-Forwarded: true` header — if present, return `404` immediately (loop prevention).
2. Check `capabilities().supports_peer_registry` — if false, return `404`.
3. Call `list_peers()` — iterate peers sequentially.
4. For each peer, POST the original `SignalRequest` to `{peer.signal_url}/api/v1/signal` with:
   - `X-Assay-Forwarded: true` header (prevents the receiving peer from re-forwarding)
   - `Authorization: Bearer <token>` header (if `ASSAY_SIGNAL_TOKEN` is set)
5. First peer to return `202 Accepted` wins — return `202` to the original caller.
6. If all peers fail or the list is empty, return `404`.

### Loop Prevention

The `X-Assay-Forwarded: true` header is the loop-prevention mechanism. A forwarded request that arrives at a peer is never re-forwarded — it either matches a local session (202) or fails (404). This guarantees at most one hop.

## Multi-Machine Setup

To deploy Assay across multiple machines:

1. **Set `ASSAY_SIGNAL_BIND=0.0.0.0`** and **`ASSAY_SIGNAL_URL=http://<machine-ip>:7432`** on each machine. Without `ASSAY_SIGNAL_URL`, the registered peer URL is `http://0.0.0.0:7432` — unroutable by other machines.
2. **Use a shared state backend** — either `LocalFsBackend` on a shared filesystem (NFS) or `SmeltBackend` with a central Smelt server.
3. **Start each Assay instance** — each registers itself as a peer automatically.
4. **Dispatch runs** — sessions on any machine can send signals to sessions on any other machine via `send_signal`. Unknown-session signals are forwarded through the peer registry.

### Environment Variables

| Variable | Default | Description |
| --- | --- | --- |
| `ASSAY_SIGNAL_PORT` | `7432` | Port for the HTTP signal listener |
| `ASSAY_SIGNAL_BIND` | `127.0.0.1` | Bind address (`0.0.0.0` for multi-machine) |
| `ASSAY_SIGNAL_URL` | _(derived)_ | **Required when `ASSAY_SIGNAL_BIND=0.0.0.0`** — override the peer-registered URL with the machine's reachable address (e.g. `http://192.168.1.10:7432`). Without this, peers register `http://0.0.0.0:7432` which is unroutable. |
| `ASSAY_SIGNAL_TOKEN` | _(none)_ | Optional bearer token for auth (shared across peers) |

## Capability Guard

Check `supports_peer_registry` before relying on peer discovery:

- `supports_peer_registry: true` — `LocalFsBackend`
- Note: `SmeltBackend` returns `false` — it implements `register_peer` as fire-and-forget but `list_peers` and `unregister_peer` are no-ops, so local signal forwarding is disabled; Smelt handles cross-instance routing server-side
- `supports_peer_registry: false` — `NoopBackend`, `LinearBackend`, `GitHubBackend`, `SshSyncBackend`

When peer registry is unavailable, signals for unknown local sessions return `404` without any forwarding attempt.
