mod config;
mod dispatch;
mod events;
mod http;
mod queue;
mod signals;
mod ssh_dispatch;

use std::path::PathBuf;

use crate::serve::queue::ServerState;

/// Minimal valid manifest TOML shared across all serve test modules.
const VALID_MANIFEST_TOML: &str = r#"[job]
name = "test-job"
repo = "."
base_ref = "main"

[environment]
runtime = "docker"
image = "alpine:3.18"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "main"
spec = "Run the test"
harness = "echo hello"
timeout = 300

[merge]
strategy = "sequential"
target = "main"
"#;

fn manifest() -> PathBuf {
    PathBuf::from("/tmp/test.toml")
}

/// Helper: create a `ServerState` with a pre-enqueued job so event POST tests
/// have a valid `job_id` to target. Returns `(Arc<Mutex<ServerState>>, job_id)`.
fn state_with_job() -> (std::sync::Arc<std::sync::Mutex<ServerState>>, String) {
    use crate::serve::types::{JobId, JobSource, JobStatus, QueuedJob, now_epoch};
    let state = std::sync::Arc::new(std::sync::Mutex::new(ServerState::new_without_events(4)));
    let job_id = {
        let mut s = state.lock().unwrap();
        s.jobs.push_back(QueuedJob {
            id: JobId::new("test-job-1"),
            manifest_path: PathBuf::from("test.smelt.toml"),
            source: JobSource::HttpApi,
            attempt: 0,
            status: JobStatus::Running,
            queued_at: now_epoch(),
            started_at: Some(now_epoch()),
            worker_host: None,
        });
        "test-job-1".to_string()
    };
    (state, job_id)
}

/// Helper: spin up an axum server on an OS-assigned port, return the base URL.
async fn start_test_server(state: std::sync::Arc<std::sync::Mutex<ServerState>>) -> String {
    start_test_server_with_auth(state, None).await
}

/// Helper: spin up an axum server with optional auth, return the base URL.
async fn start_test_server_with_auth(
    state: std::sync::Arc<std::sync::Mutex<ServerState>>,
    auth: Option<crate::serve::http_api::ResolvedAuth>,
) -> String {
    start_test_server_with_cancel(state, auth, tokio_util::sync::CancellationToken::new()).await
}

/// Helper: spin up an axum server with optional auth and a specific CancellationToken.
async fn start_test_server_with_cancel(
    state: std::sync::Arc<std::sync::Mutex<ServerState>>,
    auth: Option<crate::serve::http_api::ResolvedAuth>,
    cancel_token: tokio_util::sync::CancellationToken,
) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = crate::serve::http_api::build_router(state, auth, cancel_token);
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{addr}")
}

// ──────────────────────────────────────────────
// TUI render test (small, stays in mod.rs)
// ──────────────────────────────────────────────

#[test]
fn test_tui_render_no_panic() {
    use crate::serve::tui::render;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::sync::{Arc, Mutex};

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = Arc::new(Mutex::new(ServerState::new_without_events(2)));

    // Render with empty state — must not panic
    terminal.draw(|frame| render(frame, &state)).unwrap();

    // Add a mock job entry to state and render again
    // (directly mutate queue for test — no manifest file needed)
    {
        use crate::serve::types::{JobId, JobSource, JobStatus, QueuedJob, now_epoch};
        use std::path::PathBuf;
        let mut s = state.lock().unwrap();
        s.jobs.push_back(QueuedJob {
            id: JobId::new("job-1"),
            manifest_path: PathBuf::from("test-manifest.toml"),
            source: JobSource::HttpApi,
            attempt: 0,
            status: JobStatus::Running,
            queued_at: now_epoch(),
            started_at: Some(now_epoch()),
            worker_host: None,
        });
    }
    terminal.draw(|frame| render(frame, &state)).unwrap();
}
