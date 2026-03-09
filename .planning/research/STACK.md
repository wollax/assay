# STACK — Technology Stack Research

**Dimension:** Stack
**Milestone:** v0.1.0 Orchestration PoC
**Date:** 2026-03-09

> **Verification note:** Context7, WebSearch, Ref, and WebFetch were unavailable during this research session. Versions listed are from training data (cutoff: May 2025). All version numbers are marked with `*` where they should be verified before finalizing the decision. Use `cargo search`, `npm view`, `dotnet nuget search`, or `go list -m` to confirm.

---

## 1. What Smelt Actually Needs

Before comparing languages, define the capability axes that matter for v0.1.0:

| Capability | Why | Weight |
|---|---|---|
| **Git worktree manipulation** | Core primitive. Create, list, remove, inspect worktrees programmatically. | Critical |
| **Merge/diff operations** | Merge worktree branches, detect conflicts, extract conflict hunks for AI resolution. | Critical |
| **Process orchestration** | Spawn, monitor, and kill long-running agent processes (Claude Code CLI). Capture stdout/stderr. | Critical |
| **CLI ergonomics** | `smelt run`, `smelt status`, etc. Argument parsing, colored output, progress indicators. | High |
| **JSON parsing** | Read Assay gate results, emit structured run records. | High |
| **Cross-platform** | macOS primary, Linux secondary (CI). Windows not required for v0.1.0. | Medium |
| **Ecosystem alignment** | Assay is Rust. Shared tooling, testing patterns, CI. | Medium |
| **Developer velocity** | Time to first working PoC. Iteration speed matters for v0.1.0. | High |
| **Error handling** | Worktree and merge ops have many failure modes. Must be expressible clearly. | Medium |

---

## 2. The Git Operations Question

This is the single most important axis. Smelt's core loop is:

```
create worktrees → spawn agents → wait → merge branches → resolve conflicts → clean up
```

### The Shell-Out vs. Library Spectrum

There are two strategies for git operations:

1. **Programmatic library** — Call git operations as library functions (e.g., `repo.create_worktree()`).
2. **Shell-out to `git` CLI** — Spawn `git worktree add`, `git merge`, etc. as subprocesses and parse output.

**Key insight: No git library in any language has complete worktree + merge + conflict-extraction support.** Every language will require some shell-out to `git` for at least one critical operation. The question is how much.

| Operation | Library coverage (any lang) | Shell-out needed? |
|---|---|---|
| `worktree add/list/remove` | Partial (Rust/Go have some) | Usually yes |
| `merge --no-commit` | Partial | Often yes |
| Conflict detection | Good (libgit2-based) | No |
| Conflict hunk extraction | Poor | Yes (or manual parse) |
| `diff --stat` / `diff` | Good | No |
| Branch create/checkout | Good | No |
| Status/clean check | Good | No |

**Recommendation: Design for a hybrid approach.** Use a git library for read operations (status, diff, branch inspection, conflict detection) and shell-out to `git` CLI for write operations that are underserved by libraries (worktree management, merge with conflict markers). Wrap both behind a `GitOps` trait/interface so the implementation can migrate to pure-library later.

---

## 3. Language Comparison

### 3.1 Rust

**Git libraries:**

| Library | Version* | Approach | Worktree | Merge | Maturity |
|---|---|---|---|---|---|
| **gix** (gitoxide) | ~0.68* | Pure Rust, reimplementation | Partial (read worktree list, basic add) | No merge engine yet | Active, rapidly evolving, API unstable |
| **git2** (libgit2 bindings) | ~0.19* | FFI to libgit2 | No worktree API | Basic merge (not 3-way with markers) | Stable but stagnant upstream |

