# Assay Workflow: Desired State

Two workflow modes sharing a common foundation. The solo workflow is the stripped-down path for one developer, one project, one session at a time. The full workflow adds orchestration, parallel agents, and TUI-driven multi-session management via smelt.

---

## Shared Foundation

Both workflows are built on the same primitives:

```mermaid
erDiagram
    SPEC ||--|{ CRITERION : defines
    SPEC }o--o{ CRITERIA_LIBRARY : "includes"
    SPEC }o--o| SPEC : "extends"
    SPEC ||--o{ GATE_RUN : "history"
    SPEC ||--o{ SESSION : "work attempts"
```

**Core nouns (both modes):** Spec, Criterion, Gate

**Additional nouns (full mode only):** Milestone, Chunk, Worktree, Session, Manifest, Pipeline

---

## Solo Developer Workflow

### Philosophy

- Specs are first-class, standalone units — not subordinate to milestones
- A spec can function as its own mini-milestone (transparent wrapper if needed for cycle mechanics)
- The flow is autonomous: phases transition automatically with human checkpoints at decision points
- Gates are the machine-verifiable backbone — the defining feature of Assay

### Phase Flow

```mermaid
flowchart TD
    explore["1. EXPLORE<br/>/assay:explore"]
    plan["2. PLAN<br/>/assay:plan"]
    review["3. REVIEW PLAN<br/>human + agent critique"]
    execute["4. EXECUTE<br/>new session, auto from plan"]
    verify["5. VERIFY<br/>gates (auto) + UAT (optional)"]
    ship["6. SHIP<br/>PR / merge"]

    explore -->|"requirements crystallize"| plan
    plan -->|"plan generated"| review
    review -->|"approved"| execute
    review -->|"needs changes"| plan
    execute -->|"implementation done"| verify
    verify -->|"gates pass"| ship
    verify -->|"gates fail"| execute
    ship --> explore

    style explore fill:#e8eaf6
    style plan fill:#e3f2fd
    style review fill:#fff3e0
    style execute fill:#e8f5e9
    style verify fill:#fce4ec
    style ship fill:#f3e5f5
```

### Phase Details

#### 1. EXPLORE (`/assay:explore`)

**Purpose:** Thinking partner. Brainstorm requirements, investigate the codebase, compare approaches, make architectural decisions (clean arch vs N-layer, monorepo vs single repo, package choices).

**What happens:**

- Conversational — no fixed structure
- Agent reads code, asks questions, surfaces tradeoffs
- Architectural decisions captured as they're made
- Research can happen here (docs, library comparison)

**Output:** Clarity on what to build and key decisions. Optionally captured as notes/context.

**Transition:** User says "I know what I want" or the conversation naturally crystallizes into requirements.

#### 2. PLAN (`/assay:plan` or `assay plan quick`)

**Purpose:** Turn explored requirements into a concrete spec with verifiable criteria.

**What happens:**

- Interview: goal, acceptance criteria, optional chunk decomposition
- For simple work: flat spec (no milestone/chunk wrapper visible to user)
- For larger work: chunked spec (milestone created transparently)
- Each criterion is tagged with evaluation type (command, file check, agent report)

**Output:** Spec file(s) in `.assay/specs/` with full criteria.

**Transition:** Plan generated → automatically moves to review.

```mermaid
flowchart TD
    interview["Interview:<br/>What are you building?<br/>What does done look like?"]
    simple{"Complex enough<br/>for chunks?"}
    flat["Create flat spec<br/>(transparent 1-chunk milestone)"]
    chunked["Create milestone<br/>with N chunk specs"]
    done["Spec(s) ready"]

    interview --> simple
    simple -->|"No (< 5 criteria)"| flat
    simple -->|"Yes"| chunked
    flat --> done
    chunked --> done
```

#### 3. REVIEW PLAN

**Purpose:** Catch issues before execution. Human approval gate.

