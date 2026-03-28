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

### Deduplicate template manifest TOML constants across test modules
- Source: PR #44 review (pr-code-simplifier, pr-comment-analyzer)
- Priority: low (cosmetic, test-only)
- Description: `TEMPLATE_MANIFEST_TOML` in `config.rs` tests and `TEMPLATE_TOML` in `tracker.rs` tests are identical. If `JobManifest` gains a required field, both must be updated. Extract a shared test helper or `pub(crate)` constant, or add cross-reference comments.
- Files: `crates/smelt-cli/src/serve/config.rs`, `crates/smelt-cli/src/serve/tracker.rs`

### `sanitize()` single-pass optimization
- Source: PR #44 review (pr-code-simplifier)
- Priority: low (perf micro-optimization, correctness is fine)
- Description: `sanitize()` allocates an intermediate `String` from `to_lowercase()` then iterates again to collapse hyphens. Can be done in a single pass over the lowercased chars. Also: `to_ascii_lowercase()` vs `to_lowercase()` — current impl handles non-ASCII correctly via `to_lowercase()`.
- Files: `crates/smelt-cli/src/serve/tracker.rs`

### `MockTrackerSource` over-wrapping with `Arc<Mutex<>>`
- Source: PR #44 review (pr-code-simplifier)
- Priority: low (cosmetic, test-only)
- Description: The mock uses `Arc<Mutex<VecDeque<...>>>` but is never cloned; plain `Mutex<VecDeque>` without `Arc` would suffice. Kept for consistency with `MockSshClient` pattern. Revisit if the mock pattern is refactored broadly.
- Files: `crates/smelt-cli/src/serve/tracker.rs`

### Deduplicate manifest TOML in `StateBackendConfig` tests
- Source: PR #44 review (pr-code-simplifier)
- Priority: low (cosmetic, test-only)
- Description: The three `manifest_state_backend_*` tests each duplicate a full manifest TOML (~15 lines) differing only in `[state_backend]`. A `manifest_with_state_backend(section: &str) -> String` helper would reduce duplication.
- Files: `crates/smelt-core/src/manifest/tests/core.rs`

### Typestate pattern for `LinearTrackerSource` initialization guard
- Source: PR #46 review (pr-type-design-analyzer)
- Priority: medium (type safety improvement)
- Description: `LinearTrackerSource` allows construction in an invalid state — `transition_state()` fails at runtime if `ensure_labels()` was not called. A typestate pattern (`Uninitialized` → `Ready`) with `PhantomData` could make this a compile-time guarantee. `TrackerSource` would only be implemented on the `Ready` variant. Trade-off: small increase in callsite verbosity in S05.
- Files: `crates/smelt-cli/src/serve/linear/source.rs`

### Newtype wrappers for `IssueUuid` / `LabelUuid` to prevent transposition
- Source: PR #46 review (pr-type-design-analyzer)
- Priority: low (defense-in-depth)
- Description: `add_label(issue_id: &str, label_id: &str)` and `remove_label` accept two `&str` params that are both UUIDs — a transposition is a silent logic error. Lightweight newtypes (`IssueUuid(String)`, `LabelUuid(String)`) would make swaps a compile error.
- Files: `crates/smelt-cli/src/serve/linear/mod.rs`, `crates/smelt-cli/src/serve/linear/source.rs`

### End-to-end test: `ensure_labels()` → `transition_state()` integration
- Source: PR #46 review (pr-type-design-analyzer)
- Priority: medium (test coverage gap)
- Description: `make_source_with_cache()` in tests bypasses `ensure_labels()` entirely, so no test exercises the real initialization-then-transition path. Add one test that calls `ensure_labels()` followed by `transition_state()` to verify the UUIDs stored by ensure match those passed to mutations.
- Files: `crates/smelt-cli/src/serve/linear/source.rs`

### `MockLinearClient` Default impl
- Source: PR #46 review (pr-code-simplifier)
- Priority: low (cosmetic)
- Description: `MockLinearClient::new()` is exactly `Default::default()`. Adding `impl Default` is idiomatic Rust convention.
- Files: `crates/smelt-cli/src/serve/linear/mock.rs`
