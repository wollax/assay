# S04: SshSyncBackend and CLI/MCP factory wiring — Research

**Date:** 2026-03-27

## Summary

S04 has two distinct deliverables: (1) `SshSyncBackend` implementing all 7 `StateBackend` methods via `scp`/`ssh` shell commands, and (2) wiring `backend_from_config()` into all CLI and MCP construction sites that currently hardcode `LocalFsBackend::new(...)`.

The SshSyncBackend is the most capable remote backend (`CapabilitySet::all()`) because it mirrors the local filesystem layout on a remote host — every method has a direct scp/ssh equivalent. The CLI/MCP wiring is mechanical but touches 6 callsites across 2 files and requires adding `assay-backends` as a dependency to both `assay-cli` and `assay-mcp`.

The primary risk is scp argument construction — D163 mandates `Command::arg()` chaining (no shell interpolation) to prevent injection from user-supplied host/path values. A contract test with a path containing spaces proves this works correctly.

## Recommendation

**Split into 3 tasks:**

1. **T01 — Contract tests (red state):** Write `tests/ssh_backend.rs` with mock scp/ssh scripts (PATH override, same pattern as `github_backend.rs`). Tests prove all 7 trait methods, arg construction with spaces in paths, and CapabilitySet::all(). Should compile-fail until T02.

2. **T02 — SshSyncBackend implementation:** Implement `src/ssh.rs` with `ScpRunner` (low-level scp/ssh wrapper) and `SshSyncBackend` struct. Wire into `factory.rs` and `lib.rs`. All T01 tests pass.

3. **T03 — CLI/MCP factory wiring:** Add `assay-backends` dep to `assay-cli` and `assay-mcp`. Replace all 6 `LocalFsBackend::new(...)` callsites with `backend_from_config()`. Remove `use assay_core::state_backend::LocalFsBackend` imports. `just ready` green.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Mock subprocess for scp/ssh in tests | `write_mock_gh()` pattern in `tests/github_backend.rs` | Exact same technique — shell script in PATH, `#[serial]` for isolation |
| Atomic file writes for local cache | `std::fs::write` for simple cache files | `.ssh-run-state` cache file pattern matches `.github-issue-number` |
| Serde JSON for OrchestratorStatus | Existing `serde_json::to_string_pretty` / `from_str` | Same serialization as Linear/GitHub backends |

## Existing Code and Patterns

- `crates/assay-backends/src/github.rs` — **Primary template.** `GhRunner` wraps `Command::new("gh")` with `.arg()` chaining; `GitHubBackend` caches issue number in `run_dir/.github-issue-number`. SshSyncBackend should follow identical structure: `ScpRunner` for low-level scp/ssh calls, `SshSyncBackend` for trait impl.
- `crates/assay-backends/tests/github_backend.rs` — **Test template.** Mock shell script with PATH override, `#[serial]`, `sample_status()` helper. SSH tests need `write_mock_scp()` and `write_mock_ssh()` equivalents.
- `crates/assay-backends/src/factory.rs` — The `Ssh` arm currently returns `NoopBackend` with a warning. T02 replaces this with `SshSyncBackend::new(...)` behind `#[cfg(feature = "ssh")]`.
- `crates/assay-core/src/state_backend.rs` — `LocalFsBackend` is the reference implementation for all 7 methods. `SshSyncBackend` should mirror every method: `push_session_event` → scp state.json to remote, `read_run_state` → scp from remote, `send_message` → ssh mkdir + scp file, `poll_inbox` → ssh ls + scp pull + ssh rm, `annotate_run` → scp text file, `save_checkpoint_summary` → scp checkpoint JSON.
- `crates/assay-cli/src/commands/run.rs` — 3 callsites (`execute_orchestrated`, `execute_mesh`, `execute_gossip`) hardcode `LocalFsBackend::new(pipeline_config.assay_dir.clone())`. Each needs `backend_from_config(manifest.state_backend.as_ref().unwrap_or(&StateBackendConfig::LocalFs), pipeline_config.assay_dir.clone())`.
- `crates/assay-mcp/src/server.rs` — 3 callsites (Dag, Mesh, Gossip orch_config construction) hardcode the same. Same replacement pattern.

## Constraints

