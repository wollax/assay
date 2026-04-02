# S03: Ratatui TUI + Server Config + Graceful Shutdown — Research

**Researched:** 2026-03-23
**Domain:** Ratatui TUI, ServerConfig TOML, `smelt serve` CLI wiring, graceful shutdown
**Confidence:** HIGH

---

## Summary

S03 is pure integration and wiring — all the functional building blocks (`dispatch_loop`, `DirectoryWatcher`, `build_router`, `ServerState`) were built in S01/S02. This slice adds three new capabilities on top of them: (1) a `ServerConfig` TOML struct parsed from `--config server.toml`; (2) the `smelt serve` subcommand in `main.rs` that wires all components as concurrent tokio tasks + one background thread for the TUI; (3) a Ratatui TUI that reads `Arc<Mutex<ServerState>>` and renders a live job table.

The biggest implementation risk is the **tracing + crossterm raw-mode conflict**. The existing `tracing_subscriber::fmt().with_writer(std::io::stderr)` subscriber in `main.rs` will corrupt TUI output if left in place during TUI mode. The standard mitigation is to redirect tracing output to a file via `tracing-appender` when TUI mode is active, or disable stderr output entirely and let `smelt serve --no-tui` use the normal stderr path.

The second risk is **crossterm terminal restore on panic**. The TUI background thread must call `ratatui::restore()` (or equivalent `crossterm::terminal::disable_raw_mode` + `LeaveAlternateScreen`) even on panic. The `ratatui::init()` / `ratatui::restore()` convenience functions handle this automatically; using them is strongly preferred over manual terminal setup.

