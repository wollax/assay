---
estimated_steps: 5
estimated_files: 4
---

# T01: Add library target and integration test contract

**Slice:** S02 — In-TUI Authoring Wizard
**Milestone:** M006

## Description

`assay-tui` is currently a pure binary crate with only `src/main.rs`. Integration tests under `tests/` link against the library target — without a `src/lib.rs`, the wizard types are invisible to the test binary. This task restructures the crate to binary + library, adds `tempfile` as a dev-dependency, stubs out the `WizardState`/`WizardAction` types just enough to compile, and writes the full integration test in red state (compiles, runs, fails). The failing test is the contract for T02.

The integration test follows the exact pattern from `crates/assay-core/tests/wizard.rs`: `TempDir::new()`, drive `handle_wizard_event` with synthetic `KeyEvent`s, wait for `Submit(WizardInputs)`, call `create_from_inputs`, assert milestone TOML + chunk `gates.toml` files exist.

## Steps

1. Edit `crates/assay-tui/Cargo.toml`:
   - Add `[lib]` section: `name = "assay_tui"`, `path = "src/lib.rs"`
   - Add `assay-core.workspace = true` to dependencies if not already present (needed by test)
   - Add `[dev-dependencies]` section with `tempfile.workspace = true`

2. Create `crates/assay-tui/src/lib.rs` with public module declarations:
   ```rust
   pub mod wizard;
   ```
   (wizard_draw is added by T03 when the file is created — do NOT add it here, as an undeclared module file would cause a compile error)

3. Create `crates/assay-tui/src/wizard.rs` with stub types sufficient to compile:
   - `pub struct WizardState { pub step: usize, pub fields: Vec<Vec<String>>, pub cursor: usize, pub chunk_count: usize, pub error: Option<String> }`
   - `pub enum StepKind { Name, Description, ChunkCount, ChunkName(usize), Criteria(usize) }`
   - `pub enum WizardAction { Continue, Submit(assay_core::wizard::WizardInputs), Cancel }`
   - `impl WizardState { pub fn new() -> Self { todo!("T02") } }`
   - `pub fn handle_wizard_event(_state: &mut WizardState, _event: crossterm::event::KeyEvent) -> WizardAction { todo!("T02") }`
   - Add `pub fn current_step_kind(&self) -> StepKind { todo!("T02") }` on WizardState

4. Write `crates/assay-tui/tests/wizard_round_trip.rs`:
   - Import `assay_tui::wizard::{WizardState, WizardAction, handle_wizard_event}`
   - Import `assay_core::wizard::{WizardInputs, create_from_inputs}`
   - Import `crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers}`
   - Import `tempfile::TempDir`
   - Helper `fn key(code: KeyCode) -> KeyEvent` — constructs a `KeyEvent { code, kind: KeyEventKind::Press, modifiers: KeyModifiers::NONE, state: crossterm::event::KeyEventState::NONE }`
   - Helper `fn type_str(state: &mut WizardState, s: &str) -> ()` — calls `handle_wizard_event(state, key(KeyCode::Char(c)))` for each char
   - Main test `wizard_round_trip`:
     ```
     let tmp = TempDir::new().unwrap();
     let assay_dir = tmp.path().join(".assay");
     let specs_dir = assay_dir.join("specs");
     let mut state = WizardState::new();
     // step 0: milestone name
     type_str(&mut state, "Auth Layer");
     handle_wizard_event(&mut state, key(KeyCode::Enter));
     // step 1: description (blank — skip)
     handle_wizard_event(&mut state, key(KeyCode::Enter));
     // step 2: chunk count = 2
     handle_wizard_event(&mut state, key(KeyCode::Char('2')));
     handle_wizard_event(&mut state, key(KeyCode::Enter));
     // step 3: chunk name 1
     type_str(&mut state, "Login");
     handle_wizard_event(&mut state, key(KeyCode::Enter));
     // step 4: chunk name 2
     type_str(&mut state, "Register");
     handle_wizard_event(&mut state, key(KeyCode::Enter));
     // step 5: criteria for chunk 1
     type_str(&mut state, "User can log in with valid credentials");
     handle_wizard_event(&mut state, key(KeyCode::Enter));
     handle_wizard_event(&mut state, key(KeyCode::Enter)); // blank = done
     // step 6: criteria for chunk 2 → Submit
     type_str(&mut state, "User can create an account");
     handle_wizard_event(&mut state, key(KeyCode::Enter));
     let action = handle_wizard_event(&mut state, key(KeyCode::Enter)); // blank = Submit
     let WizardAction::Submit(inputs) = action else {
         panic!("expected Submit, got Continue or Cancel");
     };
     let result = create_from_inputs(&inputs, &assay_dir, &specs_dir);
     assert!(result.is_ok(), "create_from_inputs failed: {:?}", result.err());
     assert!(assay_dir.join("milestones/auth-layer.toml").exists(), "milestone TOML missing");
     assert!(specs_dir.join("login/gates.toml").exists(), "login gates.toml missing");
     assert!(specs_dir.join("register/gates.toml").exists(), "register gates.toml missing");
     ```

5. Run `cargo build -p assay-tui` → verify it exits 0 (stubs compile); run `cargo test -p assay-tui wizard_round_trip 2>&1 | tail -10` → should show test panics with "T02" message (not a compile error)

## Must-Haves

- [ ] `crates/assay-tui/Cargo.toml` has `[lib]` section and `tempfile.workspace = true` in `[dev-dependencies]`
- [ ] `src/lib.rs` exists with `pub mod wizard;`
- [ ] `src/wizard.rs` compiles — all public types (`WizardState`, `StepKind`, `WizardAction`, `handle_wizard_event`) are present
- [ ] `tests/wizard_round_trip.rs` compiles — `cargo build -p assay-tui` exits 0
- [ ] Integration test runs and fails (red state): `cargo test -p assay-tui wizard_round_trip` exits non-zero with a panic message, not a compile error

## Verification

- `cargo build -p assay-tui` exits 0 — binary still builds
- `cargo test -p assay-tui wizard_round_trip 2>&1 | grep -E "FAILED|panicked"` — shows failure (red state confirmed)
- `cargo test -p assay-tui wizard_round_trip 2>&1 | grep "error\[E"` — empty (no compile errors)

## Observability Impact

- Signals added/changed: None — stub types only; no runtime behavior yet
- How a future agent inspects this: `cargo test -p assay-tui wizard_round_trip -- --nocapture` reveals the test structure and failure point
- Failure state exposed: `todo!("T02")` panic message in test output clearly identifies what's missing

## Inputs

- `crates/assay-tui/Cargo.toml` — current binary-only Cargo.toml to restructure
- `crates/assay-core/src/wizard.rs` — `WizardInputs`, `WizardChunkInput`, `create_from_inputs` signatures (already implemented)
- `Cargo.toml` (workspace root) — confirms `tempfile = "3"` is in `[workspace.dependencies]`

## Expected Output

- `crates/assay-tui/Cargo.toml` — updated with `[lib]` section and `tempfile` dev-dep
- `crates/assay-tui/src/lib.rs` — new; module declarations
- `crates/assay-tui/src/wizard.rs` — new; stub types that compile
- `crates/assay-tui/tests/wizard_round_trip.rs` — new; full integration test in red state
