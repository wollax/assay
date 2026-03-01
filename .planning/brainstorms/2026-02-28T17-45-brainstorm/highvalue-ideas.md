# High-Value First Milestone — Feature Proposals

**Explorer:** explorer-highvalue
**Date:** 2026-02-28
**Question:** What combination of features creates a meaningful, usable, shippable first milestone?

---

## Context

The previous brainstorm spread features across 6 milestones:
1. Config & Persistence → 2. Spec Engine → 3. MCP Server → 4. Gate Framework + Plugin → 5. Workflow State Machine → 6. Review System

That's cautious and modular but it means **the north star demo** ("agent reads spec, does work, hits gate, gets result") doesn't happen until milestone 4. The question is: can we collapse this? Should we?

---

## Proposal 1: The Vertical Slice (Ship the North Star in v0.1)

**What:** Compress milestones 1-4 into a single first milestone by implementing minimal versions of each layer — just enough to make the end-to-end loop work.

Scope:
- **Config:** `assay init` creates `.assay/` with `config.toml`. Minimal schema: project name, spec dir path, gate definitions.
- **Specs:** Plain TOML files in `.assay/specs/` (NOT Markdown+frontmatter). Fields: `name`, `description`, `status`, `criteria[]`. Skip the rich Markdown body — that's a v0.2 enhancement.
- **Gates:** Command gates only. `GateKind::Command { cmd, args }`. `GateResult { status, stdout, stderr, duration }`. Defined in config.toml.
- **MCP Server:** stdio transport, 2 tools only: `assay/spec/get` and `assay/gate/run`. Invoked via `assay mcp serve`.
- **Claude Code Plugin:** `.mcp.json` pointing to `assay mcp serve`. One skill: "read spec and run gates."

**Why:** The north star demo IS the product pitch. Every day the demo doesn't work is a day Assay can't be explained to anyone. Shipping the full loop — even in crude form — validates the entire architecture end-to-end and creates a real user testing surface.

**Scope:** Large but achievable. ~3-4 weeks of focused work. Each layer is deliberately thin.

**Risks:**
- Breadth over depth — every layer is shallow, which means refactoring all of them in v0.2.
- TOML-only specs may feel like a regression if Markdown+frontmatter was promised.
- MCP Rust ecosystem (`rmcp`) may not be mature enough — could block the entire milestone.
- Integration bugs between 4+ layers compound. Testing the full loop requires everything to work.

---

## Proposal 2: Agent-First Minimal (Convention Over Config)

**What:** Eliminate the persistence/config layer entirely for v0.1. Use file system conventions instead of configuration. Focus all effort on making the agent loop work.

Scope:
- **No `assay init`.** Assay looks for `specs/` directory and `assay.toml` in project root. If they exist, it works. If not, it tells you what to create.
- **Specs:** TOML files in `specs/`. Minimal structure. No status tracking, no criteria satisfaction tracking. Just the spec content.
- **Gates:** Command gates only, defined inline in `assay.toml` (flat config, no `.assay/` directory).
- **MCP Server:** stdio, 2 tools: `spec/get` and `gate/run`.
- **Claude Code Plugin:** Full plugin with MCP integration + hooks that nudge the agent to check specs.
- **CLI:** `assay spec show <name>`, `assay gate run <name>`, `assay mcp serve`. That's it. No init, no list, no status.

**Why:** The fastest path to the demo. By using conventions instead of scaffolding, we skip the entire config/persistence layer and jump straight to the agent interaction. This is the Unix philosophy — do one thing well.

**Scope:** Medium. ~2-3 weeks. Fewer moving parts than Proposal 1.

**Risks:**
- No `assay init` means worse first-run experience for humans. But v0.1's primary user is the agent, not the human.
- Convention-based approach may paint us into a corner — harder to add `.assay/` directory later without breaking existing users.
- No state persistence means no workflow tracking, no gate history. Pure stateless operation.

---

## Proposal 3: Gate-First Standalone (Immediate Value, No Agents)

**What:** Ship gates as a standalone tool first. No specs, no MCP, no agents. Just `assay gate run` as a better test/lint/check runner with structured results.

Scope:
- **Config:** `assay.toml` with `[[gates]]` definitions (command, file, threshold, composite).
- **Gate Framework:** Full `GateKind` enum with all 4 deterministic variants. Rich `GateResult` with evidence, timing, status.
- **CLI:** `assay init`, `assay gate run [name]`, `assay gate run --all`, `assay gate list`, `assay gate status`.
- **Output:** Structured JSON output + human-readable terminal output. Machine-parseable for CI integration.
- **Quality:** Comprehensive tests, error handling, documentation.

**Why:** Gates are useful TODAY, without any of the other Assay infrastructure. Any developer can `assay init`, define gates in TOML, and run `assay gate run --all` instead of remembering which tests/linters/checks to run. This is a "better `just ready`" that produces structured, composable results. It creates immediate value and a user base before the agent story even starts.

**Scope:** Small-medium. ~2 weeks. Deep instead of wide.

**Risks:**
- Doesn't demonstrate the agentic value prop AT ALL. Assay looks like "yet another task runner."
- Risk of building a gold-plated gate framework that's over-engineered for v0.1 needs.
- Users who adopt gate-only Assay may not care about specs or agents — could attract the wrong audience.
- Previous brainstorm explicitly argued MCP should come early for differentiation. This contradicts that.

---

## Proposal 4: Two-Track Convergence

**What:** Split the milestone into two parallel tracks that converge at the end:

**Track A — Human-facing CLI (weeks 1-2):**
- `assay init` → config + persistence
- Spec parsing (TOML files in `.assay/specs/`)
- `assay spec list/show`
- Command gates + `assay gate run`

