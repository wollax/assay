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
use crate::serve::types::elapsed_secs_since;

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
        s.jobs
            .iter()
            .map(|j| {
                let elapsed = j
                    .started_at
                    .map(|t| format!("{}s", elapsed_secs_since(t) as u64))
                    .unwrap_or_else(|| "-".to_string());
                let manifest_name = j
                    .manifest_path
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
            })
            .collect::<Vec<_>>()
    };

    let rows: Vec<Row> = jobs
        .iter()
        .map(|(id, name, status, attempt, elapsed)| {
            Row::new(vec![
                Cell::from(id.as_str()),
                Cell::from(name.as_str()),
                Cell::from(status.as_str()),
                Cell::from(attempt.as_str()),
                Cell::from(elapsed.as_str()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(10), // id
        Constraint::Fill(1),    // name (variable)
        Constraint::Length(12), // status
        Constraint::Length(8),  // attempt
        Constraint::Length(8),  // elapsed
    ];

    let table = Table::new(rows, widths)
        .header(Row::new(vec!["Job ID", "Manifest", "Status", "Attempt", "Elapsed"]))
        .block(Block::default().title("smelt serve — jobs").borders(Borders::ALL));

    frame.render_widget(table, frame.area());
}
