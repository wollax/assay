## Why

Assay's current workflow exposes 10+ concepts (project, milestone, chunk, spec, criterion, gate, cycle, session, worktree, harness) before a solo developer writes a line of code. The workflow is also execution-focused — there's no explore/discuss phase, gate evaluation only handles shell commands (not agent criteria), and phase transitions are entirely manual. A solo dev working on one project and one session at a time needs a tighter loop: explore → plan → review → execute → verify → ship, with autonomous transitions and gates as the machine-verifiable backbone.

## What Changes

- Add **explore phase** as a plugin skill (`/assay:explore`) that loads Assay context (specs, config, codebase) for conversational requirements discovery
- Add **spec status field** (`draft | ready | approved | verified`) to `gates.toml` with auto-promotion on gate pass
- Add **`workflow::next_action()`** in `assay-core` — a state machine that reads current state (milestone, specs, gate history) and returns the next action (advance chunk, prompt UAT, prompt PR, fix and recheck)
- Add **smart gate routing** to `/assay:check` skill — auto-detects criterion types and picks the right evaluation path (command → shell, agent report → evaluator subprocess or manual flow) transparently
- Add **session retention** — configurable count + age limits with lazy eviction on session_create/session_list
- Add **`assay plan quick`** mode — creates a transparent 1-chunk milestone so flat specs get full cycle/gate mechanics without exposing milestone/chunk concepts
- Add **branch isolation heuristic** — config-driven `auto_isolate` setting (`always | never | ask`) with smart default: prompt if on protected branch, proceed if on feature branch
- Add **surface-adapted gate evidence rendering** — full gate results as PR check run/comment out of the box, collapsed/minimal rendering for terminal, TUI, and in-agent surfaces
- **Merge** `/assay:status` and `/assay:next-chunk` into `/assay:focus` (single entry point for "what am I working on?")
- **Rename** `/assay:gate-check` to `/assay:check` with expanded capability
- Add `/assay:ship` skill wrapping gate-gated PR creation with evidence

## Capabilities

### New Capabilities

- `explore-phase`: Conversational requirements discovery skill that loads Assay project context (specs, config, codebase structure) for brainstorming, research, and architectural decision-making
- `spec-status`: Spec lifecycle status field (draft → ready → approved → verified) with auto-promotion on passing gate run
- `workflow-engine`: Core state machine (`workflow::next_action()`) that determines the next workflow action from current project state, consumed by all surfaces (skills, TUI, CLI)
- `smart-gate-routing`: Unified gate check entry point that auto-detects criterion types and routes to the correct evaluation path (shell subprocess, agent report flow, evaluator subprocess)
- `session-retention`: Configurable retention limits (count + age) for WorkSessions with lazy eviction
- `plan-quick`: Streamlined planning mode that creates a transparent 1-chunk milestone for flat specs
- `branch-isolation`: Config-driven branch isolation strategy with protected-branch detection heuristic
- `gate-evidence-rendering`: Surface-adapted gate result rendering (PR check run, collapsed TUI panel, minimal terminal output)
- `solo-skill-surface`: Consolidated skill set (`/assay:explore`, `/assay:plan`, `/assay:focus`, `/assay:check`, `/assay:ship`) replacing the current fragmented skill surface

### Modified Capabilities

## Impact

- **assay-types**: New `SpecStatus` enum, `status` field on `GatesSpec`
- **assay-core/spec**: Schema change — existing specs without `status` field need migration (default to `draft`, or `verified` if passing gate history exists)
- **assay-core/workflow**: New `next_action()` function reading across milestones, specs, and gate history
- **assay-core/gate**: Gate routing logic to select evaluation path per criterion type
- **assay-core/work_session**: Retention/eviction logic
- **assay-core/config**: New `[workflow]` and `[sessions]` config sections
- **assay-core/milestone**: `plan quick` transparent 1-chunk milestone creation
- **plugins/claude-code**: New and updated skill definitions (explore, focus, check, ship; deprecate status, next-chunk, gate-check)
- **plugins/codex, plugins/opencode**: Matching skill updates
- **crates/assay-cli**: New `assay plan quick` subcommand, gate evidence formatting
- **crates/assay-tui**: Gate evidence panel rendering (explore TUI screen deferred to later milestone)
