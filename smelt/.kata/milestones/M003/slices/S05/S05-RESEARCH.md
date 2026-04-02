# S05: smelt-core Library API — Research

**Date:** 2026-03-21

## Summary

S05 is a polish slice: the core logic is complete (forge types from S01, manifest from S02, status/watch from S03, state paths from S04); S05's job is to make `smelt-core` a **presentable, embeddable Rust library** — clean public API, comprehensive doc comments, correct Cargo metadata, and a proof-of-concept external crate that validates the embedding story.

There are exactly **53 `missing_docs` errors** blocking `#![deny(missing_docs)]`, spread across six files. The fixes are mechanical doc comment additions — no architectural changes. The `pub use` re-exports in `lib.rs` are largely complete; minor gaps exist. Cargo metadata (`keywords`, `categories`, `homepage`) is missing. The `smelt-example` test crate is the only new artifact that needs creation from scratch.

The main design tension is **API surface curation**: several `pub` types in `assay.rs` (`SmeltRunManifest`, `SmeltManifestSession`, `SmeltSpec`, `SmeltCriterion`) and `git/mod.rs` (`parse_porcelain`, `GitWorktreeEntry`) are currently exported and reach the docs, but they are implementation details, not embedding targets. S05 must decide whether to restrict their visibility or document them fully. The roadmap's canonical `pub use` list omits `AssayInvoker` and Assay serde types — restricting them to `pub(crate)` is the cleanest path and avoids documenting internal types.

## Recommendation

**Three sequential tasks:**

1. **API surface curation** — Decide visibility for internal types before writing docs. Downgrade `SmeltRunManifest`, `SmeltManifestSession`, `SmeltSpec`, `SmeltCriterion`, `parse_porcelain`, and `GitWorktreeEntry` to `pub(crate)`. This eliminates ~6 of the 53 missing_docs errors automatically and avoids committing to a public API for Assay's internal translation format. Check whether any smelt-cli code references these types (unlikely — they're consumed inside assay.rs).

2. **Doc comments + `#![deny(missing_docs)]`** — File-by-file mechanical pass in this order: `error.rs` (17), `monitor.rs` (18), `forge.rs` (10), then verify `git/mod.rs`, `manifest.rs`, `assay.rs`. Add `#![deny(missing_docs)]` to `lib.rs` only after the count hits zero. Simultaneously: add crate-level usage example to `lib.rs` doc comment, add `keywords`/`categories`/`homepage` to `Cargo.toml`.

3. **`smelt-example` external crate** — Create `/tmp/smelt-example/` as a standalone Cargo project with `smelt-core` as a path dependency (`features = ["forge"]`). Single `#[test]` in `tests/api.rs` that calls `GitHubForge::new("test-token")`, `JobManifest::from_str(...)`, and `DockerProvider::new()` (allowing the last to fail if daemon absent). `cargo test` in that directory must compile and pass. Document the creation steps in the slice summary.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Counting missing_docs errors | `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps` | Gives exact line numbers; run after each file to track progress |
| Verifying forge feature isolation | `cargo tree -p smelt-core \| grep octocrab` (no feature) vs `cargo tree -p smelt-core --features forge \| grep octocrab` | Already proven in S01; re-run as regression check |
| External crate path dep | Standard `[dependencies] smelt-core = { path = "../../smelt/crates/smelt-core", features = ["forge"] }` | No workspace needed; simplest structure |
| Doc example compilation | `` ```rust,no_run `` fence in lib.rs doc | `no_run` prevents Docker/network calls in doctest; `cargo doc --no-deps` still compiles the example |

## Existing Code and Patterns

- `crates/smelt-core/src/lib.rs` — Single-line `//!` crate doc. Already has most `pub use` items. Add `#![deny(missing_docs)]` here once errors hit zero. The re-exports closely match the roadmap list; confirm `JobPhase`, `compute_job_timeout`, `BranchCollectResult`, `CollectResult` are all included.
- `crates/smelt-core/src/forge.rs` — 10 missing_docs errors: `PrState` variants (Open, Merged, Closed), `CiStatus` variants (Pending, Passing, Failing, Unknown), `PrStatus` fields (state, ci_status, review_count). All are one-liner descriptions.
- `crates/smelt-core/src/error.rs` — 17 missing_docs errors: all struct-variant fields (`operation`, `message`, `field`, `session`, `files`, `provider`, `path`, `source`). These are named fields on enum variants — `thiserror` doesn't auto-generate docs. Add `/// …` above each field.
- `crates/smelt-core/src/monitor.rs` — 18 missing_docs errors: all `RunState` fields plus `JobPhase` variants plus `JobMonitor.state` field plus `compute_job_timeout`. The `RunState` fields already have some doc comments (pr_url, pr_number, etc.) but others (job_name, phase, container_id, sessions, started_at, updated_at, pid) are undocumented.
- `crates/smelt-core/src/assay.rs` — `SmeltRunManifest`, `SmeltManifestSession`, `SmeltSpec`, `SmeltCriterion` are `pub` but are Assay's internal translation format. **Candidate for `pub(crate)` demotion.** If kept pub, their fields need docs. Simpler to restrict.
- `crates/smelt-core/src/git/mod.rs` — `parse_porcelain` and `GitWorktreeEntry` fields are candidates for `pub(crate)`. `GitOps` trait and `preflight` are legitimate library API and need full docs. `GitCli` is pub; check if it should be library API or `pub(crate)`.
- `crates/smelt-core/Cargo.toml` — Has `description` but missing `keywords`, `categories`, `homepage`. Categories: `"command-line-utilities"`, `"development-tools"`. Keywords: `"docker"`, `"ci"`, `"github"`, `"assay"`, `"infrastructure"`.

