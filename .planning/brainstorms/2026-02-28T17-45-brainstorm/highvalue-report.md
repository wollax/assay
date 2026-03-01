# High-Value First Milestone — Final Report

**Explorer:** explorer-highvalue
**Challenger:** challenger-highvalue
**Date:** 2026-02-28
**Rounds of debate:** 3
**Status:** Converged

---

## The Question

The previous brainstorm spread features across 6 milestones. What combination of features creates a meaningful, usable, shippable FIRST milestone? Should we ship the north star ("agent reads spec, does work, hits gate, gets result") in v0.1, or build foundation and ship it in v0.2?

## Answer: Ship the North Star in v0.1

The first milestone must include the MCP server. Without it, Assay is a CLI task runner competing with `just`, `make`, `lefthook`, and dozens of others. With it, Assay is an agentic development kit — a product category with no established competitor.

This was debated in round 1. The challenger initially argued for CLI-only milestone 1 (foundation first, agents later). The explorer argued MCP belongs in milestone 1. The challenger conceded in round 2, citing four arguments:

1. **The previous brainstorm already settled this.** Three independent explorer/challenger pairs converged on "agents first, not last." MCP was elevated from position 6 to position 3.
2. **CLI-only Assay has no differentiator.** A tool that parses specs and runs commands is a shell script with opinions.
3. **MCP is a thin transport layer, not a new system.** The architecture separates domain logic (assay-core) from frontends (CLI, MCP). MCP is ~200-400 lines wrapping the same code the CLI uses.
4. **Two consumers of the same core in v0.1 reveals abstraction failures early.** Building for both humans (CLI) and agents (MCP) produces better domain types than building for one consumer alone.

---

## Milestone 1: The Thin Vertical Slice

### Pre-Gate: MCP Spike (Days 1-2)

Before committing to the full milestone, spike the MCP server:
- Hardcoded MCP server with `rmcp`, stdio transport, one fake tool
- Verify `rmcp` supports tool definitions, stdio, and current MCP spec
- **GO/NO-GO decision.** If NO-GO → fall back to CLI-only (gate-first standalone) and move MCP to milestone 2

### Core Scope (Weeks 1-4, if spike passes)

| Component | Scope | CLI Surface | MCP Surface |
|-----------|-------|-------------|-------------|
| Error types | Unified `AssayError` with thiserror, `#[non_exhaustive]` | — | — |
| Config | `assay init` creates `.assay/config.toml` + `.assay/specs/` | `assay init` | — |
| Specs | TOML files in `.assay/specs/`. Fields: name, description, criteria[] | `assay spec show <name>` | `spec/get` |
| Gates | Command gates only. Defined per-criterion in spec via optional `cmd`. Structured `GateResult` | `assay gate run <spec> [criterion]` | `gate/run` |
| MCP server | stdio transport, 2 tools | `assay mcp serve` | `spec/get`, `gate/run` |
| Plugin | `.mcp.json` + `CLAUDE.md` snippet | — | — |

### Demo Sentence

> A Claude Code agent reads a spec via MCP, implements against it, runs `gate/run` to check its criteria, and gets structured pass/fail results — all programmatically through the Assay protocol.

---

## Feature Details

### 1. Error Type Foundation

- Unified `AssayError` enum with `thiserror` derives
- `#[non_exhaustive]` for forward compatibility
- Structured validation errors: `Validation { field, message }`
- Built as part of week 1 domain model work, not a separate task

### 2. Project Configuration

- `assay init` creates `.assay/` directory with:
  - `config.toml` — project name, minimal settings
  - `specs/` — empty directory for spec files
- TOML format (Rust ecosystem convention)
- No lock file in v0.1 (no concurrent access scenario)
- No schema generation in v0.1
- No XDG consideration in v0.1 — project-local only

### 3. Spec Files

**Format:** TOML frontmatter files in `.assay/specs/<name>.md` using `+++` delimiters

```
+++
name = "add-auth-flow"
description = "Implement JWT authentication for the API"

[[criteria]]
description = "All tests pass"
cmd = "cargo test"

[[criteria]]
description = "No clippy warnings"
cmd = "cargo clippy -- -D warnings"
+++
```

**Key design decisions:**

**1. All v0.1 criteria are executable (command-only).** Debated in rounds 2-3. The challenger initially argued specs (WHAT) and gates (HOW) should be separate concepts. The explorer argued that separate config locations add indirection without adding capability in v0.1. Converged position: in v0.1, every criterion has a `cmd` field. Non-executable criteria (subjective requirements like "architecture follows hexagonal pattern") are not expressible in v0.1 specs. This is a deliberate limitation, not a design flaw — v0.1 captures only automatable requirements.

