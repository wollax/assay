# Assay Workflow: Current State

This document captures how Assay's workflow operates today, as implemented in the codebase.

---

## High-Level Flow

```mermaid
flowchart TD
    init["assay init"]
    plan["assay plan / /assay:plan"]
    status["/assay:status"]
    next["/assay:next-chunk"]
    work["Implement Code"]
    gate["/assay:gate-check"]
    advance["cycle_advance"]
    pr["assay pr create"]

    init --> plan
    plan --> status
    status --> next
    next --> work
    work --> gate
    gate -->|"required_failed > 0"| work
    gate -->|"all required passed"| advance
    advance -->|"more chunks"| next
    advance -->|"all chunks done → Verify"| pr
```

---

## Concept Hierarchy

```mermaid
erDiagram
    PROJECT ||--o{ MILESTONE : contains
    MILESTONE ||--|{ CHUNK : "ordered list"
    CHUNK ||--|| SPEC : "1:1 mapping"
    SPEC ||--|{ CRITERION : defines
    SPEC ||--o{ GATE_RUN_RECORD : "history"
    MILESTONE ||--o{ WORKTREE : "optional isolation"
    SPEC ||--o{ WORK_SESSION : "tracks agent work"
    SPEC }o--o{ CRITERIA_LIBRARY : "includes"
    SPEC }o--o| SPEC : "extends (parent)"
```

**Nouns a user encounters:**
Project, Milestone, Chunk, Spec, Criterion, Gate, Cycle, Session, Worktree, Harness, Criteria Library, Precondition

---

## Milestone Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Draft : milestone_create
    Draft --> InProgress : first cycle_advance call
    InProgress --> InProgress : cycle_advance (chunk N → chunk N+1)
    InProgress --> Verify : cycle_advance (last chunk passes)
    Verify --> Complete : pr_create / manual
    Complete --> [*]
```

**Persistence:** `.assay/milestones/<slug>.toml`

| Field | Purpose |
|-------|---------|
| slug | Unique ID (from filename) |
| name | Display name |
| status | Draft / InProgress / Verify / Complete |
| chunks | Ordered `Vec<ChunkRef>` (slug, name, order) |
| completed_chunks | `Vec<String>` of finished chunk slugs |
| created_at, updated_at | Timestamps |
| pr_number, pr_url | Populated after PR creation |

---

## Spec Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Authored : spec_create / gate_wizard
    Authored --> Validated : spec_validate
    Authored --> Composed : spec_resolve (extends + include)
    Composed --> Evaluated : gate_run / gate_evaluate
    Evaluated --> Passed : all required criteria pass
    Evaluated --> Failed : required_failed > 0
    Failed --> Authored : fix and re-evaluate
```

**Format:** `.assay/specs/<slug>/gates.toml` (directory format, current)

**Composition Model:**
- `extends`: Single parent gate spec (inheritance chain)
- `include`: Multiple criteria library slugs (flat union)
- Resolution order: own criteria > included > extended

**Criterion Types:**

| Kind | Evaluated By | Requires |
|------|-------------|----------|
| Command | Shell subprocess | `cmd` field |
| FileExists | Path check | `path` field |
| AgentReport | AI agent | `prompt` field |
| EventCount | Pipeline events | Pipeline context |
| NoToolErrors | Pipeline events | Pipeline context |

---

## Gate Evaluation (Three Paths)

```mermaid
flowchart TD
    subgraph path1["Path 1: Synchronous (Commands)"]
        p1_start["gate_run(spec)"] --> p1_eval["evaluate_all()"]
        p1_eval --> p1_result["GateRunSummary"]
        p1_result --> p1_save["Persist to history"]
    end

    subgraph path2["Path 2: Agent Self-Eval (Manual)"]
        p2_start["gate_run(spec)"] --> p2_ctx["GateEvalContext created"]
        p2_ctx --> p2_report["gate_report() × N"]
        p2_report --> p2_final["gate_finalize()"]
        p2_final --> p2_save["Persist to history"]
    end

    subgraph path3["Path 3: Evaluator Subprocess (Automated)"]
        p3_start["gate_evaluate(spec)"] --> p3_spawn["Spawn headless claude"]
        p3_spawn --> p3_parse["Parse JSON output"]
        p3_parse --> p3_save["Persist to history"]
    end

    style path1 fill:#e8f5e9
    style path2 fill:#fff3e0
    style path3 fill:#e3f2fd
```

**When each path is used:**

| Path | Trigger | Criteria Supported | Solo Relevance |
|------|---------|-------------------|----------------|
| 1 - Synchronous | `/gate-check`, `cycle_advance` | Command, FileExists | High (90% case) |
| 2 - Manual agent | MCP `gate_run` with AgentReport criteria | AgentReport (multi-step) | Medium |
| 3 - Evaluator subprocess | MCP `gate_evaluate` | All types via LLM | Low (headless/pipeline) |

**History Persistence:** `.assay/history/<spec_name>/<run_id>.json`

---

## Session Lifecycle

### GateEvalContext (Ephemeral)

Created by `gate_run()` when a spec has `AgentReport` criteria. Lives in memory during evaluation.

```mermaid
flowchart LR
    create["gate_run() creates context"] --> report["gate_report() × N"]
    report --> finalize["gate_finalize()"]
    finalize --> saved[".assay/gate_sessions/<id>.json"]
```