**What happens:**

- Agent critiques the plan: missing edge cases, unclear criteria, scope creep
- Human reviews and can edit spec directly
- Criteria can be refined, added, removed
- Plan marked "ready for execution" when approved

**Output:** Approved spec(s) with finalized criteria.

**Transition:** Human marks plan as ready → new session spawns for execution.

#### 4. EXECUTE

**Purpose:** Implement the code to satisfy spec criteria.

**What happens:**

- Fresh context window (new session or subagent)
- Spec criteria loaded automatically at session start
- Agent implements, guided by criteria
- Session state tracked (Created → AgentRunning) transparently

**Key design:** Execution happens in a clean context. The plan IS the handoff — no conversation history needed. The spec is the contract.

**Transition:** Agent signals implementation complete → automatic gate evaluation.

#### 5. VERIFY

**Purpose:** Machine-verifiable proof that code meets spec. Optional human verification.

**What happens:**

- **Gates (automatic):** Smart routing — picks the right evaluation path per criterion:
  - Command criteria → shell subprocess (path 1)
  - AgentReport criteria → evaluator subprocess or in-session report (path 2/3, chosen by config)
  - All results persisted to history
- **UAT (optional):** Agent-assisted human verification in a new session
  - Human walks through functionality with agent assistance
  - Agent surfaces relevant gate results and criteria
  - Human confirms or rejects

```mermaid
flowchart TD
    start["Implementation complete"]
    gates["Run gates<br/>(auto-routed per criterion type)"]
    result{"All required<br/>criteria pass?"}
    uat{"UAT configured?"}
    human["Agent-assisted UAT<br/>(new session)"]
    uat_result{"Human approves?"}
    pass["VERIFIED"]
    fail["Back to EXECUTE<br/>(with failure context)"]

    start --> gates
    gates --> result
    result -->|No| fail
    result -->|Yes| uat
    uat -->|No| pass
    uat -->|Yes| human
    human --> uat_result
    uat_result -->|Yes| pass
    uat_result -->|No| fail
```

**Transition:** All gates pass (+ optional UAT) → ship.

#### 6. SHIP

**Purpose:** Get verified code into the main branch.

**What happens:**

- Auto-prompt: "All criteria met. Create PR?"
- Gate results included in PR body as evidence
- If chunked: advance cycle to next chunk and loop back to execute
- If last chunk / flat spec: milestone complete

**Transition:** PR merged → back to explore for next piece of work.

---

### Solo Skill Surface (Desired)

| Skill               | Purpose                                                | Replaces                              |
| ------------------- | ------------------------------------------------------ | ------------------------------------- |
| `/assay:explore`    | Thinking partner, requirements discovery               | (new)                                 |
| `/assay:plan`       | Create spec from requirements                          | `/assay:plan` (simplified)            |
| `/assay:plan quick` | Flat spec, no chunks, minimal ceremony                 | (new)                                 |
| `/assay:focus`      | Show current spec criteria + gate status               | `/assay:next-chunk` + `/assay:status` |
| `/assay:check`      | Smart gate evaluation (auto-routes all criteria types) | `/assay:gate-check` (expanded)        |
| `/assay:ship`       | Gate-gated PR with evidence                            | (new, wraps `pr_create`)              |

**Removed from solo surface:** `/assay:next-chunk` (merged into `/assay:focus`), `/assay:status` (merged into `/assay:focus`)

---

### Solo Autonomous Flow