## Missing-Docs Error Map (53 total)

| File | Count | Primary items |
|------|-------|---------------|
| `monitor.rs` | 18 | All `RunState` fields (job_name, phase, container_id, sessions, started_at, updated_at, pid); all `JobPhase` variants; `JobMonitor.state`; `compute_job_timeout` |
| `error.rs` | 17 | Named fields on `GitExecution`, `MergeConflict`, `Manifest`, `Provider` (3 fields), `Forge`, `Credential`, `Config`, `Io` variants |
| `forge.rs` | 10 | `PrState` variants (3); `CiStatus` variants (4); `PrStatus` fields (state, ci_status, review_count) |
| `git/mod.rs` | 5 | `GitWorktreeEntry` fields (path, head, branch, is_bare, is_locked); `parse_porcelain` |
| `manifest.rs` | 2 | `CredentialStatus::Resolved.source` and `CredentialStatus::Missing.source` fields |
| `assay.rs` | 1 | One undocumented item (likely a struct field) |

Downgrading `SmeltRunManifest`, `SmeltManifestSession`, `SmeltSpec`, `SmeltCriterion` to `pub(crate)` will drop the `assay.rs` count and potentially some from `git/mod.rs` (if `parse_porcelain`/`GitWorktreeEntry` follow suit), bringing the mechanical doc-comment work to ~50 items.

## Current `pub use` in `lib.rs` vs Roadmap

| Roadmap item | Currently exported? |
|---|---|
| `docker::DockerProvider` | ✅ `pub use docker::DockerProvider` |
| `forge::ForgeClient` | ✅ |
| `forge::GitHubForge` | ✅ (feature-gated) |
| `forge::PrHandle` | ✅ |
| `forge::PrStatus` | ✅ |
| `forge::ForgeConfig` | ✅ |
| `manifest::JobManifest` | ✅ |
| `monitor::JobMonitor` | ✅ |
| `monitor::RunState` | ✅ |
| `monitor::JobPhase` | ✅ |
| `collector::ResultCollector` | ✅ |
| `collector::BranchCollectResult` | ✅ |
| `provider::RuntimeProvider` | ✅ |
| `provider::ContainerId` | ✅ |
| `provider::ExecHandle` | ✅ |
| `monitor::compute_job_timeout` | ✅ |
| `assay::AssayInvoker` | ✅ (but may want to hide as pub(crate)) |

The existing re-exports in `lib.rs` are complete relative to the roadmap. No new `pub use` lines are needed. The only question is whether `AssayInvoker` and `SmeltRunManifest`-family should be hidden.

## Constraints

