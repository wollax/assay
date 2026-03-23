//! Assay TUI — terminal user interface for assay project management.
//!
//! Thin binary entry point. All application logic lives in `assay_tui::app`
//! so it can be reached by integration tests.

use std::sync::mpsc;

use assay_tui::app::App;
use crossterm::event::{self, Event};
use ratatui::DefaultTerminal;

/// Events dispatched through the TUI event loop.
///
/// The channel-based dispatch loop replaced the blocking `event::read()` in
/// `run()`. `AgentLine` and `AgentDone` variants are sent by the agent
/// background thread; `Key` and `Resize` are sent by the crossterm thread.
pub enum TuiEvent {
    /// A keyboard event from crossterm.
    Key(crossterm::event::KeyEvent),
    /// A terminal resize event.
    Resize(u16, u16),
    /// A single line of stdout from the agent subprocess.
    AgentLine(String),
    /// The agent subprocess has exited with the given exit code.
    AgentDone { exit_code: i32 },
}

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

    // Spawn a background thread to forward crossterm events into the channel.
    let tx_cross = tx.clone();
    std::thread::spawn(move || loop {
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
