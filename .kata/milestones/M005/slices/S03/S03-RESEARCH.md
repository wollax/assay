# S03: Guided Authoring Wizard — Research

**Date:** 2026-03-20

## Summary

S03 delivers `assay plan` — an interactive CLI wizard that collects a milestone goal, a 1–7 chunk breakdown, per-chunk success criteria, and shell verification commands, then writes a complete milestone TOML + `gates.toml` per chunk. The same generation logic is exposed programmatically via two new MCP tools (`milestone_create`, `spec_create`) so agent callers (Claude Code, Codex) can drive the workflow without a TTY.

The codebase is ready for S03: S01 provides `milestone_save()`, the `Milestone`/`ChunkRef` types, and `GatesSpec` with `milestone`/`order` fields. S02 provides the state machine that activates milestones. All generation logic goes in a new `assay-core::wizard` module so CLI and MCP share the same write path. The CLI wires `dialoguer` on top; MCP tools call the same core functions directly.

`dialoguer` is **not** in workspace dependencies — it must be added to root `Cargo.toml` and `crates/assay-cli/Cargo.toml`. Everything else needed (atomic writes, slug validation, spec file templates, MCP tool registration) is already present and can be reused as-is.

## Recommendation

**Build a thin `assay-core::wizard` module with pure functions; the CLI and MCP are just thin wrappers over it.**

Architecture:
1. **`assay-core::wizard`** — `WizardInputs` struct (milestone + chunks + criteria) and `create_from_inputs(inputs, assay_dir, specs_dir) -> Result<WizardResult>`. Pure, sync, no TTY dependency. Calls `milestone_save()` and writes `gates.toml` files directly (like `spec new` does).
2. **`crates/assay-cli/src/commands/plan.rs`** — uses `dialoguer` to collect inputs, converts to `WizardInputs`, calls `create_from_inputs`. Checks `stdin().is_terminal()` first; exits with a clear error when not a TTY.
3. **MCP `milestone_create`** — params: `slug`, `name`, `description`, `chunks: Vec<{slug,name,order}>`. Creates the milestone TOML only (no specs). Thin wrapper over `assay_core::wizard::create_milestone_from_params`.
4. **MCP `spec_create`** — params: `slug`, `name`, `milestone_slug`, `order`, `criteria: Vec<{name, description, cmd: Option<String>}>`. Creates a single chunk `gates.toml` and optionally patches it into the named milestone. Thin wrapper over `assay_core::wizard::create_spec_from_params`.

This mirrors the existing D001/D003 conventions: core is pure functions, CLI/MCP are call-site wrappers.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Atomic TOML writes | `NamedTempFile + write_all + sync_all + persist` in `milestone_save()` | Already tested, crash-safe. Call `milestone_save()` for milestone, write `gates.toml` with same pattern for specs. |
| Slug safety check | `validate_path_component(slug, label)` in `assay-core::history` | `pub(crate)` so accessible from `assay-core::wizard`. Checks for empty, `.`, `..`, `/`, `\`. |
| Spec file template | `handle_spec_new()` in `spec.rs` | Shows exact TOML template format; wizard generates gates.toml with same structure. |
| MCP tool registration | `#[tool(description = "...")]` + auto-discovery via `#[tool_router]` | Just add methods to `AssayServer`; router picks them up automatically (no manual registration). |
| MCP param validation | `domain_error(&e)` + `Parameters<T>` + `resolve_cwd()` | Standard pattern for all existing MCP tools — copy verbatim. |

## Existing Code and Patterns

