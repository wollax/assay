# PR #160 S01 Review — Backlog Items

Source: parallel PR review, 2026-03-20  
PR: [kata/root/M006/S01] App Scaffold, Dashboard, and Binary Fix  
Status: Critical + Important items resolved. Suggestions below deferred.

## Code Cleanup

### `Text::raw(text)` wrapper is redundant (`lib.rs`)
`ListItem::new` accepts `impl Into<Text<'a>>`, and `String` already satisfies it directly.  
Fix: `ListItem::new(text)` — drop the `Text::raw` call and the `Text` import if no longer used elsewhere.  
Risk: none (pure cleanup).

### `.block(Block::default())` is a no-op (`lib.rs`)
`Block::default()` has no border, title, or padding — calling `.block(Block::default())` on a `Paragraph` is equivalent to not calling `.block()` at all.  
Fix: remove the call.  
Risk: none.

## Test Improvements

### `navigate_down_from_no_selection_goes_to_first` — already added in PR-review fix pass
*(Resolved — added alongside the other Critical/Important test gaps.)*

### `esc_returns_to_dashboard_from_milestone_detail` only covers one source screen
Test name implies all non-Dashboard screens, but only `MilestoneDetail` is tested. `ChunkDetail`, `Wizard`, and `Settings` are not exercised.  
Fix: parameterize or add separate test for each screen variant once S02–S04 are wired.  
Relevant once S02 lands (wizard Esc behaviour may differ).

### `make_app` helper: use `ListState::default().with_selected(selected)`
`ratatui-widgets 0.3.0` ships `ListState::with_selected` — eliminates the two-step `mut` binding.  
Fix: `list_state: ListState::default().with_selected(selected)` in `make_app`.  
Risk: none (test-only change).

### `fake_milestone` struct literal is fragile
Constructs `Milestone { ... }` with all fields named explicitly. When S03/S04 add new fields (e.g. `pr_branch` variants, future metadata), this helper breaks at compile time.  
Fix: derive `Default` on `Milestone` in `assay-types` (if appropriate) to allow `Milestone { slug: ..., name: ..., ..Default::default() }`.  
Note: `deny_unknown_fields` doesn't block `Default` derivation; check whether a `Default` impl makes semantic sense for `Milestone` before adding.

## Documentation

### `#[allow(dead_code)]` on `Screen` lacks explanation (`lib.rs`)
No comment explains why the attribute is there. Future maintainers must reason about intent.  
Fix: add inline comment: `// stub variants (ChunkDetail, Settings, Wizard) used in S02–S04; suppress until then`.

### `WizardState` doc comment embeds Kata planning jargon (`lib.rs`)
`/// Form state for the multi-step in-TUI milestone creation wizard.` is now correct (fixed in review pass), but the struct body `{}` is empty with no comment — future reader won't know which fields to add.  
Fix: when S02 lands and `WizardState` gains real fields, replace the empty body with proper field docs.

### `draw_dashboard` signature explanation (`lib.rs`)
Already updated in review pass with D095 rationale in the doc comment. No further action needed.

## Error Handling (minor)

### `current_dir()?` gives no Assay-specific context on failure (`main.rs`)
If run from a deleted or unmounted directory, the raw `io::Error` message is opaque.  
Fix:
```rust
let project_root = current_dir()
    .map_err(|e| color_eyre::eyre::eyre!(
        "Cannot determine working directory: {e}\n\
         Ensure you are running assay-tui from a valid directory."
    ))?;
```
Priority: low — this edge case is unlikely in practice.

### `event::read()?` in the run loop gives no Assay context on failure (`lib.rs`)
A broken terminal pipe or SIGTERM produces a raw crossterm `io::Error`.  
Fix: `.map_err(|e| color_eyre::eyre::eyre!("Terminal event read failed: {e}"))?`  
Priority: very low — error is unrecoverable regardless; message barely matters.
