use std::path::PathBuf;
use thiserror::Error;

use crate::config::ConfigError;

/// Unified error type for all Assay operations.
///
/// New variants are added as downstream phases consume them.
/// The `#[non_exhaustive]` attribute ensures adding variants
/// is not a breaking change.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AssayError {
    /// An I/O operation failed.
    #[error("{operation} at `{path}`: {source}")]
    Io {
        /// What was being attempted (e.g., "reading config", "writing spec").
        operation: String,
        /// The file path involved.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// Config file parsing failed (invalid TOML or schema mismatch).
    #[error("parsing config `{path}`: {message}")]
    ConfigParse {
        /// The config file that failed to parse.
        path: PathBuf,
        /// The parse error message (includes line/column from toml crate).
        message: String,
    },

    /// Config validation failed (structurally valid TOML but semantically invalid).
    #[error("invalid config `{path}`:\n{}", .errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n"))]
    ConfigValidation {
        /// The config file that failed validation.
        path: PathBuf,
        /// All validation errors found.
        errors: Vec<ConfigError>,
    },

    /// Init refused because `.assay/` already exists.
    #[error(".assay/ already exists. Remove it first to reinitialize.")]
    AlreadyInitialized,
}

/// Convenience result alias for Assay operations.
pub type Result<T> = std::result::Result<T, AssayError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    fn make_io_error() -> AssayError {
        AssayError::Io {
            operation: "reading config".to_string(),
            path: PathBuf::from("/tmp/config.toml"),
            source: io::Error::new(io::ErrorKind::NotFound, "No such file or directory"),
        }
    }

    #[test]
    fn io_error_display_includes_all_context() {
        let err = make_io_error();
        let display = err.to_string();

        assert!(
            display.contains("reading config"),
            "Display should contain operation, got: {display}"
        );
        assert!(
            display.contains("/tmp/config.toml"),
            "Display should contain path, got: {display}"
        );
        assert!(
            display.contains("No such file or directory"),
            "Display should contain source message, got: {display}"
        );

        // Verify the exact format
        assert_eq!(
            display,
            "reading config at `/tmp/config.toml`: No such file or directory"
        );
    }

    #[test]
    fn io_error_source_chain() {
        use std::error::Error;

        let err = make_io_error();
        let source = err.source().expect("Io variant should have a source");

        // The source should be downcastable to io::Error
        assert!(source.downcast_ref::<io::Error>().is_some());
    }

    #[test]
    fn result_alias_works() {
        fn ok_result() -> Result<()> {
            Ok(())
        }

        fn err_result() -> Result<()> {
            Err(AssayError::Io {
                operation: "writing spec".to_string(),
                path: PathBuf::from("/tmp/spec.toml"),
                source: io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied"),
            })
        }

        assert!(ok_result().is_ok());
        assert!(err_result().is_err());
    }

    // `#[non_exhaustive]` is a compile-time property: external crates matching on
    // AssayError without a wildcard arm will get a compiler error. This cannot be
    // tested at runtime within the defining crate (where exhaustive matches are
    // allowed). The attribute's presence is verified by inspection and by the
    // compiler enforcing it on downstream consumers.
}
