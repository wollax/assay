---
estimated_steps: 5
estimated_files: 1
---

# T04: Add `assay milestone status` and `assay milestone advance` CLI subcommands

**Slice:** S02 — Development Cycle State Machine
**Milestone:** M005

## Description

Extends `crates/assay-cli/src/commands/milestone.rs` with two new subcommands: `assay milestone status` prints a human-readable progress table for all InProgress milestones, and `assay milestone advance` calls `cycle_advance` and reports success or error. Completes the CLI surface for R043 (state machine) and R044 (cycle operations). Two tests cover the no-milestones path for each command.

## Steps

1. Add `Status` and `Advance` variants to `MilestoneCommand`:
   ```rust
   /// Show progress for active (in_progress) milestones
   Status,
   /// Advance the development cycle: evaluate gates for the active chunk and mark it complete
   Advance {
       /// Slug of the milestone to advance. Defaults to the first in_progress milestone.
       #[arg(long)]
       milestone: Option<String>,
   },
   ```

2. Add dispatch arms in `handle()`:
   ```rust
   MilestoneCommand::Status => milestone_status_cmd(),
   MilestoneCommand::Advance { milestone } => milestone_advance_cmd(milestone),
   ```

3. Implement `fn milestone_status_cmd() -> anyhow::Result<i32>`:
   - `let root = project_root()?; let dir = assay_dir(&root);`
   - `let milestones = assay_core::milestone::milestone_scan(&dir)?;`
   - Filter to `InProgress`: `let active: Vec<_> = milestones.iter().filter(|m| m.status == MilestoneStatus::InProgress).collect();`
   - If empty: `println!("No active milestones."); return Ok(0);`
   - For each milestone, print: `println!("MILESTONE: {} ({})", m.slug, format!("{:?}", m.status).to_lowercase());`
   - For each chunk in milestone (sorted by order), determine if complete (slug in `completed_chunks`) or active (not in `completed_chunks`); print `  [x] <slug>  (complete)` or `  [ ] <slug>  (active)` respectively
   - Return `Ok(0)`

4. Implement `fn milestone_advance_cmd(milestone_slug: Option<String>) -> anyhow::Result<i32>`:
   - `let root = project_root()?;`
   - `let assay_dir = assay_dir(&root);`
   - `let specs_dir = root.join(".assay").join("specs");`
   - `let working_dir = root.clone();`
   - Call `assay_core::milestone::cycle_advance(&assay_dir, &specs_dir, &working_dir, milestone_slug.as_deref(), None)`
   - On `Ok(status)`: print `"Advanced: {} ({}/{} chunks complete, phase: {:?})", status.milestone_slug, status.completed_count, status.total_count, status.phase`; `return Ok(0);`
   - On `Err(e)`: `eprintln!("Error: {e}"); return Ok(1);` (do not propagate — exit code 1 is the CLI contract for cycle errors)

5. Add two tests:
   ```rust
   #[test]
   fn milestone_status_no_milestones() {
       let dir = tempfile::tempdir().unwrap();
       std::fs::create_dir_all(dir.path().join(".assay")).unwrap();
       std::env::set_current_dir(dir.path()).unwrap();
       let result = handle(MilestoneCommand::Status);
       assert!(result.is_ok());
       assert_eq!(result.unwrap(), 0);
   }

   #[test]
   fn milestone_advance_no_active_milestone() {
       let dir = tempfile::tempdir().unwrap();
       std::fs::create_dir_all(dir.path().join(".assay")).unwrap();
       std::env::set_current_dir(dir.path()).unwrap();
       let result = handle(MilestoneCommand::Advance { milestone: None });
       assert!(result.is_ok());
       assert_eq!(result.unwrap(), 1, "advance with no milestones should exit 1");
   }
   ```

## Must-Haves

- [ ] `MilestoneCommand::Status` and `MilestoneCommand::Advance { milestone: Option<String> }` variants added
- [ ] `milestone_status_cmd` prints "No active milestones." when none exist; prints progress table with `[x]`/`[ ]` for each chunk when InProgress milestones exist
- [ ] `milestone_advance_cmd` exits with code 1 and `eprintln!` on error (not panic/anyhow propagation)
- [ ] `milestone_advance_cmd` exits with code 0 and progress summary on success
- [ ] 3 CLI milestone tests pass (1 existing from S01 + 2 new)
- [ ] `cargo test --workspace` green; `just ready` green

## Verification

```bash
# CLI milestone tests (3 total: existing List + new Status + new Advance)
cargo test -p assay-cli -- milestone

# Full workspace
cargo test --workspace

# Final quality check
just ready
```

## Observability Impact

- Signals added/changed: `milestone_status_cmd` prints current progress table to stdout — human-readable cycle state inspection; `milestone_advance_cmd` prints structured success message with slug, chunk counts, and phase; errors go to stderr with "Error: " prefix
- How a future agent inspects this: `assay milestone status` gives the current cycle state in human-readable form; exit code 1 from `assay milestone advance` signals gate failure or precondition error
- Failure state exposed: `eprintln!("Error: {e}")` exposes the full `AssayError::Io` message including operation label, path, and underlying cause when cycle_advance fails

## Inputs

- `crates/assay-cli/src/commands/milestone.rs` — existing `MilestoneCommand`, `handle()`, `assay_dir()`, `project_root()` helpers (established in S01)
- `crates/assay-core/src/milestone/cycle.rs` — `cycle_advance`, `CycleStatus` (created in T02)
- `crates/assay-types/src/milestone.rs` — `MilestoneStatus::InProgress` for filter comparison

## Expected Output

- `crates/assay-cli/src/commands/milestone.rs` — `Status` and `Advance` variants added to `MilestoneCommand`; `milestone_status_cmd` and `milestone_advance_cmd` implemented; 2 new tests
