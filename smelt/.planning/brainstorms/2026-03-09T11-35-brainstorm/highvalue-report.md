# High-Value Features — Consolidated Report

> Explorer: explorer-highvalue | Challenger: challenger-highvalue
> Debate rounds: 3 | Status: Converged

## Executive Summary

Seven high-value features were proposed and pressure-tested through structured debate. The core finding: **ruthlessly scope a v1 that proves the workflow-as-code thesis in 8-10 weeks, with a clear graduation path to enterprise features.** Three proposals survived as v1 core, two were deferred, and two were cut from the near-term roadmap entirely.

The primary competitive moat is not any single feature — it's the combination of workflow-as-code + Docker-first accessibility + multi-agent orchestration that no competitor offers today.

---

## Proposals — Final Dispositions

### V1 CORE

#### 1. Workflow-as-Code SDK (MVP)

**What:** A C#/.NET SDK for defining multi-agent workflows as typed DAGs using a declarative/fluent API. V1 scope: linear pipelines + simple fan-out. No gates, conditionals, or workflow resumption in v1.

**Why this is the moat:** Once users define workflows in Smelt's SDK, switching costs are enormous. The typed, composable, testable developer experience is the "Terraform for AI agents" play. Axon has no workflow composition — it's one-task-at-a-time.

**Key design decisions from debate:**
- MVP is linear pipelines + simple fan-out. Gates, conditionals, and resumption are v2.
- V1 workflows are short-lived (minutes), which sidesteps the resumption-after-failure problem.
- Multi-repo workspace mounting is a v1 inclusion (see below) — `Workspace` accepts a list of repos from day one, not retrofitted later.
- Pipeline artifacts (typed data flow between steps) are built into the SDK, not a separate system. Agent A produces output X, Agent B consumes X — this is the pipeline model.
- **Agent interface abstraction must be designed for future extensibility** (container image + prompt template + tool config + security policy), even though the marketplace is deferred. Get this right now to avoid breaking changes later.
- No TypeScript SDK. Nail C# first.

**Scope:** 6 weeks. This is the largest and most critical work stream.

**Risks:**
- Execution semantics under the fluent API are complex (container lifecycle, fan-out parallelism, failure handling)
- API surface design is make-or-break for adoption — bad ergonomics kill the product
- 6 weeks is aggressive; quality must not be sacrificed for schedule

#### 2. Budget Controls & Structured Event Logging

**What:** Per-workflow and per-agent budget limits with circuit breakers (hard stop when $X exceeded), cost attribution, and structured event emission from every agent action. CLI-based session log viewer for debugging.

**Why:** The single most reassuring thing you can tell an engineering manager is "your agents can't spend more than $X per workflow run." Budget controls are the enterprise unlock. Structured events are the foundation for all future observability (OTel export, dashboards, replay).

**Key design decisions from debate:**
- **No custom web UI.** Export to OTel/Grafana/Datadog via structured events. TUI dashboard for local-first story.
- Session replay is a separate future initiative — not bundled here.
- Minimal CLI log viewer (structured log output, not full replay) ships in v1. Low effort, high debugging value.
- Token counting requires provider-specific adapters. Start with Claude/Anthropic, add others as agents are added.

**Scope:** 3-4 weeks, parallelizable with workflow SDK development.

**Risks:**
- Token counting accuracy varies across providers and streaming responses
- Budget enforcement granularity (per-step vs per-workflow vs per-run) needs clear design

#### 3. Security Policy Basics

**What:** Declarative security policies translated into Docker container configurations. V1 scope: network egress rules (allowlist/denylist) and resource limits (CPU, memory, time caps).

**Architecture (resolved through debate):**
- Workflow SDK *declares* security policies
- Thin policy translator *generates* Docker container configurations
- Docker runtime *enforces* them
- Three layers, clean separation — defense-in-depth

**Why:** Enterprise table stakes. Nobody deploys autonomous AI agents without sandboxing. Docker provides the enforcement primitives; Smelt adds the policy declaration layer.

**Key design decisions from debate:**
- Security enforcement is a **thin standalone layer**, not deeply coupled to the workflow SDK. If the workflow engine has bugs, security enforcement still holds at the container level.
- Secret management is deferred. The honest v1 position: "Smelt prevents agents from accessing the network; it cannot prevent agents from including secrets in their outputs." This is an unsolved industry problem — honest documentation beats false promises.
- Policy-as-code shares the workflow definition model but enforces independently.

**Scope:** 3-4 weeks, parallelizable.

**Risks:**
- Security is hard to get right — any hole is critical
- Overly restrictive defaults frustrate users
- Secret exfiltration via agent output is an acknowledged unsolved constraint

#### 4. CLI Experience

**What:** First-class CLI as the primary user interface: `smelt init`, `smelt run`, `smelt status`, `smelt logs`.

**Key decision from debate:** This is a first-class work stream, not an afterthought. The CLI should be usable by week 3 with a single hardcoded workflow, then progressively support more features as the SDK matures. If the CLI is clunky, nothing else matters.

**Scope:** Continuous, weeks 1-8.

#### 5. Multi-Repo Workspace Mounting (part of Workflow SDK)

