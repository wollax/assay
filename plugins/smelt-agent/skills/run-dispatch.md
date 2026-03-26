---
name: run-dispatch
description: >
  Dispatch an Assay run from a manifest file. Use when you need to execute
  specs — either a single session via run_manifest or a multi-session
  orchestrated run via orchestrate_run. Covers manifest format, backend
  configuration, and capturing the run_id for status queries.
---

# Run Dispatch

Launch spec sessions from a RunManifest TOML file.

## Steps

1. **Read the RunManifest TOML file.** The manifest has these key fields:
   - `[[sessions]]` — array of session entries, each with `spec` (path or name), optional `name`, optional `depends_on` (list of session names for DAG ordering)
   - `mode` — coordination mode: `"dag"` (default, sequential with dependency edges), `"mesh"` (parallel with peer messaging), or `"gossip"` (parallel with knowledge synthesis)
   - `state_backend` — optional backend config (defaults to `LocalFs` if omitted)

2. **Check the `state_backend` field.** If present:
   - `"local_fs"` — default filesystem backend, no extra setup needed
   - `{ custom = { name = "...", config = {...} } }` — a third-party backend; ensure the named backend is available and properly configured before dispatch. The `config` value is backend-specific JSON.

   If `state_backend` is omitted, the orchestrator defaults to `LocalFs`.

3. **For single-session work, use `run_manifest`:**
   ```
   run_manifest({ manifest_path: "path/to/manifest.toml" })
   ```
   Optional parameter: `timeout_secs` (default: 600 seconds per session).

4. **For multi-session orchestration, use `orchestrate_run`:**
   ```
   orchestrate_run({
     manifest_path: "path/to/manifest.toml",
     failure_policy: "skip_dependents",
     merge_strategy: "completion_time",
     conflict_resolution: "skip"
   })
   ```
   Parameters:
   - `manifest_path` (required) — path to the manifest TOML file
   - `timeout_secs` (optional) — max seconds per agent subprocess (default: 600)
   - `failure_policy` (optional) — `"skip_dependents"` (default) skips downstream sessions on failure; `"abort"` stops all dispatch on first failure
   - `merge_strategy` (optional) — `"completion_time"` (default) orders merge by finish time; `"file_overlap"` picks sessions with least file overlap
   - `conflict_resolution` (optional) — `"skip"` (default) or `"auto"`

   Note: When using `orchestrate_run` in DAG mode, the manifest must contain either multiple sessions or at least one `depends_on` edge. For single-session work, use `run_manifest` instead.

5. **Capture the `run_id` from the response.** The `orchestrate_run` response includes a `run_id` (ULID string). Save this — you need it for `orchestrate_status` queries to monitor progress and get final results.