- `gix`: Byron Bates' gitoxide project. Impressive scope — rewriting git in Rust. Worktree support is partial: can enumerate worktrees and do basic operations, but the merge engine (`gix-merge`) is incomplete. The API changes frequently (0.x semver). For Smelt's needs, `gix` would cover branch/ref operations and status, but worktree creation and merge would still need shell-out.
- `git2`: Bindings to libgit2 (C library). Stable API, but libgit2 itself never implemented `git worktree` support. Merge support exists but is low-level (you get conflict entries, not the full 3-way merge with markers that `git merge` produces). Also requires linking a C library (complicates cross-compilation).

**Process orchestration:**
- `tokio::process` — async process spawning, stdout/stderr capture, kill signals. Excellent.
- `nix` crate — POSIX signal handling (SIGTERM, SIGKILL). Mature.

**CLI:**
- `clap` ~4.5* — best-in-class CLI framework. Derive macros, subcommands, completions.
- `indicatif` — progress bars. `console` — colored output.

**JSON:** `serde` + `serde_json` — gold standard. Zero-cost deserialization.

**Verdict:** Strong for long-term, weaker for v0.1.0 velocity. The git library story is fragmented — `gix` is incomplete for Smelt's needs, `git2` lacks worktree support entirely. Shell-out to git is mandatory for core operations. Rust's compile times and borrow checker add friction during rapid prototyping.

---

### 3.2 C# / .NET

**Git libraries:**

| Library | Version* | Approach | Worktree | Merge | Maturity |
|---|---|---|---|---|---|
| **LibGit2Sharp** | ~0.30* | FFI to libgit2 | No worktree API | Basic merge | Maintenance mode — slow releases |

- LibGit2Sharp inherits libgit2's limitations: no worktree API, basic merge support. The library has been in maintenance mode for years. .NET 8+ support exists but is not actively evolving.
- There is no pure-.NET git implementation of any significance.

**Process orchestration:**
- `System.Diagnostics.Process` — built-in, synchronous and async. stdout/stderr capture, kill. Adequate but verbose.
- `CliWrap` ~3.7* — excellent wrapper. Fluent API for process spawning with piping, cancellation tokens, buffered/streaming output. **This is genuinely good.**

**CLI:**
- `System.CommandLine` ~2.0* — Microsoft's official CLI framework. Powerful but API has been in preview for years.
- `Spectre.Console` ~0.49* — rich console output, tables, progress, prompts. **Excellent.**

**JSON:** `System.Text.Json` — built-in, fast. `JsonSerializer` with source generators for AOT.

**Verdict:** LibGit2Sharp is the weakest git library of all candidates. Shell-out to git is mandatory for almost everything. CliWrap + Spectre.Console is an excellent CLI/process story. .NET's AOT compilation (`dotnet publish -r osx-arm64 --self-contained`) produces single-file binaries. Developer comfort is the main argument here, but the git library gap is the largest of any candidate.

---

### 3.3 TypeScript (Node.js)

**Git libraries:**

| Library | Version* | Approach | Worktree | Merge | Maturity |
|---|---|---|---|---|---|
| **simple-git** | ~3.27* | Shell-out wrapper | Yes (wraps `git worktree`) | Yes (wraps `git merge`) | Mature, actively maintained |
| **isomorphic-git** | ~1.27* | Pure JS reimplementation | No worktree | Limited merge | Browser-focused, not suited |

- **`simple-git` is the best fit for Smelt's git needs across all languages.** It is explicitly a typed wrapper around the `git` CLI. It already wraps `git worktree add/list/remove/prune`, `git merge`, `git diff`, `git status`, etc. The TypeScript types are good. Because it shells out to `git`, it gets 100% feature parity with whatever git version is installed — including worktrees, merge conflict markers, rerere, everything.
- `isomorphic-git` is designed for browser/service-worker use. No worktree support, limited merge. Not relevant.

**Process orchestration:**
- `execa` ~9.5* — modern process execution. Promise-based, piping, streaming, kill signals, timeout. **Excellent.**
- `node:child_process` — built-in, lower-level but capable.
- `zx` ~8.3* — Google's shell scripting toolkit. Good for scripts, too opinionated for a library.

