# M011: Code Quality III & Operational Hardening — Context

**Gathered:** 2026-03-24
**Status:** Ready for planning

## Project Description

Smelt is a mature infrastructure tool with 10 completed milestones, 290+ tests, and comprehensive documentation. Two large files remain above the 500L decomposition threshold (manifest.rs at 1924L, git/cli.rs at 1365L), there's a flaky integration test, and the CLI mixes user-facing output (`eprintln!`) with diagnostic tracing. The HTTP API lacks a health check endpoint for load balancers and monitoring.

## Why This Milestone

M009 established the 500L file threshold (D126) and decomposed three files, but missed the two largest. The flaky `test_cli_run_invalid_manifest` (10s subprocess timeout) has been failing since before M010. The `eprintln!` pattern worked during early development but is now scattered across 50+ call sites in the CLI — migrating to structured tracing makes the tool production-ready for operators who need filterable, structured diagnostic output. The health endpoint completes the operational story for `smelt serve` deployments behind load balancers.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Run `cargo test --workspace` with zero failures (flaky test fixed)
- Set `SMELT_LOG=info` to see structured tracing events for all CLI operations
- Hit `GET /health` on a running `smelt serve` instance without auth credentials and get a 200 response
- Navigate the codebase without encountering any file over 500 lines

### Entry point / environment

- Entry point: `smelt run`, `smelt serve`, `cargo test --workspace`
- Environment: local dev, CI, production server deployments
- Live dependencies involved: none

## Completion Class

- Contract complete means: all files under 500L, all tests pass, clippy/doc clean
- Integration complete means: `/health` endpoint responds 200 without auth on a real `smelt serve` instance
- Operational complete means: `SMELT_LOG=info smelt run manifest.toml` produces structured tracing output instead of bare eprintln

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- `cargo test --workspace` passes with 0 failures (no flaky tests)
- No production source file exceeds 500 lines
- `GET /health` returns 200 without auth when `[auth]` is configured
- All `eprintln!` calls in smelt-cli are replaced with tracing events (except `main.rs` error handler and `serve/tui.rs` post-restore error — see S02-RESEARCH.md)
- `cargo clippy --workspace` clean, `cargo doc --workspace --no-deps` zero warnings

## Risks and Unknowns

- **Tracing formatter for user-facing output** — replacing `eprintln!` means tracing must produce clean user-visible output (not JSON or prefixed log lines) at the default log level. Need a custom formatter layer or careful use of `tracing_subscriber::fmt` with a human-friendly format. Low risk — tracing-subscriber supports this.
- **manifest.rs decomposition seams** — 1924 lines with a single `impl JobManifest` block containing validation, loading, and serde logic. Natural seams may require extracting validation into a submodule. Low risk — M009 established the pattern (D128).

## Existing Codebase / Prior Art

- `crates/smelt-core/src/manifest.rs` — 1924 lines. Contains `JobManifest`, `Environment`, `SessionDef`, `ComposeService`, `KubernetesConfig`, `ValidationErrors`, validation logic, `resolve_repo_path()`, and tests.
- `crates/smelt-core/src/git/cli.rs` — 1365 lines. Contains `GitCli` struct, `GitOps` trait impl, 20+ async methods, and helper functions.
- `crates/smelt-cli/src/commands/run/phases.rs` — 33 `eprintln!` calls for user-facing progress messages.
- `crates/smelt-cli/src/commands/watch.rs` — 10 `eprintln!` calls.
- `crates/smelt-cli/src/serve/http_api.rs` — Current router with auth middleware; health endpoint will be added here.
- `crates/smelt-cli/tests/docker_lifecycle.rs` — `test_cli_run_invalid_manifest` with 10s subprocess timeout.
- D126 (500L threshold), D128 (file-to-directory module conversion with re-exports), D129 (tests follow implementation)

> See `.kata/DECISIONS.md` for all architectural and pattern decisions — it is an append-only register; read it during planning, append to it during execution.

## Relevant Requirements

- R060 — Large file decomposition round 2 (manifest.rs, git/cli.rs)
- R061 — Flaky test fix (test_cli_run_invalid_manifest)
- R062 — Full tracing migration (replace all eprintln! with tracing events)
- R063 — Health check endpoint (unauthenticated GET /health)

## Scope

### In Scope

- Decompose `manifest.rs` (1924L) into focused modules along natural seams
- Decompose `git/cli.rs` (1365L) into focused modules
- Fix `test_cli_run_invalid_manifest` flaky timeout
- Replace all `eprintln!` in smelt-cli with `tracing::info!`/`tracing::warn!`/`tracing::error!` events
- Configure tracing-subscriber for clean user-facing output at default level
- Add unauthenticated `GET /health` endpoint to `smelt serve`
- Update README if health endpoint is user-facing

### Out of Scope / Non-Goals

- Readiness/liveness probes with detailed status (just a simple health check)
- JSON structured log output format (tracing default human-readable format is sufficient)
- OpenTelemetry or distributed tracing integration
- Metrics or Prometheus endpoint

## Technical Constraints

- `deny(missing_docs)` enforced on both crates (D127, D070) — all new public items need docs
- Module conversion must preserve all public API signatures via re-exports (D128)
- Tests must co-locate with their implementation modules (D129)
- Auth middleware currently applies globally — health endpoint needs to bypass auth (split router groups or pre-auth route)
- `tracing_subscriber::fmt().init()` can only be called once per process (D107) — the existing branched init in `main.rs` must be updated to support the new tracing approach

## Integration Points

- axum Router in `http_api.rs` — health endpoint route and auth bypass
- `main.rs` tracing subscriber init — format layer configuration
- All CLI command modules — eprintln! → tracing migration

## Open Questions

- None — design decisions settled during discussion.
