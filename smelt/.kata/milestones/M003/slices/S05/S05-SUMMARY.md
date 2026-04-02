---
id: S05
parent: M003
milestone: M003
provides:
  - smelt-core Cargo.toml metadata: keywords, categories, homepage — ready for crates.io publish
  - lib.rs crate-level doc block with introductory paragraphs and a no_run usage example
  - "#![deny(missing_docs)] enforced in lib.rs — undocumented pub items fail cargo build at the exact file:line"
  - 34 doc comments added across error.rs, forge.rs, manifest.rs, git/mod.rs
  - 18 doc comments added in monitor.rs covering all JobPhase variants and core RunState fields
  - SmeltRunManifest, SmeltManifestSession, SmeltSpec, SmeltCriterion demoted to pub(crate) — Assay internal translation types removed from public API surface
  - /tmp/smelt-example standalone crate: imports smelt-core via path dep with forge feature, 3 passing tests — R005 validated
requires:
  - slice: S01
    provides: ForgeClient trait, GitHubForge, PrHandle/PrState/CiStatus/PrStatus/ForgeConfig — all needed stable before publishing
  - slice: S02
    provides: JobManifest.forge, RunState.pr_url/pr_number — needed stable before publishing
  - slice: S03
    provides: RunState forge/watch fields, format_pr_section — needed stable before publishing
affects:
  - S06
key_files:
  - crates/smelt-core/Cargo.toml
  - crates/smelt-core/src/lib.rs
  - crates/smelt-core/src/assay.rs
  - crates/smelt-core/src/error.rs
  - crates/smelt-core/src/forge.rs
  - crates/smelt-core/src/manifest.rs
  - crates/smelt-core/src/git/mod.rs
  - crates/smelt-core/src/monitor.rs
  - /tmp/smelt-example/Cargo.toml
  - /tmp/smelt-example/tests/api.rs
key_decisions:
  - "D067: SmeltRunManifest/Session/Spec/Criterion demoted to pub(crate) — they are Assay internal serde translation types, not embedding targets"
  - "D068: GitWorktreeEntry kept pub with doc comments — it is in the GitOps::worktree_list() return type; demotion would create private-type-in-public-interface"
  - "D069: smelt-example at /tmp/smelt-example/ outside workspace — absolute path dep proves real external embedding without contaminating workspace build graph"
  - "GitHubForge::new requires Tokio runtime (tower::buffer initialises on construction) — external crate tests must use #[tokio::test]"
  - "[[session]] TOML blocks require both harness and timeout fields — reference: examples/job-manifest-forge.toml"
patterns_established:
  - "Internal translation types use pub(crate) — demote any type that is serde-only plumbing with no embedding use case"
  - "External embedding proof: standalone Cargo project in /tmp/ outside workspace; cd /tmp/smelt-example && cargo test is the rerunnable proof"
  - "#![deny(missing_docs)] as lib.rs inner attribute — converts advisory RUSTDOCFLAGS check into a hard build-time invariant"
observability_surfaces:
  - "cargo build -p smelt-core 2>&1 | grep 'missing documentation' — immediate file:line feedback on any undocumented new pub item"
  - "cd /tmp/smelt-example && cargo test — authoritative external embedding proof; compiler errors signal visibility regressions"
  - "RUSTDOCFLAGS='-D missing_docs' cargo doc -p smelt-core --no-deps [--features forge] — zero-warning health check (now also enforced by deny(missing_docs))"
drill_down_paths:
  - .kata/milestones/M003/slices/S05/tasks/T01-SUMMARY.md
  - .kata/milestones/M003/slices/S05/tasks/T02-SUMMARY.md
  - .kata/milestones/M003/slices/S05/tasks/T03-SUMMARY.md
  - .kata/milestones/M003/slices/S05/tasks/T04-SUMMARY.md
duration: 40min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
---

# S05: smelt-core Library API

**`#![deny(missing_docs)]` enforced, Cargo metadata added, all public types documented, Assay internals demoted to `pub(crate)`, and `/tmp/smelt-example` proves external embedding — R005 validated.**

## What Happened

Four tasks in sequence polished `smelt-core` from a well-tested internal crate into a publishable library.

**T01** laid the groundwork: `Cargo.toml` gained `keywords`, `categories`, and `homepage`; `lib.rs` was expanded from a single `//!` line to a full crate-level doc block with introductory paragraphs and a `no_run` usage example. Four internal Assay translation structs (`SmeltRunManifest`, `SmeltManifestSession`, `SmeltSpec`, `SmeltCriterion`) were demoted from `pub` to `pub(crate)` — they are serde plumbing, not part of the embedding API. smelt-cli compiled clean after the demotion since it never references these types by name.

**T02** eliminated 34 `missing_docs` compiler errors across `error.rs` (17 named enum variant fields), `forge.rs` (PrState/CiStatus variants + PrStatus fields — 10 items), `manifest.rs` (CredentialStatus variant source fields — 2 items), and `git/mod.rs` (all 5 `GitWorktreeEntry` fields). A stale doctest in `lib.rs` was also corrected: `GitHubForge::new` takes one argument (`token: String`), not three as the plan's initial draft assumed.

**T03** closed the final 18 `missing_docs` errors in `monitor.rs` (10 `JobPhase` variants and 7 core `RunState` fields) and activated `#![deny(missing_docs)]` in `lib.rs`. From this point any `pub` item added without a doc comment fails `cargo build -p smelt-core` with a precise `file:line` error.