**CLI:**
- `commander` ~13* / `yargs` ~17* — mature CLI frameworks.
- `@effect/cli` — newer, typed, declarative. Worth watching.
- `chalk` / `ora` / `cli-progress` — output formatting, spinners, progress.

**JSON:** Native. `JSON.parse`/`JSON.stringify`. TypeScript interfaces for Assay gate results.

**Verdict:** `simple-git` is the strongest git story because it embraces shell-out as a feature rather than fighting it. Process orchestration via `execa` is excellent. TypeScript provides type safety with faster iteration than Rust. The main weakness: Node.js single-threaded model means process orchestration is async-callback-heavy (though `execa` + `Promise.all` handles parallel agents well). No compile step for distribution — need to bundle (e.g., `pkg`, `esbuild`, or ship as npm package).

---

### 3.4 Go

**Git libraries:**

| Library | Version* | Approach | Worktree | Merge | Maturity |
|---|---|---|---|---|---|
| **go-git** | v5.13* | Pure Go reimplementation | Partial (Worktree type exists) | No merge engine | Active, Microsoft-backed |

- `go-git` has a `Worktree` type, but it represents the *working tree* of a repository, not `git worktree` (multiple working trees). This is a common source of confusion. Actual multi-worktree support (`git worktree add/list`) is not implemented.
- No merge engine. `go-git` explicitly documents that merge is not supported.
- Good for: clone, fetch, push, commit, log, diff, status.

**Process orchestration:**
- `os/exec` — built-in, synchronous (blocks goroutine, not thread). Simple API. Kill via `cmd.Process.Kill()`.
- Goroutines for parallelism — natural fit for "spawn N agents, wait for all".

**CLI:**
- `cobra` ~1.8* + `viper` — the standard. Used by kubectl, gh, docker.
- `charmbracelet/bubbletea` — TUI framework. `charmbracelet/lipgloss` — styled output.

**JSON:** `encoding/json` — built-in. Struct tags. Adequate.

**Verdict:** Go's process orchestration (goroutines + `os/exec`) is the most natural fit for "spawn and manage N concurrent processes." `cobra` is proven at scale. But the git library story is poor — `go-git` doesn't support worktrees or merge. Shell-out to git is mandatory for all core operations. Go produces excellent single-binary distributions. Error handling is verbose but explicit.

---

## 4. Comparison Matrix

| Axis | Rust | C# / .NET | TypeScript | Go |
|---|---|---|---|---|
| **Git worktree ops** | Shell-out required | Shell-out required | `simple-git` wraps it | Shell-out required |
| **Merge/conflict** | Shell-out required | Shell-out required | `simple-git` wraps it | Shell-out required |
| **Git read ops** (status, diff, log) | `gix` or `git2` | `LibGit2Sharp` | `simple-git` | `go-git` |
| **Process orchestration** | `tokio::process` (excellent) | `CliWrap` (excellent) | `execa` (excellent) | `os/exec` + goroutines (excellent) |
| **CLI framework** | `clap` (best-in-class) | `Spectre.Console` (excellent) | `commander` (good) | `cobra` (excellent) |
| **JSON** | `serde` (excellent) | `System.Text.Json` (good) | Native (excellent) | `encoding/json` (adequate) |
| **Single binary** | Yes (static) | Yes (AOT) | No (needs bundler or runtime) | Yes (static) |
| **Assay alignment** | Same language | Different | Different | Different |
| **Developer velocity** | Slow (compile, borrow checker) | Medium | Fast | Medium-Fast |
| **Error handling** | `Result<T, E>` (excellent) | Exceptions (adequate) | `try/catch` (adequate) | `if err != nil` (verbose) |
| **Ecosystem size** | Growing | Large | Largest | Large |

---

## 5. Recommendation: Rust

### Primary rationale

