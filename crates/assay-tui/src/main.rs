fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let mut app = assay_tui::App {
        screen: assay_tui::Screen::Dashboard,
        milestones: vec![],
        list_state: ratatui::widgets::ListState::default(),
        project_root: None,
        config: None,
        show_help: false,
    };
    let result = assay_tui::run(&mut app, terminal);
    ratatui::restore();
    result
}
