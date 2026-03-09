# Research Summary: Assay v0.3.0 "Orchestration Foundation"

**Date:** 2026-03-08
**Synthesized from:** STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md
**Milestone scope:** Git worktree lifecycle, Claude Code headless launching, session tracking, independent gate evaluation with diff context, TUI gate results viewer

---

## Executive Summary

v0.3.0 is an unusually clean milestone from a dependency and risk standpoint. All five features -- worktree management, agent launching, session tracking, diff assembly, and TUI viewer -- build on existing patterns with zero new workspace dependencies. The subprocess execution pattern in `gate/mod.rs::evaluate_command()` is the template for everything: worktrees shell out to `git`, the launcher shells out to `claude`, diffs shell out to `git diff`, and sessions reuse the JSON-file-per-record pattern from `assay-core::history`. The stack research closed every door on git2, gix, similar, diffy, SQLite, and tokio::process -- all rejected with clear rationale. The roadmapper should treat "zero new deps" as a hard constraint, not a goal.

The architecture research reveals two structural prerequisites that must happen before feature work. First, `assay-tui` currently has no dependency on `assay-core` -- it is a 42-line skeleton that cannot load any data. Adding this dependency is trivial but blocks the entire TUI viewer. Second, `assay-cli/src/main.rs` is a 76K monolith. Adding worktree/session/launch subcommands without extraction will push it past maintainability limits. The CLI refactor is not optional -- it is the difference between a codebase that scales and one that doesn't.

The key differentiator for v0.3.0 is `gate_evaluate` with diff context: a single MCP tool invocation that computes the diff, launches a headless evaluator, collects structured pass/fail results, and finalizes the gate run. No MCP tool in the ecosystem does this. Everything else (worktrees, sessions, TUI) is infrastructure that makes `gate_evaluate` possible. The roadmap should be organized around delivering `gate_evaluate` as the capstone, with everything else as enabling work.

---

## Key Findings

### From STACK.md: Zero Dependencies, Maximum Reuse

- **Zero new workspace dependencies.** No changes to root `Cargo.toml` `[workspace.dependencies]`, no changes to any crate's `Cargo.toml`, no new entries in `deny.toml`. This is not aspirational -- it is verified against every proposed feature.
- **git2 is categorically rejected.** C build dependency (libgit2-sys), 80+ transitive deps, `!Send + !Sync` worktree handles that break `spawn_blocking`, and an incomplete worktree API (`prune` != `remove`). The git CLI is more correct, more complete, and adds nothing.
- **gix is categorically rejected.** 150+ transitive deps, no high-level worktree CRUD API, pre-1.0 instability. Would need `gix-command` to shell out to git anyway.
- **`std::process::Command` is the only subprocess pattern.** The codebase has one proven subprocess model (process groups, reader threads, `try_wait` polling, `libc::killpg` timeout enforcement). Introducing `tokio::process::Command` would create a second divergent pattern with no benefit.
- **Claude Code `--print` mode with `--output-format json` is the interface.** Returns `session_id` for `--resume`, structured results for parsing. No tmux, no daemon, no persistent process. Stream-json is a v0.4 concern.
- **External tool dependencies:** `git` (2.20+ for `--porcelain`) and `claude` CLI. Both detected at execution time, not startup. Features that don't need them should work without them.

### From FEATURES.md: agtx is the Reference, `gate_evaluate` is the Differentiator

- **Worktree lifecycle:** agtx provides the definitive reference. Worktrees under `.assay/worktrees/`, branch per spec (`assay/<spec-slug>`), `--force` removal with `prune` fallback. Assay should adopt this with minimal deviation.
- **Headless evaluation:** `--print` mode is sufficient for independent gate evaluation. No tmux orchestration needed. The `--json-schema` flag can enforce structured evaluation responses. Session continuation via `--resume` enables "explain this failure" follow-ups but is not required for v0.3.0.
- **Session tracking:** JSON files under `.assay/sessions/`, not SQLite. agtx uses SQLite because it manages multiple projects from a global dashboard -- a different scale. Assay is single-project, tens-to-hundreds of sessions.
- **`gate_evaluate` is genuinely novel.** No MCP tool does single-invocation AI code review against spec criteria with diff context. This is the v0.3.0 ship-or-slip feature.
- **TUI scope must be minimal.** agtx's TUI is a full kanban board -- massively over-scoped. Assay needs a single table with a detail pane, 4 keybindings, and nothing more. ~200-300 lines.
- **Anti-features confirmed:** Full kanban TUI, SQLite, tmux orchestration, multi-agent registry, plugin framework, PR lifecycle management, cyclic workflows. All out of scope.