1. **Ecosystem alignment with Assay is the decisive factor.** Smelt must read Assay's output formats (gate results, specs, checkpoints). Sharing types — literally the same Rust structs with `serde::Deserialize` — eliminates an entire category of integration bugs. If Assay publishes an `assay-types` crate (which it should), Smelt can depend on it directly. With any other language, you're maintaining a parallel set of type definitions that drift.

2. **The git library gap is the same everywhere.** Every language requires shell-out for worktree and merge operations. Rust is not uniquely disadvantaged here. The `GitOps` trait approach (library for reads, shell-out for writes) works in any language — and in Rust, `tokio::process::Command` makes shell-out ergonomic with async/await.

3. **Process orchestration in Rust is excellent.** `tokio` is the most mature async runtime in any language. Spawning N agent processes, streaming their output, applying timeouts, and handling signals is well-trodden ground. The `tokio::select!` macro makes "wait for first completion or timeout" trivial.

4. **Long-term binary distribution.** Smelt is a CLI tool that users install. A single static binary (`cargo build --release`) is the gold standard for CLI distribution. No runtime, no node_modules, no framework dependency. TypeScript's bundling story (pkg, bun compile, esbuild + shebang) works but adds friction.

5. **v0.1.0 velocity concern is real but manageable.** Rust is slower for prototyping. Mitigation: keep v0.1.0 scope tight (it already is), lean on shell-out rather than fighting git libraries, use `anyhow` for error handling during prototyping (migrate to typed errors later).

### Why not TypeScript (the runner-up)

TypeScript with `simple-git` has the strongest git operations story. If Smelt were a standalone tool with no ecosystem ties, TypeScript would be the pick. But:

- **Type drift with Assay.** Maintaining TypeScript interfaces that mirror Rust structs is ongoing tax. Every Assay format change requires manual sync.
- **Distribution.** npm package works for developers but adds a Node.js runtime dependency. Bundled binaries (bun compile, pkg) are improving but not as clean as `cargo build`.
- **Concurrency model.** Node.js async is fine for I/O-bound work, but Smelt's "manage N agent processes with timeouts and signals" benefits from Rust's explicit concurrency model.

TypeScript is the right choice if velocity is the only priority and Assay integration is deferred.

### Why not C# / .NET

- LibGit2Sharp is the weakest git library of all candidates.
- No ecosystem alignment with Assay.
- .NET AOT single-binary distribution works but is less mature than Rust or Go.
- Developer comfort is real but doesn't outweigh the ecosystem gap.

### Why not Go

- `go-git` doesn't support worktrees or merge — the two core operations.
- No ecosystem alignment with Assay.
- Go's error handling (`if err != nil`) becomes genuinely painful in code with many fallible operations (which git orchestration is).
- Go would be the pick if Smelt were purely a process orchestrator with no git library needs (like Axon).

---

## 6. Recommended Stack (Rust)

### Core dependencies

| Crate | Version* | Purpose |
|---|---|---|
| `tokio` | ~1.43* | Async runtime, process spawning, timers, signals |
| `clap` | ~4.5* | CLI argument parsing with derive macros |
| `serde` + `serde_json` | ~1.0* / ~1.0* | JSON serialization for Assay gate results |
| `anyhow` | ~1.0* | Error handling (v0.1.0 — migrate to `thiserror` later) |
| `tracing` + `tracing-subscriber` | ~0.1* / ~0.3* | Structured logging |
| `console` | ~0.15* | Colored terminal output |
| `indicatif` | ~0.17* | Progress bars for long operations |
| `tempfile` | ~3.14* | Temporary directories for test worktrees |

### Git operations layer

```
┌──────────────────────────────────────────┐
│              SmeltGitOps trait            │
├──────────────────────────────────────────┤
│  worktree_add()    → shells out to git   │
│  worktree_list()   → shells out to git   │
│  worktree_remove() → shells out to git   │
│  merge_branch()    → shells out to git   │
│  conflict_list()   → gix or shell-out    │
│  conflict_hunks()  → parse conflict markers│
│  diff_stat()       → gix                 │
│  branch_create()   → gix                 │
│  status()          → gix                 │
└──────────────────────────────────────────┘
```