```mermaid
sequenceDiagram
    participant H as Human
    participant A as Assay Agent
    participant G as Gates
    participant S as State Backend

    H->>A: /assay:explore (discuss idea)
    A->>H: Questions, tradeoffs, decisions
    H->>A: "Ready to plan"
    A->>A: /assay:plan (create spec)
    A->>H: "Here's the plan. Approve?"
    H->>A: "Approved" (or edits)
    A->>S: Mark plan ready
    Note over A: New session / cleared context
    A->>A: Load spec, implement
    A->>S: session_update(agent_running)
    A->>G: /assay:check (auto-routed)
    G-->>A: Results
    alt Gates pass
        A->>H: "All criteria met. UAT?"
        alt UAT enabled
            Note over A: New session for UAT
            A->>H: Walk through changes
            H->>A: "Approved"
        end
        A->>H: "Create PR?"
        H->>A: "Yes"
        A->>S: Complete session, create PR
    else Gates fail
        A->>A: Fix and re-check
    end
```

---

## Full Workflow (TUI + Smelt Orchestration)

### Philosophy

- Everything from solo, plus parallel execution and multi-session management
- TUI is the control plane — dashboard, not just a viewer
- Milestones and chunks are explicit, first-class concepts (not hidden)
- Worktrees provide git isolation per agent/chunk
- Sessions are visible and manageable
- Smelt handles the DAG: dependencies, ordering, parallel dispatch

### Phase Flow

```mermaid
flowchart TD
    explore["1. EXPLORE<br/>TUI or CLI"]
    plan["2. PLAN<br/>Full milestone + chunk specs"]
    review["3. REVIEW<br/>Spec review + dependency validation"]
    manifest["4. MANIFEST<br/>Generate run manifest"]
    dispatch["5. DISPATCH<br/>Smelt orchestrator"]
    execute["6. EXECUTE (parallel)<br/>N agents in N worktrees"]
    verify["7. VERIFY (per chunk)<br/>Gates + optional UAT"]
    merge["8. MERGE<br/>Conflict detection + resolution"]
    ship["9. SHIP<br/>Milestone PR"]

    explore --> plan
    plan --> review
    review -->|approved| manifest
    review -->|needs changes| plan
    manifest --> dispatch
    dispatch --> execute
    execute --> verify
    verify -->|pass| merge
    verify -->|fail| execute
    merge -->|clean| ship
    merge -->|conflicts| execute
    ship --> explore

    style explore fill:#e8eaf6
    style plan fill:#e3f2fd
    style review fill:#fff3e0
    style manifest fill:#e0f7fa
    style dispatch fill:#f1f8e9
    style execute fill:#e8f5e9
    style verify fill:#fce4ec
    style merge fill:#fff9c4
    style ship fill:#f3e5f5
```

### Additional Concepts (Full Mode)

```mermaid
flowchart LR
    subgraph milestone["Milestone"]
        chunk1["Chunk 1"]
        chunk2["Chunk 2"]
        chunk3["Chunk 3"]
    end

    subgraph isolation["Isolation"]
        wt1["Worktree 1<br/>assay/chunk-1"]
        wt2["Worktree 2<br/>assay/chunk-2"]
        wt3["Worktree 3<br/>assay/chunk-3"]
    end

    subgraph agents["Parallel Agents"]
        a1["Agent 1<br/>Session S1"]
        a2["Agent 2<br/>Session S2"]
        a3["Agent 3<br/>Session S3"]
    end

    chunk1 --> wt1 --> a1
    chunk2 --> wt2 --> a2
    chunk3 --> wt3 --> a3
```

### TUI Screen Graph (Desired)

```mermaid
flowchart TD
    dashboard["Dashboard<br/>milestone list + active sessions"]
    detail["MilestoneDetail<br/>chunk grid + agent status"]
    spec["SpecView<br/>criteria + gate history + composition"]
    agent["AgentView<br/>live output + gate results"]
    wizard["PlanWizard<br/>milestone + chunk specs"]
    gatewiz["GateWizard<br/>create/edit spec"]
    pipeline["PipelineView<br/>DAG progress + merge status"]
    analytics["Analytics<br/>failure trends + velocity"]
    settings["Settings<br/>providers, models, retention"]
    sessions["SessionBrowser<br/>all sessions, filter, inspect"]

    dashboard --> detail
    dashboard --> wizard
    dashboard --> gatewiz
    dashboard --> pipeline
    dashboard --> analytics
    dashboard --> settings
    dashboard --> sessions
    detail --> spec
    detail --> agent
    spec --> agent
    pipeline --> agent
    sessions --> agent
```