### From ARCHITECTURE.md: Five New Modules, Two Structural Prerequisites

**New modules (all in assay-core):**

| Module | Types (assay-types) | Core API | Storage |
|--------|--------------------| ---------|---------|
| `worktree` | `WorktreeState`, `WorktreeRecord`, `WorktreeConfig` | `create`, `list`, `update_state`, `remove`, `prune` | `.assay/worktrees/<id>.json` |
| `launcher` | `LaunchConfig`, `AgentProcessStatus`, `LaunchRecord` | `launch`, `check_status`, `kill`, `build_prompt` | stdout/stderr to `.assay/sessions/<id>/` |
| `session` | `WorkSession`, `SessionPhase`, `SessionOutcome` | `create`, `update_phase`, `complete`, `load`, `list` | `.assay/sessions/<id>.json` |
| `diff` | `DiffSummary`, `FileDiff`, `DiffStatus` | `summary`, `file_diffs`, `assemble_gate_context` | N/A (computed) |
| TUI screens | N/A | `Screen` trait, `GateResults` screen | N/A |

**Structural prerequisite 1: TUI needs assay-core dependency.** Currently only depends on ratatui/crossterm/color-eyre. Cannot load any gate data without this change.

**Structural prerequisite 2: CLI monolith extraction.** 76K single file. Adding 3+ new subcommand groups (worktree, session, launch) without extraction creates an unmaintainable blob. Extract into `commands/` modules early.

**Naming decision resolved:** The third "session" concept is `WorkSession` -- distinguishes from `AgentSession` (gate evaluation) and context module's `SessionInfo` (Claude Code diagnostics). Module paths disambiguate at the code level.

**Build order:** types -> core modules (parallelizable) -> TUI -> CLI -> MCP. The four core modules are independent of each other and can be built in any order.

**New error variants:** `GitOperation`, `WorktreeNotFound`, `WorktreeInvalidTransition`, `LaunchFailed`, `AgentTimeout`. Recommendation from PITFALLS.md: use sub-enums (`WorktreeError`, `AgentError`) embedded as single variants in `AssayError` to prevent the enum from growing to 40+ variants.

### From PITFALLS.md: 25 Pitfalls, 10 Critical

**Critical pitfalls by phase:**

| ID | Pitfall | Why Critical | Prevention (short) |
|----|---------|-------------|-------------------|
| P-41 | Orphaned worktrees after crash | Two-phase state (filesystem + git refs) requires two-phase cleanup | `git worktree remove --force` first, then verify; startup reconciliation via `git worktree prune` |
| P-42 | Path confusion (repo root vs worktree vs .assay/) | Three relevant directories, existing code uses raw `Path` args | Introduce `WorkContext` struct bundling all three paths; never use `current_dir()` |
| P-45 | Zombie agent subprocesses | Long-lived agents spawn grandchildren that escape process groups | Kill entire process tree, not just direct child; track PIDs in session manifest |
| P-47 | Signal forwarding to agents | SIGINT kills Assay but orphans agents, or vice versa | Two-stage: first SIGINT -> SIGTERM to agents; second SIGINT -> SIGKILL all |
| P-49 | Stale session state blocks new sessions | Crash leaves "active" session with no backing process | Startup scan for orphaned active sessions; transition to `crashed` state |
| P-50 | Concurrent session access corrupts state | Multiple processes (MCP, orchestrator, TUI) writing same session file | Atomic writes (tempfile + persist), or append-only JSONL with reduce-on-read |
| P-53 | Large diffs exceed evaluator context window | Agent changes span thousands of lines | Budget context explicitly; prioritize test files; truncate with notice |
| P-55 | Diff computed against wrong base | `main..HEAD` includes parent branch changes, not just agent work | Record base commit SHA at worktree creation; diff against that SHA |
| P-56 | Terminal state corruption on crash | SIGKILL/SIGTERM bypass panic hook | Add SIGTERM/SIGHUP handlers; alternate screen buffer (implicit with ratatui) |
| P-58 | TUI blocks on sync operations | Gate evaluation takes minutes; TUI freezes | `spawn_blocking` + channels; `crossterm::event::poll` instead of blocking `read()` |

