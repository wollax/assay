# Phase 1: Project Bootstrap & Git Operations Layer - Research

**Researched:** 2026-03-09
**Confidence baseline:** Versions verified via `cargo search` (2026-03-09). Patterns verified via Context7 official docs. Assay patterns verified by reading source.

---

## Standard Stack

All versions verified against crates.io as of 2026-03-09. Rust toolchain: stable 1.93.1, edition 2024.

### Runtime Dependencies

| Crate | Version | Purpose | Confidence |
|---|---|---|---|
| `clap` | `4.5` | CLI framework (derive macros, subcommands, `--version`, `--help`) | HIGH — verified Context7 + Assay uses same |
| `tokio` | `1` | Async runtime. Phase 1 uses it minimally (for `main`); process spawning comes later. | HIGH — verified Context7 |
| `serde` | `1` | Serialization framework with derive | HIGH — verified crates.io 1.0.228 |
| `toml` | `1` | TOML serialization/deserialization. Serde-compatible. | HIGH — verified Context7 + crates.io 1.0.6 |
| `thiserror` | `2` | Derive macro for `Error` trait. Use from day 1 (Assay uses v2, not `anyhow`). | HIGH — Assay uses thiserror v2 |
| `anyhow` | `1` | Ergonomic error handling in CLI layer (`main`, command handlers) | HIGH — Assay CLI uses anyhow |
| `tracing` | `0.1` | Structured logging | HIGH — verified 0.1.44 |
| `tracing-subscriber` | `0.3` | Log output formatting with env-filter | HIGH — verified 0.3.22 |
| `console` | `0.16` | Colored terminal output, style helpers | HIGH — verified 0.16.2 |

### Dev Dependencies

| Crate | Version | Purpose | Confidence |
|---|---|---|---|
| `assert_cmd` | `2` | CLI integration testing (spawn binary, assert stdout/stderr/exit code) | HIGH — verified 2.1.2 |
| `predicates` | `3` | Assertion matchers for assert_cmd | HIGH — verified 3.1.4 |
| `tempfile` | `3` | Temporary directories for test repos | HIGH — verified 3.26.0 |
| `insta` | `1` | Snapshot testing for output formatting | MEDIUM — useful but optional for Phase 1 |

### Explicitly NOT Needed in Phase 1

| Crate | Why Not |
|---|---|
| `gix` | No git read operations needed yet — Phase 1 only shells out to `git` for basic ops |
| `indicatif` | No progress bars in Phase 1 — only `init` and status |
| `nix` | No signal handling needed yet — no child processes to manage |
| `git2` | Never — no worktree support, C FFI overhead, stagnant upstream |

---

## Architecture Patterns

### Pattern 1: Workspace Crate Layout (from Assay)

Assay uses a workspace with multiple crates: `assay-cli`, `assay-core`, `assay-types`, `assay-mcp`. Smelt should mirror this from day 1:

```
smelt/
  Cargo.toml          # workspace root
  crates/
    smelt-cli/        # Binary crate: CLI entry point, command handlers
      src/
        main.rs
        commands/
          mod.rs
          init.rs
    smelt-core/       # Library crate: SmeltGitOps trait, git operations, domain logic
      src/
        lib.rs
        error.rs
        git/
          mod.rs      # SmeltGitOps trait definition
          cli.rs      # Shell-out implementation
        init.rs       # .smelt/ directory creation
```

**Why workspace from the start:** Assay added crates incrementally and it works well. The CLI/core split enforces the boundary between "user interface" and "domain logic" — the trait lives in core, the command handlers live in CLI. Later phases add crates without restructuring.

**Confidence:** HIGH — directly observed in Assay source.

### Pattern 2: CLI Structure (from Assay)

Assay's CLI pattern, which Smelt should replicate exactly:

1. **Optional subcommand** via `Option<Commands>` in the derive struct
2. **No-args handler** checks project state: inside project = show status, outside = error + help
3. **`run()` returns `anyhow::Result<i32>`** (exit code), `main()` just prints errors and exits
4. **`Cli::try_parse().unwrap_or_else(|e| e.exit())`** — lets clap handle parse errors with colored output
5. **`#[command(propagate_version = true)]`** — `--version` works on all subcommands

Key Assay code pattern to replicate:

```rust
async fn run() -> anyhow::Result<i32> {
    let cli = Cli::try_parse().unwrap_or_else(|e| e.exit());
    match cli.command {
        Some(Command::Init { .. }) => { /* ... */ },
        None => {
            if smelt_dir_exists {
                show_status();
                Ok(0)
            } else {
                eprintln!("Not a Smelt project. Run `smelt init` to get started.");
                Cli::command().print_help()?;
                Ok(1)
            }
        }
    }
}
```

