---
estimated_steps: 6
estimated_files: 5
---

# T03: Ratatui TUI background thread

**Slice:** S03 — Ratatui TUI + Server Config + Graceful Shutdown
**Milestone:** M006

## Description

Implement the Ratatui TUI as a `std::thread::spawn` background thread that reads `Arc<Mutex<ServerState>>` and renders a live job table every 250ms. Uses `ratatui::init()` / `ratatui::restore()` for safe terminal lifecycle (panic hook included). A shared `Arc<AtomicBool>` coordinates shutdown between the TUI thread and the tokio runtime. Unit-testable via ratatui's `TestBackend`. Wired into the serve entrypoint in T04.

## Steps

1. Add `ratatui` and `crossterm` to workspace `Cargo.toml` under `[workspace.dependencies]`:
   ```toml
   ratatui = "0.29"
   crossterm = "0.28"
   ```
   Add both to `crates/smelt-cli/Cargo.toml` `[dependencies]`:
   ```toml
   ratatui.workspace = true
   crossterm.workspace = true
   ```

2. Create `crates/smelt-cli/src/serve/tui.rs`:

   ```rust
   use std::io;
   use std::sync::atomic::{AtomicBool, Ordering};
   use std::sync::{Arc, Mutex};
   use std::time::Duration;

   use crossterm::event::{self, Event, KeyCode, KeyModifiers};
   use ratatui::layout::Constraint;
   use ratatui::widgets::{Block, Borders, Cell, Row, Table};
   use ratatui::DefaultTerminal;
   use ratatui::Frame;

   use crate::serve::queue::ServerState;

   /// Spawn the TUI background thread. Returns a JoinHandle.
   /// `shutdown` is an AtomicBool: when set to `true`, the TUI loop exits.
   /// The TUI loop also sets `shutdown = true` when the user presses `q` or Ctrl+C.
   pub(crate) fn run_tui(
       state: Arc<Mutex<ServerState>>,
       shutdown: Arc<AtomicBool>,
   ) -> std::thread::JoinHandle<()> {
       std::thread::spawn(move || {
           let mut terminal = ratatui::init();
           let result = tui_loop(&mut terminal, state, Arc::clone(&shutdown));
           ratatui::restore();
           if let Err(e) = result {
               eprintln!("TUI error: {e}");
           }
           // Signal shutdown in case TUI was the one that exited first
           shutdown.store(true, Ordering::SeqCst);
       })
   }

   fn tui_loop(
       terminal: &mut DefaultTerminal,
       state: Arc<Mutex<ServerState>>,
       shutdown: Arc<AtomicBool>,
   ) -> io::Result<()> {
       loop {
           if shutdown.load(Ordering::SeqCst) {
               break;
           }
           terminal.draw(|frame| render(frame, &state))?;
           // Poll for key events without blocking (non-blocking check)
           if event::poll(Duration::from_millis(250))? {
               if let Event::Key(key) = event::read()? {
                   match key.code {
                       KeyCode::Char('q') => {
                           shutdown.store(true, Ordering::SeqCst);
                           break;
                       }
                       KeyCode::Char('c')
                           if key.modifiers.contains(KeyModifiers::CONTROL) =>
                       {
                           shutdown.store(true, Ordering::SeqCst);
                           break;
                       }
                       _ => {}
                   }
               }
           }
       }
       Ok(())
   }

   pub(crate) fn render(frame: &mut Frame, state: &Arc<Mutex<ServerState>>) {
       // Lock briefly — clone only what we need, then release immediately
       let jobs = {
           let s = state.lock().unwrap();
           s.jobs.iter().map(|j| {
               let elapsed = j.started_at
                   .map(|t| format!("{}s", t.elapsed().as_secs()))
                   .unwrap_or_else(|| "-".to_string());
               let manifest_name = j.manifest_path
                   .file_stem()
                   .and_then(|s| s.to_str())
                   .unwrap_or("?")
                   .to_string();
               (
                   j.id.to_string(),
                   manifest_name,
                   format!("{:?}", j.status),
                   j.attempt.to_string(),
                   elapsed,
               )
           }).collect::<Vec<_>>()
       };

       let rows: Vec<Row> = jobs.iter().map(|(id, name, status, attempt, elapsed)| {
           Row::new(vec![
               Cell::from(id.as_str()),
               Cell::from(name.as_str()),
               Cell::from(status.as_str()),
               Cell::from(attempt.as_str()),
               Cell::from(elapsed.as_str()),
           ])
       }).collect();

       let widths = [
           Constraint::Length(10),   // id
           Constraint::Fill(1),      // name (variable)
           Constraint::Length(12),   // status
           Constraint::Length(8),    // attempt
           Constraint::Length(8),    // elapsed
       ];

       let table = Table::new(rows, widths)
           .header(Row::new(vec!["Job ID", "Manifest", "Status", "Attempt", "Elapsed"]))
           .block(Block::default().title("smelt serve — jobs").borders(Borders::ALL));

       frame.render_widget(table, frame.area());
   }
   ```

