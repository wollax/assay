# Integration Architecture

This document describes how the three parts of the Assay monorepo communicate: **assay** (spec-driven dev kit), **smelt** (infrastructure layer), and **plugins** (agentic AI integrations). It also covers the external **cupel** dependency.

## Data Flow Diagram

```
                                HOST                                          CONTAINER
 ┌─────────────────────────────────────────────────────┐     ┌─────────────────────────────────────┐
 │                                                     │     │                                     │
 │  ┌──────────┐    stdio      ┌────────────┐          │     │   ┌─────────┐                       │
 │  │  Plugin   │─────────────>│ assay mcp  │          │     │   │  assay  │                       │
 │  │(claude,   │  MCP JSON-RPC│  serve     │          │     │   │  run    │                       │
 │  │ opencode, │<─────────────│ (40 tools) │          │     │   │         │                       │
 │  │ codex)    │              └──────┬─────┘          │     │   └────┬────┘                       │
 │  └──────┬────┘                     │                │     │        │                             │
 │         │ hooks                    │ HTTP :7432     │     │        │ HTTP POST                   │
 │         │ (assay CLI)              │ signal server  │     │        │ SMELT_EVENT_URL             │
 │         v                          │                │     │        v                             │
 │  ┌──────────┐                      │                │     │   ┌──────────┐                      │
 │  │ assay    │                      │                │     │   │  Smelt   │                      │
 │  │ CLI      │                      │                │     │   │  Backend │                      │
 │  └──────────┘                      │                │     │   └────┬─────┘                      │
 │                                    │                │     │        │                             │
 │  ┌──────────────────────────┐      │                │     │        │ HTTP :7432                  │
 │  │  smelt server            │      │                │     │        │ (signal endpoint)           │
 │  │                          │      │                │     │        v                             │
 │  │  HTTP API (:port)        │<─────┼────────────────┼─────┼───── POST /api/v1/events            │
 │  │  ┌────────────────┐     │      │                │     │                                     │
 │  │  │ Event Ingestion │     │      │                │     │   ┌─────────────┐                   │
 │  │  │ POST /events    │     │      │                │     │   │ assay signal│                   │
 │  │  └───────┬────────┘     │      │                │     │   │ server :7432│                   │
 │  │          │              │      │                │     │   └──────▲──────┘                   │
 │  │          v              │      │                │     │          │                           │
 │  │  ┌────────────────┐     │      │                │     │          │                           │
 │  │  │ [[notify]]      │     │  PeerUpdate          │     │          │                           │
 │  │  │ Rule Engine     │─────┼──────┼────────────────┼─────┼──────────┘                           │
 │  │  └────────────────┘     │  HTTP-first           │     │   (fs fallback if HTTP fails)       │
 │  │                          │      │                │     │                                     │
 │  │  ┌────────────────┐     │      │                │     │                                     │
 │  │  │ Dispatch Engine │─────┼──────┼────────────────┼─────┼─> docker exec / k8s exec           │
 │  │  │ (Docker/K8s)    │     │      │                │     │   writes manifest + specs           │
 │  │  └────────────────┘     │      │                │     │   invokes `assay run`               │
 │  └──────────────────────────┘      │                │     │                                     │
 │                                    │                │     │                                     │
 └─────────────────────────────────────────────────────┘     └─────────────────────────────────────┘

 ┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
 │  COMPILE-TIME DEPENDENCY                                                                        │
 │                                                                                                 │
 │  assay-types ──path dep──> smelt-core                                                           │
 │  cupel (crates.io) ──────> assay-core                                                           │
 └──────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## Integration Points

| # | From | To | Binding | Protocol | Description |
|---|------|----|---------|----------|-------------|
| 1 | smelt-core | assay-types | Compile-time | Cargo path dep | `path = "../../../crates/assay-types"` imports `StateBackendConfig`, `GateSummary`, `PeerUpdate`, `SignalRequest` |
| 2 | assay-core | cupel | Compile-time | crates.io dep | cupel 1.2.0 for context budgeting and token-aware diff slicing |
| 3 | smelt dispatch | assay run | Runtime | Docker/K8s exec | Provisions container, writes manifest via base64-encoded exec, invokes `assay run /tmp/smelt-manifest.toml` |
| 4 | assay (in-container) | smelt server | Runtime | HTTP POST | `SmeltBackend` POSTs `OrchestratorStatus` JSON to `SMELT_EVENT_URL` (`/api/v1/events?job=<id>`) |
| 5 | smelt server | assay (in-container) | Runtime | HTTP POST + fs fallback | `PeerUpdate` delivered via HTTP to container signal endpoint (`:7432/api/v1/signal`), filesystem fallback to inbox dir |
| 6 | smelt server | smelt server (peer) | Runtime | HTTP POST | Cross-peer signal forwarding with `X-Assay-Forwarded` loop prevention |
| 7 | plugin | assay MCP | Runtime | stdio JSON-RPC | Plugin registers `assay mcp serve`, exposing 40 MCP tools |
| 8 | plugin hooks | assay CLI | Runtime | Shell exec | Claude Code hooks invoke `assay` CLI for gate checks and checkpoints |

## Protocol Descriptions

### 1. assay-types to smelt-core (Compile-Time Path Dependency)

`smelt-core` depends on `assay-types` via a workspace-relative path dependency:

```toml
# smelt/crates/smelt-core/Cargo.toml
[dependencies]
assay-types = { path = "../../../crates/assay-types" }
```

Imported types:

- **`StateBackendConfig`** -- enum with variants: `LocalFs`, `Linear`, `GitHub`, `Ssh`, `Smelt`, `Custom`. Used in `JobManifest` and `SmeltRunManifest` to configure how Assay persists state inside the container.
- **`GateSummary`**, **`PeerUpdate`**, **`SignalRequest`** -- signal types re-exported at the `smelt_core` top level (`pub use assay_types::signal::{...}`). This ensures both sides share identical serialization schemas with zero drift.

Changes to any of these types in `assay-types` require rebuilding smelt: `just build-smelt`.

### 2. assay-core to cupel (External Crate Dependency)

`assay-core` depends on `cupel = "1.2.0"` from crates.io. Cupel is a context engine for token-budgeted diff slicing. Assay uses it in `assay-core/src/context/budgeting.rs` to:

- Map content sources (diffs, specs, criteria) to cupel pipeline items
- Run the priority-based selection pipeline under a token budget
- Truncate oversized diffs via cupel's slicing strategy
- Report diagnostics via `cupel-otel` (optional telemetry feature)

### 3. Smelt to Assay -- Container Invocation

When `smelt run` dispatches a job, the flow is:

1. **Provision**: Smelt creates a Docker container, Compose stack, or Kubernetes pod via `RuntimeProvider`.
2. **Write manifest**: `AssayInvoker` builds a `SmeltRunManifest` (TOML) and per-session spec files. These are base64-encoded and written into the container via exec:
   ```
   sh -c "echo '<base64>' | base64 -d > /tmp/smelt-manifest.toml"
   ```
3. **Invoke Assay**: Smelt constructs and executes the command:
   ```
   assay run /tmp/smelt-manifest.toml --timeout <N> --base-branch <REF>
   ```
4. **Collect results**: After Assay completes, Smelt collects the result branch via `ResultCollector` git operations.

Environment variables injected into the container:

| Variable | Value | Purpose |
|----------|-------|---------|
| `SMELT_EVENT_URL` | `http://<host>:<port>/api/v1/events` | Event callback endpoint on smelt server |
| `SMELT_JOB_ID` | Job identifier string | Scopes events to the dispatched job |
| `SMELT_WRITE_TOKEN` | Bearer token (optional) | Authenticates event POSTs |

