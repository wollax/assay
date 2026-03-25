# Queued Work

## Backlog

### Teardown error handling cleanup in `run/phases.rs`
- Source: PR #33 review (pr-failure-finder, pr-code-reviewer)
- Priority: low (pre-existing debt, no user-facing regression)
- Description:
  1. **Silent `let _ =` on teardown** — `phases.rs` has 6 occurrences of `let _ = provider.teardown(...)` that silently discard errors. If teardown fails, the user sees "Container removed." when the container is still running. Extract a `teardown_on_error()` helper that logs warnings instead of discarding.
  2. **Double-failure context loss** — when both exec and teardown fail (`phases.rs:332-338`), the teardown error is printed but not attached to the propagated primary error.
  3. **Error chain loss via `anyhow!("{e}")`** — `monitor.write().map_err(|e| anyhow::anyhow!("{e}"))` appears 3 times, converting typed errors to string-only anyhow errors. Should use `.context()` to preserve the chain.
- Files: `crates/smelt-cli/src/commands/run/phases.rs`

### SSH client DRY opportunity: `build_ssh_args` / `build_scp_args`
- Source: PR #33 review (pr-code-simplifier)
- Priority: low (cosmetic)
- Description: The two methods share ~90% identical logic (common flags, key resolution, port handling) and differ only in `-p` vs `-P` for the port flag. Could extract a shared helper.
- Files: `crates/smelt-cli/src/serve/ssh/client.rs`