**2. Forward path to dual-track criteria.** In v0.2, add a `prompt` field for agent-evaluated criteria:
```toml
[[criteria]]
description = "Code is well-documented"
prompt = "Review the code and evaluate whether public functions have clear documentation"
```

The dual-track model becomes: `cmd` = deterministic track, `prompt` = agent-evaluated track. The enum-based design supports this extension cleanly by adding one variant and one match arm.

**3. `+++` delimiters for forward compatibility.** Using TOML frontmatter delimiters from day 1 means v0.2 can add Markdown body support by parsing everything after the closing `+++`. Existing v0.1 files are valid v0.2 files with empty bodies. Zero migration cost.

**Implementation note:** Verify if a Rust crate exists for `+++`-delimited TOML frontmatter parsing. If not, the parser is trivial: split on `+++`, parse middle as TOML, return rest as body string. Note as a build-vs-buy decision.

**What's NOT in v0.1 specs:**
- No Markdown body (empty body after closing `+++`)
- No non-executable / subjective criteria
- No status lifecycle (Draft/Active/Implementing/Review/Done)
- No dependency graphs between specs
- No task decomposition
- No spec versioning

### 4. Command Gate Evaluation

- `GateKind::Command { cmd: String }` — single variant in v0.1
- `GateResult { status: GateStatus, evidence: Evidence, duration: Duration }`
- `GateStatus`: Pass, Fail, Skip, Error
- `Evidence { stdout: String, stderr: String, exit_code: i32 }`
- Gate runs are stateless in v0.1 — no history, no audit trail
- `assay gate run <spec>` runs all executable criteria for a spec
- `assay gate run <spec> <criterion>` runs a single criterion (optional, for targeted checks)

**What's NOT in v0.1 gates:**
- No File gates
- No Threshold gates
- No Composite gates (AND/OR)
- No Agent gates
- No parallel gate evaluation
- No gate sandboxing or allowlisting

### 5. MCP Server

- stdio transport via `rmcp` crate
- Invoked via `assay mcp serve` subcommand on the CLI binary
- 2 tools:
  - `spec/get { name: string }` → returns spec content with all criteria
  - `gate/run { spec: string, criterion?: string }` → runs criteria, returns structured results
- Tool descriptions should be clear enough for agent discovery without additional prompting
- No HTTP transport, no resources, no prompts (MCP protocol features beyond tools)

**Architecture:** MCP server logic lives in `assay-core` (or a focused module within it). The `assay mcp serve` subcommand is a thin wrapper that starts the server using the same domain functions the CLI uses.

### 6. Claude Code Plugin

- `.mcp.json` entry pointing to `assay mcp serve` (stdio)
- `CLAUDE.md` snippet injected into projects:
  > This project uses Assay for spec-driven development. Before starting work, read the spec via `assay/spec/get`. After completing work, run gates via `assay/gate/run` to verify acceptance criteria.
- No skills, no hooks, no agents defined in the plugin
- Plugin structure uses existing `plugins/claude-code/` skeleton

---

## What's Explicitly NOT in Milestone 1

| Feature | Reason | When |
|---------|--------|------|
| `assay spec list` / `assay gate list` | `ls .assay/specs/` works. Convenience commands are polish. | v0.2 |
| Markdown spec bodies | TOML-only reduces parsing risk. Forward-compatible. | v0.2 |
| Spec lifecycle/status | Implies workflow engine that doesn't exist yet. | v0.2-v0.3 |
| Spec dependencies | Informational fields rot without enforcement logic. | v0.3+ |
| File/Threshold/Composite gates | Nobody has enough gates to compose yet. | v0.2 |
| Agent-evaluated gates | Requires LLM integration, cost tracking, nondeterminism handling. | v0.2-v0.3 |
| Lock file / concurrent access | Only needed when MCP and CLI run simultaneously. | v0.2 |
| Workflow state machine | Needs specs and gates to exist first. | v0.3 |
| Structured review system | Binary approval is fine for v0.1. | v0.3-v0.4 |
| TUI features | Visualization layer built after core primitives work. | v0.3+ |
| Plugin SDK | Build one plugin by hand. Extract patterns from real usage. | v0.4+ |
| Schema generation pipeline | Nice-to-have, not demo-critical. | v0.2 |
| Additional MCP tools | 2 tools is the minimum viable surface. | v0.2 |
| Codex/OpenCode plugins | One plugin first. Others after patterns emerge. | v0.3+ |

