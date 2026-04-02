# High-Value Feature Proposals for Smelt

## 1. Workflow-as-Code SDK with Typed DAG Composition

**What:** A C#/.NET SDK that lets users define multi-agent workflows as strongly-typed DAGs using a fluent/declarative API. Workflows compose steps (agent invocations, gates, fan-out/fan-in, conditionals) into reusable, testable pipelines. Think Temporal/Durable Functions meets AI orchestration.

```csharp
var workflow = Smelt.Workflow("migrate-api")
    .Step("analyze", claude => claude.Prompt("Analyze breaking changes in {repo}"))
    .FanOut("update-consumers", ctx => ctx.Output<string[]>("affected-services"),
        service => opencode => opencode.Prompt($"Update {service} for new API"))
    .Gate("human-review", Gate.PullRequestApproved)
    .Step("integration-test", codex => codex.Run("npm test"))
    .Aggregate("summary", results => claude.Prompt($"Summarize migration: {results}"));
```

**Why:** This is the **primary moat**. Axon has no workflow composition — it's one-task-at-a-time. Workflow-as-code with type safety, IDE autocompletion, and testability creates a developer experience that's extremely hard to replicate. It makes Smelt the "Terraform of AI agents" — infrastructure-as-code for agent orchestration. Users who build workflows in Smelt's SDK have massive switching costs.

**Scope:** Large (3-4 months). Core DAG engine, SDK surface, serialization/resumption, error handling, retry policies.

**Risks:**
- Workflow state management is genuinely hard (especially resumption after failure)
- API surface design is critical — bad ergonomics kill adoption
- May need to support multiple host languages eventually (TypeScript SDK)

---

## 2. Cross-Repo Coordination Engine

**What:** First-class support for workflows that span multiple repositories. A `Workspace` abstraction that can checkout N repos, maintain consistency constraints between them, and coordinate atomic cross-repo changes (e.g., update an API + all consumers + shared schema).

Features:
- Multi-repo workspace mounting in containers
- Dependency graph between repos (knows service A depends on library B)
- Atomic cross-repo PR groups (linked PRs that merge together or not at all)
- Monorepo-aware path scoping

**Why:** This is a **massive gap in the market**. Axon is explicitly single-repo per workspace. Most real enterprise work involves coordinated changes across services. Any team with microservices needs this. It's a natural extension of Smelt's container-based architecture (mount multiple repos into one workspace or fan out across containers).

**Scope:** Large (2-3 months). Workspace abstraction, git coordination, PR linking via GitHub API, dependency graph config.

**Risks:**
- Merge conflict resolution across repos is complex
- Atomic cross-repo merges aren't natively supported by GitHub — need careful orchestration
- Dependency graph maintenance could become a burden on users

---

## 3. Agent Marketplace & Plugin Registry

**What:** A registry where users can publish and consume reusable agent definitions, workflow templates, and tool integrations. Ships with first-party agents for common tasks (PR review, test generation, dependency updates, security scanning, documentation). Community can contribute custom agents with standardized interfaces.

Components:
- Agent definition format (container image + prompt template + tool config)
- Version-pinned agent references in workflows
- Private registries for enterprise
- Curated "starter kit" workflows

**Why:** Network effects create the strongest moat. Once the ecosystem has 50+ quality agents and workflow templates, users choose Smelt because the ecosystem exists — not just the runtime. This is the Docker Hub / Terraform Registry play. Axon has no ecosystem story.

**Scope:** Medium-Large (2-3 months for MVP registry + 5-10 first-party agents). Ongoing curation effort.

**Risks:**
- Cold start problem — registry is useless until it has content
- Quality control for community contributions
- Versioning and compatibility across agent updates
- Security concerns with running community-contributed container images

---

## 4. Real-Time Observability & Cost Control Dashboard

**What:** A web dashboard and CLI providing real-time visibility into running workflows: token usage per agent/step, cost accumulation with budget limits and circuit breakers, session recording/replay for debugging, structured output extraction, and workflow topology visualization.

Features:
- Live token/cost meters per agent and workflow
- Budget policies (hard limits, soft warnings, per-workflow caps)
- Session recording with full replay (inputs, outputs, tool calls, timing)
- Structured output extraction (parse agent responses into typed data)
- Workflow DAG visualization with live status
- Cost attribution per team/project

