//! Assay TUI — terminal user interface for assay project management.
//!
//! Thin binary entry point. All application logic lives in `assay_tui::app`
//! so it can be reached by integration tests.

use assay_tui::app::App;
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

    loop {
        terminal.draw(|frame| app.draw(frame))?;

        if let Event::Key(key) = event::read()?
            && app.handle_event(key)
        {
            break;
        }
    }
    Ok(())
}