Use `tokio::process::Command` for shell-out. Wrap in a `GitCli` struct implementing `SmeltGitOps`. This allows:
- Swapping in `gix` native implementations as they mature
- Mocking for tests (the trait is the test seam)
- Testing against real git repos in integration tests using `tempfile::TempDir`

### Process orchestration layer

| Crate | Purpose |
|---|---|
| `tokio::process` | Spawn agent sessions (Claude Code CLI) |
| `tokio::sync::mpsc` | Agent event channels (started, output, completed, failed) |
| `tokio::time::timeout` | Per-session timeouts |
| `nix` ~0.29* | POSIX signals (SIGTERM graceful shutdown) |

### What NOT to add

| Don't add | Why |
|---|---|
| `git2` crate | libgit2 has no worktree support. Adding a C FFI dependency for operations `gix` can handle is not worth it. |
| `tui` / `ratatui` | No TUI in v0.1.0. CLI output only. |
| `reqwest` / HTTP client | No forge API calls in v0.1.0. Add when PR creation ships. |
| `sqlx` / database | Git is the coordination substrate. No database. |
| `bollard` / Docker | No container runtime in v0.1.0. Worktree isolation, not container isolation. |
| `async-trait` | Use `impl Trait` in async (stabilized in Rust 1.75+). Avoid the allocation overhead. |
| gRPC / tonic | No IPC protocol needed. Agents are spawned processes, not services. |

---

## 7. Build and Distribution

```toml
# Cargo.toml (skeleton)
[package]
name = "smelt"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"  # verify: latest stable as of March 2026*

[dependencies]
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
console = "0.15"
indicatif = "0.17"
tempfile = "3"
nix = { version = "0.29", features = ["signal", "process"] }

[dev-dependencies]
assert_cmd = "2"     # CLI integration testing
predicates = "3"     # Assertion matchers
insta = "1"          # Snapshot testing for git output
```

**Distribution:** `cargo build --release` produces a static binary. Publish via GitHub Releases (`.tar.gz` for macOS/Linux). Homebrew tap later.

---

## 8. Risk Register

| Risk | Impact | Mitigation |
|---|---|---|
| `gix` API instability (0.x) | Medium — read ops break on upgrade | Pin version, use sparingly, prefer shell-out for v0.1.0 |
| Shell-out to `git` is slow | Low — worktree ops are infrequent | Profile if it matters. Each `git` invocation is ~5-20ms. |
| Rust compile times slow iteration | Medium — v0.1.0 velocity | Use `cargo check`, incremental compilation, split into thin crates only if build exceeds 30s |
| `gix` never ships merge engine | Low — shell-out works indefinitely | The `SmeltGitOps` trait abstracts this. Shell-out is not a hack, it's the design. |
| Assay doesn't publish shared types crate | Low — can vendor types | Plan for `assay-types` crate. Worst case: copy the structs. |

---

## 9. Decision Summary

| Decision | Choice | Rationale |
|---|---|---|
| **Language** | Rust | Ecosystem alignment with Assay, single-binary distribution, excellent async process orchestration |
| **Git strategy** | Hybrid (shell-out primary, `gix` for reads) | No library covers worktrees + merge in any language. Shell-out behind a trait is the pragmatic design. |
| **Async runtime** | `tokio` | Industry standard, process spawning, timers, signals |
| **CLI framework** | `clap` (derive) | Best-in-class, generates completions and man pages |
| **Error handling** | `anyhow` (v0.1.0) → `thiserror` (v0.2.0+) | Velocity now, precision later |
| **IPC model** | Spawned processes, not services | Agents are CLI tools, not long-running servers. `tokio::process` is the IPC layer. |

---

*`*` = version from training data (May 2025 cutoff). Verify with `cargo search <crate>` before use.*
