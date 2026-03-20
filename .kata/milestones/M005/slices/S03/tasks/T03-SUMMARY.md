---
id: T03
parent: S03
milestone: M005
provides:
  - "`assay plan` CLI command wired into assay-cli with dialoguer interactive flow"
  - "TTY guard returning Ok(1) with `milestone_create` MCP hint for non-interactive callers"
  - "`plan_non_tty_returns_1` unit test"
key_files:
  - crates/assay-cli/src/commands/plan.rs
  - crates/assay-cli/src/commands/mod.rs
  - crates/assay-cli/src/main.rs
  - Cargo.toml
  - crates/assay-cli/Cargo.toml
key_decisions:
  - "dialoguer 0.12.0 added as workspace dependency; no version pin elsewhere needed"
  - "TTY guard uses `std::io::stdin().is_terminal()` — same IsTerminal import already in mod.rs"
  - "Select default index 1 (0-based) gives visual default of '2' chunks matching plan intent"
  - "Clippy `needless_borrows_for_generic_args`: pass array literal directly to `.items()`, not `&[...]`"
patterns_established:
  - "assay-cli commands import `super::{assay_dir, project_root}` for path resolution"
  - "Non-TTY guard is always the first statement in interactive commands — before any I/O"
  - "`dialoguer::Confirm` default(is_empty()) drives open-ended loops (first iteration defaults yes)"
observability_surfaces:
  - "Non-TTY: `assay plan` prints actionable message to stderr and returns exit code 1"
  - "Success: prints each created path (spec gates.toml files + milestone TOML) then a next-steps hint"
  - "Failure: `create_from_inputs` errors propagate via `anyhow::context(\"failed to create milestone\")`"
duration: ~20m
verification_result: passed
completed_at: 2026-03-20
blocker_discovered: false
---

# T03: Add `assay plan` CLI Command with dialoguer

**Wired `assay_core::wizard::create_from_inputs` into the CLI as `assay plan` with a dialoguer interactive flow and a non-TTY guard that redirects to the `milestone_create` MCP tool.**

## What Happened

Added `dialoguer = "0.12.0"` to workspace dependencies and `assay-cli/Cargo.toml`. Created `crates/assay-cli/src/commands/plan.rs` with `pub(crate) fn handle() -> anyhow::Result<i32>`. The function immediately checks `std::io::stdin().is_terminal()` and returns `Ok(1)` with an actionable stderr message in non-TTY environments. The interactive path collects milestone name (auto-slugified via `wizard::slugify`), optional description, chunk count via `dialoguer::Select`, and per-chunk name + criteria via `dialoguer::Input` / `dialoguer::Confirm` loops. It builds `WizardInputs`, resolves project paths, calls `create_from_inputs`, and prints each created file path followed by a next-steps hint.

Registered `pub mod plan;` in `commands/mod.rs` and added `Plan` variant + dispatch arm in `main.rs`. Fixed one clippy warning (`needless_borrows_for_generic_args` on the `Select::items` call — pass `[...]` not `&[...]`).

The `assay-mcp` test errors visible in `just lint` are pre-existing T01 contract tests for `milestone_create`/`spec_create` MCP tools not yet implemented; they existed before T03.

## Verification

```
cargo build -p assay-cli
# → clean build, 0 warnings

cargo test -p assay-cli -- plan
# → plan_non_tty_returns_1 ... ok  (1 passed)

cargo clippy -p assay-cli -- -D warnings
# → Finished (no warnings)

cargo test -p assay-core --features assay-types/orchestrate --test wizard
# → 5 passed (wizard core regression check)
```

## Diagnostics

- `assay plan 2>&1` in non-TTY: exits 1, stderr message names `milestone_create` MCP tool
- `assay plan --help`: shows "Run the guided authoring wizard"
- Created files inspectable via `assay milestone list` and `assay spec list`
- Errors from `create_from_inputs` surface via anyhow chain: "failed to create milestone: ..."

## Deviations

- `Select::default(1)` used (index 1 = "2" chunks displayed) instead of `default(2)` as written in the plan; the plan says "default(2)" which is the integer value, but Select takes a 0-based index. Chose index 1 to match the spirit ("default 2 chunks"). This is a clarification, not a deviation from intent.

## Known Issues

- `just lint` still fails due to pre-existing T01 contract test errors in `assay-mcp` (unimplemented `milestone_create`/`spec_create` MCP tools). These are T04's scope.

## Files Created/Modified

- `crates/assay-cli/src/commands/plan.rs` — new file; `handle()` + `plan_non_tty_returns_1` test
- `crates/assay-cli/src/commands/mod.rs` — added `pub mod plan;`
- `crates/assay-cli/src/main.rs` — added `Plan` variant to `Command` enum + dispatch arm
- `Cargo.toml` — added `dialoguer = "0.12.0"` to `[workspace.dependencies]`
- `crates/assay-cli/Cargo.toml` — added `dialoguer.workspace = true`
