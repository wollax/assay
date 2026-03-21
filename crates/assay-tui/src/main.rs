use assay_core::{config, milestone::milestone_scan};
use assay_tui::{App, Screen};
use ratatui::widgets::ListState;
use std::env::current_dir;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let project_root = current_dir()?;
    let assay_dir = project_root.join(".assay");

    let (milestones, config, initial_screen) = if assay_dir.exists() {
        let milestones = milestone_scan(&assay_dir).unwrap_or_default();
        let config = config::load(&project_root).ok();
        (milestones, config, Screen::Dashboard)
    } else {
        (vec![], None, Screen::NoProject)
    };

    let mut app = App {
        screen: initial_screen,
        milestones,
        list_state: ListState::default(),
        project_root: Some(project_root),
        config,
        show_help: false,
    };

    let terminal = ratatui::init();
    let result = assay_tui::run(&mut app, terminal);
    ratatui::restore();
    result
}
