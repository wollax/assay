# S01: Project Scaffold & Manifest Foundation — Research

**Date:** 2026-03-17

## Summary

S01 guts the v0.1.0 orchestration code (~9400 lines in smelt-core, plus CLI commands) and replaces it with the new infrastructure-layer foundation: a new manifest format for Assay-exported TOML, a `RuntimeProvider` trait, a new error enum, and a `SmeltConfig` loader. The existing codebase uses Rust 2024 edition, a `smelt-cli` / `smelt-core` workspace, and already has well-structured TOML parsing with serde + `toml` crate. The v0.1.0 manifest (`session/manifest.rs`) is heavily oriented around orchestration concerns (DAG deps, merge strategy, scripted sessions) that no longer belong in Smelt — it must be replaced entirely, not adapted.

The new manifest schema is straightforward TOML with sections for job metadata, environment (image, resources), credentials, sessions (Assay-defined), and merge config. The workspace structure stays. Dependencies like `serde`, `toml`, `thiserror`, `tokio`, `clap`, and `chrono` are already in the workspace and reusable. New dependencies needed: `bollard` (added to workspace deps in this slice for the trait, used in S02), and potentially `url` for repo URL validation.

The main risk is getting the manifest schema right without a finalized Assay export format. Mitigation: define what Smelt needs, validate strictly, and treat the schema as the contract. Assay adapts its export to match.

## Recommendation

1. **Delete all v0.1.0 modules** in smelt-core: `orchestrate/`, `session/`, `merge/`, `worktree/`, `ai/`, `summary/`, `init.rs`, and corresponding CLI commands. Keep `git/` (cli.rs and mod.rs) — the `GitOps` trait and `GitCli` impl are reusable for result collection in later slices.
2. **New smelt-core module structure**: `manifest.rs`, `provider.rs`, `error.rs`, `config.rs`, `lib.rs`. Minimal — only what S01 needs.
3. **Manifest schema**: TOML with `[job]`, `[environment]`, `[credentials]`, `[[session]]`, `[merge]` top-level sections. Parse with `serde` + `toml` (already in workspace). Validate with dedicated methods (unique session names, required fields, valid image refs).
4. **RuntimeProvider trait**: Async trait with methods for provision, exec, collect, teardown. Defined in S01, implemented by `DockerProvider` in S02.
5. **SmeltConfig**: Load from `.smelt/config.toml` — minimal for now (default image, credential sources). Not the manifest — this is project-level config.
6. **CLI**: Replace all subcommands with `smelt run` (with `--dry-run` flag for S01 verification), `smelt status`, `smelt teardown`. S01 only implements `--dry-run` parsing + validation.
7. **Remove workspace deps** no longer needed after gut: `genai`, `petgraph`, `globset`, `similar`, `dialoguer`, `libc`, `indicatif`, `comfy-table`, `which`.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| TOML parsing | `toml` crate (already in workspace) | Battle-tested, serde integration |
| Error types | `thiserror` (already in workspace) | Derive macro, clean error enums |
| Async runtime | `tokio` (already in workspace) | Already committed to tokio ecosystem |
| CLI parsing | `clap` derive (already in workspace) | Already committed, derive API is clean |
| Docker API | `bollard` (to add) | De facto Rust Docker client, async/tokio, D005 decision |
| Date/time | `chrono` (already in workspace) | Already in workspace, serde support |

## Existing Code and Patterns

- `crates/smelt-core/src/session/manifest.rs` — v0.1.0 manifest parsing. Good reference for serde + toml patterns, validation structure, and test organization. **Do not adapt** — the schema is entirely different. Delete and rewrite.
- `crates/smelt-core/src/error.rs` — Current `SmeltError` enum with `thiserror`. Rewrite with new variants (manifest, provider, credential, config errors). Keep the `SmeltError::io()` convenience constructor pattern.
- `crates/smelt-core/src/git/cli.rs` + `git/mod.rs` — `GitOps` trait and `GitCli` impl (~1460 lines). **Keep** — reusable for repo operations in S03+. May need minor cleanup to remove references to deleted modules (e.g., `WorktreeInfo` imports).
- `crates/smelt-core/src/orchestrate/types.rs` — `RunState`, `SessionRunState`, `FailurePolicy`. Reference for state machine patterns. **Delete** — Assay owns orchestration state now.
- `crates/smelt-cli/src/main.rs` — CLI structure with clap derive. Replace subcommands but keep the same patterns (preflight, tracing init, error handling).
- `examples/agent-manifest.toml` — v0.1.0 manifest example. Replace with new format example.
- `Cargo.toml` (workspace) — Workspace dependency management. Keep the pattern, update deps.