**Cross-cutting critical pitfalls:**
- **P-62 (Error proliferation):** Solve with sub-enum pattern in first phase, carry forward.
- **P-63 (Feature coupling):** Clear interfaces between modules. Worktree exports paths, session accepts paths, evaluator accepts diff strings. No "God struct."
- **P-64 (CWD mutation):** Never call `set_current_dir()`. Always use `Command::current_dir()` or `--work-tree`/`--git-dir` flags.

---

## Implications for Roadmap

### Recommended Phase Structure

**Phase 0: Structural Prerequisites** (1 week)
- Extract CLI monolith into `commands/` modules (mechanical, high-churn, must happen first)
- Add `assay-core` dependency to `assay-tui`
- Establish sub-enum error pattern (`WorktreeError`, `AgentError`, etc.)
- Add `WorkContext` struct for path threading

*Rationale:* These are refactoring tasks that touch many files. Doing them concurrently with feature work causes merge hell. Do them once, cleanly, before any new code lands.

**Phase 1: Type Foundation** (3-4 days)
- Add all new types to `assay-types`: `WorktreeState`, `WorktreeRecord`, `WorktreeConfig`, `WorkSession`, `SessionPhase`, `SessionOutcome`, `DiffSummary`, `FileDiff`, `DiffStatus`, `LaunchConfig`, `AgentProcessStatus`, `LaunchRecord`
- Wire re-exports, update schema snapshots
- `just ready`

*Rationale:* Types are the API contract. Getting them right first prevents churn in core modules. All four core modules depend on types; none depend on each other.

**Phase 2: Core Modules** (2 weeks, parallelizable)
- `assay_core::worktree` -- git worktree CRUD, porcelain parsing, prune
- `assay_core::diff` -- git diff stat/unified/name-only, context assembly, token budgeting
- `assay_core::launcher` -- Claude Code `--print` launch, PID tracking, timeout enforcement
- `assay_core::session` -- WorkSession CRUD, phase transitions, gate run linking
- New error variants with sub-enum pattern
- `just ready`

*Rationale:* These four modules are independent at the code level. They can be built and tested in any order. Integration testing (composing them into a pipeline) happens in Phase 4.

**Phase 3: TUI Gate Results Viewer** (1 week)
- Screen trait + routing architecture
- Gate results screen: table + detail pane + summary footer
- Non-blocking event loop with `poll()`
- Snapshot tests via `TestBackend` + `insta`
- `just ready`

*Rationale:* TUI is the first user-visible deliverable. It validates that the data model (GateRunRecord) renders correctly and establishes the TUI architecture for future screens.

**Phase 4: CLI + MCP Integration** (1.5 weeks)
- `assay worktree {create,list,remove,prune}` subcommands
- `assay session {list,show}` subcommands
- `assay launch <spec>` -- the full pipeline: worktree -> agent -> gates -> result
- MCP tools: `worktree_create`, `worktree_remove`, `gate_evaluate`
- `gate_evaluate` is the capstone: diff assembly + headless evaluator + auto-finalize
- `just ready`

*Rationale:* CLI and MCP are thin wrappers over core modules. They compose the modules into user-facing workflows. `gate_evaluate` is the last piece because it depends on worktrees, diffs, the launcher, and sessions all working together.

**Phase 5: Hardening + Radical Seeds** (1 week)
- Quick wins from FEATURES.md section 6: CLI exit codes, MCP parameter validation, error messages
- Schema evolution guard: `schema_version` field on WorkSession, no `deny_unknown_fields` on session types
- Startup reconciliation: detect orphaned worktrees and crashed sessions
- Gate history summary (radical seed for v0.4.0): aggregation logic over `.assay/results/`
- `just ready`, full regression

*Rationale:* Hardening after feature work, not before. The v0.3.0 features define the surface area that needs hardening. Radical seeds are low-effort, high-optionality additions that set up v0.4.0.

### Total Estimated Duration: 6-7 weeks

---

## Confidence Assessment

