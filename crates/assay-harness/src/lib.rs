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
