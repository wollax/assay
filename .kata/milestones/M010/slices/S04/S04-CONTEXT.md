---
id: S04
milestone: M010
status: ready
---

# S04: smelt-agent plugin — Context

## Goal

Author `plugins/smelt-agent/AGENTS.md` and three skills (`run-dispatch`, `backend-status`, `peer-message`) that document how a smelt worker interacts with the backend-aware Assay API surface.

## Why this Slice

S02 wired all orchestrator state writes through `StateBackend`. S03 added capability degradation paths. The API surface is now stable enough to document. The smelt-agent plugin is the first consumer-facing artifact that describes how an external system (smelt) interacts with the backend — it validates that the API surface is understandable and complete before M011 builds concrete remote backends.

Smelt exists as a real codebase but does not yet have deep Assay integration. This plugin is forward-looking documentation that describes how a smelt worker *would* interact with the backend-aware API surface, grounding future integration work.

## Scope

### In Scope

- `plugins/smelt-agent/AGENTS.md` — system prompt describing smelt worker interaction with Assay's backend API
- `plugins/smelt-agent/skills/run-dispatch.md` — how to read a RunManifest, configure a backend, and dispatch a run
- `plugins/smelt-agent/skills/backend-status.md` — how to query `read_run_state`, interpret `OrchestratorStatus`, and report back
- `plugins/smelt-agent/skills/peer-message.md` — how to use `send_message` / `poll_inbox` for agent-to-agent coordination across machines
- All three API surfaces documented: MCP tools, CLI commands, and Rust API — smelt will hit Rust API surfaces directly; MCP and CLI are also documented to leave options open
- Harness-agnostic: the plugin is not tied to any specific agent harness (Claude Code, Codex, etc.)
- Primary consumer is an AI agent running on a smelt worker machine

### Out of Scope

- Message envelope format or protocol spec for peer messaging — document the raw `send_message`/`poll_inbox` mechanics only; protocol design is a future concern (M011+)
- Concrete remote backend implementations (LinearBackend, GitHubBackend, SshSyncBackend) — M011+
- Hooks, scripts, or executable automation (unlike claude-code plugin which has shell scripts) — this plugin is pure documentation/skills
- Multi-machine integration testing — UAT only in this milestone
- New MCP tools or Rust API changes — S04 documents what exists, does not extend it

## Constraints

- Must accurately reflect the `StateBackend` trait API as implemented in S01/S02 (7 sync methods, `CapabilitySet` flags)
- Must reference real type names, method signatures, and MCP tool names — no speculative API
- Skill format should follow the existing codex/opencode flat-file convention (D082, D119) — flat `.md` files in `plugins/smelt-agent/skills/`, not subdirectory SKILL.md
- AGENTS.md format should follow the existing `plugins/codex/AGENTS.md` as a structural model
- `peer-message` skill documents raw API only — no message protocol, no envelope format, no ack patterns

## Integration Points

### Consumes

- `crates/assay-core/src/state_backend.rs` — `StateBackend` trait method signatures, `CapabilitySet` struct, `LocalFsBackend` implementation details
- `crates/assay-types/src/orchestrate.rs` — `OrchestratorStatus`, `SessionStatus`, `MeshStatus`, `GossipStatus` schemas (documented in skills)
- `crates/assay-types/src/state_backend.rs` — `StateBackendConfig` enum (referenced in run-dispatch skill)
- `crates/assay-types/src/manifest.rs` — `RunManifest` and `state_backend` field (referenced in run-dispatch skill)
- `plugins/codex/AGENTS.md` — structural model for AGENTS.md format
- `plugins/codex/skills/*.md` — structural model for skill file format

### Produces

- `plugins/smelt-agent/AGENTS.md` — system prompt for smelt worker agents
- `plugins/smelt-agent/skills/run-dispatch.md` — run dispatch skill
- `plugins/smelt-agent/skills/backend-status.md` — backend status query skill
- `plugins/smelt-agent/skills/peer-message.md` — peer messaging skill

## Open Questions

- None — all behavioural decisions resolved during discuss.
