# S05: smelt-core Library API

**Goal:** Make `smelt-core` a presentable, embeddable Rust library with `#![deny(missing_docs)]`, complete Cargo metadata, a crate-level usage example, and a proof-of-concept external crate that validates the embedding story end-to-end.

**Demo:** `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge` exits 0 with no errors. A standalone `smelt-example` crate at `/tmp/smelt-example/` imports `smelt_core` via path dependency, calls `GitHubForge::new()` and `JobManifest::from_str()` in a `#[test]`, and `cargo test` passes.

## Must-Haves

- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps` exits 0 (no feature flag)
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge` exits 0 (forge feature)
- `#![deny(missing_docs)]` is present in `crates/smelt-core/src/lib.rs`
- `smelt-core/Cargo.toml` has `keywords`, `categories`, and `homepage` filled in
- `lib.rs` has a multi-line `//!` crate doc with a `no_run` usage example showing import paths
- Internal-only types (`SmeltRunManifest`, `SmeltManifestSession`, `SmeltSpec`, `SmeltCriterion`) are `pub(crate)` — not part of the embedding API
- `GitWorktreeEntry` struct fields have `///` doc comments (kept `pub` because it appears in the `GitOps::worktree_list()` return type)
- `/tmp/smelt-example/` is a standalone Cargo project with `smelt-core` as a path dependency (`features = ["forge"]`)
- `cd /tmp/smelt-example && cargo test` compiles and passes with at least two tests: one calling `GitHubForge::new()` and one calling `JobManifest::from_str()`
- `cargo test --workspace -q` still passes (no regressions from any API curation change)

## Proof Level

- This slice proves: contract + integration
- Real runtime required: no (smelt-example tests avoid Docker/GitHub network calls)
- Human/UAT required: no

Verification of `#![deny(missing_docs)]` is compiler-enforced (contract-level). Verification of the embedding story is integration-level: a real external crate with a real Cargo path dependency that compiles and runs tests.

## Verification

```bash
# No-forge: 0 missing_docs errors
RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "^error" | wc -l
# expected: 0

# Forge feature: 0 missing_docs errors
RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1 | grep "^error" | wc -l
# expected: 0

# Workspace tests pass (no regressions)
cargo test --workspace -q 2>&1 | grep "failed"
# expected: (no output)

# External embedding proof
cd /tmp/smelt-example && cargo test
# expected: test result: ok. N passed; 0 failed
```

## Observability / Diagnostics

- Runtime signals: `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps` emits exact file:line for every missing doc; run after each file to track progress
- Inspection surfaces: `cargo doc --open -p smelt-core --features forge` — rendered HTML docs are the user-facing artifact; `lib.rs` crate-level doc is the landing page
- Failure visibility: if `#![deny(missing_docs)]` is added before all docs are fixed, `cargo build` breaks immediately with specific file:line; use `RUSTDOCFLAGS` path for iterative checking
- Redaction constraints: none — no secrets or tokens involved

## Integration Closure

- Upstream surfaces consumed: all forge types from S01 (`ForgeClient`, `GitHubForge`, `PrHandle`, `PrStatus`, `ForgeConfig`); `RunState`/`JobPhase` from S02/S03; all existing public re-exports in `lib.rs`
- New wiring introduced in this slice: `#![deny(missing_docs)]` crate attribute; Cargo metadata fields; smelt-example path dependency
- What remains before the milestone is truly usable end-to-end: S06 (live `smelt run` → PR → `smelt watch` → merge UAT with real Docker + GITHUB_TOKEN)

## Tasks