**Recommendation:** Use `ratatui = "0.29"` (MSRV 1.74, compatible with workspace's 1.85 floor). The ratatui async example (`examples/async.rs`) is the canonical pattern for the `Arc<RwLock<State>>` + tokio + Table widget combination — follow it closely. For tracing, when TUI mode is active, swap `with_writer(stderr)` to `with_writer(tracing_appender::rolling::never(".", ".smelt/serve.log"))`.

---

## Recommendation

**Wire S03 in four tasks:**

- **T01**: `ServerConfig` struct + `examples/server.toml` (pure TOML, no runtime deps)
- **T02**: `smelt serve` subcommand in `main.rs` — starts all 4 concurrent tasks (dispatch_loop, DirectoryWatcher, axum HTTP, TUI) with `tokio::join!` / `tokio::select!`; Ctrl+C handling via `tokio::signal::ctrl_c()`
- **T03**: Ratatui TUI in `serve/tui.rs` — background `std::thread::spawn`, reads `Arc<Mutex<ServerState>>`, renders `Table` widget every 250ms; `q` / Ctrl+C exits
- **T04**: Integration — tracing redirect (file appender in TUI mode), `--no-tui` flag, `cargo test --workspace` all green

This ordering keeps T01 and T02 individually verifiable without the TUI and ensures T03 can be tested in isolation before the full wiring in T04.

---

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Terminal raw mode + alternate screen | `ratatui::init()` / `ratatui::restore()` | One-line setup/teardown; handles panic restoration via `color-eyre` hook or a simple `std::panic::set_hook` |
| Table widget with live-updating rows | `ratatui::widgets::Table` + `Row` | No custom rendering needed; `Constraint::Length` + `Constraint::Fill` handles column sizing |
| TOML config file parsing | `toml::from_str` + serde | Already a workspace dep; same pattern as `JobManifest` parsing |
| File-based tracing during TUI | `tracing-appender` (already a transitive dep via ratatui examples; add as direct dep) | Avoids stderr corruption in raw mode; non-rolling file is simplest |

---

## Existing Code and Patterns

- `crates/smelt-cli/src/main.rs` — tracing subscriber init; must be modified to conditionally redirect to file when `--config` flag is present and TUI is active
- `crates/smelt-cli/src/serve/dispatch.rs` — `dispatch_loop(state, cancel_token, max_attempts)` async fn; signature is final
- `crates/smelt-cli/src/serve/queue_watcher.rs` — `DirectoryWatcher::new(queue_dir, state).watch()` async fn
- `crates/smelt-cli/src/serve/http_api.rs` — `build_router(state) -> Router`; bind with `tokio::net::TcpListener`
- `crates/smelt-cli/src/serve/queue.rs` — `ServerState::new(max_concurrent)` — all fields are public; TUI reads `state.jobs` directly
- `crates/smelt-cli/src/serve/types.rs` — `QueuedJob { id, manifest_path, status, attempt, queued_at, started_at }` — all fields needed for TUI columns
- `crates/smelt-cli/Cargo.toml` — already has `axum`, `tokio-util`, `serde_json`, `tempfile`; need to add `ratatui` + `crossterm` + `tracing-appender`
- `examples/*.toml` — existing example manifests; `examples/server.toml` follows the same directory
- `crates/smelt-core/src/manifest.rs` — `JobManifest::from_str` + `validate()` for reference on validation pattern

---

## Constraints

- **`ratatui = "0.29"`** (MSRV 1.74) is required — `ratatui = "0.30"` has MSRV 1.86 which conflicts with the workspace `rust-version = "1.85"` declaration (Cargo will warn/error). `0.29.0` resolves crossterm to `0.28.1`, which is already in the registry.
- **TUI thread must be `std::thread::spawn`, not `tokio::spawn`** — Ratatui's crossterm backend calls blocking I/O (`read()` for keyboard events); `tokio::spawn` on a blocking call starves the runtime. The pattern from the roadmap (D101, M006 context) is confirmed by the ratatui async example.
- **`Arc<Mutex<ServerState>>` (not `RwLock`)** — `ServerState` is already wrapped in `Arc<Mutex<>>` in S01/S02; the TUI must use the same type to avoid introducing a second lock wrapper.
- **`smelt serve` is `--no-tui` skippable** — R025 specifies "disable with `--no-tui` flag"; when `--no-tui`, the background thread is never spawned and tracing stays on stderr.
- **`tokio::signal::ctrl_c()` is the existing pattern** (used in `run.rs`); for `smelt serve`, Ctrl+C must call `cancel_token.cancel()` which broadcasts to all child job tokens via D099.
- **tracing subscriber is initialized once in `main()`** — the subscriber must be chosen before the `smelt serve` branch is entered; can't re-init after first `init()` call.
- **`D034` (TOML state files)** — `run_job_task` in S01 calls `run_with_cancellation()` which writes per-job `.smelt/runs/<name>/state.toml`; `smelt status <job>` will continue to work without any changes to `status.rs`.

---

## Common Pitfalls

- **Tracing to stderr corrupts TUI** — Raw mode intercepts stdin but NOT stderr. Every `tracing::info!` line written to stderr will appear as garbled text over the TUI. Fix: in `main()`, when `Commands::Serve` is matched and `!args.no_tui`, init the subscriber with `tracing_appender::rolling::never(".smelt", "serve.log")` as writer. When `--no-tui`, use the normal stderr path.

- **Not calling `ratatui::restore()` on panic** — If the TUI thread panics, the terminal stays in raw mode and becomes unusable. Fix: call `ratatui::restore()` in a `std::panic::set_hook` wrapper around the TUI thread's body, or use `color-eyre`'s panic hook which does this automatically. Alternatively, the `ratatui::init()` function in 0.29 installs a panic hook.

- **Locking `ServerState` in the TUI thread for too long** — The TUI render function snapshots job data; if it holds the lock across the entire render, the dispatch loop and HTTP handlers will block waiting. Fix: lock briefly, clone the needed fields into a local `Vec`, then render from the local copy.

- **TUI thread joining before dispatch_loop drains** — On Ctrl+C, the sequence must be: (1) cancel_token.cancel() → dispatch_loop breaks, running job tasks receive their child token cancellation and call teardown → (2) wait for all jobs to complete (or timeout) → (3) join TUI thread. If the TUI thread is joined first, the terminal is restored before cleanup, corrupting output. Use a `std::sync::mpsc` or an `Arc<AtomicBool>` to signal the TUI thread to exit after all jobs drain.

- **`ratatui::init()` vs manual crossterm setup** — `ratatui::init()` in 0.29 calls `enable_raw_mode()` + `enter_alternate_screen()` + installs a panic hook. This is exactly what we want. Do NOT do the setup manually; the panic hook is easy to forget and will break terminal state.

- **Column widths for the job table** — `Constraint::Length(n)` for fixed-width columns (status, attempt, elapsed), `Constraint::Fill(1)` for the manifest name (variable). If the terminal is too narrow, ratatui clips cells gracefully.

- **MSRV conflict with `ratatui = "0.30"`** — `0.30.0` declares `rust-version = "1.86.0"`. Our workspace declares `rust-version = "1.85"`. Even though `rustc 1.93.1` can compile it, `cargo check` will emit a `rust-version` compatibility warning/error for the workspace. Pin to `0.29`.

---

## Open Risks

- **Graceful shutdown timing** — `smelt serve` with 2 jobs running and Ctrl+C: job tasks call `run_with_cancellation()` which must invoke container teardown before returning. If a job is in the middle of docker exec, teardown can take 5-30s. The main `smelt serve` process should wait for all spawned `tokio::spawn` handles to complete before exiting. Collecting `JoinHandle`s from `dispatch_loop` is the mechanism. Currently `dispatch_loop` does `tokio::spawn(...)` without collecting handles — the dispatch loop needs to return all JoinHandles or use a `JoinSet` to allow the caller to wait.

- **TUI in CI / non-interactive terminals** — `crossterm::terminal::is_raw_mode_enabled()` or checking `TERM`/`NO_COLOR` env vars can detect non-interactive environments. Consider auto-disabling TUI (as if `--no-tui`) when `!std::io::stdout().is_terminal()` (using `std::io::IsTerminal` stabilized in 1.70).

- **Job ID uniqueness** — The current `job-{n}` counter in `queue.rs::new_job_id()` uses a process-lifetime atomic counter. On restart, IDs start from 1 again. This is fine for the in-memory queue but could confuse `smelt status` if users try to look up a job-id from a prior run. Not a blocker for S03 but worth noting.

---

## Implementation Details

### ServerConfig Structure

```toml
# server.toml
queue_dir = "/tmp/smelt-queue"
max_concurrent = 2
retry_attempts = 3
retry_backoff_secs = 5

[server]
host = "127.0.0.1"
port = 8765
```

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub queue_dir: PathBuf,
    pub max_concurrent: usize,
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    #[serde(default = "default_retry_backoff_secs")]
    pub retry_backoff_secs: u64,
    #[serde(default)]
    pub server: ServerNetworkConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerNetworkConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}
