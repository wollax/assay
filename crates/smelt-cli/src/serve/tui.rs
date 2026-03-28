use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::widgets::{Block, Borders, Cell, Row, Table};

use crate::serve::queue::ServerState;
use crate::serve::types::{JobSource, elapsed_secs_since};

/// Spawn the TUI background thread. Returns a JoinHandle.
/// `shutdown` is an AtomicBool: when set to `true`, the TUI loop exits.
/// The TUI loop also sets `shutdown = true` when the user presses `q` or Ctrl+C.
pub(crate) fn run_tui(
    state: Arc<Mutex<ServerState>>,
    shutdown: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut terminal = ratatui::init();
        let result = tui_loop(&mut terminal, state, Arc::clone(&shutdown));
        ratatui::restore();
        if let Err(e) = result {
            eprintln!("TUI error: {e}");
        }
        // Signal shutdown in case TUI was the one that exited first
        shutdown.store(true, Ordering::SeqCst);
    })
}

fn tui_loop(
    terminal: &mut DefaultTerminal,
    state: Arc<Mutex<ServerState>>,
    shutdown: Arc<AtomicBool>,
) -> io::Result<()> {
    loop {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }
        terminal.draw(|frame| render(frame, &state))?;
        // Poll for key events without blocking (non-blocking check)
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') => {
                    shutdown.store(true, Ordering::SeqCst);
                    break;
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    shutdown.store(true, Ordering::SeqCst);
                    break;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub(crate) fn render(frame: &mut Frame, state: &Arc<Mutex<ServerState>>) {
    // Lock briefly — clone only what we need, then release immediately
    let jobs = {
        let s = state.lock().unwrap();
        s.jobs
            .iter()
            .map(|j| {
                let elapsed = j
                    .started_at
                    .map(|t| format!("{}s", elapsed_secs_since(t) as u64))
                    .unwrap_or_else(|| "-".to_string());
                let manifest_name = j
                    .manifest_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .to_string();
                let worker = j.worker_host.clone().unwrap_or_else(|| "-".to_string());
                let source = match j.source {
                    JobSource::Tracker => "Tracker",
                    JobSource::HttpApi => "HTTP",
                    JobSource::DirectoryWatch => "DirWatch",
                };
                (
                    j.id.to_string(),
                    manifest_name,
                    format!("{:?}", j.status),
                    j.attempt.to_string(),
                    elapsed,
                    worker,
                    source,
                )
            })
            .collect::<Vec<_>>()
    };

    let rows: Vec<Row> = jobs
        .iter()
        .map(|(id, name, status, attempt, elapsed, worker, source)| {
            Row::new(vec![
                Cell::from(id.as_str()),
                Cell::from(name.as_str()),
                Cell::from(*source),
                Cell::from(status.as_str()),
                Cell::from(attempt.as_str()),
                Cell::from(elapsed.as_str()),
                Cell::from(worker.as_str()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(10), // id
        Constraint::Fill(1),    // name (variable)
        Constraint::Length(10), // source
        Constraint::Length(12), // status
        Constraint::Length(8),  // attempt
        Constraint::Length(8),  // elapsed
        Constraint::Length(16), // worker
    ];

    let table = Table::new(rows, widths)
        .header(Row::new(vec![
            "Job ID", "Manifest", "Source", "Status", "Attempt", "Elapsed", "Worker",
        ]))
        .block(
            Block::default()
                .title("smelt serve — jobs")
                .borders(Borders::ALL),
        );

    frame.render_widget(table, frame.area());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::queue::ServerState;
    use crate::serve::types::{JobId, JobSource, JobStatus, QueuedJob, now_epoch};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::path::PathBuf;

    /// Helper: render the TUI into a `TestBackend` and return the buffer text.
    fn render_to_text(state: &Arc<Mutex<ServerState>>) -> String {
        let backend = TestBackend::new(120, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, state)).unwrap();
        let buf = terminal.backend().buffer().clone();
        buf.content
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect()
    }

    #[test]
    fn test_tui_render_worker_host() {
        let mut state = ServerState::new(1);
        state.jobs.push_back(QueuedJob {
            id: JobId::new("job-w1"),
            manifest_path: PathBuf::from("test.smelt.toml"),
            source: JobSource::HttpApi,
            attempt: 1,
            status: JobStatus::Running,
            queued_at: now_epoch(),
            started_at: Some(now_epoch()),
            worker_host: Some("worker-1".into()),
        });
        // Also add a job with no worker_host to test "-" rendering
        state.jobs.push_back(QueuedJob {
            id: JobId::new("job-l1"),
            manifest_path: PathBuf::from("local.smelt.toml"),
            source: JobSource::HttpApi,
            attempt: 0,
            status: JobStatus::Queued,
            queued_at: now_epoch(),
            started_at: None,
            worker_host: None,
        });

        let shared = Arc::new(Mutex::new(state));
        let text = render_to_text(&shared);

        assert!(
            text.contains("worker-1"),
            "expected 'worker-1' in TUI output"
        );
        assert!(
            text.contains("Worker"),
            "expected 'Worker' header in TUI output"
        );
        assert!(
            text.contains("Source"),
            "expected 'Source' header in TUI output"
        );
        assert!(
            text.contains("HTTP"),
            "expected 'HTTP' source for HttpApi jobs"
        );
    }

    #[test]
    fn test_tui_render_tracker_source() {
        let mut state = ServerState::new(1);
        state.jobs.push_back(QueuedJob {
            id: JobId::new("job-t1"),
            manifest_path: PathBuf::from("tracker-issue.smelt.toml"),
            source: JobSource::Tracker,
            attempt: 0,
            status: JobStatus::Queued,
            queued_at: now_epoch(),
            started_at: None,
            worker_host: None,
        });

        let shared = Arc::new(Mutex::new(state));
        let text = render_to_text(&shared);

        assert!(
            text.contains("Source"),
            "expected 'Source' header in TUI output"
        );
        assert!(
            text.contains("Tracker"),
            "expected 'Tracker' in TUI output for tracker-sourced job"
        );
    }

    #[test]
    fn test_tui_render_dirwatch_source() {
        let mut state = ServerState::new(1);
        state.jobs.push_back(QueuedJob {
            id: JobId::new("job-d1"),
            manifest_path: PathBuf::from("watched.smelt.toml"),
            source: JobSource::DirectoryWatch,
            attempt: 0,
            status: JobStatus::Queued,
            queued_at: now_epoch(),
            started_at: None,
            worker_host: None,
        });

        let shared = Arc::new(Mutex::new(state));
        let text = render_to_text(&shared);

        assert!(
            text.contains("DirWatch"),
            "expected 'DirWatch' in TUI output for directory-watch job"
        );
    }
}
