## Context

Assay currently implements a milestone → chunk → spec → gate pipeline designed for multi-agent orchestration (smelt). The solo developer path reuses this infrastructure but exposes all its complexity. The current plugin skill surface (`/assay:plan`, `/assay:status`, `/assay:next-chunk`, `/assay:gate-check`, `/assay:spec-show`) is execution-focused with no exploration phase, manual-only transitions, and gate evaluation limited to shell commands.

This design tightens the solo workflow into a 6-phase loop (explore → plan → review → execute → verify → ship) while preserving full compatibility with the smelt orchestration path. All changes are additive — no existing functionality is removed, only consolidated and extended.

Key constraints:
- All state must be surface-agnostic (CLI, TUI, any harness plugin can read/write it)
- Existing specs without the new `status` field must work without migration
- The `workflow::next_action()` function must be pure (read state, return action — no side effects)
- Gate evidence rendering is a presentation concern, not a data model change

## Goals / Non-Goals

**Goals:**
- Solo dev sees 3 concepts (spec, criteria, gate) instead of 10+
- Phase transitions flow autonomously with human checkpoints at decision points
- Gate evaluation handles all criterion types transparently from a single entry point
- Session data accumulates with configurable retention, not indefinitely
- A flat spec gets full cycle/gate mechanics via transparent 1-chunk milestone
- Branch isolation is config-driven with smart defaults
- Gate evidence renders appropriately per surface (terminal, TUI, PR, in-agent)

**Non-Goals:**
- TUI explore screen (deferred — TUI gets the skill-based explore via MCP, dedicated screen is a later milestone)
- Smelt signal emission from `next_action()` (noted for future — the state machine returns actions, smelt adds the event bus)
- Changes to the gate evaluation engine itself (paths 1/2/3 stay as-is; this change is about routing to the right path, not changing the paths)
- Criteria library or composition changes
- Milestone/chunk data model changes (we add a transparent creation mode, not a new model)

## Decisions

### D1: Spec status field lives in `gates.toml`, not a separate file

**Choice:** Add `status` field directly to `GatesSpec` struct.

**Alternatives considered:**
- Separate `.assay/specs/<slug>/approval.json` — cleaner separation but adds file management overhead for solo devs
- Status tracked only in session/milestone state — loses queryability ("which specs are draft?")

**Rationale:** The status is metadata about the spec itself. Solo devs shouldn't manage extra files. The field is optional with `#[serde(default)]` — existing specs deserialize with `status: None`, which the workflow engine treats as `draft`.

### D2: `workflow::next_action()` is a pure function in `assay-core`

**Choice:** Single function that reads milestone state, spec status, and gate history, returns a `NextAction` enum.

```rust
pub enum NextAction {
    /// No active work — suggest explore or plan
    Idle,
    /// Spec is draft — needs review/approval
    ReviewSpec { spec_name: String },
    /// Spec is approved — ready for execution
    Execute { spec_name: String, chunk_slug: Option<String> },
    /// Implementation done — run gates
    RunGates { spec_name: String },
    /// Gates failed — show failures, suggest fixes
    FixAndRecheck { spec_name: String, failed_criteria: Vec<String> },
    /// Gates passed — prompt for UAT if configured
    PromptUat { spec_name: String, gate_run_id: String },
    /// Gates + UAT passed, more chunks remain
    AdvanceChunk { milestone_slug: String, next_chunk: String },
    /// All chunks done — prompt for PR
    PromptShip { milestone_slug: String },
}
```

**Alternatives considered:**
- Hook-based triggers (plugin-specific, not portable across surfaces)
- Event-driven pub/sub (right for smelt, over-engineered for solo)

**Rationale:** Pure function is testable, surface-agnostic, and composable. Smelt can wrap it with event emission later. Skills and TUI call it and act on the result.

### D3: Smart gate routing selects evaluation path per criterion

**Choice:** The `/assay:check` skill (and underlying gate logic) inspects each criterion's `kind` field and routes accordingly:

| Criterion Kind | Evaluation Path |
|---------------|----------------|
| `Command`, `FileExists` | Path 1: `evaluate_all()` shell subprocess |
| `AgentReport` | Path 3: `gate_evaluate()` evaluator subprocess (default) or Path 2: manual `gate_report` flow (if configured) |
| `EventCount`, `NoToolErrors` | Skipped (pipeline-only) |

