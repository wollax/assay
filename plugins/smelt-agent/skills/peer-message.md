---
name: peer-message
description: >
  Send and receive messages between sessions in mesh mode, or read the
  knowledge manifest in gossip mode. Use when coordinating between
  concurrent sessions during an orchestrated run.
---

# Peer Messaging

Communicate between sessions in mesh and gossip orchestration modes.

## Mesh Mode — Direct Messaging

In mesh mode, each session receives a `mesh-roster` PromptLayer listing peers and file paths for messaging.

### Sending Messages

1. **Parse the mesh roster.** The roster is injected as a system prompt layer named `mesh-roster`. It contains:
   ```
   # Mesh Roster for session: <your-name>
   Outbox: /path/to/.assay/orchestrator/<run_id>/mesh/<your-name>/outbox
   
   # Peers
   Peer: <peer-name>  Inbox: /path/to/.assay/orchestrator/<run_id>/mesh/<peer-name>/inbox
   ```

2. **To send a message to a peer:** Write a file to your outbox under a subdirectory named after the target peer:
   ```
   <outbox>/<target-name>/<filename>
   ```
   The background routing thread polls your outbox and moves the file to the target's inbox automatically.

3. **Message format is freeform.** Use plain text, JSON, or any format your peer can parse. File names should be descriptive (e.g. `findings.json`, `request-01.txt`).

### Receiving Messages

4. **Read files from your inbox directory.** The routing thread delivers messages from other sessions into your inbox. Poll the directory to check for new files.

### Capability Guard

5. **Check `supports_messaging` before relying on messaging.** If the state backend has `supports_messaging: false` in its `CapabilitySet`, the routing thread is not running. Messages written to outboxes will not be delivered. Sessions still execute in parallel, but without inter-session communication.

   To check: query `orchestrate_status` and look at `mesh_status.messages_routed`. If the value stays at zero despite sessions writing to outboxes, messaging is likely disabled by the backend.

## Gossip Mode — Knowledge Manifest

In gossip mode, there is no direct messaging between sessions. Instead, a coordinator synthesizes completed session results into a shared knowledge manifest.

### Reading the Knowledge Manifest

6. **Find the manifest path.** Each session receives a `gossip-knowledge-manifest` PromptLayer containing the absolute path to `knowledge.json`:
   ```
   /path/to/.assay/orchestrator/<run_id>/gossip/knowledge.json
   ```

7. **Read the manifest file.** It contains a JSON array of `KnowledgeEntry` objects, one per completed session. Each entry records what that session produced and discovered.

8. **The manifest updates incrementally.** After each session completes, the coordinator synthesizes its results and atomically updates `knowledge.json`. Read the file periodically to discover new entries from peers.

### Capability Guard

9. **Check `supports_gossip_manifest` before relying on the manifest.** If the backend has `supports_gossip_manifest: false`, the knowledge manifest may not persist between coordinator rounds. Check `gossip_status.sessions_synthesized` via `orchestrate_status` — if it stays at zero despite sessions completing, manifest persistence is disabled.
