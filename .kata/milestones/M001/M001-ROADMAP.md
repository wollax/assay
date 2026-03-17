# M001: Single-Agent Harness End-to-End

**Vision:** Transform Assay from a gate evaluation toolkit into a full agent orchestration primitive. A user writes a TOML manifest declaring what spec to implement, and Assay handles the entire lifecycle: worktree creation → agent launch with auto-generated harness config → gate evaluation → merge proposal.

## Success Criteria

- A TOML manifest can drive the full single-agent pipeline end-to-end
- Claude Code is launched in an isolated worktree with generated CLAUDE.md, .mcp.json, settings, and hooks
- Pipeline failures at any stage produce structured errors with recovery guidance
- Worktrees are linked to sessions with orphan detection and collision prevention
- GateEvalContext (renamed from AgentSession) persists to disk, surviving MCP restarts

## Key Risks / Unknowns

- Claude Code `--print` mode output format stability — may have changed since research
- Hook contract interaction with Claude Code's hooks.json — needs runtime verification
- Process lifecycle edge cases (timeout, zombie processes, orphaned agents)

## Proof Strategy

- Claude Code `--print` compatibility → retire in S04 by generating valid harness config and verifying claude accepts it
- Process lifecycle → retire in S07 by exercising timeout and failure paths in the E2E pipeline

## Verification Classes

- Contract verification: `just ready` (fmt, lint, test, deny), schema snapshot tests, round-trip parsing tests
- Integration verification: real worktree creation + real claude --print invocation against a test spec
- Operational verification: pipeline handles agent timeout, crash, and exit-code failures
- UAT / human verification: run the full pipeline on a real spec and inspect the generated PR/merge proposal

## Milestone Definition of Done

This milestone is complete only when all are true:

- All 7 slices are complete with passing verification
- The E2E pipeline is exercised with a real manifest, real worktree, and real agent invocation
- Generated harness config (CLAUDE.md, .mcp.json, hooks.json) is valid
- Pipeline errors are structured with stage context at every failure point
- `just ready` passes on main after all slices are squash-merged

## Requirement Coverage

- Covers: R001–R019
- Partially covers: none
- Leaves for later: R020–R026
- Orphan risks: none

## Slices

- [x] **S01: Prerequisites — Persistence & Rename** `risk:medium` `depends:[]`
  > After this: GateEvalContext persists to disk (verified by restart test), and all "AgentSession" references are renamed. `just ready` passes.

- [x] **S02: Harness Crate & Profile Type** `risk:medium` `depends:[S01]`
  > After this: `assay-harness` crate exists in workspace, `HarnessProfile` type compiles in assay-types with schema snapshot. `just ready` passes.

- [ ] **S03: Prompt Builder, Settings Merger & Hook Contracts** `risk:medium` `depends:[S02]`
  > After this: prompt builder assembles layered prompts from spec + project context, settings merger combines base + overrides, hook contracts defined in types. Verified by unit tests.

- [ ] **S04: Claude Code Adapter** `risk:high` `depends:[S03]`
  > After this: adapter generates valid CLAUDE.md, .mcp.json, settings overrides, and hooks.json from a HarnessProfile. Verified by snapshot tests and file content assertions.

- [ ] **S05: Worktree Enhancements & Tech Debt** `risk:low` `depends:[S01]`
  > After this: worktrees have session linkage, orphan detection, collision prevention, and 15 tech debt issues are resolved. `just ready` passes.

- [ ] **S06: RunManifest Type & Parsing** `risk:low` `depends:[S02]`
  > After this: TOML manifests with `[[sessions]]` parse into RunManifest types with validation and actionable error messages. Verified by round-trip and error-case tests.

- [ ] **S07: End-to-End Pipeline** `risk:high` `depends:[S04,S05,S06]`
  > After this: `assay run <manifest.toml>` and `run_manifest` MCP tool execute the full pipeline: manifest → worktree → harness config → agent launch → gate evaluate → merge propose. Pipeline failures produce structured errors with stage context.

## Boundary Map

### S01 → S02
Produces:
  assay-types/src/session.rs → `GateEvalContext` type (renamed from AgentSession)
  assay-core/src/gate/session.rs → persistence functions: `save_context()`, `load_context()`, `list_contexts()`
  Clean compilation with no "AgentSession" references remaining

Consumes: nothing (first slice)

### S01 → S05
Produces:
  Same as S01 → S02 (S05 needs clean codebase foundation)

Consumes: nothing (first slice)

### S02 → S03
Produces:
  assay-types/src/harness.rs → `HarnessProfile` type (prompt_layers, settings, hooks)
  crates/assay-harness/Cargo.toml → crate exists, depends on assay-core + assay-types
  crates/assay-harness/src/lib.rs → crate root with module structure

Consumes from S01:
  Clean compilation foundation, `GateEvalContext` type

### S02 → S04
Produces:
  Same as S02 → S03 (S04 uses HarnessProfile as input)

### S02 → S06
Produces:
  assay-types/src/harness.rs → `HarnessProfile` type (RunManifest references it)

### S03 → S04
Produces:
  assay-harness/src/prompt.rs → `build_prompt(layers: &[PromptLayer]) -> String`
  assay-harness/src/settings.rs → `merge_settings(base: &Settings, overrides: &Settings) -> Settings`
  assay-types/src/harness.rs → `HookContract`, `HookEvent`, `PromptLayer`, `SettingsOverride` types

Consumes from S02:
  assay-harness crate structure, `HarnessProfile` type

### S04 → S07
Produces:
  assay-harness/src/claude.rs → `generate_config(profile: &HarnessProfile, worktree: &Path) -> Result<ClaudeConfig>`
  ClaudeConfig includes: claude_md content, mcp_json content, settings overrides, hooks.json content
  assay-harness/src/claude.rs → `write_config(config: &ClaudeConfig, worktree: &Path) -> Result<()>`

Consumes from S03:
  prompt builder, settings merger, hook contract types

### S05 → S07
Produces:
  assay-core/src/worktree.rs → `session_id` field on WorktreeMetadata
  assay-core/src/worktree.rs → `detect_orphans(project_root: &Path) -> Result<Vec<WorktreeMetadata>>`
  assay-core/src/worktree.rs → collision check in `create()` — rejects if spec has active worktree
  15 tech debt issues resolved (cleaner error types, missing tests, etc.)

Consumes from S01:
  Clean compilation foundation

### S06 → S07
Produces:
  assay-types/src/manifest.rs → `RunManifest` type with `sessions: Vec<SessionEntry>`
  assay-core/src/manifest.rs → `parse(toml_str: &str) -> Result<RunManifest>`, `load(path: &Path) -> Result<RunManifest>`
  Validation functions with actionable error messages

Consumes from S02:
  `HarnessProfile` type (SessionEntry references harness config)

### S07 (capstone)

Consumes from S04:
  Claude Code adapter: `generate_config()`, `write_config()`
Consumes from S05:
  Worktree: `create()` with collision prevention, session linkage
Consumes from S06:
  Manifest: `load()`, `parse()` with validation
