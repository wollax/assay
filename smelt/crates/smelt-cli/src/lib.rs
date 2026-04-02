//! Smelt CLI library — exposes command implementations for integration testing.

#![deny(missing_docs)]

/// CLI subcommand implementations (init, list, run, serve, status, watch).
pub mod commands;
/// Long-running serve mode — HTTP API, SSH queue, dispatch loop.
pub mod serve;
