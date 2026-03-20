---
estimated_steps: 5
estimated_files: 5
---

# T03: Add `assay plan` CLI Command with dialoguer

**Slice:** S03 — Guided Authoring Wizard
**Milestone:** M005

## Description

Wire the wizard core into the CLI as `assay plan`. The command checks for a TTY before entering the interactive loop and exits with a helpful message pointing to `milestone_create` MCP tool when not in an interactive terminal. The interactive path uses `dialoguer` to collect milestone name, chunks, and per-chunk criteria, then delegates to `create_from_inputs()` from T02.

## Steps

1. Add `dialoguer = "0.12.0"` to `[workspace.dependencies]` in root `Cargo.toml`. Add `dialoguer.workspace = true` to `[dependencies]` in `crates/assay-cli/Cargo.toml`.

2. Create `crates/assay-cli/src/commands/plan.rs`. Write `pub(crate) fn handle() -> anyhow::Result<i32>`:
   - First check: `if !std::io::stdin().is_terminal() { eprintln!("assay plan requires an interactive terminal.\nFor non-interactive authoring, use the milestone_create MCP tool."); return Ok(1); }` — note `std::io::IsTerminal` is already imported in `commands/mod.rs`; import it here too.
   - Collect milestone name via `dialoguer::Input::<String>::new().with_prompt("Milestone name").interact_text()?`
   - Collect optional description (use `dialoguer::Input` with allow_empty, or `dialoguer::Confirm` + conditional Input).
   - Collect chunk count via `dialoguer::Select::new().with_prompt("Number of chunks (1-7)").items(&["1","2","3","4","5","6","7"]).default(2).interact()?` — result is 0-based index, add 1 for actual count.
   - For each chunk (loop 1..=chunk_count): collect chunk name, then loop collecting criteria (use `dialoguer::Confirm` "Add a criterion?" to drive the loop; per criterion collect name, description, optional cmd).
   - Build `WizardInputs` from collected data.
   - Resolve `root = project_root()?`, `assay_dir = assay_dir(&root)`, `specs_dir = assay_dir.join("specs")` (use `config::load` to get specs_dir from config).
   - Call `assay_core::wizard::create_from_inputs(&inputs, &assay_dir, &specs_dir)` — map error with `context("failed to create milestone")?`.
   - Print output: `println!("  Created milestone '{slug}'")` then `println!("    created {}", path.display())` for each file in `result.spec_paths`, then `println!("    created {}", result.milestone_path.display())`. Print hint: `println!("\n  Milestone created as Draft. Use 'assay milestone list' to view, or run 'assay gate run <chunk>' to test a chunk.")`.
   - Add `#[cfg(test)]` block with one test: `plan_non_tty_returns_1` — since test environments are non-TTY, call `handle()` and assert it returns `Ok(1)`.

3. Add `pub mod plan;` to `crates/assay-cli/src/commands/mod.rs`.

4. In `crates/assay-cli/src/main.rs`, add `Plan` variant to the `Command` enum:
   ```rust
   /// Run the guided authoring wizard to create a milestone and chunk specs.
   #[command(name = "plan", about = "Run the guided authoring wizard")]
   Plan,
   ```
   Add dispatch arm to the `match` expression: `Some(Command::Plan) => commands::plan::handle()`.

5. Build and test: `cargo build -p assay-cli`, `cargo test -p assay-cli -- plan`. Run `just lint` to confirm no clippy warnings.

## Must-Haves

- [ ] `dialoguer = "0.12.0"` in workspace `Cargo.toml`; `dialoguer.workspace = true` in `assay-cli/Cargo.toml`
- [ ] TTY check is the **first** thing `handle()` does — before any dialoguer call
- [ ] Non-TTY path prints a message mentioning `milestone_create` MCP tool and returns `Ok(1)`
- [ ] Interactive path builds a `WizardInputs` and calls `assay_core::wizard::create_from_inputs()`
- [ ] Output lists each created file path (milestone TOML + all gates.toml files)
- [ ] `Plan` variant wired into `main.rs` dispatch
- [ ] `plan_non_tty_returns_1` test in `plan.rs` passes
- [ ] `cargo build -p assay-cli` succeeds with no warnings

## Verification

```
cargo build -p assay-cli
# Expected: clean build

cargo test -p assay-cli -- plan
# Expected: plan_non_tty_returns_1 passes

just lint
# Expected: no new clippy warnings
```

## Observability Impact

- Signals added/changed: `assay plan` prints each created file path on success; prints a hint about next steps; prints diagnostic message on non-TTY before exiting
- How a future agent inspects this: `assay plan 2>&1` in non-TTY returns exit code 1 with actionable message; created files inspectable via `assay milestone list` and `assay spec list`
- Failure state exposed: non-TTY gives actionable redirect; `create_from_inputs` errors propagate via `anyhow::context` with "failed to create milestone" prefix

## Inputs

- `crates/assay-core/src/wizard.rs` — `create_from_inputs`, `WizardInputs`, `ChunkInput`, `CriterionInput`, `WizardResult` (produced by T02)
- `crates/assay-cli/src/commands/mod.rs` — `project_root()`, `assay_dir()` helpers
- `crates/assay-cli/src/commands/milestone.rs` — reference for `handle() -> anyhow::Result<i32>` pattern
- `crates/assay-cli/src/main.rs` — `Command` enum and dispatch structure

## Expected Output

- `Cargo.toml` (workspace root) — `dialoguer = "0.12.0"` added to `[workspace.dependencies]`
- `crates/assay-cli/Cargo.toml` — `dialoguer.workspace = true` added
- `crates/assay-cli/src/commands/plan.rs` — new file with `handle()` + `plan_non_tty_returns_1` test
- `crates/assay-cli/src/commands/mod.rs` — `pub mod plan;` added
- `crates/assay-cli/src/main.rs` — `Plan` variant + dispatch arm added
