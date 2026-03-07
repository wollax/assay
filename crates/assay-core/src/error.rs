use std::path::PathBuf;
use thiserror::Error;

use crate::config::ConfigError;
use crate::spec::SpecError;

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

    /// Spec file parsing failed (invalid TOML or schema mismatch).
    #[error("parsing spec `{path}`: {message}")]
    SpecParse {
        /// The spec file that failed to parse.
        path: PathBuf,
        /// The parse error message (includes line/column from toml crate).
        message: String,
    },

    /// Spec validation failed (structurally valid TOML but semantically invalid).
    #[error("invalid spec `{path}`:\n{}", .errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n"))]
    SpecValidation {
        /// The spec file that failed validation.
        path: PathBuf,
        /// All validation errors found.
        errors: Vec<SpecError>,
    },

    /// Spec directory scanning failed (I/O error reading the directory).
    #[error("scanning specs directory `{path}`: {source}")]
    SpecScan {
        /// The directory that couldn't be scanned.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// A gate command failed to spawn or poll (I/O error during execution).
    #[error("gate execution failed for `{cmd}` in `{working_dir}`: {source}")]
    GateExecution {
        /// The command that failed.
        cmd: String,
        /// The working directory where execution was attempted.
        working_dir: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// A spec was not found by name in the specs directory.
    ///
    /// Forward-declared for Phase 8 (MCP `spec_get` tool). Not yet
    /// constructed in production code.
    #[error("spec `{name}` not found in {specs_dir}")]
    SpecNotFound {
        /// The spec name that was looked up.
        name: String,
        /// The directory that was searched.
        specs_dir: PathBuf,
    },

    /// Feature spec (`spec.toml`) parsing failed.
    #[error("parsing feature spec `{path}`: {message}")]
    FeatureSpecParse {
        /// The spec.toml file that failed to parse.
        path: PathBuf,
        /// The parse error message.
        message: String,
    },

    /// Feature spec (`spec.toml`) validation failed.
    #[error("invalid feature spec `{path}`:\n{}", .errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n"))]
    FeatureSpecValidation {
        /// The spec.toml file that failed validation.
        path: PathBuf,
        /// All validation errors found.
        errors: Vec<SpecError>,
    },

    /// Gates spec (`gates.toml`) parsing failed.
    #[error("parsing gates spec `{path}`: {message}")]
    GatesSpecParse {
        /// The gates.toml file that failed to parse.
        path: PathBuf,
        /// The parse error message.
        message: String,
    },

    /// Gates spec (`gates.toml`) validation failed.
    #[error("invalid gates spec `{path}`:\n{}", .errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n"))]
    GatesSpecValidation {
        /// The gates.toml file that failed validation.
        path: PathBuf,
        /// All validation errors found.
        errors: Vec<SpecError>,
    },

    /// Session not found (expired, finalized, or never created).
    #[error("session `{session_id}` not found")]
    SessionNotFound {
        /// The session ID that was looked up.
        session_id: String,
    },

    /// Criterion name not found in the spec.
    #[error("criterion `{criterion_name}` not found in spec `{spec_name}`")]
    InvalidCriterion {
        /// The spec that was searched.
        spec_name: String,
        /// The criterion name that was not found.
        criterion_name: String,
    },

    /// General session error.
    #[error("session error for `{session_id}`: {message}")]
    SessionError {
        /// The session ID involved.
        session_id: String,
        /// Description of the error.
        message: String,
    },

    /// Claude Code session directory not found.
    #[error("session directory not found: {message}")]
    SessionDirNotFound {
        /// Description of why the directory was not found.
        message: String,
    },

    /// Session JSONL file not found.
    #[error("session file not found: {path}")]
    SessionFileNotFound {
        /// The path that was looked up.
        path: PathBuf,
    },

    /// JSONL parse error (non-fatal per line, but surfaced for diagnostics).
    #[error("parsing session JSONL at {path} line {line}: {message}")]
    SessionParse {
        /// The file being parsed.
        path: PathBuf,
        /// The 1-based line number.
        line: usize,
        /// The parse error message.
        message: String,
    },

    /// Checkpoint file write failed.
    #[error("writing checkpoint at `{path}`: {message}")]
    CheckpointWrite {
        /// The file path that failed to write.
        path: PathBuf,
        /// Description of the write error.
        message: String,
    },

    /// Checkpoint file read or parse failed.
    #[error("reading checkpoint at `{path}`: {message}")]
    CheckpointRead {
        /// The file path that failed to read or parse.
        path: PathBuf,
        /// Description of the read/parse error.
        message: String,
    },

    /// Guard daemon is already running.
    #[error("guard daemon already running (PID {pid})")]
    GuardAlreadyRunning {
        /// The PID of the running guard process.
        pid: u32,
    },

    /// Guard daemon is not running (no PID file or process dead).
    #[error("guard daemon is not running")]
    GuardNotRunning,

    /// Guard circuit breaker tripped after too many recoveries.
    #[error("guard circuit breaker tripped: {recoveries} recoveries in {window_secs}s")]
    GuardCircuitBreakerTripped {
        /// Number of recoveries that triggered the breaker.
        recoveries: u32,
        /// The time window in seconds.
        window_secs: u64,
    },
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

    #[test]
    fn gate_execution_error_display() {
        let err = AssayError::GateExecution {
            cmd: "cargo test".to_string(),
            working_dir: PathBuf::from("/tmp/project"),
            source: io::Error::new(io::ErrorKind::NotFound, "No such file or directory"),
        };
        let display = err.to_string();

        assert_eq!(
            display,
            "gate execution failed for `cargo test` in `/tmp/project`: No such file or directory"
        );
    }

    #[test]
    fn gate_execution_error_source_chain() {
        use std::error::Error;

        let err = AssayError::GateExecution {
            cmd: "echo hi".to_string(),
            working_dir: PathBuf::from("/tmp"),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied"),
        };
        let source = err.source().expect("GateExecution should have a source");
        assert!(source.downcast_ref::<io::Error>().is_some());
    }

    #[test]
    fn spec_not_found_error_display() {
        let err = AssayError::SpecNotFound {
            name: "auth-flow".to_string(),
            specs_dir: PathBuf::from(".assay/specs/"),
        };
        let display = err.to_string();

        assert_eq!(display, "spec `auth-flow` not found in .assay/specs/");
    }

    // `#[non_exhaustive]` is a compile-time property: external crates matching on
    // AssayError without a wildcard arm will get a compiler error. This cannot be
    // tested at runtime within the defining crate (where exhaustive matches are
    // allowed). The attribute's presence is verified by inspection and by the
    // compiler enforcing it on downstream consumers.
}
