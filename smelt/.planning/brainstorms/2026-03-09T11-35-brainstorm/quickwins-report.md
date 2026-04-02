# Quick-Wins Report: Smelt v0.1

**Date:** 2026-03-09
**Participants:** explorer-quickwins, challenger-quickwins
**Rounds:** 3 (proposal → critique → refinement → alignment)

---

## Executive Summary

From 7 initial proposals, debate narrowed the scope to **4.5 focused features deliverable in ~10-13 days**. Three ideas were cut as premature (agent adapter interface, TUI, templates gallery). The surviving features form a coherent MVP: run a coding agent in a sandboxed container, control costs, and get the output into a PR.

---

## Approved Features (in build order)

### 1. Single-Agent Docker Runner (~5-7 days)

The core of Smelt. Reads config, spins up a Docker container with Claude Code, runs a prompt, streams output.

**Key decisions:**
- **Mount strategy: copy-in/copy-out.** `git clone --local` into the container (fast, uses hardlinks). After the run, `git diff` extracts the changeset. True sandboxing — agent cannot modify the host repo.
- **Agent integration:** Claude Code built directly into the runner. No abstraction layer yet.
- **Output:** Structured plain-text with `[smelt]` prefixed status lines and ANSI colors. CI-friendly by default, no TUI.
- **Container lifecycle:** Proper cleanup on SIGINT/SIGTERM. No zombie containers.
- **Workspace requirement:** Git repos only for v0.1. Non-git workspaces deferred.

### 2. `smelt init` — Slim Scaffolding (~1 day)

Generates a minimal `.smelt/config.toml` with:
- Agent type (default: `claude-code`)
- Base image (auto-detected from repo language)
- Prompt source (inline string or file path)

**No workflow schema.** The full workflow-as-code format comes later when multi-agent orchestration exists to express. This avoids premature format lock-in while still providing a first-run experience.

### 3. `--budget` Cost Guard (~1 day)

Token/cost ceiling per run. Simple implementation:
- Warning logged at 80% of budget
- SIGTERM at 100%, 5-second grace period, then SIGKILL
- Partial output captured on kill
- No "graceful wrap-up" signaling for v0.1

Differentiator: Axon has no cost controls. This feature alone unblocks enterprise adoption conversations.

### 4. Auto-PR from Agent Output (~2-3 days)

After a successful run, automatically creates a branch, applies the diff, and opens a PR with the agent's summary as description.

**Key decisions:**
- **Auth stays on the host.** `GITHUB_TOKEN` is never injected into the container. PR creation runs outside the sandbox via `gh` CLI or GitHub API.
- **Diff source:** Directly from the copy-out `git diff` (same mechanism as the runner's change extraction).
- **Scope:** Clean repos only for v0.1. Fail loudly on conflicts.

### 5. Structured Run Results (~0.5 days)

After each run, write `.smelt/runs/<timestamp>/result.json` with:
- Duration, exit code, agent type
- Token usage estimate
- Files changed count
- Schema inspired by existing CI conventions (GitHub Actions step outputs / OpenTelemetry spans) — don't invent a new format.

Foundation for future observability, TUI, and dashboards. Immediately useful for CI integration.

---

## Deferred Features

| Feature | Why Deferred | Revisit Trigger |
|---|---|---|
| **Agent Adapter Interface** (#3) | Premature abstraction from sample size of 1. Different agents (Claude Code, Codex, OpenCode) have fundamentally different interaction models. | When adding agent #2 — extract interface from real observed differences. |
| **Live Session TUI** (#4) | Two output modes (TUI + plain text) from day one is unnecessary. TUI is useless in CI/headless. Token display couples to budget feature. | v0.2, after runner is stable and structured output format is proven. |
| **Workflow Templates Gallery** (#7) | Templates for a workflow format that doesn't exist yet. QA overhead on "battle-tested" templates is real. Early adopters don't need hand-holding. | When 10+ users are asking "what do I do with this?" Ship 1 example in README for now. |

---

## Key Architectural Decisions

1. **Copy-in/copy-out over host mounts** — Sandboxing wins over convenience for v0.1. The performance cost (clone time) is acceptable; the security and correctness benefits are significant.

2. **No premature abstractions** — Claude Code integration is built directly into the runner. Interface extraction happens when a second agent is actually added, informed by real differences.

3. **Git-repo-only workspaces** — Simplifies diffing, cloning, and PR creation. Non-git support is a future concern.

4. **Credentials never enter the sandbox** — All authenticated operations (PR creation, pushing) happen on the host side. The container is a pure compute sandbox.

5. **CI-first output** — Plain text with structured prefixes. Interactive enhancements (TUI) layer on top later.

---

## Sequencing and Dependencies

```
#2 Docker Runner (5-7d)
  ├── #1 Init slim (1d, can start day 2 in parallel)
  ├── #5 Budget guard (1d, after runner streams output)
  ├── #5.5 Run results (0.5d, after runner completes runs)
  └── #6 Auto-PR (2-3d, after runner produces diffs)
```

Critical path: #2 → #6 (~8-10 days)
Parallel work: #1 can start once config format is sketched (day 2). #5 and #5.5 slot in once the runner's execution loop works.

**Total estimate: 10-13 days for a shippable v0.1.**
