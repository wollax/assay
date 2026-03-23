//! Assay TUI — terminal user interface for assay project management.
//!
//! Thin binary entry point. All application logic lives in `assay_tui::app`
//! so it can be reached by integration tests.

use std::sync::mpsc;

use assay_tui::app::{App, TuiEvent};
use crossterm::event::{self, Event};
use ratatui::DefaultTerminal;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        ratatui::restore();
        original_hook(panic_info);
    }));

    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
    let mut app = App::new()?;

    // Channel for all TUI events (key, resize, agent output).
    let (tx, rx) = mpsc::channel::<TuiEvent>();

    // Give App a clone of the sender so the `r` key handler can spawn agent
    // threads that feed events back into the main loop.
    app.event_tx = Some(tx.clone());

    // Background thread: reads raw terminal events and forwards them as TuiEvents.
    // The thread exits automatically when the sender is dropped (app exits).
    std::thread::spawn(move || {
        loop {
            match event::read() {
                Ok(Event::Key(key)) => {
                    if tx.send(TuiEvent::Key(key)).is_err() {
                        break;
                    }
                }
                Ok(Event::Resize(cols, rows)) => {
                    if tx.send(TuiEvent::Resize(cols, rows)).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    // Main event loop: dispatch each TuiEvent to app or terminal.
    while let Ok(event) = rx.recv() {
        terminal.draw(|frame| app.draw(frame))?;
        match event {
            TuiEvent::Key(key) => {
                if app.handle_event(key) {
                    break;
                }
            }
            TuiEvent::Resize(..) => {
                terminal.clear()?;
            }
            TuiEvent::AgentLine(_) | TuiEvent::AgentDone { .. } => {
                app.handle_tui_event(event);
            }
        }
    }

    Ok(())
}
