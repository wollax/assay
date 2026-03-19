# Audit: Dogfooding the assay project

**Date:** March 19, 2026
**Goal:** Dogfooding the assay project
**Codebase:** assay / `/Users/wollax/Git/personal/assay`

---

## Strengths

### Self-check spec exists and is structurally correct
`.assay/specs/self-check.toml` covers all four deterministic quality gates (`cargo fmt --check`, `cargo clippy`, `cargo test --workspace`, `cargo deny check`) plus one advisory `AgentReport` criterion for architecture review. The spec is well-formed and uses enforcement levels correctly.

### Claude Code plugin is architecturally complete
`plugins/claude-code/` ships an MCP config (wires `assay mcp serve` as a stdio server), two skills (`gate-check`, `spec-show`), PostToolUse reminders, a PreCompact checkpoint hook, and a Stop hook that blocks agent completion when gates fail. The Stop hook has five graceful-degradation guards (jq missing, loop prevention, `ASSAY_STOP_HOOK_MODE=off`, no `.assay/` dir, binary not found). CLAUDE.md provides a concise spec-first workflow description.

### Gate run history is persisted
`.assay/results/self-check/` contains five `GateRunRecord` JSON files from early development (March 6–10). The atomic-write, file-per-record history pattern works and the results are machine-readable.

### Checkpoint hook fires at real lifecycle events
The `hooks.json` wires the checkpoint hook on `PostToolUse(Task|TaskCreate|TaskUpdate)`, `PreCompact`, and `Stop`. The `.assay/checkpoints/` directory has real checkpoint data with context health metrics, confirming the hook fired during M001 development.

### Extensive architectural decisions documented
DECISIONS.md tracks 61 decisions with rationale and revisability flags — an unusually strong signal for a project this young. Combined with the Kata planning artifacts (36 validated requirements, 4 complete milestones, 1271 tests), there's a solid foundation for consistent future development.

### MCP server surface is rich and well-tested
`assay-mcp` exposes 18–22 tools across spec, gate, worktree, session, merge, context, and orchestration domains. 1,271 unit and integration tests pass. The server module is thoroughly documented (all tools, parameter types, error contracts).

---

## Gaps

### 1. `assay` binary is not installed — zero live enforcement
`which assay` returns nothing. Every hook script that invokes `assay` (Stop gate check, checkpoint save) has a guard that exits 0 when the binary is missing. This means **no gate enforcement has occurred during M002–M004 development** — the MCP server has never served a real request, and the Stop hook has been silently no-oping. The entire dogfooding loop depends on this binary being in PATH.

### 2. Self-check spec has not been run since v0.2.0
The five `GateRunRecord` files in `.assay/results/self-check/` all carry `assay_version: "0.2.0"` and the latest timestamp is March 10. The codebase is now v0.4.0 (shipped March 15), having added `gate_evaluate`, `WorkSession`, `spec_validate`, three coordination modes, and 400+ new tests. The self-check spec has never been run against v0.3.0 or v0.4.0. The `just ready` check exists but runs the test suite directly — it does not go through Assay's own gate machinery.

### 3. Only one spec for a 6-crate, 20K-line system
`.assay/specs/` contains exactly one file: `self-check.toml`. The `FeatureSpec` type (IEEE 830-style with requirements, constraints, risks, and acceptance criteria), the directory-spec format (`[srs]` indicator), and the rich `gate_evaluate` / `spec_validate` MCP tools are all unused on the project that built them. No spec exists for worktree lifecycle, session management, gate evaluation, orchestration, harness adapters, or context pruning. Assay is not using its own primary value proposition (spec-driven development) to govern itself.

### 4. Advisory `code-quality-review` gate has never executed
All five self-check run records show `skipped: 1` — the `AgentReport` criterion has never run. This gate would invoke `gate_evaluate` (the headless Claude subprocess evaluator), which is the most sophisticated gate type Assay ships. Never exercising it on the project itself means the dogfooding loop for the most important feature is completely absent.

### 5. Plugin CLAUDE.md documents only 3 of 18+ MCP tools
`plugins/claude-code/CLAUDE.md` lists `spec_list`, `spec_get`, and `gate_run`. The actual server exposes `gate_evaluate`, `gate_report`, `gate_finalize`, `gate_history`, `spec_validate`, `session_create/get/update/list`, `worktree_create/list/status/cleanup`, `merge_check`, `context_diagnose`, `estimate_tokens`, `orchestrate_run`, `orchestrate_status`. An agent developing Assay using the plugin has no workflow documentation for 15 tools, defeating the plugin's purpose.

### 6. Only 2 skills cover a much larger workflow surface
`gate-check` and `spec-show` are the only skills. No skills exist for: `worktree create/cleanup`, `session lifecycle`, `gate evaluate` (headless evaluation), `harness generate`, `context diagnose`, or `orchestrate run`. Each of these has a non-trivial workflow (parameter selection, expected outputs, error recovery) that would benefit from a skill.

