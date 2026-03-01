# High-Value Feature Report — Final

**Explorer:** explorer-highvalue
**Challenger:** challenger-highvalue
**Date:** 2026-02-28
**Rounds of debate:** 3
**Status:** Converged

---

## North Star

> What is the shortest path to an agent reading a spec, doing work, hitting a gate, and getting a result — end to end?

Every feature and every milestone should be measured against how much closer it gets to this demo moment. Assay's identity is "agentic development kit" — the agentic capabilities must not be deferred to the end of the roadmap.

---

## MVP Scope (3-4 Milestones)

The minimum viable product is the end-to-end agent loop:

1. **Config & Persistence** — `assay init` creates `.assay/` directory with `config.toml`
2. **Spec Parsing** — Parse Markdown+TOML frontmatter spec files from `.assay/specs/`
3. **Minimal MCP Server** — stdio transport, 2-4 tools (`spec/get`, `gate/run`, `workflow/status`, `review/submit`)
4. **Command Gate** — Run a shell command, return structured pass/fail with evidence
5. **Claude Code Plugin** — Working plugin that connects to the MCP server via `.mcp.json`

**Demo:** A Claude Code agent reads a spec via MCP, implements against it, triggers `cargo test` as a command gate, and gets a pass/fail result — all programmatically through the Assay protocol.

---

## Feature Proposals (Pressure-Tested)

### 1. Project Configuration & Persistence Layer

**What:** File-based persistence giving Assay durable state.
- `assay init` creates `.assay/` directory
- `.assay/config.toml` for project settings (TOML, Rust ecosystem convention)
- `.assay/specs/` for spec files
- `.assay/state.json` for mutable workflow state (consider XDG data dir as alternative during implementation)
- `.assay/lock` file with PID + timestamp for concurrent access protection
- Git-friendly: config and specs are versionable; state may be `.gitignore`d
- Auto-generated JSON schemas from schemars to `.assay/schemas/`

**Priority:** First. Everything depends on file I/O.

**Scope:** ~1 milestone.

**Key decisions:**
- State storage location: project-local `.assay/state.json` vs `$XDG_DATA_HOME/assay/<project-hash>/`. Decide during implementation based on whether state should travel with the repo.
- Lock file format: `{ "pid": <int>, "started": "<iso8601>", "actor": "<cli|mcp-server>" }`. Stale lock detection via PID liveness check.

**Risks:**
- File format decisions are hard to change later.
- State files can diverge from reality after manual edits or git operations.

---

### 2. Spec-Driven Development Engine

**What:** Rich specification authoring and tracking replacing the bare `Spec { name, description }`.

**V1 scope (MVP):**
- Specs stored as `.assay/specs/<name>.md` with TOML frontmatter (`+++` delimited)
- Frontmatter: `name`, `status` (Draft/Active/Implementing/Review/Done), `tags`
- Structured acceptance criteria as a checklist in frontmatter: `[[criteria]]` array with `description` and `satisfied: bool`
- Markdown body: free-form prose (description, context, notes)
- CLI: `assay spec new`, `assay spec list`, `assay spec show <name>`, `assay spec check <name>`
- Validation: schema-validate frontmatter on parse, reject invalid TOML with clear error messages

**Deferred to v2:**
- Dependency graphs between specs (`depends_on` field) — informational-only fields rot without enforcement; add the field when enforcement logic is built
- Task decomposition within specs — overlaps with workflow phases and agent task management
- Markdown rendering or preview

**Implementation note:** Verify Rust crate support for TOML frontmatter (`+++` delimiters). If ecosystem support is poor, a thin custom parser (split on `+++`, parse middle as TOML) is trivial. Alternatively, evaluate YAML frontmatter (`---`) as a pragmatic fallback despite TOML being the Rust convention.

**Scope:** ~1-2 milestones.

**Risks:**
- Markdown+frontmatter parsing edge cases (nested delimiters, empty frontmatter).
- Spec format is a user-facing contract — changing it later requires migration.

---

### 3. Agent Protocol Layer (MCP Server)

**What:** Expose Assay as an MCP server so AI agents interact with specs, gates, and workflows programmatically.

