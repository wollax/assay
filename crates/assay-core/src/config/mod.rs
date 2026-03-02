//! Configuration loading and validation.
//!
//! Handles reading, parsing, and validating Assay project configuration
//! from files and environment.

use std::fmt;

/// A single validation issue in a config file.
#[derive(Debug, Clone)]
pub struct ConfigError {
    /// The field path (e.g., "project_name", "[gates].default_timeout").
    pub field: String,
    /// What's wrong.
    pub message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}
