---
estimated_steps: 4
estimated_files: 12
---

# T05: Migrate remaining assay-cli and assay-tui eprintln calls

**Slice:** S01 — Structured tracing foundation and eprintln migration
**Milestone:** M009

## Description

Complete the migration by converting the remaining 33 `eprintln!` calls across 10 CLI files and 2 TUI files. After this task, zero `eprintln!` calls exist in any production crate. This task also runs `just ready` as the final slice verification.

## Steps

1. Migrate remaining CLI files (33 calls total):
   - `context.rs` (8): guard startup info → `info!`, guard errors → `error!`, configuration errors → `error!`, platform-unsupported → `error!`
   - `worktree.rs` (7): warnings → `warn!`, status messages → `info!`, errors → `error!`. **Keep 2 `eprint!` interactive prompts** ("Remove all? [y/N]", "Remove anyway? [y/N]")
   - `milestone.rs` (4): serialization/domain errors → `error!`
   - `history.rs` (4): missing project → `error!`, suggestions → `info!`
   - `main.rs` (3): no project → `error!`, help errors → `error!`
   - `pr.rs` (2): errors → `error!`
   - `init.rs` (2): errors → `error!` or `info!`
   - `plan.rs` (1): error → `error!`
   - `spec.rs` (1): error → `error!`
   - `mcp.rs` (1): tracing init failure → use stderr fallback directly since subscriber may not be up yet — convert to `eprintln!` → actually this IS the init failure case. Since the subscriber setup failed, we cannot use tracing. Keep a raw `eprintln!` as a fallback? No — the centralized `init_tracing` handles this internally by falling back to default level. The `mcp.rs` eprintln for init failure no longer exists after T02 removes `init_mcp_tracing()`. Verify this is already gone.
2. Migrate TUI files (3 calls):
   - `app.rs` (2): cycle status warning → `warn!`, config load warning → `warn!`
   - `main.rs` (1): gh CLI not found → `warn!` (D131 supersedes D125)
3. Final verification sweep:
   - `grep -rn 'eprintln!' crates/assay-cli/src/ crates/assay-core/src/ crates/assay-tui/src/ crates/assay-mcp/src/ --include='*.rs'` → zero matches
   - `grep -rn 'eprint!' crates/ --include='*.rs' | grep -v eprintln` → exactly 3 matches (1 gate.rs, 2 worktree.rs)
4. Run `just ready` — must pass (fmt, lint, test, deny all green). Fix any clippy warnings about unused imports (e.g. `std::io::Write` may no longer be needed after removing eprintln in some files).

## Must-Haves

- [ ] Zero `eprintln!` in all 4 production crates (assay-cli, assay-core, assay-tui, assay-mcp)
- [ ] 3 `eprint!` calls preserved (1 gate.rs, 2 worktree.rs)
- [ ] `just ready` passes (fmt, lint, test, deny)
- [ ] No unused import warnings from removed eprintln usage

## Verification

- `grep -rn 'eprintln!' crates/assay-cli/src/ crates/assay-core/src/ crates/assay-tui/src/ crates/assay-mcp/src/ --include='*.rs'` returns zero
- `just ready` passes
- `cargo build -p assay-cli -p assay-tui` succeeds

## Observability Impact

- Signals added/changed: All remaining unstructured stderr output now flows through the tracing subscriber. Guard daemon lifecycle, worktree operations, milestone operations, and TUI startup all emit structured events.
- How a future agent inspects this: `RUST_LOG=info` shows all user-facing events across all binaries consistently.
- Failure state exposed: All error paths now use `tracing::error!` — filterable with `RUST_LOG=error` for error-only output.

## Inputs

- T01-T04 output: telemetry module exists, subscriber wired, assay-core and batch-1 CLI files already migrated
- Remaining files: context.rs (8), worktree.rs (7), milestone.rs (4), history.rs (4), main.rs (3), pr.rs (2), init.rs (2), plan.rs (1), spec.rs (1), mcp.rs (verify T02 removed it), app.rs (2), tui/main.rs (1)

## Expected Output

- All 12 listed files modified — eprintln replaced with tracing macros
- `just ready` green — the slice is complete
