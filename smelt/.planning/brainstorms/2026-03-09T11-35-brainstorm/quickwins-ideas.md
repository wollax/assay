# Quick-Win Ideas for Smelt v0.1

Explorer: explorer-quickwins | Date: 2026-03-09

---

## 1. `smelt init` — Scaffolding CLI

**What**: A single CLI command that generates a `smelt.yaml` (or `smelt.toml`) workflow definition plus a `docker-compose.smelt.yml` with sensible defaults. Detects the repo language, infers a base image, and wires up a single-agent "fix this issue" workflow as the starter template.

**Why**: First-run experience is everything. Axon requires K8s knowledge upfront. If `smelt init` gets someone from zero to a running agent in <2 minutes on their laptop, that's the strongest differentiator we can ship. It also establishes the workflow-as-code format early, which everything else builds on.

**Scope**: ~2-3 days. CLI arg parsing, template generation, language detection heuristic. No orchestration engine needed — just file generation.

**Risks**: Committing to a config format too early. Mitigation: keep the schema minimal and version it (`version: 1`).

---

## 2. Single-Agent Docker Runner

**What**: The absolute minimum runtime: read a workflow definition, spin up one Docker container with the specified agent (Claude Code initially), mount the repo, inject the prompt, stream stdout/stderr, and collect the result. No fan-out, no multi-agent — just `smelt run`.

**Why**: This is the foundation everything else plugs into. Shipping this alone already beats "ssh into a box and run claude-code manually." It proves the core value prop (sandboxed agent execution) without the complexity of orchestration. Also directly comparable to Axon's single-agent-per-task model, but running locally on Docker instead of K8s.

**Scope**: ~3-5 days. Docker SDK integration, container lifecycle management, volume mounting, environment injection, output streaming.

**Risks**: Docker API surface is large — scope creep into networking, GPU passthrough, etc. Keep it to `docker run` equivalent first.

---

## 3. Agent Adapter Interface

**What**: Define a thin `IAgentAdapter` interface (or functional equivalent) that abstracts how Smelt talks to different coding agents. Ship adapters for Claude Code and one other agent (Codex or OpenCode) from day one.

**Why**: Multi-agent support is differentiator #4 vs Axon. But more importantly, defining the adapter interface early forces good architecture — it prevents coupling the runner to Claude Code internals. Even if users only use Claude Code initially, the interface signals "this tool is agent-agnostic" which is a strong positioning statement.

**Scope**: ~2-3 days. Interface definition, Claude Code adapter (wraps CLI), one additional adapter. The interface is small: `Start`, `SendPrompt`, `StreamOutput`, `GetResult`, `Stop`.

**Risks**: Getting the abstraction wrong — too thin and agents can't express capabilities, too thick and it's a maintenance burden. Start minimal, evolve.

---

## 4. Live Session TUI (Terminal UI)

**What**: A terminal dashboard (using Spectre.Console or similar) that shows real-time agent output, container status, token usage, and elapsed time while `smelt run` executes. Think `docker compose up` but with agent-aware formatting.

**Why**: Observability is differentiator #6 and developers love good TUIs. A live view of what the agent is doing builds trust and makes debugging easy. It's also highly demo-able — screenshots and recordings of a slick TUI spread organically. Low effort because Spectre.Console handles the hard parts.

**Scope**: ~2-3 days. Real-time log streaming panel, status bar with container state, token counter (if agent exposes it). Build on top of idea #2's output streaming.

**Risks**: Terminal compatibility across macOS/Linux/Windows terminals. Spectre.Console handles most of this. Also risk of over-investing in polish before core works — keep it functional, not fancy.

---

## 5. `smelt run --budget` Cost Guard

**What**: A simple flag that sets a token/cost ceiling for a run. When the limit is approached, the agent gets a "wrap up" signal; when hit, the container is killed. Configurable in the workflow file too.

**Why**: Cost control is the #1 anxiety for teams adopting AI agents. Axon has no cost controls. Even a basic budget guard makes Smelt feel production-safe. This is the kind of feature that unblocks enterprise adoption conversations ("yes, we have spend limits").

**Scope**: ~1-2 days on top of the runner (#2). Token counting from agent output parsing, threshold checks, graceful shutdown signal, hard kill timeout.

**Risks**: Token counting accuracy depends on agent output format — may need per-adapter parsing. Start with "approximate" and document it.

---

## 6. GitHub Integration: Auto-PR from Agent Output

**What**: After a successful agent run, automatically create a branch, commit the changes, and open a PR with the agent's summary as the PR description. `smelt run --pr` or configured in the workflow.

**Why**: The value of a coding agent is only realized when its output enters the development workflow. Manual "copy changes out of container" is friction that kills adoption. Auto-PR closes the loop and makes Smelt feel like a complete tool, not just an execution sandbox. Pairs perfectly with the assay project for PR review.

**Scope**: ~2-3 days. Git operations inside the container (or on mounted volume), GitHub API/`gh` CLI for PR creation, branch naming conventions, PR template.

**Risks**: Merge conflicts, dirty repo state, auth token management. Start with clean-repo-only and fail loudly on conflicts.

---

## 7. Workflow Templates Gallery

**What**: Ship 5-10 pre-built workflow templates covering common use cases: "fix GitHub issue", "add tests for file X", "refactor module", "update dependencies", "write docs for API". Accessible via `smelt init --template <name>` or listed with `smelt templates`.

**Why**: Templates lower the cognitive barrier from "what can I do with this?" to "pick one and go." They also serve as documentation-by-example for the workflow format. Every successful template run is a user who now understands the system. This is how Terraform, Next.js, and similar tools drive adoption.

**Scope**: ~1-2 days (assuming the workflow format from #1 exists). Markdown + YAML files, minimal CLI plumbing to list/apply them.

**Risks**: Templates that don't work are worse than no templates. Each one needs to be tested against a real repo. Start with 3-4 battle-tested ones rather than 10 untested ones.
