# Explore: Tightening the Core Solo-Developer Workflow

## The Current Workflow (as designed)

```
┌──────────┐    ┌──────────┐    ┌──────────────────────────┐    ┌──────────┐    ┌─────────┐
│  /plan   │───▶│ /status   │───▶│  /next-chunk → implement │───▶│/gate-check│───▶│cycle_   │
│          │    │           │    │                          │    │          │    │advance  │
│ interview│    │ where am  │    │  read criteria, write    │    │ run all  │    │         │
│ → create │    │ I?        │    │  code to satisfy them    │    │ criteria │    │ repeat  │
│ milestone│    │           │    │                          │    │          │    │ or PR   │
└──────────┘    └──────────┘    └──────────────────────────┘    └──────────┘    └─────────┘
```

## Concept Count Problem

A solo dev hits **at least 10 nouns** before they write a line of code:

```
project → milestone → chunk → spec → criterion → gate → cycle → session → worktree → harness
```

Compare that to what they're actually thinking:

```
"I want to build X. Here's what done looks like. Let me do it in pieces."
```

That maps to maybe **3 concepts**: goal, acceptance criteria, progress.

---

## Threads

### 1. Is milestone→chunk→spec the right decomposition for solo?

Right now: a milestone contains chunks, each chunk has a spec, each spec has criteria. But for solo work, a milestone often _is_ the spec — "add dark mode" with 5 criteria. The chunk layer adds indirection that helps parallelization (smelt) but may just be friction solo.

**Question**: Should a solo dev be able to say `assay plan` and get a **flat spec with criteria** — no milestone wrapper, no chunks — and still get the full gate/cycle experience?

**Answer**: Yes, a solo dev (ideally starting with assay discuss/explore/brainstorm, then assay plan) should be able to get a flat spec - the idea being that specs are composable/atomic and can function as their own 'mini-milestone' or 'mini-chunk'. If we have to use a milestone wrapper to get full gate/cycle experience, that's okay too - we'll do it transparently. Maybe we separate a specific assay command like 'assay plan quick' or similar for this stripped down workflow.

### 2. Where does "thinking" live?

Current skills are: plan, status, next-chunk, spec-show, gate-check. All task-execution focused. There's no Assay-native equivalent of exploring, brainstorming, investigating before committing to a plan. OpenSpec fills that gap externally, but is there a reason Assay itself doesn't have an explore/discuss phase?

Or is the deliberate boundary: "Assay starts when you know what to build; use other tools for figuring out what to build"?

**Question**: Should Assay have its own explore/discuss phase, or is that intentionally out of scope?

**Answer**: Yes, definitely - I see a lot of use in discussing with kata, openspec, BMAD, superpowers etc. - honing in on the specifics of what our requirements are and also deciding some implementation details up front (packages to use, architectural decisions (clean arch? N-layer? monorepo or single project repository? etc)) concretely helps the agents downstream. Most spec kits seem to do this as a single 'phase/turn/session', usually paired with a similar research phase and plan phase.

### 3. The gate evaluation paths are confusing

Three ways to evaluate:

| Path                                             | When                             | Solo relevance                                         |
| ------------------------------------------------ | -------------------------------- | ------------------------------------------------------ |
| `gate_run`                                       | Shell commands only              | High — the simple case                                 |
| `gate_run` → `gate_report` × N → `gate_finalize` | Agent self-eval, manual          | Medium — but feels like ceremony                       |
| `gate_evaluate`                                  | Agent eval, automated subprocess | Low for interactive solo — this is for headless agents |

For a solo dev interactively coding, path 1 is the 90% case. Path 2 is where the AI agent evaluates subjective criteria ("code is well-structured"). Path 3 is the pipeline path.

**Question**: Should the plugin skill just pick the right path transparently? Right now `/gate-check` only uses `gate_run`. Should it handle agent criteria too, hiding the multi-step ceremony?

