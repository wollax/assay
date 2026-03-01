use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Layout},
    style::{Style, Stylize},
    text::Line,
    widgets::Paragraph,
};

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
    loop {
        terminal.draw(|frame| {
            let [area] = Layout::vertical([Constraint::Fill(1)]).areas(frame.area());
            let title = Line::from("Assay TUI").bold();
            let text = Paragraph::new(title).centered().style(Style::default());
            frame.render_widget(text, area);
        })?;

        if let Event::Key(key) = event::read()?
            && key.code == KeyCode::Char('q')
        {
            break;
        }
    }
    Ok(())
}
