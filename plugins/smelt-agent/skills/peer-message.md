---
name: peer-message
description: Use send_message/poll_inbox conventions for agent-to-agent coordination in Mesh mode. Covers the outbox/inbox file convention and mesh roster PromptLayer format. Use when a smelt worker needs to communicate with peer agents during a Mesh run.
---

# Skill: Peer Message

## Overview

In Mesh mode, sessions communicate by writing files to an outbox directory. The Assay routing thread polls outboxes and delivers files to the target session's inbox. This is a file-based message-passing protocol — no sockets or channels.

## Directory Convention

For a run with `run_id = "01HXY..."` and a session named `"worker-a"`:

```
.assay/orchestrator/01HXY.../mesh/
  worker-a/
    inbox/          ← receive messages here
    outbox/
      worker-b/     ← send messages to worker-b by writing here
      worker-c/     ← send messages to worker-c by writing here
```

The routing thread polls all `outbox/<target>/` subdirectories and moves files to the named target session's `inbox/`.

## Step 1 — Find your outbox path from the roster PromptLayer

Your session receives a `"mesh-roster"` system PromptLayer at launch. Parse it to find your outbox path:

```
# Mesh Roster for session: worker-a
Outbox: /path/to/.assay/orchestrator/01HXY.../mesh/worker-a/outbox

# Peers
Peer: worker-b  Inbox: /path/to/.assay/orchestrator/01HXY.../mesh/worker-b/inbox
Peer: worker-c  Inbox: /path/to/.assay/orchestrator/01HXY.../mesh/worker-c/inbox
```

Scan for the line starting with `"Outbox: "` to get your outbox path. Peer inboxes are listed for reference but you should write to your **own outbox subdirectory** — not directly to peer inboxes.

## Step 2 — Send a message

To send a message to `worker-b`, write a file to `<your_outbox>/worker-b/<message_name>`:

```python
# Example: write a message file
import os
outbox_dir = "/path/to/.assay/orchestrator/01HXY.../mesh/worker-a/outbox/worker-b"
os.makedirs(outbox_dir, exist_ok=True)
with open(os.path.join(outbox_dir, "status-update"), "wb") as f:
    f.write(b"gate-check complete: 3 pass, 0 fail")
```

The routing thread picks this up (polling every 50ms) and moves it to `worker-b/inbox/status-update`.

## Step 3 — Receive messages

Poll your inbox directory for new files:

```python
inbox_dir = "/path/to/.assay/orchestrator/01HXY.../mesh/worker-a/inbox"
for filename in os.listdir(inbox_dir):
    path = os.path.join(inbox_dir, filename)
    with open(path, "rb") as f:
        contents = f.read()
    os.remove(path)  # consume the message
    process(filename, contents)
```

Messages are delivered as files. Read and delete them to avoid re-processing. The routing thread uses atomic rename, so partially-written messages are never visible in the inbox.

## Step 4 — Check capability before messaging

Before relying on peer messaging, verify the capability is present. In Mesh mode with `LocalFsBackend`, messaging is always supported. With custom backends, it may not be.

Indicator that messaging is degraded: `mesh_status.messages_routed` stays 0 in `orchestrate_status` despite send attempts. In this case, skip peer coordination and proceed independently.

## Notes

- Message names must be non-empty and must not contain `/`, `\`, `.`, or `..`
- Files are delivered at-least-once (delete-after-read failure may cause re-delivery)
- The routing thread exits when all sessions have completed — do not send messages after your session finishes
- For knowledge manifest sharing in Gossip mode, read the file at the path in the `"gossip-knowledge-manifest"` PromptLayer instead of using the outbox/inbox mechanism