- `crates/assay-core/src/milestone/mod.rs:milestone_save()` — call this to persist the wizard-generated `Milestone`. Already handles `create_dir_all` for the milestones directory.
- `crates/assay-core/src/history/mod.rs:validate_path_component()` — `pub(crate)`, accessible within `assay-core::wizard`. Validates slugs before writing files.
- `crates/assay-cli/src/commands/spec.rs:handle_spec_new()` — shows exact TOML template for `gates.toml` + `spec.toml`. Wizard only needs `gates.toml` (sufficient for `assay gate run`). Reuse the template format verbatim.
- `crates/assay-cli/src/commands/spec.rs:handle_spec_new()` at line ~279 — pattern for `create_dir_all` + `fs::write` + relative path display output. Follow exactly.
- `crates/assay-mcp/src/server.rs:milestone_list/get/cycle_status` — pattern for new MCP tools. Params struct with `#[derive(Deserialize, JsonSchema)]`, method with `#[tool(description = "...")]`, body starts with `resolve_cwd()` + `cwd.join(".assay")`, returns `domain_error(&e)` on failure.
- `crates/assay-cli/src/commands/mod.rs` — `std::io::IsTerminal` is already imported here. Use `std::io::stdin().is_terminal()` to gate dialoguer calls.
- `crates/assay-types/src/milestone.rs:Milestone` — the type wizard populates. Required fields: `slug`, `name`, `created_at`, `updated_at`. Status defaults to `Draft`; chunks default to empty `Vec`.
- `crates/assay-types/src/gates_spec.rs:GatesSpec` — wizard sets `milestone: Some(slug)` and `order: Some(n)` on each chunk's spec to establish milestone membership.
- `crates/assay-core/src/init.rs:init()` — model for a core function that creates multiple files and returns a summary of what was created (`InitResult { created_files: Vec<PathBuf> }`). Follow the same pattern for `WizardResult`.

## Constraints

- **`dialoguer` not in workspace** — must add `dialoguer = "0.12.0"` to `[workspace.dependencies]` in root `Cargo.toml`, then `dialoguer.workspace = true` in `crates/assay-cli/Cargo.toml`. Do NOT add to `assay-core` (core stays TTY-free).
- **Tool count goes 27 → 29** — existing test `milestone_list_tool_in_router` and `milestone_get_tool_in_router` check tool list by name; they do not assert count. But any test asserting an exact tool count (search for `list_all().len()`) must be updated. Currently none do — verify before assuming safe.
- **`validate_path_component` is `pub(crate)`** — accessible from `assay-core::wizard` because it's in the same crate. Wizard can import it as `use crate::history::validate_path_component`.
- **GatesSpec has `deny_unknown_fields`** — wizard must construct `GatesSpec` Rust structs and serialize with `toml::to_string()`, not string templating. String templates risk field ordering or quoting issues; serde ensures correctness. Exception: the milestone's human-readable gates.toml can use `toml::to_string_pretty()`.
- **Milestone TOML format** — `created_at`/`updated_at` are required fields (`DateTime<Utc>`). Always set both to `Utc::now()` at wizard call time.
- **`--features assay-types/orchestrate` still required** for `cargo test -p assay-core` standalone (pre-existing manifest.rs bug from S01). Integration tests in `crates/assay-core/tests/wizard.rs` must use this flag.
- **Non-TTY behavior** — `dialoguer` panics or hangs on non-TTY stdin. Always check `std::io::stdin().is_terminal()` before calling any dialoguer prompt; return `Err` or exit with a user-friendly message pointing to `milestone_create` MCP tool.
- **Slug uniqueness** — wizard should check that a milestone slug does not already exist in `.assay/milestones/` before writing. Return error if collision detected.

## Common Pitfalls

- **GatesSpec struct literal non-exhaustiveness** — S01 discovered that adding fields to `GatesSpec` propagated to all test sites. `wizard.rs` constructs `GatesSpec` struct literals; if other test files do too, a workspace-wide `cargo test` run will catch issues. Use `..Default::default()` if available, or explicit construction.
- **`assay spec new` writes both `spec.toml` and `gates.toml`; wizard only needs `gates.toml`** — `spec.toml` contains SRS-style requirements metadata that the wizard doesn't collect. Omitting it is correct; `load_spec_entry_with_diagnostics` returns `SpecEntry::Directory { gates, spec: None }` when `spec.toml` is absent, which is fine for `assay gate run`.
- **MCP tool with Vec parameter** — `milestone_create` needs `chunks: Vec<ChunkParams>` in its params struct. JSON-schema for nested Vec structs works with `#[derive(Deserialize, JsonSchema)]` on the inner type. Declare `ChunkParams` as a separate struct next to `MilestoneCreateParams`.
- **`spec_create` idempotency** — MCP tool should reject creation if `<specs_dir>/<slug>/gates.toml` already exists, same as `spec new` rejects an existing directory. Return `domain_error` with clear message.
- **Slug generation edge cases** — user may type "My Feature!" → slugify to "my-feature". Simple rules: `.to_lowercase()`, replace `[^a-z0-9]+` with `-`, `.trim_matches('-')`. A helper `fn slugify(s: &str) -> String` in `wizard.rs` is enough. Must also validate the slugified result with `validate_path_component`.
- **Milestone `status` field on disk** — wizard creates milestones as `Draft`; the milestone won't appear in `cycle_status` until transitioned to `InProgress`. This is correct behavior (user or agent must explicitly start the milestone), but worth documenting in the wizard's confirmation output.
- **`toml::to_string` vs `toml::to_string_pretty`** — use `to_string_pretty` for human-edited files (milestone TOML, gates.toml) so the output is readable. Same as `milestone_save` already does internally.