**T04** created the external embedding proof: a standalone `/tmp/smelt-example/` Cargo project importing `smelt-core` via `path = "..."` with `features = ["forge"]`, containing three tests — `test_githubforge_builds`, `test_jobmanifest_parses_minimal_manifest`, and `test_docker_provider_new_does_not_panic`. Two issues were discovered: `GitHubForge::new` requires a Tokio runtime (fixed by switching to `#[tokio::test]`), and `[[session]]` blocks need both `harness` and `timeout` fields (fixed by consulting the reference manifest). All three tests passed.

## Verification

- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep "^error" | wc -l` → 0
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1 | grep "^error" | wc -l` → 0
- `cargo build -p smelt-core` → 0 errors (deny(missing_docs) is now a build-time invariant)
- `cargo test --workspace -q | grep failed` → empty (197 tests total, 0 failed)
- `cd /tmp/smelt-example && cargo test` → 3 passed, 0 failed (GitHubForge, JobManifest, DockerProvider)
- `grep "smelt-example" Cargo.toml` → no output (not in workspace)

## Requirements Advanced

- R005 — `smelt-core` is now a documented, metadata-equipped library with `forge` feature flag isolation; external embedding confirmed by `/tmp/smelt-example`. **Now validated.**

## Requirements Validated

- R005 — `smelt-core` exposes a stable Rust library API: `#![deny(missing_docs)]` enforced; Cargo metadata present; external crate imports via path dependency with forge feature and calls `GitHubForge::new()`, `JobManifest::from_str()`, and `DockerProvider::new()` in passing tests. crates.io publish deferred; path dependency is sufficient API design proof.

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

- None.

## Deviations

- `GitHubForge::new` requires a Tokio runtime — not documented in S01/S05 plans; discovered empirically in T04. The tower::buffer component initialises on construction. External consumers must call from async context or use `#[tokio::test]`.
- `[[session]]` blocks require both `harness` and `timeout` fields — the T04 plan's TOML snippet was incomplete; fixed by consulting `examples/job-manifest-forge.toml`.
- `lib.rs` doctest fixed in T02 (not T01): T01 introduced a stale three-argument call; T02 corrected it to the actual one-argument signature. Minor sequencing deviation with no architectural impact.

## Known Limitations

- `/tmp/smelt-example` is ephemeral — if `/tmp` is cleared, the external embedding proof must be recreated. The Cargo project can be regenerated from the T04 summary.
- `forge_with_source()` still stringifies `octocrab::Error` (D053/D056) — upgrade to carry a `#[source]` field is an open improvement, not a blocker.
- `GitHubForge::new` returning `Result<Self>` (not `Self`) means callers must handle construction errors; this is idiomatic but requires `?` in `#[tokio::test]` or explicit `unwrap`.

## Follow-ups

- S06: Zero cargo doc warnings must be maintained — two were outstanding in `lib.rs` and `assay.rs` at S05 completion (unresolved link for `GitHubForge` without forge feature, private item link); fixed in S06/T01.
- Future: add `forge_with_source()` upgrade (D056) if error chain traversal is needed.
- Future: add `#[non_exhaustive]` to public enums (`PrState`, `CiStatus`, `JobPhase`) before crates.io publish to allow adding variants without breaking semver.

## Files Created/Modified

- `crates/smelt-core/Cargo.toml` — keywords, categories, homepage added
- `crates/smelt-core/src/lib.rs` — crate-level doc block; doctest; `#![deny(missing_docs)]`
- `crates/smelt-core/src/assay.rs` — SmeltRunManifest/Session/Spec/Criterion demoted to pub(crate)
- `crates/smelt-core/src/error.rs` — 17 doc comments on named variant fields
- `crates/smelt-core/src/forge.rs` — 10 doc comments on PrState/CiStatus variants and PrStatus fields
- `crates/smelt-core/src/manifest.rs` — 2 doc comments on CredentialStatus variant source fields
- `crates/smelt-core/src/git/mod.rs` — 5 doc comments on GitWorktreeEntry fields
- `crates/smelt-core/src/monitor.rs` — 18 doc comments on JobPhase variants and core RunState fields
- `/tmp/smelt-example/Cargo.toml` — standalone external embedding proof crate
- `/tmp/smelt-example/src/lib.rs` — placeholder lib target
- `/tmp/smelt-example/tests/api.rs` — 3 integration tests

## Forward Intelligence

### What the next slice should know
- `deny(missing_docs)` is now a build-time invariant — any new `pub` item added to smelt-core without a doc comment will fail `cargo build -p smelt-core` at the exact file:line
- `GitHubForge::new` requires a Tokio runtime — callers must be in an async context; this is a sharp edge for documentation and the UAT script
- `[[session]]` blocks require both `harness` and `timeout` fields — the reference example at `examples/job-manifest-forge.toml` is the canonical template

### What's fragile
- `lib.rs` doctest uses `#[cfg(feature = "forge")]` and calls `GitHubForge::new("ghp_token".to_string())?` — if the `forge` feature is not enabled in `cargo test --doc`, the doctest is skipped; the `no_run` attribute prevents actual execution but the snippet must still parse correctly
- `/tmp/smelt-example` is outside the workspace and not in CI — the embedding proof is manual; any API changes that break external callers won't be caught until the test is rerun

### Authoritative diagnostics
- `cd /tmp/smelt-example && cargo test` — rerunnable external embedding proof; compiler errors signal API regressions
- `cargo build -p smelt-core 2>&1 | grep "missing documentation"` — immediate signal for any new undocumented pub item

### What assumptions changed
- Plan assumed doc work would be purely additive; T04 revealed two constructor signature surprises (GitHubForge runtime requirement, session TOML fields) that required empirical discovery and fixes to the usage example.
