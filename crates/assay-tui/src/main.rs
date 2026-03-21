use assay_core::{config, milestone::milestone_scan};
use assay_tui::{App, Screen};
use ratatui::widgets::ListState;
use std::env::current_dir;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let project_root = current_dir()?;
    let assay_dir = project_root.join(".assay");

    let (milestones, scan_error, config, config_error, initial_screen) = if assay_dir.exists() {
        // Capture milestone_scan errors rather than silently treating them as "no
        // milestones". A corrupt TOML or permission error produces the same empty
        // list either way, but scan_error is rendered in the dashboard so the user
        // knows their data failed to load.
        let (milestones, scan_error) = match milestone_scan(&assay_dir) {
            Ok(m) => (m, None),
            Err(e) => (vec![], Some(format!("Failed to load milestones: {e}"))),
        };

        // Distinguish "config.toml doesn't exist yet" (normal on a fresh project,
        // no warning needed) from "config.toml exists but failed to parse" (user
        // needs to know — e.g. deny_unknown_fields rejecting an old key). Print a
        // warning to stderr before ratatui::init() while the terminal is still in
        // normal mode, and store the error in App for future Settings screen display.
        let config_path = project_root.join(".assay").join("config.toml");
        let (config, config_error) = if config_path.exists() {
            match config::load(&project_root) {
                Ok(c) => (Some(c), None),
                Err(e) => {
                    let msg = format!("config.toml found but failed to load: {e}");
                    eprintln!("assay-tui warning: {msg}");
                    (None, Some(msg))
                }
            }
        } else {
            (None, None)
        };

        (
            milestones,
            scan_error,
            config,
            config_error,
            Screen::Dashboard,
        )
    } else {
        (vec![], None, None, None, Screen::NoProject)
    };

    let mut app = App {
        screen: initial_screen,
        milestones,
        list_state: ListState::default(),
        project_root: Some(project_root),
        config,
        show_help: false,
        scan_error,
        config_error,
    };

    let terminal = ratatui::init();
    let result = assay_tui::run(&mut app, terminal);
    ratatui::restore();
    result
}
