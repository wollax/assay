//! Team state checkpointing: extraction, persistence, and team config discovery.
//!
//! Builds on the session JSONL parser (`context::parse_session`) to extract
//! agent and task state, then persists checkpoints as JSON frontmatter + markdown.

pub mod config;
pub mod extractor;
pub mod persistence;

pub use config::{discover_team_config, merge_team_config};
pub use extractor::extract_team_state;
pub use persistence::{list_checkpoints, load_latest_checkpoint, save_checkpoint, CheckpointEntry};