**V1 scope (MVP):**
- stdio transport only (matches Claude Code's MCP model, simplest deployment)
- 2-4 tools maximum:
  - `assay/spec/get` — read current spec with acceptance criteria
  - `assay/gate/run` — trigger a gate evaluation, return structured result
  - `assay/workflow/status` — get current workflow phase and gate states
  - `assay/review/submit` — submit a review with criteria assessments
- Invoked via `assay mcp serve` subcommand on the CLI binary (dual-personality: human CLI + machine MCP server)
- Concurrent access: respects `.assay/lock` file; CLI read-only commands work while MCP is active, mutating CLI commands warn and require `--force`

**Architecture decision:** The `assay-cli` binary gains an `mcp serve` subcommand rather than creating a separate `assay-mcp` crate. This avoids workspace sprawl for what's essentially a different `main()` path. The MCP server logic itself lives in `assay-core` (or a new `assay-mcp` library crate if it gets large).

**Deferred to v2:**
- HTTP transport for standalone daemon mode
- Additional tools (spec/create, gate/define, workflow/advance)
- MCP resources and prompts (beyond tools)

**Scope:** ~1-2 milestones.

**Risks:**
- MCP in Rust is immature (`rmcp` crate is young). Verify it supports needed tool definitions before committing.
- State sync between CLI and MCP — mitigated by lock file in v1.
- Agents may not reliably use MCP tools — adoption depends on prompt engineering in plugins.

---

### 4. Programmable Gate Evaluation Framework

**What:** Executable gates with multiple evaluation strategies.

**V1 scope:**
- **Command gates:** Run a shell command, pass/fail on exit code. Capture stdout/stderr as evidence.
- **File gates:** Assert file existence or content matching a regex pattern.
- **Threshold gates:** Parse a numeric value from command output, compare against a threshold (e.g., coverage ≥ 80%).
- **Composite gates:** AND/OR combinators over other gates.
- Structured `GateResult`: status (Pass/Fail/Skip/Error), evidence (output, metrics), duration, timestamp.
- Gate definitions in `.assay/config.toml` under `[[gates]]` sections.
- CLI: `assay gate run <name>`, `assay gate list`, `assay gate status`.

**Design for extensibility:**
- Enum-based strategy pattern (functional, not trait-object OOP):
  ```rust
  enum GateKind {
      Command { cmd: String, args: Vec<String> },
      File { path: PathBuf, pattern: Option<Regex> },
      Threshold { source: Box<GateKind>, metric: String, min: f64 },
      Composite { op: CompositeOp, gates: Vec<GateKind> },
      // Future: Agent { prompt: String, model: String }
  }
  ```
- The evaluation function dispatches on the enum variant. Adding Agent gates later means adding one variant and one match arm.

**Deferred to v2:**
- Agent gates (requires MCP + LLM API integration, nondeterminism handling, cost tracking)
- Gate sandboxing/allowlisting for command execution security
- Parallel gate evaluation

**Scope:** ~1-2 milestones.

**Risks:**
- Shell command execution has injection risks. V1 mitigation: gates are defined in config files (not user input), but document the risk clearly.
- Developers will skip gates if they're slow — keep default gates fast (<5s).

---

### 5. Workflow State Machine with Audit Trail

**What:** A real state machine tying specs, gates, and reviews into an enforceable process.

**V1 scope:**
- Single hardcoded default workflow with 5 phases: Specify → Implement → Verify → Review → Done
- Transition guards: gates that must pass before advancing to next phase
- Event log: every state change recorded with timestamp, actor (human/agent), and outcome
- Gate failure behavior: stay in current phase, re-evaluate after fix. No rollback, no phase regression.
- CLI: `assay workflow status`, `assay workflow advance`
- Stored in `.assay/state.json` (or XDG location per decision in #1)

**Deferred to v2:**
- Custom workflow definitions / templates
- Rollback / phase regression (requires clear semantics for what "undo" means in a dev workflow)
- Parallel phases
- Workflow branching (conditional paths)

**Scope:** ~1 milestone.

**Risks:**
- The 5-phase model is opinionated. Some projects skip Verify, some don't Ship. Acceptable for v1 since it's a starting point, but must be configurable eventually.
- Concurrent gate evaluation edge cases (one passes while another fails mid-transition).

---

### 6. Structured Review System

**What:** Move beyond binary approval to structured criteria-based reviews.

**V1 scope:**
- `Review { spec_name, reviewer, criteria: Vec<CriterionResult>, comments: Vec<String>, status: ReviewStatus }`
- `CriterionResult { name: String, passed: bool, evidence: Option<String> }`
- `ReviewStatus`: Pending → Approved / ChangesRequested
- Single reviewer (human or agent, no distinction in the model)
- Criteria derived from spec's acceptance criteria (auto-populated from spec)
- CLI: `assay review start <spec>`, `assay review submit`

**Deferred to v2:**
- Weighted scoring and rubrics
- Multi-reviewer with role-based weights
- Review threading / discussion
- Review history and analytics

**Scope:** ~1 milestone.

**Risks:**
- Even simple structured criteria add friction over "approved: yes/no." Keep the criterion count low by default.
- Agent reviews are only as good as the prompts — false positives/negatives erode trust.

---

### 7. Claude Code Plugin (Hand-Built, Not SDK)

**What:** A working Claude Code plugin that integrates with Assay's MCP server — built by hand, not from an SDK.

**V1 scope:**
- `.mcp.json` entry pointing to `assay mcp serve` (stdio)
- Skills: `assay-spec` (read current spec), `assay-gate` (run gates), `assay-review` (submit review)
- Hooks: PostToolUse hooks that remind the agent to check spec criteria after writing code
- CLAUDE.md injection: project-level instructions referencing the Assay workflow

**Future:**
- Build a second plugin (Codex or OpenCode) by hand
- Extract common patterns into a Plugin SDK organically
- The SDK is an output of real usage, not a designed input

**Scope:** ~1 milestone (after MCP server exists).

**Risks:**
- Plugin models change rapidly — today's hooks.json format may be deprecated.
- Tight coupling to Claude Code's specific plugin architecture.

---

## Build Order

```
Milestone 1:  #1 Config & Persistence
Milestone 2:  #2 Spec Engine (minimal)
Milestone 3:  #3 MCP Server (minimal, stdio, 2-4 tools)
Milestone 4:  #4 Gate Framework (command + file + threshold)
              #7 Claude Code Plugin (after MCP exists)
Milestone 5:  #5 Workflow State Machine (single default workflow)
Milestone 6:  #6 Review System (structured criteria, single reviewer)
```

**MVP = Milestones 1-4:** Config + Specs + MCP + Gates + Plugin = the end-to-end agent demo.

Milestones 5-6 add process enforcement and quality assurance depth.

---

## Cross-Cutting Constraints (Every Milestone)

These are hard requirements, not nice-to-haves:

1. **Error types:** Every module defines its error enum using `thiserror`. Errors propagate with context. No `unwrap()` in library code.
2. **Tests:** Every public function has at least one test. Integration tests for CLI commands. Property tests where applicable (spec parsing, gate evaluation).
3. **Documentation:** Public API has doc comments. No doc comments on internals unless logic is non-obvious.
4. **`just ready` passes:** Format, lint, test, deny — every commit, every milestone.

---

## What We Debated and Resolved

| Topic | Explorer Position | Challenger Position | Resolution |
|-------|-------------------|---------------------|------------|
| Build order | Persistence-first, agent-last | Agent integration must come early | **Challenger won.** MCP elevated to milestone 3. |
| Spec format | Markdown+TOML frontmatter | Pure TOML | **Explorer won.** Specs are prose documents. Markdown is natural. Verify crate support for TOML frontmatter. |
| Spec dependencies | `depends_on` field (informational) | Drop it — informational fields rot | **Challenger won.** Add when enforcement exists. |
| Plugin SDK | Build SDK up front | Extract from real plugins | **Challenger won.** Build Claude Code plugin by hand first. |
| Review rubrics | Weighted multi-dimensional rubrics | Simple pass/fail criteria | **Challenger won for v1.** Structured criteria yes, weights/rubrics no. |
| Workflow rollback | Rollback to previous phases | Unclear semantics, defer | **Challenger won.** Gate fails → stay in phase. No rollback in v1. |
| Workflow templates | Multiple built-in templates | Single hardcoded workflow | **Challenger won for v1.** Templates when we know what users need. |
| MCP state sync | File lock with PID | CLI and MCP shouldn't coexist in v1 | **Middle ground.** Lock file with PID + timestamp. Read-only CLI works; mutating CLI warns. |
| Agent gates | The killer differentiator | Defer implementation, design the interface | **Agreed.** Ship command+file+threshold. GateEvaluator enum designed for future Agent variant. |
| TUI role | (Not addressed) | Should be addressed | **Agreed.** TUI is a visualization layer built after core primitives work. Phase 2 priority. |

---

## Open Questions for Implementation

1. **State storage location:** `.assay/state.json` (travels with repo) vs `$XDG_DATA_HOME/assay/` (user-local). Decision needed before milestone 1.
2. **TOML frontmatter crate support:** Verify `gray_matter` or alternatives support `+++`-delimited TOML. Fallback: thin custom parser or YAML frontmatter.
3. **MCP Rust crate:** Evaluate `rmcp` maturity. Does it support tool definitions, stdio transport, and the current MCP spec?
4. **`assay mcp serve` vs `assay-mcp` binary:** Subcommand on existing binary (recommended) vs separate crate. Decide before milestone 3.
