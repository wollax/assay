# Smelt Brainstorm — Synthesis

**Date:** 2026-03-09
**Pairs:** 3 (quick-wins, high-value, radical)
**Agents:** 6 (3 explorer/challenger pairs, 2-3 debate rounds each)

---

## Quick Wins (v0.1 — ~10-13 days)

| Feature | Scope | Key Decision |
|---|---|---|
| **Single-Agent Docker Runner** | 5-7d | Copy-in/copy-out via `git clone --local` + `git diff`. True sandbox. |
| **`smelt init`** | 1d | Minimal `.smelt/config.toml`. No workflow schema yet. |
| **`--budget` Cost Guard** | 1d | 80% warning, SIGTERM at 100%, 5s grace, SIGKILL. |
| **Auto-PR** | 2-3d | Auth stays on host. `GITHUB_TOKEN` never enters container. |
| **Structured Run Results** | 0.5d | `.smelt/runs/<ts>/result.json` — duration, tokens, files changed. |

**Cut:** Agent adapter interface (premature with 1 agent), TUI (wrong timing), templates (no users yet).

[Full report](quickwins-report.md)

---

## High-Value Features (v1 — ~8-10 weeks)

| Feature | Scope | Priority |
|---|---|---|
| **Workflow-as-Code SDK** | 6wk | **V1 Core** — C#/.NET fluent API for typed DAGs. Linear pipelines + fan-out. |
| **Budget Controls & Structured Events** | 3-4wk | **V1 Core** — Per-workflow/agent budget limits, circuit breakers, OTel-ready events. |
| **Security Policy Basics** | 3-4wk | **V1 Core** — Declarative network egress + resource limits → Docker config. |
| **CLI Experience** | Continuous | **V1 Core** — `smelt init/run/status/logs`. First-class, not afterthought. |
| **Multi-Repo Workspace** | incl. in SDK | **V1 Core** — Design as multi-repo from day one. Retrofitting is architectural debt. |
| **Poll-Based GitHub Triggers** | 2-3wk | **V1 Stretch** — Labeled issues + PR events. Cut before SDK quality suffers. |
| **End-to-End Demo Workflow** | 1wk | **V1 Stretch** — Ship blocker. Infrastructure without a story is a demo. |

**Deferred:** OTel export (v1.5), cross-repo coordination (v2), webhooks (v2), workflow gates/conditionals (v2), context sharing (needs telemetry data), agent marketplace (needs stable interface + users), session replay (separate initiative).

[Full report](highvalue-report.md)

---

## Radical Directions (6-18 month horizon)

| Proposal | Seed Timeline | Status |
|---|---|---|
| **Codebase Digital Twin** | 3mo seed | **Approved** — Tree-sitter AST + graph store + execution history. Agents query instead of reading files. |
| **Graduated Autonomous Stewardship** | 3mo seed (Level 0) | **Approved** — Suggestion dashboard → auto-execute+PR → auto-merge → full autonomy. Trust earned, not granted. |
| **Protocol-Shaped Architecture** | Immediate | **Approved** — Design principle, not deliverable. All agent interfaces protocol-grade internally. |
| Adaptive Workflows | After Digital Twin | **Deferred** — Learn from execution history, suggest workflow improvements. |
| Temporal Debugging | After observability | **Deferred** — Checkpoint + replay. Speculative execution killed on economics. |
| Agent Mesh (P2P) | — | **Killed** — Coordination overhead exceeds work cost. Useful kernel folded into Digital Twin. |
| Workflow Genetics | — | **Killed** — No viable fitness function. IP/security concerns with cross-org sharing. |

[Full report](radical-report.md)

---

## Cross-Cutting Themes

1. **"Design for extensibility, build for today."** — Multi-repo in the core model from day one. Agent interface designed for future marketplace. But don't build the registry or cross-repo coordination until users demand it.

2. **"Credentials never enter the sandbox."** — Consistent across all three reports. All authenticated operations happen on the host side. The container is a pure compute sandbox.

3. **"No premature abstractions."** — Agent adapter interface deferred until agent #2 is actually added. Workflow templates deferred until users exist. Protocol publication deferred until interfaces are battle-tested.

4. **"Honest constraints beat false promises."** — Secret exfiltration via agent output is unsolved industry-wide. Token counting is imperfect. Document honestly rather than ship leaky abstractions.

5. **"The real competition is manual agent usage, not Axon."** — Axon validates the market. Smelt must first be dramatically better than "I'll just run Claude Code manually."

6. **"Accumulated operational intelligence"** — The radical thesis: Smelt's long-term moat is the flywheel of operational data (Digital Twin + execution history + adaptive learning). Competitors can copy features but not the accumulated data.

---

## Recommended Sequencing

```
v0.1 (2 weeks)     Docker runner + init + budget + auto-PR + run results
  ↓
v1.0 (8-10 weeks)  Workflow SDK + budget controls + security + CLI + multi-repo
  ↓
v1.5 (3 months)    Digital Twin seed + Level 0 Stewardship dashboard + OTel export
  ↓
v2.0 (6 months)    Cross-repo coordination + webhooks + workflow gates + adaptive workflows
  ↓
v3.0 (12+ months)  Graduated stewardship (Level 1-2) + protocol publication evaluation
```