**Confidence:** HIGH — directly read from Assay `crates/assay-cli/src/main.rs`.

### Pattern 3: Error Type Design (from Assay)

Assay uses `thiserror` v2 with a `#[non_exhaustive]` enum in `core`. Key patterns:

- **Contextual errors** with operation + path + source: `#[error("{operation} at \`{path}\`: {source}")]`
- **Convenience constructors**: `AssayError::io("reading config", path, source)`
- **`Result<T>` type alias** in the error module
- **CLI layer uses `anyhow`** for ergonomic `?` chaining; core layer uses typed errors

Smelt should use the same dual-error pattern:
- `smelt-core` exports `SmeltError` (thiserror, typed, non-exhaustive)
- `smelt-cli` uses `anyhow::Result` for command handlers, converting `SmeltError` via `?`

**Confidence:** HIGH — directly read from Assay `crates/assay-core/src/error.rs`.

### Pattern 4: Git Operations Behind a Trait

The `SmeltGitOps` trait is the test seam for all git operations. Phase 1 needs only:

```rust
#[trait_variant::make(Send)]
pub trait GitOps {
    /// Find the repository root (git rev-parse --show-toplevel)
    async fn repo_root(&self) -> Result<PathBuf>;

    /// Check if a path is inside a git repository
    async fn is_inside_work_tree(&self, path: &Path) -> Result<bool>;

    /// Get current branch name
    async fn current_branch(&self) -> Result<String>;

    /// Get short HEAD commit hash
    async fn head_short(&self) -> Result<String>;
}
```

The concrete implementation (`GitCli`) shells out via `tokio::process::Command`:

```rust
pub struct GitCli {
    git_binary: PathBuf,  // resolved once at startup via `which`
    repo_root: PathBuf,   // cached after first discovery
}
```

**Why async trait:** `tokio::process::Command::output()` is async. The trait must be async to avoid blocking. Rust 1.75+ supports `async fn` in traits natively (no `async-trait` crate needed). Use `trait_variant` crate if `Send` bounds are needed on the future, or just add `+ Send` directly since Rust 1.93 supports it.

**Actually:** As of Rust 1.93, `async fn` in traits works directly. The compiler handles `Send` bounds via RPITIT. No extra crate needed. Just write `async fn` in the trait.