**Track B — Agent-facing MCP (weeks 2-3):**
- MCP server skeleton (stdio, tool definitions)
- `assay/spec/get` tool reading from same spec files
- `assay/gate/run` tool delegating to same gate framework
- Claude Code plugin wiring

**Convergence (week 3-4):**
- Integration testing: CLI and MCP share the same core
- Lock file for concurrent access
- End-to-end demo

**Why:** Parallel development is efficient and manages risk. If MCP proves difficult (immature Rust crate), Track A still ships as a useful CLI tool. If config/persistence proves slow, Track B can use conventions. The two tracks share `assay-core` — specs and gates are the same code, just different frontends.

**Scope:** Large. ~3-4 weeks. But parallelizable if there are multiple contributors.

**Risks:**
- Parallel tracks can diverge — core abstractions need to be stable before Track B starts.
- "Two-track" is a planning artifact — it only works if the core domain model is designed up front.
- Risk of shipping a milestone where neither track is deep enough to be compelling on its own.

---

## Proposal 5: The MCP Spike (Prove the Hard Thing First)

**What:** Start with the riskiest unknown — the MCP server — and build everything else around proving it works.

Scope:
- **Week 1:** MCP server spike. Can we build a working MCP stdio server in Rust using `rmcp`? Hardcode everything: one fake spec, one fake gate. Just prove the protocol works.
- **Week 2:** If the spike works, backfill with real config/spec/gate loading. If it doesn't, evaluate alternatives (TypeScript MCP wrapper, subprocess bridge).
- **Week 3:** Wire up the Claude Code plugin. End-to-end test with a real agent.
- **Week 4:** Polish CLI surface. `assay init`, `assay spec show`, `assay gate run`.

**Why:** The MCP server is the single highest-risk component in the entire Assay architecture. `rmcp` is young. The MCP spec is evolving. If this doesn't work, the entire north star is blocked. Proving it early — with throwaway spikes if needed — de-risks the whole project.

**Scope:** Medium-large. ~3-4 weeks. Front-loaded risk.

**Risks:**
- If the spike fails, we've spent a week with nothing to show.
- Spike code tends to become production code. Need discipline to throw away the spike and rebuild properly.
- Biases the milestone toward "make MCP work" at the expense of getting the domain model right.

---

## Proposal 6: Domain Model + Schema Contract (Foundation That Compounds)

**What:** Don't ship any user-facing features. Instead, ship the type system, error handling, schema generation, and a comprehensive test suite. Make the domain model unshakeable.

Scope:
- All 5 quick wins from the previous brainstorm (error types, schema pipeline, config loading, spec validation, gate dispatch).
- Property-based tests for spec parsing and gate evaluation.
- JSON Schema generation + validation.
- Integration test harness for CLI commands.
- Comprehensive documentation of the domain model.

**Why:** The previous brainstorm's "quick wins" ARE the foundation. Every subsequent feature builds on these types. Getting the domain model wrong means rewriting everything. The quick wins are already designed, debated, and scoped (~10 hours). Shipping them as a milestone means v0.2 starts on solid ground.

**Scope:** Small. ~1-2 weeks. Well-defined.

**Risks:**
- Nothing user-facing ships. No demo, no feedback loop. Pure investment with no return until v0.2.
- "Perfect domain model" is a trap — you can't know if the types are right until they're used.
- Team energy may stall without visible progress.

---

## Proposal 7: Spec-to-Gate Minimum Path (Skip MCP, Ship Value)

**What:** The shortest path that demonstrates Assay's core value without the MCP complexity. Ship a CLI tool where you write a spec, define gates tied to that spec's criteria, and run them.

Scope:
- **Config:** `assay init` creates `assay.toml` and `specs/` directory.
- **Specs:** Markdown + TOML frontmatter (the full format, not simplified). Each spec has `[[criteria]]` with `kind = "command"` and `cmd = "..."`.
- **Gates are criteria:** No separate gate definitions. Each spec criterion IS a gate. `assay check <spec-name>` runs all criteria for that spec and reports structured results.
- **CLI:** `assay init`, `assay spec new <name>`, `assay spec show <name>`, `assay check <name>`, `assay check --all`.
- **Output:** Rich terminal output showing each criterion pass/fail with evidence.

**Why:** This unifies specs and gates into a single concept — the spec IS the gate definition. This is simpler, more intuitive, and matches how developers think: "does this code meet the spec?" The answer is running the spec's criteria. No separate gate configuration. No MCP complexity. Just specs with executable criteria.

**Scope:** Medium. ~2-3 weeks. Focused and deep.

**Risks:**
- Merging specs and gates may be a design dead end. The previous brainstorm deliberately separated them for composability.
- No agent story. Humans can use it; agents can't (no MCP).
- Markdown+TOML frontmatter parsing may have unexpected edge cases.
- "Criteria as gates" may not scale when gates need to be shared across specs or composed independently.

---

## Recommendation

My top pick is **Proposal 1 (Vertical Slice)** with elements of **Proposal 5 (MCP Spike)** for risk management.

Rationale:
1. The north star demo is the product pitch. Ship it first.
2. Each layer being thin is a feature, not a bug — it prevents over-engineering and creates real usage feedback.
3. Start week 1 with an MCP spike to de-risk the hardest part. If `rmcp` works, proceed with Proposal 1. If it doesn't, fall back to Proposal 7 (spec-to-gate without MCP) and move MCP to v0.2.
4. TOML-only specs for v0.1 is fine — Markdown+frontmatter is a UX enhancement, not a capability enabler.

The fallback plan (Proposal 7 if MCP fails) means the milestone always ships something meaningful.