### 7. Kata slice completion and Assay gate runs are not connected
Development uses Kata's file-based planning loop (`.kata/`) for structure and `just ready` for quality. Assay's gate machinery is never invoked as part of the Kata slice-complete → verify → squash flow. The two systems run in parallel but don't reinforce each other. A slice could complete with `just ready` passing but with Assay gate history completely stale.

### 8. `assay_version` in GateRunRecord appears hardcoded at "0.2.0"
All five result files show `"assay_version": "0.2.0"` including the March 10 run, which was post-v0.3.0 release. Either the field is not dynamically populated from the binary's cargo version, or it was set during a v0.2.0 build and not updated. This makes the audit trail misleading.

### 9. TUI is a 42-line placeholder
`crates/assay-tui/src/main.rs` renders "Assay TUI" centered on screen with no navigation, no spec display, no gate results, no session view. The `ide/README.md` says "Technology and framework TBD." Neither surface is close to useful, yet both are shipped as named crates. The TUI has no tests, no meaningful integration with `assay-core`, and adds crossterm/ratatui dependencies pulling in duplicate crates flagged by `cargo-deny`.

### 10. Codex and OpenCode plugins are near-empty stubs
`plugins/codex/` has `AGENTS.md` (6 lines), `README.md` (7 lines), and an empty `skills/` directory. `plugins/opencode/` has placeholder files and no actual content. Both README files describe installation steps but link to non-existent skill files or MCP configs. An agent on Codex or OpenCode gets no functional Assay integration despite these plugins being advertised in the README.

---

## Next Steps

**P0 — Restore the dogfooding loop (prerequisite for everything else)**

1. **Install the binary**: Run `cargo install --path crates/assay-cli` and add to PATH so hooks, MCP server, and CLI actually execute during development sessions. Document this in `CONTRIBUTING.md` as a required setup step alongside `mise install`.

2. **Run self-check on v0.4.0**: Execute `assay gate run self-check` and commit the resulting record to `.assay/results/`. This verifies the gate machinery works end-to-end on the current codebase.

**P1 — Run the advisory gate for the first time**

3. **Trigger `code-quality-review` via `gate_evaluate`**: Ensure `claude` CLI is available and run `assay gate run self-check` with the `AgentReport` criterion active. Review the structured output. Fix any findings before M005 planning. This is the primary dogfooding payoff.

**P2 — Expand the spec surface to match the product**

4. **Add `just schemas-check` to self-check spec**: The `schemas/` directory drifts from `assay-types` as types evolve. Add a `schemas-check` criterion to the self-check spec so schema drift is caught automatically.

5. **Write feature specs for core subsystems**: Create at minimum:
   - `.assay/specs/gate-evaluation.toml` — covers `gate_run`, `gate_evaluate`, `gate_history`, enforcement levels, criterion kinds
   - `.assay/specs/session-management.toml` — covers `session_create/update/list`, phase transitions, stale recovery
   - `.assay/specs/worktree-lifecycle.toml` — covers `worktree_create/status/cleanup`, ahead/behind tracking
   Use the directory spec format (`spec.toml` + individual criterion files) to exercise Assay's own SRS capability.

**P3 — Fix the plugin for agents developing Assay**

6. **Update `plugins/claude-code/CLAUDE.md`**: Add the full MCP tool table (all 18+ tools), organized by domain (spec, gate, session, worktree, context, orchestrate). Include brief usage notes for `gate_evaluate` and `orchestrate_run` — the non-obvious tools.

7. **Add 4–6 new skills**: `worktree-create`, `session-lifecycle`, `gate-evaluate` (headless evaluation workflow), `harness-generate`, `context-diagnose`. Each should mirror the pattern of `gate-check`: clear steps, structured output format, error handling guidance.

**P4 — Integrate gate runs into the Kata slice lifecycle**

8. **Add `assay gate run self-check` to slice-complete checklist**: Update `.kata/milestones/M005/M005-CONTEXT.md` (when written) to include running the self-check gate as a verification step before marking any slice done. This closes the gap between Kata's planning loop and Assay's enforcement loop.

**P5 — Fix structural issues**

9. **Fix `assay_version` in `GateRunRecord`**: Trace the version field population in `assay-core/src/evaluator.rs` and `assay-core/src/gate/mod.rs`. It should read from `env!("CARGO_PKG_VERSION")` not a hardcoded string.

10. **Stub or remove `assay-tui`**: Either commit to a minimal useful TUI (spec list + gate status dashboard using ratatui) in M005, or mark it `#[doc(hidden)]` and exclude it from `just build` defaults until it has real content. The current state sets false expectations.

---

*Generated by /audit — read-only recce, no code was modified.*
