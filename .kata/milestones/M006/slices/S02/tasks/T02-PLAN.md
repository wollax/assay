---
estimated_steps: 6
estimated_files: 1
---

# T02: Implement WizardState state machine and make integration test green

**Slice:** S02 — In-TUI Authoring Wizard
**Milestone:** M006

## Description

Implement the full pure state machine in `src/wizard.rs` — replacing all `todo!()` stubs with correct logic. The state machine handles every key event for every step kind, manages dynamic field allocation when chunk_count is confirmed, and assembles `WizardInputs` on Submit. No terminal, no rendering — `handle_wizard_event` is a pure function that can be driven by the integration test with synthetic `KeyEvent`s.

The key complexity is the dynamic step count: steps = 3 (name, description, chunk-count) + N (chunk names) + N (criteria), where N is only known after step 2. Fields are allocated in `Vec<Vec<String>>` at that point. A `current_step_kind()` helper maps raw step indices to semantic `StepKind` variants, so all other code branches on `StepKind` rather than raw numbers.

Primary acceptance gate: `cargo test -p assay-tui` passes — integration test + all unit tests green.

## Steps

1. Implement `WizardState::new()`:
   - `step: 0`, `chunk_count: 0`, `cursor: 0`, `error: None`
   - `fields: vec![vec![String::new()], vec![String::new()], vec![String::new()]]` — three initial slots: name (step 0), description (step 1), chunk-count input (step 2)

2. Implement `WizardState::current_step_kind(&self) -> StepKind`:
   - step 0 → `StepKind::Name`
   - step 1 → `StepKind::Description`
   - step 2 → `StepKind::ChunkCount`
   - `3..3+N` → `StepKind::ChunkName(step - 3)` where N = `self.chunk_count`
   - `3+N..3+2N` → `StepKind::Criteria(step - 3 - N)`
   - guard: step ≥ 3 + 2*N (past end) — treat as `StepKind::Name` as defensive fallback (should not occur in normal flow)

3. Implement `handle_wizard_event` — full logic:
   - First guard: `if key.kind != KeyEventKind::Press { return WizardAction::Continue; }`
   - Clear `state.error` on any Press event
   - Match `key.code`:
     - `KeyCode::Esc` → `WizardAction::Cancel`
     - `KeyCode::Char(c)` for `StepKind::ChunkCount`: only accept digits '1'–'7'; set `fields[2] = vec![c.to_string()]` (replace); else ignore
     - `KeyCode::Char(c)` for all other step kinds: append `c` to active buffer = `fields[step].last_mut()`, increment `cursor`
     - `KeyCode::Backspace` for single-line steps (Name, Description, ChunkCount, ChunkName): if active buffer non-empty — pop last char, decrement `cursor`; if empty and `step > 0` — decrement `step`, set `cursor` to length of new active buffer
     - `KeyCode::Backspace` for `Criteria(n)`: if last entry in `fields[step]` non-empty — pop last char; else if `fields[step].len() > 1` — remove last entry (the empty line), set cursor to len of new last entry; else (only empty entry left) — decrement `step`, set cursor accordingly
     - `KeyCode::Enter` for `Name`, `Description`, `ChunkName`: advance step; for Name validate non-empty (set `state.error` and stay if empty); push `String::new()` to `fields[step+1]` if needed to ensure active buffer exists; increment `state.step`; reset `cursor` to length of new active buffer (0 for fresh step)
     - `KeyCode::Enter` for `ChunkCount`: validate buffer is digit 1–7 (set error if not); set `state.chunk_count = n`; allocate N ChunkName field vecs + N Criteria field vecs: `for _ in 0..n { state.fields.push(vec![String::new()]); } for _ in 0..n { state.fields.push(vec![String::new()]); }`; advance step
     - `KeyCode::Enter` for `Criteria(n)`: if active buffer non-empty — push a new `String::new()` to `fields[step]` (start next criterion line), reset cursor to 0; if active buffer empty — this criteria step is done; if `n + 1 < chunk_count` — advance `step`; else — assemble and return `Submit(inputs)` (see below)
   - Submit assembly: `let slug = slugify(&fields[0][0]); let name = fields[0][0].clone(); let description = if fields[1][0].is_empty() { None } else { Some(fields[1][0].clone()) }; let chunks = (0..chunk_count).map(|i| WizardChunkInput { slug: slugify(&fields[3+i][0]), name: fields[3+i][0].clone(), criteria: fields[3+chunk_count+i].iter().filter(|s| !s.is_empty()).cloned().collect() }).collect(); WizardAction::Submit(WizardInputs { slug, name, description, chunks })`