### 4. Assay to Smelt -- Event Communication

Inside the container, when `StateBackendConfig::Smelt` is configured, Assay instantiates `SmeltBackend` (in `assay-backends`). On every orchestrator state transition, `push_session_event()` POSTs the serialized `OrchestratorStatus` JSON to:

```
POST {SMELT_EVENT_URL}?job={SMELT_JOB_ID}
Content-Type: application/json
Authorization: Bearer <token>   (when SMELT_WRITE_TOKEN is set)

{ ...OrchestratorStatus JSON... }
```

**Graceful degradation**: `SmeltBackend` never aborts a run due to communication failure. Non-2xx responses and connection errors emit `tracing::warn!` and return `Ok(())`.

### 5. Smelt to Assay -- PeerUpdate Signal Delivery

When the smelt server ingests a session-completion event (`payload.phase == "complete"`), it evaluates `[[notify]]` rules from the source job's manifest:

```toml
# In a smelt job manifest
[[notify]]
target_job = "frontend"
on_session_complete = true
```

For each matching rule, Smelt builds a `PeerUpdate` containing `GateSummary` (passed/failed/skipped counts), source session name, and source job name, then delivers it:

1. **HTTP-first** (preferred): POST `SignalRequest` wrapping the `PeerUpdate` to the target container's cached signal URL:
   ```
   POST http://<container_ip>:7432/api/v1/signal
   Content-Type: application/json
   ```
2. **Filesystem fallback**: If HTTP delivery fails (timeout, connection refused, no cached URL), write the `PeerUpdate` as a JSON file to the target session's inbox:
   ```
   <repo>/.assay/orchestrator/<run_id>/mesh/<session_name>/inbox/peer_update_<nanos>_<uuid>.json
   ```

The filesystem write uses atomic persistence (NamedTempFile + sync_all + persist).

### 6. Cross-Peer Signal Forwarding

When Assay's signal server (`:7432`) receives a `POST /api/v1/signal` for an unknown local session, it queries the peer registry and forwards to known peers. The first peer returning `202 Accepted` wins.

**Loop prevention**: Forwarded requests include `X-Assay-Forwarded: true`. A signal server receiving this header for an unknown session returns `404` immediately instead of re-forwarding. This guarantees at most one hop.

### 7. Plugins to Assay -- MCP Integration

Plugins register the Assay MCP server via stdio transport:

```json
{
  "mcpServers": {
    "assay": {
      "type": "stdio",
      "command": "assay",
      "args": ["mcp", "serve"]
    }
  }
}
```

The MCP server exposes 40 tools covering: spec CRUD, gate execution and evaluation, session management, worktree operations, milestone lifecycle, criteria management, PR creation, orchestration, context diagnostics, and signal exchange.

A separate HTTP signal server starts on port 7432 (configurable via `ASSAY_SIGNAL_PORT`) for cross-job signaling alongside the stdio MCP channel.

### 8. Plugin Hooks to Assay CLI

The Claude Code plugin defines hooks in `hooks.json` that invoke the `assay` CLI:

| Hook | Trigger | Script |
|------|---------|--------|
| PostToolUse (Write/Edit) | After file modifications | `post-tool-use.sh` -- progress tracking |
| PostToolUse (Task*) | After task operations | `checkpoint-hook.sh` -- state checkpoint |
| PreCompact | Before context compaction | `checkpoint-hook.sh` -- preserve state |
| Stop | Agent stop | `cycle-stop-check.sh` + `checkpoint-hook.sh` -- gate check and checkpoint |
