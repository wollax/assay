# M011: Code Quality III & Operational Hardening

**Vision:** All remaining large files are decomposed below 500 lines, the flaky integration test is fixed, all CLI output migrates from `eprintln!` to structured tracing, and `smelt serve` gains an unauthenticated health check endpoint — completing the production-readiness story.

## Success Criteria

- No production source file in `crates/` exceeds 500 lines
- `cargo test --workspace` passes with 0 failures (no flaky tests)
- Zero `eprintln!` calls in `crates/smelt-cli/src/` (except the error handler in `main.rs`)
- `SMELT_LOG=info smelt run --dry-run examples/job-manifest.toml` produces tracing output
- `GET /health` returns 200 on a running `smelt serve` without auth headers
- `GET /health` returns 200 even when `[auth]` is configured
- `cargo clippy --workspace` clean
- `cargo doc --workspace --no-deps` zero warnings
- All existing 290+ tests pass unchanged (behavior preserved)

## Key Risks / Unknowns

- **Tracing output format for user-facing messages** — `eprintln!("Provisioning container...")` must become a tracing event that still looks clean to users at default log level. Risk: tracing default format includes timestamps, levels, and targets which are noisy for progress messages. Mitigation: use a minimal formatter or `tracing::event!` with a custom layer.

## Proof Strategy

- **Tracing format quality** → retire in S02 by running `smelt run --dry-run` and confirming output is clean and readable without log prefixes at default level.

## Verification Classes

- Contract verification: file line counts, grep for eprintln, `cargo test/clippy/doc` all green
- Integration verification: `GET /health` returns 200 against a running `smelt serve` instance with auth configured
- Operational verification: `SMELT_LOG=info` produces structured output
- UAT / human verification: tracing output readability is subjective — human check recommended

## Milestone Definition of Done

This milestone is complete only when all are true:

- `manifest.rs` decomposed below 500L with all public API preserved via re-exports
- `git/cli.rs` decomposed below 500L with all public API preserved via re-exports
- `test_cli_run_invalid_manifest` passes reliably (not flaky)
- All `eprintln!` in smelt-cli replaced with tracing (except main.rs error handler)
- Tracing output is clean and user-readable at default level
- `GET /health` returns 200 without auth
- `cargo test --workspace` ≥290 tests, 0 failures
- `cargo clippy --workspace` clean
- `cargo doc --workspace --no-deps` zero warnings

## Requirement Coverage

- Covers: R060, R061, R062, R063
- Partially covers: none
- Leaves for later: R022 (budget/cost tracking), R026 (tracker integration)
- Orphan risks: none

## Slices

- [x] **S01: Decompose manifest.rs and git/cli.rs** `risk:medium` `depends:[]`
  > After this: `manifest.rs` and `git/cli.rs` are each below 500 lines; all public API signatures preserved via re-exports; all existing tests pass unchanged.

- [ ] **S02: Full tracing migration + flaky test fix** `risk:medium` `depends:[]`
  > After this: all `eprintln!` in smelt-cli replaced with tracing events; `smelt run --dry-run` output is clean and readable; `test_cli_run_invalid_manifest` passes reliably; `cargo test --workspace` 0 failures.

- [ ] **S03: Health endpoint + final verification** `risk:low` `depends:[S01,S02]`
  > After this: `GET /health` returns 200 without auth even when `[auth]` is configured; README updated; all milestone success criteria verified in one pass.

## Boundary Map

### S01 (independent)

Produces:
- `manifest/mod.rs` re-exporting all public types (`JobManifest`, `Environment`, `SessionDef`, `ComposeService`, `KubernetesConfig`, `ValidationErrors`, `resolve_repo_path`)
- `manifest/validation.rs` — extracted validation logic
- `git/cli/mod.rs` re-exporting `GitCli` struct and `GitOps` trait impl
- `git/cli/*.rs` — extracted method groups
- All 290+ tests passing unchanged

Consumes:
- nothing (independent)

### S02 (independent)

Produces:
- All `eprintln!` calls in smelt-cli replaced with `tracing::info!`/`tracing::warn!`/`tracing::error!`
- Updated tracing subscriber config in `main.rs` for clean user-facing output
- Fixed `test_cli_run_invalid_manifest` (increased timeout or restructured)
- All tests passing with 0 failures

Consumes:
- nothing (independent)

### S01, S02 → S03

Produces:
- `GET /health` route in `http_api.rs` — returns 200 with `{"status": "ok"}` JSON body
- Health route bypasses auth middleware
- Updated README with health endpoint documentation
- Full milestone verification pass

Consumes from S01:
- Clean decomposed codebase
Consumes from S02:
- All tracing migration complete, zero eprintln