- [x] **T01: Cargo metadata, lib.rs doc, and assay type API curation** `est:30m`
  - Why: Foundational step — adds Cargo metadata so the crate is publishable, enhances the crate-level landing page doc, and demotes internal Assay translation types to `pub(crate)` so they don't leak into the library API surface. None of these items require reading many files.
  - Files: `crates/smelt-core/Cargo.toml`, `crates/smelt-core/src/lib.rs`, `crates/smelt-core/src/assay.rs`
  - Do: (1) In `Cargo.toml`, add `keywords = ["docker", "ci", "github", "assay", "infrastructure"]`, `categories = ["command-line-utilities", "development-tools"]`, `homepage = "https://github.com/wollax/smelt"`. (2) In `lib.rs`, replace the single `//!` line with a multi-paragraph doc that names the crate, describes its purpose, then includes a ` ```rust,no_run ``` ` usage example showing `use smelt_core::{JobManifest, DockerProvider, forge::GitHubForge};` and constructing a `GitHubForge` client and a `JobManifest::from_str()` call. (3) In `assay.rs`, change `pub struct SmeltRunManifest`, `pub struct SmeltManifestSession`, `pub struct SmeltSpec`, `pub struct SmeltCriterion` to `pub(crate)` — smelt-cli only calls `AssayInvoker`'s methods (which return `String`), so these struct types never appear in smelt-cli code. Do NOT change `pub struct AssayInvoker` or any of its `pub fn` methods. (4) Verify smelt-cli still compiles (`cargo build -p smelt-cli`).
  - Verify: `cargo build -p smelt-cli` succeeds; `cargo test -p smelt-core -q` passes; `cargo doc --no-deps -p smelt-core --open` generates docs without error
  - Done when: smelt-cli compiles clean, smelt-core tests pass, `smelt-core/Cargo.toml` has all three new metadata fields, lib.rs has a `no_run` usage example in the crate doc

- [x] **T02: Doc comments for error.rs, forge.rs, manifest.rs, git/mod.rs** `est:45m`
  - Why: Eliminates 34 of the 52 missing_docs errors across four files. These are all mechanical one-liner doc descriptions on named enum variant fields and struct fields. Must be done before `#![deny(missing_docs)]` can be added.
  - Files: `crates/smelt-core/src/error.rs`, `crates/smelt-core/src/forge.rs`, `crates/smelt-core/src/manifest.rs`, `crates/smelt-core/src/git/mod.rs`
  - Do: (1) **error.rs (17 items)**: Add `///` doc above each named field in `SmeltError` thiserror variants — `operation`, `message` on `GitExecution`; `operation`, `message` on `MergeConflict`; `field`, `session` on `Manifest`; `files` on `InvalidRepoPath`; `provider`, `path` on `Provider`; `source` on `Provider` (the `#[source]` field); `operation`, `message` on `Forge`; `env_var` on `Credential`; `key`, `path` on `Config`; `path`, `source` on `Io`. (2) **forge.rs (10 items)**: Add `///` docs to `PrState` variants (`Open`, `Merged`, `Closed`), `CiStatus` variants (`Pending`, `Passing`, `Failing`, `Unknown`), and `PrStatus` fields (`state`, `ci_status`, `review_count`). (3) **manifest.rs (2 items)**: Add `/// …` to the `source` fields in `CredentialStatus::Resolved { source }` and `CredentialStatus::Missing { source }`. (4) **git/mod.rs (5 items)**: Add `///` docs to `GitWorktreeEntry` fields `path`, `head`, `branch`, `is_bare`, `is_locked` — keep the struct `pub` (it appears in `GitOps::worktree_list()` return type). After each file, run `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "\.rs:" | grep -v monitor` to confirm that file is clean.
  - Verify: `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "\.rs:" | grep -v "monitor\.rs" | wc -l` → 0 (only monitor.rs errors remain); `cargo test -p smelt-core --features forge -q` passes
  - Done when: all four files are clean under `RUSTDOCFLAGS="-D missing_docs"`; only monitor.rs errors remain in the full crate doc run

