//! Integration tests for `launch_agent_streaming`.
//!
//! These tests define the exact API contract for streaming line delivery and
//! exit-code reporting. They are expected to **fail at runtime** until
//! `launch_agent_streaming` is implemented in T02.
//!
//! All three tests compile against the stub signature added in T01.

use std::sync::mpsc;

use assay_core::pipeline::launch_agent_streaming;

/// Verify that `launch_agent_streaming` delivers every line to the receiver
/// channel in order, with no extra lines.
#[test]
fn streaming_delivers_lines_to_receiver() {
    let (line_tx, line_rx) = mpsc::channel::<String>();

    let _handle = launch_agent_streaming(
        &[
            "sh".to_string(),
            "-c".to_string(),
            r#"printf "alpha\nbeta\ngamma\n"; exit 0"#.to_string(),
        ],
        std::path::Path::new("/tmp"),
        line_tx,
    );

    // Collect all lines until the channel closes.
    let mut lines: Vec<String> = Vec::new();
    for line in line_rx {
        lines.push(line);
    }

    assert_eq!(lines, vec!["alpha", "beta", "gamma"]);
}

/// Verify that the `JoinHandle` returned by `launch_agent_streaming` resolves
/// to the correct exit code.
#[test]
fn streaming_join_handle_returns_exit_code() {
    let (line_tx, _line_rx) = mpsc::channel::<String>();

    let handle = launch_agent_streaming(
        &["sh".to_string(), "-c".to_string(), "exit 42".to_string()],
        std::path::Path::new("/tmp"),
        line_tx,
    );

    let exit_code = handle.join().expect("thread should not panic");
    assert_eq!(exit_code, 42);
}

/// Verify that a process exiting with code 1 is reported as non-zero.
#[test]
fn streaming_failed_process_returns_nonzero() {
    let (line_tx, _line_rx) = mpsc::channel::<String>();

    let handle = launch_agent_streaming(
        &[
            "sh".to_string(),
            "-c".to_string(),
            "echo err; exit 1".to_string(),
        ],
        std::path::Path::new("/tmp"),
        line_tx,
    );

    let exit_code = handle.join().expect("thread should not panic");
    assert_ne!(exit_code, 0, "expected non-zero exit code, got {exit_code}");
    assert_eq!(exit_code, 1);
}
