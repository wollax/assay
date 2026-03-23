//! Integration tests for `launch_agent_streaming`.
//!
//! Proves line-by-line delivery via mpsc channel and correct exit-code
//! reporting using real OS subprocess pipes.

use std::sync::mpsc;

use assay_core::pipeline::launch_agent_streaming;

/// Verify that `launch_agent_streaming` delivers every line to the receiver
/// channel in order, with no extra lines.
#[test]
fn streaming_delivers_lines_to_receiver() {
    let (line_tx, line_rx) = mpsc::channel::<String>();

    let handle = launch_agent_streaming(
        &[
            "sh".to_string(),
            "-c".to_string(),
            r#"printf "alpha\nbeta\ngamma\n"; exit 0"#.to_string(),
        ],
        std::path::Path::new("/tmp"),
        line_tx,
    );

    // Collect all lines until the channel closes.
    let lines: Vec<String> = line_rx.into_iter().collect();
    let exit_code = handle.join().expect("thread should not panic");

    assert_eq!(lines, vec!["alpha", "beta", "gamma"]);
    assert_eq!(exit_code, 0);
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
    assert_eq!(exit_code, 1);
}

/// Verify that `launch_agent_streaming` returns -1 for a nonexistent binary.
#[test]
fn streaming_nonexistent_binary_returns_negative_one() {
    let (line_tx, _line_rx) = mpsc::channel::<String>();

    let handle = launch_agent_streaming(
        &["__nonexistent_binary_assay_test__".to_string()],
        std::path::Path::new("/tmp"),
        line_tx,
    );

    let exit_code = handle.join().expect("thread should not panic");
    assert_eq!(exit_code, -1, "expected -1 for nonexistent binary");
}
