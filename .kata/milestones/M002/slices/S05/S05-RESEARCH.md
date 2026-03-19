# S05: Harness CLI & Scope Enforcement — Research

**Date:** 2026-03-17

## Summary

S05 adds the `assay harness` CLI subcommand family (generate, install, update, diff) and scope enforcement logic (file_scope + shared_files via globset matching, ScopeViolation detection, multi-agent awareness prompts). This is a low-risk slice because all three adapters already exist with identical function signatures (`generate_config`, `write_config`, `build_cli_args`), the CLI infrastructure is well-established with 8 existing subcommand modules, and the scope enforcement is a new ~80-line pure-logic module with no concurrency concerns.

The primary deliverables are: (1) a new `assay harness` CLI subcommand with generate/install/update/diff sub-subcommands dispatching to the three adapters, (2) scope enforcement types (`ScopeConfig`, `ScopeViolation`) and a `check_scope()` function using globset matching, (3) a `generate_scope_prompt()` function that produces multi-agent awareness text injected into harness config as an additional prompt layer, and (4) a pre-generation enrichment step that injects scope context into HarnessProfile before passing to existing adapter functions.

The scope and multi-agent awareness work belongs in `assay-harness/src/scope.rs` (harness-layer concern, not core orchestration). The CLI subcommand goes in `assay-cli/src/commands/harness.rs`. New types (`ScopeConfig`, `ScopeViolation`) go in `assay-types`.

## Recommendation

**Approach: CLI dispatch module + scope module in assay-harness + types in assay-types.**

1. **CLI (`assay-cli/src/commands/harness.rs`):** New clap subcommand `Harness` with sub-subcommands: `Generate { adapter, spec, workflow }`, `Install { adapter }`, `Update { adapter }`, `Diff { adapter }`. The `generate` subcommand dispatches to the matching adapter's `generate_config()` + prints to stdout (or `write_config()` to disk with `--output-dir`). `install` calls `write_config()` into the project root. `update` compares existing files and overwrites only changed ones. `diff` compares and prints differences without writing.

2. **Scope types (`assay-types`):** `ScopeConfig { file_scope: Vec<String>, shared_files: Vec<String> }` on `ManifestSession` (serde default, backward-compat). `ScopeViolation { file: String, violation_type: ScopeViolationType }` for enforcement results.

3. **Scope enforcement (`assay-harness/src/scope.rs`):** `check_scope(file_scope, shared_files, changed_files) -> Vec<ScopeViolation>` using `globset::GlobSet` for matching. `generate_scope_prompt(session, all_sessions) -> String` producing multi-agent awareness markdown injected as a `PromptLayer` with `PromptLayerKind::System` and high priority.

4. **Adapter integration:** Rather than modifying each adapter's `generate_config()` signature (breaking change), add a pre-generation step that injects scope prompt layers into the `HarnessProfile.prompt_layers` before passing to the existing `generate_config()`. This keeps adapters pure and scope logic centralized.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Glob pattern matching | `globset` crate (v0.4.18) | Battle-tested, supports `**` recursive globs, compiles patterns into efficient automaton. ~3 transitive deps (regex crate family). The alternative — hand-rolling glob matching — is error-prone for edge cases like `**/*.rs` and `{src,tests}/**`. Per D027. |
| CLI argument parsing | `clap` (already in workspace) | All 8 existing command modules use clap derive. Follow the pattern. |
| File diffing for `harness diff` | Simple string comparison | Config files are small (<10KB). Compare old vs new content, report changed/added/removed files. No need for a diff library — just report which files differ. |

## Existing Code and Patterns

- `crates/assay-cli/src/commands/worktree.rs` — Best template for a new subcommand module. Shows the clap Subcommand pattern, project_root/assay_dir lookups, and error handling. The `harness` module will follow identical structure.
- `crates/assay-cli/src/commands/run.rs` — Shows how harness_writer closure composes adapter functions. Lines 101-108 show the concrete claude adapter composition pattern: `generate_config(profile)` → `write_config(&config, path)` → `build_cli_args(&config)`. The `harness generate` command uses the same dispatch but without the full pipeline.
- `crates/assay-harness/src/claude.rs`, `codex.rs`, `opencode.rs` — All three share identical function signatures: `generate_config(&HarnessProfile) -> XConfig`, `write_config(&XConfig, &Path) -> io::Result<()>`, `build_cli_args(&XConfig) -> Vec<String>`. Note: `generate_config` takes only `&HarnessProfile` (no Path parameter). CLI dispatch is a simple match on adapter name string.
- `crates/assay-harness/src/prompt.rs` — `build_prompt()` assembles prompt layers by priority. Scope prompt will be injected as an additional `PromptLayer` into `HarnessProfile.prompt_layers` before calling `generate_config()`, not by modifying the builder.
- `crates/assay-types/src/manifest.rs` — `ManifestSession` already has `depends_on: Vec<String>` with `#[serde(default)]`. New `file_scope` and `shared_files` fields follow the same pattern. Type uses `deny_unknown_fields`.
- `crates/assay-types/src/harness.rs` — `HarnessProfile` has `prompt_layers: Vec<PromptLayer>`. `PromptLayerKind` enum has `System`, `Project`, `Spec`, `Custom`. Scope awareness injected as a `PromptLayer` with kind `System` and high negative priority (e.g., -100) to place it early.
- `crates/assay-core/src/pipeline.rs` — `build_harness_profile()` constructs `HarnessProfile` from `ManifestSession`. Scope enrichment happens after this function, before adapter dispatch. `HarnessWriter` type alias: `dyn Fn(&HarnessProfile, &Path) -> Result<Vec<String>, String>`.
- `crates/assay-cli/src/commands/mod.rs` — Command module registry and shared helpers (colors, formatting). New `pub mod harness;` added here. Currently has 8 modules: checkpoint, context, gate, init, mcp, run, spec, worktree.