**Alternatives considered:**
- Always use Path 3 (`gate_evaluate`) for everything — wasteful for simple shell commands
- Keep paths separate and let user choose — cognitive overhead

**Rationale:** The criterion already declares its type. Routing is a mechanical decision, not a judgment call. The skill hides the ceremony; power users can still call individual MCP tools directly.

### D4: `plan quick` creates a transparent 1-chunk milestone

**Choice:** `assay plan quick` creates a milestone with `slug = spec_slug`, a single chunk with `slug = spec_slug`, and the spec. The skill surface never shows "milestone" or "chunk" terminology.

**Alternatives considered:**
- Spec-only mode without milestone — requires parallel code paths for cycle_status, cycle_advance, gate history
- New "micro-milestone" type — adds a type to the data model for no user benefit

**Rationale:** The milestone/chunk model already works. Creating a 1:1:1 mapping (milestone:chunk:spec) gives full cycle mechanics for free. The complexity is hidden in presentation, not data model.

### D5: Branch isolation uses config + heuristic

**Choice:** New config section:

```toml
[workflow]
auto_isolate = "ask"  # "always" | "never" | "ask"
```

When `"ask"`: detect current branch. If it matches a protected pattern (main, master, develop, or user-configured list), prompt to create a worktree/branch. If already on a feature branch, proceed silently.

**Alternatives considered:**
- Always create worktree (too aggressive for solo on feature branch)
- Never isolate (unsafe when on main)
- Branch-only without worktree (sufficient for solo, but worktree is the existing primitive)

**Rationale:** Config-driven with smart default covers 90% of cases. Solo default is `"ask"`, full/smelt default is `"always"`.

### D6: Gate evidence is one data structure, multiple renderers

**Choice:** `GateRunRecord` (already exists in history) is the canonical data. Add rendering functions per surface:

- `render_terminal(record) → String` — 1-line summary
- `render_markdown_collapsed(record) → String` — collapsed detail block for in-agent
- `render_pr_body(record) → String` — summary + run ID
- `render_pr_check(record) → String` — full criterion-by-criterion with collapsible HTML

**Alternatives considered:**
- Different data structures per surface — duplication, drift risk
- Single verbose format everywhere — noisy in terminal

**Rationale:** The data is already there. This is purely a presentation layer. Renderers are simple functions with no state.

## Risks / Trade-offs

- **[Schema migration]** Adding `status` to `GatesSpec` is a non-breaking change (field is optional, defaults to `None` → treated as `draft`). But if we later want to auto-set `verified` for specs with passing history, we need a one-time backfill. → **Mitigation:** Defer backfill. New specs get status; existing specs stay `None` until next gate run.

- **[`next_action()` reads across multiple files]** The function needs to load milestones, specs, and gate history to determine state. On large projects this could be slow. → **Mitigation:** Early return on common cases (no active milestone → `Idle`). Cache milestone/spec index in memory for TUI. CLI pays the cost once per invocation.

- **[Skill deprecation]** Renaming `/assay:gate-check` → `/assay:check` and merging `/assay:status` + `/assay:next-chunk` → `/assay:focus` breaks muscle memory. → **Mitigation:** Keep old skill names as aliases for one version cycle. Log deprecation notice when used.

- **[Transparent milestone confusion]** A `plan quick` user who later runs `assay milestone list` will see their "flat spec" as a milestone. → **Mitigation:** Mark transparent milestones with a `quick: true` flag. `milestone list` can filter or annotate them.

- **[Config section growth]** Adding `[workflow]` and `[sessions]` sections grows the config surface. → **Mitigation:** Both sections are optional with sensible defaults. `assay init` doesn't generate them — they appear only when the user explicitly configures.

## Open Implementation Questions

These should be resolved during task planning, not design:

1. **Backward compatibility** — Existing specs without `status` field: default to `draft` (safe) or infer `verified` from gate history (smart but complex)?
2. **Skill alias mechanism** — Do plugin skills support aliases natively, or do we need separate SKILL.md files that redirect?
3. **Protected branch detection** — Hardcoded list (main/master/develop) or read from git config (`init.defaultBranch`, branch protection rules)?
4. **`quick: true` flag on milestones** — New field on `Milestone` struct, or inferred from 1-chunk + matching slugs?
5. **UAT configuration** — Where does "UAT enabled" live? Per-spec? Per-project config? Both?
