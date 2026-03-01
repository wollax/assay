//! Tracing initialization for the MCP server.
//!
//! All output goes to stderr — stdout is reserved exclusively for JSON-RPC.

use tracing_subscriber::EnvFilter;

/// Initialize tracing-subscriber with stderr output.
///
/// Default level is `warn`. Override via `RUST_LOG` environment variable.
/// Uses `try_init()` to avoid panicking on double-initialization.
pub(crate) fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init();
}