4. Add `#[cfg(test)]` unit tests in `src/wizard.rs`:
   - `wizard_step_kind_n1`: chunk_count=1 → step_count=5; verify `current_step_kind` at each index (0..4)
   - `wizard_step_kind_n2`: chunk_count=2 → step_count=7; verify at indices 0..6
   - `wizard_step_kind_n3`: chunk_count=3 → step_count=9; spot-check boundary indices
   - `wizard_backspace_on_empty_goes_back`: manually set step=1, empty field; press Backspace → step=0
   - `wizard_criteria_blank_enter_advances`: drive to a Criteria step; add one criterion; blank Enter → advances (or submits if last)
   - `wizard_submit_assembles_inputs`: drive a complete N=1 scenario via synthetic events; assert `WizardAction::Submit(inputs)` where `inputs.slug == "my-chunk"`, `inputs.chunks[0].slug == "chunk-a"`, `inputs.chunks[0].criteria.len() == 1`

5. Run `cargo test -p assay-tui` → all tests pass; confirm integration test output shows file paths created

6. Run `cargo clippy -p assay-tui -- -D warnings` → no warnings; run `cargo fmt --check -p assay-tui` → clean

## Must-Haves

- [ ] `WizardState::new()` initializes with 3 field slots (steps 0–2) and zeroed counters
- [ ] `current_step_kind()` maps step indices correctly for any N in 1..7
- [ ] `handle_wizard_event` guards `KeyEventKind::Press` first
- [ ] `handle_wizard_event` clears `state.error` on every press
- [ ] ChunkCount step: only accepts digits 1–7; non-digit input is silently ignored
- [ ] Name step: validates non-empty on Enter — sets `state.error` and stays if empty
- [ ] Dynamic field allocation: after ChunkCount confirmed, exactly N ChunkName + N Criteria vecs pushed
- [ ] Backspace on empty single-line step decrements step (not below 0)
- [ ] Criteria blank Enter advances to next criteria step or returns Submit on last chunk's criteria
- [ ] Submit assembles correct `WizardInputs` — slug derived via `slugify`, criteria filtered to non-empty strings
- [ ] `cargo test -p assay-tui wizard_round_trip` passes (green)
- [ ] `cargo test -p assay-tui` passes (all unit + integration tests green)

## Verification

- `cargo test -p assay-tui` exits 0
- `cargo test -p assay-tui wizard_round_trip -- --nocapture` shows milestone file path in tempdir
- `cargo test -p assay-tui wizard_step_kind` passes
- `cargo test -p assay-tui wizard_backspace` passes
- `cargo clippy -p assay-tui -- -D warnings` exits 0
- `cargo fmt --check -p assay-tui` exits 0

## Observability Impact

- Signals added/changed: `state.error: Option<String>` — inline error visible on any failed validation; cleared on next press
- How a future agent inspects this: `cargo test -p assay-tui -- --nocapture` shows per-test trace; unit tests directly exercise each step routing path
- Failure state exposed: validation errors (empty name, invalid chunk count) are preserved in `state.error`; wrong step routing produces wrong `StepKind` which unit tests immediately catch

## Inputs

- `crates/assay-tui/src/wizard.rs` — stub types from T01
- `crates/assay-tui/tests/wizard_round_trip.rs` — integration test from T01 (the contract)
- `crates/assay-core/src/wizard.rs` — `WizardInputs`, `WizardChunkInput`, `slugify` signatures
- `~/.cargo/registry/.../crossterm-0.28.1/src/event.rs` — `KeyCode`, `KeyEventKind`, `KeyEvent` field reference

## Expected Output

- `crates/assay-tui/src/wizard.rs` — fully implemented state machine; `todo!()` stubs replaced; unit tests added
- `cargo test -p assay-tui` passes — all tests green including the `wizard_round_trip` integration test
