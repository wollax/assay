//! Assay TUI — terminal user interface for assay project management.
//!
//! Thin binary entry point. All application logic lives in `assay_tui::app`
//! so it can be reached by integration tests.

use assay_tui::app::{App, TuiEvent};
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
    let (tx, rx) = std::sync::mpsc::channel::<TuiEvent>();
    app.event_tx = Some(tx.clone());

    // Background thread: crossterm events → TuiEvent channel.
    // This thread blocks forever on event::read() — that is intentional and
    // acceptable; the OS reclaims it on process exit.
    std::thread::spawn(move || {
        loop {
            match crossterm::event::read() {
                Ok(crossterm::event::Event::Key(k)) => {
                    let _ = tx.send(TuiEvent::Key(k));
                }
                Ok(crossterm::event::Event::Resize(w, h)) => {
                    let _ = tx.send(TuiEvent::Resize(w, h));
                }
                _ => {}
            }
        }
    });

    loop {
        terminal.draw(|frame| app.draw(frame))?;
        match rx.recv() {
            Ok(TuiEvent::Key(k)) => {
                if app.handle_event(k) {
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
            Err(_) => break, // channel disconnected
        }
    }
    Ok(())
}
