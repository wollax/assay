---
estimated_steps: 4
estimated_files: 4
---

# T04: Manifest validation integration and just ready

**Slice:** S01 — Manifest Dependencies & DAG Validation
**Milestone:** M002

## Description

Wire the feature gate into downstream crates (assay-cli, assay-mcp), add lightweight dependency-aware validation to the existing `manifest::validate()` function, and verify the entire workspace builds and passes `just ready`. This is the integration task that proves S01's changes compose correctly with the full codebase — not just in isolation.

## Steps

1. Enable `orchestrate` feature in `crates/assay-cli/Cargo.toml` and `crates/assay-mcp/Cargo.toml`: change `assay-core.workspace = true` to `assay-core = { workspace = true, features = ["orchestrate"] }`.
2. Add dependency-aware pre-checks to `assay-core::manifest::validate()`: when any session has non-empty `depends_on`, validate that (a) all `depends_on` entries reference an effective name of another session in the manifest, (b) no session depends on itself, (c) effective names are unique among sessions. This is a lightweight structural check — full DAG validation (cycle detection) happens in `DependencyGraph::from_manifest()` which requires the feature gate. These pre-checks don't require the feature gate since they only inspect the `Vec<String>` field.
3. Run `cargo test -p assay-mcp` and check if any MCP schema snapshots need updating from the `ManifestSession` type change. If so, accept with `cargo insta test -p assay-mcp --review`. Run `cargo test -p assay-cli`.
4. Run `just ready` (fmt, lint, test, deny). Fix any issues. Verify clean pass.

## Must-Haves

- [ ] `assay-cli` Cargo.toml enables `orchestrate` feature on assay-core
- [ ] `assay-mcp` Cargo.toml enables `orchestrate` feature on assay-core
- [ ] `manifest::validate()` rejects manifests where `depends_on` references unknown sessions
- [ ] `manifest::validate()` rejects self-dependencies
- [ ] `manifest::validate()` rejects duplicate effective names when dependencies present
- [ ] All MCP schema snapshots updated if affected
- [ ] `just ready` passes clean

## Verification

- `just ready` — full green (fmt, lint, test, deny)
- `cargo test -p assay-core --features orchestrate` — DAG tests pass in full workspace context
- `cargo test -p assay-mcp` — MCP tests pass including any updated snapshots

## Observability Impact

- Signals added/changed: `manifest::validate()` now surfaces dependency-related `ManifestError` entries alongside existing validation errors
- How a future agent inspects this: validation errors are collected in the existing `Vec<ManifestError>` and surfaced through `AssayError::ManifestValidation`
- Failure state exposed: actionable field paths like `sessions[2].depends_on[0]` with the problematic reference

## Inputs

- `crates/assay-cli/Cargo.toml` — existing dependency on assay-core (adding feature)
- `crates/assay-mcp/Cargo.toml` — existing dependency on assay-core (adding feature)
- `crates/assay-core/src/manifest.rs` — existing `validate()` function (extending)
- T01, T02, T03 outputs — feature gate, DependencyGraph, query methods

## Expected Output

- `crates/assay-cli/Cargo.toml` — assay-core with orchestrate feature enabled
- `crates/assay-mcp/Cargo.toml` — assay-core with orchestrate feature enabled
- `crates/assay-core/src/manifest.rs` — `validate()` with dependency-aware checks
- Clean `just ready` pass confirming full workspace integration
