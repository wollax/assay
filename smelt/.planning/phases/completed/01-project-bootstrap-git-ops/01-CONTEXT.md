# Phase 1: Project Bootstrap & Git Operations Layer - Context

**Gathered:** 2026-03-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Establish the Rust project skeleton, CLI entry point, and the foundational `SmeltGitOps` trait that wraps git CLI operations. Git-native state storage begins here — `.smelt/` directory structure and serialization conventions. CI pipeline runs build, test, clippy on every push.

</domain>

<decisions>
## Implementation Decisions

### `.smelt/` directory structure and visibility
- `.smelt/` is git-tracked (collaborative state shared across collaborators)
- TOML serialization format for all state files
- Minimal marker file at init (no orchestration state yet — that comes in later phases)
- Explicit `smelt init` command required to create `.smelt/` (no auto-creation)
- Auto-discover repo root via `git rev-parse --show-toplevel` and place `.smelt/` there

### CLI command structure and output style
- No-args behavior: context-aware like Assay — show status inside a project (`.smelt/` exists), error + help outside ("Not a Smelt project. Run `smelt init` to get started.")
- Only `smelt init` as a subcommand for Phase 1; add subcommands as phases deliver them
- `--help` shows only currently implemented commands (no "coming soon" placeholders)
- Colored terminal output by default, with `--no-color` flag and config setting to disable

### Error behavior and git discovery
- Git repo required for all commands including `smelt init` — fail if not in a git repo
- Fail immediately if `git` binary not found on `$PATH` (don't defer to first git operation)
- Startup sanity checks: git exists, inside a git repo
- Per-operation precondition checks: dirty working tree, detached HEAD, etc. — only when the specific operation requires clean state

### Claude's Discretion
- CLI framework choice (clap is expected but not mandated)
- Exact marker file contents and naming within `.smelt/`
- Error message wording and formatting
- CI pipeline configuration details
- `SmeltGitOps` trait method signatures and error types

</decisions>

<specifics>
## Specific Ideas

- CLI UX should mirror Assay's patterns — context-aware no-args, `init` subcommand, consistent feel across the toolchain
- Strict about prerequisites: fail fast and clearly rather than limping along with missing git

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-project-bootstrap-git-ops*
*Context gathered: 2026-03-09*