**Confidence:** HIGH for the pattern. MEDIUM for exact method signatures (Claude's discretion per CONTEXT.md).

### Pattern 5: Startup Sanity Checks

Per CONTEXT.md decisions, two checks must happen before any command runs:

1. **Git binary exists on `$PATH`** — Use the `which` crate (v8.0.2) to resolve `git` binary path. Fail with a clear message if not found.
2. **Inside a git repository** — Run `git rev-parse --show-toplevel`. Fail if exit code is non-zero.

These checks run in a `preflight()` function called before command dispatch, not inside individual commands. The resolved git binary path and repo root are then passed to `GitCli::new()`.

```rust
fn preflight() -> Result<(PathBuf, PathBuf)> {
    let git_binary = which::which("git")
        .map_err(|_| SmeltError::GitNotFound)?;

    let output = std::process::Command::new(&git_binary)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| SmeltError::GitExecution { ... })?;

    if !output.status.success() {
        return Err(SmeltError::NotAGitRepo);
    }

    let repo_root = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    Ok((git_binary, repo_root))
}
```

**Note:** Preflight is synchronous (uses `std::process::Command`) because it runs before the tokio runtime needs to be fully engaged. This avoids needing async in the very first startup path. The rest of `GitCli` uses `tokio::process::Command`.

**Confidence:** HIGH — `which` crate verified at 8.0.2, `git rev-parse --show-toplevel` is the standard idiom.

---

## Don't Hand-Roll

| Problem | Use Instead | Why |
|---|---|---|
| CLI argument parsing | `clap` derive | Generates help, completions, version, colored errors. Attempting manual parsing wastes weeks. |
| TOML serialization | `toml` crate with serde | Handles edge cases (escaping, datetime, inline tables). Writing a TOML serializer is a project in itself. |
| Error derive macros | `thiserror` | Generates `Display`, `Error`, `From` impls. Hand-rolling is verbose and error-prone. |
| Finding `git` on PATH | `which` crate | Cross-platform binary resolution. `std::process::Command::new("git")` doesn't verify existence before exec. |
| Terminal color detection | `console` crate | Handles `NO_COLOR`, `CLICOLOR`, pipe detection, Windows. Rolling your own misses half the edge cases. |
| Temporary test directories | `tempfile` crate | RAII cleanup, unique naming, race-free creation. `std::env::temp_dir()` + manual naming is fragile. |

---

## Common Pitfalls

### Pitfall 1: `git rev-parse --show-toplevel` fails in bare repos

**What happens:** If someone runs `smelt init` in a bare git repo, `--show-toplevel` returns an error. Bare repos have no working tree.

**Prevention:** Check the exit code and provide a specific error: "Smelt requires a git repository with a working tree (not a bare repository)."

**Verification:** Test with a bare repo (`git init --bare`).

**Confidence:** HIGH — well-known git behavior.

### Pitfall 2: `.smelt/` directory created but init fails partway

**What happens:** If `smelt init` creates `.smelt/` but crashes before writing the marker file, the directory exists but is incomplete. Subsequent `smelt init` sees the directory and refuses to overwrite. User is stuck.

**Prevention:** Assay's approach: create `.smelt/` directory atomically, then write files inside it. If any file write fails, remove the entire `.smelt/` directory (cleanup on error). Assay uses `std::fs::create_dir` which fails if the dir already exists — this is the correct "lock" mechanism.

**Verification:** Test init with a read-only parent directory to force mid-init failure.

**Confidence:** HIGH — observed in Assay's `init.rs` (lines 36-48).

### Pitfall 3: Colored output breaks piped/redirected output

**What happens:** ANSI escape codes appear in piped output (`smelt status | grep foo`), breaking downstream tools.

**Prevention:** The `console` crate's `Term` and `Style` types auto-detect TTY vs pipe. Additionally, respect the `NO_COLOR` environment variable (https://no-color.org/) and the `--no-color` flag. Wire `--no-color` to `console::set_colors_enabled(false)` early in startup.

**Verification:** Test output with `| cat` to verify no escape codes.

**Confidence:** HIGH — `console` 0.16 handles this.

### Pitfall 4: Edition 2024 gotchas

**What happens:** Rust 2024 edition (stable since 1.85) changes some behaviors:
- `unsafe_op_in_unsafe_fn` is deny-by-default (not relevant for Phase 1)
- `gen` is a reserved keyword
- Lifetime capture rules changed for `impl Trait` return types
- `tail_expr_drop_order` changes drop order in block tail expressions

**Prevention:** None of these significantly affect Phase 1 code. Just be aware that edition 2024 is the correct choice (Assay uses edition "2024" in workspace Cargo.toml — verified from the `resolver = "2"` workspace root; individual crate editions are workspace-inherited).

**Confidence:** HIGH — using stable 1.93.1 which fully supports edition 2024.

### Pitfall 5: `tokio::main` macro footgun with exit codes

**What happens:** If `main()` returns `Result`, Rust prints the `Debug` repr of the error, not the `Display` repr. This produces ugly output like `Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }`.

**Prevention:** Assay's pattern — `main()` matches on `run().await`, prints errors with `eprintln!("Error: {e:#}")` (the `#` uses alternate/pretty Display), and calls `std::process::exit(code)`. Never return `Result` from `main()`.

```rust
#[tokio::main]
async fn main() {
    let code = match run().await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            1
        }
    };
    std::process::exit(code);
}
```

**Confidence:** HIGH — directly observed in Assay.

### Pitfall 6: CI pipeline needs Rust toolchain caching

**What happens:** Rust builds are slow. Without caching, CI runs take 3-5 minutes for a trivial project, 10+ minutes as dependencies grow.

**Prevention:** Use `Swatinem/rust-cache@v2` in GitHub Actions. It caches `~/.cargo` and `target/` between runs. Also use `--locked` flag with `cargo build`/`cargo test` to ensure `Cargo.lock` is respected.

**Confidence:** HIGH — standard practice.

---

## Code Examples

### Example 1: Cargo.toml Workspace Root

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "MIT"
repository = "https://github.com/wollax/smelt"

[workspace.dependencies]
# CLI
clap = { version = "4.5", features = ["derive"] }

# Async
tokio = { version = "1", features = ["macros", "rt-multi-thread", "process"] }

# Serialization
serde = { version = "1", features = ["derive"] }
toml = "1"

# Error handling
thiserror = "2"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Terminal
console = "0.16"

# Utilities
which = "8"

# Dev/test
assert_cmd = "2"
predicates = "3"
tempfile = "3"
insta = "1"
```

### Example 2: smelt-cli/Cargo.toml

```toml
[package]
name = "smelt-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Smelt command-line interface"

[[bin]]
name = "smelt"
path = "src/main.rs"

[dependencies]
smelt-core.path = "../smelt-core"
clap.workspace = true
tokio.workspace = true
anyhow.workspace = true
console.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true

[dev-dependencies]
assert_cmd.workspace = true
predicates.workspace = true
tempfile.workspace = true
```

### Example 3: smelt-core/src/error.rs

```rust
use std::path::PathBuf;
use thiserror::Error;

/// Unified error type for Smelt core operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SmeltError {
    /// git binary not found on $PATH.
    #[error("`git` not found on $PATH. Smelt requires git to be installed.")]
    GitNotFound,

    /// Not inside a git repository.
    #[error("not a git repository (or any parent up to mount point)")]
    NotAGitRepo,

    /// A git command failed.
    #[error("git {operation} failed: {message}")]
    GitExecution {
        operation: String,
        message: String,
    },

    /// .smelt/ already exists.
    #[error(".smelt/ already exists in {path}. Already initialized.")]
    AlreadyInitialized { path: PathBuf },

    /// An I/O operation failed.
    #[error("{operation} at `{path}`: {source}")]
    Io {
        operation: String,
        path: PathBuf,
        source: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, SmeltError>;
```

### Example 4: .smelt/ Marker File

The minimal `.smelt/config.toml` created by `smelt init`:

```toml
# Smelt project configuration
# Documentation: https://github.com/wollax/smelt

# Smelt format version (for future migration support)
version = 1
```

This is deliberately minimal. The `version` field enables future config migrations. No other state exists at Phase 1 — orchestration state is added in later phases.

### Example 5: GitHub Actions CI Pipeline

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-Dwarnings"

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --locked
      - run: cargo test --locked
      - run: cargo clippy --locked -- -D warnings
      - run: cargo fmt --check
```

### Example 6: Shell-out to Git via tokio::process

```rust
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct GitCli {
    git_binary: PathBuf,
    repo_root: PathBuf,
}

impl GitCli {
    /// Run a git command and return stdout as a trimmed string.
    async fn run(&self, args: &[&str]) -> crate::Result<String> {
        let output = Command::new(&self.git_binary)
            .args(args)
            .current_dir(&self.repo_root)
            .output()
            .await
            .map_err(|e| SmeltError::Io {
                operation: format!("running git {}", args.first().unwrap_or(&"")),
                path: self.git_binary.clone(),
                source: e,
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SmeltError::GitExecution {
                operation: args.join(" "),
                message: stderr.trim().to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}
```

---

## Recommendations (Claude's Discretion Items)

### CLI Framework: clap (derive)

Use `clap` v4.5 with derive macros. This is the same choice as Assay. The derive API is more readable than the builder API, and Assay already provides a proven pattern for optional subcommands, `--version` propagation, and context-aware no-args behavior.

### Marker File: `.smelt/config.toml`

Use `config.toml` as the marker file (same naming as Assay's `config.toml`). Contents: just `version = 1`. Keep it minimal — the config will grow in later phases as orchestration settings are added.

### Error Messages: Follow Assay's Style

- Errors to stderr via `eprintln!`
- Format: `"Error: {context}: {detail}"`
- Use `anyhow`'s `{e:#}` formatting for error chain display
- Colored output for human-readable errors (respecting `--no-color`)
- No stack traces in release builds

### CI Pipeline: GitHub Actions

Single job with build + test + clippy + fmt. Use `Swatinem/rust-cache@v2` for caching. Use `dtolnay/rust-toolchain@stable` for toolchain setup. Set `RUSTFLAGS="-Dwarnings"` to fail on warnings.

### SmeltGitOps Trait

Start with a minimal trait covering only Phase 1 needs: `repo_root()`, `is_inside_work_tree()`, `current_branch()`, `head_short()`. Expand in Phase 2 with worktree operations. The trait is async (native async fn in trait, no `async-trait` crate — Rust 1.93 supports this).

### thiserror Over anyhow for Core

Unlike the STACK.md recommendation of "anyhow now, thiserror later," use `thiserror` in `smelt-core` from day 1. Assay does this and it works well — typed errors in the core library, `anyhow` only in the CLI binary for convenience. This avoids a migration tax later.

---

## Open Questions (for Planner)

| Question | Recommendation | Confidence |
|---|---|---|
| Should `smelt init` auto-stage `.smelt/` for git commit? | No — user should control when `.smelt/` is committed. Print a hint: "Run `git add .smelt/` to track Smelt configuration." | MEDIUM |
| Workspace or single crate for Phase 1? | Workspace from day 1. Two crates: `smelt-cli` + `smelt-core`. Matches Assay. Prevents restructuring later. | HIGH |
| `which` crate vs manual PATH resolution? | Use `which` crate (v8.0.2). Cross-platform, handles edge cases. | HIGH |
| Async trait: `trait_variant` or native? | Native async fn in trait. Rust 1.93 supports it without extra crates. If `Send` bounds are needed, add them directly on the method or use `trait_variant`. Test during implementation. | MEDIUM |

---

*Phase: 01-project-bootstrap-git-ops*
*Research completed: 2026-03-09*