## Open Risks

- **dialoguer on tmux / non-standard terminals** — dialoguer's Input/Select prompts use ANSI escape sequences. Most terminals handle these fine; tmux usually works. The `assay plan` command is a one-time authoring action (not hot path), so any edge-case behavior is low impact. Mitigation: TTY check before entering wizard avoids non-interactive hangs.
- **Criterion count per chunk** — wizard currently has no cap on criteria count. A user entering 20 criteria per chunk is valid but creates a long wizard session. Consider capping at 10 and providing a note that users can edit the file after. This is a UX decision, not a correctness risk.
- **`spec_create` must patch milestone** — if `milestone_slug` is provided, `spec_create` should add the new `ChunkRef` to the milestone's `chunks` field and call `milestone_save`. This requires `milestone_load` + mutation + `milestone_save` — a read-modify-write cycle. Race condition if two MCP calls run concurrently, but this is acceptable (same issue as any file-based workflow tool; single-agent use is the norm).
- **MCP test count assertions** — if any existing test explicitly asserts 27 tools, adding 2 breaks it. Search result shows no `list_all().len()` assertion — but double-check before finalizing T02.

## File Creation Plan

```
crates/assay-core/src/wizard.rs           ← new: WizardInputs, ChunkInput, CriterionInput,
                                               WizardResult, create_from_inputs(),
                                               create_milestone_from_params(),
                                               create_spec_from_params(), slugify()
crates/assay-core/src/lib.rs              ← add: pub mod wizard
crates/assay-core/tests/wizard.rs         ← new: integration tests for create_from_inputs,
                                               idempotency checks, non-existent milestone error
crates/assay-cli/src/commands/plan.rs     ← new: PlanCommand, handle(), collect_wizard_inputs()
crates/assay-cli/src/commands/mod.rs      ← add: pub mod plan
crates/assay-cli/src/main.rs              ← add: Plan variant + dispatch arm
crates/assay-mcp/src/server.rs            ← add: MilestoneCreateParams, SpecCreateParams,
                                               ChunkParams, CriterionParams, milestone_create(),
                                               spec_create() methods + tests
Cargo.toml (workspace)                    ← add: dialoguer = "0.12.0"
crates/assay-cli/Cargo.toml               ← add: dialoguer.workspace = true
```

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| dialoguer (Rust interactive prompts) | — | none found |
| Rust CLI wizards | — | none found |

## Sources

- `dialoguer` version "0.12.0" confirmed from `cargo search dialoguer` output — API: `Input::new().with_prompt().interact_text()`, `Select::new().with_prompt().items().interact()`, `Confirm::new().with_prompt().interact()`
- `validate_path_component` in `assay-core/src/history/mod.rs` — `pub(crate)`, checks empty/`..`/`/`/`\`; accessible from `assay-core::wizard`
- `spec new` template in `crates/assay-cli/src/commands/spec.rs:handle_spec_new()` — authoritative reference for `gates.toml` structure
- MCP tool pattern confirmed from `cycle_status`/`cycle_advance` implementations in `server.rs`
- Tool count = 27 confirmed by `grep -c "#\[tool" server.rs | grep -v tool_router|tool_handler`
- S01 Forward Intelligence: `MilestoneStatus::Default = Draft`; milestone_load overwrites slug from filename
- S02 Forward Intelligence: `--features assay-types/orchestrate` required for standalone crate tests