3. Register in `crates/smelt-cli/src/serve/mod.rs`:
   - Add `pub(crate) mod tui;`
   - Add `pub(crate) use tui::{run_tui, render};`

4. Add unit test `test_tui_render_no_panic` to `serve/tests.rs`:
   ```rust
   #[test]
   fn test_tui_render_no_panic() {
       use ratatui::backend::TestBackend;
       use ratatui::Terminal;
       use crate::serve::tui::render;
       use crate::serve::queue::ServerState;
       use std::sync::{Arc, Mutex};

       let backend = TestBackend::new(80, 24);
       let mut terminal = Terminal::new(backend).unwrap();
       let state = Arc::new(Mutex::new(ServerState::new(2)));

       // Render with empty state — must not panic
       terminal.draw(|frame| render(frame, &state)).unwrap();

       // Add a mock job entry to state and render again
       // (directly mutate queue for test — no manifest file needed)
       {
           use std::path::PathBuf;
           use crate::serve::types::{JobSource, JobStatus, QueuedJob, JobId};
           use std::time::Instant;
           let mut s = state.lock().unwrap();
           s.jobs.push_back(QueuedJob {
               id: JobId::new("job-1"),
               manifest_path: PathBuf::from("test-manifest.toml"),
               source: JobSource::HttpApi,
               attempt: 0,
               status: JobStatus::Running,
               queued_at: Instant::now(),
               started_at: Some(Instant::now()),
           });
       }
       terminal.draw(|frame| render(frame, &state)).unwrap();
   }
   ```

5. Run `cargo test -p smelt-cli serve::tests::test_tui_render_no_panic` and fix any compile/runtime errors. Note: `TestBackend` renders without touching the real terminal — safe to run in CI.

6. Run `cargo build -p smelt-cli` — must succeed with no errors; ratatui/crossterm versions must be compatible with MSRV 1.85.

## Must-Haves

- [ ] `ratatui = "0.29"` and `crossterm = "0.28"` added to workspace and smelt-cli Cargo.toml
- [ ] `serve/tui.rs` compiles without errors
- [ ] `run_tui()` spawns a `std::thread::JoinHandle<()>` (not a tokio task)
- [ ] `ratatui::init()` called at thread start; `ratatui::restore()` called at thread exit (even on error)
- [ ] `render()` holds the Mutex lock only for the snapshot clone, then releases before rendering
- [ ] `test_tui_render_no_panic` passes: empty state + one-job state both render without panic
- [ ] `cargo build -p smelt-cli` clean with no errors

## Verification

- `cargo test -p smelt-cli serve::tests::test_tui_render_no_panic -- --nocapture` → 1 test passed
- `cargo build -p smelt-cli 2>&1 | grep "^error"` → no output (no errors)
- `grep "ratatui" Cargo.toml` → `ratatui = "0.29"` present in workspace deps

## Observability Impact

- Signals added/changed: TUI thread panic is caught by ratatui's panic hook (installed by `ratatui::init()`), ensuring terminal restoration even on crash; `eprintln!("TUI error: {e}")` appears on stderr after terminal restore
- How a future agent inspects this: if TUI is visually wrong, `test_tui_render_no_panic` catches render-time panics; `TestBackend::buffer()` can be inspected for exact cell content in future tests
- Failure state exposed: TUI errors surface to stderr after `ratatui::restore()` — terminal is always restored, never left in raw mode

## Inputs

- `crates/smelt-cli/src/serve/queue.rs` — `ServerState`, `QueuedJob` struct fields needed to build table rows
- `crates/smelt-cli/src/serve/types.rs` — `JobId`, `JobStatus`, `JobSource`, `QueuedJob` for the test
- `crates/smelt-cli/src/serve/mod.rs` — module registry to add `pub(crate) mod tui;`

## Expected Output

- `crates/smelt-cli/src/serve/tui.rs` — new file with `run_tui()`, `tui_loop()`, `render()`
- `crates/smelt-cli/src/serve/mod.rs` — `pub(crate) mod tui;` + `pub(crate) use tui::{run_tui, render};` added
- `Cargo.toml` — `ratatui = "0.29"` and `crossterm = "0.28"` added to `[workspace.dependencies]`
- `crates/smelt-cli/Cargo.toml` — `ratatui.workspace = true` and `crossterm.workspace = true` added to `[dependencies]`
- `crates/smelt-cli/src/serve/tests.rs` — `test_tui_render_no_panic` test added