**What:** Workspace abstraction supports mounting multiple repos into a single agent container. No cross-repo awareness — the agent sees a filesystem with multiple directories. Coordinated (not atomic) PR creation at the workflow level.

**Why included in v1:** This is an architectural decision, not a feature. If the workspace abstraction is designed as single-repo from the start, retrofitting multi-repo later means changing the core model. Designing it as multi-repo from day one costs ~2 weeks of incremental work and immediately differentiates from Axon's single-repo constraint.

**Key constraint:** No cross-repo awareness in v1. No dependency graphs, no atomic merges. The agent sees mounted directories. Cross-repo coordination is a workflow composition concern handled by the user.

---

### V1 STRETCH

#### 6. Poll-Based GitHub Triggers

**What:** Watch for GitHub events via API polling. Scoped to labeled issues + PR events. Configurable polling intervals with smart backoff. Event deduplication via local watermark persistence.

**Why in v1 (not v2):** Without any trigger mechanism, every workflow requires manual invocation, which caps the value proposition at "slightly easier than running agents manually." Poll-based triggers are cheap (a polling loop + event filtering) and transform Smelt from "tool I invoke" to "automation I deploy."

**Key design decisions from debate:**
- GitHub-only. GitLab/Azure DevOps deferred indefinitely until demand exists.
- Scoped to labeled issues + PR events for v1. Not the full GitHub event taxonomy.
- Must account for GitHub API rate limits: 5,000 req/hr authenticated. At 1-minute intervals across 10 repos × 5 event types = 3,000 req/hr. Configurable intervals and clear rate limit documentation required.
- This is explicitly a **stretch goal** — if weeks 1-6 take longer, triggers are cut before workflow SDK quality is sacrificed.
- Poll → webhook graduation path for future deployed mode.

**Scope:** 2-3 weeks (weeks 8-10, realistically).

**Risks:**
- GitHub API rate limit consumption
- Event deduplication requires small persistence layer
- Stretch goals often get cut — and that's OK

#### 7. End-to-End Demo Workflow

**What:** One compelling, real-world demo workflow that shows the full story: define a multi-step, multi-agent pipeline in C#, run it against a real repo, see costs, see security limits enforced.

**Why a ship blocker:** Without a compelling demo, you have infrastructure and no story. This is product work, not engineering work, but it's essential for adoption.

**Scope:** 1 week (week 7-8).

---

### V1.5 / V2

| Feature | Timing | Notes |
|---|---|---|
| OTel Metrics Export | v1.5 | Builds on structured events foundation from v1 |
| Cross-Repo Coordination | v2 | Dependency graphs, linked/coordinated PRs |
| Webhook Receiver | v2 | Deployed mode — lightweight container in compose stack |
| Workflow Gates & Conditionals | v2 | Extends workflow SDK beyond linear + fan-out |
| Workflow Resumption | v2 | Required for long-running workflows |

### BACKLOG (Needs Preconditions)

| Feature | Precondition | Notes |
|---|---|---|
| Context Sharing & Memory | Real workflow telemetry showing token waste patterns | Vector indexing is a science project without defined success criteria. Observe the problem before designing the solution. |
| Agent Marketplace | Stable agent interface + ≥50 real users | Cold start problem, unstable API, security liability all compound. Design the interface to BE extensible now; build the registry when users ask for it. |
| Session Recording/Replay | Structured events foundation + user demand | Essentially building a debugging IDE. Separate initiative from cost monitoring. |

---

## V1 Schedule Overview

| Week | Workflow SDK | Budget/Observability | Security | CLI | Triggers |
|---|---|---|---|---|---|
| 1-2 | Core pipeline model, agent interface | — | — | `smelt init` scaffold | — |
| 3-4 | Container lifecycle, step execution | Budget circuit breakers | Network egress rules | `smelt run` (single workflow) | — |
| 5-6 | Fan-out, multi-repo mounting, artifacts | Structured event logging | Resource limits | `smelt status`, `smelt logs` | — |
| 7-8 | Stabilization | CLI log viewer | Integration testing | Polish | End-to-end demo |
| 8-10 | — | — | — | — | Poll-based GitHub triggers (stretch) |

---

## Key Strategic Insights from Debate

1. **The real competition is manual agent usage, not Axon.** Axon validates the market; Smelt captures the broader market via accessibility (Docker vs K8s) and capability (workflows vs tasks). But the product must first be dramatically better than "I'll just run Claude Code manually."

2. **Moats only matter if you have users to retain.** Several proposals (marketplace, context sharing) were justified as long-term moats but deferred because a zero-user product needs to focus on the experience gap, not competitive defense.

3. **Design for extensibility, build for today.** The agent interface should accommodate future marketplace/community agents, but don't build the registry. Multi-repo mounting should be in the core model, but don't build cross-repo coordination. Get the abstractions right; defer the features.

4. **Honest constraints beat false promises.** Secret exfiltration is unsolved. Token counting is imperfect. Document these honestly rather than shipping leaky abstractions that create false confidence.

5. **8-10 weeks to a shippable product.** The forcing function of "ship something usable" prevents death-by-ambition. A solid workflow engine without triggers is a product. Triggers without a solid engine is a demo.
