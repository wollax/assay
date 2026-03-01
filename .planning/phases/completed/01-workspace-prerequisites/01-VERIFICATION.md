---
phase: 01-workspace-prerequisites
verified_by: kata-verifier
status: passed
score: 8/8
verified_at: 2026-03-01T04:26:12Z
---

# Phase 01: Workspace Prerequisites — Verification

## Must-Have Results

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | schemars 1.x is the workspace-level dependency (no 0.8 remnants) | PASS | `Cargo.toml` line 17: `schemars = "1"`. `Cargo.lock` contains only `schemars 1.2.1` — no 0.8 entry. `cargo tree -p schemars` confirms `schemars v1.2.1`. |
| 2 | All existing JsonSchema derives in assay-types compile without source changes | PASS | `crates/assay-types/src/lib.rs` is unchanged (Spec, Gate, Review, Workflow, Config all carry `#[derive(JsonSchema)]`). `cargo check --workspace` exits 0 with no errors. |
| 3 | rmcp 0.17 is a workspace dependency with server and transport-io features | PASS | `Cargo.toml` line 18: `rmcp = { version = "0.17", features = ["server", "transport-io"] }`. `Cargo.lock` resolves to `rmcp v0.17.0`. |
| 4 | assay-mcp crate exists as a library and passes cargo check | PASS | `crates/assay-mcp/` exists with `Cargo.toml` and `src/lib.rs`. `cargo check -p assay-mcp` exits 0 with `Finished` — no errors or warnings. |
| 5 | `just ready` passes with zero warnings and zero errors | PASS | `just ready` ran all four steps (fmt-check, clippy, test, cargo-deny) and printed "All checks passed." Clippy produced no warnings; all tests passed (0 tests across 5 crates). `cargo-deny` issued only informational `warning[duplicate]` and `warning[license-not-encountered]` entries — these are warnings to the deny tool operator, not build errors; the check itself concluded `advisories ok, bans ok, licenses ok, sources ok`. |

## Artifact Checks

| Artifact | Exists | Min Lines | Content Valid | Notes |
|----------|--------|-----------|---------------|-------|
| `Cargo.toml` | Y | Y (25 lines, min 15) | Y | Contains `schemars = "1"` (line 17), `rmcp = { version = "0.17", features = ["server", "transport-io"] }` (line 18), and `assay-mcp = { path = "crates/assay-mcp" }` (line 14). Workspace member glob `members = ["crates/*"]` covers assay-mcp. |
| `crates/assay-mcp/Cargo.toml` | Y | Y (15 lines, min 10) | Y | Declares `assay-core.workspace = true`, `rmcp.workspace = true`, `schemars.workspace = true`, `serde.workspace = true`, `serde_json.workspace = true`, `tokio.workspace = true`. All metadata fields delegated to workspace. |
| `crates/assay-mcp/src/lib.rs` | Y | Y (4 lines, min 1) | Y | Non-empty module-level doc comment describing the crate's purpose. Compiles cleanly. |

## Key Link Checks

| Link | Status | Notes |
|------|--------|-------|
| `Cargo.toml` → `crates/assay-types/Cargo.toml`: schemars version change does not break derives | PASS | `assay-types/Cargo.toml` uses `schemars.workspace = true`. The workspace resolves to 1.2.1. All five `#[derive(JsonSchema)]` structs in `assay-types/src/lib.rs` compile without modification. |
| `Cargo.toml` → `crates/assay-mcp/Cargo.toml`: workspace member glob picks up assay-mcp; rmcp and assay-core declared as workspace deps | PASS | `members = ["crates/*"]` glob picks up `crates/assay-mcp`. `assay-mcp/Cargo.toml` references `assay-core.workspace = true` and `rmcp.workspace = true`, both of which are defined in the root `[workspace.dependencies]`. |
| `deny.toml` → `Cargo.toml`: cargo-deny license allow-list covers all new transitive dependencies | PASS | `cargo deny check` completed with `licenses ok`. The allow-list (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unicode-DFS-2016) covers all transitive deps introduced by rmcp and schemars 1.x. The `license-not-encountered` warnings are for allow-list entries with no matching crates on this platform — they are not failures. |

## Command Results

### cargo check --workspace

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.85s
```

Zero errors. Zero warnings.

### cargo check -p assay-mcp

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s
```

Zero errors. Zero warnings.

### just ready

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s
cargo test --workspace
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.45s
     Running unittests src/main.rs (assay_cli)     → 0 tests — ok
     Running unittests src/lib.rs  (assay_core)    → 0 tests — ok
     Running unittests src/lib.rs  (assay_mcp)     → 0 tests — ok
     Running unittests src/main.rs (assay_tui)     → 0 tests — ok
     Running unittests src/lib.rs  (assay_types)   → 0 tests — ok
   Doc-tests assay_core, assay_mcp, assay_types    → 0 tests — ok
cargo deny check
advisories ok, bans ok, licenses ok, sources ok
All checks passed.
```

`cargo-deny` emitted `warning[duplicate]` entries (crossterm 0.28/0.29, rustix, windows-sys, windows-targets, and derived windows arch crates) caused by `ratatui` pulling `crossterm 0.29` while `assay-tui` directly depends on `crossterm 0.28`. These are pre-existing to this phase and are only warnings (`multiple-versions = "warn"`), not errors. All four deny categories passed.

## Verdict

**Status:** passed
**Score:** 8/8 must-haves verified

All truths confirmed against actual files and command output. The workspace root `Cargo.toml` declares `schemars = "1"` and `rmcp = { version = "0.17", features = ["server", "transport-io"] }`. No schemars 0.8 entry exists anywhere in `Cargo.lock`. The `assay-mcp` crate compiles cleanly as a library crate. `assay-types/src/lib.rs` was not modified — all five structs retain their `#[derive(JsonSchema)]` attributes and compile against schemars 1.x. `just ready` exits 0 with all four checks (fmt, clippy, test, deny) passing. The phase goal is fully achieved.
