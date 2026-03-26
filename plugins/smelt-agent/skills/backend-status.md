---
name: backend-status
description: >
  Query and interpret the status of an orchestrated Assay run. Use after
  dispatching a run with orchestrate_run to monitor progress, interpret
  per-session results, and understand capability-dependent degradation.
---

# Backend Status

Query orchestrator status and interpret the results.

## Steps

1. **Call `orchestrate_status` with the `run_id`** from a prior `orchestrate_run` invocation:
   ```
   orchestrate_status({ run_id: "<ULID>" })
   ```

2. **Interpret the response envelope.** The `orchestrate_status` tool returns a JSON object with two top-level fields:
   - `status` — the `OrchestratorStatus` object (described below)
   - `merge_report` — present if a merge phase was attempted (may be null)

   Access run status via `status.phase`, `status.sessions`, etc. — not at the top level.

3. **Read the `OrchestratorStatus` fields** inside the `status` object:

   - `run_id` — unique identifier for the run
   - `phase` — current run phase (JSON wire values are snake_case):
     - `"running"` — sessions are still being dispatched or executing
     - `"completed"` — all sessions finished successfully
     - `"partial_failure"` — at least one session failed; others may have completed or been skipped
     - `"aborted"` — run was stopped due to `abort` failure policy or external signal
   - `failure_policy` — the policy in effect (`skip_dependents` or `abort`)
   - `sessions` — array of `SessionStatus` entries, each with:
     - `name` — session identifier
     - `spec` — spec path or name
     - `state` — one of `"pending"`, `"running"`, `"completed"`, `"failed"`, `"skipped"` (snake_case in JSON)
     - `started_at`, `completed_at`, `duration_secs` — timing (null if not applicable)
     - `error` — error message if the session failed
     - `skip_reason` — reason if the session was skipped (e.g. "upstream 'auth' failed")
   - `started_at`, `completed_at` — run-level timestamps

4. **Check mode-specific status fields:**

   - `mesh_status` (present when `mode = "mesh"`):
     - `members` — per-member status snapshots
     - `messages_routed` — total count of messages routed between inboxes and outboxes

   - `gossip_status` (present when `mode = "gossip"`):
     - `sessions_synthesized` — number of sessions whose results have been synthesized into the knowledge manifest
     - `knowledge_manifest_path` — absolute path to `knowledge.json` on disk
     - `coordinator_rounds` — number of coordinator synthesis rounds completed

5. **Understand CapabilitySet degradation.** The state backend's `CapabilitySet` determines what features are active:
   - If `supports_messaging: false`, the mesh routing thread does not run. `mesh_status.messages_routed` will be zero — this is expected degradation, not a failure. Sessions still execute in parallel but cannot exchange messages.
   - If `supports_gossip_manifest: false`, the gossip knowledge manifest may not persist between coordinator rounds. Check `gossip_status.sessions_synthesized` to see if synthesis is working.
   - If `supports_checkpoints: false`, team checkpoint state is not persisted across restarts.

6. **Handle errors.** If `orchestrate_status` returns an error:
   - Verify the `run_id` matches exactly what `orchestrate_run` returned
   - The state file is written before sessions start, so timing is not normally a factor when using the run_id from a completed `orchestrate_run` call
   - The orchestrator state directory may have been cleaned up