- Eviction: 50 most recent retained
- No phase state machine — linear create → report → finalize

### WorkSession (Persistent)

Created by `session_create()` for long-running agent work. Tied to a spec and worktree.

```mermaid
stateDiagram-v2
    [*] --> Created : session_create
    Created --> AgentRunning : session_update(agent_running)
    AgentRunning --> GateEvaluated : session_update(gate_evaluated)
    GateEvaluated --> Completed : session_update(completed)
    Created --> Abandoned : session_update(abandoned)
    AgentRunning --> Abandoned : session_update(abandoned)
    GateEvaluated --> Abandoned : session_update(abandoned)
```

**Persistence:** `.assay/sessions/<session_id>.json` (ULID-based)

**Fields:** id, spec_name, worktree_path, phase, transitions (audit trail), agent (command + model), gate_runs (linked IDs), tool_call_summary, assay_version

**Gap:** No retention limits. Sessions accumulate indefinitely.

---

## Worktree Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Created : worktree_create(spec_slug)
    Created --> Active : developer works in worktree
    Active --> Active : commits, gate runs
    Active --> CleanedUp : worktree_cleanup(spec_slug)
    Active --> ForceCleanedUp : worktree_cleanup(spec_slug, force=true)
    CleanedUp --> [*]
    ForceCleanedUp --> [*]
```

**Persistence:** `.assay/worktrees/<spec_slug>/` directory + `.assay/worktree.json` metadata inside

**Branch:** `assay/<spec_slug>` based off detected default branch (or user-specified base)

**Gap:** No automatic cleanup. No retention limits.

---

## cycle_advance Algorithm

```mermaid
flowchart TD
    start["cycle_advance(milestone_slug?)"] --> locate["1. Locate milestone<br/>(first InProgress or specified)"]
    locate --> verify["2. Verify status = InProgress"]
    verify --> active["3. Find active chunk<br/>(first not in completed_chunks)"]
    active --> load["4. Load spec for chunk"]
    load --> eval["5. evaluate_all_gates()"]
    eval --> check{"6. required_failed > 0?"}
    check -->|Yes| fail["Return error<br/>(NO state modified)"]
    check -->|No| mark["7. Push chunk to completed_chunks"]
    mark --> transition{"8. Any chunks left?"}
    transition -->|No| phase["Transition milestone → Verify"]
    transition -->|Yes| skip["Stay InProgress"]
    phase --> save["9. Atomic save milestone"]
    skip --> save
    save --> result["10. Return CycleStatus"]
```

**Key property:** Steps 1-6 are read-only. Gate failure leaves milestone untouched.

---

## TUI Screen Graph

```mermaid
flowchart TD
    noproject["NoProject Screen"]
    dashboard["Dashboard<br/>(milestone list)"]
    detail["MilestoneDetail<br/>(chunks, status)"]
    chunk["ChunkDetail<br/>(criteria, gate results)"]
    agent["AgentRun<br/>(live subprocess output)"]
    wizard["Wizard<br/>(create milestone + specs)"]
    gatewiz["GateWizard<br/>(create/edit gate spec)"]
    settings["Settings<br/>(providers, models)"]
    analytics["Analytics<br/>(failure frequency, velocity)"]
    mcp["McpPanel<br/>(configure MCP servers)"]
    traces["TraceViewer<br/>(inspect agent events)"]
    slash["SlashOverlay<br/>(command palette)"]

    noproject -->|"assay init"| dashboard
    dashboard --> detail
    dashboard --> wizard
    dashboard --> gatewiz
    dashboard --> settings
    dashboard --> analytics
    dashboard --> mcp
    dashboard --> traces
    dashboard --> slash
    detail --> chunk
    chunk --> agent
    wizard -->|submit| dashboard
    gatewiz -->|submit| dashboard
```

---

## Plugin Skill Surface (Claude Code)

| Skill | Purpose | MCP Tools Used |
|-------|---------|----------------|
| `/assay:plan` | Interview → create milestone + chunk specs | `milestone_create`, `spec_create` |
| `/assay:status` | Show active milestone, phase, progress | `cycle_status` |
| `/assay:next-chunk` | Load active chunk criteria + gate status | `cycle_status`, `chunk_status`, `spec_get` |
| `/assay:spec-show` | Display spec criteria | `spec_list`, `spec_get` |
| `/assay:gate-check` | Run gates, report results | `gate_run` (path 1 only) |

**Gap:** `/gate-check` only uses `gate_run` (path 1). AgentReport criteria are skipped, not evaluated.

---

## Data Retention Summary

| Data | Location | Retention | Eviction |
|------|----------|-----------|----------|
| Milestones | `.assay/milestones/` | Indefinite | None |
| Specs | `.assay/specs/` | Indefinite | None |
| Gate history | `.assay/history/` | Indefinite | None |
| Gate eval contexts | `.assay/gate_sessions/` | 50 most recent | Auto-evict on new session |
| Work sessions | `.assay/sessions/` | Indefinite | None |
| Worktrees | `.assay/worktrees/` | Indefinite | Manual cleanup only |
| Criteria libraries | `.assay/criteria/` | Indefinite | None |
| Traces | `.assay/traces/` | Indefinite | None |
