# Assay -- Project Overview

> Agentic development kit with spec-driven workflows and dual-track quality gates for AI coding agents.

**Version:** 0.5.0
**Language:** Rust (stable, Edition 2024)
**License:** MIT
**Repository:** Monorepo with three parts -- Assay, Smelt, and Plugins

---

## Executive Summary

Assay is a Rust monorepo providing a three-layer toolkit for agentic software development. The **orchestration layer** (Assay) defines spec-driven workflows, quality gates, and multi-agent coordination. The **infrastructure layer** (Smelt) handles container provisioning, forge delivery, and SSH worker pools. The **context layer** (Cupel, external crate) provides token-budgeted context windowing for agent sessions.

The project ships two binaries (`assay-cli`, `assay-tui`), one daemon (`smelt-cli`), an MCP server with 29+ tools, and four plugins targeting Claude Code, Codex, OpenCode, and Smelt Agent.

---

## Technology Stack

| Category | Technology | Version | Purpose |
|---|---|---|---|
| Language | Rust | Edition 2024 | Primary language |
| Async runtime | tokio | 1 | Async runtime (full features) |
| HTTP server | axum | 0.8 | Signal endpoint, HTTP API |
| HTTP client | reqwest | 0.13 | Linear, GitHub, Smelt backends |
| MCP | rmcp | 0.17 | Model Context Protocol server |
| CLI | clap | 4 | CLI argument parsing (derive) |
| TUI | ratatui | 0.30 | Terminal UI rendering |
| Serialization | serde / serde_json | 1 | Serialize/Deserialize |
| Schema | schemars | 1 | JSON Schema generation |
| Schema registry | inventory | 0.3 | Compile-time schema registration |
| Context engine | cupel | 1.2.0 | Token-budgeted context windowing |
| Containers | bollard | 0.20 | Docker API client |
| Kubernetes | kube | 3 | K8s API client |
| Observability | opentelemetry | 0.31 | Metrics and tracing |
| Errors | thiserror / anyhow | 2 / 1 | Error handling |
| Testing | insta | 1.46 | Snapshot testing |

---

## Architecture Overview

**Crate dependency graph (layered DAG):**

```
assay-types
    |
assay-core
    |
    +-- assay-backends   (state backends: Linear, GitHub, SSH, Smelt)
    +-- assay-harness    (agent adapters: Claude, Codex, OpenCode)
    +-- assay-mcp        (MCP server, 29+ tools)
    +-- assay-cli        (CLI binary)
    +-- assay-tui        (TUI binary)
```

**Design principles:**

- **Sync-first core** -- `assay-types` and `assay-core` contain no async code. Async lives at the edges (backends, MCP, CLI).
- **Trait-based abstractions** -- `HarnessProvider`, `StateBackend`, `RuntimeProvider`, `GitOps`, `ForgeClient` define extension points.
- **Inventory-based schema registry** -- Domain types self-register via `inventory::submit!` at compile time.
- **Feature-gated orchestration** -- Optional capabilities (telemetry, specific backends) are behind Cargo features.

---

## Repository Structure

### Assay crates (`crates/`) -- 7 crates

| Crate | Purpose |
|---|---|
| `assay-types` | Shared serializable types (serde, schemars) |
| `assay-core` | Domain logic: specs, gates, reviews, workflows |
| `assay-backends` | State backend implementations (Linear, GitHub, SSH, Smelt) |
| `assay-harness` | Agent adapters (Claude, Codex, OpenCode) |
| `assay-mcp` | MCP server with 29+ tools and signal endpoint |
| `assay-cli` | CLI binary (clap) |
| `assay-tui` | TUI binary (ratatui) |

### Smelt crates (`smelt/crates/`) -- 2 crates

| Crate | Purpose |
|---|---|
| `smelt-core` | Docker/Compose/K8s container execution, forge delivery, git operations |
| `smelt-cli` | Daemon with TUI, HTTP API, SSH worker pools, queue persistence, tracker polling |

`smelt-core` depends on `assay-types` via path dependency for shared configuration types.

### Plugins (`plugins/`) -- 4 plugins

| Plugin | Scope |
|---|---|
| `claude-code` | Full integration: hooks, MCP config, 9 skills |
| `codex` | Skill-only integration |
| `opencode` | NPM package scaffolding |
| `smelt-agent` | Infrastructure skills |

---

## Ecosystem

```
+---------------------------------------------------+
|  Context: Cupel (external crate)                  |
|  Token-budgeted context windowing for agents      |
+---------------------------------------------------+
|  Orchestration: Assay                             |
|  Specs, gates, sessions, multi-agent coordination |
+---------------------------------------------------+
|  Infrastructure: Smelt                            |
|  Container provisioning, forge delivery, workers  |
+---------------------------------------------------+
```

- **Smelt** provisions isolated execution environments (Docker, Compose, K8s), manages SSH worker pools, and delivers artifacts to forges.
- **Assay** coordinates agent sessions through spec-driven workflows with dual-track quality gates (automated + review).
- **Cupel** (separate crate, `cupel = "1.2.0"`) provides context windowing with token budgets for agent interactions.

---

## Development

### Build system

All tasks use `just` (justfile at repo root):

| Command | Description |
|---|---|
| `just build` | Build all crates |
| `just test` | Run all tests |
| `just lint` | Run clippy on all crates |
| `just fmt` | Format code |
| `just ready` | Full pre-commit gate: fmt, lint, test, deny, check-plugin-version |
| `just build-assay` | Build assay crates only |
| `just build-smelt` | Build smelt crates only |
| `just test-assay` | Test assay crates only |
| `just test-smelt` | Test smelt crates only (Docker tests skip when Docker unavailable) |
| `just test-smelt-unit` | Test smelt crates, excluding Docker integration tests |

### CI

- **Forgejo CI** (`.forgejo/workflows/ci.yml`) -- primary CI, runs on push/PR to `main`
  - `check-assay`: runs `just ready` (fmt + lint + test + deny + plugin version check)
  - `check-smelt`: fmt + test + lint + deny
  - Plugin validation job
- **GitHub Actions** (`.github/workflows/release.yml`) -- release only, triggered by `v*` tags

### Release

Tag pushes (`v*`) trigger GitHub Actions to build release binaries:

| Target | OS |
|---|---|
| `x86_64-unknown-linux-gnu` | Ubuntu |
| `aarch64-unknown-linux-gnu` | Ubuntu (cross-compiled) |
| `x86_64-apple-darwin` | macOS |
| `aarch64-apple-darwin` | macOS |

Binaries for `assay-cli` and `assay-tui` are packaged and attached to GitHub Releases.

---

## Links to Detailed Docs

| Document | Path |
|---|---|
| Assay crate scan | [`docs/scan-assay.md`](scan-assay.md) |
| Smelt crate scan | [`docs/scan-smelt.md`](scan-smelt.md) |
| Plugin scan | [`docs/scan-plugins.md`](scan-plugins.md) |
| Project scan report | [`docs/project-scan-report.json`](project-scan-report.json) |
