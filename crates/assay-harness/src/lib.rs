#![deny(missing_docs)]
//! Agent harness adapters for Assay.
//!
//! Provides prompt building, settings merging, and agent-specific config
//! generation for agentic coding sessions.

/// Prompt assembly from layered prompt sources.
pub mod prompt;

/// Settings merging and override resolution.
pub mod settings;

/// Claude Code adapter for harness profile generation.
pub mod claude;

/// Codex adapter for harness profile generation.
pub mod codex;

/// OpenCode adapter for harness profile generation.
pub mod opencode;

/// Scope enforcement and multi-agent prompt generation.
pub mod scope;

/// Claude streaming NDJSON parser.
pub mod claude_stream;

/// Harness provider trait and built-in implementations.
pub mod provider;

pub use assay_types::{HarnessError, HarnessProvider};