- [x] **T03: Doc comments for monitor.rs + enable #![deny(missing_docs)]** `est:30m`
  - Why: Closes the final 18 missing_docs errors in `monitor.rs` and adds the crate-level `#![deny(missing_docs)]` attribute to `lib.rs`. This is the step that locks the docs requirement into the CI-visible build.
  - Files: `crates/smelt-core/src/monitor.rs`, `crates/smelt-core/src/lib.rs`
  - Do: (1) Read `monitor.rs` to find all 18 undocumented items: 10 `JobPhase` variants (lines 22-31), 7 `RunState` fields (lines 37-43: `job_name`, `phase`, `container_id`, `sessions`, `started_at`, `updated_at`, `pid`), and 1 `JobMonitor.state` field (line 69). Add a `///` doc above each. (2) Add `#![deny(missing_docs)]` as the first inner attribute in `lib.rs` (after the `//!` crate doc, before any `pub mod` declarations). (3) Run `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "^error" | wc -l` → 0. (4) Run with forge feature: `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1 | grep "^error" | wc -l` → 0. (5) Run `cargo test --workspace -q` → all pass, confirm `cargo build -p smelt-cli` still clean.
  - Verify: Both `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps` and `--features forge` variants exit without `^error` lines; `cargo test --workspace -q` passes with 0 failures
  - Done when: `#![deny(missing_docs)]` is in lib.rs, both cargo doc variants are clean, all workspace tests pass

- [x] **T04: smelt-example external crate (R005 validation)** `est:30m`
  - Why: Directly validates R005 — proves `smelt-core` can be used as a path dependency by an external Rust crate and that `GitHubForge::new()`, `JobManifest::from_str()`, and `DockerProvider` are callable without going through the CLI. This is the slice's integration-proof task.
  - Files: `/tmp/smelt-example/Cargo.toml` (new), `/tmp/smelt-example/src/lib.rs` (new placeholder), `/tmp/smelt-example/tests/api.rs` (new)
  - Do: (1) Create directory structure: `mkdir -p /tmp/smelt-example/src /tmp/smelt-example/tests`. (2) Write `Cargo.toml` with `name = "smelt-example"`, `edition = "2024"`, dependencies: `smelt-core = { path = "/Users/wollax/Git/personal/smelt/crates/smelt-core", features = ["forge"] }`, dev-dependencies: `tokio = { version = "1", features = ["rt-multi-thread", "macros"] }`. (3) Write `src/lib.rs` with just `// smelt-example: external embedding proof`. (4) Write `tests/api.rs` with: `test_githubforge_builds` — `let forge = smelt_core::GitHubForge::new("test-token".to_string()); assert!(forge.is_ok(), "GitHubForge::new should succeed with a token");`; `test_jobmanifest_from_str_minimal` — parse a minimal valid TOML string into `smelt_core::JobManifest` (copy a minimal `[job]`/`[docker]`/`[assay]`/`[credentials]`/`[git]` skeleton from the existing examples, call `::from_str()`, assert `is_ok()`); `test_docker_provider_constructs` — `let config = smelt_core::SmeltConfig::default(); let provider = smelt_core::DockerProvider::new(config); drop(provider);` (does not call any Docker API). (5) Run `cd /tmp/smelt-example && cargo test` — all three tests must pass.
  - Verify: `cd /tmp/smelt-example && cargo test 2>&1 | tail -5` shows `test result: ok. 3 passed; 0 failed`
  - Done when: smelt-example `cargo test` passes; the path dependency compiles against the real smelt-core with forge feature; R005 is validated

## Files Likely Touched

- `crates/smelt-core/Cargo.toml`
- `crates/smelt-core/src/lib.rs`
- `crates/smelt-core/src/assay.rs`
- `crates/smelt-core/src/error.rs`
- `crates/smelt-core/src/forge.rs`
- `crates/smelt-core/src/manifest.rs`
- `crates/smelt-core/src/git/mod.rs`
- `crates/smelt-core/src/monitor.rs`
- `/tmp/smelt-example/Cargo.toml` (new — external, not in workspace)
- `/tmp/smelt-example/src/lib.rs` (new)
- `/tmp/smelt-example/tests/api.rs` (new)