- `#![deny(missing_docs)]` must be added to `lib.rs`, not to individual modules — a crate-level attribute applies to all `pub` items in the crate.
- `#[cfg(feature = "forge")]`-gated types still need doc comments — `GitHubForge`'s impl block items need `///` docs even though they're feature-gated; `deny(missing_docs)` checks within the feature-enabled compile too.
- `pub(crate)` on types currently used in tests: any `#[cfg(test)]` in smelt-core that references `SmeltRunManifest` etc. is unaffected — `pub(crate)` is still visible within the crate. Verify smelt-cli does **not** reference `SmeltRunManifest` directly (it probably doesn't; `AssayInvoker`'s methods are what's called).
- Workspace edition is **2024** — Rust 1.85+; all RPITIT features work; `let ... && ...` chains allowed in if-let.
- `smelt-example` lives at `/tmp/smelt-example/` (not inside the workspace) to prove external embedding. It must not be added to the workspace `Cargo.toml`. The path in its dep must be an absolute path or relative path pointing back to `smelt-core`.
- Doctest `no_run` is required for any example that provisions Docker or calls the network. Use `ignore` only if the example is too complex to compile in isolation.

## Common Pitfalls

- **`#![deny(missing_docs)]` added before errors are fixed** — Will immediately break `cargo build`. Add it as the final step of the doc-writing pass, not the first. Use `RUSTDOCFLAGS="-D missing_docs" cargo doc` (not `cargo build`) as the iterative check — it runs the doc linter without breaking the normal build.
- **Enum variant named fields need per-field `///` docs** — `thiserror` enum variants with named fields (e.g. `GitExecution { operation, message }`) require a `/// doc` on each field, not just on the variant. The `#[source]` field in `Provider` needs documentation too.
- **`forge.rs` missing_docs count includes forge-feature-gated items** — `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps` doesn't enable the forge feature; run with `--features forge` to catch `GitHubForge`-related items separately. Both runs must be clean.
- **`smelt-example` path dependency must be absolute or correct relative** — `/tmp/smelt-example/` is not inside the smelt repo. The path in `Cargo.toml` should be `path = "/absolute/path/to/smelt/crates/smelt-core"`. Test with `cargo test` from inside `/tmp/smelt-example/`.
- **`GitHubForge::new("test-token")` returns `Result<Self>`** — The example test must handle the `Result`, e.g. `let _forge = smelt_core::GitHubForge::new("test-token".to_string()).expect("client build")`. The client build itself doesn't make network calls.
- **`DockerProvider::new()` may panic if Docker is unavailable** — Use `.ok()` or a `.is_err()` check in the example test rather than `.unwrap()`. The test should not fail in environments without Docker.
- **Crate-level doc comment usage example** — Enclose in `` ```rust,no_run `` to prevent doctest from trying to connect to Docker or GitHub. Keep it short and illustrative; the goal is showing import paths, not a full workflow.

## Open Risks

- `AssayInvoker` pub vs pub(crate): smelt-cli references `AssayInvoker` directly in `run.rs`. Downgrading to `pub(crate)` would break smelt-cli since it's a different crate. **Check the exact usage in `run.rs` before downgrading.** If smelt-cli calls `AssayInvoker::build_spec_toml()` etc., then `AssayInvoker` must remain `pub` (and its API must be documented) or refactored into a smelt-cli-local type.
- The `forge.rs` missing_docs count of 10 includes `PrStatus` fields — but `PrStatus` is not `#[derive(Serialize, Deserialize)]`. If S05 decides to add these derives for completeness (e.g. for embedding), that's a minor addition but must be verified against TOML compat.
- `GitWorktreeEntry` and `parse_porcelain` — if kept pub, they need full documentation. If downgraded to `pub(crate)`, remove the `pub use git::{GitCli, GitOps, preflight}` from lib.rs selectively — keep `GitCli`, `GitOps`, `preflight` (legitimate library API) but hide `parse_porcelain` and `GitWorktreeEntry` (internal parsing helpers).
- `smelt-example` dependency on `tokio` — the test will need `#[tokio::test]` for async calls. Add `tokio = { version = "1", features = ["rt-multi-thread", "macros"] }` to the example's dev-dependencies.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust library API design | none | none found |

## Sources

- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps` — 53 errors identified across 6 files; exact line numbers pinpointed (source: local run)
- `cargo tree -p smelt-core | grep octocrab` — confirms zero octocrab without feature flag (source: local run)
- `cargo test --workspace -q` — 124 tests passing; baseline confirmed clean (source: local run)
- S01–S04 summaries — Forward Intelligence sections document all API boundaries, forge context in RunState, pub use patterns, and known gaps that S05 must close
