/// Assay event types, bounded store, and broadcast bus.
pub(crate) mod events;
/// Cross-job PeerUpdate routing via [[notify]] rules in JobManifest.
pub(crate) mod notify;
/// Persistent job queue and concurrency controller.
pub mod queue;
/// PeerUpdate signal types and filesystem-based delivery to Assay session inboxes.
pub(crate) mod signals;
/// Job data types: identifiers, status, source, and queue entries.
pub mod types;
pub(crate) use queue::ServerState;
pub(crate) mod config;
pub(crate) use config::ServerConfig;
/// GitHub client abstraction for interacting with GitHub Issues via `gh` CLI.
pub mod github;
/// Linear client abstraction for interacting with Linear Issues via GraphQL API.
pub mod linear;
/// SSH transport layer for dispatching jobs to remote workers.
pub mod ssh;
/// Tracker source trait and types for polling issues from external trackers.
pub mod tracker;
pub use ssh::{SshClient, SshOutput, SubprocessSshClient};
pub(crate) mod dispatch;
pub(crate) use dispatch::dispatch_loop;
pub(crate) mod tracker_poller;
pub(crate) use tracker_poller::{AnyTrackerSource, TrackerPoller};
pub(crate) mod queue_watcher;
pub(crate) use queue_watcher::DirectoryWatcher;
pub(crate) mod http_api;
pub(crate) use http_api::build_router;
pub(crate) mod tui;
pub(crate) use tui::run_tui;
#[cfg(test)]
mod tests;
