//! Assay TUI — terminal user interface for assay project management.
//!
//! Thin binary entry point. All application logic lives in `assay_tui::app`
//! so it can be reached by integration tests.

use std::process::{Command, Stdio};

use assay_core::pr::pr_status_poll;
use assay_tui::app::{App, TuiEvent};
use ratatui::DefaultTerminal;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Initialize centralized tracing subscriber before any other logic.
    // The guard must live until process exit to flush buffered events.
    let _tracing_guard =
        assay_core::telemetry::init_tracing(assay_core::telemetry::TracingConfig::default());

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
    let crossterm_tx = tx.clone();
    std::thread::spawn(move || {
        loop {
            match crossterm::event::read() {
                Ok(crossterm::event::Event::Key(k)) => {
                    let _ = crossterm_tx.send(TuiEvent::Key(k));
                }
                Ok(crossterm::event::Event::Resize(w, h)) => {
                    let _ = crossterm_tx.send(TuiEvent::Resize(w, h));
                }
                _ => {}
            }
        }
    });

    // Background thread: PR status polling.
    // Only spawned when `gh` is available and at least one milestone has a PR number.
    let gh_available = Command::new("gh")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok();

    if gh_available {
        let poll_targets = app.poll_targets.clone();
        let has_targets = poll_targets.lock().map(|g| !g.is_empty()).unwrap_or(false);
        if has_targets {
            let poll_tx = app.event_tx.clone().unwrap();
            std::thread::spawn(move || {
                let mut first = true;
                loop {
                    if !first {
                        std::thread::sleep(std::time::Duration::from_secs(60));
                    }
                    first = false;

                    let targets = match poll_targets.lock() {
                        Ok(guard) => guard.clone(),
                        Err(_) => continue,
                    };

                    for (slug, pr_number) in &targets {
                        let _ = std::panic::catch_unwind(|| {
                            match pr_status_poll(*pr_number) {
                                Ok(info) => {
                                    let _ = poll_tx.send(TuiEvent::PrStatusUpdate {
                                        slug: slug.clone(),
                                        info,
                                    });
                                }
                                Err(_) => {
                                    // Polling errors are silently skipped —
                                    // absent badge is the degradation signal.
                                }
                            }
                        });
                    }
                }
            });
        }
    } else {
        tracing::warn!("gh CLI not found — PR status polling disabled");
    }

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
            Ok(TuiEvent::AgentEvent(event)) => {
                app.handle_agent_event(event);
            }
            Ok(TuiEvent::AgentDone { exit_code }) => {
                app.handle_agent_done(exit_code);
            }
            Ok(TuiEvent::PrStatusUpdate { slug, info }) => {
                app.handle_pr_status_update(slug, info);
            }
            Err(_) => break, // channel disconnected
        }
    }
    Ok(())
}
