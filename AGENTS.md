# Assay — Agent Instructions

You are working within the Assay monorepo, an agentic development kit built in Rust. The repo contains two projects: **assay** (spec-driven dev kit) and **smelt** (infrastructure layer for container-based job execution).

## Build Commands

- `just build` — build all crates (both projects)
- `just test` — run all tests (both projects)
- `just lint` — run clippy (both projects)
- `just fmt` — format code
- `just ready` — run all checks (fmt, lint, test, deny, check-plugin-version)
- `just build-assay` — build assay crates only
- `just build-smelt` — build smelt crates only
- `just test-assay` — test assay crates only
- `just test-smelt` — test smelt crates only (Docker integration tests skip gracefully when Docker is unavailable)
- `just test-smelt-unit` — test smelt crates only, explicitly excluding `docker_lifecycle` integration tests
- `just lint-assay` — lint assay crates only
- `just lint-smelt` — lint smelt crates only

## Workspace Layout

Root `Cargo.toml` declares `members = ["crates/*", "smelt/crates/*"]`.

### Assay crates (`crates/`)

- `crates/assay-types` — shared serializable types (serde, schemars)
- `crates/assay-core` — domain logic (specs, gates, reviews, workflows)
- `crates/assay-backends` — state backend implementations (Linear, GitHub, Smelt, SSH)
- `crates/assay-harness` — single-agent harness for running specs
- `crates/assay-mcp` — MCP server with signal endpoint
- `crates/assay-cli` — CLI binary (clap)
- `crates/assay-tui` — TUI binary (ratatui)

### Smelt crates (`smelt/crates/`)

- `smelt/crates/smelt-core` — infrastructure layer: Docker/Compose/K8s job execution, tracker, forge delivery
- `smelt/crates/smelt-cli` — daemon binary with TUI, HTTP API, SSH worker pools, queue persistence

### Other

- `plugins/` — plugin packages for agentic AI systems (claude-code, opencode, smelt-agent)

## Cross-Project Dependencies

- `smelt-core` depends on `assay-types` via path dep (`path = "../../../crates/assay-types"`) for `StateBackendConfig` and related types
- Changes to `assay-types` may require corresponding updates in smelt-core

## Conventions

- Lean towards functional and declarative patterns
- Use workspace dependencies defined in the root `Cargo.toml`
- Types shared between crates belong in `assay-types`
- Business logic belongs in `assay-core`
- Binary crates are thin wrappers that delegate to `assay-core`
- Run `just ready` before considering work complete

<!-- rtk-instructions v2 -->
# RTK (Rust Token Killer) - Token-Optimized Commands

## Golden Rule

**Always prefix commands with `rtk`**. If RTK has a dedicated filter, it uses it. If not, it passes through unchanged. This means RTK is always safe to use.

**Important**: Even in command chains with `&&`, use `rtk`:
```bash
# ❌ Wrong
git add . && git commit -m "msg" && git push

# ✅ Correct
rtk git add . && rtk git commit -m "msg" && rtk git push
```

## RTK Commands by Workflow

### Build & Compile (80-90% savings)
```bash
rtk cargo build         # Cargo build output
rtk cargo check         # Cargo check output
rtk cargo clippy        # Clippy warnings grouped by file (80%)
rtk tsc                 # TypeScript errors grouped by file/code (83%)
rtk lint                # ESLint/Biome violations grouped (84%)
rtk prettier --check    # Files needing format only (70%)
rtk next build          # Next.js build with route metrics (87%)
```

### Test (90-99% savings)
```bash
rtk cargo test          # Cargo test failures only (90%)
rtk vitest run          # Vitest failures only (99.5%)
rtk playwright test     # Playwright failures only (94%)
rtk test <cmd>          # Generic test wrapper - failures only
```

### Git (59-80% savings)
```bash
rtk git status          # Compact status
rtk git log             # Compact log (works with all git flags)
rtk git diff            # Compact diff (80%)
rtk git show            # Compact show (80%)
rtk git add             # Ultra-compact confirmations (59%)
rtk git commit          # Ultra-compact confirmations (59%)
rtk git push            # Ultra-compact confirmations
rtk git pull            # Ultra-compact confirmations
rtk git branch          # Compact branch list
rtk git fetch           # Compact fetch
rtk git stash           # Compact stash
rtk git worktree        # Compact worktree
```

Note: Git passthrough works for ALL subcommands, even those not explicitly listed.

### GitHub (26-87% savings)
```bash
rtk gh pr view <num>    # Compact PR view (87%)
rtk gh pr checks        # Compact PR checks (79%)
rtk gh run list         # Compact workflow runs (82%)
rtk gh issue list       # Compact issue list (80%)
rtk gh api              # Compact API responses (26%)
```

### JavaScript/TypeScript Tooling (70-90% savings)
```bash
rtk pnpm list           # Compact dependency tree (70%)
rtk pnpm outdated       # Compact outdated packages (80%)
rtk pnpm install        # Compact install output (90%)
rtk npm run <script>    # Compact npm script output
rtk npx <cmd>           # Compact npx command output
rtk prisma              # Prisma without ASCII art (88%)
```

### Files & Search (60-75% savings)
```bash
rtk ls <path>           # Tree format, compact (65%)
rtk read <file>         # Code reading with filtering (60%)
rtk grep <pattern>      # Search grouped by file (75%)
rtk find <pattern>      # Find grouped by directory (70%)
```

### Analysis & Debug (70-90% savings)
```bash
rtk err <cmd>           # Filter errors only from any command
rtk log <file>          # Deduplicated logs with counts
rtk json <file>         # JSON structure without values
rtk deps                # Dependency overview
rtk env                 # Environment variables compact
rtk summary <cmd>       # Smart summary of command output
rtk diff                # Ultra-compact diffs
```

### Infrastructure (85% savings)
```bash
rtk docker ps           # Compact container list
rtk docker images       # Compact image list
rtk docker logs <c>     # Deduplicated logs
rtk kubectl get         # Compact resource list
rtk kubectl logs        # Deduplicated pod logs
```

### Network (65-70% savings)
```bash
rtk curl <url>          # Compact HTTP responses (70%)
rtk wget <url>          # Compact download output (65%)
```

### Meta Commands
```bash
rtk gain                # View token savings statistics
rtk gain --history      # View command history with savings
rtk discover            # Analyze Claude Code sessions for missed RTK usage
rtk proxy <cmd>         # Run command without filtering (for debugging)
rtk init                # Add RTK instructions to CLAUDE.md
rtk init --global       # Add RTK to ~/.claude/CLAUDE.md
```

## Token Savings Overview

| Category | Commands | Typical Savings |
|----------|----------|-----------------|
| Tests | vitest, playwright, cargo test | 90-99% |
| Build | next, tsc, lint, prettier | 70-87% |
| Git | status, log, diff, add, commit | 59-80% |
| GitHub | gh pr, gh run, gh issue | 26-87% |
| Package Managers | pnpm, npm, npx | 70-90% |
| Files | ls, read, grep, find | 60-75% |
| Infrastructure | docker, kubectl | 85% |
| Network | curl, wget | 65-70% |

Overall average: **60-90% token reduction** on common development operations.
<!-- /rtk-instructions -->