**Answer**: Yes, the plugin should handle it automatically, with optional manual activation by human or agent. The /gate-check obly using 'gate_run' is an issue - we should automatically handle it based on state (are we in a milestone/chunk/task?) and configuration.

### 4. Session lifecycle — does a solo dev need it?

`session_create` → `session_update(agent_running)` → `session_update(gate_evaluated)` → `session_update(completed)` is a 4-call lifecycle tracking mechanism. For a solo dev in one Claude Code conversation, the conversation _is_ the context.

Possible solo use cases: resuming work after context reset, auditing what happened across sessions, linking gate runs to specific work attempts.

**Question**: Does session tracking have a solo use case I'm not seeing, or is it purely smelt/multi-agent infrastructure?

**Answer**: They get it for free - if they decide to later begin using assay for parallel orchestration of changes, they have all their sessions and data saved and available already. There should be default retention limits on sessions too already, but verify this please.

### 5. The "what should I do next?" experience

Currently, the solo flow requires the dev to _know_ to call `/assay:next-chunk` after completing a chunk. The cycle is push-based (you poll status and advance manually).

**Question**: Should the flow be more pull-based? e.g., after a gate check passes, Assay proactively says "all criteria met — advance to next chunk?" Instead of expecting the dev to remember the workflow.

**Answer**: Yes, autonomous in a safe and controlled fashion. We should be able to move from having a verified, reviewed/critiqued/updated plan marked ready for execution to a new (cleared context window or new subagent etc.) session and automatically execute the plan. Similarly we should be able to move from agent-verified to optional UAT (agent-assisted human verification phase) automatically as well, handing off any results to our state backend and doing UAT in a new session. Gate checks tie into this autonomous nature - they offer machine/agent verifiable results that the code does what it is supposed to and conforms to the expected shape etc.

---

## A Sketch: What "tight solo" Could Look Like

```
┌─────────────────────────────────────────────────────────────────┐
│                    Solo Developer Flow                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. DEFINE        "assay new" or "/assay:new"                   │
│     ┌────────────────────────────────────┐                      │
│     │ What are you building?             │                      │
│     │ What does done look like?          │                      │
│     │ → flat spec with criteria          │                      │
│     │ → optional: split into chunks      │                      │
│     └────────────────────────────────────┘                      │
│                        │                                        │
│  2. WORK          read criteria → write code                    │
│     ┌────────────────────────────────────┐                      │
│     │ /assay:focus  — show what to do    │                      │
│     │ (auto-shows on conversation start) │                      │
│     └────────────────────────────────────┘                      │
│                        │                                        │
│  3. CHECK         /assay:check (or auto after commit?)          │
│     ┌────────────────────────────────────┐                      │
│     │ Run gates                          │                      │
│     │ ✓ passed → "advance?" prompt       │                      │
│     │ ✗ failed → show what's left        │                      │
│     └────────────────────────────────────┘                      │
│                        │                                        │
│  4. SHIP          auto when all criteria pass                   │
│     ┌────────────────────────────────────┐                      │
│     │ "All done. Create PR?"             │                      │
│     └────────────────────────────────────┘                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

Key differences from current:

- **Fewer nouns**: spec, criteria, gate. That's it for the common case.
- **Optional chunking**: chunks are an opt-in decomposition, not mandatory
- **Proactive flow**: the system tells you what's next rather than you polling
- **Single "check" entry point**: picks the right evaluation strategy internally

---

### 6. Anything else?

Anything I missed, got wrong, or that you want to add to the discussion?

**Answer**: I like the new direction you've outlined above - I just want to make sure we're not losing any of the defining functionality of assay, which is the gate functionality. I think what we do this milestone/spec is debate/work on/update our core workflows - we'll make a couple mermaid diagrams for our stripped-down, solo dev workflow, and our fully-featured TUI workflow with smelt parallel orchestration built in.

First - let's put our current state workflows in a document, complete with mermaid diagrams, then create our .md documents and mermaid diagrams for our desired workflow states. This gives us a good starting point and we can both edit it collaboratively as we discuss and decide.
