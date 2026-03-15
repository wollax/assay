//! Configuration loading and validation.
//!
//! Handles reading, parsing, and validating Assay project configuration
//! from files and environment.

use std::fmt;
use std::path::Path;

use assay_types::Config;

use crate::error::{AssayError, Result};

/// Result of truncating a source line to fit within a display budget.
#[derive(Debug)]
pub(crate) struct TruncatedLine {
    /// The (possibly truncated) text to display.
    pub(crate) text: String,
    /// The column offset where the caret should point within `text`.
    pub(crate) caret_offset: usize,
}

/// Translate a byte offset in a string to a (line, column) pair.
///
/// Both line and column are zero-based. Column is measured in characters
/// (not bytes) to handle multi-byte UTF-8 correctly.
pub(crate) fn translate_position(content: &str, byte_offset: usize) -> (usize, usize) {
    let clamped = byte_offset.min(content.len());
    let safe = content.floor_char_boundary(clamped);
    let before = &content[..safe];
    let line = before.matches('\n').count();
    let last_newline = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
    let col = content[last_newline..clamped].chars().count();
    (line, col)
}

/// Truncate a source line to fit within `budget` characters.
///
/// If the line already fits, returns it unchanged with the original column
/// as the caret offset. Otherwise, centers a window around `col` and adds
/// `...` ellipsis markers at the truncation points.
pub(crate) fn truncate_source_line(line: &str, col: usize, budget: usize) -> TruncatedLine {
    let chars: Vec<char> = line.chars().collect();
    if chars.len() <= budget {
        return TruncatedLine {
            text: line.to_string(),
            caret_offset: col,
        };
    }

    debug_assert!(
        budget >= 6,
        "budget must be at least 6 to fit ellipsis markers"
    );

    let ellipsis = "...";
    let elen = ellipsis.len();

    // Determine window boundaries
    let half = budget / 2;
    let mut start = col.saturating_sub(half);
    let mut end = (start + budget).min(chars.len());

    let need_left = start > 0;
    let need_right = end < chars.len();

    // Adjust for ellipsis space
    if need_left && need_right {
        // Both sides need ellipsis — shrink the visible window
        let available = budget.saturating_sub(elen * 2);
        let h = available / 2;
        start = col.saturating_sub(h);
        end = (start + available).min(chars.len());
        start = end.saturating_sub(available);
    } else if need_left {
        // Only left ellipsis
        let available = budget.saturating_sub(elen);
        start = end.saturating_sub(available);
    } else if need_right {
        // Only right ellipsis
        let available = budget.saturating_sub(elen);
        end = (start + available).min(chars.len());
    }

    let prefix = if start > 0 { ellipsis } else { "" };
    let suffix = if end < chars.len() { ellipsis } else { "" };
    let slice: String = chars[start..end].iter().collect();
    let caret_offset = col.saturating_sub(start) + prefix.len();

    TruncatedLine {
        text: format!("{prefix}{slice}{suffix}"),
        caret_offset,
    }
}

/// Format a TOML parse error with source line and caret pointer.
///
/// Uses the error's span (if available) to show the offending line from the
/// original content, truncated to ~80 characters. Falls back to just the
/// error message when no span information is available.
///
/// The file path is NOT included — callers (AssayError Display impls)
/// prepend "parsing config '{path}': " or similar.
pub(crate) fn format_toml_error(content: &str, err: &toml::de::Error) -> String {
    let message = err.message();
    let Some(span) = err.span() else {
        return message.to_string();
    };

    let (line, col) = translate_position(content, span.start);
    let source_line = content.lines().nth(line).unwrap_or("");
    let truncated = truncate_source_line(source_line, col, 80);
    let line_num = line + 1;
    let gutter_width = line_num.to_string().len();

    format!(
        "line {line_num}, column {}: {message}\n{:gutter_width$} |\n{line_num} | {}\n{:gutter_width$} | {}^",
        col + 1,
        "",
        truncated.text,
        "",
        " ".repeat(truncated.caret_offset),
    )
}

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