**Why:** AI agent costs are opaque and scary for engineering managers. Nobody wants to deploy autonomous agents that could burn $500 on a hallucination loop. Budget controls + visibility make Smelt **enterprise-safe**. Session replay is critical for debugging non-deterministic agent behavior. Axon has zero observability beyond kubectl logs.

**Scope:** Medium (2 months). Event streaming infrastructure, web UI, budget policy engine, recording/replay.

**Risks:**
- Building a good web UI is a significant effort
- Token counting accuracy varies across providers
- Session replay storage could grow large
- Real-time streaming adds infrastructure complexity

---

## 5. Intelligent Context Sharing & Memory

**What:** A shared context layer that allows agents within a workflow to efficiently share knowledge without re-processing. Includes:
- Artifact store (agents produce typed artifacts that downstream agents consume)
- Shared vector index for codebase understanding (index once, query from any agent)
- Conversation memory that persists across workflow runs (learn from past executions)
- Smart context windowing (automatically select relevant context for each agent's task)

**Why:** Current multi-agent systems waste massive tokens re-reading the same files. A shared context layer means the "analyze" step's understanding flows to the "implement" step without redundant processing. This directly reduces costs (fewer tokens) and improves quality (consistent understanding). It's architecturally complex — hard for competitors to bolt on later.

**Scope:** Large (3 months). Artifact store, vector indexing, context selection algorithms, persistence layer.

**Risks:**
- Vector indexing quality varies — bad embeddings = bad context selection
- Context window management is genuinely hard to get right
- Stale context from previous runs could mislead agents
- Storage and indexing infrastructure costs

---

## 6. GitHub-Native Integration & Event-Driven Triggers

**What:** Deep GitHub integration that makes Smelt workflows trigger automatically from repository events: PR opened, issue labeled, release published, CI failed, code review requested. Goes beyond Axon's basic issue polling to support the full GitHub event taxonomy with conditional routing.

Features:
- GitHub App / webhook receiver
- Event-to-workflow routing with filters and conditions
- PR-aware workflows (auto-review, auto-fix CI failures, auto-update deps)
- Issue triage workflows (label, assign, break down, create sub-tasks)
- Status checks integration (Smelt workflows as required checks)
- GitLab/Azure DevOps adapters for enterprise

**Why:** Event-driven triggers transform Smelt from "tool you invoke" to "automation that runs itself." This is the path to production deployment. Axon's TaskSpawner only polls issues — Smelt can react to any event with sophisticated routing. The multi-provider story (GitHub + GitLab + Azure DevOps) captures enterprise users locked into specific platforms.

**Scope:** Medium (2 months). Webhook infrastructure, event routing, GitHub App, provider adapters.

**Risks:**
- Webhook reliability and retry handling
- Security of webhook endpoints
- Rate limiting from GitHub API
- Supporting multiple Git providers multiplies maintenance

---

## 7. Sandboxed Execution with Security Policies

**What:** A comprehensive security model for running untrusted AI agents in production. Includes network policies (agents can only access specified URLs), filesystem policies (read-only mounts, no access outside workspace), resource limits (CPU/memory/time caps), secret management (inject credentials without exposing to agents), and audit logging.

Features:
- Declarative security policies per agent/workflow
- Network egress rules (allowlist URLs, block internet)
- Filesystem isolation with granular mount policies
- Secret injection via environment variables or mounted files (never in prompts)
- Resource quotas with automatic termination
- Full audit trail of agent actions

**Why:** Enterprise adoption requires security guarantees. Nobody will run AI agents in production against real codebases without sandboxing. Docker gives us the primitives — Smelt adds the policy layer that makes it enterprise-grade. This is table stakes for production use but surprisingly absent from current tools. Axon relies on basic K8s pod security which is coarser-grained.

**Scope:** Medium (6-8 weeks). Policy definition format, Docker security config generation, secret management integration, audit logging.

**Risks:**
- Security is hard to get right — any hole is a critical vulnerability
- Overly restrictive defaults will frustrate users
- Secret management integration varies across enterprises
- Testing security policies thoroughly is time-consuming