### Smelt Orchestration Flow

```mermaid
sequenceDiagram
    participant TUI as TUI / CLI
    participant O as Orchestrator
    participant DAG as DAG Engine
    participant A1 as Agent 1
    participant A2 as Agent 2
    participant G as Gates
    participant M as Merge Engine

    TUI->>O: orchestrate_run(manifest)
    O->>DAG: Build dependency graph
    DAG-->>O: Execution waves

    par Wave 1 (independent chunks)
        O->>A1: Dispatch chunk-1 (worktree)
        O->>A2: Dispatch chunk-2 (worktree)
    end

    A1->>G: Gate evaluation
    G-->>A1: Pass
    A1-->>O: chunk-1 complete

    A2->>G: Gate evaluation
    G-->>A2: Fail
    A2->>A2: Fix and retry
    A2->>G: Gate evaluation (retry)
    G-->>A2: Pass
    A2-->>O: chunk-2 complete

    O->>M: merge_check(chunk-1, chunk-2)
    M-->>O: Clean merge
    O->>M: merge_propose(milestone)
    M-->>O: Merged branch ready

    O-->>TUI: Milestone ready for PR
    TUI->>TUI: pr_create(milestone)
```

---

## Comparison: Solo vs Full

| Aspect               | Solo                                              | Full                                                                                      |
| -------------------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| **Entry point**      | `/assay:explore` → `/assay:plan`                  | TUI dashboard → PlanWizard                                                                |
| **Spec granularity** | Flat spec (optional chunks)                       | Milestone with chunked specs                                                              |
| **Execution**        | Single agent, main branch                         | N agents, N worktrees                                                                     |
| **Gate routing**     | Auto (transparent)                                | Auto + configurable per criterion                                                         |
| **Session tracking** | Transparent (free)                                | Explicit, visible in TUI                                                                  |
| **Worktrees**        | Optional                                          | Required per chunk                                                                        |
| **Merge**            | Direct commit/PR                                  | Conflict detection + resolution                                                           |
| **UAT**              | Optional, agent-assisted                          | Optional, per-chunk or per-milestone                                                      |
| **State machine**    | explore → plan → review → execute → verify → ship | + manifest → dispatch → parallel execute → merge                                          |
| **Concept count**    | 3 visible (spec, criteria, gate)                  | 10+ (milestone, chunk, spec, criterion, gate, session, worktree, manifest, pipeline, DAG) |

---

## Transition Path: Solo → Full

A solo developer using the simple workflow should be able to upgrade without starting over:

1. **Flat specs work in milestones** — a flat spec is just a 1-chunk milestone
2. **Session history carries over** — all gate runs, sessions, and history persist
3. **Worktrees are additive** — adding isolation to existing specs doesn't break anything
4. **Config grows, doesn't change** — solo config is a subset of full config

```mermaid
flowchart LR
    solo["Solo: spec + gates"]
    add_chunks["Add chunks<br/>(split spec)"]
    add_worktrees["Add worktrees<br/>(per-chunk isolation)"]
    add_smelt["Add smelt<br/>(parallel orchestration)"]
    full["Full: milestone + chunks + worktrees + smelt"]

    solo --> add_chunks --> add_worktrees --> add_smelt --> full
```

---

## Resolved Decisions

### 1. `/assay:explore` — Skill for solo, TUI screen for full

Explore is fundamentally conversational — no structured input/output, so an MCP tool doesn't fit. The skill loads context (existing specs, codebase structure, config) and then it's conversation with Assay-awareness. For TUI: a dedicated screen where explore notes persist and feed into plan creation.

