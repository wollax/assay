//! Worktree lifecycle management for agent sessions.

pub mod state;

pub use state::{GitWorktreeEntry, SessionStatus, WorktreeState, parse_porcelain};