## Constraints

- **Adapter function signatures are stable:** `generate_config(&HarnessProfile) -> XConfig` takes no Path — it produces an in-memory config. `write_config(&XConfig, &Path)` handles disk I/O. Do not modify these signatures. Scope enrichment happens by mutating `HarnessProfile.prompt_layers` before calling `generate_config()`.
- **No new PromptLayerKind variant needed:** Scope prompts use `PromptLayerKind::System` with a negative priority (e.g., -100) to sort before other layers. Adding a `Scope` variant would require schema snapshot updates across all adapter tests and is unnecessary — the content, not the kind, carries the scope semantics.
- **Backward compatibility on ManifestSession:** New `file_scope` and `shared_files` fields must use `#[serde(default, skip_serializing_if = "Vec::is_empty")]` so existing manifests without scope declarations still parse (same pattern as `depends_on`).
- **globset is a new workspace dependency:** Must be added to root `Cargo.toml` workspace dependencies and `assay-harness/Cargo.toml`. Per D027, the decision says assay-core, but scope *prompt generation* is a harness concern. Place `check_scope()` in assay-harness, `ScopeViolation` type in assay-types.
- **`deny_unknown_fields` on ManifestSession:** Already present — new fields are additive with serde defaults, so no breaking change.
- **Schema snapshots:** Modified `ManifestSession` schema (new fields) will need snapshot updates in `assay-types/tests/schema_snapshots.rs`. New types (`ScopeViolation`) need new snapshots.
- **`assay-harness` depends on `assay-core` and `assay-types`:** Already the case. No new dependency edges between crates.
- **Zero-trait convention (D001):** Adapter dispatch in CLI uses match, not trait dispatch. Scope checking is a plain function.

## Common Pitfalls

- **Glob pattern syntax mismatch:** Users may expect shell glob syntax (`*.rs`) vs globset syntax (`**/*.rs`). globset uses its own syntax which is close to gitignore. Document in field doc-comments that patterns use globset/gitignore syntax. Provide examples in manifest TOML comments.
- **Scope enforcement is advisory, not blocking:** Per D027, scope violations are advisory by default — they don't prevent execution. The `check_scope()` function returns violations; callers decide policy. Don't accidentally make scope enforcement blocking in the CLI or orchestrator.
- **`harness diff` showing secrets:** Config files may contain sensitive data (API keys in MCP config). The diff output should show file paths and whether content changed, not dump full file contents. Use a "files changed/added/removed" summary, not a line-by-line diff.
- **Empty file_scope means "all files":** If `file_scope` is empty, the session has no restrictions (owns everything). Only non-empty `file_scope` patterns enable enforcement. Document this clearly.
- **`harness install` vs `write_config`:** `install` writes to the project root (current directory), not a worktree. `write_config()` already handles directory creation. Ensure `install` uses project root, not some worktree path.
- **ManifestSession schema snapshot breakage:** Adding `file_scope` and `shared_files` fields to `ManifestSession` will change the `manifest-session-schema` snapshot. Run `cargo insta review` to accept. The `run-manifest-schema` snapshot will also change transitively.

## Open Risks

- **`harness update` semantics are underspecified:** What does "incremental update" mean for harness config? If the user manually edited CLAUDE.md, does update overwrite their changes? Recommendation: `update` regenerates and overwrites all managed files. Users who want to preserve manual edits should use `diff` first. This matches typical config-management tool behavior (terraform, helm).
- **Multi-agent prompt verbosity:** The scope prompt for a 5-session manifest could be lengthy (listing all sessions, their scopes, shared files). Need to keep it concise — agents have limited context windows. Recommendation: list only direct neighbors (dependencies and dependents) and shared file declarations, not all sessions.
- **globset compilation cost:** `GlobSet::new()` compiles patterns into a regex automaton. For typical manifests (5-20 patterns), this is sub-millisecond. Not a real risk, but `GlobSet` should be compiled once per scope check invocation, not per-file.
- **OpenCode/Codex config format fragility:** Per S04 forward intelligence, adapter output formats are based on research, not official schema validation. `harness install` for these adapters may produce configs that need manual tweaking for newer agent versions.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust clap | `pproenca/dot-skills@rust-clap` (63 installs) | available — potentially useful for clap subcommand patterns, but existing codebase has 8 examples to follow |
| globset | none found | n/a — well-documented crate, no skill needed |

No skills are directly relevant enough to recommend installing. The existing codebase provides 8 CLI command modules as templates, and globset has excellent docs.

## Sources

- globset crate API: `GlobSetBuilder::new()`, `add()`, `build()`, `matches()` — standard glob matching with gitignore semantics. (source: crates.io globset 0.4.18)
- Existing adapter signatures verified from codebase: `generate_config(&HarnessProfile) -> XConfig` (no Path), `write_config(&XConfig, &Path) -> io::Result<()>`, `build_cli_args(&XConfig) -> Vec<String>`.
- D027 decision: globset-based scope checking, advisory by default, ~80 lines.
- D001/D009/D015: Zero-trait convention, JSON persistence, closure-based control inversion.
- Boundary map S04→S05 and S05→S06: produces `check_scope()`, `generate_scope_prompt()`, updated adapters accepting scope context, and `assay harness generate|install|update|diff` CLI subcommands.