**Key principle:** Surface-agnostic state. Users can switch between skill/plugin in their harness of choice (Claude Code, Codex, OpenCode) and the Assay CLI/TUI with no loss of data or workflow state. Progressive improvement with graceful fallback to simple skill-based workflow.

### 2. Plan review — Spec status field with auto-promotion

Add `status` field to `gates.toml` with defined enum values:

```
draft → ready → approved → (gate pass) → verified
```

- Specs start as `draft` when created
- Human marks `ready` after review
- Approved by human or agent critique → `approved`
- Gate run with all-pass result auto-promotes to `verified`
- Queryable: "which specs are still draft?" / "which are verified?"

No separate approval artifact. The status is metadata about the spec, not a separate concern.

### 3. Auto-advance — State machine in core + signal emission for smelt

`assay-core` gets `workflow::next_action()` that takes current state and returns the next action (advance chunk, prompt for UAT, prompt for PR, etc.). Plugin skills and TUI consume this — the decision logic lives in one place, is testable in isolation.

For smelt orchestration: gate completion also emits a signal that the orchestrator can subscribe to. The state machine (B) is the primary mechanism; signals (C) are the notification layer on top.

```rust
// Conceptual API
enum NextAction {
    AdvanceChunk { next_chunk: String },
    PromptUat { spec_name: String, gate_run_id: String },
    PromptPr { milestone_slug: String },
    FixAndRecheck { failed_criteria: Vec<String> },
    Complete,
}

fn next_action(assay_dir: &Path) -> Result<NextAction>;
```

### 4. UAT handoff — spec_name + gate_run_id

The UAT session loads three things, all already persisted:
- **Spec** (what to verify) — on disk in `.assay/specs/`
- **Gate results** (what passed/failed) — in `.assay/history/`
- **Diff** (what changed) — from git

No direct session-to-session linkage needed. The spec is the contract, the gate run is the evidence. State backend (file, Linear, etc.) holds all necessary data.

### 5. Session retention — Count + age, lazy eviction

Configurable in `.assay/config.toml`:

```toml
[sessions]
max_count = 100       # keep N most recent
max_age_days = 90     # delete older than N days
```

Whichever limit is hit first wins. Cleanup runs lazily (on `session_create` or `session_list`), not via background daemon. Same pattern as the existing GateEvalContext eviction (50 most recent), extended to WorkSessions with user-configurable limits.

### 6. `assay plan quick` — Transparent 1-chunk milestone + config-driven branch isolation

**Milestone:** Transparent 1-chunk milestone. Cycle mechanics (cycle_status, cycle_advance, gate history) all assume a milestone exists. Create it silently — user sees `Spec: add-dark-mode (5 criteria)`, never `Milestone: add-dark-mode, Chunk 1 of 1`.

**Branch/worktree strategy:** Config-driven with smart default.

```toml
[workflow]
auto_isolate = "ask"  # "always" | "never" | "ask"
```

Behavior:
- `"ask"` (default for solo): If on a protected branch (main/develop/master), prompt "Create a branch for this work?". If already on a feature branch, proceed without asking.
- `"always"` (default for full/smelt): Silently create a worktree.
- `"never"`: Work on current branch, no questions asked.

This covers the 90% case: solo devs on a feature branch get zero friction, solo devs on main get a safety prompt, full mode always isolates.

### 7. Gate evidence — Full results, surface-adapted rendering

The gate result data structure is the same everywhere. Rendering adapts per surface:

| Surface | Rendering |
|---------|-----------|
| Terminal / CLI | 1-line summary |
| Claude Code plugin | Collapsed detail block |
| TUI | Expandable panel |
| PR body | Summary + run ID link |
| PR check run / comment | Full criterion-by-criterion, collapsible sections |

Gate results as check run / PR comment ships out of the box — smelt orchestration depends on forge PRs and needs the full detail. Terminal and in-agent surfaces parse/collapse the same data for minimal output.