## Constraints

- **Rust edition 2024, toolchain 1.93.1** — Can use `let-else`, `async fn in traits` (natively, no `async-trait` needed), and other modern Rust features.
- **Workspace structure must remain** `smelt-cli` + `smelt-core`. Adding new crates is allowed but not needed for M001.
- **No crate dependency on Assay** (D002) — Smelt consumes Assay's output via process boundary. The manifest schema is the contract.
- **bollard is the chosen Docker client** (D005) — Add to workspace deps in S01 (for trait signatures), implement in S02. Fallback to `docker` CLI shelling if exec API proves unreliable.
- **Pre-built images only** (D012) — No Dockerfile building. Manifest specifies image name + tag.
- **Assay generates manifests, Smelt consumes them** (D010) — Schema validation is strict. Unknown fields should error, not be silently ignored.
- **git/ module references** — `git/mod.rs` imports `WorktreeInfo` from `worktree` module. This import must be updated or the worktree types retained minimally when gutting.

## Common Pitfalls

- **Leaving dead code after gut** — The `lib.rs` re-exports everything. A partial delete will cause cascading compile errors. Delete all v0.1.0 modules at once, rewrite `lib.rs` from scratch, then fix `Cargo.toml` deps.
- **Over-designing the RuntimeProvider trait** — S01 only defines the trait. Keep it minimal (5-6 async methods). Let S02-S04 drive the real API shape. Methods that aren't needed until S03+ can be added then.
- **Manifest schema drift** — The Assay export format isn't finalized. Design Smelt's expected input schema and test against it. Don't try to be flexible — strict validation catches integration bugs early.
- **git module entanglement** — `git/cli.rs` and `git/mod.rs` import types from `worktree/` module. Check all imports before deleting worktree. May need to inline the `GitWorktreeEntry` type into `git/mod.rs` or create a minimal types module, or just remove worktree-related methods from `GitOps` since they're no longer needed.
- **serde deny_unknown_fields** — Use `#[serde(deny_unknown_fields)]` on manifest types to catch schema mismatches early. The v0.1.0 manifest doesn't do this.

## Open Risks

- **Manifest schema stability** — Assay M002 is still evolving. The TOML schema Smelt defines here becomes the integration contract. If Assay's export changes shape significantly after S01, manifest types need rework. Mitigation: keep manifest types simple and well-tested; schema changes are mechanical serde updates.
- **git module cleanup scope** — The `git/` module is 1460 lines with worktree-specific methods. Cleaning it up without breaking the reusable parts (branch ops, commit, push) needs careful attention. Could be deferred to S02 if risky.
- **Workspace dep cleanup** — Removing unused workspace deps (`genai`, `petgraph`, etc.) might break if any dev-dependency or test still references them indirectly. Run `cargo check` after cleanup.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| bollard (Rust Docker) | — | none found |
| Rust async patterns | `wshobson/agents@rust-async-patterns` | available (4.4K installs) |
| Rust best practices | `apollographql/skills@rust-best-practices` | available (2.7K installs) |
| Docker multi-stage | `pluginagentmarketplace/custom-plugin-docker@docker-multi-stage` | available (not relevant — S01 doesn't build images) |

No skills are directly relevant to S01's core work (manifest parsing, trait definition, code deletion). The Rust skills are generic best-practices guides — useful but not critical.

## Sources

- Existing codebase analysis (all findings from direct code reading)
- D001–D013 decisions from `.kata/DECISIONS.md`
- R001 (manifest parsing) is the primary requirement for this slice