---

## Timeline

**3-4 weeks for a single developer.** If week 3 ends with tests passing and `just ready` green, you're done. If not, the 4th week exists for hardening.

| Week | Focus | Deliverables |
|------|-------|-------------|
| 0 (days 1-2) | MCP spike | GO/NO-GO on `rmcp` |
| 1 | Domain model + config | Error types, `Spec` struct, `GateKind`/`GateResult`, spec parsing, `assay init`, `assay spec show` |
| 2 | Gate evaluation + MCP | Command gate runner, `assay gate run`, MCP server with 2 tools |
| 3 | Integration + polish | CLI completion, Claude Code plugin, integration tests (CLI + MCP against same specs), README update |
| 4 (buffer) | Hardening | Edge cases, additional tests, demo preparation, `just ready` compliance |

### Quality Bar

- Every public function has at least one unit test
- Integration tests proving CLI and MCP share the same core behavior
- `just ready` passes on every commit (format, lint, test, deny)
- Error types with context propagation, no `unwrap()` in library code
- Public API has doc comments

**Stretch goals (nice-to-have, not blocking):**
- Property-based tests for spec parsing
- End-to-end demo script/recording
- Schema generation from schemars

---

## Fallback Plan

If the MCP spike fails (day 1-2 NO-GO):

**Milestone 1 becomes CLI-only:**
- `assay init`, `assay spec show`, `assay gate run`
- Same spec format, same gate evaluation, no MCP server
- Claude Code plugin deferred to milestone 2
- MCP server becomes milestone 2 priority

This is a viable but weaker milestone — it ships a useful CLI tool without the agentic differentiator. The demo becomes: "A developer creates a spec, defines criteria with automated checks, and runs `assay gate run` to verify all criteria pass." Functional, but not exciting.

---

## What We Debated and Resolved

| Topic | Explorer Position | Challenger Position | Resolution |
|-------|-------------------|---------------------|------------|
| MCP in milestone 1 | Yes — agentic identity requires it | No — build foundation first | **Explorer won.** MCP is the differentiator. CLI-only is a task runner. |
| Spec format | TOML-only for v0.1 | Markdown+TOML for forward compat | **Explorer won.** TOML-only. Forward-compatible via `+++` delimiter convention. |
| Criteria vs gates | Embed criteria in specs | Separate concepts, separate config | **Explorer won for v0.1.** All criteria are command-only (all have `cmd`). Non-executable criteria deferred to v0.2 agent gates. |
| Gate variants | Command only | Command only | **Agreed.** File/Threshold/Composite deferred. |
| Plugin scope | `.mcp.json` + CLAUDE.md snippet | `.mcp.json` only | **Middle ground.** CLAUDE.md snippet adds workflow guidance (~5 min work). |
| Timeline | 3 weeks | 4 weeks (with quality bar) | **Challenger won.** 4 weeks is honest. |
| MCP tools | 2 tools (spec/get, gate/run) | Maybe 1 tool? | **Explorer won.** `gate/run` IS the value prop. Without it, Assay is a file reader. |
| List commands | Include spec list, gate list | Cut them | **Challenger won.** `ls` works. |
| Workflow state machine | Deferred | Deferred | **Agreed.** Needs specs + gates first. |
| Reviews | Deferred | Deferred | **Agreed.** Binary approval for v0.1. |

---

## Open Questions for Implementation

1. **`rmcp` maturity:** The MCP spike (days 1-2) will answer this. If it fails, evaluate alternatives: TypeScript MCP wrapper as subprocess, or roll a minimal stdio JSON-RPC handler by hand.
2. **Spec file extension:** `.toml` for v0.1? Or `.assay.toml` / `.spec.toml` to distinguish from generic TOML files? Decide before implementation.
3. **`assay mcp serve` lifecycle:** Does it run until killed (daemon-like)? Or does it process one request and exit? stdio MCP servers typically run continuously — verify `rmcp` behavior.
4. **Gate timeout:** Should command gates have a default timeout? Hanging `cargo test` could block the MCP server. Reasonable default: 60 seconds, configurable per-criterion.
5. **Error reporting to agents:** When `spec/get` fails (spec not found), what MCP error structure does the agent see? Design error responses that agents can act on, not just human-readable messages.