```

### TUI Pattern (from ratatui async example)

```rust
// serve/tui.rs
pub(crate) fn run_tui(state: Arc<Mutex<ServerState>>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut terminal = ratatui::init();
        let result = tui_loop(&mut terminal, state);
        ratatui::restore();
        if let Err(e) = result {
            eprintln!("TUI error: {e}");
        }
    })
}

fn tui_loop(terminal: &mut DefaultTerminal, state: Arc<Mutex<ServerState>>) -> io::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, &state))?;
        if crossterm::event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = crossterm::event::read()? {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                }
            }
        }
    }
    Ok(())
}
```

### smelt serve main wiring (sketch)

```rust
// In main.rs — Commands::Serve branch
let config = ServerConfig::load(&args.config)?;
let state = Arc::new(Mutex::new(ServerState::new(config.max_concurrent)));
let cancel_token = CancellationToken::new();

// Redirect tracing to file if TUI active
if !args.no_tui {
    // init file-appender subscriber
}

// Start 4 concurrent components:
let tui_handle = if !args.no_tui { Some(run_tui(Arc::clone(&state))) } else { None };
let watcher = DirectoryWatcher::new(config.queue_dir.clone(), Arc::clone(&state));
let router = build_router(Arc::clone(&state));
let listener = TcpListener::bind((config.server.host.as_str(), config.server.port)).await?;

tokio::select! {
    _ = dispatch_loop(Arc::clone(&state), cancel_token.clone(), config.retry_attempts) => {}
    _ = watcher.watch() => {}
    _ = axum::serve(listener, router) => {}
    _ = tokio::signal::ctrl_c() => {
        cancel_token.cancel();
    }
}

// Signal TUI to exit, join thread
if let Some(handle) = tui_handle {
    // ... signal stop, join
    handle.join().ok();
}
```

---

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | none found | none found — use ratatui async example as reference |

---

## Sources

- Ratatui async example (`~/.cargo/registry/src/.../ratatui-0.29.0/examples/async.rs`) — canonical pattern for `Arc<RwLock<State>>` + tokio + `Table` widget; `DefaultTerminal` type, `ratatui::init()` / `ratatui::restore()` convenience API (source: local registry cache, HIGH confidence)
- Ratatui table example (`~/.cargo/registry/src/.../ratatui-0.29.0/examples/table.rs`) — column constraints, `Row::new()`, `Table::new(rows, widths)` API shape (source: local registry cache, HIGH confidence)
- Ratatui 0.29.0 `Cargo.toml.orig` — crossterm 0.28.1 as dependency, MSRV 1.74.0 (source: local registry cache, HIGH confidence)
- Ratatui 0.30.0 `Cargo.toml` — MSRV 1.86.0 — do NOT use (source: local registry cache, HIGH confidence)
- D099 (M006) — CancellationToken broadcast pattern for multi-job cancellation; `cancel_token.child_token()` per job (source: `.kata/DECISIONS.md`)
- D101 (M006) — `ratatui` + `crossterm` chosen; `std::thread::spawn` TUI backend confirmed (source: `.kata/DECISIONS.md`)
- D102 (M006) — `ServerConfig` in separate `server.toml` file, not embedded in job manifests (source: `.kata/DECISIONS.md`)
