# S04: smelt-agent plugin — UAT

**Milestone:** M010
**Written:** 2025-07-27

## UAT Type

- UAT mode: artifact-driven + human-experience
- Why this mode is sufficient: S04 is pure documentation (no code changes). Artifact-driven checks confirm file existence, frontmatter validity, and tool name correctness. Human-experience UAT is required to confirm a developer new to smelt-agent can actually follow the instructions and operate Assay orchestration from a smelt worker context.

## Preconditions

- `plugins/smelt-agent/` directory exists with AGENTS.md + 3 skills
- `just ready` is green (confirms no Rust regressions from documentation additions)
- Tester has access to an Assay project with at least one spec and a `RunManifest` file

## Smoke Test

Read `plugins/smelt-agent/AGENTS.md` end-to-end. Confirm you understand: (1) what a smelt agent is, (2) which skills are available, (3) which MCP tools are available, and (4) the basic dispatch→monitor lifecycle. If you can mentally follow all four without confusion, the smoke test passes.

## Test Cases

### 1. Run Dispatch — single session

1. Open `plugins/smelt-agent/skills/run-dispatch.md`
2. Follow the "Single-session dispatch" steps: write a minimal TOML manifest (`[[sessions]]` with `name` and `spec`) and call `run_manifest` with `manifest_path` and `timeout_secs`
3. **Expected:** You produce a valid manifest and know exactly which MCP tool to call with which parameters, without needing to look up any other documentation

### 2. Run Dispatch — multi-session orchestration

1. Open `plugins/smelt-agent/skills/run-dispatch.md`
2. Follow the "Multi-session orchestration" steps: write a manifest with multiple `[[sessions]]` entries including `depends_on`, and call `orchestrate_run` with `failure_policy: "continue"` and `merge_strategy: "TopologicalOrder"`
3. **Expected:** You produce a valid manifest and know all `orchestrate_run` parameters without ambiguity

### 3. Backend Status — status poll loop

1. Open `plugins/smelt-agent/skills/backend-status.md`
2. After dispatching a run with `orchestrate_run`, follow the "Poll until complete" steps using `orchestrate_status`
3. Interpret a response with `phase: "PartialFailure"`, one session in `Failed` state, and `mesh_status: null`
4. **Expected:** You correctly identify which session failed, understand that mesh mode was not used, and know what `PartialFailure` means without additional documentation

### 4. Backend Status — CapabilitySet degradation

1. Open `plugins/smelt-agent/skills/backend-status.md`
2. Find the "CapabilitySet awareness" section
3. Interpret what it means when `orchestrate_status` shows a `warn!` event about `supports_messaging: false`
4. **Expected:** You understand the run continues without peer messaging (no failure), and know not to expect inbox/outbox files

### 5. Peer Message — mesh mode coordination

1. Open `plugins/smelt-agent/skills/peer-message.md`
2. Follow the "Mesh mode — reading the roster" steps to extract peer names and outbox paths from the roster PromptLayer
3. Follow the "Sending a message to a peer" steps to write a JSON file to the outbox directory
4. **Expected:** You can write a message file with the correct path format (`.assay/orchestrator/<run_id>/mesh/<name>/outbox/<timestamp>.json`) and understand the routing delay

### 6. Peer Message — gossip mode knowledge manifest

1. Open `plugins/smelt-agent/skills/peer-message.md`
2. Follow the "Gossip mode — reading the knowledge manifest" steps to parse the `gossip-knowledge-manifest` PromptLayer and read `knowledge.json`
3. Interpret a knowledge manifest with 2 completed sessions
4. **Expected:** You understand the manifest path format, know the file is updated atomically after each session completes, and can read gate results from it

## Edge Cases

### Missing capability — messaging not supported

1. Open `plugins/smelt-agent/skills/peer-message.md`
2. Find the CapabilitySet guard section
3. Confirm you understand what to do if `supports_messaging` is false
4. **Expected:** Skip outbox writes; the orchestrator already warned; the run continues normally

### Custom backend in manifest

1. Open `plugins/smelt-agent/skills/run-dispatch.md`
2. Find the `StateBackendConfig` section describing the `Custom` variant
3. Write a manifest with `state_backend = { name = "linear", config = {} }` in the `[state_backend]` table
4. **Expected:** You understand this is a placeholder for M011+ backends and that `LocalFs` is the only currently-supported variant

## Failure Signals

- AGENTS.md references tools not in the MCP tools table — indicates documentation staleness
- Skills describe type fields that no longer exist in assay-types — verify with `grep -r "OrchestratorStatus" crates/`
- Step instructions in a skill are ambiguous or contradictory — human tester should flag for revision
- Tool names in skills don't match `grep -n "fn.*MCP_TOOL_NAME" crates/assay-mcp/src/server.rs` — indicates server.rs divergence

## Requirements Proved By This UAT

- R075 — Plugin exists with AGENTS.md and 3 skills covering run dispatch, backend status queries, and agent-to-agent messaging; a human can follow the instructions to operate Assay orchestration from a smelt worker context

## Not Proven By This UAT

- Actual runtime invocation of MCP tools from a smelt worker — requires a live smelt worker process and running Assay MCP server
- Cross-machine peer messaging with a non-LocalFs backend — deferred to M011+ concrete backend implementations
- CapabilitySet degradation at runtime for a real remote backend that lacks messaging or gossip support
- Gossip knowledge manifest atomicity under concurrent session completions — proven by S03 unit tests, not this UAT

## Notes for Tester

- Skills use concrete file path examples; verify the paths match the actual directory structure by checking `.assay/orchestrator/<run_id>/` in a real run
- The CapabilitySet sections are advisory — the degradation behavior itself is proven by S03 tests (`test_mesh_degrades_gracefully_without_messaging`, `test_gossip_degrades_gracefully_without_manifest`)
- `send_message`/`poll_inbox` do NOT exist as MCP tools; `peer-message.md` correctly describes file-based convention — do not expect MCP tool wrappers for these operations
- Plugin follows the flat `.md` file convention (not subdirectory `SKILL.md`) consistent with codex and opencode plugins