- **D163 (mandatory):** All scp/ssh arguments via `Command::arg()` chaining. Never use shell string interpolation or `sh -c` with user-provided values.
- **D007 (sync core):** All methods are synchronous. `scp` and `ssh` are subprocess calls — naturally synchronous.
- **CapabilitySet::all():** SshSyncBackend returns all-true capabilities because the remote mirrors the local filesystem layout exactly. Every LocalFsBackend operation has a direct scp/ssh equivalent.
- **Feature gate:** `#[cfg(feature = "ssh")]` on the `ssh` module in `lib.rs` and the `Ssh` arm in `factory.rs`. No new deps needed — `std::process::Command` is stdlib.
- **No new dependencies:** Unlike Linear (reqwest), SSH uses only `std::process::Command`. The `ssh` feature flag in `Cargo.toml` already exists but gates nothing yet.
- **CLI/MCP wiring requires `assay-backends` dep:** Both `assay-cli/Cargo.toml` and `assay-mcp/Cargo.toml` need `assay-backends = { workspace = true }` added. The workspace root `Cargo.toml` already has `assay-backends` in `[workspace.dependencies]`.

## Common Pitfalls

- **scp remote path format** — scp uses `[user@]host:path` syntax. With optional user, the remote spec is `format!("{}:{}", host_spec, remote_path)` where `host_spec` is either `"user@host"` or just `"host"`. This must be a single `.arg()` — not split across two args.
- **scp with spaces in paths** — Remote paths with spaces must NOT be shell-escaped when using `Command::arg()` (no shell involved). However, scp interprets the remote path through a shell on the remote end. Must use proper quoting: wrap the remote path in single quotes inside the arg, or use `--` to prevent option parsing. The contract test with a space-in-path is the safety net.
- **ssh remote command quoting** — `ssh host "command"` passes the command through the remote shell. For `ls`, `rm`, `mkdir -p`, the paths should be shell-quoted. Use `format!("'{}'", path.replace('\'', "'\\''"))` for proper single-quote escaping.
- **Port flag differs: scp uses `-P`, ssh uses `-p`** — Easy to mix up. scp's port flag is uppercase `-P`, ssh's is lowercase `-p`.
- **Missing remote directory** — `scp` fails if the remote directory doesn't exist. `push_session_event`, `send_message`, etc. should ensure the remote directory exists via `ssh mkdir -p` before scp.
- **Factory wiring: manifest.state_backend is Option** — The field is `Option<StateBackendConfig>`. The unwrap pattern should be `.as_ref().unwrap_or(&StateBackendConfig::LocalFs)` to preserve backward compatibility with manifests that don't specify a backend.
- **Test isolation with PATH override** — Mock scp/ssh scripts in PATH must use `#[serial]` to avoid parallel test interference. Follow the exact pattern from `github_backend.rs`.

## Open Risks

- **scp remote path shell interpretation** — scp passes the remote path through the remote shell. Paths with shell metacharacters (`;`, `$`, backticks) could be interpreted. The `Command::arg()` pattern prevents local injection, but remote interpretation is a separate concern. Mitigation: document that `remote_assay_dir` should be a simple path without shell metacharacters. The contract test with spaces validates the most common edge case.
- **poll_inbox atomicity** — `poll_inbox` needs to list remote files, pull them, then delete them. Between list and delete, new files could appear. This is inherent to the scp approach and matches the same race window that exists in `LocalFsBackend` (read-then-delete). Acceptable for Tier-2 events.
- **CLI/MCP test surface** — The wiring in T03 is mechanical but could break existing tests if any test constructs a `RunManifest` with `state_backend: Some(...)` that hits the new factory path. Need to verify no existing test depends on the `LocalFsBackend::new(...)` pattern being hardcoded.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / std::process::Command | n/a | Core stdlib — no skill needed |
| scp/ssh | n/a | Standard Unix tools — no skill needed |

## Sources

- `crates/assay-backends/src/github.rs` — Primary implementation template (403 lines)
- `crates/assay-backends/tests/github_backend.rs` — Primary test template (307 lines, mock subprocess pattern)
- `crates/assay-core/src/state_backend.rs` — Trait definition and LocalFsBackend reference (402 lines)
- D163 decision — scp arg() chaining mandate
- D165 decision — backend_from_config factory fn location
- S01-SUMMARY.md — Factory structure and NoopBackend stub pattern