/// Parse a config from a TOML string without validation.
///
/// Returns the raw `toml::de::Error` on failure, preserving line/column
/// information. Callers that need validation should use [`load()`] instead.
pub fn from_str(s: &str) -> std::result::Result<Config, toml::de::Error> {
    toml::from_str(s)
}

/// Validate a parsed config for semantic correctness.
///
/// Collects **all** validation errors at once so the user can fix
/// everything in a single pass. Returns `Ok(())` when valid,
/// `Err(errors)` with every issue found otherwise.
pub fn validate(config: &Config) -> std::result::Result<(), Vec<ConfigError>> {
    let mut errors = Vec::new();

    if config.project_name.trim().is_empty() {
        errors.push(ConfigError {
            field: "project_name".into(),
            message: "required, must not be empty".into(),
        });
    }

    if config.specs_dir.trim().is_empty() {
        errors.push(ConfigError {
            field: "specs_dir".into(),
            message: "required, must not be empty".into(),
        });
    }

    if let Some(gates) = &config.gates
        && gates.default_timeout == 0
    {
        errors.push(ConfigError {
            field: "[gates].default_timeout".into(),
            message: "must be a positive integer".into(),
        });
    }

    if let Some(sessions) = &config.sessions
        && sessions.stale_threshold_secs == 0
    {
        errors.push(ConfigError {
            field: "[sessions].stale_threshold_secs".into(),
            message: "must be a positive integer (greater than zero)".into(),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Load and validate a config from `.assay/config.toml` relative to `root`.
///
/// Reads the file, parses it as TOML, and validates the result. Wraps
/// parse errors in [`AssayError::ConfigParse`] (with file path) and
/// validation errors in [`AssayError::ConfigValidation`].
pub fn load(root: &Path) -> Result<Config> {
    let path = root.join(".assay").join("config.toml");

    let content = std::fs::read_to_string(&path).map_err(|source| AssayError::Io {
        operation: "reading config".into(),
        path: path.clone(),
        source,
    })?;

    let config: Config = toml::from_str(&content).map_err(|e| AssayError::ConfigParse {
        path: path.clone(),
        message: format_toml_error(&content, &e),
    })?;

    if let Err(errors) = validate(&config) {
        return Err(AssayError::ConfigValidation { path, errors });
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    // Config and GatesConfig accessed via from_str return type and field access.

    // ── from_str tests ──────────────────────────────────────────────

    #[test]
    fn from_str_valid_all_fields() {
        let toml = r#"
project_name = "my-project"
specs_dir = "my-specs/"

[gates]
default_timeout = 600
working_dir = "/tmp"
"#;
        let config = super::from_str(toml).expect("valid TOML should parse");

        assert_eq!(config.project_name, "my-project");
        assert_eq!(config.specs_dir, "my-specs/");
        let gates = config.gates.expect("gates should be Some");
        assert_eq!(gates.default_timeout, 600);
        assert_eq!(gates.working_dir.as_deref(), Some("/tmp"));
    }

    #[test]
    fn from_str_minimal_uses_defaults() {
        let toml = r#"project_name = "test""#;
        let config = super::from_str(toml).expect("minimal TOML should parse");

        assert_eq!(config.project_name, "test");
        assert_eq!(config.specs_dir, "specs/");
        assert!(config.gates.is_none());
    }

    #[test]
    fn from_str_gates_section_uses_defaults() {
        let toml = r#"
project_name = "test"

[gates]
"#;
        let config = super::from_str(toml).expect("gates with defaults should parse");

        let gates = config.gates.expect("gates should be Some");
        assert_eq!(gates.default_timeout, 300);
        assert!(gates.working_dir.is_none());
    }

    #[test]
    fn from_str_invalid_toml_syntax() {
        let toml = "this is not valid toml ===";
        let err = super::from_str(toml).unwrap_err();
        let msg = err.to_string();

        // toml crate errors include line/column info
        assert!(
            msg.contains("TOML parse error"),
            "should contain parse error info, got: {msg}"
        );
    }

    #[test]
    fn from_str_rejects_unknown_keys() {
        let toml = r#"
project_name = "test"
unknown_key = "oops"
"#;
        let err = super::from_str(toml).unwrap_err();
        let msg = err.to_string();

        assert!(
            msg.contains("unknown field"),
            "should mention unknown field, got: {msg}"
        );
        assert!(
            msg.contains("unknown_key"),
            "should mention the bad field name, got: {msg}"
        );
    }

    #[test]
    fn from_str_rejects_unknown_gates_keys() {
        let toml = r#"
project_name = "test"

[gates]
unknown_gate_option = true
"#;
        let err = super::from_str(toml).unwrap_err();
        let msg = err.to_string();

        assert!(
            msg.contains("unknown field"),
            "should reject unknown gates key, got: {msg}"
        );
    }

    // ── validate tests ──────────────────────────────────────────────

    fn valid_config() -> assay_types::Config {
        super::from_str(r#"project_name = "test""#).unwrap()
    }

    #[test]
    fn validate_valid_config_returns_ok() {
        assert!(super::validate(&valid_config()).is_ok());
    }

    #[test]
    fn validate_valid_config_with_gates_returns_ok() {
        let config = super::from_str(
            r#"
project_name = "test"
[gates]
default_timeout = 600
"#,
        )
        .unwrap();

        assert!(super::validate(&config).is_ok());
    }

    #[test]
    fn validate_empty_project_name() {
        let mut config = valid_config();
        config.project_name = String::new();

        let errors = super::validate(&config).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "project_name");
        assert!(
            errors[0].message.contains("must not be empty"),
            "got: {}",
            errors[0].message
        );
    }

    #[test]
    fn validate_whitespace_only_project_name() {
        let mut config = valid_config();
        config.project_name = "   \t  ".to_string();

        let errors = super::validate(&config).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "project_name");
    }

    #[test]
    fn validate_empty_specs_dir() {
        let mut config = valid_config();
        config.specs_dir = String::new();

        let errors = super::validate(&config).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "specs_dir");
        assert!(
            errors[0].message.contains("must not be empty"),
            "got: {}",
            errors[0].message
        );
    }

    #[test]
    fn validate_zero_default_timeout() {
        let mut config = valid_config();
        config.gates = Some(assay_types::GatesConfig {
            default_timeout: 0,
            working_dir: None,
            max_history: None,
            evaluator_model: "sonnet".to_string(),
            evaluator_retries: 1,
            evaluator_timeout: 120,
        });

        let errors = super::validate(&config).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "[gates].default_timeout");
        assert!(
            errors[0].message.contains("positive"),
            "got: {}",
            errors[0].message
        );
    }

    // ── ERR-03: translate_position ───────────────────────────────

    #[test]
    fn translate_position_start() {
        assert_eq!(super::translate_position("hello\nworld", 0), (0, 0));
    }

    #[test]
    fn translate_position_middle_of_first_line() {
        assert_eq!(super::translate_position("hello\nworld", 3), (0, 3));
    }

    #[test]
    fn translate_position_second_line() {
        // byte 6 = 'w' in "world" (after "hello\n")
        assert_eq!(super::translate_position("hello\nworld", 6), (1, 0));
    }

    #[test]
    fn translate_position_second_line_col3() {
        assert_eq!(super::translate_position("hello\nworld", 9), (1, 3));
    }

    #[test]
    fn translate_position_at_newline() {
        // byte 5 = '\n'
        assert_eq!(super::translate_position("hello\nworld", 5), (0, 5));
    }

    #[test]
    fn translate_position_beyond_content() {
        // Clamp to end
        let (line, col) = super::translate_position("hello", 100);
        assert_eq!(line, 0);
        assert_eq!(col, 5);
    }

    // ── ERR-03: truncate_source_line ──────────────────────────────

    #[test]
    fn truncate_source_line_short_passthrough() {
        let result = super::truncate_source_line("short", 3, 80);
        assert_eq!(result.text, "short");
        assert_eq!(result.caret_offset, 3);
    }

    #[test]
    fn truncate_source_line_exact_budget() {
        let line = "a".repeat(80);
        let result = super::truncate_source_line(&line, 40, 80);
        assert_eq!(result.text, line);
        assert_eq!(result.caret_offset, 40);
    }

    #[test]
    fn truncate_source_line_long_center() {
        let line = "a".repeat(120);
        let result = super::truncate_source_line(&line, 60, 80);
        assert!(
            result.text.len() <= 80,
            "should fit in budget, got len: {}",
            result.text.len()
        );
        assert!(
            result.text.contains("..."),
            "should have ellipsis, got: {}",
            result.text
        );
    }

    #[test]
    fn truncate_source_line_col_near_start() {
        let line = "a".repeat(120);
        let result = super::truncate_source_line(&line, 5, 80);
        assert!(result.text.len() <= 80);
        // Should not have left ellipsis since col is near start
        assert!(
            !result.text.starts_with("..."),
            "should not have left ellipsis for col near start, got: {}",
            result.text
        );
    }

    #[test]
    fn truncate_source_line_col_near_end() {
        let line = "a".repeat(120);
        let result = super::truncate_source_line(&line, 115, 80);
        assert!(result.text.len() <= 80);
        // Should not have right ellipsis since col is near end
        assert!(
            !result.text.ends_with("..."),
            "should not have right ellipsis for col near end, got: {}",
            result.text
        );
    }

    // ── ERR-03: format_toml_error ─────────────────────────────────

    #[test]
    fn format_toml_error_with_span() {
        let content = "some_key = x_bad_value\n";
        let err = toml::from_str::<toml::Value>(content).unwrap_err();
        let result = super::format_toml_error(content, &err);
        assert!(
            result.contains("line 1"),
            "should contain line number, got: {result}"
        );
        assert!(
            result.contains("^"),
            "should contain caret pointer, got: {result}"
        );
    }

    #[test]
    fn format_toml_error_multiline() {
        let content = "[section]\nkey = \n";
        let err = toml::from_str::<toml::Value>(content).unwrap_err();
        let result = super::format_toml_error(content, &err);
        // Should show the specific line number and a caret pointer
        assert!(
            result.contains("line 2"),
            "should reference line 2, got: {result}"
        );
        assert!(
            result.contains('^'),
            "should contain caret pointer, got: {result}"
        );
    }

    #[test]
    fn format_toml_error_no_span() {
        // Type mismatch errors (e.g., integer where string expected) may lack span info.
        // If the error has no span, format_toml_error should return just the message.
        let content = "project_name = 42\n";
        let err = toml::from_str::<assay_types::Config>(content).unwrap_err();
        let result = super::format_toml_error(content, &err);
        // Whether or not this particular error has a span, the function should not panic.
        // If there's no span, result should be just the message (no "line" or "^").
        // If there is a span, the caret display should still work.
        assert!(!result.is_empty(), "should produce non-empty output");
    }

    #[test]
    fn validate_collects_all_errors_at_once() {
        let config = assay_types::Config {
            project_name: String::new(),
            specs_dir: String::new(),
            gates: Some(assay_types::GatesConfig {
                default_timeout: 0,
                working_dir: None,
                max_history: None,
                evaluator_model: "sonnet".to_string(),
                evaluator_retries: 1,
                evaluator_timeout: 120,
            }),
            guard: None,
            worktree: None,
            sessions: None,
        };

        let errors = super::validate(&config).unwrap_err();
        assert_eq!(
            errors.len(),
            3,
            "should collect all 3 errors, got: {errors:?}"
        );

        let fields: Vec<&str> = errors.iter().map(|e| e.field.as_str()).collect();
        assert!(fields.contains(&"project_name"));
        assert!(fields.contains(&"specs_dir"));
        assert!(fields.contains(&"[gates].default_timeout"));
    }

    // ── load tests ──────────────────────────────────────────────────

    use std::io::Write;

    fn write_config(root: &std::path::Path, content: &str) {
        let assay_dir = root.join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();
        let config_path = assay_dir.join("config.toml");
        let mut f = std::fs::File::create(&config_path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn load_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        write_config(dir.path(), r#"project_name = "loaded""#);

        let config = super::load(dir.path()).expect("valid config should load");

        assert_eq!(config.project_name, "loaded");
        assert_eq!(config.specs_dir, "specs/");
        assert!(config.gates.is_none());
    }

    #[test]
    fn load_missing_file_returns_io_error() {
        let dir = tempfile::tempdir().unwrap();
        // No .assay/config.toml created

        let err = super::load(dir.path()).unwrap_err();
        assert!(
            matches!(err, crate::error::AssayError::Io { .. }),
            "expected Io error, got: {err:?}"
        );
    }

    #[test]
    fn load_invalid_toml_returns_config_parse() {
        let dir = tempfile::tempdir().unwrap();
        write_config(dir.path(), "not valid toml ===");

        let err = super::load(dir.path()).unwrap_err();
        match &err {
            crate::error::AssayError::ConfigParse { path, message } => {
                assert!(
                    path.ends_with("config.toml"),
                    "path should end with config.toml, got: {path:?}"
                );
                assert!(
                    message.contains("line"),
                    "message should contain line info from format_toml_error, got: {message}"
                );
            }
            other => panic!("expected ConfigParse, got: {other:?}"),
        }
    }

    #[test]
    fn load_valid_toml_invalid_semantics_returns_config_validation() {
        let dir = tempfile::tempdir().unwrap();
        // Empty project_name is parseable but semantically invalid
        write_config(dir.path(), r#"project_name = """#);

        let err = super::load(dir.path()).unwrap_err();
        match &err {
            crate::error::AssayError::ConfigValidation { path, errors } => {
                assert!(
                    path.ends_with("config.toml"),
                    "path should end with config.toml, got: {path:?}"
                );
                assert!(
                    !errors.is_empty(),
                    "should have at least one validation error"
                );
            }
            other => panic!("expected ConfigValidation, got: {other:?}"),
        }
    }

    // ── SessionsConfig tests ──────────────────────────────────────

    #[test]
    fn from_str_without_sessions_section_parses_as_none() {
        let toml = r#"project_name = "test""#;
        let config = super::from_str(toml).expect("should parse without sessions");
        assert!(config.sessions.is_none());
    }

    #[test]
    fn from_str_with_sessions_section_uses_defaults() {
        let toml = r#"
project_name = "test"

[sessions]
"#;
        let config = super::from_str(toml).expect("sessions with defaults should parse");
        let sessions = config.sessions.expect("sessions should be Some");
        assert_eq!(sessions.stale_threshold_secs, 3600);
    }

    #[test]
    fn from_str_with_custom_stale_threshold() {
        let toml = r#"
project_name = "test"

[sessions]
stale_threshold = 7200
"#;
        let config = super::from_str(toml).expect("custom threshold should parse via alias");
        let sessions = config.sessions.expect("sessions should be Some");
        assert_eq!(sessions.stale_threshold_secs, 7200);
    }

    #[test]
    fn from_str_rejects_unknown_sessions_keys() {
        let toml = r#"
project_name = "test"

[sessions]
unknown_option = true
"#;
        let err = super::from_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown field"),
            "should reject unknown sessions key, got: {msg}"
        );
    }

    #[test]
    fn validate_rejects_zero_stale_threshold_secs() {
        let mut config = valid_config();
        config.sessions = Some(assay_types::SessionsConfig {
            stale_threshold_secs: 0,
        });

        let errors = super::validate(&config).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.field.contains("stale_threshold_secs")),
            "should report stale_threshold_secs validation error, got: {errors:?}"
        );
        assert!(
            errors.iter().any(|e| e.message.contains("positive")),
            "error message should mention positive, got: {errors:?}"
        );
    }

    #[test]
    fn stale_threshold_secs_alias_backward_compat() {
        // Old config files using "stale_threshold" should still parse via serde alias.
        let toml = r#"
project_name = "test"

[sessions]
stale_threshold = 7200
"#;
        let config = super::from_str(toml).expect("old stale_threshold key should parse via alias");
        let sessions = config.sessions.expect("sessions should be Some");
        assert_eq!(
            sessions.stale_threshold_secs, 7200,
            "serde alias should map old key to new field"
        );
    }
}