| Finding | Confidence | Basis |
|---------|-----------|-------|
| Zero new dependencies | **High** | Every alternative evaluated against cargo-deny constraints, existing patterns, and transitive dep counts |
| Shell-out-to-git for worktrees | **High** | git2 `!Send` limitation verified; gix worktree API incompleteness verified; existing subprocess pattern proven |
| `--print` mode sufficiency for evaluation | **High** | Claude Code docs verified; agtx reference implementation studied; no tmux needed for request-response evaluation |
| JSON files over SQLite for sessions | **High** | Scale analysis (tens-to-hundreds, not thousands); consistency with existing history module |
| `gate_evaluate` as differentiator | **High** | MCP ecosystem survey found no equivalent; closest analogs (MCPx-eval, agtx critic) are integrated, not externalized |
| CLI refactor necessity | **High** | 76K monolith + 3 new subcommand groups = unmaintainable without extraction |
| TUI architecture (Screen trait) | **Medium** | Reasonable pattern but the existing TUI is a skeleton -- actual rendering complexity unknown until implementation |
| Phase ordering and duration | **Medium** | Based on module independence analysis; actual velocity depends on test fixture complexity for git worktrees |
| Pitfall severity rankings | **Medium-High** | Ranked by blast radius and likelihood; some (P-48 PID reuse, P-54 rename detection) are edge cases that may not manifest in practice |

---

## Gaps to Address

### Gap 1: Subprocess Helper Extraction

STACK.md identifies that `evaluate_command()` is 100+ lines and v0.3.0 adds three more subprocess call sites. It proposes extracting a shared `assay_core::process` module with `ProcessConfig`/`ProcessResult` types. ARCHITECTURE.md does not include this module. **Decision needed:** extract the helper in Phase 0 (cleaner) or duplicate the pattern and refactor later (faster to start). Recommendation: extract in Phase 0. Three new call sites duplicating 100+ lines of timeout/reader-thread/killpg logic is a maintenance hazard.

### Gap 2: Launcher Sync vs Async

ARCHITECTURE.md recommends the launcher module be **async** (unlike most of assay-core), citing process monitoring and future multi-agent orchestration. STACK.md recommends **sync** (`std::process::Command` + `spawn_blocking`), citing consistency with the existing pattern. **Decision needed.** Recommendation: sync for v0.3.0. The v0.3.0 launcher is single-shot (`launch`, wait, collect result). Async adds complexity without benefit until v0.4.0's multi-agent orchestration. The guard daemon's async precedent is for event loops, not single-shot process management.

### Gap 3: `--dangerously-skip-permissions` Flag

ARCHITECTURE.md references `claude --dangerously-skip-permissions -p <prompt>` in the launch flow. STACK.md uses `claude -p <prompt>` without the flag. FEATURES.md mentions it only for agtx's interactive sessions. **Decision needed:** should Assay's launcher use this flag? Recommendation: no. `--print` mode is already non-interactive. The `--dangerously-skip-permissions` flag is for interactive sessions where Claude Code would normally prompt for permission. In `--print` mode, tools are controlled via `--allowedTools` instead.

### Gap 4: WorkSession vs SessionRecord Naming

ARCHITECTURE.md defines `WorkSession`. STACK.md defines `SessionRecord`. Both describe the same concept (persistent orchestration lifecycle record). **Decision needed.** Recommendation: `WorkSession` (ARCHITECTURE.md's choice). It distinguishes clearly from `AgentSession` and `SessionRecord` could be confused with a generic "record of a session" rather than a specific orchestration lifecycle type.

### Gap 5: Config Extension for Worktrees

ARCHITECTURE.md proposes `WorktreeConfig` with `dir` and `branch_prefix` fields, added as `pub worktrees: Option<WorktreeConfig>` on `Config`. STACK.md does not mention config changes. **Decision needed:** add this in Phase 1 (type foundation) or defer until user demand. Recommendation: add in Phase 1. The type is two fields with defaults. Adding it later means a schema-breaking change to Config.

### Gap 6: Composable Gate Definitions

FEATURES.md proposes composable gate definitions (gate templates with `inherit`) as a "radical seed." ARCHITECTURE.md does not address it. **Decision needed:** scope for v0.3.0 or defer to v0.4.0. Recommendation: defer to v0.4.0. v0.3.0 is already a large milestone. The gate history summary aggregation (also a radical seed) is lower-effort and higher-value -- implement that instead.

---

*Synthesized from parallel research -- 2026-03-08*
