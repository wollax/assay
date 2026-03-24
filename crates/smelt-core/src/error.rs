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
    GitExecution {
        /// The git sub-command or operation that failed (e.g. `"fetch"`).
        operation: String,
        /// Human-readable error message from git.
        message: String,
    },

    /// Merge conflict occurred.
    #[error("merge conflict in session '{session}': conflicting files: {}", files.join(", "))]
    MergeConflict {
        /// Name of the session in which the conflict occurred.
        session: String,
        /// List of conflicting file paths.
        files: Vec<String>,
    },

    // ── Manifest errors ─────────────────────────────────────────
    /// Manifest parsing or validation error.
    #[error("manifest error ({field}): {message}")]
    Manifest {
        /// The manifest field that failed validation.
        field: String,
        /// Human-readable description of the validation failure.
        message: String,
    },

    // ── Provider errors ─────────────────────────────────────────
    /// A runtime provider operation failed.
    #[error("provider {operation} failed: {message}")]
    Provider {
        /// The provider operation that failed (e.g. `"execute"`).
        operation: String,
        /// Human-readable error message from the provider.
        message: String,
        /// Optional underlying cause.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    // ── Forge errors ────────────────────────────────────────────
    /// A forge (GitHub/VCS API) operation failed.
    #[error("forge {operation} failed: {message}")]
    Forge {
        /// The forge operation that failed (e.g. `"create_pr"`).
        operation: String,
        /// Human-readable error message from the forge API.
        message: String,
    },

    // ── Credential errors ───────────────────────────────────────
    /// Credential resolution or validation failed.
    #[error("credential error ({provider}): {message}")]
    Credential {
        /// Name of the credential provider (e.g. `"github"`).
        provider: String,
        /// Human-readable description of the credential failure.
        message: String,
    },

    // ── Config errors ───────────────────────────────────────────
    /// Configuration loading or parsing failed.
    #[error("config error at `{path}`: {message}")]
    Config {
        /// Path to the configuration file that could not be loaded.
        path: PathBuf,
        /// Human-readable description of the configuration error.
        message: String,
    },

    // ── I/O errors ──────────────────────────────────────────────
    /// An I/O operation failed with context.
    #[error("{operation} at `{path}`: {source}")]
    Io {
        /// Description of the I/O operation that failed (e.g. `"read"`).
        operation: String,
        /// Path to the file or directory involved in the failed operation.
        path: PathBuf,
        /// Underlying I/O error.
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

    /// Convenience constructor for the [`Forge`](SmeltError::Forge) variant.
    pub fn forge(operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Forge {
            operation: operation.into(),
            message: message.into(),
        }
    }

    /// Convenience constructor for the [`Forge`](SmeltError::Forge) variant
    /// with an underlying source error stringified into the message.
    ///
    /// Note: if `octocrab::Error` proves to implement `Send + Sync + 'static`
    /// and a chain is needed, upgrade this to carry a `source` field (see
    /// `Provider` variant).  For now, stringifying is sufficient and keeps
    /// the variant `#[non_exhaustive]`-stable.  Record any change in
    /// `.kata/DECISIONS.md`.
    pub fn forge_with_source(
        operation: impl Into<String>,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        let message = message.into();
        Self::Forge {
            operation: operation.into(),
            message: format!("{message}: {source}"),
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
