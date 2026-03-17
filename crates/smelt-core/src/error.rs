//! Unified error type for Smelt core operations.

use std::path::PathBuf;

use thiserror::Error;

/// Unified error type for Smelt core operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SmeltError {
    // ── Git errors ──────────────────────────────────────────────

    /// `git` binary not found on `$PATH`.
    #[error("`git` not found on $PATH. Smelt requires git to be installed.")]
    GitNotFound,

    /// Current directory is not inside a git repository.
    #[error("not a git repository (or any parent up to mount point)")]
    NotAGitRepo,

    /// A git command failed.
    #[error("git {operation} failed: {message}")]
    GitExecution { operation: String, message: String },

    /// Merge conflict occurred.
    #[error("merge conflict in session '{session}': conflicting files: {}", files.join(", "))]
    MergeConflict { session: String, files: Vec<String> },

    // ── Manifest errors ─────────────────────────────────────────

    /// Manifest parsing or validation error.
    #[error("manifest error ({field}): {message}")]
    Manifest { field: String, message: String },

    // ── Provider errors ─────────────────────────────────────────

    /// A runtime provider operation failed.
    #[error("provider {operation} failed: {message}")]
    Provider {
        operation: String,
        message: String,
        /// Optional underlying cause.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    // ── Credential errors ───────────────────────────────────────

    /// Credential resolution or validation failed.
    #[error("credential error ({provider}): {message}")]
    Credential { provider: String, message: String },

    // ── Config errors ───────────────────────────────────────────

    /// Configuration loading or parsing failed.
    #[error("config error at `{path}`: {message}")]
    Config { path: PathBuf, message: String },

    // ── I/O errors ──────────────────────────────────────────────

    /// An I/O operation failed with context.
    #[error("{operation} at `{path}`: {source}")]
    Io {
        operation: String,
        path: PathBuf,
        source: std::io::Error,
    },
}

impl SmeltError {
    /// Convenience constructor for the [`Io`](SmeltError::Io) variant.
    pub fn io(
        operation: impl Into<String>,
        path: impl Into<PathBuf>,
        source: std::io::Error,
    ) -> Self {
        Self::Io {
            operation: operation.into(),
            path: path.into(),
            source,
        }
    }

    /// Convenience constructor for the [`Provider`](SmeltError::Provider) variant
    /// without an underlying source error.
    pub fn provider(operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            operation: operation.into(),
            message: message.into(),
            source: None,
        }
    }

    /// Convenience constructor for the [`Provider`](SmeltError::Provider) variant
    /// with an underlying source error.
    pub fn provider_with_source(
        operation: impl Into<String>,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Provider {
            operation: operation.into(),
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Convenience constructor for the [`Credential`](SmeltError::Credential) variant.
    pub fn credential(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Credential {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// Convenience constructor for the [`Config`](SmeltError::Config) variant.
    pub fn config(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Config {
            path: path.into(),
            message: message.into(),
        }
    }
}

/// A `Result` alias that uses [`SmeltError`] as the error type.
pub type Result<T> = std::result::Result<T, SmeltError>;
