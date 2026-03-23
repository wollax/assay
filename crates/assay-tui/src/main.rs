//! Assay TUI — terminal user interface for assay project management.
//!
//! Thin binary entry point. All application logic lives in `assay_tui::app`
//! so it can be reached by integration tests.

use std::sync::mpsc;

use assay_tui::app::App;
use assay_tui::event::TuiEvent;
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

    let (tx, rx) = mpsc::channel::<TuiEvent>();

    // Wire the event sender into App so the `r` key handler can forward agent events.
    app.event_tx = Some(tx.clone());

    // Spawn a background thread to forward crossterm events into the channel.
    let tx_cross = tx.clone();
    std::thread::spawn(move || {
        loop {
            if let Ok(e) = event::read() {
                match e {
                    Event::Key(k) => {
                        let _ = tx_cross.send(TuiEvent::Key(k));
                    }
                    Event::Resize(w, h) => {
                        let _ = tx_cross.send(TuiEvent::Resize(w, h));
                    }
                    _ => {}
                }
            }
        }
    });

    loop {
        terminal.draw(|frame| app.draw(frame))?;

        match rx.recv() {
            Ok(TuiEvent::Key(key)) => {
                if app.handle_event(key) {
                    break;
                }
            }
            Ok(TuiEvent::Resize(..)) => {
                terminal.clear()?;
            }
            Ok(TuiEvent::AgentLine(line)) => {
                app.handle_agent_line(line);
            }
            Ok(TuiEvent::AgentDone { exit_code }) => {
                app.handle_agent_done(exit_code);
            }
            Err(_) => break,
        }
    }

    Ok(())
}
